//! Mouvement retenu par widget : redraw du seul hôte au rythme du compositeur.
//! Aucun message d'application par image ; une notification à l'arrivée.
use super::{
    Fade, Msg,
    plugins::bar::style::convexe::{self, Edge},
};
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
    widget::{Tree, tree},
};
use iced::widget::canvas;
use iced::{Border, Color, Element, Event, Length, Point, Rectangle, Size, Transformation, Vector};
use std::{
    cell::Cell,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

static TRACE: OnceLock<bool> = OnceLock::new();
static APP_FRAMES: AtomicU64 = AtomicU64::new(0);
static APP_CURSORS: AtomicU64 = AtomicU64::new(0);
fn tracing() -> bool {
    *TRACE.get_or_init(|| std::env::var_os("HYPRDESK_MOTION_TRACE").is_some())
}
pub(super) fn note_event(message: &Msg) {
    if tracing() {
        match message {
            Msg::Frame => {
                APP_FRAMES.fetch_add(1, Ordering::Relaxed);
            }
            Msg::Cursor(..) => {
                APP_CURSORS.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }
}
#[derive(Clone, Copy)]
struct Trace {
    key: (Instant, u32),
    frames: u32,
    last: Instant,
    total: f64,
    max: f64,
    app_frames: u64,
    app_cursors: u64,
}
impl Trace {
    fn new(key: (Instant, u32), now: Instant) -> Self {
        Self {
            key,
            frames: 0,
            last: now,
            total: 0.0,
            max: 0.0,
            app_frames: APP_FRAMES.load(Ordering::Relaxed),
            app_cursors: APP_CURSORS.load(Ordering::Relaxed),
        }
    }
    fn report(self, edge: Edge) {
        if self.frames > 1 {
            eprintln!(
                "KINETIC edge={edge:?} frames={} mean_gap_ms={:.2} max_gap_ms={:.2} app_frame_messages={} cursor_messages={}",
                self.frames,
                self.total / (self.frames - 1) as f64,
                self.max,
                APP_FRAMES.load(Ordering::Relaxed) - self.app_frames,
                APP_CURSORS.load(Ordering::Relaxed) - self.app_cursors
            );
        }
    }
}

/// Ressort amorti critique exact : position ET vitesse conservées au reciblage.
/// `response` est le temps pour couvrir environ 95 % du trajet au repos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Spring {
    pub from: f32,
    pub to: f32,
    velocity: f32,
    pub start: Instant,
    pub dur: Duration,
}
impl Spring {
    pub fn rest(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            velocity: 0.0,
            start: Instant::now(),
            dur: Duration::ZERO,
        }
    }
    #[cfg(test)]
    pub fn between(from: f32, to: f32, dur: Duration) -> Self {
        Self {
            from,
            to,
            velocity: 0.0,
            start: Instant::now(),
            dur,
        }
    }
    pub fn sample(&self, now: Instant) -> (f32, f32) {
        if self.dur.is_zero() {
            return (self.to, 0.0);
        }
        let omega = 4.75 / self.dur.as_secs_f32().max(0.001);
        let t = now.saturating_duration_since(self.start).as_secs_f32();
        let a = self.from - self.to;
        let b = self.velocity + omega * a;
        let exp = (-omega * t).exp();
        (self.to + (a + b * t) * exp, (b - omega * (a + b * t)) * exp)
    }
    pub fn value(&self, now: Instant) -> f32 {
        if self.done(now) {
            self.to
        } else {
            self.sample(now).0
        }
    }
    pub fn done(&self, now: Instant) -> bool {
        let (x, v) = self.sample(now);
        (x - self.to).abs() < 0.0008 && v.abs() < 0.015
    }
    pub fn retarget(&self, to: f32, dur: Duration, now: Instant) -> Self {
        let (from, velocity) = self.sample(now);
        Self {
            from,
            to,
            velocity,
            start: now,
            dur,
        }
    }
    pub fn snap(&mut self) {
        self.from = self.to;
        self.velocity = 0.0;
        self.dur = Duration::ZERO;
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) enum Track {
    Spring(Spring),
    Ease(Fade),
}
impl Track {
    fn value(self, now: Instant) -> f32 {
        match self {
            Self::Spring(s) => s.value(now),
            Self::Ease(f) => f.value(now),
        }
    }
    fn done(self, now: Instant) -> bool {
        match self {
            Self::Spring(s) => s.done(now),
            Self::Ease(f) => f.done(now),
        }
    }
    fn target(self) -> f32 {
        match self {
            Self::Spring(s) => s.to,
            Self::Ease(f) => f.to,
        }
    }
    fn needs_commit(self) -> bool {
        match self {
            Self::Spring(s) => s.from != s.to || s.velocity != 0.0 || !s.dur.is_zero(),
            Self::Ease(f) => f.from != f.to || !f.dur.is_zero(),
        }
    }
    fn key(self) -> (Instant, u32) {
        match self {
            Self::Spring(s) => (s.start, s.to.to_bits()),
            Self::Ease(f) => (f.start, f.to.to_bits()),
        }
    }
}

pub(super) fn reveal<'a>(
    content: impl Into<Element<'a, Msg>>,
    track: Track,
    edge: Edge,
    fill: Color,
    settled: Option<Msg>,
) -> Element<'a, Msg> {
    Element::new(Surface {
        content: content.into(),
        kind: Kind::Reveal {
            track,
            edge,
            fill,
            settled: settled.map(Box::new),
        },
    })
}
pub(super) fn magnet<'a>(
    content: impl Into<Element<'a, Msg>>,
    edge: Edge,
    ink: Color,
    duration: Duration,
) -> Element<'a, Msg> {
    Element::new(Surface {
        content: content.into(),
        kind: Kind::Magnet {
            edge,
            ink,
            duration,
            strength: 1.0,
        },
    })
}
pub(super) fn floating<'a>(
    content: impl Into<Element<'a, Msg>>,
    track: Track,
    edge: Edge,
    pivot: Point,
    settled: Option<Msg>,
) -> Element<'a, Msg> {
    Element::new(Surface {
        content: content.into(),
        kind: Kind::Floating {
            track,
            edge,
            pivot,
            settled: settled.map(Box::new),
        },
    })
}
pub(super) fn feedback<'a>(
    content: impl Into<Element<'a, Msg>>,
    ink: Color,
    duration: Duration,
) -> Element<'a, Msg> {
    Element::new(Surface {
        content: content.into(),
        kind: Kind::Magnet {
            edge: Edge::Bottom,
            ink,
            duration,
            strength: 0.24,
        },
    })
}
pub(super) fn probe<'a>(
    content: impl Into<Element<'a, Msg>>,
    id: iced::window::Id,
) -> Element<'a, Msg> {
    Element::new(Surface {
        content: content.into(),
        kind: Kind::Probe(id),
    })
}
struct Surface<'a> {
    content: Element<'a, Msg>,
    kind: Kind,
}
enum Kind {
    Reveal {
        track: Track,
        edge: Edge,
        fill: Color,
        settled: Option<Box<Msg>>,
    },
    Magnet {
        edge: Edge,
        ink: Color,
        duration: Duration,
        strength: f32,
    },
    Floating {
        track: Track,
        edge: Edge,
        pivot: Point,
        settled: Option<Box<Msg>>,
    },
    Probe(iced::window::Id),
}
struct State {
    hover: Spring,
    press: Spring,
    x: Spring,
    y: Spring,
    cache: canvas::Cache,
    paint: Cell<Option<(Color, Edge, u32)>>,
    notified: Option<(Instant, u32)>,
    trace: Cell<Option<Trace>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            hover: Spring::rest(0.0),
            press: Spring::rest(0.0),
            x: Spring::rest(0.0),
            y: Spring::rest(0.0),
            cache: canvas::Cache::default(),
            paint: Cell::new(None),
            notified: None,
            trace: Cell::new(None),
        }
    }
}
fn offset(edge: Edge, b: Rectangle, progress: f32) -> Vector {
    let p = (progress.clamp(0.0, 1.0) - 0.035).max(0.0) / 0.965;
    match edge {
        Edge::Left => Vector::new(-b.width * (1.0 - p), 0.0),
        Edge::Right => Vector::new(b.width * (1.0 - p), 0.0),
        Edge::Top => Vector::new(0.0, -b.height * (1.0 - p)),
        Edge::Bottom => Vector::new(0.0, b.height * (1.0 - p)),
    }
}
fn reveal_pivot(edge: Edge, b: Rectangle) -> Point {
    match edge {
        Edge::Left => Point::new(b.x, b.center_y()),
        Edge::Right => Point::new(b.x + b.width, b.center_y()),
        Edge::Top => Point::new(b.center_x(), b.y),
        Edge::Bottom => Point::new(b.center_x(), b.y + b.height),
    }
}
fn transformed_cursor(
    cursor: mouse::Cursor,
    shift: Vector,
    scale: f32,
    pivot: Point,
) -> mouse::Cursor {
    cursor.position().map_or(mouse::Cursor::Unavailable, |p| {
        mouse::Cursor::Available(Point::new(
            pivot.x + (p.x - pivot.x - shift.x) / scale,
            pivot.y + (p.y - pivot.y - shift.y) / scale,
        ))
    })
}
fn reveal_cursor(cursor: mouse::Cursor, edge: Edge, b: Rectangle, p: f32) -> mouse::Cursor {
    let p = p.clamp(0.0, 1.0);
    transformed_cursor(cursor, offset(edge, b, p), 1.0, reveal_pivot(edge, b))
}
impl Widget<Msg, iced::Theme, iced::Renderer> for Surface<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        op: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, op);
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
        let now = Instant::now();
        let b = layout.bounds();
        let s = tree.state.downcast_mut::<State>();
        let mut child_cursor = cursor;
        match &self.kind {
            Kind::Probe(id) => {
                if matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)))
                    && let Some(p) = cursor.position()
                {
                    shell.publish(Msg::Cursor(*id, p));
                }
            }
            Kind::Reveal {
                track,
                edge,
                settled,
                ..
            } => {
                if !track.done(now) {
                    shell.request_redraw();
                } else if track.needs_commit() && s.notified != Some(track.key()) {
                    s.notified = Some(track.key());
                    if tracing()
                        && let Some(t) = s.trace.take()
                    {
                        t.report(*edge);
                    }
                    if let Some(msg) = settled {
                        shell.publish(msg.as_ref().clone());
                    }
                }
                child_cursor = if track.target() > 0.0 {
                    reveal_cursor(cursor, *edge, b, track.value(now))
                } else {
                    mouse::Cursor::Unavailable
                };
            }
            Kind::Floating { track, settled, .. } => {
                if !track.done(now) {
                    shell.request_redraw();
                } else if track.needs_commit() && s.notified != Some(track.key()) {
                    s.notified = Some(track.key());
                    if let Some(msg) = settled {
                        shell.publish(msg.as_ref().clone());
                    }
                }
                if track.target() == 0.0 {
                    child_cursor = mouse::Cursor::Unavailable;
                }
            }
            Kind::Magnet { duration, .. } => {
                let (hover, x, y) = cursor.position().map_or((0.0, 0.0, 0.0), |p| {
                    let dx = (p.x - b.center_x()) / b.width.max(1.0);
                    let dy = (p.y - b.center_y()) / b.height.max(1.0);
                    let near = (1.0 - (dx * dx + dy * dy) / 4.41).max(0.0);
                    let weight = near * near;
                    (
                        weight,
                        if weight > 0.002 {
                            dx.clamp(-0.7, 0.7)
                        } else {
                            0.0
                        },
                        if weight > 0.002 {
                            dy.clamp(-0.7, 0.7)
                        } else {
                            0.0
                        },
                    )
                });
                if (s.hover.to - hover).abs() > 0.003 {
                    s.hover = s.hover.retarget(hover, *duration, now);
                    shell.request_redraw();
                }
                for (spring, to) in [(&mut s.x, x), (&mut s.y, y)] {
                    if (spring.to - to).abs() > 0.008 {
                        *spring = spring.retarget(to, *duration, now);
                    }
                }
                let press = match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                        if cursor.is_over(b) =>
                    {
                        Some(1.0)
                    }
                    Event::Mouse(
                        mouse::Event::ButtonReleased(mouse::Button::Left)
                        | mouse::Event::CursorLeft,
                    ) => Some(0.0),
                    _ if !cursor.is_over(b) => Some(0.0),
                    _ => None,
                };
                if let Some(to) = press
                    && to != s.press.to
                {
                    s.press = s.press.retarget(
                        to,
                        if duration.is_zero() {
                            Duration::ZERO
                        } else {
                            Duration::from_millis(if to > 0.0 { 75 } else { 130 })
                        },
                        now,
                    );
                    shell.request_redraw();
                }
                if [&s.hover, &s.press, &s.x, &s.y]
                    .iter()
                    .any(|spring| !spring.done(now))
                {
                    shell.request_redraw();
                }
            }
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            child_cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let cursor = match &self.kind {
            Kind::Reveal { track, edge, .. } => {
                if track.target() > 0.0 {
                    reveal_cursor(cursor, *edge, layout.bounds(), track.value(Instant::now()))
                } else {
                    mouse::Cursor::Unavailable
                }
            }
            _ => cursor,
        };
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
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
        let s = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let b = layout.bounds();
        let (mut scale, mut shift, mut pivot) = (1.0, Vector::ZERO, b.center());
        match &self.kind {
            Kind::Reveal {
                track, edge, fill, ..
            } => {
                if tracing() && !track.done(now) {
                    let mut t = s
                        .trace
                        .get()
                        .filter(|t| t.key == track.key())
                        .unwrap_or_else(|| Trace::new(track.key(), now));
                    if t.frames > 0 {
                        let gap = now.saturating_duration_since(t.last).as_secs_f64() * 1000.0;
                        t.total += gap;
                        t.max = t.max.max(gap);
                    }
                    t.last = now;
                    t.frames += 1;
                    s.trace.set(Some(t));
                }
                let p = track.value(now).clamp(0.0, 1.0);
                let key = (*fill, *edge, p.to_bits());
                if s.paint.get() != Some(key) {
                    s.cache.clear();
                    s.paint.set(Some(key));
                }
                let geometry = s.cache.draw(renderer, b.size(), |frame| {
                    frame.fill(&convexe::drawer_path(b.size(), *edge, p, 24.0), *fill);
                });
                renderer.with_translation(Vector::new(b.x, b.y), |r| r.draw_geometry(geometry));
                shift = offset(*edge, b, p);
                // Taille de texte constante : réutilise l'atlas des glyphes.
                scale = 1.0;
                pivot = match edge {
                    Edge::Left => Point::new(b.x, b.center_y()),
                    Edge::Right => Point::new(b.x + b.width, b.center_y()),
                    Edge::Top => Point::new(b.center_x(), b.y),
                    Edge::Bottom => Point::new(b.center_x(), b.y + b.height),
                };
            }
            Kind::Floating {
                track,
                edge,
                pivot: origin,
                ..
            } => {
                let p = track.value(now).clamp(0.0, 1.0);
                if p <= 0.0001 {
                    return;
                }
                scale = 1.0;
                shift = match edge {
                    Edge::Top => Vector::new(0.0, -6.0 * (1.0 - p)),
                    Edge::Bottom => Vector::new(0.0, 6.0 * (1.0 - p)),
                    Edge::Left => Vector::new(-6.0 * (1.0 - p), 0.0),
                    Edge::Right => Vector::new(6.0 * (1.0 - p), 0.0),
                };
                pivot = Point::new(b.x + b.width * origin.x, b.y + b.height * origin.y);
            }
            Kind::Magnet {
                edge,
                ink,
                strength,
                ..
            } => {
                let over = s.hover.value(now).clamp(0.0, 1.0);
                let press = s.press.value(now).clamp(0.0, 1.0);
                // Les SVG sont rasterisés par taille par iced : une échelle
                // variable repeuplerait le cache à chaque image. Le relief et
                // la lumière portent le geste, le raster reste réutilisable.
                scale = 1.0;
                let lift = if *strength < 0.5 {
                    over - 1.5 * press
                } else {
                    (4.0 * over - press) * strength
                };
                shift = match edge {
                    Edge::Bottom => Vector::new(s.x.value(now) * 2.0 * over * strength, -lift),
                    Edge::Top => Vector::new(s.x.value(now) * 2.0 * over * strength, lift),
                    Edge::Left => Vector::new(lift, s.y.value(now) * 2.0 * over * strength),
                    Edge::Right => Vector::new(-lift, s.y.value(now) * 2.0 * over * strength),
                };
                if over > 0.001 {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: b.expand(3.0 + (3.0 * over - press) * strength),
                            border: Border::default().rounded(12),
                            ..Default::default()
                        },
                        Color {
                            a: ink.a * over * 0.08 * strength,
                            ..*ink
                        },
                    );
                }
            }
            Kind::Probe(_) => {}
        }
        let transform = Transformation::translate(pivot.x + shift.x, pivot.y + shift.y)
            * Transformation::scale(scale)
            * Transformation::translate(-pivot.x, -pivot.y);
        // Le clip reste au contour du tiroir ; le pointeur suit son contenu.
        let clip = match &self.kind {
            Kind::Reveal { .. } => b.intersection(viewport).unwrap_or_default(),
            Kind::Floating { track, edge, .. } => {
                let p = track.value(now).clamp(0.0, 1.0);
                let mut clip = b;
                match edge {
                    Edge::Top => clip.height *= p,
                    Edge::Bottom => {
                        clip.y += b.height * (1.0 - p);
                        clip.height *= p;
                    }
                    Edge::Left => clip.width *= p,
                    Edge::Right => {
                        clip.x += b.width * (1.0 - p);
                        clip.width *= p;
                    }
                }
                clip.intersection(viewport).unwrap_or_default()
            }
            _ => *viewport,
        };
        renderer.with_layer(clip, |r| {
            r.with_transformation(transform, |r| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    r,
                    theme,
                    style,
                    layout,
                    transformed_cursor(cursor, shift, scale, pivot),
                    &(*viewport * transform.inverse()),
                )
            })
        });
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Msg, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
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
    #[test]
    fn reversal_preserves_position_and_velocity() {
        let s = Spring::between(0.0, 1.0, Duration::from_millis(180));
        let at = s.start + Duration::from_millis(65);
        let before = s.sample(at);
        let reverse = s.retarget(0.0, Duration::from_millis(140), at);
        let after = reverse.sample(at);
        assert!((before.0 - after.0).abs() < 1e-6);
        assert!((before.1 - after.1).abs() < 1e-5);
        assert!(reverse.done(at + Duration::from_secs(2)));
    }
    #[test]
    fn sampling_is_independent_of_frame_rate_and_has_a_finite_stop() {
        for hz in [30, 60, 120, 144, 240] {
            let s = Spring::between(0.0, 1.0, Duration::from_millis(180));
            for n in 0..hz {
                let (x, v) = s.sample(s.start + Duration::from_secs_f32(n as f32 / hz as f32));
                assert!(x.is_finite() && v.is_finite());
                assert!((0.0..=1.001).contains(&x));
            }
            assert!(s.done(s.start + Duration::from_secs(1)));
        }
    }
    #[test]
    fn idle_tracks_do_not_request_global_completion_messages() {
        let now = Instant::now();
        assert!(!Track::Spring(Spring::rest(0.0)).needs_commit());
        assert!(
            !Track::Ease(Fade {
                from: 1.0,
                to: 1.0,
                start: now,
                dur: Duration::ZERO
            })
            .needs_commit()
        );
        assert!(Track::Spring(Spring::between(0.0, 1.0, Duration::ZERO)).needs_commit());
    }
    #[test]
    fn pointer_mapping_matches_the_drawn_content_during_reveal() {
        let b = Rectangle::new(Point::new(200.0, 80.0), Size::new(380.0, 540.0));
        let original = Point::new(270.0, 230.0);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            let p = 0.65;
            let scale = 1.0;
            let pivot = reveal_pivot(edge, b);
            let shift = offset(edge, b, p);
            let drawn = Point::new(
                pivot.x + (original.x - pivot.x) * scale + shift.x,
                pivot.y + (original.y - pivot.y) * scale + shift.y,
            );
            let result = reveal_cursor(mouse::Cursor::Available(drawn), edge, b, p)
                .position()
                .unwrap();
            assert!(result.distance(original) < 0.001);
        }
    }
    #[test]
    fn reduced_motion_is_immediate() {
        let s = Spring::between(0.0, 1.0, Duration::ZERO);
        assert_eq!(s.value(s.start), 1.0);
        assert!(s.done(s.start));
    }
}
