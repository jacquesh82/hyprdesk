//! Le périmètre de focus du centre de contrôle : Tab et Maj + Tab à
//! l'intérieur, Ctrl+Tab et Ctrl+Maj + Tab de groupe de contrôles en groupe.
//! L'enveloppe des contrôles est celle du shell, `crate::ui::focus`.
use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::{self, Focusable, Outcome};
use iced::advanced::widget::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::keyboard::{Key, key::Named};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Task, Theme, mouse};

use crate::ui::Msg;
use crate::ui::focus::step;

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

    /// Le menu ou l'infobulle d'un contrôle du groupe : ceux de l'enfant.
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Msg, Theme, Renderer>> {
        self.child.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
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
