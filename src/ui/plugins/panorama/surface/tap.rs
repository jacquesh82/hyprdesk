//! Appui sur un contenu interactif, vu avant lui : le message est publié dès
//! que le doigt (ou le bouton) se pose, même si le contenu garde l'événement
//! (un champ de saisie qui place son curseur). L'événement suit son chemin.

use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
    widget::{Tree, tree},
};
use iced::{Element, Event, Length, Rectangle, Size, Vector, touch};

use crate::ui::Msg;

pub struct Tap<'a> {
    content: Element<'a, Msg>,
    on_press: Msg,
}

pub fn tap<'a>(content: impl Into<Element<'a, Msg>>, on_press: Msg) -> Tap<'a> {
    Tap {
        content: content.into(),
        on_press,
    }
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Tap<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<()>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(())
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
        // Sur une surface layer-shell, le curseur suit le doigt.
        let bounds = layout.bounds();
        let pressed = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                cursor.position().is_some_and(|p| bounds.contains(p))
            }
            Event::Touch(touch::Event::FingerPressed { position, .. }) => {
                bounds.contains(*position)
            }
            _ => false,
        };
        if pressed {
            shell.publish(self.on_press.clone());
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

impl<'a> From<Tap<'a>> for Element<'a, Msg> {
    fn from(t: Tap<'a>) -> Self {
        Element::new(t)
    }
}
