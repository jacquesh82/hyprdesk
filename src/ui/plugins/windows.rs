//! Paquet intégré « Fenêtres » (MLG-141) : espacement et bordures des
//! fenêtres, règles de fenêtres, groupes à onglets. Les options passent par
//! le registre `crate::hyprset` et le rendu unique `ui::hyprset` ; elles sont
//! écrites dans `appearance.lua` et `rules.lua` (`crate::genfiles`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use iced::Element;

use super::super::{App, Msg as UiMsg};
use super::PluginInfo;
use crate::i18n::text;

pub const INFO: PluginInfo = PluginInfo {
    id: "windows",
    label: "Fenêtres",
    icon: "preferences-system-windows",
    summary: "Espacement et bordures des fenêtres, règles par application, groupes à onglets.",
    builtin: true,
};

pub const TABS: &[super::SubTab] = &[
    ("apparence", "Apparence"),
    ("comportement", "Comportement"),
    ("regles", "Règles"),
    ("groupes", "Groupes"),
];

impl App {
    pub(in crate::ui) fn windows_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        if self.on_subtab(INFO.id, "regles") {
            return self.rules_editor();
        }
        if self.on_subtab(INFO.id, "comportement") {
            return self.hyprset_tab("comportement-fenetres");
        }
        if self.on_subtab(INFO.id, "groupes") {
            return iced::widget::column![
                text(
                    "Un groupe range plusieurs fenêtres sous des onglets, à la place d'une \
                     seule (raccourcis de groupe : Réglages → Raccourcis)."
                )
                .font(self.font)
                .size(12)
                .color(dim),
                self.hyprset_tab("groupes"),
            ]
            .spacing(14)
            .into();
        }
        iced::widget::column![
            text(
                "Écrits dans appearance.lua, chargé par hyprland.lua ; une valeur non réglée \
                 ici dit d'où elle vient."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.theme_borders_field(),
            self.hyprset_tab("fenetres"),
        ]
        .spacing(14)
        .into()
    }
}
