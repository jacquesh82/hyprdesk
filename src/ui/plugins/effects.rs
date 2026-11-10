//! Paquet intégré « Effets » : Transitions, fondu entre fonds, couleur d'accent.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use iced::widget::{Space, button, column, radio, row, slider, text};
use iced::{Border, Color, Element, Task};

use super::super::panel::{PanelMsg, SWATCHES};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::{Rgba, Transition};

pub const INFO: PluginInfo = PluginInfo {
    id: "effects",
    label: "Effets",
    icon: "preferences-system",
    summary: "Transitions, fondu entre fonds, couleur d'accent.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Effects(m)))
}

#[derive(Debug, Clone)]
pub enum Msg {
    Transition(Transition),
    FadeMs(u64),
    CrossfadeMs(u64),
    Accent(Rgba),
    /// Idem, puis rejoue l'apparition du bureau (aperçu de la durée).
    CommitFade,
    /// Idem, puis rejoue un fondu enchaîné de fond (aperçu).
    CommitCrossfade,
}

impl App {
    pub(in crate::ui) fn tab_effects(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let swatches = row(SWATCHES[..6].iter().map(|c| {
            let col = color(*c, 1.0);
            let selected = g.theme.accent == *c;
            button(Space::new().width(32).height(32))
                .padding(0)
                .on_press(wrap(Msg::Accent(*c)))
                .style(move |_, _| button::Style {
                    background: Some(col.into()),
                    border: Border::default()
                        .rounded(16)
                        .width(if selected { 3 } else { 1 })
                        .color(if selected { fg } else { Color { a: 0.3, ..fg } }),
                    ..Default::default()
                })
                .into()
        }))
        .spacing(10);
        column![
            self.section("Apparition du bureau"),
            self.field(
                "Transition",
                "transition",
                row![
                    radio("Fondu", Transition::Fade, Some(g.transition), |v| {
                        wrap(Msg::Transition(v))
                    })
                    .font(self.font),
                    radio("Glissement", Transition::Slide, Some(g.transition), |v| {
                        wrap(Msg::Transition(v))
                    })
                    .font(self.font),
                    radio("Aucune", Transition::None, Some(g.transition), |v| {
                        wrap(Msg::Transition(v))
                    })
                    .font(self.font),
                ]
                .spacing(16),
            ),
            self.field(
                "Durée",
                "fade_ms",
                row![
                    slider(0..=1000u32, g.fade_ms.min(1000) as u32, |v| wrap(
                        Msg::FadeMs(v as u64)
                    ))
                    .step(10u32)
                    .on_release(wrap(Msg::CommitFade))
                    .width(260),
                    text(format!("{} ms", g.fade_ms)).font(self.font).size(14),
                ]
                .spacing(12),
            ),
            self.section("Fonds d'écran"),
            self.field(
                "Fondu entre deux fonds",
                "crossfade_ms",
                row![
                    slider(0..=3000u32, g.crossfade_ms.min(3000) as u32, |v| wrap(
                        Msg::CrossfadeMs(v as u64)
                    ))
                    .step(50u32)
                    .on_release(wrap(Msg::CommitCrossfade))
                    .width(260),
                    text(format!("{} ms", g.crossfade_ms))
                        .font(self.font)
                        .size(14),
                ]
                .spacing(12),
            ),
            self.section("Couleurs"),
            self.field("Couleur d'accent", "theme.accent", swatches),
        ]
        .spacing(14)
        .into()
    }

    pub(in crate::ui) fn effects_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Transition(t) => {
                self.settings.transition = Some(t);
                let c = self.commit();
                self.replay_fade();
                c
            }
            Msg::FadeMs(v) => {
                self.settings.fade_ms = Some(v);
                self.apply()
            }
            Msg::CrossfadeMs(v) => {
                self.settings.crossfade_ms = Some(v);
                self.apply()
            }
            Msg::Accent(c) => {
                self.settings.accent = Some(c);
                self.commit()
            }
            Msg::CommitFade => {
                let c = self.commit();
                self.replay_fade();
                c
            }
            Msg::CommitCrossfade => {
                let c = self.commit();
                if let Some((mon, _)) = self.panel_ctx() {
                    self.replay_crossfade(&mon);
                }
                c
            }
        }
    }
}
