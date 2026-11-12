//! Paquet intégré « Thème des applications » : le thème de hyprdesk appliqué
//! à d'autres logiciels (kitty pour commencer), réappliqué à chaque
//! changement de thème. Domaine : `crate::apptheme`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::BTreeSet;

use crate::i18n::text;
use iced::widget::{Space, button, column, container, row, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::Config;

pub const INFO: PluginInfo = PluginInfo {
    id: "apptheme",
    label: "Thème des applications",
    icon: "preferences-desktop-theme",
    summary: "Le thème de hyprdesk appliqué à d'autres logiciels : kitty pour commencer.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::AppTheme(m)))
}

#[derive(Debug, Clone, Default)]
pub struct State {
    /// Compte rendu du dernier « Appliquer maintenant ».
    pub report: Vec<(String, Result<String, String>)>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Toggle(&'static str, bool),
    ApplyNow,
    Report(Vec<(String, Result<String, String>)>),
}

/// Cibles effectives : celles de la config, ou aucune si le paquet est coupé.
pub fn effective(cfg: &Config) -> BTreeSet<String> {
    if cfg.plugin_on(INFO.id) {
        cfg.general.app_themes.clone()
    } else {
        BTreeSet::new()
    }
}

impl App {
    /// Applique le thème aux applications choisies (hors du fil de l'interface).
    pub(in crate::ui) fn sync_app_themes(&self, report: bool) -> Task<UiMsg> {
        let (targets, theme) = (effective(&self.cfg), self.cfg.general.theme.clone());
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || crate::apptheme::sync(&targets, &theme))
                    .await
                    .unwrap_or_default()
            },
            move |r| {
                for (id, res) in &r {
                    if let Err(e) = res {
                        tracing::warn!("{}", crate::tr!("thème de {id} : {e}", id = id, e = e));
                    }
                }
                if report {
                    wrap(Msg::Report(r))
                } else {
                    UiMsg::Noop
                }
            },
        )
    }

    pub(in crate::ui) fn apptheme_page(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let accent = color(g.theme.accent, 1.0);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let on = self
            .settings
            .app_themes
            .clone()
            .unwrap_or_else(|| g.app_themes.clone());
        let mut col = column![
            self.section("Applications"),
            text(crate::i18n::tr(
                "Chaque application cochée prend les couleurs du thème en place, et les reprend \
                 à chaque changement de thème. Décochée, elle retrouve son propre thème."
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(14);
        for t in crate::apptheme::TARGETS {
            let installed = (t.installed)();
            let checked = on.contains(t.id);
            let status = if installed {
                text(crate::i18n::tr("installé")).color(dim)
            } else {
                text(crate::i18n::tr("introuvable")).color(warn)
            };
            col = col.push(
                container(
                    column![
                        row![
                            column![
                                row![
                                    text(t.label).font(self.title_font()).size(14),
                                    status.font(self.font).size(12),
                                ]
                                .spacing(8)
                                .align_y(Alignment::Center),
                                text(crate::i18n::tr(t.summary))
                                    .font(self.font)
                                    .size(12)
                                    .color(dim),
                            ]
                            .spacing(4)
                            .width(Length::Fill),
                            toggler(checked).on_toggle(move |v| wrap(Msg::Toggle(t.id, v))),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center),
                        text(crate::tr!(
                            "Fichier : {path}",
                            path = (t.file)().display().to_string()
                        ))
                        .font(self.font)
                        .size(12)
                        .color(dim),
                    ]
                    .spacing(8),
                )
                .padding(12)
                .style(move |_| container::Style {
                    border: Border::default()
                        .rounded(10)
                        .width(1)
                        .color(Color { a: 0.15, ..fg }),
                    ..Default::default()
                }),
            );
        }
        // Aperçu : les couleurs que kitty recevra.
        let swatch = |c: crate::config::Rgba, label: &'static str| {
            let c = color(c, 1.0);
            row![
                container(Space::new().width(18).height(18)).style(move |_| container::Style {
                    background: Some(c.into()),
                    border: Border::default()
                        .rounded(4)
                        .width(1)
                        .color(Color { a: 0.3, ..fg }),
                    ..Default::default()
                }),
                text(crate::i18n::tr(label))
                    .font(self.font)
                    .size(12)
                    .color(dim),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        };
        col = col.push(
            row![
                swatch(g.theme.fg, "texte"),
                swatch(g.theme.bg, "fond"),
                swatch(g.theme.accent, "accent, sélection, curseur"),
            ]
            .spacing(16),
        );
        col = col.push(
            button(
                text(crate::i18n::tr("Appliquer maintenant"))
                    .font(self.font)
                    .size(13),
            )
            .padding([6, 14])
            .on_press(wrap(Msg::ApplyNow)),
        );
        if let Some(p) = &self.panel {
            for (id, r) in &p.apptheme.report {
                col = col.push(
                    match r {
                        Ok(m) => text(m.clone()).color(accent),
                        Err(e) => text(crate::tr!("{id} : {e}", id = id, e = e)).color(warn),
                    }
                    .font(self.font)
                    .size(12),
                );
            }
        }
        col.into()
    }

    pub(in crate::ui) fn apptheme_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Toggle(id, v) => {
                let mut set = self
                    .settings
                    .app_themes
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.app_themes.clone());
                if v {
                    set.insert(id.to_string());
                } else {
                    set.remove(id);
                }
                self.settings.app_themes = Some(set);
                self.commit()
            }
            Msg::ApplyNow => self.sync_app_themes(true),
            Msg::Report(r) => {
                if let Some(p) = self.panel.as_mut() {
                    p.apptheme.report = r;
                }
                Task::none()
            }
        }
    }
}
