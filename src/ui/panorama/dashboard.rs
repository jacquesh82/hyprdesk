//! Composition libre : les coordonnées sont normalisées, le portrait s'adapte.
use super::panes::{Geo, Surf};
use super::{App, Msg, PMsg, fx, instruments, wrap};
use crate::panorama::layout::Rect;
use iced::widget::{Space, button, column, container, image, pin, row, stack, text};
use iced::{Alignment, Border, Color, Element, Length, Size};

impl App {
    pub(super) fn panorama_dashboard<'a>(
        &'a self,
        s: &Surf,
        geo: &Geo,
        video_at: Option<Rect>,
    ) -> Element<'a, Msg> {
        let d = &self.cfg.general.panorama.design;
        let (bg, fg, accent) = self.panorama_palette();
        let mut layers: Vec<Element<'a, Msg>> =
            vec![Space::new().width(Length::Fill).height(Length::Fill).into()];
        for p in &geo.panes {
            let body = self.panorama_pane_view(s, geo, p, false, video_at);
            let content: Element<'a, Msg> = if p.widget == "video" {
                body
            } else {
                container(body)
                    .width(p.rect.w)
                    .height(p.rect.h)
                    .clip(true)
                    .style(move |_| container::Style {
                        background: Some(fx::alpha(bg, d.card_opacity).into()),
                        border: Border {
                            radius: d.radius.into(),
                            width: if d.border_opacity > 0.0 { 1.0 } else { 0.0 },
                            color: fx::alpha(fg, d.border_opacity),
                        },
                        ..Default::default()
                    })
                    .into()
            };
            layers.push(pin(content).x(p.rect.x).y(p.rect.y).into());
        }
        if d.controls {
            layers.push(
                container(self.panorama_controls(s, geo, fg, false))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(8)
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .into(),
            );
        }
        let _ = accent;
        stack(layers).width(s.w).height(s.h).into()
    }

    pub(super) fn panorama_calendar(&self, size: Size) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let d = &self.cfg.general.panorama.design;
        let font = (size.width / 25.0)
            .min(size.height / 13.0)
            .clamp(10.0, 25.0)
            * d.text_scale;
        let _ = bg;
        container(self.view_calendar(
            font,
            fg,
            fx::alpha(fg, 0.55),
            accent,
            Some((size.width - 24.0).max(50.0)),
            Some(0),
        ))
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    pub(super) fn panorama_shortcuts(&self, size: Size) -> Element<'_, Msg> {
        let d = &self.cfg.general.panorama.design;
        let (bg, fg, accent) = self.panorama_palette();
        if d.shortcuts.is_empty() {
            return container(
                text(crate::i18n::tr(
                    "Ajoutez vos raccourcis dans Réglages → Panorama → Raccourcis.",
                ))
                .size(14)
                .color(fg),
            )
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }
        let columns = d.shortcut_columns as usize;
        let gap = d.gap.min(20.0);
        let width = ((size.width - 16.0 - gap * (columns - 1) as f32) / columns as f32).max(28.0);
        let rows = d.shortcuts.len().div_ceil(columns);
        let height =
            ((size.height - 16.0 - gap * (rows - 1) as f32) / rows as f32).clamp(44.0, 120.0);
        let mut grid = column![].spacing(gap);
        for (r, chunk) in d.shortcuts.chunks(columns).enumerate() {
            let mut line = row![].spacing(gap);
            for (j, s) in chunk.iter().enumerate() {
                let icon = self
                    .icon_named(&s.icon, (height * 0.37).min(width * 0.35), fg)
                    .unwrap_or_else(|| {
                        text(s.name.chars().next().unwrap_or('·').to_string())
                            .size(24)
                            .color(fg)
                            .into()
                    });
                let label = column![
                    icon,
                    text(s.name.clone())
                        .size((width * 0.14).clamp(10.0, 16.0))
                        .color(fg)
                        .wrapping(text::Wrapping::None)
                ]
                .spacing(5)
                .align_x(Alignment::Center);
                line = line.push(
                    self.panorama_feedback(
                        button(
                            container(label)
                                .center_x(Length::Fill)
                                .center_y(Length::Fill),
                        )
                        .width(width)
                        .height(height)
                        .on_press(wrap(PMsg::Shortcut(r * columns + j)))
                        .style(move |_, st| button::Style {
                            background: Some(
                                fx::alpha(
                                    if matches!(
                                        st,
                                        button::Status::Hovered | button::Status::Pressed
                                    ) {
                                        accent
                                    } else {
                                        bg
                                    },
                                    if matches!(
                                        st,
                                        button::Status::Hovered | button::Status::Pressed
                                    ) {
                                        0.55
                                    } else {
                                        0.6
                                    },
                                )
                                .into(),
                            ),
                            border: Border {
                                radius: d.radius.min(16.0).into(),
                                width: 1.0,
                                color: fx::alpha(fg, 0.25),
                            },
                            ..Default::default()
                        }),
                    ),
                );
            }
            grid = grid.push(line);
        }
        container(iced::widget::scrollable(grid))
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    pub(super) fn panorama_vinyl(&self, size: Size) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let side = (size.width / 1.55).min(size.height * 0.65).max(20.0);
        let cover: Element<'_, Msg> = match &self.panorama.cover {
            Some((_, h)) => image(h.clone())
                .width(side)
                .height(side)
                .content_fit(iced::ContentFit::Cover)
                .into(),
            None => fx::album(side, bg, accent),
        };
        let art = stack![
            pin(instruments::record(side, fg, accent)).x(side * 0.45),
            container(cover).style(move |_| container::Style {
                border: Border {
                    color: fx::alpha(fg, 0.65),
                    width: 1.0,
                    radius: 0.0.into()
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    offset: iced::Vector::new(8.0, 4.0),
                    blur_radius: 16.0
                },
                ..Default::default()
            })
        ]
        .width(side * 1.45)
        .height(side);
        let media = self.panorama.media.as_ref();
        let title = media
            .filter(|m| !m.title.is_empty())
            .map(|m| m.title.clone())
            .unwrap_or_else(|| crate::i18n::tr("Rien ne joue").into());
        let playing = media.is_some_and(|m| m.playing);
        let controls = row![
            self.panorama_button(
                "media-skip-backward-symbolic",
                44.0,
                fg,
                wrap(PMsg::Control("Previous"))
            ),
            self.panorama_button(
                if playing {
                    "media-playback-pause-symbolic"
                } else {
                    "media-playback-start-symbolic"
                },
                52.0,
                fg,
                wrap(PMsg::Control("PlayPause"))
            ),
            self.panorama_button(
                "media-skip-forward-symbolic",
                44.0,
                fg,
                wrap(PMsg::Control("Next"))
            )
        ]
        .spacing(16)
        .align_y(Alignment::Center);
        container(
            column![
                art,
                text(title)
                    .size((size.height * 0.055).clamp(12.0, 26.0))
                    .color(fg)
                    .font(self.panorama_font())
                    .wrapping(text::Wrapping::None),
                controls
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .clip(true)
        .into()
    }

    pub(super) fn panorama_compact_media(&self, size: Size) -> Element<'_, Msg> {
        let (_, fg, accent) = self.panorama_palette();
        let d = &self.cfg.general.panorama.design;
        let m = self.panorama.media.as_ref();
        let playing = m.is_some_and(|s| s.playing);
        let title = m
            .filter(|s| !s.title.is_empty())
            .map(|s| s.title.clone())
            .unwrap_or_else(|| crate::i18n::tr("Rien ne joue").into());
        let artist = m.map(|s| s.artist.clone()).unwrap_or_default();
        let controls = row![
            self.panorama_button(
                "media-skip-backward-symbolic",
                44.0,
                fg,
                wrap(PMsg::Control("Previous"))
            ),
            self.panorama_button(
                if playing {
                    "media-playback-pause-symbolic"
                } else {
                    "media-playback-start-symbolic"
                },
                48.0,
                fg,
                wrap(PMsg::Control("PlayPause"))
            ),
            self.panorama_button(
                "media-skip-forward-symbolic",
                44.0,
                fg,
                wrap(PMsg::Control("Next"))
            )
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let mut col = column![
            text(title)
                .font(self.panorama_font())
                .size((size.height * 0.15).clamp(14.0, 36.0) * d.text_scale)
                .color(fg)
                .wrapping(text::Wrapping::None),
            text(artist)
                .font(self.panorama_font())
                .size((size.height * 0.08).clamp(11.0, 20.0) * d.text_scale)
                .color(fx::alpha(fg, 0.65))
                .wrapping(text::Wrapping::None),
            Space::new().height(Length::Fill)
        ]
        .spacing(8);
        if let Some(s) = m
            && let (Some(p), Some(l)) = (s.position, s.length)
            && l > 0
            && l < 48 * 3600
        {
            col = col.push(
                iced::widget::progress_bar(0.0..=1.0, (p as f32 / l as f32).clamp(0.0, 1.0))
                    .girth(4)
                    .style(move |_| iced::widget::progress_bar::Style {
                        background: fx::alpha(fg, 0.15).into(),
                        bar: accent.into(),
                        border: Border::default().rounded(2),
                    }),
            );
        }
        container(col.push(controls))
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .into()
    }
}
