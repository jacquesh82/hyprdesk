//! Mesure intrinsèque de tous les modules, centrage géométrique et débordement.
use super::Msg;
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
    widget::{Tree, tree},
};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

pub(super) struct Slot<'a> {
    pub zone: usize,
    pub priority: u8,
    pub flexible: bool,
    pub content: Element<'a, Msg>,
}

pub(super) fn strip<'a>(
    id: iced::window::Id,
    slots: Vec<Slot<'a>>,
    more: Element<'a, Msg>,
    compact: bool,
    adaptive_center: bool,
) -> Element<'a, Msg> {
    let zones = slots.iter().map(|s| s.zone).collect();
    let priorities = slots.iter().map(|s| s.priority).collect();
    let flexible = slots.iter().map(|s| s.flexible).collect();
    let mut children: Vec<_> = slots.into_iter().map(|s| s.content).collect();
    children.push(more);
    Element::new(Strip {
        id,
        zones,
        priorities,
        flexible,
        children,
        compact,
        adaptive_center,
    })
}

fn totals(widths: &[f32], zones: &[usize], visible: &[bool], more: f32) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (i, width) in widths.iter().enumerate() {
        if visible[i] && *width > 0.0 {
            out[zones[i]] += width + 2.0;
        }
    }
    if more > 0.0 {
        out[2] += more + 2.0;
    }
    out
}

fn required(t: [f32; 3]) -> f32 {
    if t[1] <= 0.0 {
        t[0] + t[2]
    } else {
        t[1] + 2.0 * t[0].max(t[2])
    }
}

fn fit(
    widths: &[f32],
    zones: &[usize],
    priorities: &[u8],
    width: f32,
    more: f32,
) -> (Vec<bool>, f32) {
    let mut visible = vec![true; widths.len()];
    let full = required(totals(widths, zones, &visible, 0.0));
    let mut candidates: Vec<_> = (0..widths.len()).collect();
    candidates.sort_by_key(|&i| (priorities[i], std::cmp::Reverse(i)));
    let mut overflow = false;
    for i in candidates {
        if required(totals(
            widths,
            zones,
            &visible,
            if overflow { more } else { 0.0 },
        )) <= width
        {
            break;
        }
        if widths[i] > 0.0 {
            visible[i] = false;
            overflow = true;
        }
    }
    (visible, full)
}

#[derive(Default)]
struct State {
    visible: Vec<bool>,
    full: f32,
    reported: Option<(Vec<usize>, u32)>,
}

struct Strip<'a> {
    id: iced::window::Id,
    children: Vec<Element<'a, Msg>>,
    zones: Vec<usize>,
    priorities: Vec<u8>,
    flexible: Vec<bool>,
    compact: bool,
    adaptive_center: bool,
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Strip<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
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
        let size = limits.max();
        let loose = layout::Limits::new(Size::ZERO, Size::new(16384.0, size.height));
        let mut nodes: Vec<_> = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(c, t)| c.as_widget_mut().layout(t, renderer, &loose))
            .collect();
        if self.adaptive_center {
            let sides = nodes
                .iter()
                .zip(&self.zones)
                .fold([0.0_f32; 2], |mut sides, (n, zone)| {
                    if *zone != 1 {
                        sides[usize::from(*zone == 2)] += n.size().width + 2.0;
                    }
                    sides
                });
            let fixed_center: f32 = nodes
                .iter()
                .zip(&self.zones)
                .zip(&self.flexible)
                .filter(|((_, zone), flexible)| **zone == 1 && !**flexible)
                .map(|((node, _), _)| node.size().width + 2.0)
                .sum();
            let budget = (size.width - 2.0 * sides[0].max(sides[1]) - fixed_center)
                .max(48.0)
                .min(size.width);
            for (i, zone) in self.zones.iter().enumerate() {
                if *zone == 1 && self.flexible[i] {
                    nodes[i] = self.children[i].as_widget_mut().layout(
                        &mut tree.children[i],
                        renderer,
                        &layout::Limits::new(Size::ZERO, Size::new(budget, size.height)),
                    );
                }
            }
        }
        let widths: Vec<_> = nodes[..self.zones.len()]
            .iter()
            .map(|n| n.size().width)
            .collect();
        let more = nodes.last().map_or(0.0, |n| n.size().width);
        let (mut visible, full) = fit(&widths, &self.zones, &self.priorities, size.width, more);
        let overflow = visible.contains(&false);
        let totals = totals(
            &widths,
            &self.zones,
            &visible,
            if overflow { more } else { 0.0 },
        );
        let mut x = [0.0, (size.width - totals[1]) / 2.0, size.width - totals[2]];
        visible.push(overflow);
        for (i, node) in nodes.iter_mut().enumerate() {
            if visible[i] {
                let zone = self.zones.get(i).copied().unwrap_or(2);
                let pos = Point::new(x[zone], (size.height - node.size().height) / 2.0);
                x[zone] += node.size().width + 2.0;
                node.move_to_mut(pos);
            } else {
                node.move_to_mut(Point::new(-20000.0, -20000.0));
            }
        }
        let state = tree.state.downcast_mut::<State>();
        state.visible = visible;
        state.full = full;
        layout::Node::with_children(size, nodes)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let visible = tree.state.downcast_ref::<State>().visible.clone();
        for (i, ((c, t), l)) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .enumerate()
        {
            if visible[i] {
                c.as_widget_mut().operate(t, l, renderer, operation);
            }
        }
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
        let state = tree.state.downcast_mut::<State>();
        let hidden: Vec<_> = state.visible[..self.zones.len()]
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (!v).then_some(i))
            .collect();
        let report = (
            hidden,
            if self.compact {
                0
            } else {
                state.full.ceil() as u32
            },
        );
        if state.reported.as_ref() != Some(&report) {
            shell.publish(Msg::BarLayout(self.id, report.0.clone(), report.1));
            state.reported = Some(report);
        }
        let visible = state.visible.clone();
        for (i, ((c, t), l)) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .enumerate()
        {
            if visible[i] {
                c.as_widget_mut()
                    .update(t, event, l, cursor, renderer, clipboard, shell, viewport);
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
        let visible = &tree.state.downcast_ref::<State>().visible;
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
            .filter(|(i, _)| visible[*i])
            .map(|(_, ((c, t), l))| {
                c.as_widget()
                    .mouse_interaction(t, l, cursor, viewport, renderer)
            })
            .max()
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
        let visible = &tree.state.downcast_ref::<State>().visible;
        for (i, ((c, t), l)) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
        {
            if visible[i] {
                c.as_widget()
                    .draw(t, renderer, theme, style, l, cursor, viewport);
            }
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
        let visible = tree.state.downcast_ref::<State>().visible.clone();
        let children: Vec<_> = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .enumerate()
            .filter(|(i, _)| visible[*i])
            .filter_map(|(_, ((c, t), l))| {
                c.as_widget_mut()
                    .overlay(t, l, renderer, viewport, translation)
            })
            .collect();
        (!children.is_empty()).then(|| overlay::Group::with_children(children).overlay())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_clock_and_controls_and_exposes_all_overflow_in_order() {
        let widths = [180.0, 240.0, 80.0, 200.0, 32.0];
        let zones = [0, 0, 1, 2, 2];
        let (visible, full) = fit(&widths, &zones, &[95, 0, 90, 20, 100], 640.0, 32.0);
        assert!(full > 640.0);
        assert!(!visible[1]);
        assert!(visible[0] && visible[2] && visible[4]);
        assert!(required(totals(&widths, &zones, &visible, 32.0)) <= 640.0);
        let hidden: Vec<_> = visible
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (!v).then_some(i))
            .collect();
        assert!(hidden.windows(2).all(|w| w[0] < w[1]));
    }
    #[test]
    fn long_center_content_is_also_reachable_and_empty_modules_do_not_overflow() {
        let (visible, _) = fit(&[0.0, 800.0, 32.0], &[0, 1, 2], &[0, 20, 100], 320.0, 32.0);
        assert_eq!(visible, vec![true, false, true]);
    }
}
