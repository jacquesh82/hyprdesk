//! Paquet intégré « Modules » : les modules du Panorama (Hue, Notes, Météo,
//! LYRA vocale…), rangés sous Plugins avec les widgets et les plugins installés.
//! La page et ses messages restent ceux du Panorama (`panorama::modules`) :
//! seul l'endroit où on les trouve change.

use iced::Element;
use iced::widget::{column, text};

use super::PluginInfo;
use crate::ui::{App, Msg as UiMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "modules",
    label: "Modules",
    icon: "applications-other",
    summary: "Modules du Panorama : Hue, Notes, Météo, LYRA vocale…",
    builtin: true,
};

impl App {
    pub(in crate::ui) fn modules_page(&self) -> Element<'_, UiMsg> {
        if !self.cfg.plugin_on("panorama") {
            return column![
                text("Les modules vivent dans le Panorama, coupé dans Plugins → Installés.")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(self.ink().dim)
            ]
            .into();
        }
        self.panorama_modules_tab()
    }
}
