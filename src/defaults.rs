//! Configuration par défaut de chaque composant (MLG-202).
//!
//! Chaque composant (un paquet de Réglages : `ui::plugins::BUILTIN`) peut
//! revenir à sa configuration par défaut, sans toucher aux autres. Cette
//! configuration réunit trois choses :
//!
//! - **ses champs de `settings.lua`**, rendus à ceux du *bureau de départ*
//!   (`config::starter_settings`). Une option absente prend la valeur par
//!   défaut de hyprdesk, et le bureau de départ ne fixe que les icônes, les
//!   widgets et la barre d'une première installation ;
//! - **les fichiers de Hyprland qu'il tient**, réécrits à leurs valeurs par
//!   défaut : `binds.lua` (Raccourcis : plus aucun raccourci coupé),
//!   `inputs.lua` (Clavier et souris : valeurs de Hyprland, en gardant la
//!   disposition du clavier), `monitors.lua` (Écrans : mode préféré, placement
//!   automatique), `autostart.lua` (Démarrage automatique : vide). Les
//!   fichiers générés depuis les réglages (`appearance.lua`, `layouts.lua`…)
//!   suivent les réglages ;
//! - **son état mémorisé** : les positions d'icônes posées à la souris
//!   (Icônes), les modes de bureau par workspace (Bureau), les places des
//!   fenêtres (Fenêtres) et l'historique des lancements (Lanceur).
//!
//! `hyprdesk.lua`, écrit à la main, n'est jamais touché : une clé qu'il fixe
//! garde sa valeur. La remise est signalée dans le plan.
//!
//! La remise globale (`all`) fait la même chose pour chaque composant. Elle
//! ajoute les notifications gardées et met de côté les thèmes et les widgets
//! perso (`hyprdesk/themes`, `hyprdesk/widgets`). Les réglages propres à
//! l'interface (configuration avancée, visite guidée vue) restent.
//!
//! Avant toute écriture, les fichiers touchés sont copiés dans
//! `hyprdesk/sauvegardes/<date>/`, et l'historique prend un instantané
//! (`hyprdesk config rollback`).
//!
//! Le son, l'énergie, le réseau, le Bluetooth, la date et l'heure, la
//! connexion, les mots de passe, le partage et l'historique ne gardent rien
//! dans hyprdesk : ils règlent le système. Ils n'ont pas de configuration par
//! défaut propre.

use std::path::{Path, PathBuf};

use crate::config::{Config, Origin};
use crate::settings::Settings;

/// Ce que tient un composant hors de `settings.lua`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    /// `binds.lua` : raccourcis coupés depuis les Réglages.
    Binds,
    /// `inputs.lua` : clavier, souris, pavé tactile.
    Inputs,
    /// `monitors.lua` : disposition des écrans.
    Monitors,
    /// `autostart.lua` : commandes lancées à l'ouverture de session.
    Autostart,
    /// Positions des icônes posées à la souris (`layout.toml`).
    IconPositions,
    /// Mode du bureau de chaque workspace (`layout.toml`).
    DesktopModes,
    /// Places des fenêtres en plein bureau (`window-places.json`).
    WindowPlaces,
    /// Historique des lancements (suggestions du lanceur).
    Launches,
    /// Notifications gardées, « Ne pas déranger », passage du Silence et
    /// autorisations des applications (remise globale).
    Notifications,
    /// Thèmes perso (`hyprdesk/themes`, remise globale).
    Themes,
    /// Widgets perso (`hyprdesk/widgets`, remise globale).
    UserWidgets,
}

impl Held {
    /// Ce que la remise en fait, en une ligne.
    pub fn label(self) -> &'static str {
        match self {
            Held::Binds => "binds.lua : tous les raccourcis réactivés",
            Held::Inputs => "inputs.lua : valeurs de Hyprland (disposition du clavier gardée)",
            Held::Monitors => "monitors.lua : mode préféré, placement automatique",
            Held::Autostart => "autostart.lua : aucune commande",
            Held::IconPositions => "positions des icônes oubliées",
            Held::DesktopModes => "mode du bureau de chaque workspace oublié",
            Held::WindowPlaces => "places des fenêtres oubliées",
            Held::Launches => "historique des lancements effacé",
            Held::Notifications => {
                "notifications gardées effacées, « Ne pas déranger » coupé, passage du Silence et autorisations des applications oubliés"
            }
            Held::Themes => "thèmes perso mis de côté",
            Held::UserWidgets => "widgets perso mis de côté",
        }
    }
}

/// Déclare, pour chaque composant, ses champs de `settings.lua` (copiés
/// depuis le bureau de départ) et ce qu'il tient ailleurs.
macro_rules! components {
    ($( $id:literal => [$($f:ident),* $(,)?] $(+ [$($h:ident),* $(,)?])? ;)*) => {
        /// Champs de `settings.lua` de chaque composant (hors options de
        /// Hyprland et options des plugins, partagées : voir [`split_fields`]).
        pub const FIELDS: &[(&str, &[&str])] = &[$(($id, &[$(stringify!($f)),*])),*];

        /// Ce que chaque composant tient hors de `settings.lua`.
        pub const HELD: &[(&str, &[Held])] = &[$(($id, &[$($(Held::$h),*)?])),*];

        fn copy_fields(id: &str, s: &mut Settings, d: &Settings, changed: &mut Vec<String>) {
            match id {
                $($id => {
                    $(
                        if s.$f != d.$f {
                            changed.push(stringify!($f).to_string());
                            s.$f = d.$f.clone();
                        }
                    )*
                })*
                _ => {}
            }
        }
    };
}

components! {
    "desktop" => [
        clipboard_history, autostart, desktop_files, float_launched, hyprbars,
        hyprbars_path, show_on, mode, glass_opacity, close_on_launch,
        forced_modes, desktops, font, font_size, language,
    ] + [DesktopModes];
    "keys" => [
        key_toggle, key_launcher, key_hide, key_settings, key_lock, key_switch,
        key_search, key_search_desktop, key_overview, key_assistant, key_take, overview_command,
        key_leave, key_media, key_essentials, key_essentials_off,
        key_essential_keys, suspend_binds,
    ] + [Binds];
    "icons" => [icons, icon_scope, icon_double_click, icon_theme, icon_size, cell]
        + [IconPositions];
    "dock" => [
        running_apps, launcher_group, launcher_position, launcher_size,
        launcher_pinned, launcher_show_settings, launcher_show_assistant,
    ];
    "launcher" => [
        apps_style, apps_suggestions, apps_categories, apps_unified, launcher_shape,
    ] + [Launches];
    "index" => [
        index_paths, index_exclude, index_exclude_regex, index_exclude_ext,
        index_exclude_cat, index_max_size, index_interval, index_autostart,
    ];
    "apps" => [apps];
    "autostart" => [] + [Autostart];
    "assistant" => [
        assistant_mode, ai_services, ai_use, launcher_smartui, smartui_style,
        smartui_auto,
    ];
    "theme" => [
        accent, theme_name, fg, bg, background, effects, reduced_transparency, reduced_motion,
        increase_contrast, text_scale,
    ];
    "apptheme" => [app_themes];
    "wallpaper" => [wallpapers];
    "widgets" => [widgets, placements, disabled];
    "modules" => [];
    "bar" => [bar];
    "panorama" => [panorama];
    "screens" => [primary, screen_zones] + [Monitors];
    "colors" => [screen_colors];
    "workspaces" => [
        workspaces, workspace_monitors, workspace_dedicated, workspace_selector,
        assigns,
    ];
    "windows" => [] + [WindowPlaces];
    "layouts" => [tiling];
    "animations" => [fx, fade_ms, transition, crossfade_ms];
    "performance" => [renderer, gpu, hyprland_gpu, worker_threads];
    "input" => [] + [Inputs];
    "lock" => [
        lock_blur, lock_dim, lock_image, lock_show_user, lock_show_battery,
        lock_show_media, lock_fingerprint, idle_manage, idle_dim, idle_lock,
        idle_dpms, idle_suspend, idle_wake_volume, idle_lid_battery, idle_lid_ac,
        idle_lid_docked,
    ];
    "plugins" => [disabled_plugins, update_commands];
    "hyprconfig" => [];
}

/// Champs partagés entre composants : réglés morceau par morceau.
#[allow(dead_code)] // relu par le test de couverture
pub const SPLIT_FIELDS: &[&str] = &["hypr", "plugin_options"];

/// Champs de l'interface elle-même : aucune remise n'y touche.
#[allow(dead_code)] // relu par le test de couverture
pub const KEPT_FIELDS: &[&str] = &["advanced_settings", "tour_seen"];

/// Options de Hyprland (`hypr`) de chaque composant, par groupe des Réglages
/// (`hyprset::GROUPS`). Une option sans groupe va à Configuration.
const HYPR_GROUPS: &[(&str, &[&str])] = &[
    (
        "windows",
        &[
            "fenetres",
            "fenetres-details",
            "groupes",
            "comportement-fenetres",
        ],
    ),
    ("input", &["souris", "pave", "curseur", "gestes"]),
    ("workspaces", &["workspaces"]),
    ("keys", &["raccourcis"]),
    ("lock", &["veille"]),
    ("wallpaper", &["fond"]),
    ("animations", &["flou"]),
    ("hyprconfig", &["avance", "permissions"]),
];

/// Composant d'une option de Hyprland (`general.gaps_in` → `windows`).
pub fn hypr_owner(path: &str) -> &'static str {
    for (group, _, paths) in crate::hyprset::GROUPS {
        if paths.contains(&path)
            && let Some((id, _)) = HYPR_GROUPS.iter().find(|(_, g)| g.contains(group))
        {
            return id;
        }
    }
    "hyprconfig"
}

/// Composant d'un jeu d'options de plugin (`plugin_options.<plugin>`).
pub fn plugin_options_owner(plugin: &str) -> &'static str {
    if plugin == crate::apptheme::PLUGIN {
        "apptheme"
    } else if plugin == crate::panorama::modules::PLUGIN
        || crate::panorama::modules::CATALOGUE
            .iter()
            .any(|(m, _, _)| *m == plugin)
    {
        "modules"
    } else {
        "plugins"
    }
}

/// Remet par défaut la part de `hypr` et de `plugin_options` que tient `id`.
fn split_fields(id: &str, s: &mut Settings, changed: &mut Vec<String>) {
    if let Some(h) = s.hypr.as_mut() {
        let before = h.clone();
        h.options.retain(|k, _| hypr_owner(k) != id);
        if id == "windows" {
            h.rules.clear();
            h.theme_borders = false;
        }
        if id == "input" {
            h.gestures.clear();
            h.devices.clear();
        }
        if id == "hyprconfig" {
            h.env.clear();
            h.permissions.clear();
        }
        if *h != before {
            changed.push("hypr".into());
        }
        if *h == crate::hyprset::Hypr::default() {
            s.hypr = None;
        }
    }
    if let Some(o) = s.plugin_options.as_mut() {
        let n = o.len();
        o.retain(|p, _| plugin_options_owner(p) != id);
        if o.len() != n {
            changed.push("plugin_options".into());
        }
        if o.is_empty() {
            s.plugin_options = None;
        }
    }
}

/// Composants qui ont une configuration par défaut (ceux qui gardent
/// quelque chose dans hyprdesk), dans l'ordre des Réglages.
pub fn components() -> impl Iterator<Item = &'static crate::ui::plugins::PluginInfo> {
    crate::ui::plugins::BUILTIN
        .iter()
        .filter(|p| holds_something(p.id))
}

/// `id` garde-t-il quelque chose dans hyprdesk ?
pub fn holds_something(id: &str) -> bool {
    FIELDS.iter().any(|(c, _)| *c == id)
}

/// Ce que tient `id` hors de `settings.lua`.
pub fn held(id: &str) -> &'static [Held] {
    HELD.iter()
        .find(|(c, _)| *c == id)
        .map(|(_, h)| *h)
        .unwrap_or(&[])
}

/// Identifiant d'un composant : identifiant, alias ou libellé des Réglages
/// (`raccourcis`, `Barre`…) ; `all`, `tout` : remise globale (`None`).
pub fn resolve(name: &str) -> Result<Option<&'static str>, String> {
    let n = name.trim();
    if matches!(n, "all" | "tout" | "--all") {
        return Ok(None);
    }
    let found = crate::ui::plugins::find(n).or_else(|| {
        crate::ui::plugins::BUILTIN.iter().find(|p| {
            p.label.eq_ignore_ascii_case(n) || crate::i18n::tr(p.label).eq_ignore_ascii_case(n)
        })
    });
    match found {
        Some(p) if holds_something(p.id) => Ok(Some(p.id)),
        Some(p) => Err(crate::tr!(
            "« {l} » ne garde rien dans hyprdesk : ses réglages sont ceux du système",
            l = crate::i18n::tr(p.label)
        )),
        None => Err(crate::tr!(
            "composant « {n} » inconnu (hyprdesk config reset : liste des composants)",
            n = n
        )),
    }
}

/// Réglages de `current` dont la part de `target` (`None` : tout) est
/// remise à celle de `starter`. Rend les champs changés.
pub fn reset_settings(
    target: Option<&str>,
    current: &Settings,
    starter: &Settings,
) -> (Settings, Vec<String>) {
    let mut s = current.clone();
    let mut changed = Vec::new();
    let ids: Vec<&str> = match target {
        Some(id) => vec![id],
        None => FIELDS.iter().map(|(c, _)| *c).collect(),
    };
    for id in ids {
        copy_fields(id, &mut s, starter, &mut changed);
        split_fields(id, &mut s, &mut changed);
    }
    let mut seen = std::collections::BTreeSet::new();
    changed.retain(|f| seen.insert(f.clone()));
    (s, changed)
}

/// Ce qu'une remise fera, avant de la faire.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Composant visé ; `None` : tout.
    pub target: Option<&'static str>,
    /// Réglages remis.
    pub settings: Settings,
    /// Champs de `settings.lua` qui changent.
    pub changed: Vec<String>,
    /// Configuration après la remise (hyprdesk.lua compris).
    pub after: Config,
    /// Ce que tient le composant ailleurs (fichiers, état).
    pub held: Vec<Held>,
    /// Clés fixées dans hyprdesk.lua : elles gardent leur valeur.
    pub overridden: Vec<String>,
}

/// Configuration chargée avec `s` à la place de `settings.lua`.
pub fn config_with(config_path: &Path, s: &Settings) -> Result<Config, String> {
    let base = config_path.parent().unwrap_or(Path::new("."));
    let sp = crate::config::lua::settings_path(config_path);
    let mut chunks = vec![(s.to_lua(), sp.display().to_string(), Origin::Settings)];
    if let Ok(src) = std::fs::read_to_string(config_path) {
        chunks.push((src, config_path.display().to_string(), Origin::User));
    }
    crate::config::lua::load_chunks(&chunks, base).map(|l| l.config)
}

/// Prépare la remise de `target` (`None` : tout), sans rien écrire.
pub fn plan(config_path: &Path, target: Option<&'static str>) -> Result<Plan, String> {
    let current = Settings::from_config(&crate::config::lua::load_settings_only(config_path)?);
    let starter = crate::config::starter_settings(config_path).map_err(|e| e.to_string())?;
    let (settings, changed) = reset_settings(target, &current, &starter);
    let after = config_with(config_path, &settings)?;
    let held: Vec<Held> = match target {
        Some(id) => held(id).to_vec(),
        None => HELD
            .iter()
            .flat_map(|(_, h)| h.iter().copied())
            .chain([Held::Notifications, Held::Themes, Held::UserWidgets])
            .collect(),
    };
    let overridden = crate::cli_config::KEYS
        .iter()
        .filter(|k| target.is_none_or(|id| k.plugin == id))
        .filter(|k| after.keys_user.contains(k.name))
        .map(|k| k.name.to_string())
        .collect();
    Ok(Plan {
        target,
        settings,
        changed,
        after,
        held,
        overridden,
    })
}

/// Options d'application d'une remise.
#[derive(Debug, Clone, Copy)]
pub struct Apply {
    /// Session Hyprland en cours : fichiers vérifiés et appliqués aussitôt
    /// (rechargement, entrées posées). Sans session (essais), seulement écrits.
    pub live: bool,
}

/// Fait la remise prévue par `plan`. Rend le dossier de la copie.
pub fn apply(config_path: &Path, plan: &Plan, opts: Apply) -> Result<PathBuf, String> {
    let reason = match plan.target {
        Some(_) => "par-defaut",
        None => "reinitialisation",
    };
    crate::history::before(config_path, reason);
    let dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    // Un dossier par remise, même deux dans la même seconde.
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let root = dir.join("hyprdesk").join("sauvegardes");
    let mut backup = root.join(&stamp);
    let mut n = 2;
    while backup.exists() {
        backup = root.join(format!("{stamp}-{n}"));
        n += 1;
    }
    std::fs::create_dir_all(&backup).map_err(|e| e.to_string())?;
    let keep = |src: &Path, name: &str| -> Result<(), String> {
        if src.is_file() {
            let dst = backup.join(name);
            if let Some(d) = dst.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            std::fs::copy(src, &dst).map_err(|e| e.to_string())?;
        }
        Ok(())
    };
    let settings_file = crate::config::lua::settings_path(config_path);
    keep(&settings_file, "settings.lua")?;
    for h in &plan.held {
        match h {
            Held::Binds => keep(&crate::binds_file::path(config_path), "binds.lua")?,
            Held::Inputs => keep(&crate::inputs::inputs_path(config_path), "inputs.lua")?,
            Held::Monitors => keep(&crate::screens::monitors_path(config_path), "monitors.lua")?,
            Held::Autostart => keep(
                &crate::session_start::lua_path(config_path),
                "autostart.lua",
            )?,
            Held::IconPositions | Held::DesktopModes => {
                keep(&crate::layout::path(), "etat/layout.toml")?
            }
            Held::WindowPlaces => keep(&crate::windows::places_path(), "etat/window-places.json")?,
            Held::Launches => keep(&crate::search::usage_path(), "etat/launches")?,
            Held::Notifications | Held::Themes | Held::UserWidgets => {}
        }
    }

    // Réglages d'abord : les fichiers générés en découlent.
    plan.settings
        .save_versioned(config_path, crate::storage::revision())
        .map_err(|e| crate::tr!("settings.lua : {e}", e = e))?;
    crate::history::note(&settings_file);

    let mut errors = Vec::new();
    for h in &plan.held {
        if let Err(e) = reset_held(config_path, *h, &dir, &backup, &plan.after, opts) {
            errors.push(e);
        }
    }
    // Fichiers de Hyprland générés depuis les réglages (apparence, layouts,
    // règles, bloc avancé d'inputs.lua…).
    if opts.live
        && let Err(e) = crate::genfiles::sync(config_path, &plan.after)
    {
        errors.push(crate::tr!("fichiers de Hyprland : {e}", e = e));
    }
    if errors.is_empty() {
        Ok(backup)
    } else {
        Err(crate::tr!(
            "remise faite en partie (copie : {b}) : {e}",
            b = backup.display(),
            e = errors.join(" ; ")
        ))
    }
}

fn remove(p: &Path) -> Result<(), String> {
    match std::fs::remove_file(p) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(crate::tr!("{p} : {e}", p = p.display(), e = e))
        }
        _ => Ok(()),
    }
}

fn reset_held(
    config_path: &Path,
    h: Held,
    dir: &Path,
    backup: &Path,
    after: &Config,
    opts: Apply,
) -> Result<(), String> {
    match h {
        Held::Binds => crate::binds_file::enable_all(config_path, opts.live).and_then(|_| {
            // Les conflits déjà répondus se reposent (MLG-209).
            crate::config::forget_key_answers(config_path);
            // Les touches que hyprdesk tient par défaut redeviennent les siennes.
            crate::binds_file::sync_hyprdesk(config_path, after, !opts.live).map(|_| ())
        }),
        Held::Inputs => {
            // La disposition du clavier reste : sans elle, plus moyen de taper.
            // Celle qu'écrit inputs.lua d'abord, sinon celle de la session.
            let file = crate::inputs::inputs_path(config_path);
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            let live = if opts.live {
                crate::inputs::read()
            } else {
                crate::inputs::Inputs::default()
            };
            let pick = |key: &str, now: String| match lua_field(&text, key) {
                Some(v) => v,
                None if opts.live => now,
                None => String::new(),
            };
            let mut i = crate::inputs::Inputs {
                kb_layout: pick("kb_layout", live.kb_layout),
                kb_variant: pick("kb_variant", live.kb_variant),
                kb_options: pick("kb_options", live.kb_options),
                // Écran des périphériques tactiles : tenu par le Panorama.
                touch_output: pick("output", live.touch_output),
                ..Default::default()
            };
            if i.kb_layout.is_empty() {
                i.kb_layout = crate::inputs::Inputs::default().kb_layout;
            }
            crate::inputs::save(&file, &i).map_err(|e| crate::tr!("inputs.lua : {e}", e = e))?;
            if opts.live {
                crate::inputs::apply(&i)?;
            }
            Ok(())
        }
        Held::Monitors => {
            let p = crate::screens::monitors_path(config_path);
            crate::storage::write(
                &p,
                crate::screens::render(&[], "").as_bytes(),
                crate::storage::revision(),
            )
            .map_err(|e| crate::tr!("monitors.lua : {e}", e = e))?;
            if opts.live {
                let _ = std::process::Command::new("hyprctl").arg("reload").output();
            }
            Ok(())
        }
        // autostart.lua vidé s'il existe ; hyprland.lua n'est pas touché.
        Held::Autostart => {
            let p = crate::session_start::lua_path(config_path);
            if p.is_file() {
                crate::storage::write(
                    &p,
                    crate::session_start::render(&[]).as_bytes(),
                    crate::storage::revision(),
                )
                .map_err(|e| crate::tr!("autostart.lua : {e}", e = e))?;
            }
            Ok(())
        }
        Held::IconPositions => {
            let mut l = crate::layout::Layout::load();
            l.clear_positions();
            l.save();
            Ok(())
        }
        Held::DesktopModes => {
            let mut l = crate::layout::Layout::load();
            l.set_modes(Default::default());
            l.save();
            Ok(())
        }
        Held::WindowPlaces => remove(&crate::windows::places_path()),
        Held::Launches => remove(&crate::search::usage_path()),
        Held::Notifications => {
            remove(&crate::notifications::dnd_path())?;
            remove(&crate::notifications::passage_path())
                .and(remove(&crate::notifications::authorizations_path()))
        }
        Held::Themes | Held::UserWidgets => {
            let name = if h == Held::Themes {
                "themes"
            } else {
                "widgets"
            };
            let src = dir.join("hyprdesk").join(name);
            let empty = std::fs::read_dir(&src)
                .map(|mut d| d.next().is_none())
                .unwrap_or(true);
            if !empty {
                std::fs::rename(&src, backup.join(name))
                    .map_err(|e| crate::tr!("{p} : {e}", p = src.display(), e = e))?;
                std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
    }
}

/// Valeur texte de `key = "…"` dans une source Lua générée (inputs.lua).
fn lua_field(src: &str, key: &str) -> Option<String> {
    let mut rest = src;
    while let Some(i) = rest.find(key) {
        let before = rest[..i].chars().next_back();
        let after = rest[i + key.len()..].trim_start();
        rest = &rest[i + key.len()..];
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        if let Some(v) = after.strip_prefix('=').map(str::trim_start)
            && let Some(v) = v.strip_prefix('"')
            && let Some(end) = v.find('"')
        {
            return Some(v[..end].to_string());
        }
    }
    None
}

/// Fait savoir au démon (s'il tourne) qu'une remise a eu lieu : il relit son
/// état mémorisé et sa configuration.
pub fn notify_daemon(target: Option<&str>) {
    let _ = crate::control::send(crate::control::Command::Defaults(
        target.unwrap_or("all").to_string(),
    ));
}

/// Rapport lisible d'un plan (avant) ou d'une remise faite (après).
pub fn describe(plan: &Plan, before: &Config) -> String {
    let mut out = String::new();
    let keys: Vec<&crate::cli_config::Key> = crate::cli_config::KEYS
        .iter()
        .filter(|k| plan.target.is_none_or(|id| k.plugin == id))
        .collect();
    let mut shown = 0;
    for k in &keys {
        let (a, b) = (k.value(before), k.value(&plan.after));
        if a != b {
            // Valeur courte : les listes (sources du Panorama…) tiennent sur la ligne.
            let dash = |v: &str| {
                if v.is_empty() {
                    "—".to_string()
                } else if v.chars().count() > 60 {
                    format!("{}…", v.chars().take(59).collect::<String>())
                } else {
                    v.to_string()
                }
            };
            out += &format!("\n  {} : {} → {}", k.name, dash(&a), dash(&b));
            shown += 1;
        }
    }
    // Champs sans clé de `hyprdesk config` (listes : icônes, fonds…).
    let keyed: Vec<&str> = keys.iter().map(|k| k.name).collect();
    let others: Vec<&str> = plan
        .changed
        .iter()
        .map(String::as_str)
        .filter(|f| {
            !keyed
                .iter()
                .any(|k| k == f || k.replace('.', "_") == *f || k.rsplit('.').next() == Some(*f))
        })
        .collect();
    if !others.is_empty() {
        out += "\n  ";
        out += &crate::tr!("settings.lua : {f}", f = others.join(", "));
        shown += 1;
    }
    if shown == 0 {
        out += "\n  ";
        out += crate::i18n::tr("réglages déjà par défaut");
    }
    for h in &plan.held {
        out += "\n  ";
        out += crate::i18n::tr(h.label());
    }
    if !plan.overridden.is_empty() {
        out += "\n  ";
        out += &crate::tr!(
            "fixé dans hyprdesk.lua, non touché : {k}",
            k = plan.overridden.join(", ")
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque champ de `Settings` appartient à un seul composant, ou à la
    /// liste des champs partagés ou gardés : un champ ajouté sans composant
    /// ne serait jamais remis par défaut.
    #[test]
    fn chaque_reglage_a_son_composant() {
        let src = include_str!("settings.rs");
        let body = src
            .split("pub struct Settings {")
            .nth(1)
            .and_then(|b| b.split("\n}").next())
            .unwrap();
        let fields: Vec<&str> = body
            .lines()
            .filter_map(|l| l.trim().strip_prefix("pub "))
            .filter_map(|l| l.split(':').next())
            .collect();
        assert!(fields.len() > 100, "{}", fields.len());
        for f in &fields {
            let owners = FIELDS.iter().filter(|(_, fs)| fs.contains(f)).count()
                + usize::from(SPLIT_FIELDS.contains(f))
                + usize::from(KEPT_FIELDS.contains(f));
            assert_eq!(owners, 1, "champ « {f} » : {owners} composant(s)");
        }
        for (id, fs) in FIELDS {
            assert!(
                crate::ui::plugins::BUILTIN.iter().any(|p| p.id == *id),
                "composant « {id} » inconnu"
            );
            for f in *fs {
                assert!(fields.contains(f), "{id} : champ « {f} » inconnu");
            }
        }
        // Chaque clé de `hyprdesk config` relève d'un composant qui a sa
        // configuration par défaut (ou de À propos, qui n'en a pas).
        for k in crate::cli_config::KEYS {
            assert!(
                holds_something(k.plugin) || k.plugin == "about",
                "{} : paquet « {} » sans configuration par défaut",
                k.name,
                k.plugin
            );
        }
        // Chaque groupe d'options de Hyprland a son composant.
        for (group, _, _) in crate::hyprset::GROUPS {
            assert!(
                HYPR_GROUPS.iter().any(|(_, g)| g.contains(group)),
                "groupe « {group} » sans composant"
            );
        }
    }

    #[test]
    fn remise_d_un_composant_sans_toucher_aux_autres() {
        let mut s = Settings {
            show_on: Some(crate::config::ShowOn::Focused),
            key_launcher: Some("SUPER + L".into()),
            launcher_size: Some(80),
            ..Default::default()
        };
        let mut h = crate::hyprset::Hypr::default();
        h.options
            .insert("general.gaps_in".into(), crate::tiling::OptValue::Num(9.0));
        h.options.insert(
            "misc.disable_hyprland_logo".into(),
            crate::tiling::OptValue::Bool(true),
        );
        s.hypr = Some(h);
        let mut po = std::collections::BTreeMap::new();
        po.insert("panorama".to_string(), Default::default());
        po.insert("apptheme".to_string(), Default::default());
        po.insert("net".to_string(), Default::default());
        s.plugin_options = Some(po);
        let starter = Settings::default();

        let (k, changed) = reset_settings(Some("keys"), &s, &starter);
        assert_eq!(changed, vec!["key_launcher".to_string()]);
        assert_eq!(k.key_launcher, None);
        assert_eq!(k.show_on, s.show_on);
        assert_eq!(k.launcher_size, Some(80));

        let (w, _) = reset_settings(Some("windows"), &s, &starter);
        let opts = &w.hypr.as_ref().unwrap().options;
        assert!(!opts.contains_key("general.gaps_in"));
        assert!(opts.contains_key("misc.disable_hyprland_logo"));

        let (m, _) = reset_settings(Some("modules"), &s, &starter);
        let po = m.plugin_options.unwrap();
        assert!(!po.contains_key("panorama"));
        assert!(po.contains_key("apptheme") && po.contains_key("net"));

        let (all, _) = reset_settings(None, &s, &starter);
        assert_eq!(all, Settings::default());
    }

    #[test]
    fn disposition_du_clavier_lue_dans_inputs_lua() {
        let src = r#"hl.config({ input = { kb_layout = "fr", kb_variant = "azerty", touchdevice = { output = "HDMI-A-1" } } })"#;
        assert_eq!(lua_field(src, "kb_layout").as_deref(), Some("fr"));
        assert_eq!(lua_field(src, "kb_variant").as_deref(), Some("azerty"));
        assert_eq!(lua_field(src, "kb_options"), None);
        assert_eq!(lua_field(src, "output").as_deref(), Some("HDMI-A-1"));
    }

    #[test]
    fn noms_des_composants() {
        assert_eq!(resolve("keys").unwrap(), Some("keys"));
        assert_eq!(resolve("bar").unwrap(), Some("bar"));
        assert_eq!(resolve("all").unwrap(), None);
        assert!(resolve("sound").is_err(), "le son règle le système");
        assert!(resolve("nope").is_err());
        assert_eq!(held("screens"), &[Held::Monitors]);
    }

    #[test]
    fn remise_ecrite_avec_copie() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-defaults-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("hyprdesk/themes")).unwrap();
        // État : dossier temporaire des tests (`layout::state_dir`).
        let cfg = dir.join("hyprdesk.lua");
        let user = "-- à moi\nhd.config({ launcher_size = 70 })\n";
        std::fs::write(&cfg, user).unwrap();
        std::fs::write(dir.join("hyprdesk/themes/a.toml"), "x").unwrap();
        std::fs::write(
            dir.join("monitors.lua"),
            "-- Généré par hyprdesk\nhl.monitor({ output = \"DP-1\", mode = \"1920x1080\", position = \"0x0\", scale = 1 })\n",
        )
        .unwrap();
        Settings {
            show_on: Some(crate::config::ShowOn::Focused),
            primary: Some("DP-1".into()),
            launcher_size: Some(90),
            ..Default::default()
        }
        .save(&cfg)
        .unwrap();
        let before = crate::config::lua::load_file(&cfg).unwrap().config;

        // Un composant : Écrans. Le reste ne bouge pas.
        let p = plan(&cfg, Some("screens")).unwrap();
        assert_eq!(p.changed, vec!["primary".to_string()]);
        let text = describe(&p, &before);
        assert!(text.contains("monitors.lua"), "{text}");
        let b = apply(&cfg, &p, Apply { live: false }).unwrap();
        assert!(b.join("settings.lua").is_file() && b.join("monitors.lua").is_file());
        let mon = std::fs::read_to_string(dir.join("monitors.lua")).unwrap();
        assert!(!mon.contains("DP-1") && mon.contains("preferred"), "{mon}");
        let c = crate::config::lua::load_file(&cfg).unwrap().config;
        assert_eq!(c.general.show_on, crate::config::ShowOn::Focused);

        // Tout : hyprdesk.lua reste, sa clé garde sa valeur et est signalée.
        let p = plan(&cfg, None).unwrap();
        assert!(
            p.overridden.iter().any(|k| k.contains("launcher")),
            "{:?}",
            p.overridden
        );
        let b = apply(&cfg, &p, Apply { live: false }).unwrap();
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), user);
        assert!(b.join("themes/a.toml").is_file(), "thèmes mis de côté");
        assert!(!dir.join("hyprdesk/themes/a.toml").exists());
        let c = crate::config::lua::load_file(&cfg).unwrap().config;
        assert_eq!(c.general.show_on, crate::config::ShowOn::default());
        assert_eq!(c.icons.len(), 3, "bureau de départ");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
