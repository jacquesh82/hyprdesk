//! Tuile Notes : un grand bouton qui ouvre l'éditeur réglé sur l'écran
//! réglé (`crate::panorama::modules::notes`).
use std::time::Instant;

use iced::widget::{button, column, container, text};
use iced::{Alignment, Border, Element, Length, Size, Task};

use super::super::{App, Msg, PMsg, fx, wrap};
use crate::panorama::modules::notes;

impl App {
    pub(in crate::ui::plugins::panorama::surface) fn panorama_notes_update(
        &mut self,
        m: PMsg,
    ) -> Task<Msg> {
        match m {
            PMsg::NotesOpen => {
                self.panorama.last_press = Some(Instant::now());
                let (command, monitor) = notes::options(&self.cfg);
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || notes::open(&command, &monitor))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::NotesOpened(r)),
                )
            }
            PMsg::NotesOpened(r) => {
                self.panorama.notes_error = r.err();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// Tuile Notes : un grand bouton qui ouvre l'éditeur sur l'écran réglé.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_notes(
        &self,
        size: Size,
    ) -> Element<'_, Msg> {
        let (_, fg, accent) = self.panorama_palette();
        let t = self.cfg.general.panorama.design.text_scale;
        let dim = fx::alpha(fg, 0.65);
        let (command, monitor) = notes::options(&self.cfg);
        let side = (size.width.min(size.height) * 0.32).clamp(40.0, 96.0);
        let icon = self
            .icon_named("accessories-text-editor", side, fg)
            .unwrap_or_else(|| text("✎").size(side * 0.7).color(fg).into());
        let hint = if command.is_empty() {
            crate::i18n::tr("Réglez l'éditeur dans Réglages → Plugins → Modules.").to_string()
        } else if monitor.is_empty() {
            crate::i18n::tr("S'ouvre sur l'écran actif.").to_string()
        } else {
            crate::tr!("S'ouvre sur {m}.", m = monitor)
        };
        let mut col = column![
            icon,
            text(crate::i18n::tr("Nouvelle note"))
                .size(17.0 * t)
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..self.panorama_font()
                })
                .color(fg),
            text(hint).size(12.0 * t).color(dim),
        ]
        .spacing(8)
        .align_x(Alignment::Center);
        if let Some(e) = &self.panorama.notes_error {
            col = col.push(
                text(e.clone())
                    .size(12.0 * t)
                    .color(crate::ui::tone::warning()),
            );
        }
        self.panorama_feedback(
            button(container(col).center_x(Length::Fill).center_y(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(12)
                .on_press(wrap(PMsg::NotesOpen))
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
