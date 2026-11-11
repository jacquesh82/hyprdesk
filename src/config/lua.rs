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
    /// `hd.disable` : widgets à retirer une fois tout déclaré.
    disabled: Vec<(Origin, String)>,
}

/// Plugin de widget chargé : sa table Lua et ses valeurs par défaut.
struct Plugin {
    table: RegistryKey,
    interval: f64,
    anchor: Anchor,
    options: Vec<String>,
}

/// Facultatif : une fonction Lua nommée `key`, ou erreur si autre chose.
fn opt_fn(t: &Table, key: &str, ctx: &str) -> mlua::Result<Option<Function>> {
    match t.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Function(f) => Ok(Some(f)),
        v => Err(err(crate::tr!(
            "{ctx} : « {key} » doit être une fonction, pas {p0}",
            ctx = ctx,
            key = key,
            p0 = v.type_name()
        ))),
    }
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
        err(crate::tr!("{ctx} : anchor « {s} » inconnu (top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right)", ctx = ctx, s = s))
    })
}

/// Charge un plugin de widget : un fichier Lua qui rend une table avec `render`.
fn load_plugin(
    lua: &Lua,
    id: &str,
    source: &crate::widgets::plugins::Source,
) -> Result<(Plugin, WidgetType), String> {
    use crate::widgets::plugins::Source;
    let (src, name, wsource) = match source {
        Source::File(p) => (
            std::fs::read_to_string(p).map_err(|e| e.to_string())?,
            p.display().to_string(),
            WidgetSource::Plugin(p.clone()),
        ),
        Source::Bundled(s) => (
            s.to_string(),
            format!("intégré:{id}.lua"),
            WidgetSource::Bundled,
        ),
    };
    reset_budget(lua).map_err(|e| e.to_string())?;
    let v: Value = lua
        .load(src.as_str())
        .set_name(format!("@{name}"))
        .eval()
        .map_err(|e| short_lua_error(&e))?;
    let Value::Table(t) = v else {
        return Err(crate::i18n::tr("le fichier doit rendre une table (return { … })").into());
    };
    let ctx = crate::tr!("plugin « {id} »", id = id);
    let run = || -> mlua::Result<(Plugin, WidgetType)> {
        check_keys(
            &t,
            &[
                "name",
                "description",
                "interval",
                "anchor",
                "options",
                "render",
                "on_click",
                "on_scroll",
                "menu",
            ],
            &ctx,
        )?;
        for k in ["on_click", "on_scroll", "menu"] {
            opt_fn(&t, k, &ctx)?;
        }
        if !matches!(t.get::<Value>("render")?, Value::Function(_)) {
            return Err(err(crate::tr!(
                "{ctx} : « render » doit être une fonction",
                ctx = ctx
            )));
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
                    return Err(err(crate::tr!(
                        "{ctx} : l'option « {k} » est réservée",
                        ctx = ctx,
                        k = k
                    )));
                }
                options.push(k);
            }
        }
        options.sort();
        let wt = WidgetType {
            id: id.to_string(),
            name: opt_string(&t, "name", &ctx)?.unwrap_or_else(|| id.to_string()),
            description: opt_string(&t, "description", &ctx)?.unwrap_or_default(),
            source: wsource,
            anchor,
            toggleable: true,
            error: None,
        };
        let interval = opt_num(&t, "interval", &ctx)?.unwrap_or(60.0).max(0.1);
        Ok((
            Plugin {
                table: lua.create_registry_value(t.clone())?,
                interval,
                anchor,
                options,
            },
            wt,
        ))
    };
    run().map_err(|e| short_lua_error(&e))
}

/// Clés présentes dans une table `hd.config`, en notation pointée (`grid.icon_size`).
fn present_keys(t: &Table) -> mlua::Result<Vec<String>> {
    let mut out = Vec::new();
    for pair in t.pairs::<String, Value>() {
        let (k, v) = pair?;
        match v {
            Value::Table(sub) if k == "grid" || k == "theme" || k == "keys" || k == "apps" => {
                for p in sub.pairs::<String, Value>() {
                    let (sk, sv) = p?;
                    // Effets du thème : un endroit à la fois (`theme.effects.bar`).
                    match sv {
                        Value::Table(eff) if k == "theme" && sk == "effects" => {
                            for e in eff.pairs::<String, Value>() {
                                out.push(format!("theme.effects.{}", e?.0));
                            }
                        }
                        _ => out.push(format!("{k}.{sk}")),
                    }
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
/// Layouts : `layout`, `layout_workspaces`, et les options de `dwindle`,
/// `master`, `scrolling` (vérifiées d'après `tiling::OPTIONS`).
fn parse_tiling(t: &Table, tiling: &mut crate::tiling::Tiling) -> mlua::Result<()> {
    const C: &str = "hd.config";
    let check = |name: &str| -> mlua::Result<()> {
        if crate::tiling::valid_name(name) {
            Ok(())
        } else {
            Err(err(crate::tr!(
                "{C} : layout « {name} » inconnu (dwindle, master, scrolling, monocle, grid, columns, rows, centered, lua:<nom>)",
                C = C,
                name = name
            )))
        }
    };
    if let Some(s) = opt_string(t, "layout", C)? {
        check(&s)?;
        tiling.layout = Some(s);
    }
    if let Some(map) = t.get::<Option<Table>>("layout_workspaces")? {
        tiling.workspaces.clear();
        for pair in map.pairs::<Value, Value>() {
            let (k, v) = pair?;
            let ws = match k {
                Value::Integer(i) => i.to_string(),
                Value::String(s) => s.to_str()?.to_string(),
                _ => {
                    return Err(err(crate::tr!(
                        "{C} : layout_workspaces attend des workspaces (\"3\", 3, \"name:web\")",
                        C = C
                    )));
                }
            };
            let Value::String(name) = v else {
                return Err(err(crate::tr!(
                    "{C} : layout du workspace {ws} : un nom de layout est attendu",
                    C = C,
                    ws = ws
                )));
            };
            let name = name.to_str()?.to_string();
            check(&name)?;
            tiling.workspaces.insert(ws, name);
        }
    }
    for section in crate::tiling::SECTIONS {
        let Some(opts) = t.get::<Option<Table>>(*section)? else {
            continue;
        };
        tiling
            .options
            .retain(|k, _| !k.starts_with(&format!("{section}.")));
        for pair in opts.pairs::<String, Value>() {
            let (k, v) = pair?;
            let path = format!("{section}.{k}");
            let Some(o) = crate::tiling::opt(&path) else {
                let known: Vec<&str> = crate::tiling::OPTIONS
                    .iter()
                    .filter(|o| o.section == *section)
                    .map(|o| o.key)
                    .collect();
                return Err(err(crate::tr!(
                    "{C} : option « {path} » inconnue (connues : {known})",
                    C = C,
                    path = path,
                    known = known.join(", ")
                )));
            };
            let raw = match v {
                Value::Boolean(b) => if b { "on" } else { "off" }.to_string(),
                Value::Integer(i) => i.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.to_str()?.to_string(),
                other => other.type_name().to_string(),
            };
            let value = o
                .parse(&raw)
                .map_err(|e| err(format!("{C} : {path} — {e}")))?;
            tiling.options.insert(path, value);
        }
    }
    Ok(())
}

fn check_keys(t: &Table, allowed: &[&str], ctx: &str) -> mlua::Result<()> {
    for pair in t.pairs::<Value, Value>() {
        let (k, _) = pair?;
        let Value::String(k) = k else {
            return Err(err(crate::tr!(
                "{ctx} : les clés doivent être des chaînes",
                ctx = ctx
            )));
        };
        let k = k.to_str()?.to_string();
        if !allowed.contains(&k.as_str()) {
            return Err(err(crate::tr!(
                "{ctx} : champ inconnu « {k} » (attendus : {p0})",
                ctx = ctx,
                k = k,
                p0 = allowed.join(", ")
            )));
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
        v => Err(err(crate::tr!(
            "{ctx} : « {key} » doit être une chaîne, pas {p0}",
            ctx = ctx,
            key = key,
            p0 = v.type_name()
        ))),
    }
}

fn opt_bool(t: &Table, key: &str, ctx: &str) -> mlua::Result<Option<bool>> {
    match t.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Boolean(b) => Ok(Some(b)),
        v => Err(err(crate::tr!(
            "{ctx} : « {key} » doit être un booléen, pas {p0}",
            ctx = ctx,
            key = key,
            p0 = v.type_name()
        ))),
    }
}

fn opt_num(t: &Table, key: &str, ctx: &str) -> mlua::Result<Option<f64>> {
    match t.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Integer(i) => Ok(Some(i as f64)),
        Value::Number(n) if n.is_finite() => Ok(Some(n)),
        Value::Number(_) => Err(err(crate::tr!(
            "{ctx} : « {key} » doit être un nombre fini",
            ctx = ctx,
            key = key
        ))),
        v => Err(err(crate::tr!(
            "{ctx} : « {key} » doit être un nombre, pas {p0}",
            ctx = ctx,
            key = key,
            p0 = v.type_name()
        ))),
    }
}

fn opt_color(t: &Table, key: &str, ctx: &str) -> mlua::Result<Option<Rgba>> {
    match opt_string(t, key, ctx)? {
        None => Ok(None),
        Some(s) => Rgba::parse(&s).map(Some).ok_or_else(|| {
            err(crate::tr!(
                "{ctx} : couleur « {s} » illisible (#rrggbb, #rrggbbaa, rgb(…), rgba(…))",
                ctx = ctx,
                s = s
            ))
        }),
    }
}

/// Une chaîne, un nombre, ou une liste des deux ; `"*"` signifie « tous ».
fn string_list(v: Value, key: &str, ctx: &str) -> mlua::Result<Option<Vec<String>>> {
    let one = |v: Value| -> mlua::Result<String> {
        match v {
            Value::String(s) => Ok(s.to_str()?.to_string()),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Number(n) if n.fract() == 0.0 => Ok((n as i64).to_string()),
            v => Err(err(crate::tr!(
                "{ctx} : « {key} » attend une chaîne, un nombre ou une liste, pas {p0}",
                ctx = ctx,
                key = key,
                p0 = v.type_name()
            ))),
        }
    };
    let list = match v {
        Value::Nil => return Ok(None),
        Value::Table(t) => t
            .sequence_values::<Value>()
            .map(|v| one(v?))
            .collect::<mlua::Result<Vec<_>>>()?,
        v => vec![one(v)?],
    };
    if list.iter().any(|s| s == "*") {
        return Ok(None);
    }
    if list.is_empty() {
        return Err(err(crate::tr!(
            "{ctx} : « {key} » est une liste vide",
            ctx = ctx,
            key = key
        )));
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
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn parse_mode(s: &str, ctx: &str) -> mlua::Result<Mode> {
    match s {
        "opaque" => Ok(Mode::Opaque),
        "glass" => Ok(Mode::Glass),
        _ => Err(err(crate::tr!(
            "{ctx} : mode « {s} » inconnu (opaque, glass)",
            ctx = ctx,
            s = s
        ))),
    }
}

fn hd_config(b: &mut Builder, t: &Table) -> mlua::Result<()> {
    const CTX: &str = "hd.config";
    check_keys(
        t,
        &[
            "keys",
            "suspend_binds",
            "clipboard_history",
            "autostart",
            "desktop_files",
            "float_launched",
            "mode",
            "fade_ms",
            "transition",
            "crossfade_ms",
            "running_apps",
            "launcher_group",
            "launcher_position",
            "launcher_size",
            "launcher_pinned",
            "launcher_show_settings",
            "launcher_show_assistant",
            "lock_blur",
            "lock_dim",
            "lock_image",
            "lock_show_user",
            "lock_show_battery",
            "lock_show_media",
            "idle_manage",
            "idle_dim",
            "idle_lock",
            "idle_dpms",
            "idle_suspend",
            "show_on",
            "close_on_launch",
            "advanced_settings",
            "tour_seen",
            "app_themes",
            "icon_theme",
            "font",
            "font_size",
            "terminal",
            "grid",
            "theme",
            "glass_opacity",
            "background",
            "workspaces",
            "workspace_monitors",
            "workspace_dedicated",
            "primary",
            "apps",
            "disabled_plugins",
            "assistant_mode",
            "fx",
            "language",
            "layout",
            "layout_workspaces",
            "dwindle",
            "master",
            "scrolling",
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
    if let Some(v) = opt_bool(t, "clipboard_history", CTX)? {
        g.clipboard_history = v;
    }
    if let Some(v) = opt_bool(t, "autostart", CTX)? {
        g.autostart = v;
    }
    if let Some(v) = opt_bool(t, "desktop_files", CTX)? {
        g.desktop_files = v;
    }
    if let Some(v) = opt_bool(t, "float_launched", CTX)? {
        g.float_launched = v;
    }
    if let Some(k) = t.get::<Option<Table>>("keys")? {
        const C: &str = "hd.config.keys";
        check_keys(
            &k,
            &[
                "toggle", "launcher", "hide", "settings", "lock", "leave", "media", "switch",
            ],
            C,
        )?;
        if let Some(v) = opt_bool(&k, "media", C)? {
            g.keys.media = v;
        }
        for (name, slot) in [
            ("toggle", &mut g.keys.toggle),
            ("launcher", &mut g.keys.launcher),
            ("hide", &mut g.keys.hide),
            ("settings", &mut g.keys.settings),
            ("lock", &mut g.keys.lock),
            ("leave", &mut g.keys.leave),
            ("switch", &mut g.keys.switch),
        ] {
            match k.get::<Value>(name)? {
                Value::Nil => {}
                Value::Boolean(false) => slot.clear(),
                Value::String(s) => {
                    let s = s.to_str()?.trim().to_string();
                    if !s.is_empty() && crate::hypr::parse_combo(&s).is_none() {
                        return Err(err(crate::tr!(
                            "{C} : raccourci « {s} » illisible (ex. \"SUPER + SHIFT + D\")",
                            C = C,
                            s = s
                        )));
                    }
                    *slot = s;
                }
                v => {
                    return Err(err(crate::tr!(
                        "{C} : « {name} » attend une chaîne ou false, pas {p0}",
                        C = C,
                        name = name,
                        p0 = v.type_name()
                    )));
                }
            }
        }
    }
    if let Some(s) = opt_string(t, "transition", CTX)? {
        g.transition = Transition::parse(&s).ok_or_else(|| {
            err(crate::tr!(
                "{CTX} : transition « {s} » inconnue (fade, slide, none)",
                CTX = CTX,
                s = s
            ))
        })?;
    }
    if let Some(v) = opt_num(t, "crossfade_ms", CTX)? {
        g.crossfade_ms = v.clamp(0.0, 10000.0) as u64;
    }
    if let Some(s) = opt_string(t, "running_apps", CTX)? {
        g.running_apps = RunningApps::parse(&s).ok_or_else(|| {
            err(crate::tr!(
                "{CTX} : running_apps « {s} » inconnu (off, workspace, all)",
                CTX = CTX,
                s = s
            ))
        })?;
    }
    if let Some(s) = opt_string(t, "launcher_position", CTX)? {
        g.launcher.position = LauncherPosition::parse(&s).ok_or_else(|| {
            err(crate::tr!(
                "{CTX} : launcher_position « {s} » inconnu (bottom, top, left, right)",
                CTX = CTX,
                s = s
            ))
        })?;
    }
    if let Some(v) = opt_num(t, "launcher_size", CTX)? {
        g.launcher.size = v.clamp(24.0, 96.0) as u16;
    }
    if let Some(l) = string_list(t.get("launcher_pinned")?, "launcher_pinned", CTX)? {
        g.launcher.pinned = l
            .into_iter()
            .map(|a| a.trim_end_matches(".desktop").to_string())
            .collect();
    }
    if let Some(v) = opt_bool(t, "lock_blur", CTX)? {
        g.lock.blur = v;
    }
    if let Some(v) = opt_num(t, "lock_dim", CTX)? {
        g.lock.dim = v.clamp(0.0, 0.9) as f32;
    }
    if let Some(v) = opt_string(t, "lock_image", CTX)? {
        g.lock.image = (!v.trim().is_empty()).then(|| expand_path(v.trim(), &b.base_dir));
    }
    if let Some(v) = opt_bool(t, "lock_show_user", CTX)? {
        g.lock.show_user = v;
    }
    if let Some(v) = opt_bool(t, "lock_show_battery", CTX)? {
        g.lock.show_battery = v;
    }
    if let Some(v) = opt_bool(t, "lock_show_media", CTX)? {
        g.lock.show_media = v;
    }
    if let Some(v) = opt_bool(t, "idle_manage", CTX)? {
        g.idle.manage = v;
    }
    for (key, slot) in [
        ("idle_dim", &mut g.idle.dim),
        ("idle_lock", &mut g.idle.lock),
        ("idle_dpms", &mut g.idle.dpms),
        ("idle_suspend", &mut g.idle.suspend),
    ] {
        if let Some(v) = opt_num(t, key, CTX)? {
            // Secondes, jusqu'à 24 h ; 0 coupe l'étape.
            *slot = v.clamp(0.0, 86_400.0) as u32;
        }
    }
    if let Some(v) = opt_bool(t, "launcher_show_settings", CTX)? {
        g.launcher.show_settings = v;
    }
    if let Some(v) = opt_bool(t, "launcher_show_assistant", CTX)? {
        g.launcher.show_assistant = v;
    }
    if let Some(s) = opt_string(t, "launcher_group", CTX)? {
        g.launcher_group = LauncherGroup::parse(&s).ok_or_else(|| {
            err(crate::tr!(
                "{CTX} : launcher_group « {s} » inconnu (none, app, workspace)",
                CTX = CTX,
                s = s
            ))
        })?;
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
            "primary" => ShowOn::Primary,
            _ => {
                return Err(err(crate::tr!(
                    "{CTX} : show_on « {s} » inconnu (all, focused, primary)",
                    CTX = CTX,
                    s = s
                )));
            }
        };
    }
    if let Some(v) = opt_bool(t, "close_on_launch", CTX)? {
        g.close_on_launch = v;
    }
    if let Some(v) = opt_bool(t, "advanced_settings", CTX)? {
        g.advanced_settings = v;
    }
    if let Some(v) = opt_bool(t, "tour_seen", CTX)? {
        g.tour_seen = v;
    }
    if let Some(list) = string_list(t.get("app_themes")?, "app_themes", CTX)? {
        for id in &list {
            if crate::apptheme::target(id).is_none() {
                let known: Vec<&str> = crate::apptheme::TARGETS.iter().map(|t| t.id).collect();
                return Err(err(crate::tr!(
                    "{CTX} : app_themes « {id} » inconnue (connues : {known})",
                    CTX = CTX,
                    id = id,
                    known = known.join(", ")
                )));
            }
        }
        g.app_themes = list.into_iter().collect();
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
    if let Some(s) = opt_string(t, "language", CTX)? {
        let l = s.trim().to_lowercase();
        if !["auto", "fr", "en"].contains(&l.as_str()) {
            return Err(err(crate::tr!(
                "{CTX} : language « {s} » inconnue (auto, fr, en)",
                CTX = CTX,
                s = s
            )));
        }
        g.language = l;
    }
    parse_tiling(t, &mut g.tiling)?;
    if let Some(fx) = t.get::<Option<Table>>("fx")? {
        g.fx = crate::fx::parse(&fx).map_err(|e| err(format!("{CTX} : {e}")))?;
    }
    if let Some(s) = opt_string(t, "assistant_mode", CTX)? {
        g.assistant_mode = crate::assistant::AssistantMode::parse(&s).ok_or_else(|| {
            err(crate::tr!(
                "{CTX} : assistant_mode « {s} » inconnu (window, terminal)",
                CTX = CTX,
                s = s
            ))
        })?;
    }
    if let Some(list) = string_list(t.get("disabled_plugins")?, "disabled_plugins", CTX)? {
        g.disabled_plugins = list.into_iter().collect();
    }
    if let Some(apps) = t.get::<Option<Table>>("apps")? {
        const C: &str = "hd.config.apps";
        let roles: Vec<&str> = crate::apps::ROLES.iter().map(|r| r.id).collect();
        check_keys(&apps, &roles, C)?;
        for r in &roles {
            if let Some(v) = opt_string(&apps, r, C)? {
                let v = v.trim().trim_end_matches(".desktop").to_string();
                if v.is_empty() {
                    continue;
                }
                // Le terminal choisi est aussi celui de hyprdesk (icônes `terminal = true`).
                if r.eq_ignore_ascii_case("terminal") {
                    g.terminal = crate::apps::terminal_command(&v);
                }
                g.apps.insert(r.to_string(), v);
            }
        }
    }
    match t.get::<Value>("primary")? {
        Value::Nil => {}
        Value::Boolean(false) => g.primary = None,
        Value::String(s) if !s.to_str()?.trim().is_empty() => {
            g.primary = Some(s.to_str()?.trim().to_string());
        }
        v => {
            return Err(err(crate::tr!(
                "{CTX} : « primary » attend un nom d'écran (\"DP-1\") ou false, pas {p0}",
                CTX = CTX,
                p0 = v.type_name()
            )));
        }
    }
    if let Some(l) = string_list(t.get("workspace_dedicated")?, "workspace_dedicated", CTX)? {
        g.workspace_dedicated = l.into_iter().collect();
    }
    if let Some(map) = t.get::<Option<Table>>("workspace_monitors")? {
        const C: &str = "hd.config.workspace_monitors";
        g.workspace_monitors_declared.clear();
        for pair in map.pairs::<Value, Value>() {
            let (k, v) = pair?;
            let Value::String(mon) = k else {
                return Err(err(crate::tr!(
                    "{C} : les clés sont des écrans (\"DP-1\", \"primary\")",
                    C = C
                )));
            };
            let mon = mon.to_str()?.to_string();
            let list = string_list(v, &mon, C)?.unwrap_or_default();
            g.workspace_monitors_declared.insert(mon, list);
        }
    }
    match t.get::<Value>("workspaces")? {
        Value::Nil => {}
        Value::Boolean(false) => g.workspaces = None,
        v => {
            let n = match v {
                Value::Integer(i) => i as f64,
                Value::Number(n) => n,
                _ => f64::NAN,
            };
            if !(1.0..=MAX_WORKSPACES as f64).contains(&n) || n.fract() != 0.0 {
                return Err(err(crate::tr!(
                    "{CTX} : « workspaces » attend un entier de 1 à {MAX_WORKSPACES}, ou false",
                    CTX = CTX,
                    MAX_WORKSPACES = MAX_WORKSPACES
                )));
            }
            g.workspaces = Some(n as u8);
        }
    }
    if let Some(grid) = t.get::<Option<Table>>("grid")? {
        const C: &str = "hd.config.grid";
        check_keys(&grid, &["cell", "gap", "margin", "icon_size"], C)?;
        let px = |k: &str| -> mlua::Result<Option<u16>> {
            Ok(opt_num(&grid, k, C)?.map(|v| v.clamp(0.0, 1024.0) as u16))
        };
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
        check_keys(&th, &["name", "fg", "bg", "accent", "effects"], C)?;
        if let Some(v) = opt_string(&th, "name", C)? {
            g.theme.name = v;
        }
        if let Some(fx) = th.get::<Option<Table>>("effects")? {
            const E: &str = "hd.config.theme.effects";
            let places: Vec<&str> = Place::ALL.iter().map(|p| p.as_str()).collect();
            check_keys(&fx, &places, E)?;
            for place in Place::ALL {
                let Some(e) = fx.get::<Option<Table>>(place.as_str())? else {
                    continue;
                };
                let ctx = format!("{E}.{}", place.as_str());
                check_keys(&e, &["opacity", "blur"], &ctx)?;
                let mut cur = g.theme.effect(place);
                if let Some(v) = opt_num(&e, "opacity", &ctx)? {
                    cur.opacity = v.clamp(0.0, 1.0) as f32;
                }
                if let Some(v) = opt_bool(&e, "blur", &ctx)? {
                    cur.blur = v;
                }
                g.theme.effects.insert(place, cur);
            }
        }
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
    check_keys(
        t,
        &[
            "workspace",
            "monitor",
            "path",
            "rotate",
            "color",
            "fit",
            "span",
        ],
        CTX,
    )?;
    let sel = selector(t, CTX)?;
    let fit = match opt_string(t, "fit", CTX)?.as_deref() {
        None | Some("cover") => Fit::Cover,
        Some("contain") => Fit::Contain,
        Some("fill") => Fit::Fill,
        Some("center") => Fit::Center,
        Some(s) => {
            return Err(err(crate::tr!(
                "{CTX} : fit « {s} » inconnu (cover, contain, fill, center)",
                CTX = CTX,
                s = s
            )));
        }
    };
    let path = opt_string(t, "path", CTX)?;
    let rotate = t.get::<Option<Table>>("rotate")?;
    let color = opt_color(t, "color", CTX)?;
    let source = match (path, rotate, color) {
        (Some(p), None, None) => WallSource::Path(expand_path(&p, &b.base_dir)),
        (None, Some(r), None) => {
            const C: &str = "hd.wallpaper.rotate";
            check_keys(&r, &["dir", "interval", "shuffle"], C)?;
            let dir = opt_string(&r, "dir", C)?
                .ok_or_else(|| err(crate::tr!("{C} : « dir » est requis", C = C)))?;
            let secs = opt_num(&r, "interval", C)?.unwrap_or(600.0);
            if !(1.0..=31_536_000.0).contains(&secs) {
                return Err(err(crate::tr!(
                    "{C} : « interval » doit valoir au moins 1 seconde",
                    C = C
                )));
            }
            WallSource::Rotate {
                dir: expand_path(&dir, &b.base_dir),
                interval: Duration::from_secs_f64(secs),
                shuffle: opt_bool(&r, "shuffle", C)?.unwrap_or(false),
            }
        }
        (None, None, Some(c)) => WallSource::Color(c),
        _ => {
            return Err(err(crate::tr!(
                "{CTX} : indiquer exactement un de « path », « rotate » ou « color »",
                CTX = CTX
            )));
        }
    };
    // span = true : bureau et verrouillage ; "lock" : verrouillage seulement ;
    // "desktop" : bureau seulement.
    let (span, span_lock) = match t.get::<Value>("span")? {
        Value::Nil | Value::Boolean(false) => (false, false),
        Value::Boolean(true) => (true, true),
        Value::String(s) => match s.to_str()?.as_ref() {
            "lock" => (false, true),
            "desktop" => (true, false),
            other => {
                return Err(err(crate::tr!(
                    "{CTX} : span « {other} » inconnu (true, false, \"lock\", \"desktop\")",
                    CTX = CTX,
                    other = other
                )));
            }
        },
        other => {
            return Err(err(crate::tr!(
                "{CTX} : « span » attend true, false, \"lock\" ou \"desktop\", pas {p0}",
                CTX = CTX,
                p0 = other.type_name()
            )));
        }
    };
    if (span || span_lock) && matches!(source, WallSource::Color(_)) {
        return Err(err(crate::tr!(
            "{CTX} : « span » s'applique à une image ou un diaporama, pas à une couleur",
            CTX = CTX
        )));
    }
    b.config.wallpapers.push(WallpaperRule {
        origin: b.origin,
        sel,
        source,
        fit,
        span,
        span_lock,
    });
    Ok(())
}

fn hd_desktop(b: &mut Builder, t: &Table) -> mlua::Result<()> {
    const CTX: &str = "hd.desktop";
    check_keys(t, &["workspace", "monitor", "mode", "inherit"], CTX)?;
    let sel = selector(t, CTX)?;
    let mode = opt_string(t, "mode", CTX)?
        .map(|m| parse_mode(&m, CTX))
        .transpose()?;
    let inherit = opt_bool(t, "inherit", CTX)?.unwrap_or(true);
    b.config.desktops.push(DesktopRule {
        origin: b.origin,
        sel,
        mode,
        inherit,
    });
    Ok(())
}

fn hd_assign(b: &mut Builder, t: &Table) -> mlua::Result<()> {
    const CTX: &str = "hd.assign";
    check_keys(t, &["app", "class", "workspace", "silent"], CTX)?;
    let app = match (opt_string(t, "app", CTX)?, opt_string(t, "class", CTX)?) {
        (Some(a), None) if !a.trim().is_empty() => {
            AppMatch::App(a.trim().trim_end_matches(".desktop").to_string())
        }
        (None, Some(c)) if !c.trim().is_empty() => AppMatch::Class(c.trim().to_string()),
        _ => {
            return Err(err(crate::tr!(
                "{CTX} : indiquer exactement un de « app » (id .desktop) ou « class » (expression régulière)",
                CTX = CTX
            )));
        }
    };
    let workspace = opt_string(t, "workspace", CTX)?
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty())
        .ok_or_else(|| err(crate::tr!("{CTX} : « workspace » est requis", CTX = CTX)))?;
    let silent = opt_bool(t, "silent", CTX)?.unwrap_or(false);
    // Une seule affectation par application : la dernière déclarée l'emporte.
    b.config.assigns.retain(|a| a.app != app);
    b.config.assigns.push(AssignRule {
        origin: b.origin,
        app,
        workspace,
        silent,
    });
    Ok(())
}

fn hd_icon(b: &mut Builder, t: &Table) -> mlua::Result<()> {
    const CTX: &str = "hd.icon";
    check_keys(
        t,
        &[
            "workspace",
            "monitor",
            "id",
            "label",
            "app",
            "path",
            "exec",
            "terminal",
            "icon",
            "pos",
        ],
        CTX,
    )?;
    let sel = selector(t, CTX)?;
    let app = opt_string(t, "app", CTX)?;
    let path = opt_string(t, "path", CTX)?;
    let exec = opt_string(t, "exec", CTX)?;
    let terminal = opt_bool(t, "terminal", CTX)?.unwrap_or(false);
    let label = opt_string(t, "label", CTX)?;
    let (kind, default_id) = match (app, path, exec) {
        (Some(a), None, None) => (
            IconKind::App(a.trim_end_matches(".desktop").to_string()),
            format!("app:{a}"),
        ),
        (None, Some(p), None) => (
            IconKind::Path(expand_path(&p, &b.base_dir)),
            format!("path:{p}"),
        ),
        (None, None, Some(c)) => {
            let id = format!("exec:{}", label.as_deref().unwrap_or(&c));
            (IconKind::Exec { cmd: c, terminal }, id)
        }
        _ => {
            return Err(err(crate::tr!(
                "{CTX} : indiquer exactement un de « app », « path » ou « exec »",
                CTX = CTX
            )));
        }
    };
    let pos = match t.get::<Option<Table>>("pos")? {
        None => None,
        Some(p) => {
            let col: f64 = p.get(1)?;
            let row: f64 = p.get(2)?;
            if col < 0.0 || row < 0.0 {
                return Err(err(crate::tr!(
                    "{CTX} : « pos » attend {colonne, ligne} positifs, à partir de 0",
                    CTX = CTX
                )));
            }
            Some((col as u16, row as u16))
        }
    };
    let id = opt_string(t, "id", CTX)?.unwrap_or(default_id);
    if b.config.icons.iter().any(|i| i.id == id && i.sel == sel) {
        return Err(err(crate::tr!(
            "{CTX} : icône « {id} » déclarée deux fois pour le même bureau (préciser « id »)",
            CTX = CTX,
            id = id
        )));
    }
    b.config.icons.push(IconDef {
        origin: b.origin,
        sel,
        id,
        label,
        kind,
        icon: opt_string(t, "icon", CTX)?,
        pos,
    });
    Ok(())
}

fn hd_widget(lua: &Lua, b: &mut Builder, kind: &str, t: &Table, in_bar: bool) -> mlua::Result<()> {
    // Migration des copies de modules sérialisées par les anciennes versions.
    // La définition hd.bar reste la seule source de ces identifiants internes.
    if !in_bar
        && b.origin == Origin::Settings
        && opt_string(t, "id", "hd.widget")?.is_some_and(|id| id.starts_with("bar:"))
    {
        tracing::warn!(
            "{}",
            crate::tr!("ancienne copie d'un module de barre ignorée dans settings.lua")
        );
        return Ok(());
    }
    let ctx = format!("hd.widget(\"{kind}\")");
    let ctx = ctx.as_str();
    let common = crate::widgets::COMMON_OPTIONS;
    let (default_interval, default_anchor, kind_v) = if let Some(bi) = crate::widgets::builtin(kind)
    {
        let allowed: Vec<&str> = common.iter().chain(bi.options).copied().collect();
        check_keys(t, &allowed, ctx)?;
        let k = match kind {
            "clock" => WidgetKind::Clock {
                format: opt_string(t, "format", ctx)?.unwrap_or_else(|| "%H:%M".into()),
            },
            "calendar" => WidgetKind::Calendar,
            "system" => {
                let show = match t.get::<Option<Table>>("show")? {
                    None => vec![SysItem::Cpu, SysItem::Ram, SysItem::Disk("/".into())],
                    Some(list) => list
                        .sequence_values::<String>()
                        .map(|s| {
                            let s = s?;
                            SysItem::parse(&s)
                                .ok_or_else(|| err(crate::tr!("{ctx} : élément « {s} » inconnu (cpu, ram, load, uptime, disk:<point de montage>)", ctx = ctx, s = s)))
                        })
                        .collect::<mlua::Result<Vec<_>>>()?,
                };
                WidgetKind::System { show }
            }
            "media" => WidgetKind::Media {
                player: opt_string(t, "player", ctx)?.unwrap_or_else(|| "*".into()),
            },
            "command" => {
                let exec = opt_string(t, "exec", ctx)?
                    .ok_or_else(|| err(crate::tr!("{ctx} : « exec » est requis", ctx = ctx)))?;
                let timeout = Duration::from_secs_f64(
                    opt_num(t, "timeout", ctx)?
                        .unwrap_or(10.0)
                        .clamp(0.1, 600.0),
                );
                WidgetKind::Command { exec, timeout }
            }
            "lua" => {
                let f: Function = match t.get::<Value>("render")? {
                    Value::Function(f) => f,
                    _ => {
                        return Err(err(crate::tr!(
                            "{ctx} : « render » doit être une fonction",
                            ctx = ctx
                        )));
                    }
                };
                let mut reg = |f: Function| -> mlua::Result<usize> {
                    b.funcs.push(lua.create_registry_value(f)?);
                    Ok(b.funcs.len() - 1)
                };
                let func = reg(f)?;
                let click = opt_fn(t, "on_click", ctx)?.map(&mut reg).transpose()?;
                let scroll = opt_fn(t, "on_scroll", ctx)?.map(&mut reg).transpose()?;
                let menu = opt_fn(t, "menu", ctx)?.map(&mut reg).transpose()?;
                WidgetKind::Lua {
                    func,
                    click,
                    scroll,
                    menu,
                }
            }
            other => {
                return Err(err(crate::tr!(
                    "{ctx} : widget intégré « {other} » sans configuration",
                    ctx = ctx,
                    other = other
                )));
            }
        };
        (bi.interval, bi.anchor, k)
    } else if let Some(p) = b.plugins.get(kind) {
        // Plugin : ses options, avec ses valeurs par défaut, passées à render(opts).
        let allowed: Vec<&str> = common
            .iter()
            .copied()
            .chain(p.options.iter().map(String::as_str))
            .collect();
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
        // Chaque fonction du plugin reçoit ses options : render(o), on_click(o),
        // on_scroll(o, sens), menu(o).
        let bind = |f: Function| -> mlua::Result<Function> {
            lua.load("local f, o = ... return function(...) return f(o, ...) end")
                .set_name("=hyprdesk")
                .call((f, opts.clone()))
        };
        let mut reg = |f: Function| -> mlua::Result<usize> {
            b.funcs.push(lua.create_registry_value(bind(f)?)?);
            Ok(b.funcs.len() - 1)
        };
        let func = reg(table.get("render")?)?;
        let click = opt_fn(&table, "on_click", ctx)?.map(&mut reg).transpose()?;
        let scroll = opt_fn(&table, "on_scroll", ctx)?
            .map(&mut reg)
            .transpose()?;
        let menu = opt_fn(&table, "menu", ctx)?.map(&mut reg).transpose()?;
        (
            p.interval,
            p.anchor,
            WidgetKind::Lua {
                func,
                click,
                scroll,
                menu,
            },
        )
    } else {
        let mut known: Vec<String> = crate::widgets::BUILTINS
            .iter()
            .map(|b| b.id.to_string())
            .collect();
        let mut plugins: Vec<String> = b.plugins.keys().cloned().collect();
        plugins.sort();
        known.extend(plugins);
        return Err(err(crate::tr!(
            "hd.widget : type « {kind} » inconnu (disponibles : {p0} ; plugins dans {p1})",
            kind = kind,
            p0 = known.join(", "),
            p1 = b.base_dir.join("hyprdesk/widgets").display()
        )));
    };
    let anchor = match opt_string(t, "anchor", ctx)? {
        None => default_anchor,
        Some(a) => parse_anchor(&a, ctx)?,
    };
    let interval = opt_num(t, "interval", ctx)?.unwrap_or(default_interval);
    if !(0.1..=31_536_000.0).contains(&interval) {
        return Err(err(crate::tr!(
            "{ctx} : « interval » doit valoir au moins 0.1 seconde",
            ctx = ctx
        )));
    }
    // Identifiant par défaut stable : compté par type ET par origine, pour que
    // les widgets ajoutés dans les réglages ne décalent pas ceux de hyprdesk.lua.
    let n = b
        .config
        .widgets
        .iter()
        .filter(|w| w.type_id == kind && w.origin == b.origin && !w.in_bar)
        .count();
    let default_id = match b.origin {
        Origin::User => format!("{kind}#{n}"),
        Origin::Settings => format!("reglages:{kind}#{n}"),
    };
    let id = opt_string(t, "id", ctx)?.unwrap_or(default_id);
    if b.config.widgets.iter().any(|w| w.id == id) {
        return Err(err(crate::tr!(
            "{ctx} : un widget « {id} » existe déjà (préciser un autre « id »)",
            ctx = ctx,
            id = id
        )));
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
        size: opt_num(t, "size", ctx)?
            .map(|v| v.clamp(0.0, 400.0) as f32)
            .unwrap_or(0.0),
        interval: Duration::from_secs_f64(interval),
        card: opt_bool(t, "card", ctx)?.unwrap_or(true),
        in_bar,
    });
    Ok(())
}

/// `hd.bar({ … })` ou `hd.bar(false)`. Un nouvel appel remplace la barre précédente.
/// Zones d'une barre (`left`, `center`, `right`) : modules intégrés, ou widgets
/// de barre créés à la volée (`bar:<préfixe><zone>:<n>`).
fn bar_zones(
    lua: &Lua,
    b: &mut Builder,
    t: &Table,
    ctx: &str,
    prefix: &str,
) -> mlua::Result<[Vec<BarItem>; 3]> {
    let mut zones: [Vec<BarItem>; 3] = Default::default();
    for (zi, zone) in ["left", "center", "right"].iter().enumerate() {
        let Some(list) = t.get::<Option<Table>>(*zone)? else {
            continue;
        };
        for (i, entry) in list.sequence_values::<Value>().enumerate() {
            let ctx = format!("{ctx}.{zone}[{}]", i + 1);
            let (kind, opts) = match entry? {
                Value::String(s) => (s.to_str()?.to_string(), lua.create_table()?),
                Value::Table(e) => {
                    let kind: String = e.get(1).map_err(|_| {
                        err(crate::tr!(
                            "{ctx} : le premier élément doit être le nom du module",
                            ctx = ctx
                        ))
                    })?;
                    // Options du widget : toutes les clés nommées.
                    let opts = lua.create_table()?;
                    for pair in e.pairs::<Value, Value>() {
                        let (k, v) = pair?;
                        if let Value::String(_) = k {
                            opts.set(k, v)?;
                        }
                    }
                    (kind, opts)
                }
                v => {
                    return Err(err(crate::tr!(
                        "{ctx} : attend un nom de module ou une table, pas {p0}",
                        ctx = ctx,
                        p0 = v.type_name()
                    )));
                }
            };
            if let Some(item) = BarItem::builtin(&kind) {
                zones[zi].push(item);
                continue;
            }
            let id = format!("bar:{prefix}{zone}:{}", i + 1);
            opts.set("id", id.as_str())?;
            hd_widget(lua, b, &kind, &opts, true)
                .map_err(|e| err(format!("{ctx} : {}", short_lua_error(&e))))?;
            zones[zi].push(BarItem::Widget(id));
        }
    }
    Ok(zones)
}

fn hd_bar(lua: &Lua, b: &mut Builder, v: Value) -> mlua::Result<()> {
    const CTX: &str = "hd.bar";
    b.config.widgets.retain(|w| !w.in_bar);
    match b.origin {
        Origin::Settings => b.config.keys_settings.insert("bar".into()),
        Origin::User => b.config.keys_user.insert("bar".into()),
    };
    let t = match v {
        Value::Boolean(false) | Value::Nil => {
            b.config.bar = None;
            return Ok(());
        }
        Value::Table(t) => t,
        v => {
            return Err(err(crate::tr!(
                "{CTX} : attend une table ou false, pas {p0}",
                CTX = CTX,
                p0 = v.type_name()
            )));
        }
    };
    check_keys(
        &t,
        &[
            "monitor", "position", "height", "left", "center", "right", "portrait", "compact",
        ],
        CTX,
    )?;
    let position = match opt_string(&t, "position", CTX)?.as_deref() {
        None | Some("top") => BarPosition::Top,
        Some("bottom") => BarPosition::Bottom,
        Some(s) => {
            return Err(err(crate::tr!(
                "{CTX} : position « {s} » inconnue (top, bottom)",
                CTX = CTX,
                s = s
            )));
        }
    };
    let height = opt_num(&t, "height", CTX)?
        .unwrap_or(36.0)
        .clamp(16.0, 160.0) as u16;
    let compact = match opt_string(&t, "compact", CTX)? {
        None => BarCompact::Auto,
        Some(s) => BarCompact::parse(&s).ok_or_else(|| {
            err(crate::tr!(
                "{CTX} : compact « {s} » inconnu (auto, always, never)",
                CTX = CTX,
                s = s
            ))
        })?,
    };
    let zones = bar_zones(lua, b, &t, CTX, "")?;
    // Écrans verticaux : leurs propres zones.
    let portrait = match t.get::<Option<Table>>("portrait")? {
        None => None,
        Some(p) => {
            const C: &str = "hd.bar.portrait";
            check_keys(&p, &["left", "center", "right"], C)?;
            Some(bar_zones(lua, b, &p, C, "p:")?)
        }
    };
    b.config.bar = Some(BarDef {
        monitors: string_list(t.get("monitor")?, "monitor", CTX)?,
        position,
        height,
        zones,
        portrait,
        compact,
    });
    Ok(())
}

fn hd_component(b: &mut Builder, name: &str, t: &Table) -> mlua::Result<()> {
    let ctx = format!("hd.component(\"{name}\")");
    let ctx = ctx.as_str();
    check_keys(t, &["exec", "restart"], ctx)?;
    let exec = opt_string(t, "exec", ctx)?
        .ok_or_else(|| err(crate::tr!("{ctx} : « exec » est requis", ctx = ctx)))?;
    if b.config.components.iter().any(|c| c.name == name) {
        return Err(err(crate::tr!("{ctx} : composant déjà déclaré", ctx = ctx)));
    }
    b.config.components.push(ComponentDef {
        name: name.to_string(),
        exec,
        restart: opt_bool(t, "restart", ctx)?.unwrap_or(true),
    });
    Ok(())
}

fn hd_place(b: &mut Builder, id: &str, t: &Table) -> mlua::Result<()> {
    let ctx = format!("hd.place(\"{id}\")");
    let ctx = ctx.as_str();
    check_keys(t, &["anchor", "x", "y", "size", "card"], ctx)?;
    let patch = PlacePatch {
        anchor: opt_string(t, "anchor", ctx)?
            .map(|a| parse_anchor(&a, ctx))
            .transpose()?,
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
            if origin == Origin::Settings && !id.starts_with("bar:") {
                // settings.lua est aussi lu seul : conserver le placement généré des widgets utilisateur.
                b.config.placements.insert(
                    id,
                    Placement {
                        anchor: p.anchor.unwrap_or_default(),
                        x: p.x.unwrap_or(0.0),
                        y: p.y.unwrap_or(0.0),
                        size: p.size.unwrap_or(0.0),
                        card: p.card.unwrap_or(true),
                    },
                );
            } else {
                tracing::warn!(
                    "{}",
                    crate::tr!("hd.place : aucun widget « {id} »", id = id)
                );
            }
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
/// Budget renouvelé à chaque appel, également appliqué aux coroutines.
fn reset_budget(lua: &Lua) -> mlua::Result<()> {
    set_budget(lua, Duration::from_secs(5))
}

fn set_budget(lua: &Lua, budget: Duration) -> mlua::Result<()> {
    let deadline = std::time::Instant::now() + budget;
    lua.set_global_hook(
        mlua::HookTriggers::new().every_nth_instruction(10_000),
        move |_, _| {
            if std::time::Instant::now() >= deadline {
                Err(err(crate::i18n::tr("budget d'exécution Lua dépassé (5 s)")))
            } else {
                Ok(mlua::VmState::Continue)
            }
        },
    )
}

fn shell_output(cmd: &str) -> mlua::Result<String> {
    static RUNTIME: std::sync::LazyLock<tokio::runtime::Runtime> = std::sync::LazyLock::new(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime des commandes Lua")
    });
    match RUNTIME.block_on(crate::processes::output(
        tokio::process::Command::new("sh").args(["-c", cmd]),
        Duration::from_secs(5),
    )) {
        Ok(o) if o.status.code() == Some(127) => Err(err(
            "commande introuvable : vérifier les dépendances du widget",
        )),
        Ok(o) => Ok(String::from_utf8_lossy(&o.stdout).trim_end().to_string()),
        Err(e) => Err(err(e.to_string())),
    }
}

fn install_api(lua: &Lua) -> mlua::Result<()> {
    let hd = lua.create_table()?;

    fn with_builder<R>(
        lua: &Lua,
        f: impl FnOnce(&mut Builder) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        let cell = lua
            .app_data_ref::<RefCell<Builder>>()
            .ok_or_else(|| err("hd.* appelé hors du chargement de la config"))?;
        let mut b = cell.borrow_mut();
        f(&mut b)
    }

    hd.set(
        "_config",
        lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_config(b, &t)))?,
    )?;
    hd.set(
        "_wallpaper",
        lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_wallpaper(b, &t)))?,
    )?;
    hd.set(
        "_desktop",
        lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_desktop(b, &t)))?,
    )?;
    hd.set(
        "_assign",
        lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_assign(b, &t)))?,
    )?;
    hd.set(
        "_icon",
        lua.create_function(|lua, t: Table| with_builder(lua, |b| hd_icon(b, &t)))?,
    )?;
    hd.set(
        "_widget",
        lua.create_function(|lua, (kind, t): (String, Option<Table>)| {
            let t = match t {
                Some(t) => t,
                None => lua.create_table()?,
            };
            with_builder(lua, |b| hd_widget(lua, b, &kind, &t, false))
        })?,
    )?;
    hd.set(
        "_on",
        lua.create_function(|lua, (event, f): (String, Function)| {
            let ev = HookEvent::parse(&event).ok_or_else(|| {
                err(crate::tr!(
                    "hd.on : événement « {event} » inconnu (show, hide, workspace)",
                    event = event
                ))
            })?;
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
        lua.create_function(|lua, (id, t): (String, Table)| {
            with_builder(lua, |b| hd_place(b, &id, &t))
        })?,
    )?;
    hd.set(
        "_bar",
        lua.create_function(|lua, v: Value| with_builder(lua, |b| hd_bar(lua, b, v)))?,
    )?;
    hd.set(
        "_component",
        lua.create_function(|lua, (name, t): (String, Table)| {
            with_builder(lua, |b| hd_component(b, &name, &t))
        })?,
    )?;
    hd.set(
        "_disable",
        lua.create_function(|lua, id: String| {
            with_builder(lua, |b| {
                b.disabled.push((b.origin, id));
                Ok(())
            })
        })?,
    )?;
    hd.set(
        "exec",
        lua.create_function(|_, cmd: String| shell_output(&cmd))?,
    )?;
    // Lance sans attendre, par Hyprland (le programme ne dépend pas de hyprdesk).
    hd.set(
        "spawn",
        lua.create_function(|_, cmd: String| {
            if crate::hypr::exec(&cmd).is_err() {
                let _ = std::process::Command::new("setsid")
                    .args(["-f", "sh", "-c", &cmd])
                    .spawn();
            }
            Ok(())
        })?,
    )?;
    // Traduction : le texte français dans la langue de l'interface (catalogue
    // de hyprdesk, sinon tel quel) ; `hd.lang()` pour les plugins qui ont les leurs.
    hd.set(
        "t",
        lua.create_function(|_, s: String| Ok(crate::i18n::tr(&s).to_string()))?,
    )?;
    hd.set(
        "lang",
        lua.create_function(|_, ()| Ok(crate::i18n::code()))?,
    )?;
    hd.set("version", env!("CARGO_PKG_VERSION"))?;
    // Chemin du hyprdesk en cours : les plugins l'appellent sans dépendre du PATH.
    hd.set(
        "exe",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "hyprdesk".into()),
    )?;
    lua.globals().set("hd", hd)?;
    // Les fonctions Rust ne connaissent pas la ligne appelante : chaque `hd.x`
    // relance l'erreur au niveau 2, pour qu'elle désigne `fichier:ligne` de
    // l'appel dans la config, comme une erreur Lua ordinaire.
    lua.load(
        r#"
        for _, name in ipairs({ "config", "wallpaper", "desktop", "assign", "icon", "widget", "place", "disable", "component", "bar", "on" }) do
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
    func_base: usize,
}

impl Loaded {
    fn func(&self, idx: usize) -> mlua::Result<Function> {
        let key =
            self.funcs
                .get(idx.checked_sub(self.func_base).ok_or_else(|| {
                    err(crate::i18n::tr("fonction Lua périmée après rechargement"))
                })?)
                .ok_or_else(|| err(crate::tr!("fonction Lua #{idx} introuvable", idx = idx)))?;
        self.lua.registry_value(key)
    }

    /// Appelle la fonction `render` d'un widget Lua.
    /// Appelle `render` : une chaîne, ou une table { text, icon, tooltip, state }.
    pub fn render(&self, idx: usize) -> Result<Rendered, String> {
        reset_budget(&self.lua).map_err(|e| e.to_string())?;
        let f = self.func(idx).map_err(|e| e.to_string())?;
        let v = f.call::<Value>(()).map_err(|e| short_lua_error(&e))?;
        match v {
            Value::Table(t) => {
                let s = |k: &str| -> Result<Option<String>, String> {
                    opt_string(&t, k, "render").map_err(|e| short_lua_error(&e))
                };
                let state = match s("state")? {
                    None => RenderState::Normal,
                    Some(x) => RenderState::parse(&x).ok_or_else(|| crate::tr!("render : state « {x} » inconnu (normal, active, warning, critical, hidden)", x = x))?,
                };
                Ok(Rendered {
                    text: s("text")?.unwrap_or_default(),
                    icon: s("icon")?,
                    tooltip: s("tooltip")?,
                    state,
                })
            }
            other => Ok(Rendered {
                text: scalar_text(&other).map_err(|t| {
                    crate::tr!("render doit rendre une chaîne ou une table, pas {t}", t = t)
                })?,
                ..Default::default()
            }),
        }
    }

    /// Appelle `on_click` (sans argument) ou `on_scroll` (sens ±1).
    pub fn call(&self, idx: usize, arg: Option<i64>) -> Result<(), String> {
        reset_budget(&self.lua).map_err(|e| e.to_string())?;
        let f = self.func(idx).map_err(|e| e.to_string())?;
        match arg {
            Some(a) => f.call::<()>(a),
            None => f.call::<()>(()),
        }
        .map_err(|e| short_lua_error(&e))
    }

    /// Appelle `menu` ; les fonctions `action` des entrées sont rangées dans
    /// `actions`, l'entrée en garde l'index.
    pub fn menu(&self, idx: usize, actions: &mut Vec<Function>) -> Result<Vec<MenuEntry>, String> {
        reset_budget(&self.lua).map_err(|e| e.to_string())?;
        let f = self.func(idx).map_err(|e| e.to_string())?;
        let t: Table = match f.call::<Value>(()).map_err(|e| short_lua_error(&e))? {
            Value::Table(t) => t,
            Value::Nil => return Ok(Vec::new()),
            v => {
                return Err(crate::tr!(
                    "menu doit rendre une liste, pas {p0}",
                    p0 = v.type_name()
                ));
            }
        };
        actions.clear();
        let mut out = Vec::new();
        for v in t.sequence_values::<Value>() {
            match v.map_err(|e| e.to_string())? {
                Value::String(s) if s.to_string_lossy() == "-" => out.push(MenuEntry::Separator),
                Value::Table(e) => {
                    let gs = |k: &str| opt_string(&e, k, "menu").map_err(|e| short_lua_error(&e));
                    if let Some(h) = gs("header")? {
                        out.push(MenuEntry::Header(h));
                        continue;
                    }
                    let action = match e.get::<Value>("action").map_err(|e| e.to_string())? {
                        Value::Function(f) => {
                            actions.push(f);
                            Some(actions.len() as i64 - 1)
                        }
                        _ => None,
                    };
                    let flag = |k: &str| opt_bool(&e, k, "menu").map_err(|e| short_lua_error(&e));
                    // `{ input = "…", secret = true, action = function(text) … end }`
                    if let Some(placeholder) = gs("input")? {
                        out.push(MenuEntry::Input {
                            label: gs("label")?,
                            placeholder,
                            secret: flag("secret")?.unwrap_or(false),
                            action,
                        });
                        continue;
                    }
                    out.push(MenuEntry::Item {
                        label: gs("label")?.unwrap_or_default(),
                        detail: gs("detail")?,
                        icon: gs("icon")?,
                        checked: flag("checked")?,
                        enabled: !flag("disabled")?.unwrap_or(false),
                        action,
                        confirm: gs("confirm")?,
                    });
                }
                other => {
                    return Err(crate::tr!(
                        "menu : entrée inattendue ({p0})",
                        p0 = other.type_name()
                    ));
                }
            }
        }
        Ok(out)
    }

    /// Appelle un hook `hd.on` avec `{ event, monitor, workspace }`.
    pub fn hook(
        &self,
        idx: usize,
        event: &str,
        monitor: &str,
        workspace: &str,
    ) -> Result<(), String> {
        reset_budget(&self.lua).map_err(|e| e.to_string())?;
        let f = self.func(idx).map_err(|e| e.to_string())?;
        let t = self.lua.create_table().map_err(|e| e.to_string())?;
        let _ = t.set("event", event);
        let _ = t.set("monitor", monitor);
        let _ = t.set("workspace", workspace);
        f.call::<()>(t).map_err(|e| short_lua_error(&e))
    }
}

fn scalar_text(v: &Value) -> Result<String, &'static str> {
    match v {
        Value::String(s) => Ok(s.to_string_lossy().to_string()),
        Value::Nil => Ok(String::new()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        v => Err(v.type_name()),
    }
}

/// Première ligne utile d'une erreur Lua (sans la pile d'appels).
pub fn short_lua_error(e: &mlua::Error) -> String {
    let s = e.to_string();
    let line = s.lines().next().unwrap_or(&s).trim();
    let line = line.strip_prefix("runtime error: ").unwrap_or(line);
    line.strip_prefix("syntax error: ")
        .unwrap_or(line)
        .to_string()
}

/// Exécute une source de config. `base_dir` sert aux chemins relatifs et à `require`.
/// Exécute une source de config. `base_dir` sert aux chemins relatifs et à `require`.
#[cfg(test)]
pub fn load_source(src: &str, chunk_name: &str, base_dir: &Path) -> Result<Loaded, String> {
    load_chunks(
        &[(src.to_string(), chunk_name.to_string(), Origin::User)],
        base_dir,
    )
}

/// Exécute plusieurs sources dans le MÊME état Lua, dans l'ordre : ce qui
/// vient après l'emporte (dernière déclaration, dernière valeur de `hd.config`).
pub fn load_chunks(chunks: &[(String, String, Origin)], base_dir: &Path) -> Result<Loaded, String> {
    let lua = Lua::new();
    reset_budget(&lua).map_err(|e| e.to_string())?;
    let setup = || -> mlua::Result<()> {
        install_api(&lua)?;
        let package: Table = lua.globals().get("package")?;
        let old: String = package.get("path")?;
        let mods = base_dir.join("hyprdesk");
        package.set(
            "path",
            format!("{0}/?.lua;{0}/?/init.lua;{old}", mods.display()),
        )?;
        Ok(())
    };
    setup().map_err(|e| format!("initialisation Lua : {e}"))?;
    // Plugins de widgets : chargés avant la config, chacun isolément ; un
    // plugin cassé apparaît en erreur dans le catalogue sans bloquer le reste.
    let mut catalog = crate::widgets::builtin_types();
    let mut plugins = std::collections::HashMap::new();
    for (id, source) in crate::widgets::plugins::all(&crate::widgets::plugins::dirs(base_dir)) {
        let broken = |error: String| WidgetType {
            id: id.clone(),
            name: id.clone(),
            description: String::new(),
            source: match &source {
                crate::widgets::plugins::Source::File(p) => WidgetSource::Plugin(p.clone()),
                crate::widgets::plugins::Source::Bundled(_) => WidgetSource::Bundled,
            },
            anchor: Anchor::TopRight,
            toggleable: false,
            error: Some(error),
        };
        if !crate::widgets::plugins::valid_id(&id) {
            catalog.push(broken(
                "nom invalide : minuscules, chiffres, - et _ seulement".into(),
            ));
            continue;
        }
        if crate::widgets::builtin(&id).is_some() {
            catalog.push(broken(format!("« {id} » est le nom d'un widget intégré")));
            continue;
        }
        match load_plugin(&lua, &id, &source) {
            Ok((p, wt)) => {
                plugins.insert(id.clone(), p);
                catalog.push(wt);
            }
            Err(e) => {
                tracing::warn!("plugin {id} : {e}");
                catalog.push(broken(e));
            }
        }
    }
    let config = Config {
        catalog,
        ..Config::default()
    };
    lua.set_app_data(RefCell::new(Builder {
        config,
        funcs: Vec::new(),
        base_dir: base_dir.to_path_buf(),
        origin: Origin::User,
        plugins,
        places: Vec::new(),
        disabled: Vec::new(),
    }));
    for (src, name, origin) in chunks {
        if let Some(b) = lua.app_data_ref::<RefCell<Builder>>() {
            b.borrow_mut().origin = *origin;
        }
        reset_budget(&lua).map_err(|e| e.to_string())?;
        lua.load(src.as_str())
            .set_name(format!("@{name}"))
            .exec()
            .map_err(|e| short_lua_error(&e))?;
    }
    let mut b = lua
        .remove_app_data::<RefCell<Builder>>()
        .expect("builder posé plus haut")
        .into_inner();
    apply_places(&mut b);
    b.config.general.expand_workspace_monitors();
    // Widgets désactivés depuis les réglages, quelle que soit leur origine.
    for (origin, id) in std::mem::take(&mut b.disabled) {
        match b.config.widgets.iter().position(|w| w.id == id) {
            Some(i) => {
                let w = b.config.widgets.remove(i);
                b.config.disabled.insert(id, w.type_id);
            }
            // settings.lua est aussi lu seul (état de l'écran Réglages) : les
            // widgets de hyprdesk.lua n'y existent pas, mais leur désactivation
            // doit survivre, sinon le prochain enregistrement l'efface et en
            // éteindre un autre rallume celui-ci. Le type se lit dans l'id
            // (`system#0`).
            None if origin == Origin::Settings => {
                let type_id = id
                    .rsplit_once('#')
                    .map(|(t, _)| t)
                    .unwrap_or(&id)
                    .to_string();
                b.config.disabled.insert(id, type_id);
            }
            None => tracing::warn!("hd.disable : aucun widget « {id} »"),
        }
    }
    static NEXT_FUNCTION: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let func_base =
        NEXT_FUNCTION.fetch_add(b.funcs.len() + 1, std::sync::atomic::Ordering::Relaxed);
    for widget in &mut b.config.widgets {
        if let WidgetKind::Lua {
            func,
            click,
            scroll,
            menu,
        } = &mut widget.kind
        {
            *func += func_base;
            for id in [click, scroll, menu].into_iter().flatten() {
                *id += func_base;
            }
        }
    }
    for (_, id) in &mut b.config.hooks {
        *id += func_base;
    }
    Ok(Loaded {
        lua,
        config: b.config,
        funcs: b.funcs,
        func_base,
    })
}

/// Module écrit par l'écran Réglages, à côté de la config de l'utilisateur.
pub fn settings_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("hyprdesk")
        .join("settings.lua")
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
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("lecture de {} : {e}", path.display()))?;
    chunks.push((src, path.display().to_string(), Origin::User));
    load_chunks(&chunks, base)
}

/// Charge SEULEMENT `settings.lua`, pour que l'écran Réglages relise ses propres valeurs.
pub fn load_settings_only(config_path: &Path) -> Result<Config, String> {
    let sp = settings_path(config_path);
    let Ok(src) = std::fs::read_to_string(&sp) else {
        return Ok(Config::default());
    };
    let base = config_path.parent().unwrap_or(Path::new("."));
    load_chunks(&[(src, sp.display().to_string(), Origin::Settings)], base).map(|l| l.config)
}

// ---------------------------------------------------------------------------
// Thread Lua

enum Req {
    Load {
        path: PathBuf,
        reply: oneshot::Sender<Result<Config, String>>,
    },
    Render {
        func: usize,
        reply: oneshot::Sender<Result<Rendered, String>>,
    },
    Call {
        func: usize,
        arg: Option<i64>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Menu {
        func: usize,
        reply: oneshot::Sender<Result<Vec<MenuEntry>, String>>,
    },
    MenuAction {
        action: usize,
        /// Texte saisi (entrée `input`), passé à la fonction Lua.
        arg: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Hook {
        funcs: Vec<usize>,
        event: String,
        monitor: String,
        workspace: String,
    },
}

/// Poignée clonable vers le thread qui possède l'état Lua.
#[derive(Clone, Debug)]
pub struct LuaHandle {
    tx: std::sync::mpsc::SyncSender<Req>,
}

impl LuaHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Req>(64);
        std::thread::Builder::new()
            .name("hyprdesk-lua".into())
            .spawn(move || {
                let mut current: Option<Loaded> = None;
                // Actions du dernier menu ouvert (fonctions Lua des entrées).
                let mut actions: Vec<Function> = Vec::new();
                let mut action_base = 0usize;
                let mut next_action = 0usize;
                while let Ok(req) = rx.recv() {
                    match req {
                        Req::Load { path, reply } => match load_file(&path) {
                            Ok(l) => {
                                let cfg = l.config.clone();
                                if reply.send(Ok(cfg)).is_ok() {
                                    actions.clear();
                                    action_base = next_action;
                                    current = Some(l);
                                }
                            }
                            // On garde l'état précédent : la dernière config valide reste active.
                            Err(e) => {
                                let _ = reply.send(Err(e));
                            }
                        },
                        Req::Render { func, reply } => {
                            if reply.is_closed() {
                                continue;
                            }
                            let r = current
                                .as_ref()
                                .ok_or_else(|| "aucune config chargée".to_string())
                                .and_then(|l| l.render(func));
                            let _ = reply.send(r);
                        }
                        Req::Call { func, arg, reply } => {
                            if reply.is_closed() {
                                continue;
                            }
                            let r = current
                                .as_ref()
                                .ok_or_else(|| "aucune config chargée".to_string())
                                .and_then(|l| l.call(func, arg));
                            let _ = reply.send(r);
                        }
                        Req::Menu { func, reply } => {
                            if reply.is_closed() {
                                continue;
                            }
                            let r = current
                                .as_ref()
                                .ok_or_else(|| "aucune config chargée".to_string())
                                .and_then(|l| l.menu(func, &mut actions))
                                .map(|mut entries| {
                                    action_base = next_action;
                                    next_action += actions.len();
                                    for entry in &mut entries {
                                        if let MenuEntry::Item {
                                            action: Some(action),
                                            ..
                                        } = entry
                                        {
                                            *action += action_base as i64;
                                        }
                                    }
                                    entries
                                });
                            let _ = reply.send(r);
                        }
                        Req::MenuAction { action, arg, reply } => {
                            if reply.is_closed() {
                                continue;
                            }
                            let r = match action
                                .checked_sub(action_base)
                                .and_then(|index| actions.get(index))
                            {
                                Some(f) => current
                                    .as_ref()
                                    .ok_or_else(|| "aucune config chargée".to_string())
                                    .and_then(|l| {
                                        reset_budget(&l.lua).map_err(|e| e.to_string())?;
                                        f.call::<()>(arg).map_err(|e| short_lua_error(&e))
                                    }),
                                None => Err("action de menu introuvable".into()),
                            };
                            let _ = reply.send(r);
                        }
                        Req::Hook {
                            funcs,
                            event,
                            monitor,
                            workspace,
                        } => {
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
        self.tx
            .try_send(Req::Load { path, reply })
            .map_err(|_| "thread Lua arrêté".to_string())?;
        tokio::time::timeout(Duration::from_secs(15), rx)
            .await
            .map_err(|_| "thread Lua occupé : délai dépassé".to_string())?
            .map_err(|_| "thread Lua arrêté".to_string())?
    }

    pub async fn render(&self, func: usize) -> Result<Rendered, String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .try_send(Req::Render { func, reply })
            .map_err(|_| "thread Lua arrêté".to_string())?;
        tokio::time::timeout(Duration::from_secs(15), rx)
            .await
            .map_err(|_| "thread Lua occupé : délai dépassé".to_string())?
            .map_err(|_| "thread Lua arrêté".to_string())?
    }

    pub async fn call(&self, func: usize, arg: Option<i64>) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .try_send(Req::Call { func, arg, reply })
            .map_err(|_| "thread Lua arrêté".to_string())?;
        tokio::time::timeout(Duration::from_secs(15), rx)
            .await
            .map_err(|_| "thread Lua occupé : délai dépassé".to_string())?
            .map_err(|_| "thread Lua arrêté".to_string())?
    }

    pub async fn menu(&self, func: usize) -> Result<Vec<MenuEntry>, String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .try_send(Req::Menu { func, reply })
            .map_err(|_| "thread Lua arrêté".to_string())?;
        tokio::time::timeout(Duration::from_secs(15), rx)
            .await
            .map_err(|_| "thread Lua occupé : délai dépassé".to_string())?
            .map_err(|_| "thread Lua arrêté".to_string())?
    }

    pub async fn menu_action(&self, action: usize, arg: Option<String>) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .try_send(Req::MenuAction { action, arg, reply })
            .map_err(|_| "thread Lua arrêté".to_string())?;
        tokio::time::timeout(Duration::from_secs(15), rx)
            .await
            .map_err(|_| "thread Lua occupé : délai dépassé".to_string())?
            .map_err(|_| "thread Lua arrêté".to_string())?
    }

    pub fn hook(&self, funcs: Vec<usize>, event: &str, monitor: &str, workspace: &str) {
        if funcs.is_empty() {
            return;
        }
        let _ = self.tx.try_send(Req::Hook {
            funcs,
            event: event.into(),
            monitor: monitor.into(),
            workspace: workspace.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(src: &str) -> Result<Loaded, String> {
        load_source(src, "test.lua", Path::new("/cfg"))
    }

    #[test]
    fn bar_widgets_do_not_shift_desktop_widget_identifiers() {
        let loaded = load("hd.bar({ center={'clock'} }) hd.widget('clock')").unwrap();
        assert_eq!(
            loaded.config.widgets.iter().find(|w| !w.in_bar).unwrap().id,
            "clock#0"
        );
    }

    #[test]
    fn function_handles_cannot_be_reused_by_a_new_lua_state() {
        let previous = load("hd.widget('lua',{ render=function() return 'old' end })").unwrap();
        let current = load("hd.widget('lua',{ render=function() return 'new' end })").unwrap();
        let WidgetKind::Lua { func: old, .. } = previous.config.widgets[0].kind else {
            panic!()
        };
        let WidgetKind::Lua { func: new, .. } = current.config.widgets[0].kind else {
            panic!()
        };
        assert!(current.render(old).is_err());
        assert_eq!(current.render(new).unwrap().text, "new");
        assert_eq!(previous.render(old).unwrap().text, "old");
    }

    #[test]
    fn veille_geree() {
        let c = load(
            "hd.config({ idle_manage = true, idle_lock = 600, idle_suspend = 0, idle_dim = 1e9 })",
        )
        .unwrap()
        .config;
        let i = &c.general.idle;
        assert!(i.manage);
        assert_eq!((i.lock, i.suspend, i.dim), (600, 0, 86_400));
        assert_eq!(i.dpms, crate::config::IdleCfg::default().dpms);
        assert!(load("hd.config({ idle_lock = 'tard' })").is_err());
    }

    #[test]
    fn chemin_du_binaire_expose() {
        assert!(load("assert(type(hd.exe) == 'string' and #hd.exe > 0)").is_ok());
    }

    #[test]
    fn non_finite_numbers_and_overflow_are_configuration_errors() {
        for source in [
            "hd.widget('clock', { interval=0/0 })",
            "hd.widget('clock', { interval=math.huge })",
            "hd.wallpaper({ rotate={ dir='/tmp', interval=1e300 } })",
            "hd.config({ font_size=0/0 })",
        ] {
            assert!(load(source).is_err(), "{source}");
        }
    }

    #[test]
    fn lua_loops_are_interruptible_including_coroutines() {
        let lua = Lua::new();
        for source in [
            "while true do end",
            "local c=coroutine.create(function() while true do end end); assert(coroutine.resume(c))",
        ] {
            set_budget(&lua, Duration::from_millis(20)).unwrap();
            let error = lua.load(source).exec().unwrap_err().to_string();
            assert!(error.contains("budget"), "{error}");
        }
    }

    #[tokio::test]
    async fn stale_menu_actions_never_run_another_menus_action() {
        let dir =
            std::env::temp_dir().join(format!("hyprdesk-menu-{}", crate::storage::revision()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.lua");
        std::fs::write(&path,"hd.widget('lua', { render=function() return 'ok' end, menu=function() return {{label='Action', action=function() end}} end })").unwrap();
        let handle = LuaHandle::spawn();
        let cfg = handle.load(path).await.unwrap();
        let WidgetKind::Lua {
            menu: Some(index), ..
        } = cfg.widgets[0].kind
        else {
            panic!()
        };
        let action_id = |entries: Vec<MenuEntry>| match entries[0] {
            MenuEntry::Item {
                action: Some(id), ..
            } => id as usize,
            _ => panic!(),
        };
        let old = action_id(handle.menu(index).await.unwrap());
        let current = action_id(handle.menu(index).await.unwrap());
        assert_ne!(old, current);
        assert!(handle.menu_action(old, None).await.is_err());
        handle.menu_action(current, None).await.unwrap();
        std::fs::remove_dir_all(dir).unwrap();
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
        assert_eq!(
            c.wallpapers[0].source,
            WallSource::Path(home().join("w/a.jpg"))
        );
        assert_eq!(
            c.wallpapers[1].sel.workspaces,
            Some(vec!["1".into(), "2".into()])
        );
        assert!(
            matches!(&c.wallpapers[1].source, WallSource::Rotate { dir, shuffle: true, .. } if dir == Path::new("/cfg/rot"))
        );
        assert_eq!(c.wallpapers[1].fit, Fit::Contain);
        assert_eq!(c.desktops[0].sel.workspaces, Some(vec!["4".into()]));
        assert!(!c.desktops[0].inherit);
        assert_eq!(c.icons[0].kind, IconKind::App("firefox".into()));
        assert_eq!(c.icons[1].pos, Some((0, 1)));
        assert_eq!(
            c.icons[2].kind,
            IconKind::Exec {
                cmd: "htop".into(),
                terminal: true
            }
        );
        assert_eq!(c.widgets.len(), 3);
        assert_eq!(c.widgets[0].anchor, Anchor::TopLeft);
        assert!(
            matches!(c.widgets[1].kind, WidgetKind::System { ref show } if show[1] == SysItem::Disk("/home".into()))
        );
        let WidgetKind::Lua { func, .. } = c.widgets[2].kind else {
            panic!()
        };
        assert_eq!(
            l.render(func).unwrap().text,
            format!("salut {}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(c.hooks.len(), 1);
    }

    #[test]
    fn barre_des_ecrans_verticaux() {
        let c = load(
            r#"hd.bar({ compact = "always", left = { "workspaces", "window" }, right = { "volume", "clock" },
                        portrait = { left = { "workspaces" }, right = { { "clock", format = "%H:%M" } } } })"#,
        )
        .unwrap()
        .config;
        let b = c.bar.unwrap();
        assert_eq!(b.compact, BarCompact::Always);
        let p = b.portrait.unwrap();
        assert_eq!(p[0], vec![BarItem::Workspaces]);
        assert!(matches!(&p[2][0], BarItem::Widget(id) if id == "bar:p:right:1"));
        // Les deux horloges sont des widgets distincts.
        assert_eq!(c.widgets.iter().filter(|w| w.in_bar).count(), 2);
        assert!(load(r#"hd.bar({ compact = "parfois" })"#).is_err());
        assert!(load(r#"hd.bar({ portrait = { haut = {} } })"#).is_err());
    }

    #[test]
    fn etalement_au_verrouillage() {
        let span = |src: &str| {
            let c = load(src).unwrap().config;
            (c.wallpapers[0].span, c.wallpapers[0].span_lock)
        };
        assert_eq!(
            span(r#"hd.wallpaper({ path = "/a.png", span = true })"#),
            (true, true)
        );
        assert_eq!(
            span(r#"hd.wallpaper({ path = "/a.png", span = "lock" })"#),
            (false, true)
        );
        assert_eq!(
            span(r#"hd.wallpaper({ path = "/a.png", span = "desktop" })"#),
            (true, false)
        );
        assert_eq!(span(r#"hd.wallpaper({ path = "/a.png" })"#), (false, false));
        assert!(load(r#"hd.wallpaper({ path = "/a.png", span = "ailleurs" })"#).is_err());
        assert!(load(r##"hd.wallpaper({ color = "#000000", span = "lock" })"##).is_err());
    }

    #[test]
    fn layouts() {
        let c = load(
            r#"hd.config({ layout = "master", layout_workspaces = { [3] = "monocle", ["name:web"] = "grid" },
                           master = { mfact = 0.6, orientation = "center" }, dwindle = { force_split = 2 } })"#,
        )
        .unwrap()
        .config;
        let t = &c.general.tiling;
        assert_eq!(t.layout.as_deref(), Some("master"));
        assert_eq!(t.workspaces["3"], "monocle");
        assert_eq!(t.workspaces["name:web"], "grid");
        assert_eq!(t.options.len(), 3);
        // Nom, option et valeur vérifiés.
        for bad in [
            r#"hd.config({ layout = "spirale" })"#,
            r#"hd.config({ master = { inconnue = 1 } })"#,
            r#"hd.config({ master = { mfact = 3 } })"#,
            r#"hd.config({ scrolling = { direction = "ailleurs" } })"#,
        ] {
            assert!(load(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn ecran_dedie() {
        let c = load(
            r#"hd.config({ workspaces = 6, workspace_dedicated = { "HDMI-A-2" },
                           workspace_monitors = { ["HDMI-A-2"] = { 6 } } })"#,
        )
        .unwrap()
        .config;
        let g = &c.general;
        // Déclaré : tel quel ; effectif : tous les autres sur l'écran principal.
        assert_eq!(g.workspace_monitors_declared.len(), 1);
        assert_eq!(g.workspace_monitors["HDMI-A-2"], ["6"]);
        let rest: Vec<&str> = g.workspace_monitors["primary"]
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(rest, ["1", "2", "3", "4", "5", "7", "8", "9", "10"]);
        // Sans écran dédié : rien d'ajouté.
        let c = load(r#"hd.config({ workspace_monitors = { ["DP-1"] = { 1 } } })"#)
            .unwrap()
            .config;
        assert_eq!(c.general.workspace_monitors.len(), 1);
    }

    #[test]
    fn ecran_principal() {
        let c = load(r#"hd.config({ primary = "DP-1", show_on = "primary" }) hd.bar({ monitor = "primary" })"#)
            .unwrap()
            .config;
        assert_eq!(c.general.primary.as_deref(), Some("DP-1"));
        assert_eq!(c.general.show_on, ShowOn::Primary);
        assert_eq!(c.bar.unwrap().monitors, Some(vec!["primary".to_string()]));
        assert!(load("hd.config({ primary = 3 })").is_err());
        assert_eq!(
            load("hd.config({ primary = false })")
                .unwrap()
                .config
                .general
                .primary,
            None
        );
    }

    #[test]
    fn workspaces_et_affectations() {
        let c = load(
            r#"hd.config({ workspaces = 6 })
               hd.assign({ app = "firefox.desktop", workspace = 2 })
               hd.assign({ app = "firefox", workspace = "3", silent = true })"#,
        )
        .unwrap()
        .config;
        assert_eq!(c.general.workspaces, Some(6));
        assert_eq!(c.assigns.len(), 1, "la dernière affectation l'emporte");
        assert_eq!(c.assigns[0].app, AppMatch::App("firefox".into()));
        assert_eq!(
            (c.assigns[0].workspace.as_str(), c.assigns[0].silent),
            ("3", true)
        );
        for bad in [
            "hd.config({ workspaces = 0 })",
            "hd.config({ workspaces = 2.5 })",
            "hd.config({ workspaces = 'six' })",
            "hd.assign({ workspace = 2 })",
            "hd.assign({ app = 'a', class = 'b', workspace = 2 })",
            "hd.assign({ app = 'a' })",
        ] {
            assert!(load(bad).is_err(), "{bad}");
        }
        assert_eq!(
            load("hd.config({ workspaces = false })")
                .unwrap()
                .config
                .general
                .workspaces,
            None
        );
    }

    #[test]
    fn champ_inconnu_refuse() {
        let e = load("\n\nhd.icon({ app = \"x\", lable = \"faute\" })")
            .err()
            .unwrap();
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
        let e = load(r##"hd.wallpaper({ path = "a", color = "#000000" })"##)
            .err()
            .unwrap();
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
        std::fs::write(
            dir.join("hyprdesk/icones.lua"),
            r#"hd.icon({ app = "kitty" })"#,
        )
        .unwrap();
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
        assert!(
            c.keys_settings.contains("grid.icon_size") && c.keys_user.contains("grid.icon_size")
        );
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
        assert!(
            c.catalog
                .iter()
                .find(|t| t.id == "casse")
                .unwrap()
                .error
                .is_some()
        );
        assert_eq!(
            c.catalog.iter().find(|t| t.id == "meteo").unwrap().name,
            "Météo"
        );
        let render = |i: usize| match c.widgets[i].kind {
            WidgetKind::Lua { func, .. } => l.render(func).unwrap().text,
            _ => panic!(),
        };
        assert_eq!(render(0), "il fait beau à Paris");
        assert_eq!(render(1), "il fait beau à Lyon");
        assert_eq!(c.widgets[1].id, "meteo#0");
        assert_eq!(c.widgets[1].anchor, Anchor::TopLeft);
        assert_eq!(c.widgets[1].interval, Duration::from_secs(900));
        // hd.place des réglages s'applique au widget de hyprdesk.lua.
        assert_eq!(c.widgets[2].id, "clock#0");
        assert_eq!(
            (c.widgets[2].anchor, c.widgets[2].y),
            (Anchor::Center, 10.0)
        );
        assert_eq!(
            c.placements.get("clock#0").map(|p| p.anchor),
            Some(Anchor::Center)
        );
        let e = load_chunks(
            &[(
                r#"hd.widget("meteo", { pays = "FR" })"#.into(),
                "x.lua".into(),
                Origin::User,
            )],
            &dir,
        )
        .err()
        .unwrap();
        assert!(e.contains("champ inconnu « pays »"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn barre() {
        let l = load(
            r#"hd.widget("clock")
               hd.bar({ position = "bottom", height = 30, left = { "workspaces", "window" },
                        center = { { "clock", format = "%H:%M:%S" } }, right = { "volume", "system", "desktop" } })"#,
        )
        .unwrap();
        let c = &l.config;
        let bar = c.bar.as_ref().unwrap();
        assert_eq!((bar.position, bar.height), (BarPosition::Bottom, 30));
        assert_eq!(bar.zones[0], vec![BarItem::Workspaces, BarItem::Window]);
        assert_eq!(bar.zones[1], vec![BarItem::Widget("bar:center:1".into())]);
        assert_eq!(bar.zones[2][1], BarItem::Widget("bar:right:2".into()));
        let w = c.widgets.iter().find(|w| w.id == "bar:center:1").unwrap();
        assert!(w.in_bar);
        assert_eq!(
            w.kind,
            WidgetKind::Clock {
                format: "%H:%M:%S".into()
            }
        );
        // Les modules de barre ne sont pas sur le bureau.
        let d = super::super::resolve::resolve(c, "M", "1");
        assert_eq!(d.widgets.len(), 1);
        assert!(c.keys_user.contains("bar"));
        let e = load(r#"hd.bar({ left = { "meteo-inconnue" } })"#)
            .err()
            .unwrap();
        assert!(e.contains("hd.bar.left[1]"), "{e}");
        let off = load(r#"hd.bar({ left = { "clock" } }) hd.bar(false)"#).unwrap();
        assert!(off.config.bar.is_none() && off.config.widgets.is_empty());
    }

    #[test]
    fn plugin_riche_clic_et_menu() {
        let l = load(
            r#"local etat = { n = 0 }
               hd.widget("lua", {
                 render = function() return { text = tostring(etat.n), icon = "x-symbolic", state = etat.n > 1 and "warning" or "normal" } end,
                 on_click = function() etat.n = etat.n + 1 end,
                 on_scroll = function(d) etat.n = etat.n + d * 10 end,
                 menu = function() return { { header = "Titre" }, { label = "Ajouter", action = function() etat.n = 100 end }, "-",
                                            { label = "Coché", checked = true, disabled = true } } end,
               })"#,
        )
        .unwrap();
        let WidgetKind::Lua {
            func,
            click: Some(c),
            scroll: Some(sc),
            menu: Some(m),
        } = l.config.widgets[0].kind
        else {
            panic!()
        };
        assert_eq!(l.render(func).unwrap().icon.as_deref(), Some("x-symbolic"));
        l.call(c, None).unwrap();
        l.call(c, None).unwrap();
        assert_eq!(l.render(func).unwrap().state, RenderState::Warning);
        l.call(sc, Some(1)).unwrap();
        assert_eq!(l.render(func).unwrap().text, "12");
        let mut actions = Vec::new();
        let menu = l.menu(m, &mut actions).unwrap();
        assert_eq!(menu.len(), 4);
        assert_eq!(menu[0], MenuEntry::Header("Titre".into()));
        assert!(
            matches!(&menu[1], MenuEntry::Item { label, action: Some(0), .. } if label == "Ajouter")
        );
        assert_eq!(menu[2], MenuEntry::Separator);
        assert!(matches!(
            &menu[3],
            MenuEntry::Item {
                checked: Some(true),
                enabled: false,
                ..
            }
        ));
        actions[0].call::<()>(()).unwrap();
        assert_eq!(l.render(func).unwrap().text, "100");
    }

    #[test]
    fn menu_avec_saisie() {
        let l = load(
            r#"local recu = ""
               hd.widget("lua", {
                 render = function() return { text = recu } end,
                 menu = function() return { { input = "Mot de passe", label = "Wi-Fi « Maison »", secret = true,
                                              action = function(t) recu = t end } } end,
               })"#,
        )
        .unwrap();
        let WidgetKind::Lua {
            func,
            menu: Some(m),
            ..
        } = l.config.widgets[0].kind
        else {
            panic!()
        };
        let mut actions = Vec::new();
        let menu = l.menu(m, &mut actions).unwrap();
        assert_eq!(
            menu,
            vec![MenuEntry::Input {
                label: Some("Wi-Fi « Maison »".into()),
                placeholder: "Mot de passe".into(),
                secret: true,
                action: Some(0),
            }]
        );
        // L'action reçoit le texte saisi.
        actions[0].call::<()>("s3cret").unwrap();
        assert_eq!(l.render(func).unwrap().text, "s3cret");
    }

    #[test]
    fn widget_inconnu() {
        let e = load(r#"hd.widget("meteo", {})"#).err().unwrap();
        assert!(e.contains("type « meteo » inconnu"), "{e}");
    }
}
