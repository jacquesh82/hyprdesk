//! Configuration en ligne de commande : `hyprdesk config [list|get|set|unset]`.
//!
//! Même effet que l'écran Réglages : la valeur est écrite dans le Lua généré
//! `hyprdesk/settings.lua`, que le démon surveille et recharge aussitôt. Les
//! clés sont rangées par paquet de réglages (mêmes identifiants que
//! `ui::plugins::BUILTIN`) ; un paquet déclare ses clés ici. Ce que fixe
//! `hyprdesk.lua` garde le dernier mot : la clé est alors signalée.

use std::path::Path;

use crate::config::lua::{load_file, load_settings_only};
use crate::config::{
    AppMatch, BarPosition, Config, Fit, LauncherGroup, MAX_WORKSPACES, Mode, Place, Rgba,
    RunningApps, Selector, ShowOn, Transition, WallSource,
};
use crate::settings::{BarSettings, Settings, hex};

/// Une clé réglable : son paquet, son aide, et comment la lire ou l'écrire.
pub struct Key {
    pub name: &'static str,
    /// Paquet de réglages (identifiant de `ui::plugins::BUILTIN`).
    pub plugin: &'static str,
    /// Valeurs admises, puis à quoi sert la clé.
    pub help: &'static str,
    get: fn(&Config) -> String,
    set: fn(&mut Settings, &str, &Config) -> Result<(), String>,
    unset: fn(&mut Settings),
}

fn on_off(b: bool) -> String {
    if b { "on" } else { "off" }.into()
}

pub fn parse_bool(v: &str) -> Result<bool, String> {
    match v.to_lowercase().as_str() {
        "on" | "true" | "yes" | "oui" | "1" => Ok(true),
        "off" | "false" | "no" | "non" | "0" => Ok(false),
        _ => Err(crate::tr!("« {v} » : on ou off attendu", v = v)),
    }
}

fn number<T: std::str::FromStr + PartialOrd + std::fmt::Display + Copy>(
    v: &str,
    min: T,
    max: T,
) -> Result<T, String> {
    let n: T = v
        .parse()
        .map_err(|_| crate::tr!("« {v} » : nombre attendu", v = v))?;
    if n < min || n > max {
        return Err(crate::tr!(
            "« {v} » : entre {min} et {max}",
            v = v,
            min = min,
            max = max
        ));
    }
    Ok(n)
}

fn choice<T>(v: &str, parse: impl Fn(&str) -> Option<T>, allowed: &str) -> Result<T, String> {
    parse(v).ok_or_else(|| {
        crate::tr!(
            "« {v} » : valeurs admises {allowed}",
            v = v,
            allowed = allowed
        )
    })
}

fn combo(v: &str) -> Result<String, String> {
    if matches!(v, "none" | "aucun" | "off" | "") {
        return Ok(String::new());
    }
    crate::hypr::parse_combo(v)
        .map(|_| v.to_string())
        .ok_or_else(|| {
            crate::tr!(
                "« {v} » : raccourci illisible (ex. \"SUPER + SHIFT + D\")",
                v = v
            )
        })
}

fn show_on(s: &str) -> Option<ShowOn> {
    Some(match s {
        "all" => ShowOn::All,
        "focused" => ShowOn::Focused,
        "primary" => ShowOn::Primary,
        _ => return None,
    })
}

fn mode(s: &str) -> Option<Mode> {
    Some(match s {
        "opaque" => Mode::Opaque,
        "glass" => Mode::Glass,
        _ => return None,
    })
}

fn mode_str(m: Mode) -> String {
    match m {
        Mode::Opaque => "opaque",
        Mode::Glass => "glass",
    }
    .into()
}

fn fit(s: &str) -> Option<Fit> {
    Some(match s {
        "cover" => Fit::Cover,
        "contain" => Fit::Contain,
        "fill" => Fit::Fill,
        "center" => Fit::Center,
        _ => return None,
    })
}

/// La barre des réglages, créée depuis la config si elle n'y est pas encore.
fn bar<'a>(s: &'a mut Settings, c: &Config) -> &'a mut BarSettings {
    s.bar.get_or_insert_with(|| BarSettings::from_config(c))
}

/// Fond d'écran des réglages pour tous les bureaux.
fn wall_all(c: &Config) -> String {
    c.wallpapers
        .iter()
        .find(|w| w.sel == Selector::default())
        .map(|w| match &w.source {
            WallSource::Path(p) => p.display().to_string(),
            WallSource::Color(c) => hex(*c),
            WallSource::Rotate { dir, interval, .. } => {
                crate::tr!(
                    "diaporama {p0} ({p1} s)",
                    p0 = dir.display(),
                    p1 = interval.as_secs()
                )
            }
        })
        .unwrap_or_default()
}

macro_rules! keys {
    ($($name:literal, $plugin:literal, $help:literal, |$c:ident| $get:expr, |$s:ident, $v:ident, $cfg:ident| $set:expr, |$u:ident| $unset:expr;)*) => {
        &[$(Key {
            name: $name,
            plugin: $plugin,
            help: $help,
            get: |$c| $get,
            set: |$s, $v, $cfg| { let _ = &$cfg; $set },
            unset: |$u| $unset,
        },)*]
    };
}

/// Clés fixes, rangées par paquet. Les clés dynamiques (`assign.<app>`,
/// `widget.<type>`) sont traitées à part.
pub static KEYS: &[Key] = keys![
    "show_on", "desktop", "all | focused | primary — écrans où le bureau s'affiche",
        |c| c.general.show_on.as_str().into(),
        |s, v, c| { s.show_on = Some(choice(v, show_on, "all, focused, primary")?); Ok(()) },
        |s| s.show_on = None;
    "mode", "desktop", "opaque | glass — fond net, ou flouté et teinté",
        |c| mode_str(c.general.mode),
        |s, v, c| { s.mode = Some(choice(v, mode, "opaque, glass")?); Ok(()) },
        |s| s.mode = None;
    "glass_opacity", "desktop", "0..1 — opacité du voile en mode glass",
        |c| format!("{}", c.general.glass_opacity),
        |s, v, c| { s.glass_opacity = Some(number(v, 0.0f32, 1.0)?); Ok(()) },
        |s| s.glass_opacity = None;
    "app_themes", "apptheme", "kitty,… | none — applications qui prennent le thème de hyprdesk",
        |c| c.general.app_themes.iter().cloned().collect::<Vec<_>>().join(","),
        |s, v, c| {
            let mut set = std::collections::BTreeSet::new();
            if v.trim() != "none" {
                for id in v.split(',').map(str::trim).filter(|x| !x.is_empty()) {
                    if crate::apptheme::target(id).is_none() {
                        return Err(crate::tr!("« {v} » : application inconnue", v = id));
                    }
                    set.insert(id.to_string());
                }
            }
            s.app_themes = Some(set);
            Ok(())
        },
        |s| s.app_themes = None;
    "advanced_settings", "desktop", "on | off — écran Réglages : tous les détails (off : l'essentiel)",
        |c| on_off(c.general.advanced_settings),
        |s, v, c| { s.advanced_settings = Some(parse_bool(v)?); Ok(()) },
        |s| s.advanced_settings = None;
    "close_on_launch", "desktop", "on | off — masquer le bureau après un lancement",
        |c| on_off(c.general.close_on_launch),
        |s, v, c| { s.close_on_launch = Some(parse_bool(v)?); Ok(()) },
        |s| s.close_on_launch = None;
    "float_launched", "desktop", "on | off — fenêtres ouvertes depuis le bureau : flottantes",
        |c| on_off(c.general.float_launched),
        |s, v, c| { s.float_launched = Some(parse_bool(v)?); Ok(()) },
        |s| s.float_launched = None;
    "suspend_binds", "desktop", "on | off — full-desktop : suspendre les raccourcis de Hyprland",
        |c| on_off(c.general.suspend_binds),
        |s, v, c| { s.suspend_binds = Some(parse_bool(v)?); Ok(()) },
        |s| s.suspend_binds = None;
    "font", "desktop", "nom de famille — police de l'interface",
        |c| c.general.font.clone().unwrap_or_default(),
        |s, v, c| { s.font = Some(v.to_string()); Ok(()) },
        |s| s.font = None;
    "language", "desktop", "auto | fr | en — langue de l'interface (auto : celle du système)",
        |c| c.general.language.clone(),
        |s, v, c| {
            let l = v.trim().to_lowercase();
            if !["auto", "fr", "en"].contains(&l.as_str()) {
                return Err(crate::tr!("« {v} » : auto, fr ou en", v = v));
            }
            s.language = Some(l);
            Ok(())
        },
        |s| s.language = None;
    "layout", "layouts", "dwindle | master | scrolling | monocle | grid | columns | rows | centered | lua:<nom> | none — layout par défaut",
        |c| c.general.tiling.layout.clone().unwrap_or_default(),
        |s, v, c| {
            let v = v.trim();
            let mut t = s.tiling.clone().unwrap_or_else(|| c.general.tiling.clone());
            if v == "none" {
                t.layout = None;
            } else if crate::tiling::valid_name(v) {
                t.layout = Some(v.to_string());
            } else {
                return Err(crate::tr!("« {v} » : layout inconnu", v = v));
            }
            s.tiling = Some(t);
            Ok(())
        },
        |s| {
            if let Some(t) = s.tiling.as_mut() {
                t.layout = None;
            }
        };
    "font_size", "desktop", "6..96 — taille du texte",
        |c| format!("{}", c.general.font_size),
        |s, v, c| { s.font_size = Some(number(v, 6.0f32, 96.0)?); Ok(()) },
        |s| s.font_size = None;
    "keys.toggle", "desktop", "raccourci | none — full-desktop (SUPER + D)",
        |c| c.general.keys.toggle.clone(),
        |s, v, c| { s.key_toggle = Some(combo(v)?); Ok(()) },
        |s| s.key_toggle = None;
    "keys.launcher", "desktop", "raccourci | none — launcher-only (SUPER + ALT + D)",
        |c| c.general.keys.launcher.clone(),
        |s, v, c| { s.key_launcher = Some(combo(v)?); Ok(()) },
        |s| s.key_launcher = None;
    "keys.hide", "desktop", "raccourci | none — no-desktop (SUPER + CTRL + D)",
        |c| c.general.keys.hide.clone(),
        |s, v, c| { s.key_hide = Some(combo(v)?); Ok(()) },
        |s| s.key_hide = None;
    "keys.settings", "desktop", "raccourci | none — ouvrir les réglages (SUPER + SHIFT + D)",
        |c| c.general.keys.settings.clone(),
        |s, v, c| { s.key_settings = Some(combo(v)?); Ok(()) },
        |s| s.key_settings = None;
    "keys.lock", "desktop", "raccourci | none — verrouiller la session ; none : celui de hyprland.lua (SUPER + L), réglé : hyprdesk le remplace",
        |c| c.general.keys.lock.clone(),
        |s, v, c| { s.key_lock = Some(combo(v)?); Ok(()) },
        |s| s.key_lock = None;
    "keys.leave", "desktop", "raccourci | none — sortie de secours",
        |c| c.general.keys.leave.clone(),
        |s, v, c| { s.key_leave = Some(combo(v)?); Ok(()) },
        |s| s.key_leave = None;
    "keys.media", "desktop", "on | off — touches de volume, de luminosité et de lecture gérées par hyprdesk (avec affichage)",
        |c| on_off(c.general.keys.media),
        |s, v, c| { s.key_media = Some(parse_bool(v)?); Ok(()) },
        |s| s.key_media = None;
    "grid.icon_size", "icons", "8..1024 — taille des icônes du bureau",
        |c| c.general.grid.icon_size.to_string(),
        |s, v, c| { s.icon_size = Some(number(v, 8u16, 1024)?); Ok(()) },
        |s| s.icon_size = None;
    "grid.cell", "icons", "16..1024 — taille des cases",
        |c| c.general.grid.cell.to_string(),
        |s, v, c| { s.cell = Some(number(v, 16u16, 1024)?); Ok(()) },
        |s| s.cell = None;
    "icon_theme", "icons", "nom — thème d'icônes",
        |c| c.general.icon_theme.clone().unwrap_or_default(),
        |s, v, c| { s.icon_theme = Some(v.to_string()); Ok(()) },
        |s| s.icon_theme = None;
    "running_apps", "launcher", "off | workspace | all — fenêtres du lanceur",
        |c| c.general.running_apps.as_str().into(),
        |s, v, c| { s.running_apps = Some(choice(v, RunningApps::parse, "off, workspace, all")?); Ok(()) },
        |s| s.running_apps = None;
    "launcher_group", "launcher", "none | app | workspace — regroupement du lanceur",
        |c| c.general.launcher_group.as_str().into(),
        |s, v, c| { s.launcher_group = Some(choice(v, LauncherGroup::parse, "none, app, workspace")?); Ok(()) },
        |s| s.launcher_group = None;
    "launcher_position", "launcher", "bottom | top | left | right — bord du lanceur",
        |c| c.general.launcher.position.as_str().into(),
        |s, v, c| { s.launcher_position = Some(choice(v, crate::config::LauncherPosition::parse, "bottom, top, left, right")?); Ok(()) },
        |s| s.launcher_position = None;
    "launcher_size", "launcher", "24..96 — taille des icônes du lanceur",
        |c| c.general.launcher.size.to_string(),
        |s, v, c| { s.launcher_size = Some(number(v, 24u16, 96)?); Ok(()) },
        |s| s.launcher_size = None;
    "launcher_pinned", "launcher", "app,app,… | none — applications épinglées (ids .desktop)",
        |c| c.general.launcher.pinned.join(","),
        |s, v, c| {
            s.launcher_pinned = Some(if v == "none" { Vec::new() } else {
                v.split(',').map(|a| a.trim().trim_end_matches(".desktop").to_string()).filter(|a| !a.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.launcher_pinned = None;
    "launcher_show_settings", "launcher", "on | off — bouton des réglages dans le lanceur",
        |c| on_off(c.general.launcher.show_settings),
        |s, v, c| { s.launcher_show_settings = Some(parse_bool(v)?); Ok(()) },
        |s| s.launcher_show_settings = None;
    "launcher_show_assistant", "launcher", "on | off — bouton de l'assistant IA dans le lanceur",
        |c| on_off(c.general.launcher.show_assistant),
        |s, v, c| { s.launcher_show_assistant = Some(parse_bool(v)?); Ok(()) },
        |s| s.launcher_show_assistant = None;
    "lock_blur", "lock", "on | off — fond flouté sur l'écran de verrouillage",
        |c| on_off(c.general.lock.blur),
        |s, v, c| { s.lock_blur = Some(parse_bool(v)?); Ok(()) },
        |s| s.lock_blur = None;
    "lock_dim", "lock", "0..0.9 — assombrissement du fond verrouillé",
        |c| format!("{}", c.general.lock.dim),
        |s, v, c| { s.lock_dim = Some(number(v, 0.0f32, 0.9)?); Ok(()) },
        |s| s.lock_dim = None;
    "lock_image", "lock", "chemin | none — image propre au verrouillage (sinon le fond du bureau)",
        |c| c.general.lock.image.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
        |s, v, c| {
            s.lock_image = if v == "none" { None } else {
                let p = crate::config::lua::expand_path(v, Path::new("/"));
                if !p.is_file() { return Err(crate::tr!("« {v} » : image introuvable", v = v)); }
                Some(p)
            };
            Ok(())
        },
        |s| s.lock_image = None;
    "lock_show_user", "lock", "on | off — nom et avatar",
        |c| on_off(c.general.lock.show_user),
        |s, v, c| { s.lock_show_user = Some(parse_bool(v)?); Ok(()) },
        |s| s.lock_show_user = None;
    "lock_show_battery", "lock", "on | off — batterie",
        |c| on_off(c.general.lock.show_battery),
        |s, v, c| { s.lock_show_battery = Some(parse_bool(v)?); Ok(()) },
        |s| s.lock_show_battery = None;
    "lock_show_media", "lock", "on | off — morceau en cours, lecture / pause",
        |c| on_off(c.general.lock.show_media),
        |s, v, c| { s.lock_show_media = Some(parse_bool(v)?); Ok(()) },
        |s| s.lock_show_media = None;
    "idle_manage", "lock", "on | off — hyprdesk gère la veille (lance hypridle avec ses délais)",
        |c| on_off(c.general.idle.manage),
        |s, v, c| { s.idle_manage = Some(parse_bool(v)?); Ok(()) },
        |s| s.idle_manage = None;
    "idle_dim", "lock", "0..86400 s — assombrir l'écran après inactivité (0 : jamais)",
        |c| c.general.idle.dim.to_string(),
        |s, v, c| { s.idle_dim = Some(number(v, 0u32, 86_400)?); Ok(()) },
        |s| s.idle_dim = None;
    "idle_lock", "lock", "0..86400 s — verrouiller après inactivité (0 : jamais)",
        |c| c.general.idle.lock.to_string(),
        |s, v, c| { s.idle_lock = Some(number(v, 0u32, 86_400)?); Ok(()) },
        |s| s.idle_lock = None;
    "idle_dpms", "lock", "0..86400 s — éteindre les écrans après inactivité (0 : jamais)",
        |c| c.general.idle.dpms.to_string(),
        |s, v, c| { s.idle_dpms = Some(number(v, 0u32, 86_400)?); Ok(()) },
        |s| s.idle_dpms = None;
    "idle_suspend", "lock", "0..86400 s — mettre en veille après inactivité (0 : jamais)",
        |c| c.general.idle.suspend.to_string(),
        |s, v, c| { s.idle_suspend = Some(number(v, 0u32, 86_400)?); Ok(()) },
        |s| s.idle_suspend = None;
    "assistant_mode", "assistant", "window | terminal — ouverture de LYRA (fournisseur : apps.ai)",
        |c| c.general.assistant_mode.as_str().into(),
        |s, v, c| { s.assistant_mode = Some(choice(v, crate::assistant::AssistantMode::parse, "window, terminal")?); Ok(()) },
        |s| s.assistant_mode = None;
    "wallpaper", "wallpaper", "chemin d'image | #couleur — fond de tous les bureaux",
        |c| wall_all(c),
        |s, v, c| {
            let source = match Rgba::parse(v) {
                Some(col) => WallSource::Color(col),
                None => {
                    let p = crate::config::lua::expand_path(v, Path::new("/"));
                    if !p.is_file() {
                        return Err(crate::tr!("« {v} » : ni une image existante, ni une couleur", v = v));
                    }
                    WallSource::Path(p)
                }
            };
            let keep = s.wallpaper(&Selector::default()).map(|w| w.fit).unwrap_or_default();
            s.set_wallpaper(Selector::default(), source, keep);
            Ok(())
        },
        |s| s.remove_wallpaper(&Selector::default());
    "wallpaper.fit", "wallpaper", "cover | contain | fill | center — ajustement du fond",
        |c| c.wallpapers.iter().find(|w| w.sel == Selector::default())
            .map(|w| crate::settings::fit_str(w.fit).to_string()).unwrap_or_default(),
        |s, v, c| {
            let f = choice(v, fit, "cover, contain, fill, center")?;
            let Some(w) = s.wallpaper(&Selector::default()).cloned() else {
                return Err(crate::i18n::tr("aucun fond réglé pour tous les bureaux (hyprdesk config set wallpaper …)").into());
            };
            s.set_wallpaper(w.sel, w.source, f);
            Ok(())
        },
        |s| {
            if let Some(w) = s.wallpaper(&Selector::default()).cloned() {
                s.set_wallpaper(w.sel, w.source, Fit::default());
            }
        };
    "wallpaper.span", "wallpaper", "on | lock | desktop | off — étaler l'image sur tous les écrans (lock : au verrouillage seulement)",
        |c| c.wallpapers.iter().find(|w| w.sel == Selector::default())
            .map(|w| match (w.span, w.span_lock) {
                (true, true) => "on".to_string(),
                (false, true) => "lock".to_string(),
                (true, false) => "desktop".to_string(),
                (false, false) => "off".to_string(),
            }).unwrap_or_default(),
        |s, v, c| {
            let (span, span_lock) = match v.trim() {
                "lock" => (false, true),
                "desktop" => (true, false),
                other => { let on = parse_bool(other)?; (on, on) }
            };
            match s.wallpaper(&Selector::default()) {
                None => return Err(crate::i18n::tr("aucun fond réglé pour tous les bureaux (hyprdesk config set wallpaper …)").into()),
                Some(w) if (span || span_lock) && matches!(w.source, WallSource::Color(_)) => {
                    return Err(crate::i18n::tr("une couleur ne s'étale pas : choisir une image ou un diaporama").into());
                }
                Some(_) => s.set_wallpaper_span(&Selector::default(), span, span_lock),
            }
            Ok(())
        },
        |s| s.set_wallpaper_span(&Selector::default(), false, false);
    "bar", "bar", "on | off — barre native",
        |c| on_off(c.bar.is_some()),
        |s, v, c| { bar(s, c).enabled = parse_bool(v)?; Ok(()) },
        |s| s.bar = None;
    "bar.position", "bar", "top | bottom — bord de la barre",
        |c| c.bar.as_ref().map(|b| if b.position == BarPosition::Top { "top" } else { "bottom" }.to_string()).unwrap_or_default(),
        |s, v, c| {
            bar(s, c).position = match v { "top" => BarPosition::Top, "bottom" => BarPosition::Bottom, _ => return Err(crate::tr!("« {v} » : top ou bottom", v = v)) };
            Ok(())
        },
        |s| { if let Some(b) = s.bar.as_mut() { b.position = BarPosition::Top; } };
    "bar.height", "bar", "24..64 — hauteur de la barre",
        |c| c.bar.as_ref().map(|b| b.height.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).height = number(v, 24u16, 64)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.height = BarSettings::default().height; } };
    "bar.monitor", "bar", "all | primary | <écran> — écrans de la barre",
        |c| c.bar.as_ref().map(|b| b.monitors.as_ref().map(|l| l.join(",")).unwrap_or_else(|| "all".into())).unwrap_or_default(),
        |s, v, c| {
            bar(s, c).monitors = match v { "all" | "*" => None, other => Some(other.split(',').map(|m| m.trim().to_string()).collect()) };
            Ok(())
        },
        |s| { if let Some(b) = s.bar.as_mut() { b.monitors = None; } };
    "bar.compact", "bar", "auto | always | never — barre compacte (auto : écrans étroits ou verticaux)",
        |c| c.bar.as_ref().map(|b| b.compact.as_str().to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).compact = choice(v, crate::config::BarCompact::parse, "auto, always, never")?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.compact = Default::default(); } };
    "primary", "screens", "<écran> | none — écran principal (ancre 0×0, bureau, barre, réglages)",
        |c| c.general.primary.clone().unwrap_or_default(),
        |s, v, c| { s.primary = (!matches!(v, "none" | "")).then(|| v.to_string()); Ok(()) },
        |s| s.primary = None;
    "workspaces", "workspaces", "1..30 | off — workspaces permanents",
        |c| c.general.workspaces.map(|n| n.to_string()).unwrap_or_else(|| "off".into()),
        |s, v, c| {
            s.workspaces = if v == "off" { None } else { Some(number(v, 1u8, MAX_WORKSPACES)?) };
            Ok(())
        },
        |s| s.workspaces = None;
    "workspace_dedicated", "workspaces", "écran,écran,… | none — écrans dédiés (seulement leurs workspaces)",
        |c| c.general.workspace_dedicated.iter().cloned().collect::<Vec<_>>().join(","),
        |s, v, c| {
            s.workspace_dedicated = Some(if v == "none" { Default::default() } else {
                v.split(',').map(|m| m.trim().to_string()).filter(|m| !m.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.workspace_dedicated = None;
    "transition", "effects", "fade | slide | none — apparition du bureau",
        |c| c.general.transition.as_str().into(),
        |s, v, c| { s.transition = Some(choice(v, Transition::parse, "fade, slide, none")?); Ok(()) },
        |s| s.transition = None;
    "fade_ms", "effects", "0..5000 — durée de la transition",
        |c| c.general.fade_ms.to_string(),
        |s, v, c| { s.fade_ms = Some(number(v, 0u64, 5000)?); Ok(()) },
        |s| s.fade_ms = None;
    "crossfade_ms", "effects", "0..10000 — fondu entre deux fonds",
        |c| c.general.crossfade_ms.to_string(),
        |s, v, c| { s.crossfade_ms = Some(number(v, 0u64, 10000)?); Ok(()) },
        |s| s.crossfade_ms = None;
    "theme.preset", "theme", "nom d'un thème livré — applique ses couleurs et ses effets",
        |c| c.general.theme.name.clone(),
        |s, v, c| {
            let p = crate::theme::PRESETS.iter().find(|p| p.name.eq_ignore_ascii_case(v)).ok_or_else(|| {
                let all: Vec<&str> = crate::theme::PRESETS.iter().map(|p| p.name).collect();
                crate::tr!("« {v} » : thèmes livrés {p0}", v = v, p0 = all.join(", "))
            })?;
            s.apply_preset(p);
            Ok(())
        },
        |s| { s.theme_name = None; s.fg = None; s.bg = None; s.accent = None; s.effects.clear(); };
    "theme.name", "theme", "texte — nom du thème",
        |c| c.general.theme.name.clone(),
        |s, v, c| { s.theme_name = Some(v.to_string()); Ok(()) },
        |s| s.theme_name = None;
    "theme.fg", "theme", "#rrggbb — couleur du texte",
        |c| hex(c.general.theme.fg),
        |s, v, c| { s.fg = Some(color_value(v)?); Ok(()) },
        |s| s.fg = None;
    "theme.bg", "theme", "#rrggbb — couleur des surfaces",
        |c| hex(c.general.theme.bg),
        |s, v, c| { s.bg = Some(color_value(v)?); Ok(()) },
        |s| s.bg = None;
    "theme.accent", "theme", "#rrggbb — couleur d'accent",
        |c| hex(c.general.theme.accent),
        |s, v, c| { s.accent = Some(color_value(v)?); Ok(()) },
        |s| s.accent = None;
    "background", "theme", "#rrggbb — fond quand aucune image ne s'applique",
        |c| hex(c.general.background),
        |s, v, c| { s.background = Some(color_value(v)?); Ok(()) },
        |s| s.background = None;
];

fn color_value(v: &str) -> Result<Rgba, String> {
    Rgba::parse(v).ok_or_else(|| crate::tr!("« {v} » : couleur illisible (#rrggbb)", v = v))
}

/// Effet d'une clé `theme.effects.<endroit>.<opacity|blur>`.
fn effect_key(key: &str) -> Option<(Place, &str)> {
    let rest = key.strip_prefix("theme.effects.")?;
    let (place, field) = rest.split_once('.')?;
    let place = Place::parse(place)?;
    matches!(field, "opacity" | "blur").then_some((place, field))
}

/// Clés dynamiques : `apps.<rôle>`, `assign.<app>` (workspaces), `widget.<type>`
/// (widgets) et `theme.effects.<endroit>.<opacity|blur>` (thème).
fn dynamic_get(key: &str, c: &Config) -> Option<String> {
    if let Some((place, field)) = effect_key(key) {
        let e = c.general.theme.effect(place);
        return Some(if field == "blur" {
            on_off(e.blur)
        } else {
            format!("{}", (e.opacity * 100.0).round() / 100.0)
        });
    }
    // Layouts : `layout.<workspace>`, `<dwindle|master|scrolling>.<option>`.
    if let Some(ws) = key.strip_prefix("layout.") {
        return Some(
            c.general
                .tiling
                .workspaces
                .get(ws)
                .cloned()
                .unwrap_or_default(),
        );
    }
    if let Some(o) = crate::tiling::opt(key) {
        return Some(
            c.general
                .tiling
                .options
                .get(&o.path())
                .map(crate::tiling::OptValue::display)
                .unwrap_or_default(),
        );
    }
    if let Some(ws) = key.strip_prefix("workspace.") {
        return Some(
            c.general
                .workspace_monitors
                .iter()
                .find(|(_, l)| l.iter().any(|w| w == ws))
                .map_or("auto".to_string(), |(m, _)| m.clone()),
        );
    }
    if let Some(id) = key.strip_prefix("plugin.") {
        crate::ui::plugins::find(id)?;
        return Some(on_off(c.plugin_on(id)));
    }
    if let Some(r) = key.strip_prefix("apps.") {
        let role = crate::apps::role(r)?;
        return Some(c.general.apps.get(r).cloned().unwrap_or_else(|| {
            crate::apps::system_default(role)
                .map(|v| crate::tr!("{v} (système)", v = v))
                .unwrap_or_default()
        }));
    }
    if let Some(app) = key.strip_prefix("assign.") {
        let m = AppMatch::App(app.to_string());
        return Some(
            c.assigns
                .iter()
                .find(|a| a.app == m)
                .map(|a| format!("{}{}", a.workspace, if a.silent { " silent" } else { "" }))
                .unwrap_or_default(),
        );
    }
    if let Some(t) = key.strip_prefix("widget.") {
        return Some(on_off(
            c.widgets.iter().any(|w| w.type_id == t && !w.in_bar),
        ));
    }
    None
}

fn dynamic_set(
    key: &str,
    v: Option<&str>,
    s: &mut Settings,
    c: &Config,
) -> Option<Result<(), String>> {
    if let Some((place, field)) = effect_key(key) {
        let Some(v) = v else {
            s.effects.remove(&place);
            return Some(Ok(()));
        };
        let mut e = s
            .effects
            .get(&place)
            .copied()
            .unwrap_or_else(|| c.general.theme.effect(place));
        match field {
            "blur" if !place.blurrable() => {
                return Some(Err(crate::tr!(
                    "{p0} : pas de flou propre (dessinés sur le fond d'écran)",
                    p0 = place.label()
                )));
            }
            "blur" => match parse_bool(v) {
                Ok(b) => e.blur = b,
                Err(err) => return Some(Err(err)),
            },
            _ => match number(v, 0.0f32, 1.0) {
                Ok(n) => e.opacity = n,
                Err(err) => return Some(Err(err)),
            },
        }
        s.effects.insert(place, e);
        return Some(Ok(()));
    }
    if let Some(ws) = key.strip_prefix("layout.") {
        let mut t = s.tiling.clone().unwrap_or_else(|| c.general.tiling.clone());
        match v.map(str::trim) {
            None | Some("none") | Some("") => {
                t.workspaces.remove(ws);
            }
            Some(l) if crate::tiling::valid_name(l) => {
                t.workspaces.insert(ws.to_string(), l.to_string());
            }
            Some(l) => return Some(Err(crate::tr!("« {v} » : layout inconnu", v = l))),
        }
        s.tiling = Some(t);
        return Some(Ok(()));
    }
    if let Some(o) = crate::tiling::opt(key) {
        let mut t = s.tiling.clone().unwrap_or_else(|| c.general.tiling.clone());
        match v.map(str::trim) {
            None | Some("none") | Some("") => {
                t.options.remove(&o.path());
            }
            Some(raw) => match o.parse(raw) {
                Ok(val) => {
                    t.options.insert(o.path(), val);
                }
                Err(e) => return Some(Err(e)),
            },
        }
        s.tiling = Some(t);
        return Some(Ok(()));
    }
    if let Some(ws) = key.strip_prefix("workspace.") {
        let mut map = s
            .workspace_monitors
            .clone()
            .unwrap_or_else(|| c.general.workspace_monitors_declared.clone());
        for l in map.values_mut() {
            l.retain(|w| w != ws);
        }
        match v.map(str::trim) {
            None | Some("auto") | Some("") => {}
            Some(mon) => {
                let l = map.entry(mon.to_string()).or_default();
                l.push(ws.to_string());
                l.sort_by_key(|w| w.parse::<i64>().unwrap_or(i64::MAX));
            }
        }
        map.retain(|_, l| !l.is_empty());
        s.workspace_monitors = Some(map);
        return Some(Ok(()));
    }
    if let Some(id) = key.strip_prefix("plugin.") {
        if crate::ui::plugins::find(id).is_none() {
            return Some(Err(crate::tr!(
                "paquet « {id} » inconnu (hyprdesk config list)",
                id = id
            )));
        }
        if crate::config::CORE_PLUGINS.contains(&id) {
            return Some(Err(crate::tr!(
                "« {id} » fait partie du cœur : toujours actif",
                id = id
            )));
        }
        let on = match v.map(parse_bool) {
            None => true,
            Some(Ok(b)) => b,
            Some(Err(e)) => return Some(Err(e)),
        };
        let mut set = s
            .disabled_plugins
            .clone()
            .unwrap_or_else(|| c.general.disabled_plugins.clone());
        if on {
            set.remove(id);
        } else {
            set.insert(id.to_string());
        }
        s.disabled_plugins = Some(set);
        return Some(Ok(()));
    }
    if let Some(r) = key.strip_prefix("apps.") {
        let Some(role) = crate::apps::role(r) else {
            let all: Vec<&str> = crate::apps::ROLES.iter().map(|r| r.id).collect();
            return Some(Err(crate::tr!(
                "rôle « {r} » inconnu ({p0})",
                r = r,
                p0 = all.join(", ")
            )));
        };
        return Some(match v.map(str::trim) {
            None | Some("none") | Some("") => {
                s.apps.remove(r);
                Ok(())
            }
            Some(v) => {
                let v = v.trim_end_matches(".desktop").to_string();
                s.apps.insert(r.to_string(), v.clone());
                // Comme depuis l'écran : l'application devient celle du système.
                crate::apps::apply_system(role, &v).map(|_| ())
            }
        });
    }
    if let Some(app) = key.strip_prefix("assign.") {
        let m = AppMatch::App(app.to_string());
        return Some(match v.map(str::trim) {
            None | Some("none") | Some("") => {
                s.assigns.retain(|a| a.app != m);
                Ok(())
            }
            Some(v) => {
                let mut parts = v.split_whitespace();
                let ws = parts.next().unwrap_or_default();
                let silent = parts.next() == Some("silent");
                s.set_assign(m, ws, silent);
                Ok(())
            }
        });
    }
    if let Some(t) = key.strip_prefix("widget.") {
        let Some(ty) = c.catalog.iter().find(|w| w.id == t) else {
            let known: Vec<&str> = c.catalog.iter().map(|w| w.id.as_str()).collect();
            return Some(Err(crate::tr!(
                "widget « {t} » inconnu ({p0})",
                t = t,
                p0 = known.join(", ")
            )));
        };
        let on = match v {
            None => false,
            Some(v) => match parse_bool(v) {
                Ok(b) => b,
                Err(e) => return Some(Err(e)),
            },
        };
        let user_ids: Vec<String> = c
            .widgets
            .iter()
            .filter(|w| w.type_id == t && w.origin == crate::config::Origin::User && !w.in_bar)
            .map(|w| w.id.clone())
            .collect();
        let taken: Vec<String> = c.widgets.iter().map(|w| w.id.clone()).collect();
        s.set_widget_type(t, on, ty.anchor, &user_ids, &taken);
        return Some(Ok(()));
    }
    None
}

/// Clé surchargée par hyprdesk.lua (sans effet depuis les réglages).
fn overridden(key: &str, c: &Config) -> bool {
    match effect_key(key) {
        Some((place, _)) => c
            .keys_user
            .contains(&format!("theme.effects.{p0}", p0 = place.as_str())),
        None => c.keys_user.contains(key),
    }
}

/// Libellé d'un paquet de réglages, depuis le registre de l'interface.
fn plugin_label(id: &str) -> &'static str {
    crate::ui::plugins::BUILTIN
        .iter()
        .find(|p| p.id == id)
        .map_or("", |p| crate::i18n::tr(p.label))
}

fn usage() -> String {
    crate::i18n::tr_lines(USAGE)
}

const USAGE: &str =
    "Usage : hyprdesk config [list [paquet]] | get <clé> | set <clé> <valeur> | unset <clé>
Clés dynamiques : assign.<app> <workspace> [silent] | none ; widget.<type> on | off";

pub fn run(config_path: &Path, args: &[String]) -> Result<String, String> {
    let full = || load_file(config_path).map(|l| l.config);
    match args.first().map(String::as_str).unwrap_or("list") {
        "list" | "ls" => {
            let c = full()?;
            let only = args.get(1).map(String::as_str);
            let mut out = String::new();
            for p in crate::ui::plugins::BUILTIN {
                if only.is_some_and(|o| o != p.id) {
                    continue;
                }
                let keys: Vec<&Key> = KEYS.iter().filter(|k| k.plugin == p.id).collect();
                let extra = match p.id {
                    // Écran de chaque workspace placé, puis affectations d'applications.
                    "workspaces" => c
                        .general
                        .workspace_monitors
                        .values()
                        .flatten()
                        .map(|w| format!("workspace.{w}", w = w))
                        .chain(c.assigns.iter().filter_map(|a| match &a.app {
                            AppMatch::App(id) => Some(format!("assign.{id}", id = id)),
                            AppMatch::Class(_) => None,
                        }))
                        .collect::<Vec<_>>(),
                    "widgets" => c
                        .catalog
                        .iter()
                        .map(|w| format!("widget.{p0}", p0 = w.id))
                        .collect(),

                    "layouts" => (1..=c.general.workspaces.unwrap_or(10).max(1))
                        .map(|w| format!("layout.{w}"))
                        .chain(crate::tiling::OPTIONS.iter().map(|o| o.path()))
                        .collect(),
                    "plugins" => crate::ui::plugins::BUILTIN
                        .iter()
                        .filter(|p| !crate::config::CORE_PLUGINS.contains(&p.id))
                        .map(|p| format!("plugin.{p0}", p0 = p.id))
                        .collect(),
                    "apps" => crate::apps::ROLES
                        .iter()
                        .map(|r| format!("apps.{p0}", p0 = r.id))
                        .collect(),
                    "theme" => Place::ALL
                        .iter()
                        .flat_map(|p| {
                            let o = format!("theme.effects.{p0}.opacity", p0 = p.as_str());
                            let b = format!("theme.effects.{p0}.blur", p0 = p.as_str());
                            if p.blurrable() { vec![o, b] } else { vec![o] }
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                if keys.is_empty() && extra.is_empty() {
                    continue;
                }
                out += &format!("[{}] {}\n", p.id, crate::i18n::tr(p.label));
                for k in keys {
                    let v = (k.get)(&c);
                    out += &format!(
                        "  {:<18} = {:<26} {}{}\n",
                        k.name,
                        if v.is_empty() { "—".into() } else { v },
                        crate::i18n::tr(k.help),
                        if overridden(k.name, &c) {
                            crate::i18n::tr("  (fixé dans hyprdesk.lua)")
                        } else {
                            ""
                        }
                    );
                }
                for name in extra {
                    let v = dynamic_get(&name, &c).unwrap_or_default();
                    out += &format!("  {name:<18} = {v}\n");
                }
            }
            if out.is_empty() {
                return Err(crate::tr!(
                    "paquet « {p0} » sans clés réglables",
                    p0 = only.unwrap_or("")
                ));
            }
            Ok(out.trim_end().to_string())
        }
        "get" => {
            let key = args.get(1).ok_or_else(usage)?;
            let c = full()?;
            KEYS.iter()
                .find(|k| k.name == key)
                .map(|k| (k.get)(&c))
                .or_else(|| dynamic_get(key, &c))
                .ok_or_else(|| {
                    crate::tr!("clé « {key} » inconnue (hyprdesk config list)", key = key)
                })
        }
        cmd @ ("set" | "unset") => {
            let key = args.get(1).ok_or_else(usage)?;
            let value = (cmd == "set").then(|| args[2..].join(" "));
            if cmd == "set" && args.len() < 3 {
                return Err(usage());
            }
            let c = full()?;
            let mut s = Settings::from_config(&load_settings_only(config_path)?);
            match (KEYS.iter().find(|k| k.name == key), value.as_deref()) {
                (Some(k), Some(v)) => (k.set)(&mut s, v.trim(), &c)?,
                (Some(k), None) => (k.unset)(&mut s),
                (None, v) => dynamic_set(key, v, &mut s, &c).ok_or_else(|| {
                    crate::tr!("clé « {key} » inconnue (hyprdesk config list)", key = key)
                })??,
            }
            s.save_versioned(config_path, crate::storage::revision())
                .map_err(|e| crate::tr!("settings.lua : {e}", e = e))?;
            let after = full()?;
            let now = KEYS
                .iter()
                .find(|k| k.name == key)
                .map(|k| (k.get)(&after))
                .or_else(|| dynamic_get(key, &after))
                .unwrap_or_default();
            let mut msg = format!("{key} = {}", if now.is_empty() { "—" } else { &now });
            if overridden(key, &after) {
                msg += crate::i18n::tr("  (fixé dans hyprdesk.lua : sans effet tant qu'il y est)");
            }
            if let Some(p) = KEYS.iter().find(|k| k.name == key) {
                msg += &format!("\n[{}] {}", p.plugin, plugin_label(p.plugin));
            }
            Ok(msg)
        }
        "-h" | "--help" | "help" => Ok(usage()),
        other => Err(crate::tr!(
            "« {other} » : {p0}",
            other = other,
            p0 = usage()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Aide de `hyprdesk config` : usage et description de chaque clé traduits.
    #[test]
    fn aide_config_traduite() {
        let items = USAGE.lines().chain(KEYS.iter().map(|k| k.help));
        let missing = crate::i18n::untranslated("config", items);
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }

    fn tmp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "hyprdesk-cli-{}-{}",
            std::process::id(),
            crate::storage::revision()
        ));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("hyprdesk.lua");
        std::fs::write(&p, "hd.config({ fade_ms = 50 })\nhd.widget(\"clock\")\n").unwrap();
        p
    }

    fn run_s(p: &Path, a: &[&str]) -> Result<String, String> {
        run(p, &a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn regler_lire_retirer() {
        let p = tmp();
        assert_eq!(run_s(&p, &["get", "show_on"]).unwrap(), "all");
        run_s(&p, &["set", "show_on", "primary"]).unwrap();
        run_s(&p, &["set", "primary", "DP-1"]).unwrap();
        run_s(&p, &["set", "launcher_group", "app"]).unwrap();
        run_s(&p, &["set", "keys.launcher", "SUPER", "+", "ALT", "+", "L"]).unwrap();
        run_s(&p, &["set", "assign.firefox", "2", "silent"]).unwrap();
        run_s(&p, &["set", "widget.clock", "off"]).unwrap();
        assert_eq!(run_s(&p, &["get", "show_on"]).unwrap(), "primary");
        assert_eq!(
            run_s(&p, &["get", "keys.launcher"]).unwrap(),
            "SUPER + ALT + L"
        );
        assert_eq!(run_s(&p, &["get", "assign.firefox"]).unwrap(), "2 silent");
        assert_eq!(run_s(&p, &["get", "widget.clock"]).unwrap(), "off");
        // Valeurs refusées, clé inconnue, clé fixée par l'utilisateur.
        assert!(run_s(&p, &["set", "show_on", "partout"]).is_err());
        assert!(run_s(&p, &["set", "workspaces", "99"]).is_err());
        assert!(run_s(&p, &["get", "nope"]).is_err());
        assert!(
            run_s(&p, &["set", "fade_ms", "300"])
                .unwrap()
                .contains("hyprdesk.lua")
        );
        // Retirer rend la valeur par défaut, et la liste range par paquet.
        run_s(&p, &["unset", "show_on"]).unwrap();
        run_s(&p, &["unset", "assign.firefox"]).unwrap();
        assert_eq!(run_s(&p, &["get", "show_on"]).unwrap(), "all");
        assert_eq!(run_s(&p, &["get", "assign.firefox"]).unwrap(), "");
        let list = run_s(&p, &["list"]).unwrap();
        assert!(list.contains("[launcher] Lanceur") && list.contains("launcher_group"));
        assert!(run_s(&p, &["list", "screens"]).unwrap().contains("primary"));
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn chaque_cle_appartient_a_un_paquet() {
        let ids: Vec<&str> = crate::ui::plugins::BUILTIN.iter().map(|p| p.id).collect();
        for k in KEYS {
            assert!(ids.contains(&k.plugin), "{} → {}", k.name, k.plugin);
        }
    }
}
