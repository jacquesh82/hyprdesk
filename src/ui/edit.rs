//! Menu d'édition des champs de saisie (MLG-309) : un clic droit dans un
//! champ propose Couper, Copier, Coller, Tout sélectionner (Apple HIG, « Edit
//! menus »), sur toutes les surfaces de hyprdesk.
//!
//! `zone` enveloppe la vue de chaque surface. Un clic droit au-dessus d'un
//! champ (`text_input`, `text_editor`) lui donne le focus, sonde sa sélection
//! (un Ctrl+C rejoué vers un presse-papiers à blanc : rien d'écrit, rien à
//! copier — ni dans un champ masqué) et demande le menu, dessiné par le
//! composant commun (`super::context`). La commande choisie revient par une
//! opération sur la zone de la surface ; elle est rejouée au prochain
//! événement comme le raccourci clavier du champ (Ctrl+X, Ctrl+C, Ctrl+V,
//! Ctrl+A) : le champ fait lui-même le travail, avec ses propres règles.

use iced::advanced::widget::{Operation, Tree, operation, tree};
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, clipboard, layout, mouse, overlay, renderer,
};
use iced::keyboard::{self, Modifiers, key};
use iced::window::Id;
use iced::{Element, Event, Length, Point, Rectangle, Size, Task, Vector};

use super::Msg;

/// Commande du menu d'édition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Cut,
    Copy,
    Paste,
    SelectAll,
}

impl Command {
    /// Touche du raccourci (avec Ctrl).
    fn key(self) -> (&'static str, key::Code) {
        match self {
            Command::Cut => ("x", key::Code::KeyX),
            Command::Copy => ("c", key::Code::KeyC),
            Command::Paste => ("v", key::Code::KeyV),
            Command::SelectAll => ("a", key::Code::KeyA),
        }
    }

    /// Le raccourci, tel que le champ le reçoit, puis les modificateurs rendus.
    fn events(self, restore: Modifiers) -> [Event; 4] {
        let (c, code) = self.key();
        let k = keyboard::Key::Character(c.into());
        let physical = key::Physical::Code(code);
        [
            Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::CTRL)),
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: k.clone(),
                modified_key: k.clone(),
                physical_key: physical,
                location: keyboard::Location::Standard,
                modifiers: Modifiers::CTRL,
                text: None,
                repeat: false,
            }),
            Event::Keyboard(keyboard::Event::KeyReleased {
                key: k.clone(),
                modified_key: k,
                physical_key: physical,
                location: keyboard::Location::Standard,
                modifiers: Modifiers::CTRL,
            }),
            Event::Keyboard(keyboard::Event::ModifiersChanged(restore)),
        ]
    }
}

/// Enveloppe la vue d'une surface : clic droit dans un champ → menu d'édition.
pub(super) fn zone<'a>(content: Element<'a, Msg>, surface: Id) -> Element<'a, Msg> {
    Element::new(Zone { content, surface })
}

/// Fait jouer `cmd` au champ qui a le focus dans la surface `surface`.
pub(super) fn run(surface: Id, cmd: Command) -> Task<Msg> {
    iced::advanced::widget::operate(Deliver { surface, cmd })
        .discard()
        // Un message de plus : la surface est redessinée, la zone reçoit un
        // événement et rejoue la commande.
        .chain(Task::done(Msg::Noop))
}

/// Le presse-papiers contient du texte (`wl-paste --list-types`).
pub(super) fn clipboard_has_text() -> bool {
    std::process::Command::new("wl-paste")
        .args(["--no-newline", "--list-types"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some_and(|o| has_text(&String::from_utf8_lossy(&o.stdout)))
}

/// Types MIME annoncés : du texte parmi eux.
fn has_text(types: &str) -> bool {
    types.lines().map(str::trim).any(|t| {
        t.starts_with("text/") || matches!(t, "UTF8_STRING" | "STRING" | "TEXT" | "COMPOUND_TEXT")
    })
}

struct Zone<'a> {
    content: Element<'a, Msg>,
    surface: Id,
}

#[derive(Default)]
struct State {
    /// Surface de la zone, pour que l'opération reconnaisse la sienne.
    surface: Option<Id>,
    /// Commande reçue, à rejouer au prochain événement.
    pending: Option<Command>,
    /// Modificateurs réellement tenus, rendus après une commande rejouée.
    modifiers: Modifiers,
}

/// Opération : dépose la commande dans la zone de la surface visée.
struct Deliver {
    surface: Id,
    cmd: Command,
}

impl Operation for Deliver {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn custom(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn std::any::Any,
    ) {
        if let Some(s) = state.downcast_mut::<State>()
            && s.surface == Some(self.surface)
        {
            s.pending = Some(self.cmd);
        }
    }
}

/// Opération : le champ sous le point, compté dans l'ordre des champs.
/// Le contenu d'une zone défilante est décalé de son défilement et rogné à
/// sa zone visible.
struct Find {
    at: Point,
    /// Décalage des défilements englobants ; celui du prochain parcours.
    offset: Vector,
    next: Vector,
    clip: Option<Rectangle>,
    next_clip: Option<Rectangle>,
    index: usize,
    hit: Option<usize>,
}

impl Find {
    fn new(at: Point) -> Self {
        Self {
            at,
            offset: Vector::ZERO,
            next: Vector::ZERO,
            clip: None,
            next_clip: None,
            index: 0,
            hit: None,
        }
    }
}

impl Operation for Find {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        let (offset, clip) = (self.offset, self.clip);
        self.offset += std::mem::take(&mut self.next);
        if let Some(c) = self.next_clip.take() {
            self.clip = Some(
                self.clip
                    .map_or(c, |o| o.intersection(&c).unwrap_or_default()),
            );
        }
        operate(self);
        self.offset = offset;
        self.clip = clip;
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        self.next = translation;
        self.next_clip = Some(bounds - self.offset);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _state: &mut dyn operation::Focusable,
    ) {
        let shown = bounds - self.offset;
        if shown.contains(self.at) && self.clip.is_none_or(|c| c.contains(self.at)) {
            self.hit = Some(self.index);
        }
        self.index += 1;
    }
}

/// Opération : le champ `target` (rang de `Find`) prend le focus, les autres
/// le perdent. Un champ qui l'a déjà le garde, sélection comprise.
struct FocusOnly {
    target: usize,
    index: usize,
}

impl Operation for FocusOnly {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if self.index == self.target {
            if !state.is_focused() {
                state.focus();
            }
        } else if state.is_focused() {
            state.unfocus();
        }
        self.index += 1;
    }
}

/// Presse-papiers à blanc : retient ce que le champ y écrirait.
#[derive(Default)]
struct Probe {
    written: bool,
}

impl Clipboard for Probe {
    fn read(&self, _kind: clipboard::Kind) -> Option<String> {
        None
    }

    fn write(&mut self, _kind: clipboard::Kind, contents: String) {
        self.written |= !contents.is_empty();
    }
}

impl Zone<'_> {
    /// Rejoue `cmd` (raccourci clavier) au contenu de la zone.
    #[allow(clippy::too_many_arguments)]
    fn replay(
        &mut self,
        tree: &mut Tree,
        cmd: Command,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Msg>,
        viewport: &Rectangle,
    ) {
        let restore = tree.state.downcast_ref::<State>().modifiers;
        for e in cmd.events(restore) {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                &e,
                layout,
                mouse::Cursor::Unavailable,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Zone<'_> {
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

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
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
        op: &mut dyn Operation,
    ) {
        let s = tree.state.downcast_mut::<State>();
        s.surface = Some(self.surface);
        op.custom(None, layout.bounds(), s);
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
        let s = tree.state.downcast_mut::<State>();
        if let Event::Keyboard(keyboard::Event::ModifiersChanged(m)) = event {
            s.modifiers = *m;
        }
        if let Some(cmd) = s.pending.take() {
            self.replay(tree, cmd, layout, renderer, clipboard, shell, viewport);
            shell.request_redraw();
        }
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event
            && let Some(at) = cursor.position_over(layout.bounds())
        {
            let mut find = Find::new(at);
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                &mut find,
            );
            if let Some(target) = find.hit {
                let mut focus = FocusOnly { target, index: 0 };
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    &mut focus,
                );
                // Sonde : y a-t-il une sélection à copier ?
                let mut probe = Probe::default();
                self.replay(
                    tree,
                    Command::Copy,
                    layout,
                    renderer,
                    &mut probe,
                    shell,
                    viewport,
                );
                shell.publish(Msg::EditMenu(self.surface, probe.written));
                shell.request_redraw();
                shell.capture_event();
                return;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presse_papiers_texte() {
        assert!(has_text("image/png\ntext/plain;charset=utf-8\n"));
        assert!(has_text("UTF8_STRING"));
        assert!(!has_text("image/png\nx-special/gnome-copied-files"));
        assert!(!has_text(""));
    }

    #[test]
    fn raccourci_rejoue_puis_modificateurs_rendus() {
        let ev = Command::Paste.events(Modifiers::SHIFT);
        assert!(matches!(
            &ev[1],
            Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Character(c), modifiers, .. })
                if c.as_str() == "v" && modifiers.command()
        ));
        assert!(matches!(
            ev[3],
            Event::Keyboard(keyboard::Event::ModifiersChanged(m)) if m == Modifiers::SHIFT
        ));
    }

    /// Champ sous le point : dans une zone défilée, à sa place affichée.
    #[test]
    fn champ_trouve_dans_une_zone_defilee() {
        struct Dummy;
        impl operation::Focusable for Dummy {
            fn is_focused(&self) -> bool {
                false
            }
            fn focus(&mut self) {}
            fn unfocus(&mut self) {}
        }
        struct Scroll;
        impl operation::Scrollable for Scroll {
            fn snap_to(&mut self, _: operation::scrollable::RelativeOffset<Option<f32>>) {}
            fn scroll_to(&mut self, _: operation::scrollable::AbsoluteOffset<Option<f32>>) {}
            fn scroll_by(
                &mut self,
                _: operation::scrollable::AbsoluteOffset,
                _: Rectangle,
                _: Rectangle,
            ) {
            }
        }
        let field = |y: f32| Rectangle::new(Point::new(10.0, y), Size::new(200.0, 30.0));
        let mut find = Find::new(Point::new(50.0, 115.0));
        find.focusable(None, field(10.0), &mut Dummy);
        find.scrollable(
            None,
            Rectangle::new(Point::new(0.0, 100.0), Size::new(300.0, 200.0)),
            Rectangle::new(Point::new(0.0, 100.0), Size::new(300.0, 900.0)),
            Vector::new(0.0, 400.0),
            &mut Scroll,
        );
        find.traverse(&mut |op| {
            // Champ à y = 100 dans le contenu : défilé hors de la vue.
            op.focusable(None, field(100.0), &mut Dummy);
            // Champ à y = 500 : affiché à y = 100, sous le point.
            op.focusable(None, field(500.0), &mut Dummy);
        });
        assert_eq!(find.hit, Some(2));
        // Hors de la zone défilante, le décalage ne s'applique plus.
        find.focusable(None, field(100.0), &mut Dummy);
        assert_eq!(find.hit, Some(3));
    }
}
