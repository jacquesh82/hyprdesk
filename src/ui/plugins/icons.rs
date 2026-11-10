//! Paquet intégré « Icônes » : Raccourcis d'applications du bureau, taille, thème d'icônes.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use iced::widget::{button, column, pick_list, row, slider, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length, Task};

use super::super::panel::{Panel, PanelMsg};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::Origin;
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
    pub add_to_workspace: bool,
    pub themes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    IconSize(u16),
    CellSize(u16),
    Theme(String),
    Themes(Vec<String>),
    AppFilter(String),
    AddToWorkspace(bool),
    AddApp(String),
    RemoveIcon(usize),
    ResetLayout,
}

impl App {
    pub(in crate::ui) fn tab_icons<'a>(&'a self, p: &'a Panel) -> Element<'a, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
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
            let scope = match &ic.sel.workspaces {
                None => "tous les bureaux".to_string(),
                Some(w) => format!("workspace {}", w.join(", ")),
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
                            text(a.id.clone()).font(self.font).size(11).color(dim)
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
        column![
            self.section("Raccourcis"),
            shortcuts,
            text(format!(
                "+ {user_icons} icône(s) déclarée(s) dans hyprdesk.lua"
            ))
            .font(self.font)
            .size(12)
            .color(dim),
            self.field(
                "Ajouter une application",
                "",
                column![
                    text_input("Rechercher (firefox, terminal…)", &p.icons.app_filter)
                        .font(self.font)
                        .on_input(|s| wrap(Msg::AppFilter(s)))
                        .padding(8),
                    toggler(p.icons.add_to_workspace)
                        .label("Seulement sur ce workspace")
                        .on_toggle(|v| wrap(Msg::AddToWorkspace(v))),
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
                    .placeholder("Thème du système")
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
            Msg::AddToWorkspace(v) => {
                panel.icons.add_to_workspace = v;
                Task::none()
            }
            Msg::AddApp(id) => {
                let ws = panel
                    .icons
                    .add_to_workspace
                    .then(|| self.panel_ctx().map(|c| c.1))
                    .flatten();
                self.settings.add_app(&id, ws.as_deref());
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
