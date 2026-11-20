//! Island : une capsule noire centrée, vide au repos comme la référence.
//! Le strip révèle les modules dans une seule capsule au survol ; une
//! notification peut occuper le centre sans dévoiler les autres commandes.
use iced::widget::{button, container};
use iced::{Background, Border, Color};

use super::{Tokens, color};
use crate::ui::plugins::bar::layout::PillSkin;
use crate::ui::plugins::bar::surface::BarPalette;

/// Survol 150 (minflair `animFast`), appui 90, commutation 180.
pub(super) const TOKENS: Tokens = Tokens {
    hover: 150,
    press: 90,
    switch: 180,
};

/// Rayon des capsules de module : presque une pilule dans la pilule.
const CAPSULE: f32 = 12.0;

/// La lame du conducteur reste transparente : la matière vit dans les
/// pilules, la fenêtre laisse voir le fond entre elles.
pub(super) fn surface(pal: &BarPalette) -> iced::widget::container::Style {
    container::Style {
        text_color: Some(color(pal.fg, 1.0)),
        ..Default::default()
    }
}

/// Les couleurs explicites de barre sont résolues par `bar_palette`.
pub(super) fn pill(pal: &BarPalette) -> PillSkin {
    PillSkin {
        fill: color(pal.bg, pal.opacity),
        border: color(pal.fg, 0.08),
        pinned: false,
        notification: false,
        duration: std::time::Duration::from_millis(180),
    }
}

/// Chrome des modules : capsules teintées par le texte du thème — survol
/// discret, marque d'activité, plus forte à l'appui (alphas de Continuité).
pub(super) fn module_button(
    alpha: f32,
    fg: Color,
    active: bool,
    st: button::Status,
) -> button::Style {
    let a = match st {
        button::Status::Pressed => 0.18,
        button::Status::Hovered => 0.08,
        _ if active => 0.12,
        _ => 0.0,
    };
    button::Style {
        background: (a > 0.0).then_some(Background::Color(Color { a: a * alpha, ..fg })),
        text_color: fg,
        border: Border::default().rounded(CAPSULE),
        ..Default::default()
    }
}
