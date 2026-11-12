//! Paquet intégré « Icônes » : Raccourcis d'applications du bureau, taille, thème d'icônes.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, pick_list, radio, row, slider, text_input, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length, Task};

use super::super::panel::{Panel, PanelMsg};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::{IconScope, Origin};
use crate::launch::AppInfo;

pub const INFO: PluginInfo = PluginInfo {
    id: "icons",
    label: "Icônes",
    icon: "view-grid",
    summary: "Raccourcis d'applications du bureau, taille, thème d'icônes.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Icons(m)))
}

/// État propre à ce paquet, porté par le tiroir des réglages.
#[derive(Default)]
pub struct State {
    pub app_filter: String,
    pub themes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    IconSize(u16),
    CellSize(u16),
    Theme(String),
    Themes(Vec<String>),
    AppFilter(String),
    Scope(IconScope),
    DoubleClick(bool),
    AddApp(String),
    RemoveIcon(usize),
    ResetLayout,
}

impl App {
    pub(in crate::ui) fn tab_icons<'a>(&'a self, p: &'a Panel) -> Element<'a, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let themes: Vec<String> = p.icons.themes.clone();
        let mut shortcuts = column![].spacing(6);
        if self.settings.icons.is_empty() {
            shortcuts = shortcuts.push(
                text("Aucun raccourci ajouté ici.")
                    .font(self.font)
                    .size(13)
                    .color(dim),
            );
        }
        for (i, ic) in self.settings.icons.iter().enumerate() {
            let name = match &ic.kind {
                crate::config::IconKind::App(a) => self
                    .apps
                    .iter()
                    .find(|x| &x.id == a)
                    .map(|x| x.name.clone())
                    .unwrap_or_else(|| a.clone()),
                crate::config::IconKind::Path(p) => p.display().to_string(),
                crate::config::IconKind::Exec { cmd, .. } => cmd.clone(),
            };
            let scope = match (&ic.sel.monitors, &ic.sel.workspaces) {
                (Some(m), _) if g.icon_scope == IconScope::Screen => {
                    crate::tr!("écran {list}", list = m.join(", "))
                }
                (_, Some(w)) if g.icon_scope == IconScope::Workspace => {
                    crate::tr!("workspace {list}", list = w.join(", "))
                }
                _ => crate::i18n::tr("tous les bureaux").to_string(),
            };
            shortcuts = shortcuts.push(
                row![
                    text(name).font(self.font).size(14).width(Length::Fill),
                    text(scope).font(self.font).size(12).color(dim),
                    button(text("Retirer").font(self.font).size(13))
                        .padding([4, 10])
                        .on_press(wrap(Msg::RemoveIcon(i))),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }
        let user_icons = self
            .cfg
            .icons
            .iter()
            .filter(|i| i.origin == Origin::User)
            .count();
        let filter = p.icons.app_filter.to_lowercase();
        let mut picker = column![].spacing(2);
        if !filter.is_empty() {
            let matches: Vec<&AppInfo> = self
                .apps
                .iter()
                .filter(|a| {
                    a.name.to_lowercase().contains(&filter) || a.id.to_lowercase().contains(&filter)
                })
                .take(12)
                .collect();
            if matches.is_empty() {
                picker = picker.push(
                    text("Aucune application trouvée.")
                        .font(self.font)
                        .size(13)
                        .color(dim),
                );
            }
            for a in matches {
                picker = picker.push(
                    button(
                        row![
                            text(a.name.clone())
                                .font(self.font)
                                .size(14)
                                .width(Length::Fill),
                            text(a.id.clone()).font(self.font).size(12).color(dim)
                        ]
                        .spacing(8),
                    )
                    .width(Length::Fill)
                    .padding([6, 10])
                    .on_press(wrap(Msg::AddApp(a.id.clone())))
                    .style(move |_, s| button::Style {
                        background: matches!(s, button::Status::Hovered)
                            .then(|| Background::Color(Color { a: 0.1, ..fg })),
                        text_color: fg,
                        border: Border::default().rounded(6),
                        ..Default::default()
                    }),
                );
            }
        }
        let scope_radio = |label: &'static str, v: IconScope| {
            radio(crate::i18n::tr(label), v, Some(g.icon_scope), |v| wrap(Msg::Scope(v)))
                .font(self.font)
        };
        column![
            self.section("Comportement"),
            self.field(
                "Portée des icônes",
                "icon_scope",
                column![
                    scope_radio("Globales — les mêmes icônes partout", IconScope::Global),
                    scope_radio("Par écran — chaque écran a les siennes", IconScope::Screen),
                    scope_radio("Par workspace — chaque workspace a les siennes", IconScope::Workspace),
                    text(crate::i18n::tr(
                        "Une icône ajoutée (ici ou en déposant un fichier sur le bureau) va sur l'écran ou le workspace en cours ; celles d'avant restent où elles étaient."
                    ))
                    .font(self.font)
                    .size(12)
                    .color(dim),
                ]
                .spacing(8),
            ),
            self.field(
                "Ouvrir d'un double clic (un clic sélectionne)",
                "icon_double_click",
                toggler(g.icon_double_click).on_toggle(|v| wrap(Msg::DoubleClick(v))),
            ),
            text(crate::i18n::tr(
                "Clic droit sur une icône : ouvrir, épingler au dock, renommer, retirer du bureau… Échap retire la sélection, Entrée ouvre l'icône sélectionnée."
            ))
            .font(self.font)
            .size(12)
            .color(dim),
            self.section("Raccourcis"),
            shortcuts,
            text(crate::tr!(
                "+ {user_icons} icône(s) déclarée(s) dans hyprdesk.lua",
                user_icons = user_icons
            ))
            .font(self.font)
            .size(12)
            .color(dim),
            self.field(
                "Ajouter une application",
                "",
                column![
                    text_input(
                        crate::i18n::tr("Rechercher (firefox, terminal…)"),
                        &p.icons.app_filter
                    )
                    .font(self.font)
                    .on_input(|s| wrap(Msg::AppFilter(s)))
                    .padding(8),
                    picker,
                ]
                .spacing(8),
            ),
            self.section("Apparence"),
            self.field(
                "Taille des icônes",
                "grid.icon_size",
                row![
                    slider(24..=128u16, g.grid.icon_size, |v| wrap(Msg::IconSize(v)))
                        .on_release(UiMsg::Panel(PanelMsg::Commit))
                        .width(260),
                    text(format!("{} px", g.grid.icon_size))
                        .font(self.font)
                        .size(14),
                ]
                .spacing(12),
            ),
            self.field(
                "Taille des cases",
                "grid.cell",
                row![
                    slider(64..=192u16, g.grid.cell, |v| wrap(Msg::CellSize(v)))
                        .on_release(UiMsg::Panel(PanelMsg::Commit))
                        .width(260),
                    text(format!("{} px", g.grid.cell)).font(self.font).size(14),
                ]
                .spacing(12),
            ),
            self.field(
                "Thème d'icônes",
                "icon_theme",
                pick_list(themes, g.icon_theme.clone(), |t| wrap(Msg::Theme(t)))
                    .font(self.font)
                    .placeholder(crate::i18n::tr("Thème du système"))
                    .width(300),
            ),
            self.field(
                "Disposition de ce bureau",
                "",
                button(text("Réinitialiser les positions").font(self.font).size(14))
                    .padding([6, 12])
                    .on_press(wrap(Msg::ResetLayout)),
            ),
        ]
        .spacing(14)
        .into()
    }

    pub(in crate::ui) fn icons_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::IconSize(v) => {
                self.settings.icon_size = Some(v);
                self.apply()
            }
            Msg::CellSize(v) => {
                self.settings.cell = Some(v);
                self.apply()
            }
            Msg::Theme(t) => {
                self.settings.icon_theme = Some(t);
                self.commit()
            }
            Msg::Themes(t) => {
                panel.icons.themes = t;
                Task::none()
            }
            Msg::AppFilter(f) => {
                panel.icons.app_filter = f;
                Task::none()
            }
            Msg::Scope(v) => {
                self.settings.icon_scope = Some(v);
                self.commit()
            }
            Msg::DoubleClick(v) => {
                self.settings.icon_double_click = Some(v);
                self.commit()
            }
            Msg::AddApp(id) => {
                let (mon, ws) = self.panel_ctx().unwrap_or_default();
                let sel = self.cfg.general.icon_scope.selector(&mon, &ws);
                self.settings.add_app(&id, sel);
                if let Some(p) = self.panel.as_mut() {
                    p.icons.app_filter.clear();
                }
                self.commit()
            }
            Msg::RemoveIcon(i) => {
                self.settings.remove_icon(i);
                self.commit()
            }
            Msg::ResetLayout => {
                let (mon, ws) = self.panel_ctx().unwrap_or_default();
                self.layout.clear_desktop(&mon, &ws);
                self.layout.save();
                Task::none()
            }
        }
    }
}
