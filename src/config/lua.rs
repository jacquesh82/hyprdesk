//! Chargement de `hyprdesk.lua` : l'environnement `hd.*`, et le thread qui
//! possède l'état Lua (fonctions `render` des widgets, hooks `hd.on`).
//!
//! L'état Lua n'est pas `Send` : il vit sur un thread dédié, et le reste du
//! programme lui parle par messages. Un rechargement construit un NOUVEL état ;
//! l'ancien n'est remplacé que si le nouveau fichier s'exécute sans erreur.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mlua::{Function, Lua, RegistryKey, Table, Value};
use tokio::sync::oneshot;

use super::model::*;

/// Fonctions Lua retenues par une config (render, hooks), indexées par position.
struct Builder {
    config: Config,
    funcs: Vec<RegistryKey>,
    base_dir: PathBuf,
    /// Fichier en cours d'exécution : `settings.lua` (écran Réglages) ou celui de l'utilisateur.
    origin: Origin,
    /// Plugins de widgets chargés, par nom.
    plugins: std::collections::HashMap<String, Plugin>,
    /// `hd.place` dans l'ordre d'appel : les réglages, puis l'utilisateur.
    places: Vec<(Origin, String, PlacePatch)>,
}

/// Plugin de widget chargé : sa table Lua et ses valeurs par défaut.
struct Plugin {
    table: RegistryKey,
    interval: f64,
    anchor: Anchor,
    options: Vec<String>,
}

/// Placement partiel (`hd.place`) : les champs absents gardent leur valeur.
#[derive(Default)]
struct PlacePatch {
    anchor: Option<Anchor>,
    x: Option<f32>,
    y: Option<f32>,
    size: Option<f32>,
    card: Option<bool>,
}

fn parse_anchor(s: &str, ctx: &str) -> mlua::Result<Anchor> {
    Anchor::parse(s).ok_or_else(|| {
        err(format!("{ctx} : anchor « {s} » inconnu (top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right)"))
    })
}

/// Charge un plugin de widget : un fichier Lua qui rend une table avec `render`.
fn load_plugin(lua: &Lua, id: &str, path: &Path) -> Result<(Plugin, WidgetType), String> {
    let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: Value = lua.load(src.as_str()).set_name(format!("@{}", path.display())).eval().map_err(|e| short_lua_error(&e))?;
    let Value::Table(t) = v else { return Err("le fichier doit rendre une table (return { … })".into()) };
    let ctx = format!("plugin « {id} »");
    let run = || -> mlua::Result<(Plugin, WidgetType)> {
        check_keys(&t, &["name", "description", "interval", "anchor", "options", "render"], &ctx)?;
        if !matches!(t.get::<Value>("render")?, Value::Function(_)) {
            return Err(err(format!("{ctx} : « render » doit être une fonction")));
        }
        let anchor = match opt_string(&t, "anchor", &ctx)? {
            Some(a) => parse_anchor(&a, &ctx)?,
            None => Anchor::TopRight,
        };
        let mut options = Vec::new();
        if let Some(o) = t.get::<Option<Table>>("options")? {
            for pair in o.pairs::<String, Value>() {
                let k = pair?.0;
                if crate::widgets::COMMON_OPTIONS.contains(&k.as_str()) {
                    return Err(err(format!("{ctx} : l'option « {k} » est réservée")));
                }
                options.push(k);
            }
        }
        options.sort();
        let wt = WidgetType {
            id: id.to_string(),
            name: opt_string(&t, "name", &ctx)?.unwrap_or_else(|| id.to_string()),
            description: opt_string(&t, "description", &ctx)?.unwrap_or_default(),
            source: WidgetSource::Plugin(path.to_path_buf()),
            anchor,
            toggleable: true,
            error: None,
        };
        let interval = opt_num(&t, "interval", &ctx)?.unwrap_or(60.0).max(0.1);
        Ok((Plugin { table: lua.create_registry_value(t.clone())?, interval, anchor, options }, wt))
    };
    run().map_err(|e| short_lua_error(&e))
}

/// Clés présentes dans une table `hd.config`, en notation pointée (`grid.icon_size`).
fn present_keys(t: &Table) -> mlua::Result<Vec<String>> {
    let mut out = Vec::new();
    for pair in t.pairs::<String, Value>() {
        let (k, v) = pair?;
        match v {
            Value::Table(sub) if k == "grid" || k == "theme" || k == "keys" => {
                for p in sub.pairs::<String, Value>() {
                    out.push(format!("{k}.{}", p?.0));
                }
            }
            _ => out.push(k),
        }
    }
    Ok(out)
}

fn err(msg: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(msg.into())
}

/// Refuse les champs inconnus : une faute de frappe doit se voir, comme dans Hyprland.
fn check_keys(t: &Table, allowed: &[&str], ctx: &str) -> mlua::Result<()> {
    for pair in t.pairs::<Value, Value>() {
        let (k, _) = pair?;
        let Value::String(k) = k else {
            return Err(err(format!("{ctx} : les clés doivent être des chaînes")));
        };
        let k = k.to_str()?.to_string();
        if !allowed.contains(&k.as_str()) {
            return Err(err(format!("{ctx} : champ inconnu « {k} » (attendus : {})", allowed.join(", "))));
        }
    }
    Ok(())
}

fn opt_string(t: &Table, key: &str, ctx: &str) -> mlua::Result<Option<String>> {
    match t.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::String(s) => Ok(Some(s.to_str()?.to_string())),
        Value::Integer(i) => Ok(Some(i.to_string())),
        Value::Number(n) => Ok(Some(n.to_string())),
        v => Err(err(format!("{ctx} : « {key} » doit être une chaîne, pas {}", v.type_name()))),
    }
}

fn opt_bool(t: &Table, key: &str, ctx: &str) -> mlua::Result<Option<bool>> {
    match t.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Boolean(b) => Ok(Some(b)),
        v => Err(err(format!("{ctx} : « {key} » doit être un booléen, pas {}", v.type_name()))),
    }
}

fn opt_num(t: &Table, key: &str, ctx: &str) -> mlua::Result<Option<f64>> {
    match t.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Integer(i) => Ok(Some(i as f64)),
        Value::Number(n) => Ok(Some(n)),
        v => Err(err(format!("{ctx} : « {key} » doit être un nombre, pas {}", v.type_name()))),
    }
}

fn opt_color(t: &Table, key: &str, ctx: &str) -> mlua::Result<Option<Rgba>> {
    match opt_string(t, key, ctx)? {
        None => Ok(None),
        Some(s) => Rgba::parse(&s)
            .map(Some)
            .ok_or_else(|| err(format!("{ctx} : couleur « {s} » illisible (#rrggbb, #rrggbbaa, rgb(…), rgba(…))"))),
    }
}

/// Une chaîne, un nombre, ou une liste des deux ; `"*"` signifie « tous ».
fn string_list(v: Value, key: &str, ctx: &str) -> mlua::Result<Option<Vec<String>>> {
    let one = |v: Value| -> mlua::Result<String> {
        match v {
            Value::String(s) => Ok(s.to_str()?.to_string()),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Number(n) if n.fract() == 0.0 => Ok((n as i64).to_string()),
            v => Err(err(format!("{ctx} : « {key} » attend une chaîne, un nombre ou une liste, pas {}", v.type_name()))),
        }
    };
    let list = match v {
        Value::Nil => return Ok(None),
        Value::Table(t) => t.sequence_values::<Value>().map(|v| one(v?)).collect::<mlua::Result<Vec<_>>>()?,
        v => vec![one(v)?],
    };
    if list.iter().any(|s| s == "*") {
        return Ok(None);
    }
    if list.is_empty() {
        return Err(err(format!("{ctx} : « {key} » est une liste vide")));
    }
    Ok(Some(list))
}

fn selector(t: &Table, ctx: &str) -> mlua::Result<Selector> {
    Ok(Selector {
        workspaces: string_list(t.get("workspace")?, "workspace", ctx)?,
        monitors: string_list(t.get("monitor")?, "monitor", ctx)?,
    })
}

/// `~/x` → `$HOME/x` ; un chemin relatif part du dossier de la config.
pub fn expand_path(p: &str, base: &Path) -> PathBuf {
    if p == "~" {
        return home();
    }
    if let Some(rest) = p.strip_prefix("~/") {
        return home().join(rest);
    }
    let pb = PathBuf::from(p);
    if pb.is_absolute() { pb } else { base.join(pb) }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn parse_mode(s: &str, ctx: &str) -> mlua::Result<Mode> {
    match s {
        "opaque" => Ok(Mode::Opaque),
        "glass" => Ok(Mode::Glass),
        _ => Err(err(format!("{ctx} : mode « {s} » inconnu (opaque, glass)"))),
    }
}

fn hd_config(b: &mut Builder, t: &Table) -> mlua::Result<()> {
    const CTX: &str = "hd.config";
    check_keys(
        t,
        &[
            "keys", "suspend_binds", "float_launched", "mode", "fade_ms", "transition", "crossfade_ms", "running_apps", "show_on", "close_on_launch", "icon_theme",
            "font", "font_size", "terminal", "grid", "theme", "glass_opacity", "background",
        ],
        CTX,
    )?;
    let keys = present_keys(t)?;
    match b.origin {
        Origin::Settings => b.config.keys_settings.extend(keys),
        Origin::User => b.config.keys_user.extend(keys),
    }
    let g = &mut b.config.general;
    if let Some(v) = opt_bool(t, "suspend_binds", CTX)? {
        g.suspend_binds = v;
    }
    if let Some(v) = opt_bool(t, "float_launched", CTX)? {
        g.float_launched = v;
    }
    if let Some(k) = t.get::<Option<Table>>("keys")? {
        const C: &str = "hd.config.keys";
        check_keys(&k, &["toggle", "settings", "leave"], C)?;
        for (name, slot) in [("toggle", &mut g.keys.toggle), ("settings", &mut g.keys.settings), ("leave", &mut g.keys.leave)] {
            match k.get::<Value>(name)? {
                Value::Nil => {}
                Value::Boolean(false) => slot.clear(),
                Value::String(s) => {
                    let s = s.to_str()?.trim().to_string();
                    if !s.is_empty() && crate::hypr::parse_combo(&s).is_none() {
                        return Err(err(format!("{C} : raccourci « {s} » illisible (ex. \"SUPER + SHIFT + D\")")));
                    }
                    *slot = s;
                }
                v => return Err(err(format!("{C} : « {name} » attend une chaîne ou false, pas {}", v.type_name()))),
            }
        }
    }
    if let Some(s) = opt_string(t, "transition", CTX)? {
        g.transition = Transition::parse(&s).ok_or_else(|| err(format!("{CTX} : transition « {s} » inconnue (fade, slide, none)")))?;
    }
    if let Some(v) = opt_num(t, "crossfade_ms", CTX)? {
        g.crossfade_ms = v.clamp(0.0, 10000.0) as u64;
    }
    if let Some(s) = opt_string(t, "running_apps", CTX)? {
        g.running_apps =
            RunningApps::parse(&s).ok_or_else(|| err(format!("{CTX} : running_apps « {s} » inconnu (off, workspace, all)")))?;
    }
    if let Some(m) = opt_string(t, "mode", CTX)? {
        g.mode = parse_mode(&m, CTX)?;
    }
    if let Some(v) = opt_num(t, "fade_ms", CTX)? {
        g.fade_ms = v.clamp(0.0, 5000.0) as u64;
    }
    if let Some(s) = opt_string(t, "show_on", CTX)? {
        g.show_on = match s.as_str() {
            "all" => ShowOn::All,
            "focused" => ShowOn::Focused,
            _ => return Err(err(format!("{CTX} : show_on « {s} » inconnu (all, focused)"))),
        };
    }
    if let Some(v) = opt_bool(t, "close_on_launch", CTX)? {
        g.close_on_launch = v;
    }
    if let Some(v) = opt_string(t, "icon_theme", CTX)? {
        g.icon_theme = Some(v);
    }
    if let Some(v) = opt_string(t, "font", CTX)? {
        g.font = Some(v);
    }
    if let Some(v) = opt_num(t, "font_size", CTX)? {
        g.font_size = v.clamp(6.0, 96.0) as f32;
    }
    if let Some(v) = opt_string(t, "terminal", CTX)? {
        g.terminal = v;
    }
    if let Some(v) = opt_num(t, "glass_opacity", CTX)? {
        g.glass_opacity = v.clamp(0.0, 1.0) as f32;
    }
    if let Some(c) = opt_color(t, "background", CTX)? {
        g.background = c;
    }
    if let Some(grid) = t.get::<Option<Table>>("grid")? {
        const C: &str = "hd.config.grid";
        check_keys(&grid, &["cell", "gap", "margin", "icon_size"], C)?;
        let px = |k: &str| -> mlua::Result<Option<u16>> { Ok(opt_num(&grid, k, C)?.map(|v| v.clamp(0.0, 1024.0) as u16)) };
        if let Some(v) = px("cell")? {
            g.grid.cell = v.max(16);
        }
        if let Some(v) = px("gap")? {
            g.grid.gap = v;
        }
        if let Some(v) = px("margin")? {
            g.grid.margin = v;
        }
        if let Some(v) = px("icon_size")? {
            g.grid.icon_size = v.max(8);
        }
    }
    if let Some(th) = t.get::<Option<Table>>("theme")? {
        const C: &str = "hd.config.theme";
        check_keys(&th, &["fg", "bg", "accent"], C)?;
        if let Some(c) = opt_color(&th, "fg", C)? {
            g.theme.fg = c;
        }
        if let Some(c) = opt_color(&th, "bg", C)? {
            g.theme.bg = c;
        }
        if let Some(c) = opt_color(&th, "accent", C)? {
            g.theme.accent = c;
        }
    }
    Ok(())
}

fn hd_wallpaper(b: &mut Builder, t: &Table) -> mlua::Result<()> {
    const CTX: &str = "hd.wallpaper";
    check_keys(t, &["workspace", "monitor", "path", "rotate", "color", "fit"], CTX)?;
    let sel = selector(t, CTX)?;
    let fit = match opt_string(t, "fit", CTX)?.as_deref() {
        None | Some("cover") => Fit::Cover,
        Some("contain") => Fit::Contain,
        Some("fill") => Fit::Fill,
        Some("center") => Fit::Center,
        Some(s) => return Err(err(format!("{CTX} : fit « {s} » inconnu (cover, contain, fill, center)"))),
    };
    let path = opt_string(t, "path", CTX)?;
    let rotate = t.get::<Option<Table>>("rotate")?;
    let color = opt_color(t, "color", CTX)?;
    let source = match (path, rotate, color) {
        (Some(p), None, None) => WallSource::Path(expand_path(&p, &b.base_dir)),
        (None, Some(r), None) => {
            const C: &str = "hd.wallpaper.rotate";
            check_keys(&r, &["dir", "interval", "shuffle"], C)?;
            let dir = opt_string(&r, "dir", C)?.ok_or_else(|| err(format!("{C} : « dir » est requis")))?;
            let secs = opt_num(&r, "interval", C)?.unwrap_or(600.0);
            if secs < 1.0 {
                return Err(err(format!("{C} : « interval » doit valoir au moins 1 seconde")));
            }
            WallSource::Rotate {
                dir: expand_path(&dir, &b.base_dir),
                interval: Duration::from_secs_f64(secs),
                shuffle: opt_bool(&r, "shuffle", C)?.unwrap_or(false),
            }
        }
        (None, None, Some(c)) => WallSource::Color(c),
        _ => return Err(err(format!("{CTX} : indiquer exactement un de « path », « rotate » ou « color »"))),
    };
    b.config.wallpapers.push(WallpaperRule { origin: b.origin, sel, source, fit });
    Ok(())
}

fn hd_desktop(b: &mut Builder, t: &Table) -> mlua::Result<()> {
    const CTX: &str = "hd.desktop";
    check_keys(t, &["workspace", "monitor", "mode", "inherit"], CTX)?;
    let sel = selector(t, CTX)?;
    let mode = opt_string(t, "mode", CTX)?.map(|m| parse_mode(&m, CTX)).transpose()?;
    let inherit = opt_bool(t, "inherit", CTX)?.unwrap_or(true);
    b.config.desktops.push(DesktopRule { origin: b.origin, sel, mode, inherit });
    Ok(())
}

fn hd_icon(b: &mut Builder, t: &Table) -> mlua::Result<()> {
    const CTX: &str = "hd.icon";
    check_keys(t, &["workspace", "monitor", "id", "label", "app", "path", "exec", "terminal", "icon", "pos"], CTX)?;
    let sel = selector(t, CTX)?;
    let app = opt_string(t, "app", CTX)?;
    let path = opt_string(t, "path", CTX)?;
    let exec = opt_string(t, "exec", CTX)?;
    let terminal = opt_bool(t, "terminal", CTX)?.unwrap_or(false);
    let label = opt_string(t, "label", CTX)?;
    let (kind, default_id) = match (app, path, exec) {
        (Some(a), None, None) => (IconKind::App(a.trim_end_matches(".desktop").to_string()), format!("app:{a}")),
        (None, Some(p), None) => (IconKind::Path(expand_path(&p, &b.base_dir)), format!("path:{p}")),
        (None, None, Some(c)) => {
            let id = format!("exec:{}", label.as_deref().unwrap_or(&c));
            (IconKind::Exec { cmd: c, terminal }, id)
        }
        _ => return Err(err(format!("{CTX} : indiquer exactement un de « app », « path » ou « exec »"))),
    };
    let pos = match t.get::<Option<Table>>("pos")? {
        None => None,
        Some(p) => {
            let col: f64 = p.get(1)?;
            let row: f64 = p.get(2)?;
            if col < 0.0 || row < 0.0 {
                return Err(err(format!("{CTX} : « pos » attend {{colonne, ligne}} positifs, à partir de 0")));
            }
            Some((col as u16, row as u16))
        }
    };
    let id = opt_string(t, "id", CTX)?.unwrap_or(default_id);
    if b.config.icons.iter().any(|i| i.id == id && i.sel == sel) {
        return Err(err(format!("{CTX} : icône « {id} » déclarée deux fois pour le même bureau (préciser « id »)")));
    }
    b.config.icons.push(IconDef { origin: b.origin, sel, id, label, kind, icon: opt_string(t, "icon", CTX)?, pos });
    Ok(())
}

fn hd_widget(lua: &Lua, b: &mut Builder, kind: &str, t: &Table) -> mlua::Result<()> {
    let ctx = format!("hd.widget(\"{kind}\")");
    let ctx = ctx.as_str();
    let common = crate::widgets::COMMON_OPTIONS;
    let (default_interval, default_anchor, kind_v) = if let Some(bi) = crate::widgets::builtin(kind) {
        let allowed: Vec<&str> = common.iter().chain(bi.options).copied().collect();
        check_keys(t, &allowed, ctx)?;
        let k = match kind {
            "clock" => WidgetKind::Clock { format: opt_string(t, "format", ctx)?.unwrap_or_else(|| "%H:%M".into()) },
            "calendar" => WidgetKind::Calendar,
            "system" => {
                let show = match t.get::<Option<Table>>("show")? {
                    None => vec![SysItem::Cpu, SysItem::Ram, SysItem::Disk("/".into())],
                    Some(list) => list
                        .sequence_values::<String>()
                        .map(|s| {
                            let s = s?;
                            SysItem::parse(&s)
                                .ok_or_else(|| err(format!("{ctx} : élément « {s} » inconnu (cpu, ram, load, uptime, disk:<point de montage>)")))
                        })
                        .collect::<mlua::Result<Vec<_>>>()?,
                };
                WidgetKind::System { show }
            }
            "media" => WidgetKind::Media { player: opt_string(t, "player", ctx)?.unwrap_or_else(|| "*".into()) },
            "command" => {
                let exec = opt_string(t, "exec", ctx)?.ok_or_else(|| err(format!("{ctx} : « exec » est requis")))?;
                let timeout = Duration::from_secs_f64(opt_num(t, "timeout", ctx)?.unwrap_or(10.0).clamp(0.1, 600.0));
                WidgetKind::Command { exec, timeout }
            }
            "lua" => {
                let f: Function = match t.get::<Value>("render")? {
                    Value::Function(f) => f,
                    _ => return Err(err(format!("{ctx} : « render » doit être une fonction"))),
                };
                b.funcs.push(lua.create_registry_value(f)?);
                WidgetKind::Lua { func: b.funcs.len() - 1 }
            }
            other => return Err(err(format!("{ctx} : widget intégré « {other} » sans configuration"))),
        };
        (bi.interval, bi.anchor, k)
    } else if let Some(p) = b.plugins.get(kind) {
        // Plugin : ses options, avec ses valeurs par défaut, passées à render(opts).
        let allowed: Vec<&str> = common.iter().copied().chain(p.options.iter().map(String::as_str)).collect();
        check_keys(t, &allowed, ctx)?;
        let table: Table = lua.registry_value(&p.table)?;
        let opts = lua.create_table()?;
        if let Some(defaults) = table.get::<Option<Table>>("options")? {
            for pair in defaults.pairs::<Value, Value>() {
                let (k, v) = pair?;
                opts.set(k, v)?;
            }
        }
        for k in &p.options {
            let v: Value = t.get(k.as_str())?;
            if !v.is_nil() {
                opts.set(k.as_str(), v)?;
            }
        }
        let render: Function = table.get("render")?;
        let wrapper: Function = lua.load("local f, o = ... return function() return f(o) end").set_name("=hyprdesk").call((render, opts))?;
        b.funcs.push(lua.create_registry_value(wrapper)?);
        (p.interval, p.anchor, WidgetKind::Lua { func: b.funcs.len() - 1 })
    } else {
        let mut known: Vec<String> = crate::widgets::BUILTINS.iter().map(|b| b.id.to_string()).collect();
        let mut plugins: Vec<String> = b.plugins.keys().cloned().collect();
        plugins.sort();
        known.extend(plugins);
        return Err(err(format!(
            "hd.widget : type « {kind} » inconnu (disponibles : {} ; plugins dans {})",
            known.join(", "),
            b.base_dir.join("hyprdesk/widgets").display()
        )));
    };
    let anchor = match opt_string(t, "anchor", ctx)? {
        None => default_anchor,
        Some(a) => parse_anchor(&a, ctx)?,
    };
    let interval = opt_num(t, "interval", ctx)?.unwrap_or(default_interval);
    if interval < 0.1 {
        return Err(err(format!("{ctx} : « interval » doit valoir au moins 0.1 seconde")));
    }
    // Identifiant par défaut stable : compté par type ET par origine, pour que
    // les widgets ajoutés dans les réglages ne décalent pas ceux de hyprdesk.lua.
    let n = b.config.widgets.iter().filter(|w| w.type_id == kind && w.origin == b.origin).count();
    let default_id = match b.origin {
        Origin::User => format!("{kind}#{n}"),
        Origin::Settings => format!("reglages:{kind}#{n}"),
    };
    let id = opt_string(t, "id", ctx)?.unwrap_or(default_id);
    if b.config.widgets.iter().any(|w| w.id == id) {
        return Err(err(format!("{ctx} : un widget « {id} » existe déjà (préciser un autre « id »)")));
    }
    b.config.widgets.push(WidgetDef {
        origin: b.origin,
        sel: selector(t, ctx)?,
        id,
        type_id: kind.to_string(),
        kind: kind_v,
        anchor,
        x: opt_num(t, "x", ctx)?.unwrap_or(0.0) as f32,
        y: opt_num(t, "y", ctx)?.unwrap_or(0.0) as f32,
        size: opt_num(t, "size", ctx)?.map(|v| v.clamp(0.0, 400.0) as f32).unwrap_or(0.0),
        interval: Duration::from_secs_f64(interval),
        card: opt_bool(t, "card", ctx)?.unwrap_or(true),
    });
    Ok(())
}

fn hd_place(b: &mut Builder, id: &str, t: &Table) -> mlua::Result<()> {
    let ctx = format!("hd.place(\"{id}\")");
    let ctx = ctx.as_str();
    check_keys(t, &["anchor", "x", "y", "size", "card"], ctx)?;
    let patch = PlacePatch {
        anchor: opt_string(t, "anchor", ctx)?.map(|a| parse_anchor(&a, ctx)).transpose()?,
        x: opt_num(t, "x", ctx)?.map(|v| v as f32),
        y: opt_num(t, "y", ctx)?.map(|v| v as f32),
        size: opt_num(t, "size", ctx)?.map(|v| v.clamp(0.0, 400.0) as f32),
        card: opt_bool(t, "card", ctx)?,
    };
    b.places.push((b.origin, id.to_string(), patch));
    Ok(())
}

/// Applique les `hd.place` une fois tous les widgets déclarés (ceux de
/// hyprdesk.lua compris) ; retient ceux des réglages pour l'écran ⚙.
fn apply_places(b: &mut Builder) {
    for (origin, id, p) in std::mem::take(&mut b.places) {
        let Some(w) = b.config.widgets.iter_mut().find(|w| w.id == id) else {
            tracing::warn!("hd.place : aucun widget « {id} »");
            continue;
        };
        if let Some(v) = p.anchor {
            w.anchor = v;
        }
        if let Some(v) = p.x {
            w.x = v;
        }
        if let Some(v) = p.y {
            w.y = v;
        }
        if let Some(v) = p.size {
            w.size = v;
        }
        if let Some(v) = p.card {
            w.card = v;
        }
        if origin == Origin::Settings {
            b.config.placements.insert(id, w.placement());
        }
    }
}

/// Exécute une commande shell et rend sa sortie standard (aide `hd.exec`).
fn shell_output(cmd: &str) -> String {
    match std::process::Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim_end().to_string(),
        Err(e) => format!("erreur : {e}"),
    }
}

fn install_api(lua: &Lua) -> mlua::Result<()> {
    let hd = lua.create_table()?;

    fn with_builder<R>(lua: &Lua, f: impl FnOnce(&mut Builder) -> mlua::Result<R>) -> mlua::Result<R> {
        let cell = lua.app_data_ref::<RefCell<Builder>>().ok_or_else(|| err("hd.* appelé hors du chargement de la config"))?;
        let mut b = cell.borrow_mut();
        f(&mut b)
    }

    hd.set("_config", lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_config(b, &t)))?)?;
    hd.set("_wallpaper", lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_wallpaper(b, &t)))?)?;
    hd.set("_desktop", lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_desktop(b, &t)))?)?;
    hd.set("_icon", lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_icon(b, &t)))?)?;
    hd.set(
        "_widget",
        lua.create_function(|lua, (kind, t): (String, Option<Table>)| {
            let t = match t {
                Some(t) => t,
                None => lua.create_table()?,
            };
            with_builder(lua, |b| hd_widget(lua, b, &kind, &t))
        })?,
    )?;
    hd.set(
        "_on",
        lua.create_function(|lua, (event, f): (String, Function)| {
            let ev = HookEvent::parse(&event).ok_or_else(|| err(format!("hd.on : événement « {event} » inconnu (show, hide, workspace)")))?;
            let key = lua.create_registry_value(f)?;
            with_builder(lua, |b| {
                b.funcs.push(key);
                let idx = b.funcs.len() - 1;
                b.config.hooks.push((ev, idx));
                Ok(())
            })
        })?,
    )?;
    hd.set(
        "_place",
        lua.create_function(|lua, (id, t): (String, Table)| with_builder(lua, |b| hd_place(b, &id, &t)))?,
    )?;
    hd.set("exec", lua.create_function(|_, cmd: String| Ok(shell_output(&cmd)))?)?;
    hd.set("version", env!("CARGO_PKG_VERSION"))?;
    lua.globals().set("hd", hd)?;
    // Les fonctions Rust ne connaissent pas la ligne appelante : chaque `hd.x`
    // relance l'erreur au niveau 2, pour qu'elle désigne `fichier:ligne` de
    // l'appel dans la config, comme une erreur Lua ordinaire.
    lua.load(
        r#"
        for _, name in ipairs({ "config", "wallpaper", "desktop", "icon", "widget", "place", "on" }) do
            local f = hd["_" .. name]
            hd["_" .. name] = nil
            hd[name] = function(...)
                local ok, e = pcall(f, ...)
                if not ok then
                    local msg = tostring(e):match("^[^\n]*"):gsub("^runtime error: ", "")
                    error(msg, 2)
                end
            end
        end
        "#,
    )
    .set_name("=hyprdesk")
    .exec()?;
    Ok(())
}

/// État Lua chargé : la config et les fonctions qu'elle référence.
pub struct Loaded {
    pub lua: Lua,
    pub config: Config,
    funcs: Vec<RegistryKey>,
}

impl Loaded {
    fn func(&self, idx: usize) -> mlua::Result<Function> {
        let key = self.funcs.get(idx).ok_or_else(|| err(format!("fonction Lua #{idx} introuvable")))?;
        self.lua.registry_value(key)
    }

    /// Appelle la fonction `render` d'un widget Lua.
    pub fn render(&self, idx: usize) -> Result<String, String> {
        let f = self.func(idx).map_err(|e| e.to_string())?;
        match f.call::<Value>(()) {
            Ok(Value::String(s)) => Ok(s.to_string_lossy().to_string()),
            Ok(Value::Nil) => Ok(String::new()),
            Ok(Value::Integer(i)) => Ok(i.to_string()),
            Ok(Value::Number(n)) => Ok(n.to_string()),
            Ok(v) => Err(format!("render doit rendre une chaîne, pas {}", v.type_name())),
            Err(e) => Err(short_lua_error(&e)),
        }
    }

    /// Appelle un hook `hd.on` avec `{ event, monitor, workspace }`.
    pub fn hook(&self, idx: usize, event: &str, monitor: &str, workspace: &str) -> Result<(), String> {
        let f = self.func(idx).map_err(|e| e.to_string())?;
        let t = self.lua.create_table().map_err(|e| e.to_string())?;
        let _ = t.set("event", event);
        let _ = t.set("monitor", monitor);
        let _ = t.set("workspace", workspace);
        f.call::<()>(t).map_err(|e| short_lua_error(&e))
    }
}

/// Première ligne utile d'une erreur Lua (sans la pile d'appels).
pub fn short_lua_error(e: &mlua::Error) -> String {
    let s = e.to_string();
    let line = s.lines().next().unwrap_or(&s).trim();
    let line = line.strip_prefix("runtime error: ").unwrap_or(line);
    line.strip_prefix("syntax error: ").unwrap_or(line).to_string()
}

/// Exécute une source de config. `base_dir` sert aux chemins relatifs et à `require`.
/// Exécute une source de config. `base_dir` sert aux chemins relatifs et à `require`.
#[cfg(test)]
pub fn load_source(src: &str, chunk_name: &str, base_dir: &Path) -> Result<Loaded, String> {
    load_chunks(&[(src.to_string(), chunk_name.to_string(), Origin::User)], base_dir)
}

/// Exécute plusieurs sources dans le MÊME état Lua, dans l'ordre : ce qui
/// vient après l'emporte (dernière déclaration, dernière valeur de `hd.config`).
pub fn load_chunks(chunks: &[(String, String, Origin)], base_dir: &Path) -> Result<Loaded, String> {
    let lua = Lua::new();
    let setup = || -> mlua::Result<()> {
        install_api(&lua)?;
        let package: Table = lua.globals().get("package")?;
        let old: String = package.get("path")?;
        let mods = base_dir.join("hyprdesk");
        package.set("path", format!("{0}/?.lua;{0}/?/init.lua;{old}", mods.display()))?;
        Ok(())
    };
    setup().map_err(|e| format!("initialisation Lua : {e}"))?;
    // Plugins de widgets : chargés avant la config, chacun isolément ; un
    // plugin cassé apparaît en erreur dans le catalogue sans bloquer le reste.
    let mut catalog = crate::widgets::builtin_types();
    let mut plugins = std::collections::HashMap::new();
    for (id, path) in crate::widgets::plugins::discover(&crate::widgets::plugins::dirs(base_dir)) {
        let broken = |error: String| WidgetType {
            id: id.clone(),
            name: id.clone(),
            description: String::new(),
            source: WidgetSource::Plugin(path.clone()),
            anchor: Anchor::TopRight,
            toggleable: false,
            error: Some(error),
        };
        if !crate::widgets::plugins::valid_id(&id) {
            catalog.push(broken("nom invalide : minuscules, chiffres, - et _ seulement".into()));
            continue;
        }
        if crate::widgets::builtin(&id).is_some() {
            catalog.push(broken(format!("« {id} » est le nom d'un widget intégré")));
            continue;
        }
        match load_plugin(&lua, &id, &path) {
            Ok((p, wt)) => {
                plugins.insert(id.clone(), p);
                catalog.push(wt);
            }
            Err(e) => {
                tracing::warn!("plugin {} : {e}", path.display());
                catalog.push(broken(e));
            }
        }
    }
    let mut config = Config::default();
    config.catalog = catalog;
    lua.set_app_data(RefCell::new(Builder {
        config,
        funcs: Vec::new(),
        base_dir: base_dir.to_path_buf(),
        origin: Origin::User,
        plugins,
        places: Vec::new(),
    }));
    for (src, name, origin) in chunks {
        if let Some(b) = lua.app_data_ref::<RefCell<Builder>>() {
            b.borrow_mut().origin = *origin;
        }
        lua.load(src.as_str()).set_name(format!("@{name}")).exec().map_err(|e| short_lua_error(&e))?;
    }
    let mut b = lua.remove_app_data::<RefCell<Builder>>().expect("builder posé plus haut").into_inner();
    apply_places(&mut b);
    Ok(Loaded { lua, config: b.config, funcs: b.funcs })
}

/// Module écrit par l'écran Réglages, à côté de la config de l'utilisateur.
pub fn settings_path(config_path: &Path) -> PathBuf {
    config_path.parent().unwrap_or(Path::new(".")).join("hyprdesk").join("settings.lua")
}

/// Charge `hyprdesk/settings.lua` (s'il existe) PUIS la config de l'utilisateur :
/// le Lua écrit à la main passe après les réglages faits à la souris.
pub fn load_file(path: &Path) -> Result<Loaded, String> {
    let base = path.parent().unwrap_or(Path::new("."));
    let mut chunks = Vec::new();
    let sp = settings_path(path);
    if let Ok(src) = std::fs::read_to_string(&sp) {
        chunks.push((src, sp.display().to_string(), Origin::Settings));
    }
    let src = std::fs::read_to_string(path).map_err(|e| format!("lecture de {} : {e}", path.display()))?;
    chunks.push((src, path.display().to_string(), Origin::User));
    load_chunks(&chunks, base)
}

/// Charge SEULEMENT `settings.lua`, pour que l'écran Réglages relise ses propres valeurs.
pub fn load_settings_only(config_path: &Path) -> Result<Config, String> {
    let sp = settings_path(config_path);
    let Ok(src) = std::fs::read_to_string(&sp) else { return Ok(Config::default()) };
    let base = config_path.parent().unwrap_or(Path::new("."));
    load_chunks(&[(src, sp.display().to_string(), Origin::Settings)], base).map(|l| l.config)
}

// ---------------------------------------------------------------------------
// Thread Lua

enum Req {
    Load { path: PathBuf, reply: oneshot::Sender<Result<Config, String>> },
    Render { func: usize, reply: oneshot::Sender<Result<String, String>> },
    Hook { funcs: Vec<usize>, event: String, monitor: String, workspace: String },
}

/// Poignée clonable vers le thread qui possède l'état Lua.
#[derive(Clone, Debug)]
pub struct LuaHandle {
    tx: std::sync::mpsc::Sender<Req>,
}

impl LuaHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Req>();
        std::thread::Builder::new()
            .name("hyprdesk-lua".into())
            .spawn(move || {
                let mut current: Option<Loaded> = None;
                while let Ok(req) = rx.recv() {
                    match req {
                        Req::Load { path, reply } => match load_file(&path) {
                            Ok(l) => {
                                let cfg = l.config.clone();
                                current = Some(l);
                                let _ = reply.send(Ok(cfg));
                            }
                            // On garde l'état précédent : la dernière config valide reste active.
                            Err(e) => {
                                let _ = reply.send(Err(e));
                            }
                        },
                        Req::Render { func, reply } => {
                            let r = current.as_ref().ok_or_else(|| "aucune config chargée".to_string()).and_then(|l| l.render(func));
                            let _ = reply.send(r);
                        }
                        Req::Hook { funcs, event, monitor, workspace } => {
                            if let Some(l) = &current {
                                for f in funcs {
                                    if let Err(e) = l.hook(f, &event, &monitor, &workspace) {
                                        tracing::warn!("hook hd.on(\"{event}\") : {e}");
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .expect("lancement du thread Lua");
        LuaHandle { tx }
    }

    pub async fn load(&self, path: PathBuf) -> Result<Config, String> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(Req::Load { path, reply }).map_err(|_| "thread Lua arrêté".to_string())?;
        rx.await.map_err(|_| "thread Lua arrêté".to_string())?
    }

    pub async fn render(&self, func: usize) -> Result<String, String> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(Req::Render { func, reply }).map_err(|_| "thread Lua arrêté".to_string())?;
        rx.await.map_err(|_| "thread Lua arrêté".to_string())?
    }

    pub fn hook(&self, funcs: Vec<usize>, event: &str, monitor: &str, workspace: &str) {
        if funcs.is_empty() {
            return;
        }
        let _ = self.tx.send(Req::Hook { funcs, event: event.into(), monitor: monitor.into(), workspace: workspace.into() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(src: &str) -> Result<Loaded, String> {
        load_source(src, "test.lua", Path::new("/cfg"))
    }

    #[test]
    fn config_complete() {
        let l = load(
            r##"
            hd.config({ mode = "glass", fade_ms = 200, show_on = "focused", grid = { cell = 80 },
                        theme = { accent = "rgb(ff80bf)" } })
            hd.wallpaper({ path = "~/w/a.jpg" })
            hd.wallpaper({ workspace = { 1, "2" }, monitor = "DP-1", rotate = { dir = "rot", interval = 30, shuffle = true }, fit = "contain" })
            hd.wallpaper({ workspace = "5", color = "#000000" })
            hd.desktop({ workspace = 4, mode = "opaque", inherit = false })
            hd.icon({ app = "firefox.desktop" })
            hd.icon({ workspace = "2", label = "Projets", path = "/tmp", pos = { 0, 1 } })
            hd.icon({ label = "htop", exec = "htop", terminal = true })
            hd.widget("clock", { format = "%H:%M:%S", anchor = "top-left", size = 64 })
            hd.widget("system", { show = { "cpu", "disk:/home" } })
            hd.widget("lua", { render = function() return "salut " .. hd.version end })
            hd.on("show", function(e) end)
            "##,
        )
        .unwrap();
        let c = &l.config;
        assert_eq!(c.general.mode, Mode::Glass);
        assert_eq!(c.general.fade_ms, 200);
        assert_eq!(c.general.show_on, ShowOn::Focused);
        assert_eq!(c.general.grid.cell, 80);
        assert_eq!(c.general.theme.accent, Rgba(0xff, 0x80, 0xbf, 0xff));
        assert_eq!(c.wallpapers.len(), 3);
        assert_eq!(c.wallpapers[0].source, WallSource::Path(home().join("w/a.jpg")));
        assert_eq!(c.wallpapers[1].sel.workspaces, Some(vec!["1".into(), "2".into()]));
        assert!(matches!(&c.wallpapers[1].source, WallSource::Rotate { dir, shuffle: true, .. } if dir == Path::new("/cfg/rot")));
        assert_eq!(c.wallpapers[1].fit, Fit::Contain);
        assert_eq!(c.desktops[0].sel.workspaces, Some(vec!["4".into()]));
        assert!(!c.desktops[0].inherit);
        assert_eq!(c.icons[0].kind, IconKind::App("firefox".into()));
        assert_eq!(c.icons[1].pos, Some((0, 1)));
        assert_eq!(c.icons[2].kind, IconKind::Exec { cmd: "htop".into(), terminal: true });
        assert_eq!(c.widgets.len(), 3);
        assert_eq!(c.widgets[0].anchor, Anchor::TopLeft);
        assert!(matches!(c.widgets[1].kind, WidgetKind::System { ref show } if show[1] == SysItem::Disk("/home".into())));
        let WidgetKind::Lua { func } = c.widgets[2].kind else { panic!() };
        assert_eq!(l.render(func).unwrap(), format!("salut {}", env!("CARGO_PKG_VERSION")));
        assert_eq!(c.hooks.len(), 1);
    }

    #[test]
    fn champ_inconnu_refuse() {
        let e = load("\n\nhd.icon({ app = \"x\", lable = \"faute\" })").err().unwrap();
        assert!(e.contains("champ inconnu « lable »"), "{e}");
        // La position désigne la ligne de l'appel dans la config.
        assert!(e.contains("test.lua:3:"), "{e}");
        assert!(!e.contains("runtime error"), "{e}");
    }

    #[test]
    fn erreur_de_syntaxe_lisible() {
        let e = load("hd.config({ mode = ").err().unwrap();
        assert!(e.contains("test.lua"), "{e}");
    }

    #[test]
    fn sources_de_fond_exclusives() {
        let e = load(r##"hd.wallpaper({ path = "a", color = "#000000" })"##).err().unwrap();
        assert!(e.contains("exactement un"), "{e}");
    }

    #[test]
    fn etoile_signifie_tous() {
        let l = load(r#"hd.icon({ workspace = "*", app = "a" })"#).unwrap();
        assert_eq!(l.config.icons[0].sel.workspaces, None);
    }

    #[test]
    fn require_depuis_le_dossier_hyprdesk() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("hyprdesk")).unwrap();
        std::fs::write(dir.join("hyprdesk/icones.lua"), r#"hd.icon({ app = "kitty" })"#).unwrap();
        let l = load_source(r#"require("icones")"#, "t.lua", &dir).unwrap();
        assert_eq!(l.config.icons[0].kind, IconKind::App("kitty".into()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reglages_puis_utilisateur() {
        let chunks = vec![
            (r##"hd.config({ show_on = "focused", grid = { icon_size = 64 } }) hd.wallpaper({ color = "#111111" }) hd.icon({ app = "a" })"##.to_string(), "settings.lua".to_string(), Origin::Settings),
            (r##"hd.config({ grid = { icon_size = 32 } }) hd.wallpaper({ color = "#222222" })"##.to_string(), "user.lua".to_string(), Origin::User),
        ];
        let c = load_chunks(&chunks, Path::new("/cfg")).unwrap().config;
        assert_eq!(c.general.show_on, ShowOn::Focused);
        assert_eq!(c.general.grid.icon_size, 32);
        assert!(c.keys_settings.contains("grid.icon_size") && c.keys_user.contains("grid.icon_size"));
        assert!(!c.keys_user.contains("show_on"));
        assert_eq!(c.wallpapers[0].origin, Origin::Settings);
        assert_eq!(c.wallpapers[1].origin, Origin::User);
        assert_eq!(c.icons[0].origin, Origin::Settings);
    }

    #[test]
    fn plugin_de_widget_et_placement() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-plugin-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("hyprdesk/widgets")).unwrap();
        std::fs::write(
            dir.join("hyprdesk/widgets/meteo.lua"),
            r#"return { name = "Météo", interval = 900, anchor = "top-left", options = { ville = "Paris" },
                        render = function(o) return "il fait beau à " .. o.ville end }"#,
        )
        .unwrap();
        std::fs::write(dir.join("hyprdesk/widgets/casse.lua"), "return 42").unwrap();
        let chunks = vec![
            (r#"hd.widget("meteo", { id = "m" }) hd.place("clock#0", { anchor = "center", y = 10 })"#.to_string(), "s.lua".to_string(), Origin::Settings),
            (r#"hd.widget("meteo", { ville = "Lyon" }) hd.widget("clock")"#.to_string(), "u.lua".to_string(), Origin::User),
        ];
        let l = load_chunks(&chunks, &dir).unwrap();
        let c = &l.config;
        let ids: Vec<&str> = c.catalog.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"clock") && ids.contains(&"meteo") && ids.contains(&"casse"));
        assert!(c.catalog.iter().find(|t| t.id == "casse").unwrap().error.is_some());
        assert_eq!(c.catalog.iter().find(|t| t.id == "meteo").unwrap().name, "Météo");
        let render = |i: usize| match c.widgets[i].kind {
            WidgetKind::Lua { func } => l.render(func).unwrap(),
            _ => panic!(),
        };
        assert_eq!(render(0), "il fait beau à Paris");
        assert_eq!(render(1), "il fait beau à Lyon");
        assert_eq!(c.widgets[1].id, "meteo#0");
        assert_eq!(c.widgets[1].anchor, Anchor::TopLeft);
        assert_eq!(c.widgets[1].interval, Duration::from_secs(900));
        // hd.place des réglages s'applique au widget de hyprdesk.lua.
        assert_eq!(c.widgets[2].id, "clock#0");
        assert_eq!((c.widgets[2].anchor, c.widgets[2].y), (Anchor::Center, 10.0));
        assert_eq!(c.placements.get("clock#0").map(|p| p.anchor), Some(Anchor::Center));
        let e = load_chunks(&[(r#"hd.widget("meteo", { pays = "FR" })"#.into(), "x.lua".into(), Origin::User)], &dir).err().unwrap();
        assert!(e.contains("champ inconnu « pays »"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn widget_inconnu() {
        let e = load(r#"hd.widget("meteo", {})"#).err().unwrap();
        assert!(e.contains("type « meteo » inconnu"), "{e}");
    }
}
