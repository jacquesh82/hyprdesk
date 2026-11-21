//! Appui long sur un contenu interactif (tuile de module) : le widget voit
//! l'appui avant les boutons qu'il enveloppe. Doigt (ou souris) tenu sans
//! bouger pendant `HOLD` : `on_hold` est publié, et le relâcher qui suit ne
//! déclenche aucun bouton dessous. Un geste qui bouge annule l'appui long
//! (le défilement, les curseurs restent libres).

use std::time::{Duration, Instant};

use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
    widget::{Tree, tree},
};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, touch, window};

use crate::ui::Msg;

/// Durée d'un appui long.
pub const HOLD: Duration = Duration::from_millis(650);
/// Mouvement toléré pendant l'appui.
const SLOP: f32 = 12.0;

pub struct Hold<'a> {
    content: Element<'a, Msg>,
    on_hold: Msg,
}

pub fn hold<'a>(content: impl Into<Element<'a, Msg>>, on_hold: Msg) -> Hold<'a> {
    Hold {
        content: content.into(),
        on_hold,
    }
}

#[derive(Debug, Default)]
struct State {
    /// Appui en cours : (position, début, doigt).
    start: Option<(Point, Instant, Option<touch::Finger>)>,
    fired: bool,
}

/// L'appui commencé à `t0` est devenu long à `now`.
fn due(t0: Instant, now: Instant) -> bool {
    now.duration_since(t0) >= HOLD
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Hold<'_> {
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
        let b = layout.bounds();
        let s = tree.state.downcast_mut::<State>();
        let mut child_event = event.clone();
        let mut child_cursor = cursor;
        let finger = |f: Option<touch::Finger>| s.start.is_some_and(|(_, _, g)| g == f);
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let (p, f) = match event {
                    Event::Touch(touch::Event::FingerPressed { id, position }) => {
                        (Some(*position), Some(*id))
                    }
                    _ => (cursor.position(), None),
                };
                if let Some(p) = p.filter(|p| b.contains(*p))
                    && s.start.is_none()
                {
                    let now = Instant::now();
                    *s = State {
                        start: Some((p, now, f)),
                        fired: false,
                    };
                    shell.request_redraw_at(now + HOLD);
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if finger(None) => {
                if let Some((p0, _, _)) = s.start
                    && !s.fired
                    && (*position - p0).x.hypot((*position - p0).y) > SLOP
                {
                    *s = State::default();
                }
            }
            Event::Touch(touch::Event::FingerMoved { id, position }) if finger(Some(*id)) => {
                if let Some((p0, _, _)) = s.start
                    && !s.fired
                    && (*position - p0).x.hypot((*position - p0).y) > SLOP
                {
                    *s = State::default();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some((_, t0, _)) = s.start
                    && !s.fired
                {
                    if due(t0, *now) {
                        s.fired = true;
                        shell.publish(self.on_hold.clone());
                    } else {
                        shell.request_redraw_at(t0 + HOLD);
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if finger(None) => {
                if s.fired {
                    child_cursor = mouse::Cursor::Unavailable;
                    shell.capture_event();
                }
                *s = State::default();
            }
            Event::Touch(
                touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
            ) if finger(Some(*id)) => {
                if s.fired {
                    // Les boutons dessous relâchent sans déclencher.
                    child_cursor = mouse::Cursor::Unavailable;
                    child_event = Event::Touch(touch::Event::FingerLost {
                        id: *id,
                        position: Point::new(f32::NAN, f32::NAN),
                    });
                    shell.capture_event();
                }
                *s = State::default();
            }
            _ => {}
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            &child_event,
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
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

impl<'a> From<Hold<'a>> for Element<'a, Msg> {
    fn from(h: Hold<'a>) -> Self {
        Element::new(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duree_de_l_appui_long() {
        let t0 = Instant::now();
        assert!(!due(t0, t0 + Duration::from_millis(300)));
        assert!(due(t0, t0 + HOLD));
    }
}
