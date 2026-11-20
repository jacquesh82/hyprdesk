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
        // Vidéo logée dans la colonne des raccourcis : le fond de la carte
        // ne doit pas passer sous la fenêtre (le trou resterait couvert).
        let lodged = geo.panes.iter().any(|q| q.bare && q.widget == "video")
            && d.shortcut_profile == "system";
        for p in &geo.panes {
            let body = self.panorama_pane_view(s, geo, p, false, video_at);
            let clear = lodged && p.widget == "shortcuts";
            let content: Element<'a, Msg> = if p.widget == "video" {
                body
            } else {
                container(body)
                    .width(p.rect.w)
                    .height(p.rect.h)
                    .clip(true)
                    .style(move |_| container::Style {
                        background: (!clear).then(|| fx::alpha(bg, d.card_opacity).into()),
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

    /// Barre compacte des profils (icônes), pour les compositions sans bloc
    /// de raccourcis : on change de profil d'un appui, où qu'on soit.
    pub(super) fn panorama_profile_bar(&self) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let mut r = row![].spacing(6);
        for id in crate::panorama::design::PROFILES {
            let on = self.cfg.general.panorama.design.shortcut_profile == *id;
            r = r.push(
                self.panorama_feedback(
                    button(iced::widget::svg(profile_icon(id)).width(26).height(26))
                        .padding(6)
                        .on_press(wrap(PMsg::Profile(id.to_string())))
                        .style(move |_, st| button::Style {
                            background: Some(
                                fx::alpha(
                                    if on || matches!(st, button::Status::Pressed) {
                                        accent
                                    } else {
                                        bg
                                    },
                                    if on { 0.55 } else { 0.7 },
                                )
                                .into(),
                            ),
                            border: Border {
                                radius: 10.0.into(),
                                width: 1.0,
                                color: fx::alpha(fg, 0.2),
                            },
                            ..Default::default()
                        }),
                ),
            );
        }
        r.into()
    }

    /// Gestionnaire des tâches : programmes en cours, triés sur la mesure
    /// de la tuile touchée ; « Terminer » demande un second appui.
    pub(super) fn panorama_tasks_view(&self, s: &Surf) -> Element<'_, Msg> {
        let Some(t) = self.panorama.tasks.as_ref() else {
            return Space::new().into();
        };
        let (bg, fg, accent) = self.panorama_palette();
        let dim = fx::alpha(fg, 0.65);
        let size = (s.h / 30.0).clamp(12.0, 16.0);
        let pill = move |on: bool| {
            move |_: &iced::Theme, st: button::Status| button::Style {
                background: Some(
                    fx::alpha(
                        accent,
                        if on || matches!(st, button::Status::Pressed) {
                            0.6
                        } else {
                            0.18
                        },
                    )
                    .into(),
                ),
                border: Border::default().rounded(10),
                text_color: fg,
                ..Default::default()
            }
        };
        let mut head = row![
            text(crate::i18n::tr("Programmes en cours"))
                .size(size * 1.2)
                .color(fg)
                .width(Length::Fill)
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        for (id, label) in [
            ("cpu", "CPU"),
            ("mem", "Mémoire"),
            ("gpu", "GPU"),
            ("vram", "VRAM"),
            ("io", "Disque"),
        ] {
            head = head.push(
                self.panorama_feedback(
                    button(text(crate::i18n::tr(label)).size(size).color(fg))
                        .padding([5, 12])
                        .on_press(wrap(PMsg::TasksSort(id)))
                        .style(pill(t.sort == id)),
                ),
            );
        }
        head = head.push(
            self.panorama_feedback(
                button(text("✕").size(size).color(fg))
                    .padding([5, 12])
                    .on_press(wrap(PMsg::TasksClose))
                    .style(pill(false)),
            ),
        );
        let gib = |b: u64| {
            let g = b as f64 / (1u64 << 30) as f64;
            if g >= 1.0 {
                format!("{g:.1} Gio")
            } else {
                format!("{:.0} Mio", b as f64 / (1u64 << 20) as f64)
            }
        };
        let col_w = [
            Length::FillPortion(5),
            Length::FillPortion(2),
            Length::FillPortion(2),
            Length::FillPortion(2),
            Length::FillPortion(2),
            Length::FillPortion(2),
        ];
        let cell = |v: String, on: bool, w: Length| {
            text(v)
                .size(size)
                .color(if on { accent } else { fg })
                .width(w)
                .wrapping(text::Wrapping::None)
        };
        // En-tête fixe (hors défilement), sur une bande : colonnes en
        // semi-gras, celle du tri à l'accent et soulignée d'une flèche.
        let bold = iced::Font {
            weight: iced::font::Weight::Semibold,
            ..self.panorama_font()
        };
        let hcell = |label: String, on: bool, w: Length| {
            text(if on { format!("{label} ▾") } else { label })
                .size(size * 0.9)
                .font(bold)
                .color(if on { accent } else { fg })
                .width(w)
                .wrapping(text::Wrapping::None)
        };
        let header = container(
            row![
                hcell(crate::i18n::tr("Programme").into(), false, col_w[0]),
                hcell("CPU".into(), t.sort == "cpu", col_w[1]),
                hcell(crate::i18n::tr("Mémoire").into(), t.sort == "mem", col_w[2]),
                hcell("GPU".into(), t.sort == "gpu", col_w[3]),
                hcell("VRAM".into(), t.sort == "vram", col_w[4]),
                hcell(crate::i18n::tr("Disque").into(), t.sort == "io", col_w[5]),
                Space::new().width(110),
            ]
            .spacing(8),
        )
        .padding([8, 10])
        .style(move |_| container::Style {
            background: Some(fx::alpha(accent, 0.18).into()),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: fx::alpha(accent, 0.45),
            },
            ..Default::default()
        });
        let mut rows = column![].spacing(2);
        if t.list.is_empty() {
            rows = rows.push(text(crate::i18n::tr("Relevé…")).size(size).color(dim));
        }
        for (row_i, p) in t.list.iter().take(40).enumerate() {
            let armed = t.kill == Some(p.pid);
            let kill: Element<'_, Msg> = if p.own {
                self.panorama_feedback(
                    button(
                        text(if armed {
                            crate::i18n::tr("Confirmer ?")
                        } else {
                            crate::i18n::tr("Terminer")
                        })
                        .size(size * 0.85)
                        .color(fg),
                    )
                    .width(110)
                    .padding([3, 8])
                    .on_press(wrap(PMsg::TaskKill(p.pid)))
                    .style(move |_, st| button::Style {
                        background: Some(
                            if armed {
                                Color::from_rgba8(0xe0, 0x4f, 0x5f, 0.85)
                            } else {
                                fx::alpha(
                                    fg,
                                    if matches!(st, button::Status::Pressed) {
                                        0.3
                                    } else {
                                        0.1
                                    },
                                )
                            }
                            .into(),
                        ),
                        border: Border::default().rounded(8),
                        text_color: fg,
                        ..Default::default()
                    }),
                )
            } else {
                Space::new().width(110).into()
            };
            // Lignes alternées : l'œil suit la ligne jusqu'à « Terminer ».
            let odd = row_i % 2 == 1;
            rows = rows.push(
                container(
                    row![
                        cell(format!("{} · {}", p.name, p.pid), false, col_w[0]),
                        cell(format!("{:.1} %", p.cpu), t.sort == "cpu", col_w[1]),
                        cell(gib(p.mem), t.sort == "mem", col_w[2]),
                        cell(
                            p.gpu.map_or("—".into(), |g| format!("{g:.0} %")),
                            t.sort == "gpu",
                            col_w[3]
                        ),
                        cell(
                            p.vram.map_or("—".into(), |v| format!("{v} Mio")),
                            t.sort == "vram",
                            col_w[4]
                        ),
                        cell(
                            if p.io > 0.0 {
                                format!("{}/s", gib(p.io as u64))
                            } else {
                                "—".into()
                            },
                            t.sort == "io",
                            col_w[5]
                        ),
                        kill,
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding([4, 10])
                .style(move |_| container::Style {
                    background: odd.then(|| fx::alpha(fg, 0.05).into()),
                    border: Border::default().rounded(8),
                    ..Default::default()
                }),
            );
        }
        // Un tiers : la mesure (graphique) ; deux tiers : la liste.
        let lines: Vec<crate::widgets::SysLine> = self
            .panorama
            .sys_ids
            .get(&t.metric)
            .cloned()
            .into_iter()
            .collect();
        // Toucher la mesure ramène à la page (comme ✕).
        let indicator = button(super::instruments::metrics(
            "graphs",
            &lines,
            &self.panorama.history,
            self.cfg.general.panorama.design.history_seconds as usize,
            (fg, accent),
            self.panorama_font(),
            self.cfg.general.panorama.design.text_scale,
            self.panorama_graph_phase(),
        ))
        .padding(0)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .on_press(wrap(PMsg::TasksClose))
        .style(move |_, _| button::Style {
            background: Some(fx::alpha(fg, 0.04).into()),
            border: Border::default().rounded(14),
            ..Default::default()
        });
        container(
            row![
                indicator,
                column![
                    head,
                    header,
                    iced::widget::scrollable(rows)
                        .height(Length::Fill)
                        .direction(iced::widget::scrollable::Direction::Vertical(
                            iced::widget::scrollable::Scrollbar::new()
                                .width(4)
                                .scroller_width(4)
                                .spacing(6),
                        ))
                ]
                .spacing(12)
                .width(Length::FillPortion(2)),
            ]
            .spacing(18),
        )
        .padding(18)
        .width(Length::Fill)
        .height(Length::Fill)
        // Fond plein : la page derrière ne transparaît pas.
        .style(move |_| container::Style {
            background: Some(
                Color {
                    r: bg.r * 0.6,
                    g: bg.g * 0.6,
                    b: bg.b * 0.6,
                    a: 1.0,
                }
                .into(),
            ),
            ..Default::default()
        })
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
        // Source en cours : service (navigateur) ou webradio (mpv).
        let playing = self
            .panorama
            .video
            .as_ref()
            .map(|v| v.name.as_str())
            .or(self.panorama.radio.as_ref().map(|r| r.0.as_str()));
        let pill = move |on: bool| {
            move |_: &iced::Theme, st: button::Status| button::Style {
                background: Some(
                    fx::alpha(
                        accent,
                        if on || matches!(st, button::Status::Pressed) {
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
            }
        };
        let audio = d.shortcut_profile == "audio";
        let radio_tab = audio && self.panorama.music_tab == "radio";
        // Musique : onglets Playlist et Webradio ; Vidéo : un titre.
        let mut head = if audio {
            let tab = |label: &'static str, id: &'static str, on: bool| {
                self.panorama_feedback(
                    button(text(crate::i18n::tr(label)).size(13).color(fg))
                        .padding([6, 12])
                        .on_press(wrap(PMsg::MusicTab(id)))
                        .style(pill(on)),
                )
            };
            row![
                tab("Playlist", "playlist", !radio_tab),
                Space::new().width(6),
                tab("Webradio", "radio", radio_tab),
                Space::new().width(Length::Fill),
            ]
            .align_y(Alignment::Center)
        } else {
            row![
                text(crate::i18n::tr("Plateformes"))
                    .size(16.0 * d.text_scale)
                    .font(iced::Font {
                        weight: iced::font::Weight::Semibold,
                        ..self.panorama_font()
                    })
                    .color(fg)
                    .width(Length::Fill)
            ]
            .align_y(Alignment::Center)
        };
        if playing.is_some() {
            head = head.push(
                self.panorama_feedback(
                    button(text(crate::i18n::tr("Fermer")).size(13).color(fg))
                        .padding([6, 12])
                        .on_press(wrap(PMsg::VideoClose))
                        .style(pill(false)),
                ),
            );
        }
        // Cadrage : zoom et décalage de la fenêtre derrière la zone du
        // lecteur, mémorisés pour la source.
        if self.panorama.video.is_some() && self.panorama.framing {
            let side = ((size.width - 16.0 - 2.0 * gap) / 3.0)
                .min((size.height - 16.0 - 34.0 - 4.0 * gap) / 4.0)
                .max(36.0);
            let key = |label: &'static str, icon: &'static str, op: &'static str| {
                let glyph: Element<'_, Msg> = if icon.is_empty() {
                    text(crate::i18n::tr(label)).size(13).color(fg).into()
                } else {
                    self.icon_named(icon, side * 0.4, fg)
                        .unwrap_or_else(|| text(crate::i18n::tr(label)).size(13).color(fg).into())
                };
                self.panorama_feedback(
                    button(
                        container(glyph)
                            .center_x(Length::Fill)
                            .center_y(Length::Fill),
                    )
                    .width(side)
                    .height(side)
                    .on_press(wrap(PMsg::Frame(op)))
                    .style(move |_, st| button::Style {
                        background: Some(
                            fx::alpha(
                                if matches!(st, button::Status::Pressed) {
                                    accent
                                } else {
                                    bg
                                },
                                0.6,
                            )
                            .into(),
                        ),
                        border: Border {
                            radius: 12.0.into(),
                            width: 1.0,
                            color: fx::alpha(fg, 0.25),
                        },
                        text_color: fg,
                        ..Default::default()
                    }),
                )
            };
            let zoom = self
                .panorama_current_source()
                .map(|s| format!("{} %", (s.zoom * 100.0).round()))
                .unwrap_or_default();
            let pad = column![
                row![
                    key("Zoom −", "zoom-out-symbolic", "zoom-"),
                    key("Monter", "go-up-symbolic", "up"),
                    key("Zoom +", "zoom-in-symbolic", "zoom+"),
                ]
                .spacing(gap),
                row![
                    key("Gauche", "go-previous-symbolic", "left"),
                    key("Réinitialiser", "view-refresh-symbolic", "reset"),
                    key("Droite", "go-next-symbolic", "right"),
                ]
                .spacing(gap),
                row![
                    container(text(zoom).size(13).color(fg))
                        .width(side)
                        .height(side)
                        .center_x(side)
                        .center_y(side),
                    key("Descendre", "go-down-symbolic", "down"),
                    key("OK", "object-select-symbolic", "done"),
                ]
                .spacing(gap),
            ]
            .spacing(gap)
            .align_x(Alignment::Center);
            return column![
                container(head).height(34),
                container(pad).width(Length::Fill).center_x(Length::Fill)
            ]
            .spacing(gap)
            .padding(8)
            .into();
        }
        let mut list = column![].spacing(gap / 2.0);
        if radio_tab {
            list = self.panorama_radio_list(list, playing);
        } else {
            let sources = self.panorama_sources();
            // Petites tuiles carrées, autant par rangée que la largeur le permet.
            let aw = (size.width - 16.0).max(1.0);
            let side = (84.0 * d.text_scale).min(aw);
            let cols = ((aw + gap) / (side + gap)).floor().max(1.0) as usize;
            let mut line = row![].spacing(gap);
            for (i, src) in sources.iter().enumerate() {
                let on = playing == Some(src.name.as_str());
                line =
                    line.push(self.panorama_source_tile(src, on, side, wrap(PMsg::VideoOpen(i))));
                if (i + 1) % cols == 0 {
                    list = list.push(line);
                    line = row![].spacing(gap);
                }
            }
            if !sources.len().is_multiple_of(cols) {
                list = list.push(line);
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

    /// Onglet Webradio : le service web choisi (s'il y en a un), les
    /// radios favorites, puis la dernière recherche (LYRA : « mets FIP »).
    fn panorama_radio_list<'a>(
        &'a self,
        mut list: iced::widget::Column<'a, Msg>,
        playing: Option<&str>,
    ) -> iced::widget::Column<'a, Msg> {
        let c = &self.cfg.general.panorama;
        let (_, fg, _) = self.panorama_palette();
        let line_h = 46.0;
        let small = |t: String| text(t).size(12).color(fx::alpha(fg, 0.7));
        if let Some((_, label, Some(url))) = crate::panorama::radio::service(&c.webradio) {
            let src = crate::panorama::config::Source {
                name: label.to_string(),
                url: url.to_string(),
                ..Default::default()
            };
            list = list.push(self.panorama_source_row(
                &src,
                playing == Some(label),
                line_h,
                wrap(PMsg::PlayUrl(label.to_string(), url.to_string())),
                None,
            ));
        }
        // Parcourir l'annuaire au doigt (le Panorama n'a pas de clavier) :
        // les plus écoutées du pays, puis des genres.
        let (_, _, accent) = self.panorama_palette();
        let current = self
            .panorama
            .radio_results
            .as_ref()
            .map(|(q, _)| q.as_str());
        let chip = |label: String, query: String| {
            let on = current == Some(query.as_str());
            self.panorama_feedback(
                button(text(label).size(13).color(fg))
                    .padding([6, 12])
                    .on_press(wrap(PMsg::RadioSearch(query, false)))
                    .style(move |_, st| button::Style {
                        background: Some(
                            fx::alpha(
                                accent,
                                if on || matches!(st, button::Status::Pressed) {
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
            )
        };
        let country = crate::panorama::radio::local_country();
        let mut chips = row![chip(
            crate::i18n::tr("Populaires").to_string(),
            format!("@{country}")
        )]
        .spacing(6);
        for (tag, label) in crate::panorama::radio::GENRES {
            chips = chips.push(chip(crate::i18n::tr(label).to_string(), format!("#{tag}")));
        }
        list = list.push(
            iced::widget::scrollable(chips).direction(
                iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(3)
                        .scroller_width(3)
                        .spacing(4),
                ),
            ),
        );
        list = list.push(small(crate::i18n::tr("Favoris").to_string()));
        for r in c.radios.iter().filter(|r| r.show != "off") {
            list = list.push(self.panorama_source_row(
                r,
                playing == Some(r.name.as_str()),
                line_h,
                wrap(PMsg::PlayUrl(r.name.clone(), r.url.clone())),
                Some((true, wrap(PMsg::RadioFav(r.name.clone(), r.url.clone())))),
            ));
        }
        if self.panorama.radio_busy {
            list = list.push(small(crate::i18n::tr("Recherche…").to_string()));
        }
        if let Some(e) = &self.panorama.radio_error {
            list = list.push(small(e.clone()));
        }
        if let Some((q, found)) = &self.panorama.radio_results {
            list = list.push(small(crate::tr!(
                "Résultats : {q}",
                q = crate::panorama::radio::query_label(q)
            )));
            for r in found {
                let fav = c.radios.iter().any(|f| f.url == r.url);
                list = list.push(self.panorama_source_row(
                    r,
                    playing == Some(r.name.as_str()),
                    line_h,
                    wrap(PMsg::PlayUrl(r.name.clone(), r.url.clone())),
                    Some((fav, wrap(PMsg::RadioFav(r.name.clone(), r.url.clone())))),
                ));
            }
        }
        list.push(small(
            crate::i18n::tr(
                "Cherchez une station par son nom dans Réglages → Panorama → Lecteur, ou demandez à LYRA : « mets FIP », « une radio jazz ».",
            )
            .to_string(),
        ))
    }

    /// Ligne d'une source (service, radio) : icône ou initiale, nom, « En
    /// cours » ; `fav` : étoile (favori ou non) et son message.
    fn panorama_source_row<'a>(
        &'a self,
        src: &crate::panorama::config::Source,
        on: bool,
        line_h: f32,
        press: Msg,
        fav: Option<(bool, Msg)>,
    ) -> Element<'a, Msg> {
        let d = &self.cfg.general.panorama.design;
        let (bg, fg, accent) = self.panorama_palette();
        let radius = d.radius.min(14.0);
        let icon_side = line_h * 0.62;
        let icon: Element<'a, Msg> = (!src.icon.is_empty())
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
            // Radio lancée qui n'a pas encore de son : « Connexion… ».
            let connecting = self.panorama.radio_connecting
                && self
                    .panorama
                    .radio
                    .as_ref()
                    .is_some_and(|r| r.1 == src.url || r.0 == src.name);
            label = label.push(
                text(crate::i18n::tr(if connecting {
                    "Connexion…"
                } else {
                    "En cours"
                }))
                .size(11.0 * d.text_scale)
                .color(fx::alpha(fg, 0.7)),
            );
        }
        let style = move |hot: bool, on: bool| button::Style {
            background: Some(
                fx::alpha(if hot { accent } else { bg }, if hot { 0.45 } else { 0.5 }).into(),
            ),
            border: Border {
                radius: radius.into(),
                width: 1.0,
                color: fx::alpha(if on { accent } else { fg }, if on { 0.8 } else { 0.2 }),
            },
            text_color: fg,
            ..Default::default()
        };
        let main = self.panorama_feedback(
            button(row![icon, label].spacing(12).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(line_h)
                .padding([4, 10])
                .on_press(press)
                .style(move |_, st| style(on || matches!(st, button::Status::Pressed), on)),
        );
        match fav {
            None => main,
            Some((is_fav, msg)) => row![
                main,
                self.panorama_feedback(
                    button(
                        container(text(if is_fav { "★" } else { "☆" }).size(18).color(fg))
                            .center_x(Length::Fill)
                            .center_y(Length::Fill),
                    )
                    .width(line_h)
                    .height(line_h)
                    .on_press(msg)
                    .style(move |_, st| style(matches!(st, button::Status::Pressed), false)),
                )
            ]
            .spacing(6)
            .into(),
        }
    }

    /// Plateforme en petite tuile carrée : son logo au centre, son nom dessous ;
    /// celle qui joue en évidence.
    fn panorama_source_tile<'a>(
        &'a self,
        src: &crate::panorama::config::Source,
        on: bool,
        side: f32,
        press: Msg,
    ) -> Element<'a, Msg> {
        let d = &self.cfg.general.panorama.design;
        let (bg, fg, accent) = self.panorama_palette();
        let radius = d.radius.min(side * 0.18);
        let icon_side = side * 0.5;
        let icon: Element<'a, Msg> = (!src.icon.is_empty())
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
                    .center_x(icon_side)
                    .center_y(icon_side)
                    .style(move |_| container::Style {
                        background: Some(fx::alpha(accent, 0.3).into()),
                        border: Border::default().rounded(icon_side * 0.24),
                        ..Default::default()
                    })
                    .into()
            });
        let name = text(src.name.clone())
            .size((side * 0.13).clamp(10.0, 14.0))
            .color(fg)
            .wrapping(text::Wrapping::None);
        self.panorama_feedback(
            button(
                container(
                    column![icon, name]
                        .spacing(side * 0.06)
                        .align_x(Alignment::Center),
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill),
            )
            .width(side)
            .height(side)
            .padding(side * 0.06)
            .on_press(press)
            .style(move |_, st| {
                let hot = on || matches!(st, button::Status::Pressed);
                button::Style {
                    background: Some(
                        fx::alpha(if hot { accent } else { bg }, if hot { 0.45 } else { 0.5 })
                            .into(),
                    ),
                    border: Border {
                        radius: radius.into(),
                        width: 1.0,
                        color: fx::alpha(if on { accent } else { fg }, if on { 0.8 } else { 0.2 }),
                    },
                    text_color: fg,
                    ..Default::default()
                }
            }),
        )
    }

    /// Bloc de raccourcis : sélecteur de profil (vidéo, audio,
    /// télécommande, système) au-dessus des touches du profil choisi.
    /// Bloc des raccourcis de hauteur `h` : place du sélecteur de profils en
    /// haut (marge comprise) et des points de pages en bas.
    pub(super) fn panorama_shortcuts_frame(&self, h: f32) -> (f32, f32) {
        let gap = self.cfg.general.panorama.design.gap.min(20.0);
        let seg_h = (h * 0.16).clamp(34.0, 52.0);
        (8.0 + seg_h + gap, 30.0 + gap + 8.0)
    }

    pub(super) fn panorama_shortcuts<'a>(
        &'a self,
        size: Size,
        media_h: Option<f32>,
    ) -> Element<'a, Msg> {
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
        // Les profils, puis le micro de LYRA vocale.
        let slots = profiles.len() + 1;
        let seg_w = ((size.width - 16.0 - gap * (slots - 1) as f32) / slots as f32).max(28.0);
        let seg_h = (size.height * 0.16).clamp(34.0, 52.0);
        let named = seg_w >= 110.0;
        // Onglet d'un profil (le profil Système vient après le micro).
        let profile_tab = |id: &'static str| -> Element<'_, Msg> {
            let (label, _) = crate::panorama::design::profile_label(id);
            let on = d.shortcut_profile == id;
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
            // Au survol : ce que fait le bouton, en dessous.
            let tip = container(
                text(crate::i18n::tr(label))
                    .size(13)
                    .color(fg)
                    .wrapping(text::Wrapping::None),
            )
            .padding([4, 10])
            .style(move |_| container::Style {
                background: Some(fx::alpha(bg, 0.92).into()),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: fx::alpha(accent, 0.5),
                },
                ..Default::default()
            });
            Element::from(
                iced::widget::tooltip(
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
                    tip,
                    iced::widget::tooltip::Position::Bottom,
                )
                .gap(4),
            )
        };
        let mut tabs = row![].spacing(gap);
        for id in profiles.iter().filter(|p| **p != "power") {
            tabs = tabs.push(profile_tab(id));
        }
        // Micro de LYRA vocale : coupé (gris), à l'écoute, réveillé (« Lyra »
        // entendu, accent), installation.
        {
            use super::VoiceUi;
            let on = self.cfg.general.panorama.voice;
            let state = &self.panorama.voice;
            let hot = matches!(
                state,
                VoiceUi::Awake | VoiceUi::Thinking | VoiceUi::Speaking
            );
            let label = match state {
                VoiceUi::Installing(_) => crate::i18n::tr("Installation de la voix…"),
                VoiceUi::Awake => crate::i18n::tr("Je vous écoute…"),
                VoiceUi::Thinking => crate::i18n::tr("LYRA réfléchit…"),
                VoiceUi::Speaking => crate::i18n::tr("LYRA parle…"),
                VoiceUi::Error(_) if on => crate::i18n::tr("LYRA vocale : erreur"),
                _ if on => crate::i18n::tr("LYRA vocale : dites « Lyra »"),
                _ => crate::i18n::tr("LYRA vocale (coupée)"),
            };
            // Dernière phrase entendue : de quoi comprendre un mot manqué.
            let label: String = match &self.panorama.voice_heard {
                Some(h) if on && !h.is_empty() => {
                    crate::tr!(
                        "{l} — entendu : « {h} »",
                        l = label.to_string(),
                        h = h.clone()
                    )
                }
                _ => label.to_string(),
            };
            let side = (seg_h * 0.62).min(34.0);
            let icon = iced::widget::svg(mic_icon(on))
                .width(side)
                .height(side)
                .opacity(if on || matches!(state, VoiceUi::Installing(_)) {
                    1.0_f32
                } else {
                    0.45
                });
            let tip = container(
                text(label)
                    .size(13)
                    .color(fg)
                    .wrapping(text::Wrapping::None),
            )
            .padding([4, 10])
            .style(move |_| container::Style {
                background: Some(fx::alpha(bg, 0.92).into()),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: fx::alpha(accent, 0.5),
                },
                ..Default::default()
            });
            tabs = tabs.push(
                iced::widget::tooltip(
                    self.panorama_feedback(
                        button(
                            container(icon)
                                .center_x(Length::Fill)
                                .center_y(Length::Fill),
                        )
                        .width(seg_w)
                        .height(seg_h)
                        .padding(4)
                        .on_press(wrap(PMsg::VoiceToggle))
                        .style(tile_style(hot)),
                    ),
                    tip,
                    iced::widget::tooltip::Position::Bottom,
                )
                .gap(4),
            );
        }
        if profiles.contains(&"power") {
            tabs = tabs.push(profile_tab("power"));
        }

        // Sous le bloc : les touches, ou les fenêtres ouvertes à placer.
        let page = self.panorama.shortcuts_page.min(1);
        let pager = container(Self::panorama_pager(2, page, fg, accent, |i| {
            wrap(PMsg::ShortcutsPage(i))
        }))
        .width(Length::Fill)
        .center_x(Length::Fill);
        // Glisser dans le bloc : des touches aux fenêtres et retour (la page
        // du Panorama, elle, ne bouge pas).
        let motion = self.panorama_motion();
        let paging = move |body: Element<'a, Msg>| -> Element<'a, Msg> {
            super::swipe::swipe(body, super::swipe::Axis::Horizontal, move |d| {
                wrap(PMsg::ShortcutsPage((page as i32 + d).clamp(0, 1) as usize))
            })
            .motion(motion)
            .on_press(wrap(PMsg::Pressed))
            .into()
        };
        // Média en cours logé sous le sélecteur (zone posée par-dessus) : il
        // remplace les touches et les points du bloc.
        if media_h.is_some() && page == 0 {
            return column![tabs, Space::new().width(Length::Fill).height(Length::Fill)]
                .spacing(gap)
                .padding(8)
                .into();
        }
        if page == 1 {
            return column![
                tabs,
                paging(
                    container(self.panorama_windows())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                ),
                pager
            ]
            .spacing(gap)
            .padding(8)
            .into();
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
                .center_y(Length::Fill),
                pager
            ]
            .spacing(gap)
            .padding(8)
            .into();
        }
        // La télécommande garde son pavé de trois colonnes.
        let columns = (d.shortcut_columns as usize).max(1);
        let width = ((size.width - 16.0 - gap * (columns - 1) as f32) / columns as f32).max(28.0);
        let rows = list.len().div_ceil(columns);
        // Place des touches : sous le sélecteur, au-dessus des points (30 px).
        let room = size.height - 16.0 - seg_h - gap - 30.0 - gap;
        let height = ((room - gap * (rows - 1) as f32) / rows as f32).clamp(40.0, 120.0);
        // Touches toujours carrées : le plus petit des deux côtés.
        let side = width.min(height);
        let (width, height) = (side, side);
        let mut grid = column![].spacing(gap);
        for (r, chunk) in list.chunks(columns).enumerate() {
            let mut line = row![].spacing(gap);
            for (j, s) in chunk.iter().enumerate() {
                // Lecture / Pause : la touche suit l'état du lecteur.
                let pause = s.command.trim() == "hyprdesk media play-pause"
                    && (self.panorama.media.as_ref().is_some_and(|m| m.playing)
                        || (self.panorama.radio.is_some() && !self.panorama.radio_paused));
                // Éteindre, redémarrer : armé par un premier appui.
                let armed = self.panorama_armed(r * columns + j);
                let (name, icon_name) = if pause {
                    (
                        crate::i18n::tr("Pause").to_string(),
                        "media-playback-pause-symbolic",
                    )
                } else if armed {
                    (crate::i18n::tr("Confirmer ?").to_string(), s.icon.as_str())
                } else {
                    (s.name.clone(), s.icon.as_str())
                };
                let icon = self
                    .icon_named(icon_name, (height * 0.37).min(width * 0.35), fg)
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
                        text(name)
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
                        .style(tile_style(armed)),
                    ),
                );
            }
            grid = grid.push(line);
        }
        column![
            tabs,
            paging(
                container(iced::widget::scrollable(grid))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            ),
            pager
        ]
        .spacing(gap)
        .padding(8)
        .into()
    }

    /// Fenêtres ouvertes, chacune avec un bouton par écran : un appui la
    /// pose sur le workspace actif de cet écran (sans la suivre). L'écran
    /// où elle se trouve est en évidence ; ceux que le Panorama couvre
    /// n'en reçoivent pas.
    fn panorama_windows(&self) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let dim = fx::alpha(fg, 0.65);
        let covered: Vec<String> = self
            .panorama_monitors()
            .into_iter()
            .filter(|m| !self.panorama.hidden.contains(m))
            .collect();
        let screens: Vec<&str> = self
            .monitors
            .iter()
            .filter(|m| !covered.contains(&m.name))
            .map(|m| m.name.as_str())
            .collect();
        let video = self
            .panorama
            .video
            .as_ref()
            .and_then(|v| v.address.as_deref());
        let wins: Vec<&crate::hypr::Client> = self
            .clients
            .iter()
            .filter(|c| !c.workspace.starts_with("special:") && Some(c.address.as_str()) != video)
            .collect();
        let mut list = column![
            text(crate::i18n::tr("Fenêtres ouvertes"))
                .size(15)
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..self.panorama_font()
                })
                .color(fg)
        ]
        .spacing(6);
        if wins.is_empty() {
            list = list.push(
                text(crate::i18n::tr("Aucune fenêtre ouverte"))
                    .size(13)
                    .color(dim),
            );
        }
        let pill = move |on: bool| {
            move |_: &iced::Theme, st: button::Status| button::Style {
                background: Some(
                    fx::alpha(
                        accent,
                        if on || matches!(st, button::Status::Pressed) {
                            0.6
                        } else {
                            0.18
                        },
                    )
                    .into(),
                ),
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: fx::alpha(accent, if on { 0.9 } else { 0.3 }),
                },
                text_color: fg,
                ..Default::default()
            }
        };
        for c in wins {
            let img = self
                .class_imgs
                .get(&c.class)
                .copied()
                .unwrap_or(super::super::IconImg::None);
            let icon: Element<'_, Msg> = self
                .img_element(img, 26.0, 1.0)
                .unwrap_or_else(|| Space::new().width(26).height(26).into());
            let title = if c.title.is_empty() {
                &c.class
            } else {
                &c.title
            };
            let mut line = row![
                icon,
                container(
                    column![
                        text(title.clone())
                            .size(13)
                            .color(fg)
                            .wrapping(text::Wrapping::None),
                        text(format!("{} · {}", c.class, c.monitor))
                            .size(11)
                            .color(dim)
                            .wrapping(text::Wrapping::None),
                    ]
                    .spacing(1)
                )
                .width(Length::Fill)
                .clip(true),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            for m in &screens {
                let here = c.monitor == *m;
                let b = button(
                    text(m.to_string())
                        .size(12)
                        .color(fg)
                        .wrapping(text::Wrapping::None),
                )
                .padding([6, 10])
                .style(pill(here));
                line = line.push(self.panorama_feedback(if here {
                    b
                } else {
                    b.on_press(wrap(PMsg::SendWindow(c.address.clone(), m.to_string())))
                }));
            }
            list = list.push(container(line).width(Length::Fill).padding([4, 8]).style(
                move |_| container::Style {
                    background: Some(fx::alpha(bg, 0.5).into()),
                    border: Border {
                        radius: 10.0.into(),
                        width: 1.0,
                        color: fx::alpha(fg, 0.15),
                    },
                    ..Default::default()
                },
            ));
        }
        iced::widget::scrollable(list)
            .height(Length::Fill)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .scroller_width(4)
                    .spacing(6),
            ))
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

impl App {
    /// Mises à jour du profil Système : état, avancement pendant la mise à
    /// jour, paquets en attente par source ; Rechercher, Mettre à jour.
    pub(super) fn panorama_updates(&self, size: Size) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let dim = fx::alpha(fg, 0.6);
        let u = self.panorama.updates.as_ref();
        let state = u.map_or("checking", |u| u.state.as_str());
        let total = u.map_or(0, |u| u.sources.iter().map(|s| s.1.len()).sum::<usize>());
        let big = (size.height / 16.0).clamp(15.0, 24.0);
        let small = (big * 0.62).clamp(11.0, 15.0);
        let pill = move |hot: bool| {
            move |_: &iced::Theme, st: button::Status| button::Style {
                background: Some(
                    fx::alpha(
                        if hot { accent } else { bg },
                        if hot || matches!(st, button::Status::Pressed) {
                            0.6
                        } else {
                            0.45
                        },
                    )
                    .into(),
                ),
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: fx::alpha(fg, 0.2),
                },
                text_color: fg,
                ..Default::default()
            }
        };
        let busy = matches!(state, "checking" | "running");
        let action = |label: &str, arg: &'static str, hot: bool, on: bool| {
            let b = button(text(crate::i18n::tr(label).to_string()).size(small + 1.0))
                .padding([8, 16])
                .style(pill(hot));
            self.panorama_feedback(if on {
                b.on_press(wrap(PMsg::UpdatesRun(arg)))
            } else {
                b
            })
        };
        let title: String = match state {
            "checking" => crate::i18n::tr("Recherche des mises à jour…").into(),
            "running" => match u.and_then(|u| u.progress) {
                Some(p) => crate::tr!("Mise à jour en cours : {p} %", p = p),
                None => crate::i18n::tr("Mise à jour en cours…").into(),
            },
            "failed" => crate::i18n::tr("La dernière mise à jour a échoué").into(),
            _ if total == 0 => crate::i18n::tr("Le système est à jour").into(),
            _ => crate::tr!("{n} paquet(s) à mettre à jour", n = total),
        };
        let mut head = column![
            text(title)
                .size(big)
                .color(if state == "failed" { accent } else { fg })
                .wrapping(text::Wrapping::None)
        ]
        .spacing(4);
        // Dernière recherche, ou ce qui se passe pendant la mise à jour.
        let detail = match u {
            Some(u) if state == "running" => {
                let phase = match u.phase.as_str() {
                    "download" => "téléchargement",
                    "check" => "vérifications",
                    "install" => "installation",
                    "hooks" => "finalisation",
                    _ => "préparation",
                };
                let mut d = format!("{} · {}", u.step, crate::i18n::tr(phase));
                if let Some(c) = &u.current {
                    d.push_str(&format!(" : {c}"));
                }
                Some(d)
            }
            Some(u) if u.checked > 0 => {
                chrono::DateTime::from_timestamp(u.checked as i64, 0).map(|t| {
                    crate::tr!(
                        "Dernière recherche : {t}",
                        t = t
                            .with_timezone(&chrono::Local)
                            .format("%d/%m %H:%M")
                            .to_string()
                    )
                })
            }
            _ => None,
        };
        if let Some(d) = detail {
            head = head.push(
                text(d)
                    .size(small)
                    .color(dim)
                    .wrapping(text::Wrapping::None),
            );
        }
        let mut buttons = row![
            action("Rechercher", "--refresh", false, !busy),
            action("Mettre à jour", "--upgrade", true, !busy && total > 0),
        ]
        .spacing(8);
        if state == "failed" {
            buttons = buttons.push(
                self.panorama_feedback(
                    button(text(crate::i18n::tr("Voir le détail").to_string()).size(small + 1.0))
                        .padding([8, 16])
                        .style(pill(false))
                        .on_press(wrap(PMsg::UpdatesRun("--report"))),
                ),
            );
        }
        let top = row![
            container(head).width(Length::Fill),
            buttons.align_y(Alignment::Center)
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let mut col = column![top].spacing(10);
        if state == "running" {
            let f = u.and_then(|u| u.progress).unwrap_or(0) as f32 / 100.0;
            col = col.push(
                iced::widget::progress_bar(0.0..=1.0, f)
                    .girth(6)
                    .style(move |_| iced::widget::progress_bar::Style {
                        background: fx::alpha(fg, 0.15).into(),
                        bar: accent.into(),
                        border: Border::default().rounded(3),
                    }),
            );
        }
        // Paquets par source : nom, puis versions (pacman : « a -> b »).
        let mut list = column![].spacing(3);
        for (source, packages) in u.map(|u| u.sources.as_slice()).unwrap_or_default() {
            if packages.is_empty() {
                continue;
            }
            list = list.push(
                text(format!("{source} — {}", packages.len()))
                    .size(small + 1.0)
                    .color(accent),
            );
            for pkg in packages {
                let (name, versions) = match pkg.split_once(' ') {
                    Some((n, v)) => (n.to_string(), v.replace(" -> ", " → ")),
                    None => (pkg.clone(), String::new()),
                };
                list = list.push(
                    row![
                        text(name)
                            .size(small)
                            .color(fg)
                            .wrapping(text::Wrapping::None)
                            .width(Length::Fill),
                        text(versions)
                            .size(small)
                            .color(dim)
                            .wrapping(text::Wrapping::None),
                    ]
                    .spacing(12),
                );
            }
        }
        col = col.push(
            iced::widget::scrollable(list.padding(iced::Padding::ZERO.right(10)))
                .height(Length::Fill),
        );
        container(col)
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

/// Icônes colorées des profils (vidéo, musique, système, raccourcis),
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
                "#ee0979",
                "#ff6a00",
                r##"<path d="M24 11v12" stroke="#fff" stroke-width="3.5" stroke-linecap="round"/><path d="M16.5 16.2a11 11 0 1 0 15 0" fill="none" stroke="#fff" stroke-width="3.5" stroke-linecap="round"/>"##,
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
        "power" => 2,
        _ => 3,
    }]
    .clone()
}

/// Icône du micro de LYRA vocale : dégradé violet-bleu (active) ou gris.
pub(super) fn mic_icon(on: bool) -> iced::widget::svg::Handle {
    let (a, b) = if on {
        ("#7f5af0", "#2cb1ff")
    } else {
        ("#5b6470", "#3a4048")
    };
    iced::widget::svg::Handle::from_memory(
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{a}"/><stop offset="1" stop-color="{b}"/></linearGradient></defs><rect x="2" y="2" width="44" height="44" rx="12" fill="url(#g)"/><rect x="19" y="10" width="10" height="18" rx="5" fill="#fff"/><path d="M14 23a10 10 0 0 0 20 0" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round"/><path d="M24 33v5M19 38h10" stroke="#fff" stroke-width="3" stroke-linecap="round"/></svg>"##
        )
        .into_bytes(),
    )
}
