//! Texte utile en lecture seule : sélectionnable et copiable (HIG LAY-05).
//!
//! Un `text` ne se sélectionne pas : ni glissé, ni double clic, ni Ctrl+C.
//! Sous Iced 0.14, seul `text_editor` sait sélectionner, et sa sélection vit
//! dans un `Content` que la vue ne peut ni créer ni garder (elle emprunte
//! `&self`). `selectable` garde donc ce contenu dans l'arbre des widgets, à
//! la place du texte : une surface n'a ni état à porter ni message à traiter.
//!
//! Lecture seule : les actions d'édition de l'éditeur sont reçues puis
//! ignorées. Copier est fait par l'éditeur lui-même (Ctrl+C, Ctrl+X, menu
//! d'édition de `edit.rs`). La molette lui échappe : le conteneur défile.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer};
use iced::widget::text_editor::{self, Action};
use iced::{Border, Color, Element, Event, Font, Length, Pixels, Rectangle, Renderer, Size, Theme};

/// Action de l'éditeur interne : traitée ici, jamais renvoyée à l'application.
#[derive(Debug, Clone)]
struct Act(Action);

/// État d'un texte dans l'arbre : sa sélection, et l'état de son éditeur.
struct State {
    content: text_editor::Content,
    editor: Tree,
}

impl State {
    /// Remplace l'état quand le texte a changé : la sélection s'efface.
    fn sync(&mut self, text: &str, fresh: impl FnOnce() -> Self) {
        if self.content.text() != text {
            *self = fresh();
        }
    }

    /// Joue une action de l'éditeur : sélection et déplacement seulement.
    fn apply(&mut self, action: Action) {
        // La molette déplace le curseur d'un éditeur : sans objet ici, et le
        // conteneur doit continuer de défiler.
        if action.is_edit() || matches!(action, Action::Scroll { .. }) {
            return;
        }
        self.content.perform(action);
    }
}

/// Texte statique, sélectionnable et copiable, en lecture seule.
pub struct Selectable {
    text: String,
    font: Option<Font>,
    size: Option<Pixels>,
    color: Option<Color>,
}

/// Un texte utile — chemin, adresse, avertissement, erreur — que l'on peut
/// sélectionner et copier. Se pare comme `text` : `.font`, `.size`, `.color`.
pub(in crate::ui) fn selectable(text: impl Into<String>) -> Selectable {
    Selectable {
        text: text.into(),
        font: None,
        size: None,
        color: None,
    }
}

/// Couleur de sélection : le slot `selection` de la palette, sinon l'accent.
fn selection() -> Color {
    let a = super::tone::rgba("accent");
    super::tokens::selection(Color::from_rgba8(a.0, a.1, a.2, f32::from(a.3) / 255.0))
}

impl Selectable {
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// L'éditeur interne, paré comme le texte qu'il remplace : marge nulle,
    /// fond transparent, sans bord.
    fn editor<'a>(&self, content: &'a text_editor::Content) -> Element<'a, Act> {
        let mut editor = iced::widget::text_editor(content).padding(0).on_action(Act);
        if let Some(font) = self.font {
            editor = editor.font(font);
        }
        if let Some(size) = self.size {
            editor = editor.size(size);
        }
        let color = self.color;
        editor
            .style(move |theme: &Theme, status| {
                let base = text_editor::default(theme, status);
                text_editor::Style {
                    background: Color::TRANSPARENT.into(),
                    border: Border::default(),
                    value: color.unwrap_or(base.value),
                    selection: selection(),
                    ..base
                }
            })
            .into()
    }

    /// L'état d'un texte : son contenu et celui de l'éditeur qui le montre.
    fn make_state(&self) -> State {
        let content = text_editor::Content::with_text(&self.text);
        let editor = {
            let element = self.editor(&content);
            let widget = element.as_widget();
            Tree {
                tag: widget.tag(),
                state: widget.state(),
                children: widget.children(),
            }
        };
        State { content, editor }
    }

    /// La place que prend le texte, posé en `text` dans les mêmes limites.
    fn text_size(&self, renderer: &Renderer, limits: &layout::Limits) -> Size {
        let mut probe = iced::widget::text(self.text.as_str());
        if let Some(font) = self.font {
            probe = probe.font(font);
        }
        if let Some(size) = self.size {
            probe = probe.size(size);
        }
        let mut element: Element<'_, Act> = probe.into();
        let widget = element.as_widget();
        let mut tree = Tree {
            tag: widget.tag(),
            state: widget.state(),
            children: widget.children(),
        };
        element
            .as_widget_mut()
            .layout(&mut tree, renderer, limits)
            .size()
    }

    fn with_editor<T>(&self, tree: &Tree, f: impl FnOnce(&Element<'_, Act>, &Tree) -> T) -> T {
        let state = tree.state.downcast_ref::<State>();
        let element = self.editor(&state.content);
        f(&element, &state.editor)
    }

    fn with_editor_mut<T>(
        &self,
        tree: &mut Tree,
        f: impl FnOnce(&mut Element<'_, Act>, &mut Tree) -> T,
    ) -> T {
        let state = tree.state.downcast_mut::<State>();
        let mut element = self.editor(&state.content);
        f(&mut element, &mut state.editor)
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Selectable {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(self.make_state())
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        let text = &self.text;
        state.sync(text, || self.make_state());
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self.with_editor_mut(tree, |editor, inner| {
            editor.as_widget_mut().layout(inner, renderer, limits)
        });
        // L'éditeur occupe toute la largeur offerte, comme un champ : on rend
        // au texte la place qu'il prendrait en `text`.
        let width = self.text_size(renderer, limits).width;
        layout::Node::with_children(
            Size::new(width, node.size().height),
            node.children().to_vec(),
        )
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let mut actions = Vec::new();
        let mut local = Shell::new(&mut actions);
        self.with_editor_mut(tree, |editor, inner| {
            editor.as_widget_mut().update(
                inner, event, layout, cursor, renderer, clipboard, &mut local, viewport,
            );
        });

        // La molette survolée est prise par l'éditeur : elle est rendue au
        // conteneur, qui doit défiler.
        let wheel = matches!(event, Event::Mouse(mouse::Event::WheelScrolled { .. }));
        if local.is_event_captured() && !wheel {
            shell.capture_event();
        }
        local.revalidate_layout(|| shell.invalidate_layout());
        shell.request_redraw_at(local.redraw_request());
        shell.request_input_method(local.input_method());

        if !actions.is_empty() {
            let state = tree.state.downcast_mut::<State>();
            for Act(action) in actions {
                state.apply(action);
            }
            shell.invalidate_layout();
            shell.request_redraw();
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.with_editor_mut(tree, |editor, inner| {
            editor
                .as_widget_mut()
                .operate(inner, layout, renderer, operation)
        });
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
        self.with_editor(tree, |editor, inner| {
            editor
                .as_widget()
                .draw(inner, renderer, theme, style, layout, cursor, viewport)
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.with_editor(tree, |editor, inner| {
            editor
                .as_widget()
                .mouse_interaction(inner, layout, cursor, viewport, renderer)
        })
    }
}

impl<'a, Message: 'a> From<Selectable> for Element<'a, Message> {
    fn from(selectable: Selectable) -> Self {
        Element::new(selectable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(text: &str) -> State {
        selectable(text).make_state()
    }

    #[test]
    fn edition_ignoree() {
        let mut s = state("/tmp/hyprdesk-avant.lua");
        s.apply(Action::Edit(text_editor::Edit::Insert('x')));
        s.apply(Action::Edit(text_editor::Edit::Enter));
        assert_eq!(s.content.text(), "/tmp/hyprdesk-avant.lua");
        assert!(s.content.selection().is_none());
    }

    #[test]
    fn tout_selectionner() {
        let mut s = state("/tmp/hyprdesk-avant.lua");
        s.apply(Action::SelectAll);
        assert_eq!(
            s.content.selection().as_deref(),
            Some("/tmp/hyprdesk-avant.lua")
        );
    }

    #[test]
    fn texte_change_efface_la_selection() {
        let mut s = state("ancien");
        s.apply(Action::SelectAll);
        s.sync("ancien", || state("ancien"));
        assert!(
            s.content.selection().is_some(),
            "même texte : rien ne bouge"
        );
        s.sync("nouveau", || state("nouveau"));
        assert_eq!(s.content.text(), "nouveau");
        assert!(s.content.selection().is_none());
    }

    #[test]
    fn molette_ignoree() {
        let mut s = state("lignes 1, 5");
        s.apply(Action::SelectAll);
        s.apply(Action::Scroll { lines: 3 });
        assert_eq!(s.content.selection().as_deref(), Some("lignes 1, 5"));
    }
}
