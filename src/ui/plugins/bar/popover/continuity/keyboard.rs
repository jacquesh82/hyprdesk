//! Les boutons et curseurs natifs d'Iced ne prennent pas le focus au clavier.
use iced::advanced::widget::operation::{self, Focusable, focusable};
use iced::advanced::widget::{self, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::keyboard::{Key, key::Named};
use iced::{Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Task, Theme, mouse};

use crate::ui::Msg;

pub(super) const PANE: &str = "continuity-controls";
pub(super) const SCROLL: &str = "continuity-cards";

pub(in crate::ui) fn handle(key: &Key, shift: bool) -> Task<Msg> {
    match key {
        Key::Named(Named::Tab) => {
            let scope = widget::Id::new(PANE);
            if shift {
                widget::operate(operation::scope(scope, focusable::focus_previous()))
            } else {
                widget::operate(operation::scope(scope, focusable::focus_next()))
            }
        }
        Key::Named(Named::PageUp | Named::PageDown) => iced::widget::operation::scroll_by(
            SCROLL,
            iced::widget::operation::AbsoluteOffset {
                x: 0.0,
                y: if *key == Key::Named(Named::PageUp) {
                    -260.0
                } else {
                    260.0
                },
            },
        ),
        _ => Task::none(),
    }
}

pub(super) fn press<'a>(
    child: impl Into<Element<'a, Msg>>,
    message: Option<Msg>,
    ring: Color,
) -> Element<'a, Msg> {
    let child = child.into();
    match message {
        Some(message) => Element::new(Keyboard {
            child,
            action: Action::Press(message),
            ring,
        }),
        None => child,
    }
}

pub(super) fn adjust<'a>(
    child: impl Into<Element<'a, Msg>>,
    value: f32,
    max: f32,
    change: impl Fn(f32) -> Msg + 'a,
    commit: Option<Msg>,
    ring: Color,
) -> Element<'a, Msg> {
    Element::new(Keyboard {
        child: child.into(),
        action: Action::Adjust {
            value,
            max,
            change: Box::new(change),
            commit,
        },
        ring,
    })
}

enum Action<'a> {
    Press(Msg),
    Adjust {
        value: f32,
        max: f32,
        change: Box<dyn Fn(f32) -> Msg + 'a>,
        commit: Option<Msg>,
    },
}

struct Keyboard<'a> {
    child: Element<'a, Msg>,
    action: Action<'a>,
    ring: Color,
}

#[derive(Default)]
struct Focus(bool);

impl Focusable for Focus {
    fn is_focused(&self) -> bool {
        self.0
    }
    fn focus(&mut self) {
        self.0 = true;
    }
    fn unfocus(&mut self) {
        self.0 = false;
    }
}

impl Widget<Msg, Theme, Renderer> for Keyboard<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Focus>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Focus::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.child)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.child));
    }

    fn size(&self) -> Size<Length> {
        self.child.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.focusable(None, layout.bounds(), tree.state.downcast_mut::<Focus>());
        self.child
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Msg>,
        viewport: &Rectangle,
    ) {
        if let Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) = event
            && tree.state.downcast_ref::<Focus>().0
        {
            match (&self.action, key) {
                (Action::Press(message), Key::Named(Named::Enter | Named::Space)) => {
                    shell.publish(message.clone());
                    shell.capture_event();
                    return;
                }
                (
                    Action::Adjust {
                        value,
                        max,
                        change,
                        commit,
                    },
                    Key::Named(name),
                ) if matches!(
                    name,
                    Named::ArrowLeft | Named::ArrowRight | Named::ArrowUp | Named::ArrowDown
                ) =>
                {
                    let delta = if matches!(name, Named::ArrowLeft | Named::ArrowDown) {
                        -0.05
                    } else {
                        0.05
                    };
                    shell.publish(change((value + delta).clamp(0.0, *max)));
                    if let Some(commit) = commit {
                        shell.publish(commit.clone());
                    }
                    shell.capture_event();
                    return;
                }
                _ => {}
            }
        }
        self.child.as_widget_mut().update(
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

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use renderer::Renderer as _;

        let child = &tree.children[0];
        self.child
            .as_widget()
            .draw(child, renderer, theme, style, layout, cursor, viewport);
        if tree.state.downcast_ref::<Focus>().0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: Border::default().rounded(8).width(2).color(self.ring),
                    ..Default::default()
                },
                Color::TRANSPARENT,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.child.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}
