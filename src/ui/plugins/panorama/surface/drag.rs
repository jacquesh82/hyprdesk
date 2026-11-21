//! Défilement au doigt (ou à la souris) d'une liste qui défile : on touche
//! n'importe où dans la zone et on glisse, sans viser la barre de
//! défilement. Tant que le geste reste sous le seuil, les boutons reçoivent
//! l'appui comme d'habitude ; au-delà, le mouvement devient un défilement
//! (molette en pixels, transmise au `scrollable` enveloppé) et le relâcher
//! ne déclenche aucun bouton.

use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
    widget::{Tree, tree},
};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, touch};

use crate::ui::Msg;

/// Distance avant de reconnaître un glisser (et non un appui).
const SLOP: f32 = 14.0;

pub struct Drag<'a> {
    content: Element<'a, Msg>,
}

/// Enveloppe `content` (un `scrollable`, ou un conteneur qui en porte un).
pub fn drag_scroll<'a>(content: impl Into<Element<'a, Msg>>) -> Drag<'a> {
    Drag {
        content: content.into(),
    }
}

#[derive(Debug, Default)]
struct State {
    start: Option<Point>,
    /// Doigt suivi (aucun : la souris).
    finger: Option<touch::Finger>,
    last: Point,
    dragging: bool,
}

/// Le geste depuis `start` jusqu'à `p` dépasse le seuil.
fn beyond(start: Point, p: Point) -> bool {
    let v = p - start;
    v.x.hypot(v.y) > SLOP
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Drag<'_> {
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
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => Ev::Release(None),
            Event::Touch(touch::Event::FingerPressed { id, position }) if b.contains(*position) => {
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
        let mut child_event = event.clone();
        let mut child_cursor = cursor;
        match ev {
            Ev::Press(p, f) if s.start.is_none() => {
                *s = State {
                    start: Some(p),
                    finger: f,
                    last: p,
                    dragging: false,
                };
            }
            Ev::Move(p, f) if s.finger == f && s.start.is_some() => {
                let start = s.start.unwrap_or(p);
                if !s.dragging && beyond(start, p) {
                    s.dragging = true;
                }
                if s.dragging {
                    // Le contenu suit le doigt : molette en pixels, sous le
                    // point de départ (dans la zone qui défile).
                    let d: Vector = p - s.last;
                    s.last = p;
                    child_event = Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Pixels { x: d.x, y: d.y },
                    });
                    child_cursor = mouse::Cursor::Available(start);
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Ev::Release(f) if s.finger == f && s.start.is_some() => {
                if s.dragging {
                    // Les boutons relâchent sans déclencher.
                    child_cursor = mouse::Cursor::Unavailable;
                    if let Event::Touch(touch::Event::FingerLifted { id, .. }) = event {
                        child_event = Event::Touch(touch::Event::FingerLost {
                            id: *id,
                            position: Point::new(f32::NAN, f32::NAN),
                        });
                    }
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
        if tree.state.downcast_ref::<State>().dragging {
            return mouse::Interaction::Grabbing;
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

impl<'a> From<Drag<'a>> for Element<'a, Msg> {
    fn from(d: Drag<'a>) -> Self {
        Element::new(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seuil_du_glisser() {
        let o = Point::ORIGIN;
        assert!(!beyond(o, Point::new(8.0, 8.0)));
        assert!(beyond(o, Point::new(15.0, 0.0)));
        assert!(beyond(o, Point::new(0.0, -20.0)));
    }
}
