//! Onglet « Bureau » du paquet Animations (ancien paquet « Effets ») :
//! transitions du bureau de hyprdesk, fondu entre fonds. Opacité et flou
//! des surfaces : Thème → Transparence.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{column, radio, row};
use iced::{Alignment, Color, Element, Task};

use super::super::panel::{PanelMsg, SliderLook, SliderSpec};
use super::super::{App, Msg as UiMsg, color};
use super::PluginMsg;
use crate::config::Transition;

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

/// Le curseur « Durée » du fondu d'apparition, de 0 à 1000 ms par pas de 10 :
/// sa valeur se saisit au clavier (HIG SEL-08).
pub(in crate::ui) fn fade_spec() -> SliderSpec {
    SliderSpec::new(
        "fade_ms",
        (0.0, 1000.0),
        10.0,
        |v| wrap(Msg::FadeMs(v.round() as u64)),
        wrap(Msg::CommitFade),
    )
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
                self.slider(
                    &fade_spec(),
                    g.fade_ms.min(1000) as f64,
                    |v| format!("{} ms", v.round() as u64),
                    SliderLook::fixed(13.0, 260, Alignment::Start),
                ),
            ),
            self.section("Fonds d'écran"),
            self.field(
                "Fondu entre deux fonds",
                "crossfade_ms",
                self.slider(
                    &SliderSpec::new(
                        "crossfade_ms",
                        (0.0, 3000.0),
                        50.0,
                        |v| wrap(Msg::CrossfadeMs(v.round() as u64)),
                        wrap(Msg::CommitCrossfade)
                    ),
                    g.crossfade_ms.min(3000) as f64,
                    |v| format!("{} ms", v.round() as u64),
                    SliderLook::fixed(13.0, 260, Alignment::Start),
                ),
            ),
            text("Couleurs, opacité et flou de chaque endroit : paquet Thème.")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
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
