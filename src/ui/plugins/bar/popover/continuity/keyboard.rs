//! Le périmètre de focus du centre de contrôle : Tab et Maj + Tab à
//! l'intérieur, Ctrl+Tab et Ctrl+Maj + Tab de groupe de contrôles en groupe.
//! L'enveloppe des contrôles est celle du shell, `crate::ui::focus`.
use iced::advanced::widget::{self, operation};
use iced::keyboard::{Key, key::Named};
use iced::{Element, Task};

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
        Key::Named(Named::Tab) if control => widget::operate(operation::scope(
            widget::Id::new(PANE),
            crate::ui::focus::jump(&GROUPS, shift),
        )),
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

/// Un groupe de contrôles du volet, par son rang dans [`GROUPS`].
pub(super) fn group<'a>(rank: usize, child: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    crate::ui::focus::group(GROUPS[rank], child)
}
