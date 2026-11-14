//! Mouvement local de la barre : dessin et redessins uniquement pendant le geste.
//! Les dispositions et les boîtes de clic ne suivent jamais les transformations.
use std::time::{Duration, Instant};

use iced::advanced::{
    Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Transformation, Vector};

use super::{Fade, Msg};

/// Hauteur de la montée d'une touche soulevée (style Graphite), en pixels.
pub(super) const LIFT_PX: f32 = 4.0;
/// Fraction de la montée au survol d'une touche inactive.
const HOVER_LIFT: f32 = 0.25;

/// Aspect de la capsule qui suit le module survolé.
#[derive(Clone, Copy)]
pub(super) enum RailLook {
    /// Capsule remplie (Continuité).
    Fill(Color),
}

pub(super) fn anchor<'a>(
    content: impl Into<Element<'a, Msg>>,
    bar: iced::window::Id,
    key: String,
    duration: Duration,
) -> Element<'a, Msg> {
    let press = !matches!(key.as_str(), "window" | "workspaces" | "tray" | "desktop");
    Element::new(Motion {
        content: content.into(),
        anchor: Some((bar, key)),
        rail: None,
        press,
        lift: None,
        raised: false,
        ink: Color::TRANSPARENT,
        duration,
        inert: false,
        transform: None,
    })
}

pub(super) fn press<'a>(
    content: impl Into<Element<'a, Msg>>,
    duration: Duration,
) -> Element<'a, Msg> {
    Element::new(Motion {
        content: content.into(),
        anchor: None,
        rail: None,
        press: true,
        lift: None,
        raised: false,
        ink: Color::TRANSPARENT,
        duration,
        inert: false,
        transform: None,
    })
}

pub(super) fn rail<'a>(
    content: impl Into<Element<'a, Msg>>,
    selected: Option<usize>,
    eligible: Vec<bool>,
    look: RailLook,
    duration: Duration,
) -> Element<'a, Msg> {
    Element::new(Motion {
        content: content.into(),
        anchor: None,
        rail: Some((selected, eligible, look)),
        press: false,
        lift: None,
        raised: false,
        ink: Color::TRANSPARENT,
        duration,
        inert: false,
        transform: None,
    })
}

/// Touche soulevée (Graphite) : monte quand elle devient active ou survolée,
/// redescend sinon ; l'ancienne touche part de la hauteur affichée, sans
/// rejouer les transitions intermédiaires. Les boîtes de clic ne bougent pas.
pub(super) fn lift<'a>(
    content: impl Into<Element<'a, Msg>>,
    raised: bool,
    ink: Color,
    duration: Duration,
) -> Element<'a, Msg> {
    Element::new(Motion {
        content: content.into(),
        anchor: None,
        rail: None,
        press: true,
        lift: Some(duration),
        raised,
        ink,
        duration,
        inert: false,
        transform: None,
    })
}

pub(super) fn inert<'a>(content: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    Element::new(Motion {
        content: content.into(),
        anchor: None,
        rail: None,
        press: false,
        lift: None,
        raised: false,
        ink: Color::TRANSPARENT,
        duration: Duration::ZERO,
        inert: true,
        transform: None,
    })
}

pub(super) fn surface<'a>(
    content: impl Into<Element<'a, Msg>>,
    scale: f32,
    offset: Vector,
    pivot: iced::Point,
) -> Element<'a, Msg> {
    Element::new(Motion {
        content: content.into(),
        anchor: None,
        rail: None,
        press: false,
        lift: None,
        raised: false,
        ink: Color::TRANSPARENT,
        duration: Duration::ZERO,
        inert: false,
        transform: Some((scale, offset, pivot)),
    })
}

#[derive(Clone, Copy, Debug)]
struct Travel {
    from: Rectangle,
    to: Rectangle,
    fade: Fade,
}

impl Travel {
    fn value(self, now: Instant) -> Rectangle {
        let t = self.fade.value(now);
        Rectangle {
            x: self.from.x + (self.to.x - self.from.x) * t,
            y: self.from.y + (self.to.y - self.from.y) * t,
            width: self.from.width + (self.to.width - self.from.width) * t,
            height: self.from.height + (self.to.height - self.from.height) * t,
        }
    }
}

#[derive(Default)]
struct State {
    anchor: Option<(iced::window::Id, String, Rectangle)>,
    travel: Option<Travel>,
    visibility: Option<Fade>,
    press: Option<Fade>,
    lift: Option<Fade>,
}

impl State {
    fn retarget(&mut self, target: Option<Rectangle>, now: Instant, duration: Duration) {
        if let Some(to) = target
            && self.travel.is_none_or(|t| t.to != to)
        {
            let from = self.travel.map_or(to, |t| t.value(now));
            self.travel = Some(Travel {
                from,
                to,
                fade: Fade {
                    from: 0.0,
                    to: 1.0,
                    start: now,
                    dur: duration,
                },
            });
        }
        let to = if target.is_some() { 1.0 } else { 0.0 };
        if self.visibility.is_none_or(|f| f.to != to) {
            self.visibility = Some(Fade {
                from: self.visibility.map_or(0.0, |f| f.value(now)),
                to,
                start: now,
                dur: duration.min(Duration::from_millis(100)),
            });
        }
    }

    fn animating(&self, now: Instant) -> bool {
        self.visibility.is_some_and(|f| !f.done(now))
            || self.press.is_some_and(|f| !f.done(now))
            || self.lift.is_some_and(|f| !f.done(now))
            || self.travel.is_some_and(|t| !t.fade.done(now))
    }

    /// Recible la montée de la touche : elle part toujours de la hauteur
    /// affichée, les changements rapides convergent sans rejouer les étapes.
    fn retarget_lift(&mut self, to: f32, now: Instant, dur: Duration) {
        if self.lift.is_none_or(|f| f.to != to) {
            self.lift = Some(Fade {
                from: self.lift.map_or(0.0, |f| f.value(now)),
                to,
                start: now,
                dur,
            });
        }
    }
}

struct Motion<'a> {
    content: Element<'a, Msg>,
    anchor: Option<(iced::window::Id, String)>,
    rail: Option<(Option<usize>, Vec<bool>, RailLook)>,
    press: bool,
    /// Touche soulevée : la durée de sa montée (`None` : sans lift).
    lift: Option<Duration>,
    /// La touche est la clé de l'espace actif.
    raised: bool,
    /// Encre du reflet d'arête d'une touche soulevée.
    ink: Color,
    duration: Duration,
    inert: bool,
    transform: Option<(f32, Vector, iced::Point)>,
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Motion<'_> {
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
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        if self.inert {
            return;
        }
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        if self.inert {
            return;
        }
        let now = Instant::now();
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<State>();
        if let Some((bar, key)) = &self.anchor {
            let value = (*bar, key.clone(), bounds);
            if state.anchor.as_ref() != Some(&value) {
                state.anchor = Some(value);
                shell.publish(Msg::BarAnchor(*bar, key.clone(), bounds));
            }
        }
        if let Some((selected, eligible, _)) = &self.rail {
            let target = layout.children().enumerate().find_map(|(i, child)| {
                let b = child.bounds();
                let active = selected.map_or_else(
                    || eligible.get(i).copied().unwrap_or(false) && cursor.is_over(b),
                    |s| i == s,
                );
                (active && b.width > 0.0 && b.intersects(viewport))
                    .then_some(b - Vector::new(bounds.x, bounds.y))
            });
            state.retarget(target, now, self.duration);
        }
        if self.press {
            let to = match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    if cursor.is_over(bounds) =>
                {
                    Some(1.0)
                }
                Event::Mouse(
                    mouse::Event::ButtonReleased(mouse::Button::Left) | mouse::Event::CursorLeft,
                )
                | Event::Window(iced::window::Event::Unfocused) => Some(0.0),
                Event::Mouse(mouse::Event::CursorMoved { .. }) if !cursor.is_over(bounds) => {
                    Some(0.0)
                }
                _ => None,
            };
            if let Some(to) = to
                && state.press.is_none_or(|f| f.to != to)
            {
                state.press = Some(Fade {
                    from: state.press.map_or(0.0, |f| f.value(now)),
                    to,
                    start: now,
                    dur: self.duration.min(Duration::from_millis(90)),
                });
            }
        }
        if let Some(dur) = self.lift {
            // Cible : la touche active est soulevée, une touche survolée monte
            // d'un pixel ; sinon elle reste gravée au niveau de la surface.
            let hovered = !matches!(cursor, mouse::Cursor::Unavailable) && cursor.is_over(bounds);
            let to = if self.raised {
                1.0
            } else if hovered {
                HOVER_LIFT
            } else {
                0.0
            };
            state.retarget_lift(to, now, dur);
        }
        if state.animating(now) && bounds.intersects(viewport) {
            shell.request_redraw();
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
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
        if self.inert {
            return mouse::Interaction::default();
        }
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
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        if let Some((_, _, look)) = self.rail
            && let Some(travel) = state.travel
        {
            let rect = if self.duration.is_zero() {
                travel.to
            } else {
                travel.value(now)
            } + Vector::new(layout.bounds().x, layout.bounds().y);
            let visibility = state.visibility.map_or(0.0, |f| {
                if self.duration.is_zero() {
                    f.to
                } else {
                    f.value(now)
                }
            });
            let radius = super::design::SMALL_RADIUS.min(rect.height / 2.0);
            match look {
                RailLook::Fill(fill) => renderer.fill_quad(
                    renderer::Quad {
                        bounds: rect,
                        border: Border::default().rounded(radius),
                        ..Default::default()
                    },
                    Color {
                        a: fill.a * visibility,
                        ..fill
                    },
                ),
            }
        }
        let press = state.press.map_or(0.0, |f| {
            if self.duration.is_zero() {
                0.0
            } else {
                f.value(now)
            }
        });
        let lift = self.lift.map_or(0.0, |_| {
            state.lift.map_or(0.0, |f| {
                if self.duration.is_zero() {
                    f.to
                } else {
                    f.value(now)
                }
            })
        });
        let b = layout.bounds();
        if lift > 0.0 {
            let radius = super::design::SMALL_RADIUS.min(b.height / 2.0);
            // Le creux laissé au sol par la touche soulevée.
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: b.x + 2.5,
                        y: b.y + 2.5,
                        width: (b.width - 5.0).max(0.0),
                        height: (b.height - 5.0).max(0.0),
                    },
                    border: Border::default().rounded(radius),
                    ..Default::default()
                },
                Color::from_rgba(0.0, 0.0, 0.0, 0.10 * lift),
            );
            // Ombre courte portée sous la touche, entre elle et la surface.
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: b.x + 2.0,
                        y: b.y - LIFT_PX * lift + 2.0,
                        width: (b.width - 4.0).max(0.0),
                        height: (b.height - 4.0).max(0.0),
                    },
                    border: Border::default().rounded(radius),
                    shadow: iced::Shadow {
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.30 * lift),
                        offset: Vector::new(0.0, 2.0),
                        blur_radius: 5.0,
                    },
                    ..Default::default()
                },
                Color::TRANSPARENT,
            );
        }
        let (scale, offset, pivot) =
            self.transform
                .unwrap_or((1.0, Vector::ZERO, iced::Point::new(0.5, 0.5)));
        let center = iced::Point::new(b.x + b.width * pivot.x, b.y + b.height * pivot.y);
        // La touche soulevée monte ; l'appui l'enfonce d'un pixel.
        let sink = if self.lift.is_some() { press } else { 0.0 };
        let ty = -LIFT_PX * lift + sink;
        let transform = Transformation::translate(center.x + offset.x, center.y + offset.y + ty)
            * Transformation::scale(scale * (1.0 - 0.03 * press))
            * Transformation::translate(-center.x, -center.y);
        renderer.with_transformation(transform, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        });
        if lift > 0.0 {
            // Reflet fin sur l'arête de la touche soulevée.
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: b.x + 2.0,
                        y: b.y - LIFT_PX * lift,
                        width: (b.width - 4.0).max(0.0),
                        height: 1.0,
                    },
                    ..Default::default()
                },
                Color {
                    a: (0.12 + 0.30 * lift) * self.ink.a,
                    ..self.ink
                },
            );
        }
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Msg, iced::Theme, iced::Renderer>> {
        if self.inert {
            return None;
        }
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
    use iced::Point;
    #[test]
    fn retargets_from_the_visible_position_and_stops() {
        let now = Instant::now();
        let duration = Duration::from_millis(180);
        let a = Rectangle::new(Point::ORIGIN, Size::new(28.0, 28.0));
        let b = a + Vector::new(100.0, 0.0);
        let c = a + Vector::new(200.0, 0.0);
        let mut s = State::default();
        s.retarget(Some(a), now, duration);
        s.retarget(Some(b), now, duration);
        let halfway = now + Duration::from_millis(90);
        let visible = s.travel.unwrap().value(halfway);
        s.retarget(Some(c), halfway, duration);
        assert_eq!(s.travel.unwrap().value(halfway), visible);
        assert!(!s.animating(halfway + duration));
        s.retarget(None, halfway + duration, duration);
        assert_eq!(s.visibility.unwrap().to, 0.0);
        assert!(!s.animating(halfway + duration * 2));
    }
    #[test]
    fn no_motion_uses_the_target_immediately() {
        let now = Instant::now();
        let b = Rectangle::new(Point::new(24.0, 0.0), Size::new(28.0, 28.0));
        let mut s = State::default();
        s.retarget(Some(b), now, Duration::ZERO);
        assert_eq!(s.travel.unwrap().value(now), b);
        assert_eq!(s.visibility.unwrap().value(now), 1.0);
        assert!(!s.animating(now));
    }
    #[test]
    fn la_touche_redescend_de_sa_hauteur_affichee() {
        let now = Instant::now();
        let dur = Duration::from_millis(170);
        let mut s = State::default();
        s.retarget_lift(1.0, now, dur);
        let half = now + Duration::from_millis(85);
        let visible = s.lift.unwrap().value(half);
        // La redescense part de la hauteur affichée (convergence rapide).
        s.retarget_lift(0.0, half, dur);
        let f = s.lift.unwrap();
        assert_eq!(f.value(half), visible);
        assert_eq!(f.to, 0.0);
        // Sans mouvement : la touche saute à sa hauteur cible.
        s.retarget_lift(1.0, half, Duration::ZERO);
        assert_eq!(s.lift.unwrap().value(half), 1.0);
    }
}
