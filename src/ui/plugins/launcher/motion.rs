//! Transformation décorative : la boîte et les événements du bouton restent
//! fixes. Contrairement à `float`, aucune surcouche ne masque le pointeur.
use std::time::{Duration, Instant};

use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::{Element, Event, Length, Point, Rectangle, Size, Transformation, Vector};

use super::UiMsg;
use crate::ui::Fade;

/// Conteneur interactif transformé au dessin, sans surcouche ni déplacement
/// de ses zones de clic. Sert aussi à relever les limites réelles des boutons.
pub(super) fn surface<'a>(
    content: impl Into<Element<'a, UiMsg>>,
    scale: f32,
    offset: Vector,
    pivot: Point,
) -> Element<'a, UiMsg> {
    Element::new(Surface {
        content: content.into(),
        scale,
        offset,
        pivot,
        anchor: None,
    })
}

pub(super) fn anchor<'a>(
    content: impl Into<Element<'a, UiMsg>>,
    monitor: String,
    key: String,
) -> Element<'a, UiMsg> {
    Element::new(Surface {
        content: content.into(),
        scale: 1.0,
        offset: Vector::ZERO,
        pivot: Point::ORIGIN,
        anchor: Some((monitor, key)),
    })
}

struct Surface<'a> {
    content: Element<'a, UiMsg>,
    scale: f32,
    offset: Vector,
    pivot: Point,
    anchor: Option<(String, String)>,
}

#[derive(Default)]
struct Bounds(Option<(String, String, Rectangle)>);

impl Widget<UiMsg, iced::Theme, iced::Renderer> for Surface<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Bounds>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(Bounds::default())
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
        shell: &mut Shell<'_, UiMsg>,
        viewport: &Rectangle,
    ) {
        if let Some((monitor, key)) = &self.anchor {
            let state = tree.state.downcast_mut::<Bounds>();
            let value = (monitor.clone(), key.clone(), layout.bounds());
            if state.0.as_ref() != Some(&value) {
                state.0 = Some(value);
                shell.publish(UiMsg::LauncherAnchor(
                    monitor.clone(),
                    key.clone(),
                    layout.bounds(),
                ));
            }
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
        use iced::advanced::Renderer;
        let bounds = layout.bounds();
        let pivot = Point::new(
            bounds.x + bounds.width * self.pivot.x,
            bounds.y + bounds.height * self.pivot.y,
        );
        let transform = Transformation::translate(pivot.x + self.offset.x, pivot.y + self.offset.y)
            * Transformation::scale(self.scale)
            * Transformation::translate(-pivot.x, -pivot.y);
        renderer.with_layer(*viewport, |renderer| {
            renderer.with_transformation(transform, |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    &(*viewport * transform.inverse()),
                );
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
    ) -> Option<iced::advanced::overlay::Element<'b, UiMsg, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

pub(super) fn face<'a>(
    content: impl Into<Element<'a, UiMsg>>,
    scale: f32,
    offset: Vector,
    duration: Duration,
) -> Element<'a, UiMsg> {
    Element::new(Face {
        content: content.into(),
        scale,
        offset,
        duration,
    })
}

struct Face<'a> {
    content: Element<'a, UiMsg>,
    scale: f32,
    offset: Vector,
    duration: Duration,
}

#[derive(Default)]
struct Press {
    fade: Option<Fade>,
}

impl Widget<UiMsg, iced::Theme, iced::Renderer> for Face<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Press>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Press::default())
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

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, UiMsg>,
        _viewport: &Rectangle,
    ) {
        let press = tree.state.downcast_mut::<Press>();
        let over = cursor.is_over(layout.bounds().expand(6.0));
        let target = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if over => Some(1.0),
            Event::Mouse(
                mouse::Event::ButtonReleased(mouse::Button::Left) | mouse::Event::CursorLeft,
            )
            | Event::Window(iced::window::Event::Unfocused) => Some(0.0),
            Event::Mouse(mouse::Event::CursorMoved { .. }) if !over => Some(0.0),
            _ => None,
        };
        let now = Instant::now();
        if let Some(to) = target {
            let previous = press.fade.map_or(0.0, |f| f.to);
            if to != previous {
                press.fade = Some(Fade {
                    from: press.fade.map_or(0.0, |f| f.value(now)),
                    to,
                    start: now,
                    dur: self
                        .duration
                        .min(Duration::from_millis(if to > 0.0 { 100 } else { 160 })),
                });
                shell.request_redraw();
            }
        }
        if press.fade.is_some_and(|f| !f.done(now)) {
            shell.request_redraw();
        }
        // Le bouton parent reçoit toujours pression, relâchement et annulation.
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer;
        let press = if self.duration.is_zero() {
            0.0
        } else {
            tree.state
                .downcast_ref::<Press>()
                .fade
                .map_or(0.0, |f| f.value(Instant::now()))
        };
        let center = layout.bounds().center();
        let transform =
            Transformation::translate(center.x + self.offset.x, center.y + self.offset.y)
                * Transformation::scale(self.scale * (1.0 - 0.045 * press))
                * Transformation::translate(-center.x, -center.y);
        renderer.with_layer(*viewport, |renderer| {
            renderer.with_transformation(transform, |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    layout,
                    mouse::Cursor::Unavailable,
                    &(*viewport * transform.inverse()),
                );
            });
        });
    }
}
