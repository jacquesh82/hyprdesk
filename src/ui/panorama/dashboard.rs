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

    /// Colonne « playlist » des profils Vidéo et Musique : les services du
    /// lecteur, celui qui joue en évidence ; un appui l'ouvre dans la zone
    /// du lecteur.
    pub(super) fn panorama_playlist(&self, size: Size) -> Element<'_, Msg> {
        let c = &self.cfg.general.panorama;
        let d = &c.design;
        let (bg, fg, accent) = self.panorama_palette();
        let gap = d.gap.min(14.0);
        let playing = self.panorama.video.as_ref().map(|v| v.name.as_str());
        let title = if d.shortcut_profile == "audio" {
            "Musique"
        } else {
            "Vidéo"
        };
        let mut head = row![
            text(crate::i18n::tr(title))
                .size(16.0 * d.text_scale)
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..self.panorama_font()
                })
                .color(fg)
                .width(Length::Fill)
        ]
        .align_y(Alignment::Center);
        if playing.is_some() {
            head = head.push(
                self.panorama_feedback(
                    button(text(crate::i18n::tr("Fermer")).size(13).color(fg))
                        .padding([6, 12])
                        .on_press(wrap(PMsg::VideoClose))
                        .style(move |_, st| button::Style {
                            background: Some(
                                fx::alpha(
                                    accent,
                                    if matches!(st, button::Status::Pressed) {
                                        0.6
                                    } else {
                                        0.25
                                    },
                                )
                                .into(),
                            ),
                            border: Border::default().rounded(10),
                            text_color: fg,
                            ..Default::default()
                        }),
                ),
            );
        }
        let sources = c.active_sources();
        // Lignes tactiles : toute la hauteur partagée, entre 44 et 72 px.
        let room = size.height - 16.0 - 34.0 - gap;
        let n = sources.len().max(1) as f32;
        let line_h = ((room - gap / 2.0 * (n - 1.0)) / n).clamp(40.0, 72.0);
        let icon_side = line_h * 0.62;
        let mut list = column![].spacing(gap / 2.0);
        for (i, src) in sources.iter().enumerate() {
            let on = playing == Some(src.name.as_str());
            let icon: Element<'_, Msg> = (!src.icon.is_empty())
                .then(|| self.icon_named(&src.icon, icon_side, fg))
                .flatten()
                .unwrap_or_else(|| {
                    let letter = src
                        .name
                        .chars()
                        .find(|ch| ch.is_alphanumeric())
                        .map(|ch| ch.to_uppercase().to_string())
                        .unwrap_or_else(|| "•".into());
                    container(text(letter).size(icon_side * 0.55).color(fg))
                        .width(icon_side)
                        .height(icon_side)
                        .center_x(icon_side)
                        .center_y(icon_side)
                        .style(move |_| container::Style {
                            background: Some(fx::alpha(accent, 0.3).into()),
                            border: Border::default().rounded(icon_side * 0.24),
                            ..Default::default()
                        })
                        .into()
                });
            let mut label = column![
                text(src.name.clone())
                    .size((line_h * 0.26).clamp(12.0, 17.0) * d.text_scale)
                    .color(fg)
                    .wrapping(text::Wrapping::None)
            ];
            if on {
                label = label.push(
                    text(crate::i18n::tr("En cours"))
                        .size(11.0 * d.text_scale)
                        .color(fx::alpha(fg, 0.7)),
                );
            }
            list = list.push(
                self.panorama_feedback(
                    button(row![icon, label].spacing(12).align_y(Alignment::Center))
                        .width(Length::Fill)
                        .height(line_h)
                        .padding([4, 10])
                        .on_press(wrap(PMsg::VideoOpen(i)))
                        .style(move |_, st| {
                            let hot = on || matches!(st, button::Status::Pressed);
                            button::Style {
                                background: Some(
                                    fx::alpha(
                                        if hot { accent } else { bg },
                                        if hot { 0.45 } else { 0.5 },
                                    )
                                    .into(),
                                ),
                                border: Border {
                                    radius: d.radius.min(14.0).into(),
                                    width: 1.0,
                                    color: fx::alpha(
                                        if on { accent } else { fg },
                                        if on { 0.8 } else { 0.2 },
                                    ),
                                },
                                text_color: fg,
                                ..Default::default()
                            }
                        }),
                ),
            );
        }
        if sources.is_empty() {
            list = list.push(
                text(crate::i18n::tr(
                    "Aucune source : ajoutez-en dans Réglages → Panorama.",
                ))
                .size(13)
                .color(fg),
            );
        }
        column![
            container(head).height(34),
            iced::widget::scrollable(list)
                .height(Length::Fill)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .scroller_width(4)
                        .spacing(6),
                ))
        ]
        .spacing(gap)
        .padding(8)
        .into()
    }

    /// Bloc de raccourcis : sélecteur de profil (vidéo, audio,
    /// télécommande, système) au-dessus des touches du profil choisi.
    pub(super) fn panorama_shortcuts(&self, size: Size) -> Element<'_, Msg> {
        let d = &self.cfg.general.panorama.design;
        let (bg, fg, accent) = self.panorama_palette();
        let gap = d.gap.min(20.0);
        let radius = d.radius.min(16.0);
        let tile_style = move |on: bool| {
            move |_: &iced::Theme, st: button::Status| {
                let hot = on || matches!(st, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: Some(
                        fx::alpha(if hot { accent } else { bg }, if hot { 0.55 } else { 0.6 })
                            .into(),
                    ),
                    border: Border {
                        radius: radius.into(),
                        width: 1.0,
                        color: fx::alpha(if on { accent } else { fg }, if on { 0.8 } else { 0.25 }),
                    },
                    text_color: fg,
                    ..Default::default()
                }
            }
        };

        // Sélecteur : icône et nom ; icône seule quand la place manque.
        let profiles = crate::panorama::design::PROFILES;
        let seg_w = ((size.width - 16.0 - gap * (profiles.len() - 1) as f32)
            / profiles.len() as f32)
            .max(28.0);
        let seg_h = (size.height * 0.16).clamp(34.0, 52.0);
        let named = seg_w >= 110.0;
        let mut tabs = row![].spacing(gap);
        for id in profiles {
            let (label, _) = crate::panorama::design::profile_label(id);
            let on = d.shortcut_profile == *id;
            let mut inner = row![].spacing(6).align_y(Alignment::Center);
            // Icône colorée du profil (pas teintée par le thème).
            let side = (seg_h * 0.62).min(34.0);
            inner = inner.push(iced::widget::svg(profile_icon(id)).width(side).height(side));
            if named {
                inner = inner.push(
                    text(crate::i18n::tr(label))
                        .size((seg_h * 0.3).clamp(11.0, 15.0))
                        .color(fg)
                        .wrapping(text::Wrapping::None),
                );
            }
            tabs = tabs.push(
                self.panorama_feedback(
                    button(
                        container(inner)
                            .center_x(Length::Fill)
                            .center_y(Length::Fill),
                    )
                    .width(seg_w)
                    .height(seg_h)
                    .padding(4)
                    .on_press(wrap(PMsg::Profile(id.to_string())))
                    .style(tile_style(on)),
                ),
            );
        }

        let list = d.active_shortcuts();
        if list.is_empty() {
            return column![
                tabs,
                container(
                    text(crate::i18n::tr(
                        "Ajoutez vos raccourcis dans Réglages → Panorama → Raccourcis.",
                    ))
                    .size(14)
                    .color(fg),
                )
                .height(Length::Fill)
                .center_y(Length::Fill)
            ]
            .spacing(gap)
            .padding(8)
            .into();
        }
        // La télécommande garde son pavé de trois colonnes.
        let columns = if d.shortcut_profile == "remote" {
            3
        } else {
            (d.shortcut_columns as usize).max(1)
        };
        let width = ((size.width - 16.0 - gap * (columns - 1) as f32) / columns as f32).max(28.0);
        let rows = list.len().div_ceil(columns);
        let room = size.height - 16.0 - seg_h - gap;
        let height = ((room - gap * (rows - 1) as f32) / rows as f32).clamp(40.0, 120.0);
        let mut grid = column![].spacing(gap);
        for (r, chunk) in list.chunks(columns).enumerate() {
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
                let mut label = column![icon].spacing(5).align_x(Alignment::Center);
                // Touches basses : l'icône seule.
                if height >= 64.0 {
                    label = label.push(
                        text(s.name.clone())
                            .size((width * 0.14).clamp(10.0, 16.0))
                            .color(fg)
                            .wrapping(text::Wrapping::None),
                    );
                }
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
                        .style(tile_style(false)),
                    ),
                );
            }
            grid = grid.push(line);
        }
        column![
            tabs,
            container(iced::widget::scrollable(grid))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_y(Length::Fill)
        ]
        .spacing(gap)
        .padding(8)
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

/// Icônes colorées des profils (vidéo, musique, télécommande, réglages),
/// dessinées ici : un carré arrondi en dégradé et un glyphe blanc.
fn profile_icon(id: &str) -> iced::widget::svg::Handle {
    use std::sync::LazyLock;
    fn tile(from: &str, to: &str, glyph: &str) -> iced::widget::svg::Handle {
        iced::widget::svg::Handle::from_memory(format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{from}"/><stop offset="1" stop-color="{to}"/></linearGradient></defs><rect x="2" y="2" width="44" height="44" rx="12" fill="url(#g)"/>{glyph}</svg>"##
        ).into_bytes())
    }
    static ICONS: LazyLock<[iced::widget::svg::Handle; 4]> = LazyLock::new(|| {
        [
            tile(
                "#ff5f6d",
                "#ffc371",
                r##"<rect x="10" y="12" width="28" height="20" rx="4" fill="none" stroke="#fff" stroke-width="3"/><path d="M21 17.5v9l7.5-4.5z" fill="#fff"/><path d="M18 37h12" stroke="#fff" stroke-width="3" stroke-linecap="round"/>"##,
            ),
            tile(
                "#c471f5",
                "#fa71cd",
                r##"<path d="M19.5 33V15.5l14-3.5v18" fill="none" stroke="#fff" stroke-width="3" stroke-linejoin="round"/><circle cx="16" cy="33" r="4.5" fill="#fff"/><circle cx="30" cy="30" r="4.5" fill="#fff"/>"##,
            ),
            tile(
                "#2193b0",
                "#6dd5ed",
                r##"<circle cx="24" cy="24" r="13" fill="none" stroke="#fff" stroke-width="3"/><circle cx="24" cy="24" r="4" fill="#fff"/><path d="M24 13.5l3 3.5h-6zM24 34.5l3-3.5h-6zM13.5 24l3.5-3v6zM34.5 24l-3.5-3v6z" fill="#fff"/>"##,
            ),
            tile(
                "#11998e",
                "#38ef7d",
                r##"<circle cx="24" cy="24" r="12.5" fill="none" stroke="#fff" stroke-width="5" stroke-dasharray="4.91 4.91"/><circle cx="24" cy="24" r="9.5" fill="#fff"/><circle cx="24" cy="24" r="4" fill="#25c17f"/>"##,
            ),
        ]
    });
    ICONS[match id {
        "video" => 0,
        "audio" => 1,
        "remote" => 2,
        _ => 3,
    }]
    .clone()
}
