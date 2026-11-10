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
}

/// Clés présentes dans une table `hd.config`, en notation pointée (`grid.icon_size`).
fn present_keys(t: &Table) -> mlua::Result<Vec<String>> {
    let mut out = Vec::new();
    for pair in t.pairs::<String, Value>() {
        let (k, v) = pair?;
        match v {
            Value::Table(sub) if k == "grid" || k == "theme" => {
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
            "mode", "fade_ms", "transition", "crossfade_ms", "running_apps", "show_on", "close_on_launch", "icon_theme",
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
    let common = ["workspace", "monitor", "id", "anchor", "x", "y", "size", "interval", "card"];
    let extra: &[&str] = match kind {
        "clock" => &["format"],
        "calendar" => &[],
        "system" => &["show"],
        "media" => &["player"],
        "command" => &["exec", "timeout"],
        "lua" => &["render"],
        _ => return Err(err(format!("hd.widget : type « {kind} » inconnu (clock, calendar, system, media, command, lua)"))),
    };
    let allowed: Vec<&str> = common.iter().chain(extra).copied().collect();
    check_keys(t, &allowed, ctx)?;

    let (default_interval, kind_v) = match kind {
        "clock" => (1.0, WidgetKind::Clock { format: opt_string(t, "format", ctx)?.unwrap_or_else(|| "%H:%M".into()) }),
        "calendar" => (60.0, WidgetKind::Calendar),
        "system" => {
            let show = match t.get::<Option<Table>>("show")? {
                None => vec![SysItem::Cpu, SysItem::Ram, SysItem::Disk("/".into())],
                Some(list) => list
                    .sequence_values::<String>()
                    .map(|s| {
                        let s = s?;
                        SysItem::parse(&s).ok_or_else(|| err(format!("{ctx} : élément « {s} » inconnu (cpu, ram, load, uptime, disk:<point de montage>)")))
                    })
                    .collect::<mlua::Result<Vec<_>>>()?,
            };
            (2.0, WidgetKind::System { show })
        }
        "media" => (1.0, WidgetKind::Media { player: opt_string(t, "player", ctx)?.unwrap_or_else(|| "*".into()) }),
        "command" => {
            let exec = opt_string(t, "exec", ctx)?.ok_or_else(|| err(format!("{ctx} : « exec » est requis")))?;
            let timeout = Duration::from_secs_f64(opt_num(t, "timeout", ctx)?.unwrap_or(10.0).clamp(0.1, 600.0));
            (60.0, WidgetKind::Command { exec, timeout })
        }
        "lua" => {
            let f: Function = match t.get::<Value>("render")? {
                Value::Function(f) => f,
                _ => return Err(err(format!("{ctx} : « render » doit être une fonction"))),
            };
            b.funcs.push(lua.create_registry_value(f)?);
            (5.0, WidgetKind::Lua { func: b.funcs.len() - 1 })
        }
        _ => unreachable!(),
    };
    let anchor = match opt_string(t, "anchor", ctx)? {
        None => Anchor::TopRight,
        Some(a) => Anchor::parse(&a).ok_or_else(|| {
            err(format!("{ctx} : anchor « {a} » inconnu (top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right)"))
        })?,
    };
    let interval = opt_num(t, "interval", ctx)?.unwrap_or(default_interval);
    if interval < 0.1 {
        return Err(err(format!("{ctx} : « interval » doit valoir au moins 0.1 seconde")));
    }
    let n = b.config.widgets.len();
    b.config.widgets.push(WidgetDef {
        sel: selector(t, ctx)?,
        id: opt_string(t, "id", ctx)?.unwrap_or_else(|| format!("{kind}#{n}")),
        kind: kind_v,
        anchor,
        x: opt_num(t, "x", ctx)?.unwrap_or(0.0) as f32,
        y: opt_num(t, "y", ctx)?.unwrap_or(0.0) as f32,
        size: opt_num(t, "size", ctx)?.map(|v| v.clamp(6.0, 400.0) as f32).unwrap_or(0.0),
        interval: Duration::from_secs_f64(interval),
        card: opt_bool(t, "card", ctx)?.unwrap_or(true),
    });
    Ok(())
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
    hd.set("exec", lua.create_function(|_, cmd: String| Ok(shell_output(&cmd)))?)?;
    hd.set("version", env!("CARGO_PKG_VERSION"))?;
    lua.globals().set("hd", hd)?;
    // Les fonctions Rust ne connaissent pas la ligne appelante : chaque `hd.x`
    // relance l'erreur au niveau 2, pour qu'elle désigne `fichier:ligne` de
    // l'appel dans la config, comme une erreur Lua ordinaire.
    lua.load(
        r#"
        for _, name in ipairs({ "config", "wallpaper", "desktop", "icon", "widget", "on" }) do
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
    lua.set_app_data(RefCell::new(Builder {
        config: Config::default(),
        funcs: Vec::new(),
        base_dir: base_dir.to_path_buf(),
        origin: Origin::User,
    }));
    for (src, name, origin) in chunks {
        if let Some(b) = lua.app_data_ref::<RefCell<Builder>>() {
            b.borrow_mut().origin = *origin;
        }
        lua.load(src.as_str()).set_name(format!("@{name}")).exec().map_err(|e| short_lua_error(&e))?;
    }
    let b = lua.remove_app_data::<RefCell<Builder>>().expect("builder posé plus haut").into_inner();
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
    fn widget_inconnu() {
        let e = load(r#"hd.widget("meteo", {})"#).err().unwrap();
        assert!(e.contains("type « meteo » inconnu"), "{e}");
    }
}
