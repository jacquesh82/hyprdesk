//! L'enveloppe de focus du shell (HIG ENT-01, ENT-02, ENT-07, ENT-11). Les
//! boutons, interrupteurs et curseurs natifs d'Iced ne prennent pas le focus
//! au clavier : enveloppés, ils entrent dans le parcours de Tab et Maj + Tab,
//! portent l'anneau de focus, et Entrée ou Espace les activent (les flèches
//! règlent un curseur). Une seule enveloppe pour toutes les surfaces : le
//! centre de contrôle, les modules de la barre, le tiroir des Réglages.
use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::{self, Focusable, focusable};
use iced::advanced::widget::{self, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::keyboard::{Key, key::Named};
use iced::{Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Task, Theme, mouse};

use crate::ui::Msg;

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

/// L'enveloppe d'un curseur de 0 à `max` : les flèches le règlent par pas
/// de 0,05 (les curseurs du centre de contrôle).
pub(in crate::ui) fn adjust<'a>(
    child: impl Into<Element<'a, Msg>>,
    value: f32,
    max: f32,
    change: impl Fn(f32) -> Msg + 'a,
    commit: Option<Msg>,
    ring: Color,
) -> Element<'a, Msg> {
    adjust_by(child, value, (0.0, max), 0.05, change, commit, ring)
}

/// L'enveloppe d'un curseur sur `range`, réglé aux flèches par `step` (les
/// curseurs du tiroir des Réglages, HIG ENT-01).
pub(in crate::ui) fn adjust_by<'a>(
    child: impl Into<Element<'a, Msg>>,
    value: f32,
    range: (f32, f32),
    step: f32,
    change: impl Fn(f32) -> Msg + 'a,
    commit: Option<Msg>,
    ring: Color,
) -> Element<'a, Msg> {
    Element::new(Keyboard {
        child: child.into(),
        action: Action::Adjust {
            value,
            range,
            step,
            change: Box::new(change),
            commit,
        },
        ring,
    })
}

/// L'enveloppe d'une commande native dont on ne connaît pas le message
/// (bouton, interrupteur, case, liste) : Entrée ou Espace lui envoient un
/// clic au centre de ses bornes (HIG ENT-01). `on` faux : la commande telle
/// quelle, hors du parcours au clavier.
pub(in crate::ui) fn click<'a>(
    child: impl Into<Element<'a, Msg>>,
    ring: Color,
    on: bool,
) -> Element<'a, Msg> {
    let child = child.into();
    if !on {
        return child;
    }
    Element::new(Keyboard {
        child,
        action: Action::Click,
        ring,
    })
}

enum Action<'a> {
    Press(Msg),
    Adjust {
        value: f32,
        range: (f32, f32),
        step: f32,
        change: Box<dyn Fn(f32) -> Msg + 'a>,
        commit: Option<Msg>,
    },
    Click,
}

struct Keyboard<'a, R = Renderer> {
    child: Element<'a, Msg, Theme, R>,
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

impl<R: renderer::Renderer> Widget<Msg, Theme, R> for Keyboard<'_, R> {
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

    fn layout(&mut self, tree: &mut Tree, renderer: &R, limits: &layout::Limits) -> layout::Node {
        self.child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &R,
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
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Msg>,
        viewport: &Rectangle,
    ) {
        // L'anneau ne sert qu'au clavier : un clic de souris, où que ce soit,
        // le retire — Espace tapé ensuite dans un champ n'active plus la
        // commande qui le portait.
        if let Event::Mouse(mouse::Event::ButtonPressed(_))
        | Event::Touch(iced::touch::Event::FingerPressed { .. }) = event
        {
            let focus = tree.state.downcast_mut::<Focus>();
            if focus.0 {
                focus.0 = false;
                shell.request_redraw();
            }
        }
        if let Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) = event
            && tree.state.downcast_ref::<Focus>().0
            && !shell.is_event_captured()
        {
            match (&self.action, key) {
                (Action::Press(message), Key::Named(Named::Enter | Named::Space)) => {
                    shell.publish(message.clone());
                    shell.capture_event();
                    return;
                }
                (Action::Click, Key::Named(Named::Enter | Named::Space)) => {
                    // Un clic au centre : appui puis relâchement, comme la
                    // souris — le bouton publie au relâchement, l'interrupteur
                    // à l'appui.
                    // Chaque événement a son propre `Shell` : l'appui capturé
                    // par le bouton ne doit pas lui masquer le relâchement.
                    let at = mouse::Cursor::Available(layout.bounds().center());
                    for e in [
                        mouse::Event::ButtonPressed(mouse::Button::Left),
                        mouse::Event::ButtonReleased(mouse::Button::Left),
                    ] {
                        let mut messages = Vec::new();
                        let mut sub = Shell::new(&mut messages);
                        self.child.as_widget_mut().update(
                            &mut tree.children[0],
                            &Event::Mouse(e),
                            layout,
                            at,
                            renderer,
                            clipboard,
                            &mut sub,
                            viewport,
                        );
                        shell.merge(sub, |m| m);
                    }
                    shell.capture_event();
                    return;
                }
                (
                    Action::Adjust {
                        value,
                        range,
                        step,
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
                        -step
                    } else {
                        *step
                    };
                    shell.publish(change((value + delta).clamp(range.0, range.1)));
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
        renderer: &mut R,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
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

    /// Le menu d'une liste, l'infobulle d'une commande : ceux de l'enfant.
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &R,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Msg, Theme, R>> {
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
        renderer: &R,
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
    use super::*;
    use iced::advanced::clipboard;
    use iced::keyboard::{Location, Modifiers, key::Code, key::Physical};

    /// Entrée sur une commande enveloppée par [`click`], avec ou sans focus :
    /// les messages qu'elle publie (moteur de rendu nul, sans fenêtre).
    fn enter(child: Element<'_, Msg, Theme, ()>, focused: bool) -> Vec<Msg> {
        let mut el: Element<'_, Msg, Theme, ()> = Element::new(Keyboard {
            child,
            action: Action::Click,
            ring: Color::WHITE,
        });
        let mut tree = Tree::new(&el);
        let limits = layout::Limits::new(Size::ZERO, Size::new(240.0, 48.0));
        let node = el.as_widget_mut().layout(&mut tree, &(), &limits);
        tree.state.downcast_mut::<Focus>().0 = focused;
        let event = Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: Key::Named(Named::Enter),
            modified_key: Key::Named(Named::Enter),
            physical_key: Physical::Code(Code::Enter),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        });
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        el.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &(),
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(Size::new(240.0, 48.0)),
        );
        drop(shell);
        messages
    }

    #[test]
    fn entree_active_un_bouton_et_un_interrupteur() {
        // HIG ENT-01 : le clic simulé active un bouton (publié au relâchement)
        // et un interrupteur (à l'appui), une fois chacun ; sans focus, rien.
        let button = || {
            iced::widget::button(iced::widget::text("Appliquer"))
                .on_press(Msg::Noop)
                .into()
        };
        let toggler = || iced::widget::toggler(false).on_toggle(|_| Msg::Noop).into();
        assert_eq!(enter(button(), true).len(), 1);
        assert_eq!(enter(toggler(), true).len(), 1);
        assert!(enter(button(), false).is_empty());
        assert!(enter(toggler(), false).is_empty());
    }

    #[test]
    fn fleches_d_un_curseur_au_pas_et_bornees() {
        // `adjust_by` : le pas du réglage, borné à son domaine.
        let arrow = |value: f32, key: Named| {
            let mut el: Element<'_, Msg, Theme, ()> = Element::new(Keyboard {
                child: iced::widget::Space::new().width(100).height(20).into(),
                action: Action::Adjust {
                    value,
                    range: (10.0, 20.0),
                    step: 4.0,
                    change: Box::new(Msg::VolumeSet),
                    commit: None,
                },
                ring: Color::WHITE,
            });
            let mut tree = Tree::new(&el);
            let limits = layout::Limits::new(Size::ZERO, Size::new(240.0, 48.0));
            let node = el.as_widget_mut().layout(&mut tree, &(), &limits);
            tree.state.downcast_mut::<Focus>().0 = true;
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            el.as_widget_mut().update(
                &mut tree,
                &Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: Key::Named(key),
                    modified_key: Key::Named(key),
                    physical_key: Physical::Code(Code::ArrowUp),
                    location: Location::Standard,
                    modifiers: Modifiers::empty(),
                    text: None,
                    repeat: false,
                }),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &(),
                &mut clipboard::Null,
                &mut shell,
                &Rectangle::with_size(Size::new(240.0, 48.0)),
            );
            drop(shell);
            match messages.as_slice() {
                [Msg::VolumeSet(v)] => Some(*v),
                _ => None,
            }
        };
        assert_eq!(arrow(12.0, Named::ArrowRight), Some(16.0));
        assert_eq!(arrow(18.0, Named::ArrowUp), Some(20.0));
        assert_eq!(arrow(12.0, Named::ArrowLeft), Some(10.0));
        assert_eq!(arrow(12.0, Named::Enter), None);
    }

    #[test]
    fn un_clic_de_souris_retire_l_anneau() {
        // L'anneau est celui du clavier : un clic, ailleurs, le retire.
        let mut el: Element<'_, Msg, Theme, ()> = Element::new(Keyboard {
            child: iced::widget::button(iced::widget::text("Appliquer"))
                .on_press(Msg::Noop)
                .into(),
            action: Action::Click,
            ring: Color::WHITE,
        });
        let mut tree = Tree::new(&el);
        let limits = layout::Limits::new(Size::ZERO, Size::new(240.0, 48.0));
        let node = el.as_widget_mut().layout(&mut tree, &(), &limits);
        tree.state.downcast_mut::<Focus>().0 = true;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        el.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(iced::Point::new(500.0, 500.0)),
            &(),
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(Size::new(240.0, 48.0)),
        );
        drop(shell);
        assert!(!tree.state.downcast_ref::<Focus>().0);
        assert!(messages.is_empty());
    }
}
