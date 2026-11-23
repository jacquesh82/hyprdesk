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
    AppMatch, BarPosition, Config, Fit, LauncherGroup, LidAction, MAX_WORKSPACES, Mode, Place,
    Rgba, RunningApps, Selector, ShowOn, Transition, WallSource,
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

impl Key {
    /// Valeur de la clé dans `c`, telle que `hyprdesk config get` l'écrit.
    pub fn value(&self, c: &Config) -> String {
        (self.get)(c)
    }
}

/// `hyprdesk config reset` sans composant : ce que veut dire « par défaut »
/// pour chacun.
fn defaults_help() -> String {
    let mut out = crate::i18n::tr(
        "hyprdesk config reset <composant|all> [--yes] : remet un composant (ou tout) à sa configuration par défaut.\nPar défaut : ses réglages de settings.lua rendus à ceux du bureau de départ, les fichiers de Hyprland qu'il tient réécrits, son état mémorisé oublié. hyprdesk.lua n'est jamais touché.\n",
    )
    .to_string();
    for p in crate::defaults::components() {
        out += &format!("\n  {:<12} {}", p.id, crate::i18n::tr(p.label));
        let held: Vec<&str> = crate::defaults::held(p.id)
            .iter()
            .map(|h| crate::i18n::tr(h.label()))
            .collect();
        if !held.is_empty() {
            out += &format!(" — {}", held.join(", "));
        }
    }
    out += "\n  all          ";
    out += crate::i18n::tr(
        "tout : en plus, notifications gardées effacées, thèmes et widgets perso mis de côté",
    );
    out
}

fn on_off(b: bool) -> String {
    if b { "on" } else { "off" }.into()
}

/// Clé de réglage d'une valeur de forme d'un pack (MLG-276) :
/// `<section>.<clé>` du manifeste → clé de `hyprdesk config`. Seules les
/// clés d'apparence et de disposition sont admises : un pack ne touche ni
/// aux raccourcis, ni aux commandes, ni aux applications.
pub fn pack_key(shape: &str) -> Option<&'static Key> {
    let (section, key) = shape.split_once('.')?;
    let alias = match (section, key) {
        ("dock", "position") => "launcher_position",
        ("dock", "size" | "icon_size") => "launcher_size",
        ("dock", "running_apps") => "running_apps",
        ("dock", "group") => "launcher_group",
        ("dock", "show_settings") => "launcher_show_settings",
        ("dock", "show_assistant") => "launcher_show_assistant",
        ("launcher", "style" | "view") => "apps_style",
        ("launcher", "close_on_launch") => "close_on_launch",
        ("launcher", "suggestions" | "show_recent") => "apps_suggestions",
        ("launcher", "categories") => "apps_categories",
        ("launcher", "unified") => "apps_unified",
        ("workspaces", "selector" | "representation") => "workspace_selector",
        ("lock", k) => return find_allowed(&format!("lock_{k}")),
        ("desktop", "mode") => "mode",
        ("desktop", "glass_opacity") => "glass_opacity",
        ("desktop", "icon_size") => "grid.icon_size",
        ("desktop", "cell") => "grid.cell",
        ("desktop", "transition") => "transition",
        ("desktop", "fade_ms") => "fade_ms",
        ("bar", "edge") => "bar.position",
        ("bar", "thickness") => "bar.height",
        ("bar", "edge_offset") => "bar.margin",
        ("bar", "fullscreen_behavior" | "fullscreen") => "bar.layer",
        _ => return find_allowed(shape),
    };
    find_allowed(alias)
}

/// Préfixes des clés qu'un pack peut poser.
const PACK_KEYS: &[&str] = &[
    "bar.",
    "launcher_position",
    "launcher_size",
    "launcher_group",
    "launcher_show_",
    "running_apps",
    "apps_style",
    "apps_suggestions",
    "apps_categories",
    "apps_unified",
    "workspace_selector",
    "lock_blur",
    "lock_dim",
    "lock_show_",
    "mode",
    "glass_opacity",
    "grid.",
    "transition",
    "fade_ms",
    "crossfade_ms",
    "dock.",
    "launcher.",
    "workspaces.",
    "notifications.",
    "widgets.",
    "controls.",
    "lock.",
    "wallpaper.fit",
    "close_on_launch",
];

fn find_allowed(name: &str) -> Option<&'static Key> {
    if !PACK_KEYS
        .iter()
        .any(|p| name == *p || (p.ends_with(['.', '_']) && name.starts_with(p)))
        || name == "bar.monitor"
    {
        return None;
    }
    KEYS.iter().find(|k| k.name == name)
}

/// Pose une valeur de forme d'un pack dans `s` (même validation que
/// `hyprdesk config set`).
pub fn apply_pack_value(
    s: &mut Settings,
    shape: &str,
    value: &str,
    cfg: &Config,
) -> Result<(), String> {
    // Options d'apparence des composants (MLG-279) : toutes admises.
    if crate::design_options::find(shape).is_some()
        || crate::design_options::screen_find(shape).is_some()
    {
        return crate::design_options::set(s, cfg, shape, value);
    }
    let key = pack_key(shape)
        .ok_or_else(|| crate::tr!("clé de forme non admise pour un pack : {k}", k = shape))?;
    (key.set)(s, value, cfg)
}

/// Forme du lanceur à modifier (MLG-278), partie de celle en place.
fn ls<'a>(s: &'a mut Settings, c: &Config) -> &'a mut crate::config::LauncherShape {
    s.launcher_shape
        .get_or_insert(c.general.apps_launcher.shape)
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

const LID_CHOICES: &str = "system, suspend, hibernate, lock, screen-off, nothing";

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
                "« {v} » : raccourci illisible (ex. \"SHIFT + SUPER + D\")",
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
    "icon_scope", "desktop", "global | screen | workspace — icônes partout, par écran ou par workspace",
        |c| c.general.icon_scope.as_str().into(),
        |s, v, c| { s.icon_scope = Some(choice(v, crate::config::IconScope::parse, "global, screen, workspace")?); Ok(()) },
        |s| s.icon_scope = None;
    "icon_double_click", "desktop", "on | off — icônes ouvertes d'un double clic (un clic : sélection)",
        |c| on_off(c.general.icon_double_click),
        |s, v, c| { s.icon_double_click = Some(parse_bool(v)?); Ok(()) },
        |s| s.icon_double_click = None;
    "hyprbars", "desktop", "on | off — barres de titre Hyprbars (plugin Hyprland) en plein bureau",
        |c| on_off(c.general.hyprbars),
        |s, v, c| { s.hyprbars = Some(parse_bool(v)?); Ok(()) },
        |s| s.hyprbars = None;
    "float_launched", "desktop", "on | off — fenêtres ouvertes depuis le bureau : flottantes",
        |c| on_off(c.general.float_launched),
        |s, v, c| { s.float_launched = Some(parse_bool(v)?); Ok(()) },
        |s| s.float_launched = None;
    "suspend_binds", "keys", "on | off — full-desktop : suspendre les raccourcis de Hyprland",
        |c| on_off(c.general.suspend_binds),
        |s, v, c| { s.suspend_binds = Some(parse_bool(v)?); Ok(()) },
        |s| s.suspend_binds = None;
    "clipboard_history", "desktop", "on | off — historique du presse-papiers (hyprdesk lance cliphist ; « :clip » dans la recherche)",
        |c| on_off(c.general.clipboard_history),
        |s, v, c| { s.clipboard_history = Some(parse_bool(v)?); Ok(()) },
        |s| s.clipboard_history = None;
    "index.paths", "index", "dossier,dossier | none — racines indexées par la recherche :f (défaut : home et volumes montés)",
        |c| c.general.index_paths.join(", "),
        |s, v, c| {
            s.index_paths = Some(if v.trim().is_empty() || v.trim() == "none" {
                Vec::new()
            } else {
                v.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.index_paths = None;
    "index.exclude", "index", "nom,nom | none — dossiers exclus de l'index :f, en plus des défauts (.git, node_modules, target…)",
        |c| c.general.index_exclude.join(", "),
        |s, v, c| {
            s.index_exclude = Some(if v.trim().is_empty() || v.trim() == "none" {
                Vec::new()
            } else {
                v.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.index_exclude = None;
    "index.exclude_regex", "index", "regex,regex | none — exclusions par expression régulière sur le chemin complet",
        |c| c.general.index_exclude_regex.join(", "),
        |s, v, c| {
            s.index_exclude_regex = Some(if v.trim().is_empty() || v.trim() == "none" {
                Vec::new()
            } else {
                v.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.index_exclude_regex = None;
    "index.exclude_ext", "index", "ext,ext | none — extensions écartées de l'index (ex. iso,vdi)",
        |c| c.general.index_exclude_ext.join(", "),
        |s, v, c| {
            s.index_exclude_ext = Some(if v.trim().is_empty() || v.trim() == "none" {
                Vec::new()
            } else {
                v.split(',').map(|p| p.trim().trim_start_matches('.').to_lowercase()).filter(|p| !p.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.index_exclude_ext = None;
    "index.exclude_cat", "index", "catégorie,catégorie | none — catégories écartées de l'index (documents, images, vidéos, programmes, audio, archives, autres)",
        |c| c.general.index_exclude_cat.join(", "),
        |s, v, c| {
            s.index_exclude_cat = Some(if v.trim().is_empty() || v.trim() == "none" {
                Vec::new()
            } else {
                v.split(',').map(|p| p.trim().to_lowercase()).filter(|p| !p.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.index_exclude_cat = None;
    "index.max_size", "index", "Mio — taille maximale indexée par fichier (0 : illimité)",
        |c| c.general.index_max_size.to_string(),
        |s, v, c| { s.index_max_size = Some(v.trim().parse().map_err(|_| crate::tr!("« {v} » : nombre attendu", v = v))?); Ok(()) },
        |s| s.index_max_size = None;
    "index.interval", "index", "secondes — rescan de sûreté de l'index :f (défaut 21600, soit 6 h)",
        |c| c.general.index_interval.to_string(),
        |s, v, c| { s.index_interval = Some(v.trim().parse().map_err(|_| crate::tr!("« {v} » : nombre attendu", v = v))?); Ok(()) },
        |s| s.index_interval = None;
    "index.autostart", "index", "on | off — démon d'index lancé avec hyprdesk (off : au premier « :f »)",
        |c| on_off(c.general.index_autostart),
        |s, v, c| { s.index_autostart = Some(parse_bool(v)?); Ok(()) },
        |s| s.index_autostart = None;
    "autostart", "desktop", "on | off — lancer les applications de ~/.config/autostart et /etc/xdg/autostart (une fois par session)",
        |c| on_off(c.general.autostart),
        |s, v, c| { s.autostart = Some(parse_bool(v)?); Ok(()) },
        |s| s.autostart = None;
    "desktop_files", "desktop", "on | off — fichiers du dossier Bureau et Corbeille sur le bureau",
        |c| on_off(c.general.desktop_files),
        |s, v, c| { s.desktop_files = Some(parse_bool(v)?); Ok(()) },
        |s| s.desktop_files = None;
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
    "renderer", "performance", "hardware | software — moteur de rendu de l'interface : carte graphique (wgpu, repli logiciel sans carte utilisable) ou processeur (tiny-skia) ; pris en compte à la relance de hyprdesk",
        |c| c.general.renderer.id().into(),
        |s, v, c| {
            s.renderer = Some(crate::render::Renderer::from_id(v).ok_or_else(|| {
                crate::tr!("« {v} » : hardware ou software", v = v)
            })?);
            Ok(())
        },
        |s| s.renderer = None;
    "gpu", "performance", "hyprland | low | high — carte graphique de hyprdesk : celle de Hyprland (défaut, sans recopie), économe (intégrée) ou puissante (dédiée) ; pris en compte à la relance",
        |c| c.general.gpu.id().into(),
        |s, v, c| {
            s.gpu = Some(crate::render::Gpu::from_id(v).ok_or_else(|| {
                crate::tr!("« {v} » : hyprland, low ou high", v = v)
            })?);
            Ok(())
        },
        |s| s.gpu = None;
    "hyprland_gpu", "performance", "<adresse PCI> | auto — carte de rendu de Hyprland (0000:01:00.0, voir lspci) ; prise en compte au prochain démarrage de Hyprland",
        |c| if c.general.hyprland_gpu.is_empty() { "auto".into() } else { c.general.hyprland_gpu.clone() },
        |s, v, c| {
            let v = v.trim();
            if matches!(v, "auto" | "") {
                s.hyprland_gpu = Some(String::new());
                return Ok(());
            }
            if !crate::render_gpu::valid_slot(v) {
                return Err(crate::tr!("« {v} » : adresse PCI attendue (0000:01:00.0) ou auto", v = v.to_string()));
            }
            s.hyprland_gpu = Some(v.to_string());
            Ok(())
        },
        |s| s.hyprland_gpu = None;
    "worker_threads", "performance", "0..64 — fils d'exécution de l'interface (défaut 4 ; 0 : un par cœur) ; pris en compte à la relance",
        |c| c.general.worker_threads.to_string(),
        |s, v, c| {
            s.worker_threads = Some(
                v.trim()
                    .parse::<f64>()
                    .ok()
                    .and_then(crate::render::workers_valid)
                    .ok_or_else(|| crate::tr!("« {v} » : 0 (un par cœur) ou 1 à 64", v = v))?,
            );
            Ok(())
        },
        |s| s.worker_threads = None;
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
    "keys.toggle", "keys", "raccourci | none — mode suivant : no-desktop → launcher-only → full-desktop (SUPER + D)",
        |c| c.general.keys.toggle.clone(),
        |s, v, c| { s.key_toggle = Some(combo(v)?); Ok(()) },
        |s| s.key_toggle = None;
    "keys.launcher", "keys", "raccourci | none — launcher-only (SUPER + ALT + D)",
        |c| c.general.keys.launcher.clone(),
        |s, v, c| { s.key_launcher = Some(combo(v)?); Ok(()) },
        |s| s.key_launcher = None;
    "keys.hide", "keys", "raccourci | none — no-desktop (SUPER + CTRL + D)",
        |c| c.general.keys.hide.clone(),
        |s, v, c| { s.key_hide = Some(combo(v)?); Ok(()) },
        |s| s.key_hide = None;
    "keys.settings", "keys", "raccourci | none — ouvrir ou fermer les réglages (SUPER + SHIFT + D)",
        |c| c.general.keys.settings.clone(),
        |s, v, c| { s.key_settings = Some(combo(v)?); Ok(()) },
        |s| s.key_settings = None;
    "keys.lock", "keys", "raccourci | none — verrouiller la session ; none : celui de hyprland.lua (SUPER + L), réglé : hyprdesk le remplace",
        |c| c.general.keys.lock.clone(),
        |s, v, c| { s.key_lock = Some(combo(v)?); Ok(()) },
        |s| s.key_lock = None;
    "keys.switch", "keys", "raccourci | none — sélecteur de fenêtres (Alt+Tab ; avec Maj : en arrière)",
        |c| c.general.keys.switch.clone(),
        |s, v, c| { s.key_switch = Some(combo(v)?); Ok(()) },
        |s| s.key_switch = None;
    "keys.search", "keys", "raccourci | none — lanceur d'applications, dans tous les modes (SUPER + R)",
        |c| c.general.keys.search.clone(),
        |s, v, c| { s.key_search = Some(combo(v)?); Ok(()) },
        |s| s.key_search = None;
    "keys.search_desktop", "keys", "raccourci | none — plein bureau : un autre raccourci pour le lanceur (none : keys.search)",
        |c| c.general.keys.search_desktop.clone(),
        |s, v, c| { s.key_search_desktop = Some(combo(v)?); Ok(()) },
        |s| s.key_search_desktop = None;
    "keys.take", "keys", "raccourci,raccourci,… | none — raccourcis que hyprdesk prend à hyprland.lua (déliés là-bas)",
        |c| c.general.keys.take.join(","),
        |s, v, c| {
            s.key_take = Some(if matches!(v.trim(), "none" | "") { Vec::new() } else {
                v.split(',').map(|k| combo(k.trim())).filter(|k| !matches!(k, Ok(k) if k.is_empty())).collect::<Result<Vec<_>, _>>()?
            });
            Ok(())
        },
        |s| s.key_take = None;
    "keys.overview", "keys", "raccourci | none — vue d'ensemble des fenêtres (hyprdesk overview)",
        |c| c.general.keys.overview.clone(),
        |s, v, c| { s.key_overview = Some(combo(v)?); Ok(()) },
        |s| s.key_overview = None;
    "keys.assistant", "keys", "raccourci | none — ouvrir ou refermer LYRA, l'assistant IA (hyprdesk assistant, SUPER + SHIFT + L)",
        |c| c.general.keys.assistant.clone(),
        |s, v, c| { s.key_assistant = Some(combo(v)?); Ok(()) },
        |s| s.key_assistant = None;
    "overview_command", "keys", "commande | none — vue d'ensemble par un plugin de Hyprland (sinon le sélecteur de hyprdesk)",
        |c| if c.general.overview_command.is_empty() { "none".into() } else { c.general.overview_command.clone() },
        |s, v, c| { s.overview_command = Some(if v.trim() == "none" { String::new() } else { v.trim().to_string() }); Ok(()) },
        |s| s.overview_command = None;
    "keys.leave", "keys", "raccourci | none — sortie de secours",
        |c| c.general.keys.leave.clone(),
        |s, v, c| { s.key_leave = Some(combo(v)?); Ok(()) },
        |s| s.key_leave = None;
    "keys.media", "keys", "on | off — touches de volume, de luminosité et de lecture gérées par hyprdesk (avec affichage)",
        |c| on_off(c.general.keys.media),
        |s, v, c| { s.key_media = Some(parse_bool(v)?); Ok(()) },
        |s| s.key_media = None;
    "keys.essentials", "keys", "on | off — raccourcis essentiels de fenêtres, de workspaces et d'écrans posés par hyprdesk là où hyprland.lua n'en a pas",
        |c| on_off(c.general.keys.essentials),
        |s, v, c| { s.key_essentials = Some(parse_bool(v)?); Ok(()) },
        |s| s.key_essentials = None;
    "keys.essentials_off", "keys", "id,id,… | none — essentiels coupés (hyprdesk keys les liste)",
        |c| c.general.keys.essentials_off.join(","),
        |s, v, c| {
            s.key_essentials_off = Some(if matches!(v.trim(), "none" | "") { Vec::new() } else {
                v.split(',').map(|k| k.trim()).filter(|k| !k.is_empty()).map(|k| {
                    if crate::essentials::CATALOGUE.iter().any(|e| e.id == k) {
                        Ok(k.to_string())
                    } else {
                        Err(crate::tr!("essentiel « {k} » inconnu", k = k))
                    }
                }).collect::<Result<Vec<_>, _>>()?
            });
            Ok(())
        },
        |s| s.key_essentials_off = None;
    "keys.essential_keys", "keys", "id.sens=raccourci,… | none — combinaisons choisies pour des essentiels (ex. swap.left=SUPER + CTRL + h)",
        |c| c.general.keys.essential_keys.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(","),
        |s, v, c| {
            let mut map = std::collections::BTreeMap::new();
            if !matches!(v.trim(), "none" | "") {
                for pair in v.split(',') {
                    let (slot, k) = pair.split_once('=').ok_or_else(|| crate::tr!("« {p} » : attendu id=raccourci", p = pair))?;
                    if !crate::essentials::slots().iter().any(|s| s == slot.trim()) {
                        return Err(crate::tr!("essentiel « {k} » inconnu", k = slot.trim()));
                    }
                    map.insert(slot.trim().to_string(), combo(k.trim())?);
                }
            }
            s.key_essential_keys = Some(map);
            Ok(())
        },
        |s| s.key_essential_keys = None;
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
    "running_apps", "dock", "off | workspace | all — fenêtres du dock",
        |c| c.general.running_apps.as_str().into(),
        |s, v, c| { s.running_apps = Some(choice(v, RunningApps::parse, "off, workspace, all")?); Ok(()) },
        |s| s.running_apps = None;
    "launcher_group", "dock", "none | app | workspace — regroupement du dock",
        |c| c.general.launcher_group.as_str().into(),
        |s, v, c| { s.launcher_group = Some(choice(v, LauncherGroup::parse, "none, app, workspace")?); Ok(()) },
        |s| s.launcher_group = None;
    "launcher_position", "dock", "bottom | top | left | right — bord du dock",
        |c| c.general.launcher.position.as_str().into(),
        |s, v, c| { s.launcher_position = Some(choice(v, crate::config::LauncherPosition::parse, "bottom, top, left, right")?); Ok(()) },
        |s| s.launcher_position = None;
    "launcher_size", "dock", "24..96 — taille des icônes du dock",
        |c| c.general.launcher.size.to_string(),
        |s, v, c| { s.launcher_size = Some(number(v, 24u16, 96)?); Ok(()) },
        |s| s.launcher_size = None;
    "launcher_pinned", "dock", "app,app,… | none — applications épinglées (ids .desktop)",
        |c| c.general.launcher.pinned.join(","),
        |s, v, c| {
            s.launcher_pinned = Some(if v == "none" { Vec::new() } else {
                v.split(',').map(|a| a.trim().trim_end_matches(".desktop").to_string()).filter(|a| !a.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.launcher_pinned = None;
    "launcher_show_settings", "dock", "on | off — bouton des réglages dans le dock",
        |c| on_off(c.general.launcher.show_settings),
        |s, v, c| { s.launcher_show_settings = Some(parse_bool(v)?); Ok(()) },
        |s| s.launcher_show_settings = None;
    "launcher_show_assistant", "dock", "on | off — bouton de l'assistant IA dans le dock",
        |c| on_off(c.general.launcher.show_assistant),
        |s, v, c| { s.launcher_show_assistant = Some(parse_bool(v)?); Ok(()) },
        |s| s.launcher_show_assistant = None;
    "apps_style", "launcher", "auto | sheet | spotlight — lanceur d'applications : grande feuille en plein bureau et carte centrée sinon, ou toujours l'une",
        |c| c.general.apps_launcher.style.as_str().to_string(),
        |s, v, c| { s.apps_style = Some(crate::config::AppsStyle::parse(v.trim()).ok_or_else(|| crate::i18n::tr("auto, sheet ou spotlight attendu").to_string())?); Ok(()) },
        |s| s.apps_style = None;
    "apps_suggestions", "launcher", "0..16 — applications les plus lancées mises en avant",
        |c| c.general.apps_launcher.suggestions.to_string(),
        |s, v, c| { s.apps_suggestions = Some(number(v, 0u8, 16)?); Ok(()) },
        |s| s.apps_suggestions = None;
    "apps_unified", "launcher", "on | off — le même lanceur dans tous les modes (keys.search pris à hyprland.lua) ; off : celui de hyprland.lua hors plein bureau",
        |c| on_off(c.general.apps_launcher.unified),
        |s, v, c| { s.apps_unified = Some(parse_bool(v)?); Ok(()) },
        |s| s.apps_unified = None;
    "apps_categories", "launcher", "on | off — catégories dans la grande feuille",
        |c| on_off(c.general.apps_launcher.categories),
        |s, v, c| { s.apps_categories = Some(parse_bool(v)?); Ok(()) },
        |s| s.apps_categories = None;
    "lock_blur", "lock", "on | off — fond flouté sur l'écran de verrouillage",
        |c| on_off(c.general.lock.blur),
        |s, v, c| { s.lock_blur = Some(parse_bool(v)?); Ok(()) },
        |s| s.lock_blur = None;
    "lock_dim", "lock", "0..0.9 — assombrissement du fond verrouillé",
        |c| format!("{}", c.general.lock.dim),
        |s, v, c| { s.lock_dim = Some(number(v, 0.0f32, 0.9)?); Ok(()) },
        |s| s.lock_dim = None;
    "lock_image", "lock", "chemin d'image ou de vidéo | none — fond propre au verrouillage (sinon le fond du bureau)",
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
    "lock_fingerprint", "lock", "on | off — empreinte (fprintd) à la place du mot de passe, si un lecteur est là (désactivée par défaut)",
        |c| on_off(c.general.lock.fingerprint),
        |s, v, c| { s.lock_fingerprint = Some(parse_bool(v)?); Ok(()) },
        |s| s.lock_fingerprint = None;
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
    "idle_wake_volume", "lock", "on | off — écrans éteints à la main (Panorama → Système → Veille des écrans) : un changement de volume les rallume",
        |c| on_off(c.general.idle.wake_volume),
        |s, v, c| { s.idle_wake_volume = Some(parse_bool(v)?); Ok(()) },
        |s| s.idle_wake_volume = None;
    "idle_lid_battery", "lock", "system | suspend | hibernate | lock | screen-off | nothing — capot refermé, sur batterie",
        |c| c.general.idle.lid_battery.as_str().into(),
        |s, v, c| { s.idle_lid_battery = Some(choice(v, LidAction::parse, LID_CHOICES)?); Ok(()) },
        |s| s.idle_lid_battery = None;
    "idle_lid_ac", "lock", "system | suspend | hibernate | lock | screen-off | nothing — capot refermé, sur secteur",
        |c| c.general.idle.lid_ac.as_str().into(),
        |s, v, c| { s.idle_lid_ac = Some(choice(v, LidAction::parse, LID_CHOICES)?); Ok(()) },
        |s| s.idle_lid_ac = None;
    "idle_lid_docked", "lock", "system | suspend | hibernate | lock | screen-off | nothing — capot refermé avec un écran externe",
        |c| c.general.idle.lid_docked.as_str().into(),
        |s, v, c| { s.idle_lid_docked = Some(choice(v, LidAction::parse, LID_CHOICES)?); Ok(()) },
        |s| s.idle_lid_docked = None;
    "ai_use.smartui", "assistant", "lyra | id d'un fournisseur — fournisseur IA de SmartUI design (fournisseurs : Réglages → IA)",
        |c| c.general.ai.for_consumer("smartui").id,
        |s, v, c| {
            if c.general.ai.service(v).is_none() {
                let ids: Vec<String> = c.general.ai.all().into_iter().map(|s| s.id).collect();
                return Err(crate::tr!("fournisseur « {v} » inconnu ({ids})", v = v.to_string(), ids = ids.join(", ")));
            }
            s.ai_use.insert("smartui".into(), v.to_string());
            Ok(())
        },
        |s| { s.ai_use.remove("smartui"); };
    "launcher_smartui", "launcher", "on | off — SmartUI design : icônes redessinées par l'IA et validées",
        |c| on_off(c.general.launcher.smartui),
        |s, v, c| { s.launcher_smartui = Some(parse_bool(v)?); Ok(()) },
        |s| s.launcher_smartui = None;
    "smartui_auto", "launcher", "on | off — SmartUI design : icônes générées et validées d'office (défaut : off ; envoie nom et icône de chaque application au fournisseur d'IA)",
        |c| on_off(crate::smartui::auto_on(c)),
        |s, v, c| { s.smartui_auto = Some(parse_bool(v)?); Ok(()) },
        |s| s.smartui_auto = None;
    "smartui_style", "launcher", "texte | vide — style demandé à l'IA pour SmartUI design",
        |c| c.general.launcher.smartui_style.clone(),
        |s, v, c| { s.smartui_style = Some(v.to_string()); Ok(()) },
        |s| s.smartui_style = None;
    "assistant_mode", "assistant", "window | terminal — ouverture de LYRA (fournisseur : apps.ai)",
        |c| c.general.assistant_mode.as_str().into(),
        |s, v, c| { s.assistant_mode = Some(choice(v, crate::assistant::AssistantMode::parse, "window, terminal")?); Ok(()) },
        |s| s.assistant_mode = None;
    "wallpaper", "wallpaper", "chemin d'image ou de vidéo | #couleur — fond de tous les bureaux",
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
    "bar.position", "bar", "top | bottom | left | right — bord de la barre",
        |c| c.bar.as_ref().map(|b| b.position.as_str().to_string()).unwrap_or_default(),
        |s, v, c| {
            bar(s, c).position = BarPosition::parse(v).ok_or_else(|| crate::tr!("« {v} » : top, bottom, left ou right", v = v))?;
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
    "bar.convex.frame_width", "bar", "0..20 — épaisseur du cadre Convex (px)",
        |c| c.bar.as_ref().map(|b| b.convex.frame_width.to_string()).unwrap_or_default(),
        |s,v,c| { bar(s,c).convex.frame_width=number(v,0u16,20)?; Ok(()) },
        |s| { if let Some(b)=s.bar.as_mut() { b.convex.frame_width=crate::config::ConvexDef::default().frame_width; } };
    "bar.convex.radius", "bar", "0..96 — arrondi intérieur du cadre Convex (px)",
        |c| c.bar.as_ref().map(|b| b.convex.radius.to_string()).unwrap_or_default(),
        |s,v,c| { bar(s,c).convex.radius=number(v,0u16,96)?; Ok(()) },
        |s| { if let Some(b)=s.bar.as_mut() { b.convex.radius=crate::config::ConvexDef::default().radius; } };
    "bar.convex.duration", "bar", "0..800 — durée d'ouverture des panneaux Convex (ms)",
        |c| c.bar.as_ref().map(|b| b.convex.duration.to_string()).unwrap_or_default(),
        |s,v,c| { bar(s,c).convex.duration=number(v,0u16,800)?; Ok(()) },
        |s| { if let Some(b)=s.bar.as_mut() { b.convex.duration=crate::config::ConvexDef::default().duration; } };
    "bar.convex.auto_hide", "bar", "on | off — masquage automatique du dock Convex",
        |c| c.bar.as_ref().map(|b| on_off(b.convex.auto_hide)).unwrap_or_default(),
        |s,v,c|{bar(s,c).convex.auto_hide=parse_bool(v)?;Ok(())},
        |s|{if let Some(b)=s.bar.as_mut(){b.convex.auto_hide=true;}};
    "bar.convex.attached_panels", "bar", "on | off — panneaux rattachés au cadre Convex",
        |c| c.bar.as_ref().map(|b| on_off(b.convex.attached_panels)).unwrap_or_default(),
        |s,v,c| { bar(s,c).convex.attached_panels=parse_bool(v)?; Ok(()) },
        |s| { if let Some(b)=s.bar.as_mut() { b.convex.attached_panels=true; } };
    "bar.style", "bar", "continuity | graphite | island | convex — style de la barre : pose son rendu, sa composition et sa géométrie",
        |c| c.bar.as_ref().map(|b| b.style.as_str().to_string()).unwrap_or_default(),
        |s, v, c| {
            let all: Vec<&str> = crate::config::BAR_STYLES.iter().map(|(_, id, _)| *id).collect();
            let style = choice(v, crate::config::BarStyle::parse, &all.join(", "))?;
            let b = bar(s, c);
            b.style = style;
            b.position = style.default_position();
            b.height = style.default_height();
            b.margin = style.default_margin();
            b.portrait = None;
            b.zones = style.preset_settings();
            Ok(())
        },
        |s| { if let Some(b) = s.bar.as_mut() {
            let style = crate::config::BarStyle::default();
            b.style = style;
            b.position = style.default_position();
            b.height = style.default_height();
            b.margin = style.default_margin();
            b.portrait = None;
            b.zones = style.preset_settings();
        } };
    "bar.margin", "bar", "0..32 — marge entre la barre et le bord de l'écran (0 : collée)",
        |c| c.bar.as_ref().map(|b| b.margin.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).margin = number(v, 0u16, 32)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.margin = BarSettings::default().margin; } };
    "bar.length", "bar", "full | content | <pixels> | <n>% — longueur de la barre le long de son bord",
        |c| c.bar.as_ref().map(|b| b.shape.length.as_text()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.length = choice(v, crate::config::BarLength::parse, "full, content, 48..8192, 5%..100%")?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.length = Default::default(); } };
    "bar.min_length", "bar", "0..8192 — longueur minimale (0 : aucune)",
        |c| c.bar.as_ref().map(|b| b.shape.min_length.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.min_length = number(v, 0u16, 8192)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.min_length = crate::config::BarShape::default().min_length; } };
    "bar.max_length", "bar", "0..8192 — longueur maximale (0 : aucune)",
        |c| c.bar.as_ref().map(|b| b.shape.max_length.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.max_length = number(v, 0u16, 8192)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.max_length = crate::config::BarShape::default().max_length; } };
    "bar.alignment", "bar", "start | center | end — place d'une barre plus courte que son bord",
        |c| c.bar.as_ref().map(|b| match b.shape.alignment { crate::config::BarAlign::Start => "start", crate::config::BarAlign::Center => "center", crate::config::BarAlign::End => "end" }.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.alignment = choice(v, |x| match x { "start" => Some(crate::config::BarAlign::Start), "center" => Some(crate::config::BarAlign::Center), "end" => Some(crate::config::BarAlign::End), _ => None }, "start, center, end")?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.alignment = Default::default(); } };
    "bar.side_margin", "bar", "0..400 — marges sur les côtés (barre flottante)",
        |c| c.bar.as_ref().map(|b| b.shape.side_margin.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.side_margin = number(v, 0u16, 400)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.side_margin = crate::config::BarShape::default().side_margin; } };
    "bar.layout", "bar", "continuous | groups — lame continue ou groupes détachés (Continuité)",
        |c| c.bar.as_ref().map(|b| if b.shape.groups { "groups" } else { "continuous" }.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.groups = choice(v, |x| match x { "continuous" => Some(false), "groups" => Some(true), _ => None }, "continuous, groups")?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.groups = false; } };
    "bar.radius", "bar", "style | 0..64 — rayon des coins de la barre",
        |c| c.bar.as_ref().and_then(|b| b.shape.radius).map(|v| v.to_string()).unwrap_or_else(|| "style".into()),
        |s, v, c| { bar(s, c).shape.radius = if v.trim() == "style" { None } else { Some(number(v, 0u16, 64)?) }; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.radius = None; } };
    "bar.padding", "bar", "style | 0..32 — marge intérieure de la barre",
        |c| c.bar.as_ref().and_then(|b| b.shape.padding).map(|v| v.to_string()).unwrap_or_else(|| "style".into()),
        |s, v, c| { bar(s, c).shape.padding = if v.trim() == "style" { None } else { Some(number(v, 0u16, 32)?) }; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.padding = None; } };
    "bar.item_gap", "bar", "style | 0..32 — espacement des modules",
        |c| c.bar.as_ref().and_then(|b| b.shape.item_gap).map(|v| v.to_string()).unwrap_or_else(|| "style".into()),
        |s, v, c| { bar(s, c).shape.item_gap = if v.trim() == "style" { None } else { Some(number(v, 0u16, 32)?) }; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.item_gap = None; } };
    "bar.icon_size", "bar", "style | 10..48 — taille des icônes de la barre",
        |c| c.bar.as_ref().and_then(|b| b.shape.icon_size).map(|v| v.to_string()).unwrap_or_else(|| "style".into()),
        |s, v, c| { bar(s, c).shape.icon_size = if v.trim() == "style" { None } else { Some(number(v, 10u16, 48)?) }; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.icon_size = None; } };
    "bar.font_size", "bar", "style | 8..32 — taille du texte de la barre",
        |c| c.bar.as_ref().and_then(|b| b.shape.font_size).map(|v| v.to_string()).unwrap_or_else(|| "style".into()),
        |s, v, c| { bar(s, c).shape.font_size = if v.trim() == "style" { None } else { Some(number(v, 8.0f32, 32.0)?) }; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.font_size = None; } };
    "bar.visibility", "bar", "always | autohide — barre toujours visible, ou masquée hors survol",
        |c| c.bar.as_ref().map(|b| if b.shape.visibility == crate::config::BarVisibility::AutoHide { "autohide" } else { "always" }.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.visibility = choice(v, |x| match x { "always" => Some(crate::config::BarVisibility::Always), "autohide" => Some(crate::config::BarVisibility::AutoHide), _ => None }, "always, autohide")?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.visibility = Default::default(); } };
    "bar.show_delay", "bar", "0..3000 — délai d'apparition au contact du bord (ms)",
        |c| c.bar.as_ref().map(|b| b.shape.show_delay.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.show_delay = number(v, 0u16, 3000)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.show_delay = crate::config::BarShape::default().show_delay; } };
    "bar.hide_delay", "bar", "0..10000 — délai avant de masquer (ms)",
        |c| c.bar.as_ref().map(|b| b.shape.hide_delay.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.hide_delay = number(v, 0u16, 10000)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.hide_delay = crate::config::BarShape::default().hide_delay; } };
    "bar.reveal_zone", "bar", "1..40 — épaisseur de la zone qui fait revenir la barre (px)",
        |c| c.bar.as_ref().map(|b| b.shape.reveal_zone.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.reveal_zone = number(v, 1u16, 40)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.reveal_zone = crate::config::BarShape::default().reveal_zone; } };
    "bar.reserve_space", "bar", "on | off — les fenêtres laissent la place de la barre",
        |c| c.bar.as_ref().map(|b| on_off(b.shape.reserve_space)).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.reserve_space = parse_bool(v)?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.reserve_space = true; } };
    "bar.layer", "bar", "top | overlay | bottom — au-dessus des fenêtres (sous le plein écran), de tout, ou dessous",
        |c| c.bar.as_ref().map(|b| match b.shape.layer { crate::config::BarLayer::Top => "top", crate::config::BarLayer::Overlay => "overlay", crate::config::BarLayer::Bottom => "bottom" }.to_string()).unwrap_or_default(),
        |s, v, c| { bar(s, c).shape.layer = choice(v, |x| match x { "top" => Some(crate::config::BarLayer::Top), "overlay" => Some(crate::config::BarLayer::Overlay), "bottom" => Some(crate::config::BarLayer::Bottom), _ => None }, "top, overlay, bottom")?; Ok(()) },
        |s| { if let Some(b) = s.bar.as_mut() { b.shape.layer = Default::default(); } };
    "launcher.width", "launcher", "360..2000 — largeur de la carte du lanceur (px)",
        |c| c.general.apps_launcher.shape.width.to_string(),
        |s, v, c| { ls(s, c).width = number(v, 360u16, 2000)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.width = crate::config::LauncherShape::default().width; } };
    "launcher.max_height", "launcher", "0..1 — hauteur maximale, part de l'écran (0 : celle du lanceur)",
        |c| c.general.apps_launcher.shape.max_height.to_string(),
        |s, v, c| { ls(s, c).max_height = number(v, 0.0f32, 1.0)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.max_height = crate::config::LauncherShape::default().max_height; } };
    "launcher.radius", "launcher", "0..64 — rayon des coins de la carte",
        |c| c.general.apps_launcher.shape.radius.to_string(),
        |s, v, c| { ls(s, c).radius = number(v, 0u16, 64)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.radius = crate::config::LauncherShape::default().radius; } };
    "launcher.padding", "launcher", "0..64 — marge intérieure de la carte",
        |c| c.general.apps_launcher.shape.padding.to_string(),
        |s, v, c| { ls(s, c).padding = number(v, 0u16, 64)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.padding = crate::config::LauncherShape::default().padding; } };
    "launcher.columns", "launcher", "2..12 — colonnes de la grille d'applications",
        |c| c.general.apps_launcher.shape.columns.to_string(),
        |s, v, c| { ls(s, c).columns = number(v, 2u8, 12)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.columns = crate::config::LauncherShape::default().columns; } };
    "launcher.row_height", "launcher", "28..96 — hauteur d'une ligne de résultat (px)",
        |c| c.general.apps_launcher.shape.row_height.to_string(),
        |s, v, c| { ls(s, c).row_height = number(v, 28u16, 96)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.row_height = crate::config::LauncherShape::default().row_height; } };
    "launcher.max_results", "launcher", "1..30 — résultats visibles",
        |c| c.general.apps_launcher.shape.max_results.to_string(),
        |s, v, c| { ls(s, c).max_results = number(v, 1u8, 30)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.max_results = crate::config::LauncherShape::default().max_results; } };
    "launcher.backdrop_dim", "launcher", "0..0.9 — voile sur le bureau derrière le lanceur",
        |c| c.general.apps_launcher.shape.backdrop_dim.to_string(),
        |s, v, c| { ls(s, c).backdrop_dim = number(v, 0.0f32, 0.9)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.backdrop_dim = crate::config::LauncherShape::default().backdrop_dim; } };
    "launcher.open_duration", "launcher", "0..1500 — durée d'ouverture (ms, 0 : celle du thème)",
        |c| c.general.apps_launcher.shape.open_duration.to_string(),
        |s, v, c| { ls(s, c).open_duration = number(v, 0u16, 1500)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.open_duration = crate::config::LauncherShape::default().open_duration; } };
    "launcher.offset", "launcher", "-800..800 — décalage vertical de la carte (px)",
        |c| c.general.apps_launcher.shape.offset.to_string(),
        |s, v, c| { ls(s, c).offset = number(v, -800i16, 800)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.offset = crate::config::LauncherShape::default().offset; } };
    "launcher.selection_radius", "launcher", "0..32 — rayon du résultat choisi",
        |c| c.general.apps_launcher.shape.selection_radius.to_string(),
        |s, v, c| { ls(s, c).selection_radius = number(v, 0u16, 32)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.selection_radius = crate::config::LauncherShape::default().selection_radius; } };
    "launcher.border", "launcher", "0..1 — opacité du filet de la carte",
        |c| c.general.apps_launcher.shape.border.to_string(),
        |s, v, c| { ls(s, c).border = number(v, 0.0f32, 1.0)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.border = crate::config::LauncherShape::default().border; } };
    "launcher.shadow", "launcher", "0..1 — opacité de l'ombre de la carte",
        |c| c.general.apps_launcher.shape.shadow.to_string(),
        |s, v, c| { ls(s, c).shadow = number(v, 0.0f32, 1.0)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.shadow = crate::config::LauncherShape::default().shadow; } };
    "launcher.empty_view", "launcher", "suggestions | favorites | field — avant la saisie : les plus lancées, les épinglées, ou le champ seul",
        |c| { let x = c.general.apps_launcher.shape.empty_view; match x { crate::config::EmptyView::Suggestions => "suggestions", crate::config::EmptyView::Favorites => "favorites", crate::config::EmptyView::Field => "field" }.to_string() },
        |s, v, c| { ls(s, c).empty_view = choice(v, |x| match x { "suggestions" => Some(crate::config::EmptyView::Suggestions), "favorites" => Some(crate::config::EmptyView::Favorites), "field" => Some(crate::config::EmptyView::Field), _ => None }, "suggestions, favorites, field")?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.empty_view = crate::config::LauncherShape::default().empty_view; } };
    "launcher.open_effect", "launcher", "fade | scale | slide | none — effet d'ouverture",
        |c| { let x = c.general.apps_launcher.shape.open_effect; match x { crate::config::OpenEffect::Fade => "fade", crate::config::OpenEffect::Scale => "scale", crate::config::OpenEffect::Slide => "slide", crate::config::OpenEffect::None => "none" }.to_string() },
        |s, v, c| { ls(s, c).open_effect = choice(v, |x| match x { "fade" => Some(crate::config::OpenEffect::Fade), "scale" => Some(crate::config::OpenEffect::Scale), "slide" => Some(crate::config::OpenEffect::Slide), "none" => Some(crate::config::OpenEffect::None), _ => None }, "fade, scale, slide, none")?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.open_effect = crate::config::LauncherShape::default().open_effect; } };
    "launcher.anchor", "launcher", "center | top — carte au centre ou en haut de l'écran",
        |c| { let x = c.general.apps_launcher.shape.top; if x { "top" } else { "center" }.to_string() },
        |s, v, c| { ls(s, c).top = choice(v, |x| match x { "center" => Some(false), "top" => Some(true), _ => None }, "center, top")?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.top = crate::config::LauncherShape::default().top; } };
    "launcher.effects_quality", "launcher", "low | medium | high — effets du verre (low : sans lentille ni reflet)",
        |c| { let x = c.general.apps_launcher.shape.quality; match x { crate::config::EffectsQuality::Low => "low", crate::config::EffectsQuality::Medium => "medium", crate::config::EffectsQuality::High => "high" }.to_string() },
        |s, v, c| { ls(s, c).quality = choice(v, |x| match x { "low" => Some(crate::config::EffectsQuality::Low), "medium" => Some(crate::config::EffectsQuality::Medium), "high" => Some(crate::config::EffectsQuality::High), _ => None }, "low, medium, high")?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.quality = crate::config::LauncherShape::default().quality; } };
    "launcher.close_on_outside_click", "launcher", "on | off — un clic hors de la carte ferme le lanceur",
        |c| on_off(c.general.apps_launcher.shape.close_on_outside_click),
        |s, v, c| { ls(s, c).close_on_outside_click = parse_bool(v)?; Ok(()) },
        |s| { if let Some(l) = s.launcher_shape.as_mut() { l.close_on_outside_click = true; } };
    "bar.opacity", "bar", "0..1 | theme — opacité du fond de la barre (0 : transparente)",
        |c| c.bar.as_ref().and_then(|b| b.colors.opacity).map(|o| format!("{}", (o * 100.0).round() / 100.0)).unwrap_or_else(|| "theme".into()),
        |s, v, c| {
            bar(s, c).colors.opacity = if matches!(v, "theme" | "none" | "") { None } else { Some(number(v, 0.0f32, 1.0)?) };
            Ok(())
        },
        |s| { if let Some(b) = s.bar.as_mut() { b.colors.opacity = None; } };
    "panorama.screens", "panorama", "<écran>,… | none — écrans au rôle Panorama (connecteur ou identifiant EDID)",
        |c| c.general.panorama.screens.join(","),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.screens = if matches!(v, "none" | "") { Vec::new() } else { v.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect() };
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.screens.clear(); } };
    "panorama.pages", "panorama", "JSON — pages après la vue par défaut [{name,tiles:[{widget,x,y,w,h}]}] ; ou media,system,clock… (une page par widget)",
        |c| serde_json::to_string(&c.general.panorama.pages).unwrap_or_default(),
        |s, v, c| {
            let l: Vec<crate::panorama::design::Page> = if v.trim_start().starts_with('[') {
                let l: Vec<crate::panorama::design::Page> = serde_json::from_str(v).map_err(|e| e.to_string())?;
                for p in &l {
                    p.validate()?;
                }
                l
            } else {
                let l: Vec<_> = v.split(',').map(str::trim).filter(|x| crate::panorama::PAGES.iter().any(|(id, _)| id == x)).map(crate::panorama::design::Page::single).collect();
                if l.is_empty() {
                    return Err(crate::i18n::tr("widgets connus : media, system, clock, video").to_string());
                }
                l
            };
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.pages = l;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.pages = crate::panorama::Config::default().pages; } };
    "panorama.layout", "panorama", "single | 1x2 | 1x3 | 2x2 | 3x3 | dashboard — une page à la fois, l'écran découpé en zones, ou la composition libre",
        |c| c.general.panorama.layout.id().to_string(),
        |s, v, c| {
            let l = crate::panorama::config::Layout::parse(v).ok_or_else(|| crate::i18n::tr("dispositions : single, 1x2, 1x3, 2x2, 3x3, dashboard").to_string())?;
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.layout = l;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.layout = Default::default(); } };
    "panorama.first", "panorama", "1/2 | 1/3 | auto — part de la zone 1 de la disposition active (1x2, 1x3 ; auto : 1/2 en 1x2, 1/3 en 1x3)",
        |c| c.general.panorama.first_of(c.general.panorama.layout).map(|f| f.id().to_string()).unwrap_or_else(|| "auto".into()),
        |s, v, c| {
            let f = if matches!(v, "auto" | "") { None } else { Some(crate::zones::First::parse(v).ok_or_else(|| crate::i18n::tr("parts : 1/2, 1/3, auto").to_string())?) };
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            let l = p.layout;
            p.set_first(l, f);
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let l = p.layout; p.set_first(l, None); } };
    "panorama.panes", "panorama", "media,video,system — widget de chaque zone de la disposition active (9 au plus)",
        |c| c.general.panorama.panes_of(c.general.panorama.layout).join(","),
        |s, v, c| {
            let l: Vec<String> = v.split(',').map(|x| x.trim().to_string()).filter(|x| crate::panorama::PAGES.iter().any(|(id, _)| id == x)).collect();
            if l.is_empty() {
                return Err(crate::i18n::tr("widgets connus : media, system, clock, video").to_string());
            }
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            let lay = p.layout;
            p.set_panes(lay, l);
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let l = p.layout; p.set_panes(l, Vec::new()); } };
    "panorama.sources", "panorama", "JSON — sources vidéo [{name,url,icon}] (icon : nom ou chemin, vide : initiale)",
        |c| crate::panorama::icons::sources_json(&c.general.panorama.sources),
        |s, v, c| {
            let l = crate::panorama::icons::sources_from_json(v)?;
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.sources = l;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.sources = crate::panorama::config::default_sources(); } };
    "panorama.music_sources", "panorama", "JSON — services de musique du profil Musique [{name,url,icon}]",
        |c| crate::panorama::icons::sources_json(&c.general.panorama.music_sources),
        |s, v, c| {
            let l = crate::panorama::icons::sources_from_json(v)?;
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.music_sources = l;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.music_sources = crate::panorama::config::default_music_sources(); } };
    "panorama.webradio", "panorama", "radio-browser | radio-garden | radio-net | tunein — service de l'onglet Webradio",
        |c| c.general.panorama.webradio.clone(),
        |s, v, c| {
            if crate::panorama::radio::service(v).is_none() {
                return Err("radio-browser | radio-garden | radio-net | tunein".into());
            }
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.webradio = v.into();
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.webradio = crate::panorama::radio::DEFAULT_SERVICE.into(); } };
    "panorama.radios", "panorama", "JSON — radios favorites de l'onglet Webradio [{name,url}] (url : flux)",
        |c| crate::panorama::icons::sources_json(&c.general.panorama.radios),
        |s, v, c| {
            let l = crate::panorama::icons::sources_from_json(v)?;
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.radios = l;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.radios = crate::panorama::radio::default_radios(); } };
    "panorama.browser", "panorama", "<commande> | auto — navigateur Chromium des sources vidéo",
        |c| if c.general.panorama.browser.is_empty() { "auto".into() } else { c.general.panorama.browser.clone() },
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.browser = if matches!(v, "auto" | "") { String::new() } else { v.to_string() };
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.browser.clear(); } };
    "panorama.voice", "assistant", "on | off — LYRA vocale : écoute du mot « Lyra » et réponses parlées (hyprdesk voice setup)",
        |c| if c.general.panorama.voice { "on".into() } else { "off".into() },
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.voice = match v { "on" | "true" | "1" => true, "off" | "false" | "0" => false, _ => return Err("on | off".into()) };
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.voice = false; } };
    "panorama.voice_mic", "assistant", "<nœud PipeWire> | default — micro de LYRA vocale (wpctl status, node.name)",
        |c| if c.general.panorama.voice_mic.is_empty() { "default".into() } else { c.general.panorama.voice_mic.clone() },
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.voice_mic = if matches!(v, "default" | "") { String::new() } else { v.trim().to_string() };
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.voice_mic.clear(); } };
    "panorama.voice_halo", "assistant", "screens | panorama | off — halo de LYRA vocale pendant la conversation",
        |c| c.general.panorama.voice_halo.clone(),
        |s, v, c| {
            if !matches!(v, "screens" | "panorama" | "off") { return Err("screens | panorama | off".into()); }
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.voice_halo = v.into();
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.voice_halo = "screens".into(); } };
    "panorama.voice_halo_color", "assistant", "lyra | accent — couleur du halo (violet-bleu de LYRA, ou accent du thème)",
        |c| c.general.panorama.voice_halo_color.clone(),
        |s, v, c| {
            if !matches!(v, "lyra" | "accent") { return Err("lyra | accent".into()); }
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.voice_halo_color = v.into();
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.voice_halo_color = "lyra".into(); } };
    "panorama.stream_mood", "panorama", "aurora | sunset | ocean — ambiance de la barre « flux vocal » (webradio)",
        |c| c.general.panorama.stream_mood.clone(),
        |s, v, c| {
            if !matches!(v, "aurora" | "sunset" | "ocean") { return Err("aurora | sunset | ocean".into()); }
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.stream_mood = v.into();
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.stream_mood = "aurora".into(); } };
    "panorama.zone", "panorama", "<écran>:<bottom|top|left|right>:<0.1..0.5> | none — zone Panorama : une bande d'écran réservée, que les fenêtres laissent libre",
        |c| c.general.panorama.zone.as_ref().map_or("none".into(), |z| format!("{}:{}:{}", z.screen, z.side, z.size)),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.zone = if matches!(v, "none" | "") {
                None
            } else {
                let f: Vec<&str> = v.split(':').collect();
                let side = f.get(1).copied().unwrap_or("bottom");
                if f[0].trim().is_empty() || !crate::panorama::config::Zone::SIDES.contains(&side) {
                    return Err("<écran>:<bottom|top|left|right>:<0.1..0.5> | none".into());
                }
                let size = f.get(2).and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.25).clamp(0.1, 0.5);
                Some(crate::panorama::config::Zone { screen: f[0].trim().into(), side: side.into(), size })
            };
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.zone = None; } };
    "panorama.sleep_minutes", "panorama", "0..120 — minutes sans lecture ni toucher avant l'horloge du Panorama (0 : jamais)",
        |c| c.general.panorama.sleep_minutes.to_string(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.sleep_minutes = number(v, 0u32, 120)?;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.sleep_minutes = crate::panorama::Config::default().sleep_minutes; } };
    "panorama.video_dim.on", "panorama", "on | off — voile sombre autour de la vidéo du Panorama pendant la lecture",
        |c| on_off(c.general.panorama.video_dim.on),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.video_dim.on = parse_bool(v)?;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.video_dim.on = crate::panorama::config::VideoDim::default().on; } };
    "panorama.video_dim.level", "panorama", "10..90 — intensité du voile autour de la vidéo, en %",
        |c| c.general.panorama.video_dim.level.to_string(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.video_dim.level = number(v, 10u8, 90)?;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.video_dim.level = crate::panorama::config::VideoDim::default().level; } };
    "panorama.video_dim.scope", "panorama", "all | background — voile sur le fond et les zones, ou sur le fond seul",
        |c| c.general.panorama.video_dim.scope.clone(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.video_dim.scope = match v { "all" | "background" => v.to_string(), _ => return Err("all | background".into()) };
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.video_dim.scope = crate::panorama::config::VideoDim::default().scope; } };
    "panorama.video_dim.paused", "panorama", "on | off — garder le voile quand la vidéo est en pause",
        |c| on_off(c.general.panorama.video_dim.paused),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.video_dim.paused = parse_bool(v)?;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.video_dim.paused = crate::panorama::config::VideoDim::default().paused; } };
    "panorama.video_dim.fade", "panorama", "0..3000 — durée du fondu du voile, en ms",
        |c| c.general.panorama.video_dim.fade.to_string(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.video_dim.fade = number(v, 0u32, crate::panorama::config::VideoDim::FADE_MAX)?;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.video_dim.fade = crate::panorama::config::VideoDim::default().fade; } };
    "panorama.video_dim.hover", "panorama", "0..100 — part du voile gardée quand la souris est sur le Panorama, en % (100 : inchangé)",
        |c| c.general.panorama.video_dim.hover.to_string(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.video_dim.hover = number(v, 0u8, 100)?;
            s.panorama = Some(p);
            Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { p.video_dim.hover = crate::panorama::config::VideoDim::default().hover; } };
    "panorama.design.preset", "panorama", "cinematic | forest | telemetry | studio | applications — composition et style de départ",
        |c| c.general.panorama.design.get("preset").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.apply_preset(v)?; p.layout = if v == "cinematic" { crate::panorama::config::Layout::Single } else { crate::panorama::config::Layout::Dashboard };
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let _=p.design.apply_preset("cinematic");p.layout=crate::panorama::config::Layout::Single; } };
    "panorama.design.wallpaper", "panorama", "<chemin> | vide — image de fond du bandeau",
        |c| c.general.panorama.design.get("wallpaper").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("wallpaper", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("wallpaper", &d.get("wallpaper").unwrap_or_default()); } };
    "panorama.design.background", "panorama", "#rrggbb | vide — couleur de fond (vide : thème)",
        |c| c.general.panorama.design.get("background").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("background", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("background", &d.get("background").unwrap_or_default()); } };
    "panorama.design.foreground", "panorama", "#rrggbb | vide — couleur du texte (vide : thème)",
        |c| c.general.panorama.design.get("foreground").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("foreground", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("foreground", &d.get("foreground").unwrap_or_default()); } };
    "panorama.design.accent", "panorama", "#rrggbb | vide — couleur des jauges et commandes",
        |c| c.general.panorama.design.get("accent").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("accent", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("accent", &d.get("accent").unwrap_or_default()); } };
    "panorama.design.dimming", "panorama", "0..1 — assombrissement de l’image de fond",
        |c| c.general.panorama.design.get("dimming").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("dimming", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("dimming", &d.get("dimming").unwrap_or_default()); } };
    "panorama.design.card_opacity", "panorama", "0..1 — opacité des zones",
        |c| c.general.panorama.design.get("card_opacity").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("card_opacity", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("card_opacity", &d.get("card_opacity").unwrap_or_default()); } };
    "panorama.design.radius", "panorama", "0..64 — arrondi des zones",
        |c| c.general.panorama.design.get("radius").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("radius", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("radius", &d.get("radius").unwrap_or_default()); } };
    "panorama.design.gap", "panorama", "0..64 — espace entre les zones",
        |c| c.general.panorama.design.get("gap").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("gap", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("gap", &d.get("gap").unwrap_or_default()); } };
    "panorama.design.padding", "panorama", "0..64 — marge autour de la composition",
        |c| c.general.panorama.design.get("padding").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("padding", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("padding", &d.get("padding").unwrap_or_default()); } };
    "panorama.design.text_scale", "panorama", "0.5..2 — échelle du texte",
        |c| c.general.panorama.design.get("text_scale").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("text_scale", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("text_scale", &d.get("text_scale").unwrap_or_default()); } };
    "panorama.design.effects", "panorama", "on | off — décor 3D et lumières",
        |c| c.general.panorama.design.flag("effects").map(on_off).unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("effects", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("effects", &d.get("effects").unwrap_or_default()); } };
    "panorama.design.controls", "panorama", "on | off — commandes du bandeau",
        |c| c.general.panorama.design.flag("controls").map(on_off).unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("controls", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("controls", &d.get("controls").unwrap_or_default()); } };
    "panorama.design.clock_style", "panorama", "digital | flip | analog — style de l’horloge",
        |c| c.general.panorama.design.get("clock_style").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("clock_style", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("clock_style", &d.get("clock_style").unwrap_or_default()); } };
    "panorama.design.seconds", "panorama", "on | off — secondes de l’horloge",
        |c| c.general.panorama.design.flag("seconds").map(on_off).unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("seconds", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("seconds", &d.get("seconds").unwrap_or_default()); } };
    "panorama.design.date", "panorama", "on | off — date sous l’horloge",
        |c| c.general.panorama.design.flag("date").map(on_off).unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("date", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("date", &d.get("date").unwrap_or_default()); } };
    "panorama.design.system_style", "panorama", "cards | gauges | graphs — présentation des mesures",
        |c| c.general.panorama.design.get("system_style").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("system_style", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("system_style", &d.get("system_style").unwrap_or_default()); } };
    "panorama.design.metrics", "panorama", "cpu,ram,gpu,vram,disk,load,uptime,cpu_temp,gpu_temp — mesures affichées",
        |c| c.general.panorama.design.get("metrics").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("metrics", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("metrics", &d.get("metrics").unwrap_or_default()); } };
    "panorama.design.history_seconds", "panorama", "15..600 — durée de l’historique des courbes",
        |c| c.general.panorama.design.get("history_seconds").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("history_seconds", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("history_seconds", &d.get("history_seconds").unwrap_or_default()); } };
    "panorama.design.media_style", "panorama", "standard | vinyl | compact — présentation du lecteur",
        |c| c.general.panorama.design.get("media_style").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("media_style", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("media_style", &d.get("media_style").unwrap_or_default()); } };
    "panorama.design.shortcut_profile", "panorama", "video | audio | system | power — profil du bloc de raccourcis (system : la liste shortcuts ; power : Système, alimentation et mises à jour)",
        |c| c.general.panorama.design.get("shortcut_profile").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("shortcut_profile", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("shortcut_profile", &d.get("shortcut_profile").unwrap_or_default()); } };
    "panorama.design.shortcut_columns", "panorama", "2..8 — colonnes de raccourcis",
        |c| c.general.panorama.design.get("shortcut_columns").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("shortcut_columns", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("shortcut_columns", &d.get("shortcut_columns").unwrap_or_default()); } };
    "panorama.design.tiles", "panorama", "JSON — zones {widget,x,y,w,h}, coordonnées de 0 à 1",
        |c| c.general.panorama.design.get("tiles").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("tiles", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("tiles", &d.get("tiles").unwrap_or_default()); } };
    "panorama.design.shortcuts", "panorama", "JSON — raccourcis {name,icon,command}",
        |c| c.general.panorama.design.get("shortcuts").unwrap_or_default(),
        |s, v, c| {
            let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone());
            p.design.set("shortcuts", v)?;
            s.panorama = Some(p); Ok(())
        },
        |s| { if let Some(p) = s.panorama.as_mut() { let d = crate::panorama::design::Design::default(); let _ = p.design.set("shortcuts", &d.get("shortcuts").unwrap_or_default()); } };
    "panorama.design.font_family", "panorama", "<nom> | vide — police du bandeau (vide : thème)",
        |c| c.general.panorama.design.get("font_family").unwrap_or_default(),
        |s, v, c| { let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone()); p.design.set("font_family", v)?; s.panorama=Some(p); Ok(()) },
        |s| { if let Some(p)=s.panorama.as_mut() {let _=p.design.set("font_family", &crate::panorama::design::Design::default().get("font_family").unwrap_or_default());} };
    "panorama.design.border_opacity", "panorama", "0..1 — opacité des contours des zones",
        |c| c.general.panorama.design.get("border_opacity").unwrap_or_default(),
        |s, v, c| { let mut p = s.panorama.clone().unwrap_or_else(|| c.general.panorama.clone()); p.design.set("border_opacity", v)?; s.panorama=Some(p); Ok(()) },
        |s| { if let Some(p)=s.panorama.as_mut() {let _=p.design.set("border_opacity", &crate::panorama::design::Design::default().get("border_opacity").unwrap_or_default());} };
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
    "workspace_selector", "workspaces", "relief | rail | thumbnails | icons | numbers | dots — style du sélecteur de la barre",
        |c| c.general.workspace_selector.as_str().into(),
        |s, v, c| { s.workspace_selector = Some(choice(v, crate::config::WsSelector::parse, "relief, rail, thumbnails, icons, numbers, dots")?); Ok(()) },
        |s| s.workspace_selector = None;
    "workspace_dedicated", "workspaces", "écran,écran,… | none — écrans dédiés (seulement leurs workspaces)",
        |c| c.general.workspace_dedicated.iter().cloned().collect::<Vec<_>>().join(","),
        |s, v, c| {
            s.workspace_dedicated = Some(if v == "none" { Default::default() } else {
                v.split(',').map(|m| m.trim().to_string()).filter(|m| !m.is_empty()).collect()
            });
            Ok(())
        },
        |s| s.workspace_dedicated = None;
    "transition", "animations", "fade | slide | none — apparition du bureau",
        |c| c.general.transition.as_str().into(),
        |s, v, c| { s.transition = Some(choice(v, Transition::parse, "fade, slide, none")?); Ok(()) },
        |s| s.transition = None;
    "fade_ms", "animations", "0..5000 — durée de la transition",
        |c| c.general.fade_ms.to_string(),
        |s, v, c| { s.fade_ms = Some(number(v, 0u64, 5000)?); Ok(()) },
        |s| s.fade_ms = None;
    "crossfade_ms", "animations", "0..10000 — fondu entre deux fonds",
        |c| c.general.crossfade_ms.to_string(),
        |s, v, c| { s.crossfade_ms = Some(number(v, 0u64, 10000)?); Ok(()) },
        |s| s.crossfade_ms = None;
    "theme.preset", "theme", "pack du moteur de thème (nom ou identifiant) — applique ses couleurs et ses effets",
        |c| c.general.theme.name.clone(),
        |s, v, c| {
            // Packs du moteur (embarqués, installés) : il fait foi (MLG-231).
            let all = crate::theme_engine::bridge::catalog(&crate::theme_engine::paths::Roots::from_env());
            let e = all.iter().find(|e| e.id.eq_ignore_ascii_case(v) || e.theme.name.eq_ignore_ascii_case(v)).ok_or_else(|| {
                let noms: Vec<&str> = all.iter().map(|e| e.theme.name.as_str()).collect();
                crate::tr!("« {v} » : thèmes livrés {p0}", v = v, p0 = noms.join(", "))
            })?;
            s.apply_theme(&e.theme);
            Ok(())
        },
        |s| { s.theme_name = None; s.fg = None; s.bg = None; s.accent = None; s.effects.clear(); };
    "theme.reduced_transparency", "theme", "on | off — transparence réduite : surfaces presque opaques, sans reflet ni réfraction",
        |c| on_off(c.general.reduced_transparency),
        |s, v, c| { s.reduced_transparency = Some(parse_bool(v)?); Ok(()) },
        |s| s.reduced_transparency = None;
    "theme.increase_contrast", "theme", "on | off — contraste renforcé : texte à 7:1, repères à 4,5:1, surfaces plus denses",
        |c| on_off(c.general.increase_contrast),
        |s, v, c| { s.increase_contrast = Some(parse_bool(v)?); Ok(()) },
        |s| s.increase_contrast = None;
    "theme.text_scale", "theme", "0.75..2 — échelle du texte de l'interface (1 : celle du thème)",
        |c| format!("{}", c.general.text_scale),
        |s, v, c| { let x: f32 = v.trim().replace(',', ".").parse().map_err(|_| crate::tr!("« {v} » : nombre attendu", v = v))?; if !(0.75..=2.0).contains(&x) { return Err(crate::i18n::tr("échelle du texte attendue entre 0,75 et 2").to_string()); } s.text_scale = Some(x); Ok(()) },
        |s| s.text_scale = None;
    "theme.reduced_motion", "theme", "on | off — mouvement réduit : ouvertures en fondu court, sans échelle ni déplacement",
        |c| on_off(c.general.reduced_motion),
        |s, v, c| { s.reduced_motion = Some(parse_bool(v)?); Ok(()) },
        |s| s.reduced_motion = None;
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

/// Slots de la palette du thème : ceux du moteur (sémantiques et dérivés),
/// plus les couleurs libres du pack actif publiées dans `palette.json`.
fn palette_slots() -> Vec<String> {
    let mut slots: Vec<String> =
        crate::theme_engine::resolve::from_theme(&crate::config::Theme::default())
            .palette
            .into_keys()
            .collect();
    if let Some(doc) =
        crate::theme_engine::state::read_palette(&crate::theme_engine::paths::Roots::from_env())
    {
        slots.extend(doc.colors.into_keys());
    }
    slots.sort();
    slots.dedup();
    slots
}

fn known_slot(slot: &str) -> bool {
    palette_slots().iter().any(|s| s == slot)
}

/// Effet d'une clé `theme.effects.<endroit>.<opacity|blur|adaptive>`.
fn effect_key(key: &str) -> Option<(Place, &str)> {
    let rest = key.strip_prefix("theme.effects.")?;
    let (place, field) = rest.split_once('.')?;
    let place = Place::parse(place)?;
    matches!(field, "opacity" | "blur" | "adaptive").then_some((place, field))
}

/// Couleur de la barre : `bar.color.<emplacement>` ou `bar.module_color.<module>`.
enum BarColorKey<'a> {
    Slot(&'a str),
    Module(&'a str),
}

fn bar_color_key(key: &str) -> Option<BarColorKey<'_>> {
    if let Some(k) = key.strip_prefix("bar.color.") {
        return crate::config::BAR_COLOR_SLOTS
            .iter()
            .any(|(s, _)| *s == k)
            .then_some(BarColorKey::Slot(k));
    }
    key.strip_prefix("bar.module_color.")
        .filter(|m| !m.is_empty())
        .map(BarColorKey::Module)
}

/// Clés dynamiques : `apps.<rôle>`, `assign.<app>` (workspaces), `widget.<type>`
/// (widgets), `bar.color.<emplacement>`, `bar.module_color.<module>` (barre) et
/// `theme.effects.<endroit>.<opacity|blur|adaptive>` (thème).
fn dynamic_get(key: &str, c: &Config) -> Option<String> {
    // Options d'apparence des composants (MLG-279).
    if let Some(o) = crate::design_options::find(key) {
        return Some(o.shown(c));
    }
    // Surcharges par écran (MLG-281).
    if let Some((sel, o)) = crate::design_options::screen_find(key) {
        return Some(
            c.general
                .plugin_options
                .get(&format!("ui.screen.{sel}"))
                .and_then(|m| m.get(o.key))
                .map(|v| v.display())
                .unwrap_or_default(),
        );
    }
    if let Some(k) = bar_color_key(key) {
        let colors = c.bar.as_ref().map(|b| &b.colors);
        let v = match k {
            BarColorKey::Slot(s) => colors.and_then(|c| c.slot(s)),
            BarColorKey::Module(m) => colors.and_then(|c| c.modules.get(m)),
        };
        return Some(v.map(|c| c.to_text()).unwrap_or_else(|| "theme".into()));
    }
    if let Some((place, field)) = effect_key(key) {
        let e = c.general.theme.effect(place);
        return Some(match field {
            "blur" => on_off(e.blur),
            "adaptive" => e.adaptive.map_or_else(
                || "off".to_string(),
                |m| format!("{}", (m * 100.0).round() / 100.0),
            ),
            _ => format!("{}", (e.opacity * 100.0).round() / 100.0),
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
    if let Some((l, what)) = panorama_zone_key(key) {
        let p = &c.general.panorama;
        return Some(match what {
            "panes" => p.panes_of(l).join(","),
            _ => p
                .first_of(l)
                .map(|f| f.id().to_string())
                .unwrap_or_else(|| "auto".into()),
        });
    }
    if let Some(k) = key.strip_prefix("screen_zones.") {
        return Some(
            c.general
                .screen_zones
                .get(k)
                .map(crate::zones::Split::id)
                .unwrap_or_else(|| "1".into()),
        );
    }
    if let Some(k) = key.strip_prefix("forced.") {
        return Some(
            c.general
                .forced_modes
                .get(k)
                .cloned()
                .unwrap_or_else(|| "none".into()),
        );
    }
    if key == "perf.profile" {
        let o = &c.general.fx.options;
        return Some(match crate::perf::active(o) {
            Some(p) => p.id().into(),
            None if crate::perf::untouched(o) => "hyprland".into(),
            None => "custom".into(),
        });
    }
    if let Some(k) = key.strip_prefix("fx.") {
        let fx = &c.general.fx;
        if let Some(leaf) = k.strip_prefix("anim.") {
            return Some(match fx.anims.get(leaf) {
                None => "hyprland".into(),
                Some(a) if !a.enabled => "off".into(),
                Some(a) => {
                    let mut v = vec![format!("{}", a.speed), a.curve.clone()];
                    v.extend(a.style.clone());
                    v.join(" ")
                }
            });
        }
        if crate::fx::opt(k).is_some() {
            return Some(
                fx.options
                    .get(k)
                    .map(crate::tiling::OptValue::display)
                    .unwrap_or_else(|| "hyprland".into()),
            );
        }
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
        let t = crate::widgets::plugins::canonical(t);
        return Some(on_off(
            c.widgets.iter().any(|w| w.type_id == t && !w.in_bar),
        ));
    }
    None
}

/// Clés renommées : l'ancien nom (français) reste accepté par `get` et `set`.
fn canonical_key(key: &str) -> &str {
    match key {
        "panorama.flux_ambiance" => "panorama.stream_mood",
        k => k,
    }
}

/// Valeurs admises d'une option Hyprland (layouts, effets), pour l'aide.
fn kind_values(k: &crate::tiling::Kind) -> String {
    use crate::tiling::Kind;
    match k {
        Kind::Bool => "on | off".into(),
        Kind::Num { min, max, .. } => format!("{min}..{max}"),
        Kind::IntChoice(l) => l
            .iter()
            .map(|(n, _)| n.to_string())
            .collect::<Vec<_>>()
            .join(" | "),
        Kind::Choice(l) => l.iter().map(|(s, _)| *s).collect::<Vec<_>>().join(" | "),
        Kind::Color => "#rrggbb[aa]".into(),
        Kind::Gradient => "#rrggbb #rrggbb… [<angle>deg]".into(),
        Kind::Text => crate::i18n::tr("texte").into(),
        Kind::Sides { max } => format!("0..{max} | t,r,b,l"),
    }
}

/// Aide d'une clé dynamique (`hyprdesk config list`) : valeurs admises, puis
/// ce qu'elle règle, comme pour les clés fixes.
fn dynamic_help(key: &str, c: &Config) -> String {
    if let Some(o) = crate::design_options::find(key) {
        return tr(o.help).to_string();
    }
    if let Some((_, o)) = crate::design_options::screen_find(key) {
        return tr(o.help).to_string();
    }
    use crate::i18n::tr;
    if let Some(k) = bar_color_key(key) {
        return match k {
            BarColorKey::Slot(s) => crate::tr!(
                "#rrggbb | #sombre/#clair | <slot du thème> | theme — couleur de la barre : {label}",
                label = crate::config::BAR_COLOR_SLOTS
                    .iter()
                    .find(|(id, _)| *id == s)
                    .map_or(s, |(_, l)| tr(l))
            ),
            BarColorKey::Module(m) => {
                crate::tr!(
                    "#rrggbb | #sombre/#clair | <slot du thème> | theme — couleur du module « {m} »",
                    m = m
                )
            }
        };
    }
    if let Some((place, field)) = effect_key(key) {
        return if field == "blur" {
            crate::tr!(
                "on | off — flou derrière : {place}",
                place = tr(place.label())
            )
        } else if field == "adaptive" {
            crate::tr!(
                "off | on | 0..1 (plafond) — verre densifié selon le fond d'écran : {place}",
                place = tr(place.label())
            )
        } else {
            crate::tr!(
                "0..1 — opacité du fond : {place}",
                place = tr(place.label())
            )
        };
    }
    if let Some(ws) = key.strip_prefix("layout.") {
        return crate::tr!(
            "<layout> | none — layout du workspace {ws} (none : le layout par défaut)",
            ws = ws
        );
    }
    if key == "perf.profile" {
        return crate::i18n::tr(
            "performance | balanced | quality | hyprland — priorité des effets (custom : réglés à la main ; détail dans [animations])",
        )
        .to_string();
    }
    if let Some(k) = key.strip_prefix("fx.") {
        if let Some(name) = k.strip_prefix("anim.") {
            let label = crate::fx::leaf(name).map_or(name, |l| tr(l.label));
            return crate::tr!(
                "<vitesse> <courbe> [style] | off | hyprland — animation : {label}",
                label = label
            );
        }
        if let Some(o) = crate::fx::opt(k) {
            return format!("{} | hyprland — {}", kind_values(&o.kind), tr(o.label));
        }
    }
    if let Some(o) = crate::tiling::opt(key) {
        return format!("{} | none — {}", kind_values(&o.kind), tr(o.label));
    }
    if let Some(ws) = key.strip_prefix("workspace.") {
        let declared = c
            .general
            .workspace_monitors_declared
            .values()
            .flatten()
            .any(|w| w == ws);
        return if declared {
            crate::tr!(
                "<écran> | primary | auto — écran du workspace {ws}",
                ws = ws
            )
        } else {
            crate::tr!(
                "<écran> | primary | auto — écran du workspace {ws} (placé d'office : écrans dédiés)",
                ws = ws
            )
        };
    }
    if let Some(r) = key.strip_prefix("apps.") {
        let label = crate::apps::role(r).map_or(r, |role| tr(role.label));
        return crate::tr!(
            "<application> — application par défaut : {label}",
            label = label
        );
    }
    if key.starts_with("assign.") {
        return tr("<workspace> [silent] | none — workspace où l'application s'ouvre").to_string();
    }
    if let Some(t) = key.strip_prefix("widget.") {
        let name = c
            .catalog
            .iter()
            .find(|w| w.id == t)
            .map_or(t, |w| w.name.as_str());
        return crate::tr!(
            "on | off — widget « {name} » sur le bureau",
            name = tr(name)
        );
    }
    if let Some(id) = key.strip_prefix("plugin.") {
        let label = crate::ui::plugins::find(id).map_or(id, |p| tr(p.label));
        return crate::tr!(
            "on | off — paquet « {label} » (off : retiré des Réglages et arrêté)",
            label = label
        );
    }
    String::new()
}

fn dynamic_set(
    key: &str,
    v: Option<&str>,
    s: &mut Settings,
    c: &Config,
) -> Option<Result<(), String>> {
    if crate::design_options::find(key).is_some()
        || crate::design_options::screen_find(key).is_some()
    {
        return Some(match v {
            Some(v) => crate::design_options::set(s, c, key, v),
            None => {
                crate::design_options::unset(s, c, key);
                Ok(())
            }
        });
    }
    if let Some(k) = bar_color_key(key) {
        let value = match v.map(str::trim) {
            None | Some("theme" | "none" | "") => None,
            Some(v) => match crate::config::ColorRef::parse(v) {
                Some(crate::config::ColorRef::Slot(slot)) if !known_slot(&slot) => {
                    return Some(Err(crate::tr!(
                        "« {v} » : ni une couleur, ni un slot de la palette du thème ({slots})",
                        v = v,
                        slots = palette_slots().join(", ")
                    )));
                }
                Some(c) => Some(c),
                None => {
                    return Some(Err(crate::tr!(
                        "« {v} » : couleur illisible (#rrggbb, #sombre/#clair ou slot du thème comme active)",
                        v = v
                    )));
                }
            },
        };
        let colors = &mut bar(s, c).colors;
        match k {
            BarColorKey::Slot(slot) => colors.set_slot(slot, value),
            BarColorKey::Module(m) => match value {
                Some(v) => {
                    colors.modules.insert(m.to_string(), v);
                }
                None => {
                    colors.modules.remove(m);
                }
            },
        }
        return Some(Ok(()));
    }
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
            "adaptive" => match v.trim() {
                "off" | "false" | "no" => e.adaptive = None,
                "on" | "true" | "yes" => e.adaptive = Some(1.0),
                n => match number(n, 0.0f32, 1.0) {
                    Ok(m) => e.adaptive = Some(m),
                    Err(err) => return Some(Err(err)),
                },
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
    if let Some((l, what)) = panorama_zone_key(key) {
        let mut p = s
            .panorama
            .clone()
            .unwrap_or_else(|| c.general.panorama.clone());
        match (what, v.map(str::trim)) {
            ("panes", None | Some("")) => p.set_panes(l, Vec::new()),
            ("panes", Some(raw)) => {
                let list: Vec<String> = raw
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| crate::panorama::PAGES.iter().any(|(id, _)| id == x))
                    .collect();
                if list.is_empty() {
                    return Some(Err(crate::i18n::tr(
                        "widgets connus : media, system, clock, video",
                    )
                    .into()));
                }
                p.set_panes(l, list);
            }
            (_, None | Some("" | "auto")) => p.set_first(l, None),
            (_, Some(raw)) => match crate::zones::First::parse(raw) {
                Some(f) if l.grid().is_some_and(crate::zones::Grid::has_first) => {
                    p.set_first(l, Some(f))
                }
                _ => return Some(Err(crate::i18n::tr("parts : 1/2, 1/3, auto").into())),
            },
        }
        s.panorama = Some(p);
        return Some(Ok(()));
    }
    if let Some(k) = key.strip_prefix("screen_zones.") {
        if k.is_empty() {
            return Some(Err(crate::i18n::tr("screen_zones.<écran> attendu").into()));
        }
        let mut m = s
            .screen_zones
            .clone()
            .unwrap_or_else(|| c.general.screen_zones.clone());
        match v.map(str::trim) {
            None | Some("" | "none" | "1") => {
                m.remove(k);
            }
            Some(raw) => match crate::zones::Split::parse(raw) {
                Some(split) => {
                    m.insert(k.to_string(), split);
                }
                None => {
                    return Some(Err(crate::tr!(
                        "« {v} » n'est pas un découpage (1, 1x2, 1x2:1/3, 1x3, 1x3:1/2, 2x2, 3x3)",
                        v = raw
                    )));
                }
            },
        }
        s.screen_zones = Some(m);
        return Some(Ok(()));
    }
    if let Some(k) = key.strip_prefix("forced.") {
        if k.is_empty() || k == "ws:" {
            return Some(Err(crate::i18n::tr(
                "forced.<écran> ou forced.ws:<workspace> attendu",
            )
            .into()));
        }
        let mut m = s
            .forced_modes
            .clone()
            .unwrap_or_else(|| c.general.forced_modes.clone());
        match v.map(str::trim) {
            None | Some("none" | "") => {
                m.remove(k);
            }
            Some(mode) if crate::config::lua::FORCED_MODES.contains(&mode) => {
                m.insert(k.to_string(), mode.to_string());
            }
            Some(other) => {
                return Some(Err(crate::tr!(
                    "« {v} » n'est pas un mode (no-desktop, launcher-only, full-desktop)",
                    v = other
                )));
            }
        }
        s.forced_modes = Some(m);
        return Some(Ok(()));
    }
    if key == "perf.profile" {
        let mut fx = s.fx.clone().unwrap_or_else(|| c.general.fx.clone());
        match v.map(str::trim) {
            None | Some("" | "none" | "hyprland") => crate::perf::clear(&mut fx.options),
            Some(id) => match crate::perf::Profile::from_id(id) {
                Some(p) => p.apply(&mut fx.options),
                None => {
                    return Some(Err(crate::tr!(
                        "« {v} » : profil inconnu (performance, balanced, quality, hyprland)",
                        v = id
                    )));
                }
            },
        }
        s.fx = Some(fx);
        return Some(Ok(()));
    }
    if let Some(k) = key.strip_prefix("fx.") {
        let mut fx = s.fx.clone().unwrap_or_else(|| c.general.fx.clone());
        let v = v
            .map(str::trim)
            .filter(|v| !matches!(*v, "" | "none" | "hyprland"));
        if let Some(name) = k.strip_prefix("anim.") {
            let Some(leaf) = crate::fx::leaf(name) else {
                return Some(Err(crate::tr!(
                    "élément animé « {n} » inconnu (hyprdesk config list animations)",
                    n = name
                )));
            };
            match v {
                None => {
                    fx.anims.remove(name);
                }
                Some(raw) => match parse_anim(leaf, raw) {
                    Ok(a) => {
                        fx.anims.insert(name.to_string(), a);
                    }
                    Err(e) => return Some(Err(e)),
                },
            }
        } else if let Some(o) = crate::fx::opt(k) {
            match v {
                None => {
                    fx.options.remove(k);
                }
                Some(raw) => match o.parse(raw) {
                    Ok(val) => {
                        fx.options.insert(k.to_string(), val);
                    }
                    Err(e) => return Some(Err(e)),
                },
            }
        } else {
            return Some(Err(crate::tr!(
                "effet « {k} » inconnu (hyprdesk config list animations)",
                k = k
            )));
        }
        s.fx = Some(fx);
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
        let t = crate::widgets::plugins::canonical(t);
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

/// `panorama.zones.<disposition>.panes|first` : (disposition, réglage).
fn panorama_zone_key(key: &str) -> Option<(crate::panorama::config::Layout, &str)> {
    let rest = key.strip_prefix("panorama.zones.")?;
    let (l, what) = rest.rsplit_once('.')?;
    let l = crate::panorama::config::Layout::parse(l).filter(|l| l.grid().is_some())?;
    matches!(what, "panes" | "first").then_some((l, what))
}

fn usage() -> String {
    crate::i18n::tr_lines(USAGE)
}

const USAGE: &str =
    "Usage : hyprdesk config [list [paquet]] | get <clé> | set <clé> <valeur> | unset <clé> | reset [<composant>|all] [--yes]
Clés dynamiques : assign.<app> <workspace> [silent] | none ; widget.<type> on | off ;
bar.color.<text|background|accent|warning|critical> #rrggbb | theme ; bar.module_color.<module> #rrggbb | theme ;
fx.<section.option> <valeur> | hyprland ; fx.anim.<élément> on | off | <vitesse> [courbe] [style] | hyprland ;
forced.<écran> | forced.ws:<workspace> no-desktop | launcher-only | full-desktop | none
screen_zones.<écran> 1 | 1x2 | 1x2:1/3 | 1x3 | 1x3:1/2 | 2x2 | 3x3
panorama.zones.<1x2|1x3|2x2|3x3>.panes <widget>,… ; panorama.zones.<1x2|1x3>.first 1/2 | 1/3 | auto
Historique (30 jours) : history [--json] | diff <id> | rollback <id|AAAA-MM-JJ[THH:MM]> [--yes]";

/// `fx.anim.<élément>` : `on`, `off`, ou `<vitesse> [courbe] [style]`.
fn parse_anim(leaf: &crate::fx::Leaf, raw: &str) -> Result<crate::fx::Anim, String> {
    let mut a = crate::fx::Anim::new_for(leaf);
    let mut parts = raw.split_whitespace();
    match parts.next() {
        Some("off" | "false" | "no") => {
            a.enabled = false;
            return Ok(a);
        }
        Some("on" | "true" | "yes") | None => return Ok(a),
        Some(speed) => {
            a.speed = speed
                .replace(',', ".")
                .parse::<f64>()
                .ok()
                .filter(|v| (0.1..=100.0).contains(v))
                .ok_or_else(|| {
                    crate::tr!(
                        "« {v} » : vitesse entre 0.1 et 100 (dixièmes de seconde)",
                        v = speed
                    )
                })?;
        }
    }
    if let Some(curve) = parts.next() {
        if crate::fx::curve(curve).is_none() {
            let known: Vec<&str> = crate::fx::CURVES.iter().map(|c| c.name).collect();
            return Err(crate::tr!(
                "courbe « {c} » inconnue ({p0})",
                c = curve,
                p0 = known.join(", ")
            ));
        }
        a.curve = curve.to_string();
    }
    if let Some(style) = parts.next() {
        if !leaf.styles.contains(&style) {
            return Err(crate::tr!(
                "style « {s} » non admis pour {l} ({p0})",
                s = style,
                l = leaf.name,
                p0 = leaf.styles.join(", ")
            ));
        }
        a.style = Some(style.to_string());
    }
    Ok(a)
}

pub fn run(config_path: &Path, args: &[String]) -> Result<String, String> {
    let full = || load_file(config_path).map(|l| l.config);
    // Historique de la configuration : écritures faites d'ici, motif « cli ».
    let _reason = crate::history::reason("cli");
    match args.first().map(String::as_str).unwrap_or("list") {
        "history" | "historique" => history_list(config_path, args.iter().any(|a| a == "--json")),
        "diff" => {
            let id = args.get(1).ok_or_else(usage)?;
            let (snap, diffs) = crate::history::diff(config_path, id)?;
            if diffs.is_empty() {
                return Ok(crate::tr!("{id} : identique à l'état actuel", id = snap.id));
            }
            let mut out = format!("{} — {}\n", snap.id, describe(&snap));
            for d in &diffs {
                out += &format!("\n{} ({})\n{}", d.file, d.change.label(), d.patch);
            }
            Ok(out.trim_end().to_string())
        }
        "rollback" | "retour" => {
            let yes = args.iter().any(|a| a == "--yes");
            let id = args[1..].iter().find(|a| *a != "--yes").ok_or_else(usage)?;
            if !yes {
                let (snap, plan) = crate::history::plan(config_path, id)?;
                let mut out =
                    crate::tr!("retour à {id} ({p0}) :", id = snap.id, p0 = describe(&snap));
                if plan.restore.is_empty() && plan.remove.is_empty() {
                    out += "\n  ";
                    out += crate::i18n::tr("rien à changer : identique à l'état actuel");
                }
                for f in &plan.restore {
                    out += "\n  ";
                    out += &crate::tr!("restauré : {f}", f = f);
                }
                for f in &plan.remove {
                    out += "\n  ";
                    out += &crate::tr!("supprimé : {f}", f = f);
                }
                out += "\n";
                out += &crate::tr!(
                    "hyprdesk config rollback {id} --yes pour l'appliquer (l'état actuel est gardé dans l'historique)",
                    id = snap.id
                );
                return Err(out);
            }
            let r = crate::history::rollback(config_path, id)?;
            let mut out = crate::tr!(
                "retour à {id} fait : {n} fichier(s) restauré(s), {m} supprimé(s)",
                id = r.id,
                n = r.plan.restore.len(),
                m = r.plan.remove.len()
            );
            for f in r.plan.restore.iter().chain(&r.plan.remove) {
                out += &format!("\n  {f}");
            }
            out += "\n";
            out += &crate::tr!(
                "état d'avant gardé : {id} (hyprdesk config rollback {id} pour y revenir)",
                id = r.safety
            );
            Ok(out)
        }
        // Configuration par défaut, d'un composant ou de tout (MLG-202) :
        // sans --yes, ce qui changerait ; avec, la remise (copie d'abord).
        "reset" | "defaults" => {
            let yes = args.iter().any(|a| a == "--yes");
            let name = args[1..].iter().find(|a| *a != "--yes");
            let target = match (name, yes) {
                (Some(n), _) => crate::defaults::resolve(n)?,
                // Ancienne forme : `hyprdesk config reset --yes` remet tout.
                (None, true) => None,
                (None, false) => return Ok(defaults_help()),
            };
            let before = full()?;
            let plan = crate::defaults::plan(config_path, target)?;
            let what = match target {
                Some(id) => crate::tr!("{l} par défaut", l = plugin_label(id)),
                None => crate::i18n::tr("toute la configuration par défaut").to_string(),
            };
            let detail = crate::defaults::describe(&plan, &before);
            if !yes {
                let arg = target.unwrap_or("all");
                return Ok(crate::tr!(
                    "{what} :{detail}\nhyprdesk config reset {arg} --yes pour le faire (une copie est gardée)",
                    what = what,
                    detail = detail,
                    arg = arg
                ));
            }
            let live = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some();
            let backup =
                crate::defaults::apply(config_path, &plan, crate::defaults::Apply { live })?;
            crate::defaults::notify_daemon(target);
            Ok(crate::tr!(
                "{what} : fait.{detail}\ncopie de l'ancienne configuration : {b}",
                what = what,
                detail = detail,
                b = backup.display()
            ))
        }
        "list" | "ls" => {
            let c = full()?;
            let only = args.get(1).map(String::as_str);
            let mut out = String::new();
            // Clés déjà montrées : une clé partagée par deux paquets (les effets
            // de Performance, détaillés dans Animations) ne sort qu'une fois.
            let mut seen = std::collections::BTreeSet::<String>::new();
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
                    // Couleurs de la barre, puis celles des modules déjà réglées.
                    "bar" => crate::config::BAR_COLOR_SLOTS
                        .iter()
                        .map(|(k, _)| format!("bar.color.{k}"))
                        .chain(
                            c.bar
                                .iter()
                                .flat_map(|b| b.colors.modules.keys())
                                .map(|m| format!("bar.module_color.{m}")),
                        )
                        .collect(),
                    "animations" => crate::fx::OPTIONS
                        .iter()
                        .map(|o| format!("fx.{}", o.path()))
                        .chain(
                            crate::fx::LEAVES
                                .iter()
                                .map(|l| format!("fx.anim.{}", l.name)),
                        )
                        .collect(),
                    "performance" => std::iter::once("perf.profile".to_string())
                        .chain(crate::perf::items().map(|i| format!("fx.{}", i.path)))
                        .collect(),
                    "theme" => Place::ALL
                        .iter()
                        .flat_map(|p| {
                            let o = format!("theme.effects.{p0}.opacity", p0 = p.as_str());
                            let b = format!("theme.effects.{p0}.blur", p0 = p.as_str());
                            let a = format!("theme.effects.{p0}.adaptive", p0 = p.as_str());
                            if p.blurrable() {
                                vec![o, b, a]
                            } else {
                                vec![o, a]
                            }
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                if keys.is_empty() && extra.is_empty() {
                    continue;
                }
                out += &format!("[{}] {}\n", p.id, crate::i18n::tr(p.label));
                let extra: Vec<String> = extra
                    .into_iter()
                    .filter(|n| seen.insert(n.clone()))
                    .collect();
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
                    let line = format!(
                        "  {:<18} = {:<26} {}",
                        name,
                        if v.is_empty() { "—".into() } else { v },
                        dynamic_help(&name, &c)
                    );
                    out += line.trim_end();
                    out += "\n";
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
            let key = canonical_key(args.get(1).ok_or_else(usage)?);
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
            let key = canonical_key(args.get(1).ok_or_else(usage)?);
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
            // Le moteur de thème fait foi (MLG-231) : un thème changé ici lui
            // est posé tout de suite, démon lancé ou non.
            if after.general.theme != c.general.theme
                && let Err(e) = crate::theme_engine::bridge::adopt(
                    &crate::theme_engine::paths::Roots::from_env(),
                    &after.general.theme,
                    "cli",
                )
            {
                tracing::warn!("{}", crate::tr!("moteur de thème : {e}", e = e));
            }
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

/// Date, heure et motif d'un instantané.
fn describe(s: &crate::history::Snapshot) -> String {
    format!(
        "{} · {}",
        s.time().format("%Y-%m-%d %H:%M:%S"),
        crate::history::reason_label(&s.reason)
    )
}

/// `hyprdesk config history [--json]`.
fn history_list(config_path: &Path, json: bool) -> Result<String, String> {
    let entries = crate::history::entries(config_path);
    if json {
        let v: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.snap.id,
                    "time": e.snap.time().to_rfc3339(),
                    "reason": e.snap.reason,
                    "label": crate::history::reason_label(&e.snap.reason),
                    "files": e.snap.files,
                    "changed": e.changed,
                })
            })
            .collect();
        return serde_json::to_string_pretty(&v).map_err(|e| e.to_string());
    }
    if entries.is_empty() {
        return Ok(crate::i18n::tr("historique vide : aucun instantané pour l'instant").into());
    }
    let mut out = String::new();
    for e in &entries {
        let changed = if e.changed.is_empty() {
            "—".to_string()
        } else {
            e.changed.join(", ")
        };
        out += &format!("{:<34} {}  → {changed}\n", e.snap.id, describe(&e.snap));
    }
    out += crate::i18n::tr(
        "(chaque instantané garde l'état d'avant la modification ; → : fichiers changés ensuite)",
    );
    Ok(out)
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
    fn panorama_design_persists_and_rejects_invalid_geometry() {
        let p = tmp();
        run_s(&p, &["set", "panorama.design.preset", "studio"]).unwrap();
        assert_eq!(run_s(&p, &["get", "panorama.layout"]).unwrap(), "dashboard");
        assert_eq!(
            run_s(&p, &["get", "panorama.design.media_style"]).unwrap(),
            "vinyl"
        );
        run_s(&p, &["set", "panorama.design.clock_style", "analog"]).unwrap();
        run_s(
            &p,
            &["set", "panorama.design.wallpaper", "/tmp/my forest.jpg"],
        )
        .unwrap();
        run_s(&p,&["set","panorama.design.tiles",r#"[{"widget":"clock","x":0,"y":0,"w":0.4,"h":1},{"widget":"calendar","x":0.5,"y":0,"w":0.5,"h":1}]"#]).unwrap();
        let before = run_s(&p, &["get", "panorama.design.tiles"]).unwrap();
        assert!(
            run_s(
                &p,
                &[
                    "set",
                    "panorama.design.tiles",
                    r#"[{"widget":"clock","x":0.9,"y":0,"w":0.4,"h":1}]"#
                ]
            )
            .is_err()
        );
        assert_eq!(
            run_s(&p, &["get", "panorama.design.tiles"]).unwrap(),
            before
        );
        assert_eq!(
            run_s(&p, &["get", "panorama.design.wallpaper"]).unwrap(),
            "/tmp/my forest.jpg"
        );
        assert_eq!(
            run_s(&p, &["get", "panorama.design.clock_style"]).unwrap(),
            "analog"
        );
        run_s(&p, &["unset", "panorama.design.clock_style"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "panorama.design.clock_style"]).unwrap(),
            "digital"
        );
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

    /// Modes imposés par écran ou workspace.
    #[test]
    fn modes_imposes() {
        let p = tmp();
        run_s(&p, &["set", "panorama.zones.2x2.panes", "clock,video"]).unwrap();
        run_s(&p, &["set", "panorama.zones.1x3.first", "1/2"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "panorama.zones.2x2.panes"]).unwrap(),
            "clock,video,system,clock"
        );
        assert_eq!(
            run_s(&p, &["get", "panorama.zones.1x3.first"]).unwrap(),
            "1/2"
        );
        assert!(run_s(&p, &["set", "panorama.zones.2x2.first", "1/2"]).is_err());
        run_s(
            &p,
            &[
                "set",
                "panorama.sources",
                r#"[{"name":"Arte","url":"https://www.arte.tv"}]"#,
            ],
        )
        .unwrap();
        assert!(
            run_s(&p, &["get", "panorama.sources"])
                .unwrap()
                .contains("arte.tv")
        );
        assert_eq!(run_s(&p, &["get", "screen_zones.DP-2"]).unwrap(), "1");
        run_s(&p, &["set", "screen_zones.DP-2", "1x3:1/2"]).unwrap();
        assert_eq!(run_s(&p, &["get", "screen_zones.DP-2"]).unwrap(), "1x3:1/2");
        assert!(run_s(&p, &["set", "screen_zones.DP-2", "4x4"]).is_err());
        run_s(&p, &["unset", "screen_zones.DP-2"]).unwrap();
        assert_eq!(run_s(&p, &["get", "screen_zones.DP-2"]).unwrap(), "1");
        assert_eq!(run_s(&p, &["get", "forced.DP-1"]).unwrap(), "none");
        run_s(&p, &["set", "forced.DP-1", "full-desktop"]).unwrap();
        run_s(&p, &["set", "forced.ws:5", "no-desktop"]).unwrap();
        assert_eq!(run_s(&p, &["get", "forced.DP-1"]).unwrap(), "full-desktop");
        assert_eq!(run_s(&p, &["get", "forced.ws:5"]).unwrap(), "no-desktop");
        assert!(run_s(&p, &["set", "forced.DP-1", "plein"]).is_err());
        run_s(&p, &["unset", "forced.DP-1"]).unwrap();
        assert_eq!(run_s(&p, &["get", "forced.DP-1"]).unwrap(), "none");
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    /// Animations et effets en ligne de commande (fx.*).
    #[test]
    fn effets_en_ligne_de_commande() {
        let p = tmp();
        assert_eq!(
            run_s(&p, &["get", "fx.decoration.rounding"]).unwrap(),
            "hyprland"
        );
        assert_eq!(run_s(&p, &["get", "perf.profile"]).unwrap(), "hyprland");
        run_s(&p, &["set", "perf.profile", "performance"]).unwrap();
        assert_eq!(run_s(&p, &["get", "perf.profile"]).unwrap(), "performance");
        assert_eq!(
            run_s(&p, &["get", "fx.decoration.blur.enabled"]).unwrap(),
            "off"
        );
        run_s(&p, &["set", "fx.decoration.blur.enabled", "on"]).unwrap();
        assert_eq!(run_s(&p, &["get", "perf.profile"]).unwrap(), "custom");
        assert!(run_s(&p, &["set", "perf.profile", "turbo"]).is_err());
        run_s(&p, &["set", "perf.profile", "hyprland"]).unwrap();
        assert_eq!(run_s(&p, &["get", "perf.profile"]).unwrap(), "hyprland");
        run_s(&p, &["set", "fx.decoration.rounding", "12"]).unwrap();
        assert_eq!(run_s(&p, &["get", "fx.decoration.rounding"]).unwrap(), "12");
        assert!(run_s(&p, &["set", "fx.decoration.rounding", "99"]).is_err());
        assert!(run_s(&p, &["set", "fx.decoration.inconnu", "1"]).is_err());
        run_s(&p, &["set", "fx.anim.windowsIn", "3 ease-out popin"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "fx.anim.windowsIn"]).unwrap(),
            "3 ease-out popin"
        );
        run_s(&p, &["set", "fx.anim.windowsOut", "off"]).unwrap();
        assert_eq!(run_s(&p, &["get", "fx.anim.windowsOut"]).unwrap(), "off");
        assert!(run_s(&p, &["set", "fx.anim.windowsIn", "3 ease-out fade"]).is_err());
        assert!(run_s(&p, &["set", "fx.anim.nimporte", "on"]).is_err());
        assert!(
            run_s(&p, &["list", "animations"])
                .unwrap()
                .contains("fx.anim.windowsIn")
        );
        run_s(&p, &["unset", "fx.anim.windowsIn"]).unwrap();
        run_s(&p, &["set", "fx.decoration.rounding", "hyprland"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "fx.anim.windowsIn"]).unwrap(),
            "hyprland"
        );
        assert_eq!(
            run_s(&p, &["get", "fx.decoration.rounding"]).unwrap(),
            "hyprland"
        );
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    /// Style et marge de la barre en ligne de commande.
    #[test]
    fn style_et_marge_de_la_barre() {
        let p = tmp();
        // Barre absente : champs vides ; les défauts s'écrivent à l'activation.
        assert_eq!(run_s(&p, &["get", "bar.style"]).unwrap(), "");
        assert_eq!(run_s(&p, &["get", "bar.margin"]).unwrap(), "");
        run_s(&p, &["set", "bar", "on"]).unwrap();
        run_s(&p, &["set", "bar.style", "graphite"]).unwrap();
        assert_eq!(run_s(&p, &["get", "bar.style"]).unwrap(), "graphite");
        // Le style pose toute la barre : géométrie et composition.
        assert_eq!(run_s(&p, &["get", "bar.position"]).unwrap(), "bottom");
        assert_eq!(run_s(&p, &["get", "bar.height"]).unwrap(), "48");
        assert_eq!(run_s(&p, &["get", "bar.margin"]).unwrap(), "0");
        // La marge reste ajustable ensuite.
        run_s(&p, &["set", "bar.margin", "12"]).unwrap();
        assert_eq!(run_s(&p, &["get", "bar.margin"]).unwrap(), "12");
        // Refus : style inconnu, marge hors bornes.
        assert!(run_s(&p, &["set", "bar.style", "pastel"]).is_err());
        assert!(run_s(&p, &["set", "bar.margin", "64"]).is_err());
        run_s(&p, &["unset", "bar.style"]).unwrap();
        assert_eq!(run_s(&p, &["get", "bar.style"]).unwrap(), "continuity");
        assert_eq!(run_s(&p, &["get", "bar.position"]).unwrap(), "top");
        assert_eq!(run_s(&p, &["get", "bar.height"]).unwrap(), "36");
        assert_eq!(run_s(&p, &["get", "bar.margin"]).unwrap(), "0");
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn anciens_noms_et_liste_coherente() {
        let p = tmp();
        // Ancienne clé acceptée, lue sous son nouveau nom.
        run_s(&p, &["set", "panorama.flux_ambiance", "ocean"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "panorama.stream_mood"]).unwrap(),
            "ocean"
        );
        // Anciennes valeurs acceptées, réécrites en anglais.
        run_s(&p, &["set", "workspace_selector", "miniatures"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "workspace_selector"]).unwrap(),
            "thumbnails"
        );
        run_s(&p, &["set", "apps_style", "plein"]).unwrap();
        assert_eq!(run_s(&p, &["get", "apps_style"]).unwrap(), "sheet");
        run_s(&p, &["set", "bar", "on"]).unwrap();
        run_s(&p, &["set", "bar.style", "continuite"]).unwrap();
        assert_eq!(run_s(&p, &["get", "bar.style"]).unwrap(), "continuity");
        // Booléens en on/off partout.
        assert_eq!(run_s(&p, &["get", "panorama.design.date"]).unwrap(), "on");
        assert_eq!(run_s(&p, &["get", "bar.convex.auto_hide"]).unwrap(), "on");
        // Liste : chaque clé une seule fois, et toutes décrites.
        let list = run_s(&p, &["list"]).unwrap();
        let keys: Vec<&str> = list
            .lines()
            .filter(|l| l.starts_with("  "))
            .filter_map(|l| l.split(" = ").next())
            .map(str::trim)
            .collect();
        let unique: std::collections::BTreeSet<&&str> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len(), "clé répétée dans la liste");
        for l in list.lines().filter(|l| l.starts_with("  ")) {
            assert!(l.contains(" — "), "clé sans description : {l}");
        }
    }

    #[test]
    fn convex_parameters_roundtrip_through_cli() {
        let p = tmp();
        run_s(&p, &["set", "bar", "on"]).unwrap();
        for (key, value) in [
            ("bar.convex.frame_width", "10"),
            ("bar.convex.radius", "56"),
            ("bar.convex.duration", "420"),
            ("bar.convex.attached_panels", "off"),
        ] {
            run_s(&p, &["set", key, value]).unwrap();
            assert_eq!(run_s(&p, &["get", key]).unwrap(), value);
        }
        assert!(run_s(&p, &["set", "bar.convex.frame_width", "21"]).is_err());
        run_s(&p, &["unset", "bar.convex.radius"]).unwrap();
        assert_eq!(run_s(&p, &["get", "bar.convex.radius"]).unwrap(), "44");
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    /// Forme d'un pack (MLG-276) : clés d'apparence admises, posées avec la
    /// validation du CLI ; raccourcis et commandes refusés.
    #[test]
    fn forme_des_packs() {
        assert_eq!(
            pack_key("bar.thickness").map(|k| k.name),
            Some("bar.height")
        );
        assert_eq!(
            pack_key("dock.position").map(|k| k.name),
            Some("launcher_position")
        );
        assert_eq!(pack_key("lock.dim").map(|k| k.name), Some("lock_dim"));
        assert_eq!(
            pack_key("workspaces.selector").map(|k| k.name),
            Some("workspace_selector")
        );
        assert!(pack_key("bar.monitor").is_none());
        assert!(pack_key("keys.launcher").is_none());
        assert!(pack_key("desktop.terminal").is_none());
        assert!(pack_key("lock.image").is_none());
        let p = tmp();
        run_s(&p, &["set", "bar", "on"]).unwrap();
        let shape: std::collections::BTreeMap<String, String> = [
            ("bar.thickness", "42"),
            ("dock.position", "left"),
            ("workspaces.selector", "relief"),
            ("bar.style", "tordu"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let refused = crate::theme::apply_pack_shape(&p, &shape).unwrap();
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert!(refused[0].starts_with("bar.style"));
        assert_eq!(run_s(&p, &["get", "bar.height"]).unwrap(), "42");
        assert_eq!(run_s(&p, &["get", "launcher_position"]).unwrap(), "left");
        assert_eq!(run_s(&p, &["get", "workspace_selector"]).unwrap(), "relief");
    }

    /// Transparence adaptative d'une surface (MLG-261).
    #[test]
    fn transparence_adaptative() {
        let p = tmp();
        // La barre, posée sur le fond d'écran, se densifie par défaut
        // (HIG FON-01) ; un menu, qui passe devant les fenêtres, non.
        assert_eq!(
            run_s(&p, &["get", "theme.effects.bar.adaptive"]).unwrap(),
            "0.95"
        );
        assert_eq!(
            run_s(&p, &["get", "theme.effects.menus.adaptive"]).unwrap(),
            "off"
        );
        run_s(&p, &["set", "theme.effects.bar.adaptive", "0.9"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "theme.effects.bar.adaptive"]).unwrap(),
            "0.9"
        );
        run_s(&p, &["set", "theme.effects.widgets.adaptive", "on"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "theme.effects.widgets.adaptive"]).unwrap(),
            "1"
        );
        assert!(run_s(&p, &["set", "theme.effects.bar.adaptive", "2"]).is_err());
        run_s(&p, &["set", "theme.effects.bar.adaptive", "off"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "theme.effects.bar.adaptive"]).unwrap(),
            "off"
        );
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    /// Couleurs et transparence de la barre en ligne de commande.
    #[test]
    fn couleurs_de_la_barre() {
        let p = tmp();
        assert_eq!(run_s(&p, &["get", "bar.opacity"]).unwrap(), "theme");
        assert_eq!(run_s(&p, &["get", "bar.color.text"]).unwrap(), "theme");
        // Couleurs d'une barre affichée (une barre coupée s'écrit `hd.bar(false)`).
        run_s(&p, &["set", "bar", "on"]).unwrap();
        run_s(&p, &["set", "bar.opacity", "0.4"]).unwrap();
        run_s(&p, &["set", "bar.color.accent", "#ff80bf"]).unwrap();
        run_s(&p, &["set", "bar.module_color.clock", "#8aff80"]).unwrap();
        assert_eq!(run_s(&p, &["get", "bar.opacity"]).unwrap(), "0.4");
        assert_eq!(run_s(&p, &["get", "bar.color.accent"]).unwrap(), "#ff80bf");
        assert_eq!(
            run_s(&p, &["get", "bar.module_color.clock"]).unwrap(),
            "#8aff80"
        );
        assert!(
            run_s(&p, &["list", "bar"])
                .unwrap()
                .contains("bar.module_color.clock")
        );
        // Refus : opacité hors bornes, couleur illisible, emplacement inconnu.
        assert!(run_s(&p, &["set", "bar.opacity", "2"]).is_err());
        assert!(run_s(&p, &["set", "bar.color.text", "rouge"]).is_err());
        assert!(run_s(&p, &["set", "bar.color.texte", "#ffffff"]).is_err());
        // Couleurs qui suivent le mode (MLG-263) : paire sombre/clair, slot.
        run_s(&p, &["set", "bar.color.accent", "#2dd4bf/#006B5F"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "bar.color.accent"]).unwrap(),
            "#2dd4bf/#006b5f"
        );
        run_s(&p, &["set", "bar.module_color.clock", "success"]).unwrap();
        assert_eq!(
            run_s(&p, &["get", "bar.module_color.clock"]).unwrap(),
            "success"
        );
        assert!(run_s(&p, &["set", "bar.color.accent", "#2dd4bf/vert"]).is_err());
        // Retour au thème.
        run_s(&p, &["set", "bar.color.accent", "theme"]).unwrap();
        run_s(&p, &["unset", "bar.module_color.clock"]).unwrap();
        run_s(&p, &["unset", "bar.opacity"]).unwrap();
        assert_eq!(run_s(&p, &["get", "bar.color.accent"]).unwrap(), "theme");
        assert_eq!(
            run_s(&p, &["get", "bar.module_color.clock"]).unwrap(),
            "theme"
        );
        assert_eq!(run_s(&p, &["get", "bar.opacity"]).unwrap(), "theme");
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
