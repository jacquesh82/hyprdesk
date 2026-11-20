//! Tuile Hue : les pièces montrées (Réglages → Panorama → Modules), leurs
//! lampes en mini tuiles, curseurs de luminosité, « Tout éteindre ».
use std::time::{Duration, Instant};

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Border, Element, Length, Size, Task};

use super::super::{App, Msg, PMsg, fx, wrap};
use crate::panorama::modules::hue;

/// Relevé des pièces Hue tant que la tuile est à l'écran.
const HUE_EVERY: Duration = Duration::from_secs(10);

impl App {
    /// Une tuile Hue est-elle affichée ?
    /// (Zone d'une composition, ou module de la bande du profil Raccourcis.)
    fn panorama_hue_shown(&self) -> bool {
        let d = &self.cfg.general.panorama.design;
        self.panorama_surfs().iter().any(|s| {
            self.panorama_geo(s).panes.iter().any(|p| {
                p.widget == "hue"
                    || (p.widget == "modules"
                        && d.shortcut_profile == "system"
                        && d.modules().iter().any(|m| m == "hue"))
            })
        })
    }

    /// Relit le pont et ses pièces.
    pub(in crate::ui) fn panorama_hue_refresh(&mut self) -> Task<Msg> {
        self.panorama.hue_at = Some(Instant::now());
        Task::perform(
            async {
                tokio::task::spawn_blocking(|| match hue::load() {
                    Some(b) => (true, hue::rooms(&b)),
                    None => (false, Ok(Vec::new())),
                })
                .await
                .unwrap_or((false, Ok(Vec::new())))
            },
            |(paired, r)| wrap(PMsg::HueRooms(paired, r)),
        )
    }

    /// À chaque battement : relevé quand la tuile est là et qu'il est temps.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_hue_tick(&mut self) -> Task<Msg> {
        if self
            .panorama
            .hue_at
            .is_some_and(|t| t.elapsed() < HUE_EVERY)
            || !self.panorama_hue_shown()
        {
            return Task::none();
        }
        self.panorama_hue_refresh()
    }

    pub(in crate::ui::plugins::panorama::surface) fn panorama_hue_update(
        &mut self,
        m: PMsg,
    ) -> Task<Msg> {
        match m {
            PMsg::HueRooms(paired, r) => {
                self.panorama.hue_paired = paired;
                self.panorama.hue_read = true;
                match r {
                    Ok(list) => {
                        self.panorama.hue = list;
                        self.panorama.hue_error = None;
                    }
                    Err(e) => self.panorama.hue_error = Some(e),
                }
                Task::none()
            }
            PMsg::HueSet(id, on, bri) => {
                self.panorama.last_press = Some(Instant::now());
                // Réponse immédiate à l'écran ; le relevé suit.
                // Groupe 0 : toutes les lampes du pont.
                if id == "0"
                    && let Some(on) = on
                {
                    for r in self.panorama.hue.iter_mut() {
                        r.on = on;
                        for l in r.lights.iter_mut().filter(|l| l.reachable) {
                            l.on = on;
                        }
                    }
                }
                if let Some(r) = self.panorama.hue.iter_mut().find(|r| r.id == id) {
                    if let Some(on) = on {
                        r.on = on;
                        for l in r.lights.iter_mut().filter(|l| l.reachable) {
                            l.on = on;
                        }
                    }
                    if let Some(b) = bri {
                        r.on = true;
                        r.bri = b;
                    }
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let b = hue::load().ok_or_else(|| {
                                crate::i18n::tr("Pont Hue non appairé").to_string()
                            })?;
                            hue::set(&b, &id, on, bri)?;
                            hue::rooms(&b)
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::HueRooms(true, r)),
                )
            }
            PMsg::HueDrag(id, v) => {
                self.panorama.last_press = Some(Instant::now());
                if let Some(r) = self.panorama.hue.iter_mut().find(|r| r.id == id) {
                    r.bri = v;
                }
                self.panorama.hue_drag = Some((false, id, v));
                Task::none()
            }
            PMsg::HueLightDrag(id, v) => {
                self.panorama.last_press = Some(Instant::now());
                self.panorama.hue_drag = Some((true, id, v));
                Task::none()
            }
            PMsg::HueRelease => match self.panorama.hue_drag.take() {
                Some((false, id, v)) => self.panorama_hue_update(PMsg::HueSet(id, None, Some(v))),
                Some((true, id, v)) => self.panorama_hue_update(PMsg::HueLight(id, None, Some(v))),
                None => Task::none(),
            },
            PMsg::HueFocus(id) => {
                self.panorama.hue_focus = id;
                Task::none()
            }
            PMsg::HueLight(id, on, bri) => {
                self.panorama.last_press = Some(Instant::now());
                self.panorama.hue_focus = Some(id.clone());
                for r in self.panorama.hue.iter_mut() {
                    if let Some(l) = r.lights.iter_mut().find(|l| l.id == id) {
                        if let Some(on) = on {
                            l.on = on;
                        }
                        if let Some(b) = bri {
                            l.on = true;
                            l.bri = b;
                        }
                    }
                    r.on = r.lights.iter().any(|l| l.on);
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let b = hue::load().ok_or_else(|| {
                                crate::i18n::tr("Pont Hue non appairé").to_string()
                            })?;
                            hue::set_light(&b, &id, on, bri)?;
                            hue::rooms(&b)
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::HueRooms(true, r)),
                )
            }
            _ => Task::none(),
        }
    }

    /// Tuile Hue : pièce par pièce (celles montrées dans Réglages → Hue),
    /// un en-tête (appui : toute la pièce, − / + : luminosité) puis ses
    /// lampes en pastilles (appui : allumer, éteindre).
    pub(in crate::ui::plugins::panorama::surface) fn panorama_hue(
        &self,
        size: Size,
    ) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let d = &self.cfg.general.panorama.design;
        let dim = fx::alpha(fg, 0.65);
        let t = d.text_scale;
        let title = text(crate::i18n::tr("Lumières"))
            .size(16.0 * t)
            .font(iced::Font {
                weight: iced::font::Weight::Semibold,
                ..self.panorama_font()
            })
            .color(fg);
        let mut col = column![title].spacing(12);
        // Avant le premier relevé : rien d'affirmé sur l'appairage.
        if !self.panorama.hue_read {
            col = col.push(text(crate::i18n::tr("Lecture…")).size(13.0 * t).color(dim));
            return container(col).padding(12).into();
        }
        if !self.panorama.hue_paired {
            col = col.push(
                text(crate::i18n::tr(
                    "Pont Hue non appairé : Réglages → Panorama → Modules, puis le bouton du pont.",
                ))
                .size(13.0 * t)
                .color(dim),
            );
            return container(col).padding(12).into();
        }
        // Largeur utile : marges et barre de défilement (à côté) retirées.
        let avail = (size.width - 24.0 - 14.0).max(90.0);
        // « Tout éteindre » : le groupe 0 du pont (toutes les lampes),
        // en face du titre.
        let any_on = self.panorama.hue.iter().any(|r| r.on);
        let off = button(
            text(crate::i18n::tr("Tout éteindre"))
                .size(13.0 * t)
                .color(fg),
        )
        .padding([6, 14])
        .on_press_maybe(any_on.then(|| wrap(PMsg::HueSet("0".into(), Some(false), None))))
        .style(move |_, st| button::Style {
            background: Some(
                fx::alpha(
                    accent,
                    match st {
                        button::Status::Pressed => 0.6,
                        button::Status::Disabled => 0.06,
                        _ => 0.2,
                    },
                )
                .into(),
            ),
            border: Border::default().rounded(10),
            text_color: fg,
            ..Default::default()
        });
        col = column![
            row![
                text(crate::i18n::tr("Lumières"))
                    .size(16.0 * t)
                    .font(iced::Font {
                        weight: iced::font::Weight::Semibold,
                        ..self.panorama_font()
                    })
                    .color(fg)
                    .width(Length::Fill),
                self.panorama_feedback(off),
            ]
            .align_y(Alignment::Center)
            .width(avail)
        ]
        .spacing(12);
        if let Some(e) = &self.panorama.hue_error {
            col = col.push(
                text(e.clone())
                    .size(12.0 * t)
                    .color(iced::Color::from_rgb8(0xff, 0xb8, 0x6c)),
            );
        }
        let hidden = hue::hidden(&self.cfg);
        let gap = d.gap.min(10.0);
        let mut shown = 0;
        for r in self.panorama.hue.iter().filter(|r| !hidden.contains(&r.id)) {
            shown += 1;
            let lit = r.lights.iter().filter(|l| l.on && l.reachable).count();
            let pct = (r.bri as f32 / 254.0 * 100.0).round() as u32;
            let state = if r.on {
                crate::tr!(
                    "{lit}/{n} allumée(s) · {pct} %",
                    lit = lit,
                    n = r.lights.len(),
                    pct = pct
                )
            } else {
                crate::i18n::tr("Éteinte").to_string()
            };
            let head = row![
                self.panorama_feedback(
                    button(
                        row![
                            container(Space::new().width(10).height(10)).style(move |_| {
                                container::Style {
                                    background: Some(
                                        if r.on { accent } else { fx::alpha(fg, 0.25) }.into(),
                                    ),
                                    border: Border::default().rounded(5),
                                    ..Default::default()
                                }
                            }),
                            column![
                                text(r.name.clone())
                                    .size(15.0 * t)
                                    .font(iced::Font {
                                        weight: iced::font::Weight::Semibold,
                                        ..self.panorama_font()
                                    })
                                    .color(fg)
                                    .wrapping(text::Wrapping::None),
                                text(state).size(12.0 * t).color(dim),
                            ]
                            .spacing(1),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::FillPortion(2))
                    .padding([4, 6])
                    .on_press(wrap(PMsg::HueSet(r.id.clone(), Some(!r.on), None)))
                    .style(|_, _| button::Style::default()),
                ),
                // Luminosité de la pièce : le curseur suit le doigt, le pont
                // ne reçoit la valeur qu'au relâcher.
                {
                    let id = r.id.clone();
                    let value = match &self.panorama.hue_drag {
                        Some((false, d, v)) if *d == r.id => *v,
                        _ if r.on => r.bri,
                        _ => 1,
                    };
                    hue_slider(value, fg, accent, move |v| {
                        wrap(PMsg::HueDrag(id.clone(), v))
                    })
                    .width(Length::FillPortion(3))
                },
            ]
            .spacing(14)
            .align_y(Alignment::Center);
            // Lampes : mini tuiles carrées (ampoule). Survol ou appui : nom
            // et curseur de la lampe sous les tuiles ; appui : allumer,
            // éteindre.
            let side = 56.0;
            let inner = avail - 16.0;
            let cols = ((inner + gap) / (side + gap)).floor().max(1.0) as usize;
            let mut lights = column![].spacing(gap);
            let mut line = row![].spacing(gap);
            let warm = iced::Color::from_rgb8(0xff, 0xd2, 0x6a);
            for (k, l) in r.lights.iter().enumerate() {
                let on = l.on && l.reachable;
                let focused = self.panorama.hue_focus.as_deref() == Some(l.id.as_str());
                let ink = if on {
                    warm
                } else if l.reachable {
                    fx::alpha(fg, 0.5)
                } else {
                    fx::alpha(fg, 0.2)
                };
                let bulb = iced::widget::svg(BULB.clone())
                    .width(26)
                    .height(26)
                    .style(move |_, _| iced::widget::svg::Style { color: Some(ink) });
                let mut b = button(
                    container(bulb)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                )
                .width(side)
                .height(side)
                .padding(0)
                .style(move |_, st| button::Style {
                    background: Some(
                        if on {
                            fx::alpha(accent, 0.35)
                        } else if matches!(st, button::Status::Pressed) {
                            fx::alpha(accent, 0.25)
                        } else {
                            fx::alpha(fg, 0.06)
                        }
                        .into(),
                    ),
                    border: Border {
                        radius: 12.0.into(),
                        width: if focused { 2.0 } else { 1.0 },
                        color: if focused { accent } else { fx::alpha(fg, 0.15) },
                    },
                    text_color: fg,
                    ..Default::default()
                });
                if l.reachable {
                    b = b.on_press(wrap(PMsg::HueLight(l.id.clone(), Some(!l.on), None)));
                }
                line = line.push(
                    iced::widget::mouse_area(self.panorama_feedback(b))
                        .on_enter(wrap(PMsg::HueFocus(Some(l.id.clone())))),
                );
                if (k + 1) % cols == 0 {
                    lights = lights.push(line);
                    line = row![].spacing(gap);
                }
            }
            if !r.lights.len().is_multiple_of(cols) {
                lights = lights.push(line);
            }
            // Lampe désignée : son nom, son état, son curseur.
            if let Some(l) = r
                .lights
                .iter()
                .find(|l| self.panorama.hue_focus.as_deref() == Some(l.id.as_str()))
            {
                let value = match &self.panorama.hue_drag {
                    Some((true, d, v)) if *d == l.id => *v,
                    _ if l.on => l.bri,
                    _ => 1,
                };
                let sub = if !l.reachable {
                    crate::i18n::tr("Injoignable").to_string()
                } else if l.on {
                    format!("{} %", (value as f32 / 254.0 * 100.0).round() as u32)
                } else {
                    crate::i18n::tr("Éteint").to_string()
                };
                let id = l.id.clone();
                let mut detail = row![
                    column![
                        text(l.name.clone())
                            .size(13.0 * t)
                            .color(fg)
                            .wrapping(text::Wrapping::None),
                        text(sub).size(11.0 * t).color(dim),
                    ]
                    .spacing(1)
                    .width(Length::FillPortion(2)),
                ]
                .spacing(14)
                .align_y(Alignment::Center);
                if l.reachable {
                    detail = detail.push(
                        hue_slider(value, fg, accent, move |v| {
                            wrap(PMsg::HueLightDrag(id.clone(), v))
                        })
                        .width(Length::FillPortion(3)),
                    );
                }
                lights = lights.push(container(detail).padding([4, 6]).clip(true));
            }
            col = col.push(
                container(column![head, lights].spacing(8))
                    .width(avail)
                    .padding(8)
                    .style(move |_| container::Style {
                        background: Some(
                            if r.on {
                                fx::alpha(accent, 0.12)
                            } else {
                                fx::alpha(bg, 0.45)
                            }
                            .into(),
                        ),
                        border: Border {
                            radius: 14.0.into(),
                            width: 1.0,
                            color: fx::alpha(
                                if r.on { accent } else { fg },
                                if r.on { 0.6 } else { 0.15 },
                            ),
                        },
                        ..Default::default()
                    }),
            );
        }
        if shown == 0 && self.panorama.hue_error.is_none() {
            col = col.push(
                text(crate::i18n::tr("Aucune pièce montrée : Réglages → Hue."))
                    .size(13.0 * t)
                    .color(dim),
            );
        }
        // Barre de défilement à côté du contenu (pas par-dessus).
        container(
            iced::widget::scrollable(col)
                .height(Length::Fill)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .scroller_width(4)
                        .spacing(8),
                )),
        )
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

/// Ampoule des tuiles de lampe (teintée selon l'état).
static BULB: std::sync::LazyLock<iced::widget::svg::Handle> = std::sync::LazyLock::new(|| {
    iced::widget::svg::Handle::from_memory(
        br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18h6M10 21h4"/><path d="M12 3a6 6 0 0 0-3.5 10.9c.9.7 1.5 1.6 1.5 2.6V17h4v-.5c0-1 .6-1.9 1.5-2.6A6 6 0 0 0 12 3z"/></svg>"##
            .to_vec(),
    )
});

/// Curseur de luminosité (1 à 254) : il suit le doigt, `HueRelease` envoie
/// la valeur au pont au relâcher.
fn hue_slider<'a>(
    value: u8,
    fg: iced::Color,
    accent: iced::Color,
    on_change: impl Fn(u8) -> Msg + 'a,
) -> iced::widget::Slider<'a, u8, Msg> {
    use iced::widget::slider;
    slider(1..=254u8, value, on_change)
        .on_release(wrap(PMsg::HueRelease))
        .height(32)
        .style(move |_, st| {
            let hot = matches!(st, slider::Status::Dragged);
            slider::Style {
                rail: slider::Rail {
                    backgrounds: (accent.into(), fx::alpha(fg, 0.18).into()),
                    width: 6.0,
                    border: Border::default().rounded(3),
                },
                handle: slider::Handle {
                    shape: slider::HandleShape::Circle {
                        radius: if hot { 13.0 } else { 11.0 },
                    },
                    background: fg.into(),
                    border_width: 2.0,
                    border_color: accent,
                },
            }
        })
}
