//! Les boutons et curseurs natifs d'Iced ne prennent pas le focus au clavier.
//! Ce module leur donne un périmètre de focus : Tab et Maj + Tab à
//! l'intérieur, Ctrl+Tab et Ctrl+Maj + Tab de groupe de contrôles en groupe.
use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::{self, Focusable, Outcome, focusable};
use iced::advanced::widget::{self, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::keyboard::{Key, key::Named};
use iced::{Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Task, Theme, mouse};

use crate::ui::Msg;

pub(super) const PANE: &str = "continuity-controls";
pub(super) const SCROLL: &str = "continuity-cards";

/// Les groupes de contrôles du volet, dans l'ordre de lecture : Ctrl+Tab
/// passe au premier contrôle du suivant, Ctrl+Maj + Tab à celui du précédent
/// (HIG ENT-11).
pub(super) const GROUPS: [&str; 4] = [
    "continuity-group-0",
    "continuity-group-1",
    "continuity-group-2",
    "continuity-group-3",
];

pub(in crate::ui) fn handle(key: &Key, shift: bool, control: bool) -> Task<Msg> {
    match key {
        Key::Named(Named::Tab) if control => {
            widget::operate(operation::scope(widget::Id::new(PANE), jump(shift)))
        }
        Key::Named(Named::Tab) => step(widget::Id::new(PANE), shift),
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

/// Le contrôle suivant (précédent) de la surface nommée `scope` prend le
/// focus ; l'opération lui est bornée, elle ne va pas aux autres surfaces
/// (HIG ENT-02 : le volet des Réglages, la barre).
pub(in crate::ui) fn step(scope: widget::Id, previous: bool) -> Task<Msg> {
    let inner: Box<dyn Operation<Msg>> = if previous {
        Box::new(focusable::focus_previous())
    } else {
        Box::new(focusable::focus_next())
    };
    widget::operate(operation::scope(scope, inner))
}

/// Groupe cible de Ctrl+Tab : celui qui suit (précède) le groupe du contrôle
/// qui a le focus ; sans focus, le premier (le dernier). `counts` : le nombre
/// de contrôles de chaque groupe, dans l'ordre de lecture ; un groupe vide est
/// sauté, la boucle refermée.
fn target(counts: &[usize], focused: Option<usize>, previous: bool) -> Option<usize> {
    let n = counts.len();
    (1..=n).find_map(|k| {
        let i = match (focused, previous) {
            (Some(f), false) => (f + k) % n,
            (Some(f), true) => (f + n - k) % n,
            (None, false) => k - 1,
            (None, true) => n - k,
        };
        (counts[i] > 0).then_some(i)
    })
}

/// Rang d'un identifiant de groupe dans l'ordre de lecture.
fn position(id: &widget::Id) -> Option<usize> {
    GROUPS.iter().position(|g| id == &widget::Id::new(g))
}

/// Saute au premier contrôle du groupe suivant (précédent). Deux passages,
/// comme `focus_next` : où est le focus, puis où le poser.
fn jump(previous: bool) -> impl Operation<Msg> {
    struct Locate {
        previous: bool,
        /// Contrôles de chaque groupe, dans l'ordre de lecture.
        counts: Vec<usize>,
        current: Option<usize>,
        focused: Option<usize>,
    }

    impl Operation<Msg> for Locate {
        fn container(&mut self, id: Option<&widget::Id>, _bounds: Rectangle) {
            if let Some(i) = id.and_then(position) {
                if i >= self.counts.len() {
                    self.counts.resize(i + 1, 0);
                }
                self.current = Some(i);
            }
        }

        fn focusable(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn Focusable,
        ) {
            if let Some(i) = self.current {
                self.counts[i] += 1;
                if state.is_focused() {
                    self.focused = Some(i);
                }
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Msg>)) {
            operate(self);
        }

        fn finish(&self) -> Outcome<Msg> {
            match target(&self.counts, self.focused, self.previous) {
                Some(i) => Outcome::Chain(Box::new(Enter {
                    target: widget::Id::new(GROUPS[i]),
                    current: false,
                    done: false,
                })),
                None => Outcome::None,
            }
        }
    }

    /// Deuxième passage : le premier contrôle du groupe cible prend le focus,
    /// les autres le perdent.
    struct Enter {
        target: widget::Id,
        current: bool,
        done: bool,
    }

    impl Operation<Msg> for Enter {
        fn container(&mut self, id: Option<&widget::Id>, _bounds: Rectangle) {
            self.current = id == Some(&self.target);
        }

        fn focusable(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            state: &mut dyn Focusable,
        ) {
            if self.current && !self.done {
                state.focus();
                self.done = true;
            } else {
                state.unfocus();
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Msg>)) {
            operate(self);
        }
    }

    Locate {
        previous,
        counts: Vec::new(),
        current: None,
        focused: None,
    }
}

/// Un groupe de contrôles du volet, par son rang dans [`GROUPS`]. Ne change
/// ni la mise en page ni le dessin : il se borne à se nommer aux opérations.
pub(super) fn group<'a>(rank: usize, child: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    Element::new(Group {
        id: widget::Id::new(GROUPS[rank]),
        child: child.into(),
    })
}

struct Group<'a> {
    id: widget::Id,
    child: Element<'a, Msg>,
}

impl Widget<Msg, Theme, Renderer> for Group<'_> {
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
        operation.container(Some(&self.id), layout.bounds());
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
        self.child.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
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

/// L'enveloppe de focus d'un contrôle : anneau, activation par Entrée ou
/// Espace. Celle des cartes du centre de contrôle (HIG ENT-07, ENT-11) comme
/// des modules de la barre (HIG ENT-02).
pub(in crate::ui) fn press<'a>(
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

#[cfg(test)]
mod tests {
    use super::target;

    /// Quatre groupes, le deuxième sans contrôle.
    const COUNTS: [usize; 4] = [3, 0, 4, 2];

    #[test]
    fn groupes_suivant_et_precedent() {
        // Suivant : le groupe vide est sauté ; la boucle est refermée.
        assert_eq!(target(&COUNTS, Some(0), false), Some(2));
        assert_eq!(target(&COUNTS, Some(2), false), Some(3));
        assert_eq!(target(&COUNTS, Some(3), false), Some(0));
        // Précédent.
        assert_eq!(target(&COUNTS, Some(0), true), Some(3));
        assert_eq!(target(&COUNTS, Some(2), true), Some(0));
        assert_eq!(target(&COUNTS, Some(3), true), Some(2));
    }

    #[test]
    fn groupes_sans_focus() {
        // Sans focus : le premier groupe, le dernier pour « précédent ».
        assert_eq!(target(&COUNTS, None, false), Some(0));
        assert_eq!(target(&COUNTS, None, true), Some(3));
        assert_eq!(target(&[0, 0, 2], None, false), Some(2));
        assert_eq!(target(&[0, 0, 2], None, true), Some(2));
    }

    #[test]
    fn groupes_vides_ou_seul() {
        assert_eq!(target(&[0, 0], Some(0), false), None);
        assert_eq!(target(&[], None, false), None);
        // Seul le groupe du focus a des contrôles : son premier.
        assert_eq!(target(&[0, 2, 0], Some(1), true), Some(1));
    }
}
