//! Halo de LYRA vocale : une lueur sur le pourtour de l'écran pendant la
//! conversation (« Lyra » entendu, réflexion, réponse dite). Surface de
//! calque au-dessus de tout, que la souris et le doigt traversent.

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::ui::Msg;

/// Couleurs du halo : dégradé de LYRA (violet, bleu) ou l'accent du thème.
pub(in crate::ui) fn colors(mode: &str, accent: Color) -> (Color, Color) {
    if mode == "accent" {
        (accent, Color { a: 1.0, ..accent })
    } else {
        (
            Color::from_rgb8(0x7f, 0x5a, 0xf0),
            Color::from_rgb8(0x2c, 0xb1, 0xff),
        )
    }
}

struct Halo {
    from: Color,
    to: Color,
    /// Pulsation, 0 à 1.
    pulse: f32,
    /// Intensité selon l'état (écoute, réflexion, parole).
    strength: f32,
}

impl canvas::Program<Msg> for Halo {
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
        // Couches concentriques, de plus en plus pâles vers l'intérieur.
        let layers = 14;
        let a = self.strength * (0.55 + 0.45 * self.pulse);
        for i in 0..layers {
            let inset = i as f32 * 3.0 + 1.5;
            let fall = 1.0 - i as f32 / layers as f32;
            let alpha = a * fall * fall * 0.55;
            let rect = Path::rounded_rectangle(
                Point::new(inset, inset),
                Size::new((w - 2.0 * inset).max(1.0), (h - 2.0 * inset).max(1.0)),
                (18.0_f32).into(),
            );
            let g = canvas::gradient::Linear::new(Point::new(0.0, 0.0), Point::new(w, h))
                .add_stop(
                    0.0,
                    Color {
                        a: alpha,
                        ..self.from
                    },
                )
                .add_stop(
                    0.5,
                    Color {
                        a: alpha,
                        ..self.to
                    },
                )
                .add_stop(
                    1.0,
                    Color {
                        a: alpha,
                        ..self.from
                    },
                );
            f.stroke(
                &rect,
                Stroke {
                    style: canvas::Style::Gradient(g.into()),
                    width: 3.5,
                    ..Stroke::default()
                },
            );
        }
        vec![f.into_geometry()]
    }
}

/// Vue du halo ; `t` : secondes écoulées (pulsation), `state` : intensité.
pub(in crate::ui) fn view<'a>(from: Color, to: Color, t: f32, strength: f32) -> Element<'a, Msg> {
    let pulse = 0.5 + 0.5 * (t * std::f32::consts::TAU / 1.6).sin();
    canvas::Canvas::new(Halo {
        from,
        to,
        pulse,
        strength,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
