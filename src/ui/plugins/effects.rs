//! Paquet intégré « Effets » : transitions du bureau, fondu entre fonds.
//! Couleurs et effets de surface : paquet « Thème ».
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{column, radio, row, slider};
use iced::{Color, Element, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::Transition;

pub const INFO: PluginInfo = PluginInfo {
    id: "effects",
    label: "Effets",
    icon: "preferences-system",
    summary: "Transitions du bureau et fondu entre fonds d'écran.",
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
    /// Idem, puis rejoue l'apparition du bureau (aperçu de la durée).
    CommitFade,
    /// Idem, puis rejoue un fondu enchaîné de fond (aperçu).
    CommitCrossfade,
}

impl App {
    pub(in crate::ui) fn tab_effects(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        column![
            self.section("Apparition du bureau"),
            self.field(
                "Transition",
                "transition",
                row![
                    radio(
                        crate::i18n::tr("Fondu"),
                        Transition::Fade,
                        Some(g.transition),
                        |v| { wrap(Msg::Transition(v)) }
                    )
                    .font(self.font),
                    radio(
                        crate::i18n::tr("Glissement"),
                        Transition::Slide,
                        Some(g.transition),
                        |v| { wrap(Msg::Transition(v)) }
                    )
                    .font(self.font),
                    radio(
                        crate::i18n::tr("Aucune"),
                        Transition::None,
                        Some(g.transition),
                        |v| { wrap(Msg::Transition(v)) }
                    )
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
            text("Couleurs, opacité et flou de chaque endroit : paquet Thème.")
                .font(self.font)
                .size(12)
                .color(Color { a: 0.6, ..fg }),
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
