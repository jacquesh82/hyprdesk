//! Verre dessiné par hyprdesk au-dessus du fond d'une surface (MLG-268) :
//! filet, reflet, réfraction au bord, reflet qui suit le pointeur.
//!
//! Hyprland ne fait que flouter ce qu'il y a derrière une couche ; la
//! lumière du verre est donc dessinée ici, par la surface elle-même. La
//! réfraction est simulée : une bande au bord où la lumière se concentre
//! et se décompose (liseré froid dehors, chaud dedans), comme le verre
//! courbé d'une lentille — l'image derrière n'est pas déplacée, la surface
//! n'en dispose pas.

use iced::advanced::{
    Layout, Widget, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::{Background, Border, Color, Element, Length, Point, Rectangle, Size};

use crate::config::Place;
use crate::theme_engine::design::Glass;

/// Couche de verre à poser au-dessus d'une surface (même rayon).
pub struct Rim {
    glass: Glass,
    ink: Color,
    radius: f32,
}

/// Pose le verre de `place` au-dessus de `content` : `radius` est celui du
/// fond de la surface, `ink` la couleur du texte du thème.
pub fn over<'a, M: 'a>(
    content: impl Into<Element<'a, M, iced::Theme, iced::Renderer>>,
    place: Place,
    radius: f32,
    ink: Color,
) -> Element<'a, M, iced::Theme, iced::Renderer> {
    let glass = super::tokens::glass(place);
    if !visible(&glass) {
        return content.into();
    }
    let rim: Element<'a, M, iced::Theme, iced::Renderer> = Element::new(Rim { glass, ink, radius });
    iced::widget::stack![content.into(), rim].into()
}

/// Le verre a quelque chose à dessiner.
pub fn visible(g: &Glass) -> bool {
    g.highlight > 0.001 || g.refraction > 0.001 || g.border > 0.001
}

#[derive(Default)]
struct State {
    /// Dernière position du pointeur dessinée (reflet qui le suit).
    last: Option<Point>,
}

fn quad(
    bounds: Rectangle,
    radius: impl Into<iced::border::Radius>,
    border: (f32, Color),
) -> renderer::Quad {
    renderer::Quad {
        bounds,
        border: Border {
            radius: radius.into(),
            width: border.0,
            color: border.1,
        },
        ..Default::default()
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Rim {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, M>,
        _viewport: &Rectangle,
    ) {
        // Reflet qui suit le pointeur : redessiner au mouvement, sans
        // message d'application.
        if !self.glass.pointer_response || self.glass.highlight <= 0.0 {
            return;
        }
        if let iced::Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) =
            event
        {
            let state = tree.state.downcast_mut::<State>();
            let now = cursor.position_over(layout.bounds());
            if now != state.last {
                state.last = now;
                shell.request_redraw();
            }
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let b = layout.bounds();
        if b.width < 2.0 || b.height < 2.0 {
            return;
        }
        let g = &self.glass;
        let r = self.radius.min(b.width / 2.0).min(b.height / 2.0);
        let ink = self.ink;
        let a = |k: f32| Color {
            a: ink.a * k.clamp(0.0, 1.0),
            ..ink
        };
        // La lumière est blanche, en thème clair comme sombre.
        let light = |k: f32| Color::from_rgba(1.0, 1.0, 1.0, k.clamp(0.0, 1.0));
        let clear = Color::TRANSPARENT;

        // Reflet : lumière venue d'en haut, sur la moitié supérieure.
        if g.highlight > 0.0 {
            let top = Rectangle {
                height: b.height * 0.5,
                ..b
            };
            let gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
                .add_stop(0.0, light(g.highlight * 0.55))
                .add_stop(0.35, light(g.highlight * 0.18))
                .add_stop(1.0, clear);
            renderer.fill_quad(
                quad(top, iced::border::Radius::new(0).top(r), (0.0, clear)),
                Background::Gradient(gradient.into()),
            );
            // Lumière qui suit le pointeur : halo doux, en disques
            // concentriques.
            if g.pointer_response
                && let Some(p) = cursor.position_over(b)
            {
                let size = (b.width.min(b.height) * 0.9).clamp(60.0, 320.0);
                for i in 0..6 {
                    let s = size * (1.0 - i as f32 / 7.0);
                    let disc = Rectangle {
                        x: p.x - s / 2.0,
                        y: p.y - s / 2.0,
                        width: s,
                        height: s,
                    };
                    if let Some(clip) = disc.intersection(&b) {
                        renderer.with_layer(clip, |renderer| {
                            renderer.fill_quad(
                                quad(disc, s / 2.0, (0.0, clear)),
                                light(g.highlight * 0.05),
                            );
                        });
                    }
                }
            }
        }

        // Réfraction : bande au bord où la lumière se concentre, puis
        // liserés décomposés (froid dehors, chaud dedans).
        if g.refraction > 0.0 && g.refraction_width > 0.5 {
            let w = g.refraction_width.min(b.width / 3.0).min(b.height / 3.0);
            let steps = 5;
            for i in 0..steps {
                let t = i as f32 / steps as f32;
                let inset = w * t;
                let band = Rectangle {
                    x: b.x + inset,
                    y: b.y + inset,
                    width: b.width - 2.0 * inset,
                    height: b.height - 2.0 * inset,
                };
                renderer.fill_quad(
                    quad(
                        band,
                        (r - inset).max(0.0),
                        (
                            w / steps as f32 + 0.5,
                            light(g.refraction * 0.16 * (1.0 - t) * (1.0 - t)),
                        ),
                    ),
                    clear,
                );
            }
            let cold = Color::from_rgba(0.55, 0.85, 1.0, 0.22 * g.refraction);
            let warm = Color::from_rgba(1.0, 0.78, 0.55, 0.16 * g.refraction);
            renderer.fill_quad(quad(b, r, (1.0, cold)), clear);
            let inner = Rectangle {
                x: b.x + 1.5,
                y: b.y + 1.5,
                width: b.width - 3.0,
                height: b.height - 3.0,
            };
            renderer.fill_quad(quad(inner, (r - 1.5).max(0.0), (1.0, warm)), clear);
        }

        // Filet du verre.
        if g.border > 0.0 {
            renderer.fill_quad(quad(b, r, (1.0, a(g.border))), clear);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verre_vide_sans_couche() {
        let g = Glass {
            highlight: 0.0,
            refraction: 0.0,
            border: 0.0,
            ..Glass::default()
        };
        assert!(!visible(&g));
        assert!(visible(&Glass::default()));
    }
}
