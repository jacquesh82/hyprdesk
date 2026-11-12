//! Disposition des widgets du bureau : chacun à son ancrage (avec ses
//! décalages), mesuré, puis écarté de ceux déjà placés s'il les recouvre.
//! Un widget ancré en haut ou au centre glisse vers le bas, un widget ancré en
//! bas glisse vers le haut ; s'il sort de l'écran, il glisse sur le côté.
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
    widget::{Operation, Tree},
};
use iced::{Alignment, Element, Event, Length, Point, Rectangle, Size, Vector};

use super::Msg;

/// Où se place un widget : alignements et marges (haut, droite, bas, gauche).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    pub x: Alignment,
    pub y: Alignment,
    pub pad: [f32; 4],
}

/// Écart minimal entre deux widgets.
const GAP: f32 = 16.0;

/// Position voulue d'une boîte de taille `size` dans une zone `area`.
fn wanted(spot: Spot, size: Size, area: Size) -> Point {
    let [top, right, bottom, left] = spot.pad;
    let axis = |a: Alignment, len: f32, total: f32, start: f32, end: f32| match a {
        Alignment::Start => start,
        Alignment::End => total - end - len,
        Alignment::Center => (total - len) / 2.0 + (start - end) / 2.0,
    };
    Point::new(
        axis(spot.x, size.width, area.width, left, right),
        axis(spot.y, size.height, area.height, top, bottom),
    )
}

fn overlaps(a: Rectangle, b: Rectangle) -> bool {
    a.x < b.x + b.width + GAP
        && b.x < a.x + a.width + GAP
        && a.y < b.y + b.height + GAP
        && b.y < a.y + a.height + GAP
}

/// Places des boîtes, dans l'ordre : chacune à son ancrage, écartée de celles
/// déjà placées si elle les recouvre.
pub fn arrange(items: &[(Spot, Size)], area: Size) -> Vec<Point> {
    let mut placed: Vec<Rectangle> = Vec::new();
    for (spot, size) in items {
        let p = wanted(*spot, *size, area);
        let mut r = Rectangle::new(p, *size);
        let fits = |r: &Rectangle| {
            r.x >= 0.0 && r.y >= 0.0 && r.x + r.width <= area.width && r.y + r.height <= area.height
        };
        let mut sideways = false;
        // Chaque tour écarte d'une boîte : au plus une fois chacune, deux sens.
        for _ in 0..(2 * placed.len() + 2) {
            let Some(o) = placed.iter().find(|o| overlaps(r, **o)).copied() else {
                break;
            };
            let mut next = r;
            if !sideways {
                next.y = if spot.y == Alignment::End {
                    o.y - r.height - GAP
                } else {
                    o.y + o.height + GAP
                };
                if !fits(&next) {
                    // Plus de place en hauteur : on repart de l'ancrage, sur le côté.
                    sideways = true;
                    next = Rectangle::new(p, *size);
                    next.x = if spot.x == Alignment::End {
                        o.x - r.width - GAP
                    } else {
                        o.x + o.width + GAP
                    };
                }
            } else {
                next.x = if spot.x == Alignment::End {
                    o.x - r.width - GAP
                } else {
                    o.x + o.width + GAP
                };
            }
            r = next;
        }
        // Jamais hors de la zone (un widget trop grand reste au bord).
        r.x = r.x.clamp(0.0, (area.width - r.width).max(0.0));
        r.y = r.y.clamp(0.0, (area.height - r.height).max(0.0));
        placed.push(r);
    }
    placed.iter().map(|r| Point::new(r.x, r.y)).collect()
}

/// Les widgets d'un bureau, disposés sans se recouvrir.
pub fn desk<'a>(items: Vec<(Spot, Element<'a, Msg>)>) -> Element<'a, Msg> {
    let (spots, children) = items.into_iter().unzip();
    Element::new(Desk { spots, children })
}

struct Desk<'a> {
    spots: Vec<Spot>,
    children: Vec<Element<'a, Msg>>,
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Desk<'_> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let area = limits.width(Length::Fill).height(Length::Fill).max();
        let loose = layout::Limits::new(Size::ZERO, area);
        let mut nodes: Vec<layout::Node> = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(c, t)| c.as_widget_mut().layout(t, renderer, &loose))
            .collect();
        let items: Vec<(Spot, Size)> = self
            .spots
            .iter()
            .zip(&nodes)
            .map(|(s, n)| (*s, n.size()))
            .collect();
        for (n, p) in nodes.iter_mut().zip(arrange(&items, area)) {
            n.move_to_mut(p);
        }
        layout::Node::with_children(area, nodes)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((c, t), l) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                c.as_widget_mut().operate(t, l, renderer, operation);
            }
        });
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Msg>,
        viewport: &Rectangle,
    ) {
        for ((c, t), l) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .rev()
        {
            c.as_widget_mut()
                .update(t, event, l, cursor, renderer, clipboard, shell, viewport);
            if shell.is_event_captured() {
                return;
            }
        }
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .rev()
            .map(|((c, t), l)| {
                c.as_widget()
                    .mouse_interaction(t, l, cursor, viewport, renderer)
            })
            .find(|i| *i != mouse::Interaction::None)
            .unwrap_or_default()
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for ((c, t), l) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            c.as_widget()
                .draw(t, renderer, theme, style, l, cursor, viewport);
        }
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Msg, iced::Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Size = Size {
        width: 1000.0,
        height: 800.0,
    };

    fn spot(x: Alignment, y: Alignment) -> Spot {
        Spot {
            x,
            y,
            pad: [32.0, 32.0, 32.0, 32.0],
        }
    }

    #[test]
    fn ancrages() {
        let s = Size::new(200.0, 100.0);
        let p = arrange(
            &[
                (spot(Alignment::End, Alignment::Start), s),
                (spot(Alignment::Center, Alignment::Center), s),
                (spot(Alignment::Start, Alignment::End), s),
            ],
            AREA,
        );
        assert_eq!(p[0], Point::new(768.0, 32.0));
        assert_eq!(p[1], Point::new(400.0, 350.0));
        assert_eq!(p[2], Point::new(32.0, 668.0));
    }

    /// Même ancrage : empilés, sans se recouvrir ; en bas, vers le haut.
    #[test]
    fn sans_superposition() {
        let s = Size::new(200.0, 100.0);
        let top = spot(Alignment::End, Alignment::Start);
        let p = arrange(&[(top, s), (top, s), (top, s)], AREA);
        assert_eq!(p[0].y, 32.0);
        assert_eq!(p[1].y, 32.0 + 100.0 + GAP);
        assert_eq!(p[2].y, 32.0 + 2.0 * (100.0 + GAP));
        let bottom = spot(Alignment::Start, Alignment::End);
        let q = arrange(&[(bottom, s), (bottom, s)], AREA);
        assert_eq!(q[1].y, q[0].y - 100.0 - GAP);
        // Colonne pleine : le suivant passe sur le côté.
        let tall = Size::new(200.0, 500.0);
        let r = arrange(&[(top, tall), (top, tall)], AREA);
        assert_eq!(r[1].x, r[0].x - 200.0 - GAP);
        for set in [&p[..], &q[..], &r[..]] {
            for (i, a) in set.iter().enumerate() {
                for b in &set[i + 1..] {
                    assert!(a != b);
                }
            }
        }
    }
}
