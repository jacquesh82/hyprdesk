//! Glisser d'une page à l'autre, au doigt ou à la souris (repli pour essayer
//! sans écran tactile). Le contenu suit le doigt ; au relâcher, un geste
//! assez long ou assez vif publie `on_swipe(+1)` (page suivante) ou `-1`.
//! Tant que le geste n'a pas dépassé le seuil, les boutons de la page
//! reçoivent l'appui comme d'habitude ; au-delà, ils ne le reçoivent plus.
//!
//! Un geste sur l'autre axe (de bas en haut ou de haut en bas pour un
//! bandeau paysage) publie `on_cross(+1)` (vers le haut) ou `-1`.

use std::time::{Duration, Instant};

use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, touch};

use super::super::Msg;
use super::super::kinetic::Spring;

/// Distance avant de reconnaître un glisser (et non un appui).
const SLOP: f32 = 28.0;
/// Part de la page à parcourir pour changer de page sans élan.
const PAGE_PART: f32 = 0.18;
/// Vitesse (px/ms) qui change de page même sur un geste court.
const FLICK: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

pub struct Swipe<'a> {
    content: Element<'a, Msg>,
    axis: Axis,
    on_swipe: Box<dyn Fn(i32) -> Msg + 'a>,
    /// Geste sur l'autre axe : +1 vers le haut (ou la gauche), -1 vers le bas.
    on_cross: Option<Box<dyn Fn(i32) -> Msg + 'a>>,
    /// Publié à chaque appui (activité : retarde la veille).
    on_press: Option<Msg>,
    duration: Duration,
}

pub fn swipe<'a>(
    content: impl Into<Element<'a, Msg>>,
    axis: Axis,
    on_swipe: impl Fn(i32) -> Msg + 'a,
) -> Swipe<'a> {
    Swipe {
        content: content.into(),
        axis,
        on_swipe: Box::new(on_swipe),
        on_cross: None,
        on_press: None,
        duration: Duration::from_millis(240),
    }
}

impl<'a> Swipe<'a> {
    pub fn motion(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
    pub fn on_cross(mut self, f: impl Fn(i32) -> Msg + 'a) -> Self {
        self.on_cross = Some(Box::new(f));
        self
    }
    pub fn on_press(mut self, m: Msg) -> Self {
        self.on_press = Some(m);
        self
    }
}

#[derive(Debug, Default)]
struct State {
    start: Option<Point>,
    /// Doigt suivi (aucun : la souris).
    finger: Option<touch::Finger>,
    dragging: bool,
    /// Geste reconnu sur l'autre axe (le contenu ne suit pas).
    crossing: bool,
    /// Déplacement sur l'autre axe.
    cross: f32,
    offset: f32,
    last: Option<(f32, Instant)>,
    /// px/ms, sur l'axe.
    velocity: f32,
    settle: Option<Spring>,
}

impl State {
    fn reset(&mut self) {
        *self = State::default();
    }
}

/// Sens du changement de page au relâcher (0 : on reste).
pub fn decide(offset: f32, velocity: f32, extent: f32) -> i32 {
    if offset <= -extent * PAGE_PART || velocity <= -FLICK {
        1
    } else if offset >= extent * PAGE_PART || velocity >= FLICK {
        -1
    } else {
        0
    }
}

impl Swipe<'_> {
    fn along(&self, v: Vector) -> (f32, f32) {
        match self.axis {
            Axis::Horizontal => (v.x, v.y),
            Axis::Vertical => (v.y, v.x),
        }
    }
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Swipe<'_> {
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
        Size::new(Length::Fill, Length::Fill)
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Fill);
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        layout::Node::with_children(limits.max(), vec![child])
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        op: &mut dyn iced::advanced::widget::Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(&mut tree.children[0], child, renderer, op);
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
        let b = layout.bounds();
        let extent = match self.axis {
            Axis::Horizontal => b.width,
            Axis::Vertical => b.height,
        };
        let mut child_cursor = cursor;
        let mut forward = true;
        {
            let s = tree.state.downcast_mut::<State>();
            // (position, doigt) de l'événement, selon sa nature.
            enum Ev {
                Press(Point, Option<touch::Finger>),
                Move(Point, Option<touch::Finger>),
                Release(Option<touch::Finger>),
                Other,
            }
            let ev = match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    match cursor.position_over(b) {
                        Some(p) => Ev::Press(p, None),
                        None => Ev::Other,
                    }
                }
                Event::Mouse(mouse::Event::CursorMoved { position }) => Ev::Move(*position, None),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    Ev::Release(None)
                }
                Event::Touch(touch::Event::FingerPressed { id, position })
                    if b.contains(*position) =>
                {
                    Ev::Press(*position, Some(*id))
                }
                Event::Touch(touch::Event::FingerMoved { id, position }) => {
                    Ev::Move(*position, Some(*id))
                }
                Event::Touch(
                    touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
                ) => Ev::Release(Some(*id)),
                _ => Ev::Other,
            };
            match ev {
                Ev::Press(p, f) => {
                    if s.start.is_none() {
                        s.reset();
                        s.start = Some(p);
                        s.finger = f;
                        if let Some(m) = &self.on_press {
                            shell.publish(m.clone());
                        }
                    }
                }
                Ev::Move(p, f) if s.start.is_some() && s.finger == f => {
                    let start = s.start.unwrap_or(p);
                    let (along, across) = self.along(p - start);
                    if !s.dragging && !s.crossing {
                        if along.abs() > SLOP && along.abs() > across.abs() * 1.5 {
                            s.dragging = true;
                        } else if self.on_cross.is_some()
                            && across.abs() > SLOP
                            && across.abs() > along.abs() * 1.5
                        {
                            s.crossing = true;
                        }
                    }
                    if s.crossing {
                        let now = Instant::now();
                        if let Some((prev, t)) = s.last {
                            let dt = now.duration_since(t).as_secs_f32() * 1000.0;
                            if dt > 0.5 {
                                s.velocity = 0.7 * ((across - prev) / dt) + 0.3 * s.velocity;
                            }
                        }
                        s.last = Some((across, now));
                        s.cross = across;
                        shell.capture_event();
                        forward = false;
                    }
                    if s.dragging {
                        let now = Instant::now();
                        if let Some((prev, t)) = s.last {
                            let dt = now.duration_since(t).as_secs_f32() * 1000.0;
                            if dt > 0.5 {
                                s.velocity = 0.7 * ((along - prev) / dt) + 0.3 * s.velocity;
                            }
                        }
                        s.last = Some((along, now));
                        s.offset = along;
                        shell.request_redraw();
                        shell.capture_event();
                        forward = false;
                    }
                }
                Ev::Release(f) if s.start.is_some() && s.finger == f => {
                    if s.crossing {
                        let cancelled =
                            matches!(event, Event::Touch(touch::Event::FingerLost { .. }));
                        let cross_extent = match self.axis {
                            Axis::Horizontal => b.height,
                            Axis::Vertical => b.width,
                        };
                        // Même règle que les pages : assez long ou assez vif.
                        let dir = if cancelled {
                            0
                        } else {
                            decide(s.cross, s.velocity, cross_extent)
                        };
                        if dir != 0
                            && let Some(f) = &self.on_cross
                        {
                            shell.publish(f(dir));
                        }
                        shell.capture_event();
                        child_cursor = mouse::Cursor::Unavailable;
                        s.reset();
                    } else if s.dragging {
                        let velocity = if s
                            .last
                            .is_some_and(|(_, t)| t.elapsed() < Duration::from_millis(100))
                        {
                            s.velocity
                        } else {
                            0.0
                        };
                        let cancelled =
                            matches!(event, Event::Touch(touch::Event::FingerLost { .. }));
                        let dir = if cancelled {
                            0
                        } else {
                            decide(s.offset, velocity, extent)
                        };
                        if dir != 0 {
                            shell.publish((self.on_swipe)(dir));
                        }
                        shell.request_redraw();
                        shell.capture_event();
                        // Les boutons relâchent sans déclencher.
                        child_cursor = mouse::Cursor::Unavailable;
                        if dir == 0 && !self.duration.is_zero() {
                            let spring = Spring::rest(s.offset * 0.85).retarget(
                                0.0,
                                self.duration,
                                Instant::now(),
                            );
                            s.reset();
                            s.settle = Some(spring);
                        } else {
                            s.reset();
                        }
                    } else {
                        s.reset();
                    }
                }
                _ => {}
            }
            if s.dragging || s.crossing {
                child_cursor = mouse::Cursor::Unavailable;
            }
            if let Some(settle) = s.settle {
                if settle.done(Instant::now()) || self.duration.is_zero() {
                    s.settle = None;
                } else {
                    shell.request_redraw();
                }
            }
        }
        if !forward {
            return;
        }
        let Some(child) = layout.children().next() else {
            return;
        };
        // Un relâcher après glisser part sans position : les boutons ne
        // le prennent pas pour un clic.
        let released_drag = child_cursor == mouse::Cursor::Unavailable;
        let ev = match event {
            Event::Touch(touch::Event::FingerLifted { id, .. }) if released_drag => {
                Event::Touch(touch::Event::FingerLost {
                    id: *id,
                    position: Point::new(f32::NAN, f32::NAN),
                })
            }
            e => e.clone(),
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            &ev,
            child,
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
        let s = tree.state.downcast_ref::<State>();
        if s.dragging {
            return mouse::Interaction::Grabbing;
        }
        let Some(child) = layout.children().next() else {
            return mouse::Interaction::None;
        };
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            child,
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
        let Some(child) = layout.children().next() else {
            return;
        };
        let b = layout.bounds();
        // Résistance : le contenu suit le doigt à 85 %.
        let d = if self.duration.is_zero() {
            0.0
        } else if s.dragging {
            s.offset * 0.85
        } else {
            s.settle.map_or(0.0, |s| s.value(Instant::now()))
        };
        let shift = match self.axis {
            Axis::Horizontal => Vector::new(d, 0.0),
            Axis::Vertical => Vector::new(0.0, d),
        };
        renderer.with_layer(b, |r| {
            r.with_translation(shift, |r| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    r,
                    theme,
                    style,
                    child,
                    cursor,
                    viewport,
                );
            });
        });
    }
}

impl<'a> From<Swipe<'a>> for Element<'a, Msg> {
    fn from(s: Swipe<'a>) -> Self {
        Element::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_au_relacher() {
        // Page de 2560 px : 18 % ≈ 461 px.
        assert_eq!(decide(-500.0, 0.0, 2560.0), 1);
        assert_eq!(decide(500.0, 0.0, 2560.0), -1);
        assert_eq!(decide(-100.0, 0.0, 2560.0), 0);
        // Geste court mais vif.
        assert_eq!(decide(-100.0, -0.9, 2560.0), 1);
        assert_eq!(decide(80.0, 1.2, 2560.0), -1);
    }
}
