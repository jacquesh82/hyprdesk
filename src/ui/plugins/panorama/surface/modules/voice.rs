//! Tuile LYRA vocale : le micro (ouvrir, couper), l'état de l'écoute et la
//! dernière phrase entendue.
use iced::widget::{button, column, container, text};
use iced::{Alignment, Border, Element, Length, Size};

use super::super::{App, Msg, PMsg, fx, wrap};

impl App {
    /// Module LYRA vocale : le micro (un appui l'ouvre ou le coupe), l'état
    /// de l'écoute et la dernière phrase entendue.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_voice_module(
        &self,
        size: Size,
    ) -> Element<'_, Msg> {
        use super::super::VoiceUi;
        let (_, fg, accent) = self.panorama_palette();
        let t = self.cfg.general.panorama.design.text_scale;
        let dim = fx::alpha(fg, 0.65);
        let on = self.cfg.general.panorama.voice;
        let state = &self.panorama.voice;
        let label = match state {
            VoiceUi::Installing(_) => crate::i18n::tr("Installation de la voix…"),
            VoiceUi::Awake => crate::i18n::tr("Je vous écoute…"),
            VoiceUi::Thinking => crate::i18n::tr("LYRA réfléchit…"),
            VoiceUi::Speaking => crate::i18n::tr("LYRA parle…"),
            VoiceUi::Error(_) if on => crate::i18n::tr("LYRA vocale : erreur"),
            _ if on => crate::i18n::tr("LYRA vocale : dites « Lyra »"),
            _ => crate::i18n::tr("LYRA vocale (coupée)"),
        };
        let hot = matches!(
            state,
            VoiceUi::Awake | VoiceUi::Thinking | VoiceUi::Speaking
        );
        let side = (size.width.min(size.height) * 0.34).clamp(40.0, 110.0);
        let mut col = column![
            iced::widget::svg(super::super::dashboard::mic_icon(on))
                .width(side)
                .height(side)
                .opacity(if on { 1.0_f32 } else { 0.45 }),
            text(label)
                .size(13.0 * t)
                .color(if hot { accent } else { fg }),
        ]
        .spacing(10)
        .align_x(Alignment::Center);
        if let Some(h) = self
            .panorama
            .voice_heard
            .as_ref()
            .filter(|h| on && !h.is_empty())
        {
            col = col.push(
                text(crate::tr!("« {h} »", h = h.clone()))
                    .size(12.0 * t)
                    .color(dim),
            );
        }
        self.panorama_feedback(
            button(container(col).center_x(Length::Fill).center_y(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
                .on_press(wrap(PMsg::VoiceToggle))
                .style(move |_, st| button::Style {
                    background: matches!(st, button::Status::Pressed)
                        .then(|| fx::alpha(accent, 0.3).into()),
                    border: Border::default().rounded(14),
                    text_color: fg,
                    ..Default::default()
                }),
        )
    }
}
