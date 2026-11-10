//! Paquets de réglages intégrés (built-in packages).
//!
//! Chaque entrée de l'écran Réglages est un paquet autonome : un manifeste
//! ([`PluginInfo`]), ses messages, son état, sa vue et sa mise à jour. La
//! coquille du tiroir (`panel.rs`) ne connaît que ce registre : la navigation
//! se construit depuis [`BUILTIN`], chaque message est routé vers son paquet.
//! Ajouter un paquet : un module ici, une ligne dans `BUILTIN`, une variante
//! dans [`PluginMsg`] et un bras dans `update` / `view`. Conception complète :
//! `docs/PLUGINS.md`.

use iced::{Element, Task};

use super::panel::Panel;
use super::{App, Msg};

pub mod apps;
pub mod assistant;
pub mod bar;
pub mod desktop;
pub mod effects;
pub mod icons;
pub mod launcher;
pub mod lock;
#[allow(clippy::module_inception)]
pub mod plugins;
pub mod screens;
pub mod share;
pub mod wallpaper;
pub mod widgets;
pub mod workspaces;

/// Manifeste d'un paquet de réglages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginInfo {
    /// Identifiant stable (onglet, `hyprdesk settings <id>`).
    pub id: &'static str,
    /// Libellé de la navigation.
    pub label: &'static str,
    /// Icône (nom freedesktop ; glyphe intégré dans `ui/icons.rs`).
    pub icon: &'static str,
    /// Une phrase : ce que le paquet règle.
    pub summary: &'static str,
    /// Livré avec hyprdesk (statut « build-in package »).
    pub builtin: bool,
}

/// Paquets intégrés, dans l'ordre de la navigation.
pub const BUILTIN: &[PluginInfo] = &[
    desktop::INFO,
    icons::INFO,
    launcher::INFO,
    apps::INFO,
    assistant::INFO,
    wallpaper::INFO,
    widgets::INFO,
    bar::INFO,
    screens::INFO,
    workspaces::INFO,
    effects::INFO,
    lock::INFO,
    share::INFO,
    plugins::INFO,
];

/// Paquet d'identifiant donné (ou d'alias de ligne de commande).
pub fn find(id: &str) -> Option<&'static PluginInfo> {
    let id = match id {
        "bureau" => "desktop",
        "icones" | "icônes" => "icons",
        "fond" => "wallpaper",
        "barre" => "bar",
        "effets" => "effects",
        "ecrans" | "écrans" => "screens",
        "lanceur" | "dock" => "launcher",
        "applications" | "defaults" => "apps",
        "extensions" | "paquets" => "plugins",
        "ia" | "ai" | "lyra" => "assistant",
        "partage" | "export" | "import" => "share",
        "verrouillage" | "verrou" => "lock",
        other => other,
    };
    BUILTIN.iter().find(|p| p.id == id)
}

/// Message d'un paquet, enveloppé dans `PanelMsg::Plugin`.
#[derive(Debug, Clone)]
pub enum PluginMsg {
    Desktop(desktop::Msg),
    Icons(icons::Msg),
    Launcher(launcher::Msg),
    Apps(apps::Msg),
    Assistant(assistant::Msg),
    Plugins(plugins::Msg),
    Share(share::Msg),
    Lock(lock::Msg),
    Wallpaper(wallpaper::Msg),
    Widgets(widgets::Msg),
    Bar(bar::Msg),
    Screens(screens::Msg),
    Workspaces(workspaces::Msg),
    Effects(effects::Msg),
}

pub(in crate::ui) fn update(app: &mut App, m: PluginMsg) -> Task<Msg> {
    match m {
        PluginMsg::Desktop(m) => app.desktop_update(m),
        PluginMsg::Icons(m) => app.icons_update(m),
        PluginMsg::Launcher(m) => app.launcher_update(m),
        PluginMsg::Apps(m) => app.apps_update(m),
        PluginMsg::Assistant(m) => app.assistant_update(m),
        PluginMsg::Plugins(m) => app.plugins_update(m),
        PluginMsg::Share(m) => app.share_update(m),
        PluginMsg::Lock(m) => app.lock_update(m),
        PluginMsg::Wallpaper(m) => app.wallpaper_update(m),
        PluginMsg::Widgets(m) => app.widgets_update(m),
        PluginMsg::Bar(m) => app.bar_update(m),
        PluginMsg::Screens(m) => app.screens_update(m),
        PluginMsg::Workspaces(m) => app.workspaces_update(m),
        PluginMsg::Effects(m) => app.effects_update(m),
    }
}

pub(in crate::ui) fn view<'a>(app: &'a App, p: &'a Panel, id: &str) -> Element<'a, Msg> {
    match id {
        "icons" => app.tab_icons(p),
        "launcher" => app.launcher_page(),
        "apps" => app.apps_page(),
        "assistant" => app.assistant_page(),
        "plugins" => app.plugins_page(),
        "share" => app.share_page(),
        "lock" => app.lock_page(),
        "wallpaper" => app.tab_wallpaper(p),
        "widgets" => app.tab_widgets(p),
        "bar" => app.tab_bar(),
        "screens" => app.tab_screens(),
        "workspaces" => app.tab_workspaces(p),
        "effects" => app.tab_effects(),
        _ => {
            let (_, ws) = app.panel_ctx().unwrap_or_default();
            app.tab_desktop(&ws)
        }
    }
}

/// Un paquet devient visible : de quoi se remplir (détection, vignettes…).
pub(in crate::ui) fn on_show(app: &mut App, id: &str) -> Task<Msg> {
    match id {
        "screens" => App::detect_screens(),
        "apps" => apps::load_system(),
        "assistant" => assistant::load_status(),
        "wallpaper" => match &app.panel {
            Some(p) if p.wallpaper.images.is_empty() => {
                wallpaper::load_images(p.wallpaper.folder.clone())
            }
            _ => Task::none(),
        },
        _ => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registre() {
        let ids: Vec<&str> = BUILTIN.iter().map(|p| p.id).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "identifiants uniques");
        assert!(BUILTIN.iter().all(|p| p.builtin && !p.label.is_empty()));
        assert_eq!(find("fond").map(|p| p.id), Some("wallpaper"));
        assert_eq!(find("screens").map(|p| p.id), Some("screens"));
        assert!(find("inconnu").is_none());
    }
}
