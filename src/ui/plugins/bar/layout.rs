//! Mesure intrinsèque de tous les modules, centrage géométrique et débordement.
use crate::ui::Msg;
use crate::ui::kinetic::Spring;
use iced::advanced::{
    Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, overlay, renderer,
    widget::{Tree, tree},
};
use iced::{Border, Color, Element, Event, Length, Point, Rectangle, Size, Vector};
use std::time::{Duration, Instant};

pub(in crate::ui) struct Slot<'a> {
    pub zone: usize,
    pub priority: u8,
    pub flexible: bool,
    pub content: Element<'a, Msg>,
}

/// Capsule Island : couleurs résolues et états qui imposent son expansion.
pub(in crate::ui) struct PillSkin {
    pub fill: Color,
    pub border: Color,
    pub pinned: bool,
    pub notification: bool,
    pub duration: Duration,
}

/// Débord de la pilule autour des modules mesurés, en pixels.
const PILL_INSET: f32 = 12.0;
const IDLE_WIDTH: f32 = 112.0;
const IDLE_HEIGHT: f32 = 28.0;
pub(super) const GROUP_INSET: f32 = 3.0;

pub(in crate::ui) fn strip<'a>(
    id: iced::window::Id,
    slots: Vec<Slot<'a>>,
    more: Element<'a, Msg>,
    compact: bool,
    adaptive_center: bool,
    pill: Option<PillSkin>,
    grouped: bool,
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
        pill,
        grouped,
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
    /// Étendue `(début, fin)` de chaque zone, coordonnées du strip ; `(0, 0)`
    /// : zone vide. Publiée avec le rapport de débordement pour peindre le
    /// fond de zones d'un style derrière les modules.
    extents: [(f32, f32); 3],
    reported: Option<Report>,
    island: IslandState,
}

struct IslandState {
    width: Spring,
    hover: bool,
    input: Option<Rectangle>,
}
impl Default for IslandState {
    fn default() -> Self {
        Self {
            width: Spring::rest(IDLE_WIDTH),
            hover: false,
            input: None,
        }
    }
}

fn capsule(bounds: Rectangle, width: f32, height: f32) -> Rectangle {
    let width = width.min(bounds.width);
    Rectangle::new(
        Point::new(bounds.center_x() - width * 0.5, bounds.y),
        Size::new(width.min(bounds.width), height.min(bounds.height)),
    )
}

impl State {
    fn island_widths(&self, bounds: Rectangle, skin: &PillSkin) -> (f32, f32) {
        let center = self.extents[1];
        let idle = if skin.notification && center.1 > center.0 {
            (center.1 - center.0 + 2.0 * PILL_INSET).max(IDLE_WIDTH)
        } else {
            IDLE_WIDTH
        };
        let expanded = self
            .extents
            .iter()
            .filter(|(a, b)| b > a)
            .map(|(a, b)| {
                ((bounds.width * 0.5 - a)
                    .abs()
                    .max((b - bounds.width * 0.5).abs()))
                    * 2.0
                    + 2.0 * PILL_INSET
            })
            .fold(idle, f32::max)
            .min(bounds.width);
        (idle.min(bounds.width), expanded)
    }
    fn island_child(&self, i: usize, zones: &[usize], skin: &PillSkin, now: Instant) -> bool {
        self.visible[i]
            && ((self.island.hover || skin.pinned) && self.island.width.done(now)
                || skin.notification && zones.get(i) == Some(&1))
    }
}

/// Rapport de mesure du strip : modules cachés, largeur pleine, étendues.
type Report = (Vec<usize>, u32, [(f32, f32); 3]);

struct Strip<'a> {
    id: iced::window::Id,
    children: Vec<Element<'a, Msg>>,
    zones: Vec<usize>,
    priorities: Vec<u8>,
    flexible: Vec<bool>,
    compact: bool,
    adaptive_center: bool,
    /// Peindre et révéler la capsule centrale Island.
    pill: Option<PillSkin>,
    grouped: bool,
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
        let group_inset = if self.grouped { GROUP_INSET } else { 0.0 };
        let budget = if self.pill.is_some() {
            (size.width - 2.0 * PILL_INSET - 16.0).max(0.0)
        } else {
            (size.width - 6.0 * group_inset).max(0.0)
        };
        let (mut visible, full) = fit(&widths, &self.zones, &self.priorities, budget, more);
        let overflow = visible.contains(&false);
        let totals = totals(
            &widths,
            &self.zones,
            &visible,
            if overflow { more } else { 0.0 },
        );
        let mut x = [
            group_inset,
            (size.width - totals[1]) / 2.0,
            size.width - totals[2] - group_inset,
        ];
        if self.pill.is_some() {
            // Les trois zones restent configurables, mais occupent une seule
            // capsule centrale. L'horloge conserve son centrage géométrique.
            x[0] = x[1] - totals[0] - 8.0;
            x[2] = (size.width + totals[1]) / 2.0 + 8.0;
            if totals[1] <= 0.0 {
                x[0] = (size.width - totals[0] - totals[2] - 8.0) / 2.0;
                x[2] = x[0] + totals[0] + 8.0;
            }
        }
        visible.push(overflow);
        let mut extents = [(0.0_f32, 0.0_f32); 3];
        for (i, node) in nodes.iter_mut().enumerate() {
            if visible[i] {
                let zone = self.zones.get(i).copied().unwrap_or(2);
                let w = node.size().width;
                let x0 = x[zone];
                let pos = Point::new(x0, (size.height - node.size().height) / 2.0);
                x[zone] += w + 2.0;
                node.move_to_mut(pos);
                let e = &mut extents[zone];
                if e.0 == 0.0 && e.1 == 0.0 {
                    *e = (x0, x0 + w);
                } else {
                    e.0 = e.0.min(x0);
                    e.1 = e.1.max(x0 + w);
                }
            } else {
                node.move_to_mut(Point::new(-20000.0, -20000.0));
            }
        }
        let state = tree.state.downcast_mut::<State>();
        state.visible = visible;
        state.full = full + 6.0 * group_inset;
        state.extents = extents;
        layout::Node::with_children(size, nodes)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let visible: Vec<_> = state
            .visible
            .iter()
            .enumerate()
            .map(|(i, visible)| {
                *visible
                    && self
                        .pill
                        .as_ref()
                        .is_none_or(|skin| state.island_child(i, &self.zones, skin, now))
            })
            .collect();
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
            state.extents,
        );
        if state.reported.as_ref() != Some(&report) {
            shell.publish(Msg::BarLayout(
                self.id,
                report.0.clone(),
                report.1,
                report.2,
            ));
            state.reported = Some(report);
        }
        let now = Instant::now();
        if let Some(skin) = &self.pill {
            let bounds = layout.bounds();
            let (idle, expanded) = state.island_widths(bounds, skin);
            let current = state.island.width.value(now);
            let hit = capsule(bounds, current, bounds.height);
            let hover =
                !matches!(event, Event::Mouse(mouse::Event::CursorLeft)) && cursor.is_over(hit);
            if state.island.hover != hover {
                shell.request_redraw();
            }
            state.island.hover = hover;
            let target = if hover || skin.pinned { expanded } else { idle };
            if state.island.width.to != target {
                state.island.width = state.island.width.retarget(target, skin.duration, now);
                shell.request_redraw();
            }
            if !state.island.width.done(now) {
                shell.request_redraw();
            }
            // Une région stable pendant le mouvement : aucun message global
            // par image. Les jours transparents rendent les clics au bureau.
            let input_width = if state.island.width.done(now) {
                target
            } else {
                state.island.width.from.max(target)
            };
            let input = capsule(bounds, input_width, bounds.height);
            if state.island.input != Some(input) {
                state.island.input = Some(input);
                shell.publish(Msg::BarIslandInput(self.id, input));
            }
        } else {
            state.island = IslandState::default();
        }
        let visible: Vec<_> = state
            .visible
            .iter()
            .enumerate()
            .map(|(i, visible)| {
                *visible
                    && self
                        .pill
                        .as_ref()
                        .is_none_or(|skin| state.island_child(i, &self.zones, skin, now))
            })
            .collect();
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
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        if self.pill.is_some()
            && cursor.is_over(capsule(
                layout.bounds(),
                state.island.width.value(now),
                layout.bounds().height,
            ))
        {
            return mouse::Interaction::Pointer;
        }
        let visible = &state.visible;
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
            .filter(|(i, _)| {
                visible[*i]
                    && self
                        .pill
                        .as_ref()
                        .is_none_or(|skin| state.island_child(*i, &self.zones, skin, now))
            })
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
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        // Une seule capsule noire, indépendante des modules au repos.
        let mut clip = *viewport;
        if let Some(skin) = &self.pill {
            let b = layout.bounds();
            let width = state.island.width.value(now);
            let (_, expanded) = state.island_widths(b, skin);
            let p = ((width - IDLE_WIDTH) / (expanded - IDLE_WIDTH).max(1.0)).clamp(0.0, 1.0);
            let h = IDLE_HEIGHT + (b.height - 4.0 - IDLE_HEIGHT).max(0.0) * p;
            let bounds = capsule(b, width, h);
            clip = bounds.intersection(viewport).unwrap_or_default();
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border::default()
                        .rounded(h / 2.0)
                        .width(1.0)
                        .color(skin.border),
                    shadow: iced::Shadow {
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.40),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 4.0,
                    },
                    snap: true,
                },
                skin.fill,
            );
            // Verre du thème sur la capsule (MLG-274).
            crate::ui::glass::paint(
                renderer,
                bounds,
                h / 2.0,
                &crate::ui::tokens::glass(crate::config::Place::Bar),
                Color {
                    a: 1.0,
                    ..skin.border
                },
                cursor,
            );
        }
        let visible = &state.visible;
        for (i, ((c, t), l)) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
        {
            let shown = self.pill.as_ref().is_none_or(|skin| {
                (state.island.hover || skin.pinned)
                    || skin.notification && self.zones.get(i) == Some(&1)
            });
            if visible[i] && shown {
                renderer.with_layer(clip, |renderer| {
                    c.as_widget()
                        .draw(t, renderer, theme, style, l, cursor, &clip)
                });
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
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let visible: Vec<_> = state
            .visible
            .iter()
            .enumerate()
            .map(|(i, visible)| {
                *visible
                    && self
                        .pill
                        .as_ref()
                        .is_none_or(|skin| state.island_child(i, &self.zones, skin, now))
            })
            .collect();
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
