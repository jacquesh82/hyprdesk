//! Disposition des widgets du bureau : chacun à son ancrage (avec ses
//! décalages), mesuré, puis — s'il recouvre un widget déjà placé — posé à la
//! place libre la plus proche. Refaite à chaque mise en page, donc aussi quand
//! l'écran change de taille : deux widgets ne se recouvrent jamais tant que
//! l'écran a la place de les montrer tous.
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
    widget::{Operation, Tree},
};
use iced::{Alignment, Element, Event, Length, Point, Rectangle, Size, Vector};

use crate::ui::Msg;

/// Où se place un widget : alignements et marges (haut, droite, bas, gauche).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    pub x: Alignment,
    pub y: Alignment,
    pub pad: [f32; 4],
    /// Marges de l'écran (haut, droite, bas, gauche) qu'un widget écarté
    /// évite tant qu'il trouve sa place sans elles (dock, bords).
    pub safe: [f32; 4],
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

/// Places des boîtes, dans l'ordre : chacune à son ancrage si la place est
/// libre, sinon à la place libre la plus proche — contre une boîte déjà
/// placée ou contre un bord. Une boîte ancrée en haut ou au centre préfère
/// glisser vers le bas, une boîte ancrée en bas vers le haut. Quand l'écran
/// change de taille, la disposition est refaite ainsi : les widgets restent
/// écartés tant qu'il y a la place. Écran trop petit pour tous : la boîte
/// va là où elle recouvre le moins les autres.
pub fn arrange(items: &[(Spot, Size)], area: Size) -> Vec<Point> {
    let mut placed: Vec<Rectangle> = Vec::new();
    for (spot, size) in items {
        let p = clamp_into(wanted(*spot, *size, area), *size, area);
        let free = |r: &Rectangle| !placed.iter().any(|o| overlaps(*r, *o));
        let here = Rectangle::new(p, *size);
        let r = if free(&here) {
            here
        } else {
            // D'abord dans les marges de l'écran, puis contre ses bords.
            let [top, right, bottom, left] = spot.safe;
            let safe = Rectangle {
                x: left,
                y: top,
                width: area.width - left - right,
                height: area.height - top - bottom,
            };
            let whole = Rectangle::new(Point::ORIGIN, area);
            [safe, whole]
                .into_iter()
                .find_map(|zone| nearest_free(p, *size, zone, spot.y, &placed))
                .unwrap_or_else(|| least_covering(p, *size, whole, &placed))
        };
        placed.push(r);
    }
    placed.iter().map(|r| Point::new(r.x, r.y)).collect()
}

/// Plus de place libre : la position de la zone où la boîte recouvre le
/// moins les autres (à surface égale, la plus proche de `p`).
fn least_covering(p: Point, size: Size, zone: Rectangle, placed: &[Rectangle]) -> Rectangle {
    let (xs, ys) = candidates(p, size, zone, placed);
    let covered = |r: &Rectangle| -> f32 {
        placed
            .iter()
            .filter_map(|o| r.intersection(o))
            .map(|i| i.width * i.height)
            .sum()
    };
    let mut best = (f32::MAX, f32::MAX, Rectangle::new(p, size));
    for &x in &xs {
        for &y in &ys {
            let r = Rectangle::new(clamp_into(Point::new(x, y), size, zone.size()), size);
            let key = (covered(&r), (r.x - p.x).powi(2) + (r.y - p.y).powi(2));
            if key.0 < best.0 || (key.0 == best.0 && key.1 < best.1) {
                best = (key.0, key.1, r);
            }
        }
    }
    best.2
}

/// Positions à essayer : l'ancrage, les bords de la zone, et contre chaque
/// boîte placée (à côté, au-dessus, en dessous, alignée).
fn candidates(p: Point, size: Size, zone: Rectangle, placed: &[Rectangle]) -> (Vec<f32>, Vec<f32>) {
    let (w, h) = (size.width, size.height);
    let mut xs = vec![p.x, zone.x, zone.x + zone.width - w];
    let mut ys = vec![p.y, zone.y, zone.y + zone.height - h];
    for o in placed {
        xs.extend([o.x, o.x + o.width + GAP, o.x - w - GAP, o.x + o.width - w]);
        ys.extend([o.y, o.y + o.height + GAP, o.y - h - GAP, o.y + o.height - h]);
    }
    (xs, ys)
}

/// `p` ramené pour que la boîte tienne dans la zone (au bord si trop grande).
fn clamp_into(p: Point, size: Size, area: Size) -> Point {
    Point::new(
        p.x.clamp(0.0, (area.width - size.width).max(0.0)),
        p.y.clamp(0.0, (area.height - size.height).max(0.0)),
    )
}

/// Place libre de `zone` la plus proche de `p` pour une boîte `size`, parmi
/// les positions contre une boîte placée ou contre un bord de la zone.
fn nearest_free(
    p: Point,
    size: Size,
    zone: Rectangle,
    anchor_y: Alignment,
    placed: &[Rectangle],
) -> Option<Rectangle> {
    let (xs, ys) = candidates(p, size, zone, placed);
    let inside = |r: &Rectangle| {
        r.x >= zone.x - 0.5
            && r.y >= zone.y - 0.5
            && r.x + r.width <= zone.x + zone.width + 0.5
            && r.y + r.height <= zone.y + zone.height + 0.5
    };
    // Les widgets s'empilent : glisser en hauteur, dans le sens de
    // l'ancrage, coûte moins que sur le côté ou à rebours.
    let cost = |r: &Rectangle| {
        let (dx, dy) = (r.x - p.x, r.y - p.y);
        let against = if anchor_y == Alignment::End { dy > 0.0 } else { dy < 0.0 };
        4.0 * dx * dx + dy * dy * if against { 1.5 } else { 1.0 }
    };
    let mut best: Option<(f32, Rectangle)> = None;
    for &x in &xs {
        for &y in &ys {
            let r = Rectangle::new(Point::new(x, y), size);
            if !inside(&r) || placed.iter().any(|o| overlaps(r, *o)) {
                continue;
            }
            let c = cost(&r);
            if best.is_none_or(|(b, _)| c < b) {
                best = Some((c, r));
            }
        }
    }
    best.map(|(_, r)| r)
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
            safe: [32.0, 32.0, 32.0, 32.0],
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

    fn no_overlap(items: &[(Spot, Size)], p: &[Point], area: Size) {
        let rects: Vec<Rectangle> = items
            .iter()
            .zip(p)
            .map(|((_, s), p)| Rectangle::new(*p, *s))
            .collect();
        for (i, a) in rects.iter().enumerate() {
            assert!(
                a.x >= 0.0 && a.y >= 0.0 && a.x + a.width <= area.width + 0.5 && a.y + a.height <= area.height + 0.5,
                "hors de l'écran {area:?} : {a:?}"
            );
            for b in &rects[i + 1..] {
                assert!(!overlaps(*a, *b), "recouvrement dans {area:?} : {a:?} / {b:?}");
            }
        }
    }

    /// L'écran rétrécit (résolution, échelle, rotation) : tant que la place
    /// suffit, aucun widget n'en recouvre un autre, quels que soient leurs
    /// ancrages (MLG-340).
    #[test]
    fn geometrie_changee_sans_recouvrement() {
        use Alignment::{Center, End, Start};
        let items = [
            (spot(End, Start), Size::new(320.0, 180.0)),
            (spot(End, Start), Size::new(320.0, 240.0)),
            (spot(Center, Center), Size::new(400.0, 300.0)),
            (spot(Start, End), Size::new(280.0, 160.0)),
            (spot(End, End), Size::new(360.0, 200.0)),
            (spot(Start, Start), Size::new(300.0, 220.0)),
            (spot(End, Center), Size::new(260.0, 260.0)),
            (spot(Center, Start), Size::new(500.0, 120.0)),
        ];
        // Huit grands widgets sur les grands écrans, six sur les petits.
        for (w, h, n) in [
            (2560.0, 1400.0, 8),
            (1920.0, 1040.0, 8),
            (1707.0, 920.0, 8),
            (1440.0, 860.0, 6),
            (1280.0, 680.0, 6),
            (1080.0, 1880.0, 8),
        ] {
            let area = Size::new(w, h);
            no_overlap(&items[..n], &arrange(&items[..n], area), area);
        }
    }

    /// Une place libre ailleurs : le widget écarté la prend, même quand
    /// glisser dans le sens de l'ancrage le ferait sortir de l'écran.
    #[test]
    fn ecarte_vers_la_place_libre() {
        use Alignment::{End, Start};
        let area = Size::new(1000.0, 500.0);
        let items = [
            (spot(End, Start), Size::new(300.0, 300.0)),
            (spot(End, Start), Size::new(300.0, 300.0)),
            (spot(End, Start), Size::new(300.0, 300.0)),
        ];
        no_overlap(&items, &arrange(&items, area), area);
    }
}
