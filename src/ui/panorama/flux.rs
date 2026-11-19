//! Barre « flux vocal » de la webradio : un fond de couleurs qui respire au
//! rythme du son (graves, médiums, aigus) et, au centre, le flux qui défile.
//! Trois ambiances : aurore, coucher de soleil, océan.

use std::collections::VecDeque;

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Font, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use super::super::Msg;

fn palette(id: &str) -> [Color; 3] {
    let rgb = |r: u8, g: u8, b: u8| Color::from_rgb8(r, g, b);
    match id {
        "sunset" => [
            rgb(0xff, 0x7e, 0x5f),
            rgb(0xfe, 0x4a, 0x8b),
            rgb(0x6a, 0x2c, 0x91),
        ],
        "ocean" => [
            rgb(0x00, 0xc6, 0xff),
            rgb(0x00, 0x72, 0xff),
            rgb(0x0b, 0x1f, 0x4d),
        ],
        _ => [
            rgb(0x2a, 0xf5, 0x98),
            rgb(0x00, 0x9e, 0xfd),
            rgb(0x7f, 0x5a, 0xf0),
        ],
    }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

struct Flux {
    colors: [Color; 3],
    levels: [f32; 4],
    history: Vec<f32>,
    title: String,
    subtitle: String,
    font: Font,
    t: f32,
    paused: bool,
    /// Avancée vers le relevé suivant (0 à 1) : défilement continu.
    phase: f32,
}

impl canvas::Program<Msg> for Flux {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let Size {
            width: w,
            height: h,
        } = bounds.size();
        let [all, low, mid, high] = if self.paused { [0.0; 4] } else { self.levels };
        let [c0, c1, c2] = self.colors;
        let dark = Color::from_rgb(0.02, 0.025, 0.04);
        // Fond : deux dégradés qui se croisent et dérivent lentement, sans
        // bord net ; leur éclat suit les graves, médiums et aigus.
        // Le fond tourne lentement (un tour en ~40 s) : il bouge vraiment,
        // sans à-coup ; ses couleurs suivent des niveaux lissés sur ~1 s.
        let ang = self.t * std::f32::consts::TAU / 40.0;
        let (cx, cy) = (w / 2.0, h / 2.0);
        let reach = (w * w + h * h).sqrt() / 2.0;
        let dir = |a: f32| Point::new(cx + reach * a.cos(), cy + reach * a.sin());
        let breathe = (self.t * 0.35).sin() * 0.5 + 0.5;
        let bg = canvas::gradient::Linear::new(dir(ang + std::f32::consts::PI), dir(ang))
            .add_stop(0.0, mix(dark, c2, 0.40 + 0.30 * low))
            .add_stop(0.35 + 0.15 * breathe, mix(dark, c1, 0.35 + 0.35 * mid))
            .add_stop(0.70 + 0.10 * breathe, mix(dark, c0, 0.30 + 0.35 * high))
            .add_stop(1.0, mix(dark, c2, 0.30 + 0.25 * low));
        f.fill_rectangle(Point::ORIGIN, bounds.size(), bg);
        // Voile qui tourne en sens inverse, plus vite : la lumière balaie.
        let ang2 = -self.t * std::f32::consts::TAU / 23.0;
        let veil = canvas::gradient::Linear::new(dir(ang2 + std::f32::consts::PI), dir(ang2))
            .add_stop(0.0, Color { a: 0.0, ..c0 })
            .add_stop(
                0.5,
                Color {
                    a: 0.10 + 0.18 * all,
                    ..c1
                },
            )
            .add_stop(1.0, Color { a: 0.0, ..c2 });
        f.fill_rectangle(Point::ORIGIN, bounds.size(), veil);
        // Barre « flux vocal » : capsule centrale, le son défile en continu
        // de droite à gauche en barres symétriques, lissées.
        let bar_h = (h * 0.34).max(30.0);
        let (bx, by, bw) = (w * 0.06, h * 0.5 - bar_h / 2.0, w * 0.88);
        let capsule = Path::rounded_rectangle(
            Point::new(bx, by),
            Size::new(bw, bar_h),
            (bar_h / 2.0).into(),
        );
        f.fill(&capsule, Color::from_rgba(0.0, 0.0, 0.0, 0.22));
        let n = 72usize;
        let inner = bw - bar_h;
        let step = inner / n as f32;
        let mid_y = h * 0.5;
        // Lissage entre voisines (noyau 1-4-6-4-1).
        let hist = &self.history;
        let at = |k: isize| -> f32 {
            let len = hist.len() as isize;
            let i = (len - 1 - k).clamp(0, (len - 1).max(0));
            hist.get(i as usize).copied().unwrap_or(0.0)
        };
        let smooth = |k: isize| -> f32 {
            (at(k - 2) + 4.0 * at(k - 1) + 6.0 * at(k) + 4.0 * at(k + 1) + at(k + 2)) / 16.0
        };
        let start = bx + bar_h / 2.0;
        for k in 0..=n {
            // k = 0 : barre la plus récente, à droite ; le décalage `phase`
            // fait glisser l'ensemble entre deux relevés.
            let x = start + inner - (k as f32 + self.phase) * step - step / 2.0;
            if x < start {
                continue;
            }
            let v = if self.paused {
                0.04
            } else {
                smooth(k as isize).max(0.04)
            };
            let half = v * (bar_h * 0.42);
            let pos = ((x - start) / inner).clamp(0.0, 1.0);
            // Les barres naissent doucement à droite et s'éteignent à gauche.
            let fade = (pos * 5.0).min((1.0 - pos) * 6.0).clamp(0.0, 1.0);
            let tint = mix(c1, c0, pos);
            f.stroke(
                &Path::line(Point::new(x, mid_y - half), Point::new(x, mid_y + half)),
                Stroke::default()
                    .with_width((step * 0.55).max(1.5))
                    .with_color(Color {
                        a: (0.45 + 0.55 * pos) * fade,
                        ..tint
                    })
                    .with_line_cap(canvas::LineCap::Round),
            );
        }
        // Nom de la station, et « En direct » / « En pause ».
        let size = (h * 0.09).clamp(14.0, 34.0);
        f.fill_text(Text {
            content: self.title.clone(),
            position: Point::new(w / 2.0, by - size * 1.1),
            size: size.into(),
            color: Color::WHITE,
            font: Font {
                weight: iced::font::Weight::Semibold,
                ..self.font
            },
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Text::default()
        });
        f.fill_text(Text {
            content: self.subtitle.clone(),
            position: Point::new(w / 2.0, by + bar_h + size * 0.9),
            size: (size * 0.55).into(),
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.75),
            font: self.font,
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Text::default()
        });
        vec![f.into_geometry()]
    }
}

/// Vue « flux vocal » : ambiance, niveaux, historique (0 à 1), station.
#[allow(clippy::too_many_arguments)]
pub(in crate::ui) fn view<'a>(
    ambiance: &str,
    levels: [f32; 4],
    history: &VecDeque<f32>,
    title: String,
    subtitle: String,
    font: Font,
    t: f32,
    paused: bool,
    phase: f32,
) -> Element<'a, Msg> {
    canvas::Canvas::new(Flux {
        colors: palette(ambiance),
        levels,
        history: history.iter().copied().collect(),
        title,
        subtitle,
        font,
        t,
        paused,
        phase: phase.clamp(0.0, 1.0),
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
