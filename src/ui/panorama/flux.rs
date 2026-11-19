//! Barre « flux vocal » de la webradio : un fond de couleurs qui respire au
//! rythme du son (graves, médiums, aigus) et, au centre, le flux qui défile.
//! Trois ambiances : aurore, coucher de soleil, océan.

use std::collections::VecDeque;

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Font, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use super::super::Msg;

/// Ambiances proposées (Réglages) : (identifiant, libellé).
pub const AMBIANCES: &[(&str, &str)] = &[
    ("aurora", "Aurore"),
    ("sunset", "Coucher de soleil"),
    ("ocean", "Océan"),
];

fn palette(id: &str) -> [Color; 3] {
    let rgb = |r: u8, g: u8, b: u8| Color::from_rgb8(r, g, b);
    match id {
        "sunset" => [rgb(0xff, 0x7e, 0x5f), rgb(0xfe, 0x4a, 0x8b), rgb(0x6a, 0x2c, 0x91)],
        "ocean" => [rgb(0x00, 0xc6, 0xff), rgb(0x00, 0x72, 0xff), rgb(0x0b, 0x1f, 0x4d)],
        _ => [rgb(0x2a, 0xf5, 0x98), rgb(0x00, 0x9e, 0xfd), rgb(0x7f, 0x5a, 0xf0)],
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
        let Size { width: w, height: h } = bounds.size();
        let [all, low, mid, high] = if self.paused { [0.0; 4] } else { self.levels };
        let [c0, c1, c2] = self.colors;
        let dark = Color::from_rgb(0.02, 0.025, 0.04);
        // Fond : dégradé dont l'éclat suit l'énergie, et qui dérive lentement.
        let drift = (self.t * 0.07).sin() * 0.5 + 0.5;
        let bg = canvas::gradient::Linear::new(Point::new(0.0, 0.0), Point::new(w, h))
            .add_stop(0.0, mix(dark, c2, 0.35 + 0.45 * low))
            .add_stop(0.35 + 0.3 * drift, mix(dark, c1, 0.25 + 0.55 * mid))
            .add_stop(1.0, mix(dark, c0, 0.2 + 0.5 * high));
        f.fill_rectangle(Point::ORIGIN, bounds.size(), bg);
        // Halos qui battent sur les graves.
        for (i, c) in self.colors.iter().enumerate() {
            let phase = self.t * (0.11 + i as f32 * 0.05) + i as f32 * 2.1;
            let center = Point::new(
                w * (0.5 + 0.35 * phase.cos()),
                h * (0.5 + 0.3 * (phase * 1.3).sin()),
            );
            let r = h * (0.35 + 0.35 * low + 0.1 * i as f32);
            for k in 0..6 {
                let rr = r * (1.0 - k as f32 * 0.15);
                f.fill(
                    &Path::circle(center, rr.max(1.0)),
                    Color {
                        a: 0.05 + 0.05 * all,
                        ..*c
                    },
                );
            }
        }
        // Barre « flux vocal » : capsule centrale, le son défile de droite à
        // gauche en barres symétriques.
        let bar_h = (h * 0.34).max(30.0);
        let (bx, by, bw) = (w * 0.06, h * 0.5 - bar_h / 2.0, w * 0.88);
        f.fill(
            &Path::rounded_rectangle(Point::new(bx, by), Size::new(bw, bar_h), (bar_h / 2.0).into()),
            Color::from_rgba(0.0, 0.0, 0.0, 0.28),
        );
        let n = 72usize;
        let step = (bw - bar_h) / n as f32;
        let mid_y = h * 0.5;
        for i in 0..n {
            let v = self
                .history
                .get(self.history.len().saturating_sub(n) + i)
                .copied()
                .unwrap_or(0.0);
            let v = if self.paused { 0.04 } else { v.max(0.04) };
            let half = v * (bar_h * 0.42);
            let x = bx + bar_h / 2.0 + i as f32 * step + step / 2.0;
            let tint = mix(c1, c0, i as f32 / n as f32);
            f.stroke(
                &Path::line(Point::new(x, mid_y - half), Point::new(x, mid_y + half)),
                Stroke::default()
                    .with_width((step * 0.55).max(1.5))
                    .with_color(Color {
                        a: 0.55 + 0.45 * (i as f32 / n as f32),
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
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
