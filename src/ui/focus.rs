//! L'enveloppe de focus du shell (HIG ENT-01, ENT-02, ENT-07, ENT-11). Les
//! boutons, interrupteurs et curseurs natifs d'Iced ne prennent pas le focus
//! au clavier : enveloppés, ils entrent dans le parcours de Tab et Maj + Tab,
//! portent l'anneau de focus, et Entrée ou Espace les activent (les flèches
//! règlent un curseur). Une seule enveloppe pour toutes les surfaces : le
//! centre de contrôle, les modules de la barre, le tiroir des Réglages.
use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::{self, Focusable, Outcome, focusable};
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
    steps(
        child,
        Steps {
            value,
            range,
            step,
            change: Box::new(change),
            commit,
            enter: None,
            id: None,
        },
        ring,
    )
}

/// Le réglage d'un curseur au clavier : sa valeur, son domaine et son pas
/// (flèches), le message de chaque valeur et celui qui l'enregistre.
pub(in crate::ui) struct Steps<'a> {
    pub value: f32,
    pub range: (f32, f32),
    pub step: f32,
    pub change: Box<dyn Fn(f32) -> Msg + 'a>,
    pub commit: Option<Msg>,
    /// Entrée ou Espace : la saisie de la valeur (HIG SEL-08) ; rien sinon.
    pub enter: Option<Box<Msg>>,
    /// Le nom du curseur aux opérations : la saisie refermée lui rend le
    /// focus (`iced::widget::operation::focus`).
    pub id: Option<widget::Id>,
}

/// L'enveloppe d'un curseur réglé par [`Steps`] : les flèches le règlent,
/// Entrée ouvre la saisie de sa valeur quand le site en a une (le curseur
/// partagé des Réglages, HIG ENT-01).
pub(in crate::ui) fn steps<'a>(
    child: impl Into<Element<'a, Msg>>,
    steps: Steps<'a>,
    ring: Color,
) -> Element<'a, Msg> {
    Element::new(Keyboard {
        child: child.into(),
        action: Action::Adjust(steps),
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
    Adjust(Steps<'a>),
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
        let id = match &self.action {
            Action::Adjust(steps) => steps.id.as_ref(),
            _ => None,
        };
        operation.focusable(id, layout.bounds(), tree.state.downcast_mut::<Focus>());
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
                    Action::Adjust(Steps {
                        enter: Some(message),
                        ..
                    }),
                    Key::Named(Named::Enter | Named::Space),
                ) => {
                    shell.publish((**message).clone());
                    shell.capture_event();
                    return;
                }
                (
                    Action::Adjust(Steps {
                        value,
                        range,
                        step,
                        change,
                        commit,
                        ..
                    }),
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
fn position(groups: &[&'static str], id: &widget::Id) -> Option<usize> {
    groups.iter().position(|g| id == &widget::Id::new(g))
}

/// Saute au premier contrôle du groupe suivant (précédent) parmi `groups`,
/// dans l'ordre de lecture : Ctrl+Tab et Ctrl+Maj + Tab (HIG ENT-01, ENT-11).
/// Deux passages, comme `focus_next` : où est le focus, puis où le poser.
pub(in crate::ui) fn jump(groups: &'static [&'static str], previous: bool) -> impl Operation<Msg> {
    struct Locate {
        groups: &'static [&'static str],
        previous: bool,
        /// Contrôles de chaque groupe, dans l'ordre de lecture.
        counts: Vec<usize>,
        current: Option<usize>,
        focused: Option<usize>,
    }

    impl Operation<Msg> for Locate {
        fn container(&mut self, id: Option<&widget::Id>, _bounds: Rectangle) {
            if let Some(i) = id.and_then(|id| position(self.groups, id)) {
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
                    groups: self.groups,
                    target: widget::Id::new(self.groups[i]),
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
        groups: &'static [&'static str],
        target: widget::Id,
        current: bool,
        done: bool,
    }

    impl Operation<Msg> for Enter {
        fn container(&mut self, id: Option<&widget::Id>, _bounds: Rectangle) {
            // Seuls les groupes comptent : une colonne, un conteneur à
            // l'intérieur d'un groupe ne le referment pas.
            if let Some(id) = id.filter(|id| position(self.groups, id).is_some()) {
                self.current = id == &self.target;
            }
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
        groups,
        previous,
        counts: Vec::new(),
        current: None,
        focused: None,
    }
}

/// Un groupe de contrôles, nommé `id` aux opérations ([`jump`]). Ne change
/// ni la mise en page ni le dessin.
pub(in crate::ui) fn group<'a>(
    id: &'static str,
    child: impl Into<Element<'a, Msg>>,
) -> Element<'a, Msg> {
    Element::new(Group {
        id: widget::Id::new(id),
        child: child.into(),
    })
}

struct Group<'a, R = Renderer> {
    id: widget::Id,
    child: Element<'a, Msg, Theme, R>,
}

impl<R: renderer::Renderer> Widget<Msg, Theme, R> for Group<'_, R> {
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
        renderer: &R,
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
        renderer: &mut R,
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

/// Le `scrollable` nommé `scroll` défile jusqu'à la commande qui a le focus
/// quand elle sort de la vue (HIG ENT-01) : Tab ne pose jamais le focus hors
/// de vue. Deux passages : où sont la vue et le focus, puis le défilement.
pub(in crate::ui) fn reveal(scroll: &'static str) -> Task<Msg> {
    widget::operate(Reveal {
        target: widget::Id::new(scroll),
        enter: false,
        inside: false,
        view: None,
        focused: None,
    })
}

/// Marge gardée entre la commande révélée et le bord de la vue.
const REVEAL_MARGIN: f32 = 12.0;

/// Décalage vertical qui montre `focused` dans la vue d'un `scrollable`
/// (`view`, son contenu `content`, défilé de `translation`) ; `None` : déjà
/// visible. Les bornes du focus sont celles du contenu non défilé.
fn revealed(
    view: Rectangle,
    content: Rectangle,
    translation: iced::Vector,
    focused: Rectangle,
) -> Option<f32> {
    let top = view.y + translation.y;
    let bottom = top + view.height;
    let y = if focused.y < top {
        focused.y - view.y - REVEAL_MARGIN
    } else if focused.y + focused.height > bottom {
        focused.y + focused.height - view.y - view.height + REVEAL_MARGIN
    } else {
        return None;
    };
    Some(y.clamp(0.0, (content.height - view.height).max(0.0)))
}

struct Reveal {
    target: widget::Id,
    /// Le `scrollable` visé vient d'être vu : son contenu est le prochain
    /// parcours.
    enter: bool,
    inside: bool,
    view: Option<(Rectangle, Rectangle, iced::Vector)>,
    focused: Option<Rectangle>,
}

impl Operation<Msg> for Reveal {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Msg>)) {
        let entering = std::mem::take(&mut self.enter);
        if entering {
            self.inside = true;
        }
        operate(self);
        if entering {
            self.inside = false;
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: iced::Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&self.target) {
            self.enter = true;
            self.view = Some((bounds, content_bounds, translation));
        }
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if self.inside && state.is_focused() {
            self.focused = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<Msg> {
        let (Some((view, content, translation)), Some(focused)) = (self.view, self.focused) else {
            return Outcome::None;
        };
        match revealed(view, content, translation, focused) {
            Some(y) => Outcome::Chain(Box::new(ScrollTo {
                target: self.target.clone(),
                y,
            })),
            None => Outcome::None,
        }
    }
}

/// Deuxième passage de [`reveal`] : le `scrollable` visé défile à `y`.
struct ScrollTo {
    target: widget::Id,
    y: f32,
}

impl Operation<Msg> for ScrollTo {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Msg>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: iced::Vector,
        state: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&self.target) {
            state.scroll_to(operation::scrollable::AbsoluteOffset {
                x: None,
                y: Some(self.y),
            });
        }
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
                action: Action::Adjust(Steps {
                    value,
                    range: (10.0, 20.0),
                    step: 4.0,
                    change: Box::new(Msg::VolumeSet),
                    commit: None,
                    enter: matches!(key, Named::Space | Named::Enter).then(|| Box::new(Msg::Noop)),
                    id: None,
                }),
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
                // Entrée (ici Espace) ouvre la saisie : -1 la signale.
                [Msg::Noop] => Some(-1.0),
                _ => None,
            }
        };
        assert_eq!(arrow(12.0, Named::ArrowRight), Some(16.0));
        assert_eq!(arrow(18.0, Named::ArrowUp), Some(20.0));
        assert_eq!(arrow(12.0, Named::ArrowLeft), Some(10.0));
        // Un site qui a une saisie de la valeur : Entrée ou Espace l'ouvre.
        assert_eq!(arrow(12.0, Named::Enter), Some(-1.0));
        assert_eq!(arrow(12.0, Named::Space), Some(-1.0));
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

    #[test]
    fn le_focus_hors_de_vue_fait_defiler() {
        // Vue de 100 px en haut d'un contenu de 400 px.
        let view = Rectangle::new(iced::Point::new(0.0, 50.0), Size::new(200.0, 100.0));
        let content = Rectangle::new(iced::Point::new(0.0, 50.0), Size::new(200.0, 400.0));
        let at = |y: f32| Rectangle::new(iced::Point::new(0.0, y), Size::new(80.0, 20.0));
        let none = iced::Vector::ZERO;
        // Visible : rien ne bouge.
        assert_eq!(revealed(view, content, none, at(60.0)), None);
        // Sous la vue : son bas affleure, marge comprise.
        assert_eq!(revealed(view, content, none, at(200.0)), Some(82.0));
        // Défilé de 200 px, le focus au-dessus : son haut, marge comprise.
        let down = iced::Vector::new(0.0, 200.0);
        assert_eq!(revealed(view, content, down, at(100.0)), Some(38.0));
        // Jamais au-delà du contenu, ni avant son début.
        assert_eq!(revealed(view, content, none, at(440.0)), Some(300.0));
        assert_eq!(revealed(view, content, down, at(52.0)), Some(0.0));
    }

    /// Les opérations d'un arbre de moteur nul, enchaînements compris.
    fn run(
        el: &mut Element<'_, Msg, Theme, ()>,
        tree: &mut Tree,
        op: impl Operation<Msg> + 'static,
    ) {
        let limits = layout::Limits::new(Size::ZERO, Size::new(240.0, 480.0));
        let node = el.as_widget_mut().layout(tree, &(), &limits);
        let mut op: Box<dyn Operation<Msg>> = Box::new(op);
        loop {
            el.as_widget_mut().operate(
                tree,
                Layout::new(&node),
                &(),
                &mut operation::black_box(op.as_mut()),
            );
            match op.finish() {
                Outcome::Chain(next) => op = next,
                _ => break,
            }
        }
    }

    /// La commande enveloppée qui a le focus : son rang parmi `focused`.
    fn focused(tree: &Tree) -> Vec<bool> {
        let mut out = Vec::new();
        fn walk(t: &Tree, out: &mut Vec<bool>) {
            if t.tag == tree::Tag::of::<Focus>() {
                out.push(t.state.downcast_ref::<Focus>().0);
            }
            for c in &t.children {
                walk(c, out);
            }
        }
        walk(tree, &mut out);
        out
    }

    #[test]
    fn tab_puis_ctrl_tab_de_groupe_en_groupe() {
        // HIG ENT-01 : deux Tab vont à la deuxième commande ; Ctrl+Tab au
        // premier contrôle du groupe suivant, la boucle refermée.
        const GROUPS: [&str; 2] = ["essai-nav", "essai-contenu"];
        let command = || -> Element<'static, Msg, Theme, ()> {
            Element::new(Keyboard {
                child: iced::widget::button(iced::widget::text("Ouvrir"))
                    .on_press(Msg::Noop)
                    .into(),
                action: Action::Click,
                ring: Color::WHITE,
            })
        };
        let group = |id: &'static str, n: usize| -> Element<'static, Msg, Theme, ()> {
            Element::new(Group {
                id: widget::Id::new(id),
                child: iced::widget::Column::with_children((0..n).map(|_| command())).into(),
            })
        };
        let mut el: Element<'_, Msg, Theme, ()> =
            iced::widget::column![group(GROUPS[0], 2), group(GROUPS[1], 2)].into();
        let mut tree = Tree::new(&el);
        run(&mut el, &mut tree, focusable::focus_next());
        run(&mut el, &mut tree, focusable::focus_next());
        assert_eq!(focused(&tree), [false, true, false, false]);
        run(&mut el, &mut tree, jump(&GROUPS, false));
        assert_eq!(focused(&tree), [false, false, true, false]);
        run(&mut el, &mut tree, jump(&GROUPS, false));
        assert_eq!(focused(&tree), [true, false, false, false]);
        run(&mut el, &mut tree, jump(&GROUPS, true));
        assert_eq!(focused(&tree), [false, false, true, false]);
    }
}
