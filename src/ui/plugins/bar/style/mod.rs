//! Styles visuels de la barre : un style est un rendu complet — surface,
//! chrome des modules, touches d'espace et durées de mouvement — porté par un
//! module de ce répertoire et choisi par le réglage `hd.bar({ style = … })`.
//!
//! Le conducteur commun reste `ui::plugins::bar::surface` (zones, mesure,
//! débordement, popovers, actions) ; un style ne décide que de l'apparence et
//! du mouvement.
//!
//! Pour ajouter un style :
//! 1. une variante dans `config::BarStyle` (avec `BAR_STYLES`, ses défauts de
//!    géométrie, ses chaînes dans `locales/en.toml`, son choix CLI et Réglages) ;
//! 2. un module ici, qui définit ses `TOKENS` et implémente `surface`,
//!    `module_button` et `workspaces` ;
//! 3. le brancher dans les dispatchs de ce fichier, et une scène jumelle dans
//!    `ui::preview` + `scripts/verify.sh`.
use std::time::Duration;

use iced::widget::{container, text, tooltip};
use iced::{Color, Element};

use crate::config::{BarPosition, BarStyle};
use crate::ui::plugins::bar::motion;
use crate::ui::plugins::bar::surface::{BarPalette, BarSurface, URGENT_ICON};
use crate::ui::{App, Msg};

pub(in crate::ui) mod continuite;
pub(in crate::ui) mod convexe;
mod graphite;
mod island;

mod selector;

/// Durées de mouvement d'un style, en millisecondes (le coupe-circuit
/// `transition = "none"` s'applique toujours par-dessus, via `bar_duration`).
#[derive(Clone, Copy)]
pub(in crate::ui) struct Tokens {
    /// Capsule de survol d'un module, ancrage des menus.
    pub hover: u64,
    /// Compression à l'appui.
    pub press: u64,
    /// Changement d'espace : capsule glissante ou touche qui se soulève.
    pub switch: u64,
}

const fn tokens(style: BarStyle) -> Tokens {
    match style {
        BarStyle::Continuite => continuite::TOKENS,
        BarStyle::Graphite => graphite::TOKENS,
        BarStyle::Island => island::TOKENS,
        BarStyle::Convexe => convexe::TOKENS,
    }
}

/// Jetons du style courant de la barre, pour les sites communs de `ui::plugins::bar::surface`.
pub(in crate::ui) fn durations(app: &App) -> Tokens {
    tokens(app.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default())
}

/// Fond de la barre : la lame du style. Island et Convex rendent une lame
/// transparente — leur silhouette vit dans le fond de zones ci-dessous.
pub(in crate::ui) fn surface(
    style: BarStyle,
    pal: &BarPalette,
    top: bool,
) -> iced::widget::container::Style {
    match style {
        BarStyle::Continuite => continuite::surface(pal, top),
        BarStyle::Graphite => graphite::surface(pal, top),
        BarStyle::Island => island::surface(pal),
        BarStyle::Convexe => convexe::surface(pal),
    }
}

/// Fond de zones d’un style — le contrat commun : le conducteur mesure
/// l’étendue horizontale de chaque zone (`Msg::BarLayout`) et le style peint
/// sa silhouette derrière les modules, aux limites mesurées.
/// Island peint sa capsule centrale dans le strip (`PillSkin`, quads du
/// renderer : rayon plein et ombre portée). Convex rend un canvas à niches
/// posé sous le strip ; un changement d’étendues le repositionne à l’image
/// suivante. Continuité peint trois fonds à partir des mêmes mesures.
pub(in crate::ui) fn zone_pill(
    style: BarStyle,
    pal: &BarPalette,
) -> Option<crate::ui::plugins::bar::layout::PillSkin> {
    match style {
        BarStyle::Island => Some(island::pill(pal)),
        _ => None,
    }
}

/// Fond des groupes mesurés ou des niches, derrière le rail de survol.
pub(in crate::ui) fn zone_underlay<'a>(
    style: BarStyle,
    pal: &BarPalette,
    height: f32,
    extents: [(f32, f32); 3],
    top: bool,
) -> Option<Element<'a, Msg>> {
    match style {
        BarStyle::Continuite => Some(continuite::groups(pal, height, extents)),
        BarStyle::Convexe => Some(convexe::underlay(pal, height, extents, top)),
        _ => None,
    }
}

/// Chrome d’un bouton de module (survol, appui, module actif).
pub(in crate::ui) fn module_button(
    style: BarStyle,
    pal: &BarPalette,
    alpha: f32,
    active: bool,
    st: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let fg = color(pal.fg, alpha);
    let bg = color(pal.bg, 1.0);
    match style {
        BarStyle::Continuite => continuite::module_button(pal, alpha, active, st),
        BarStyle::Graphite => graphite::module_button(alpha, fg, bg, active, st),
        BarStyle::Island => island::module_button(alpha, fg, active, st),
        BarStyle::Convexe => convexe::module_button(alpha, fg, bg, active, st),
    }
}

/// Aspect de la capsule qui suit le module survolé autour du ruban entier.
pub(in crate::ui) fn rail_look(style: BarStyle, pal: &BarPalette) -> motion::RailLook {
    match style {
        // Plus claire que la lame (0,16) : le survol éclaircit, comme macOS.
        BarStyle::Continuite => motion::RailLook::Fill(continuite::tone(pal, 0.28, 1.0)),
        BarStyle::Graphite => motion::RailLook::Fill(color(pal.fg, 0.07)),
        BarStyle::Island => motion::RailLook::Fill(color(pal.fg, 0.08)),
        BarStyle::Convexe => motion::RailLook::Fill(color(pal.fg, 0.07)),
    }
}

/// Encre d’une sélection : accent du thème dans tous les styles.
pub(in crate::ui) fn ink(style: BarStyle, pal: &BarPalette) -> Color {
    match style {
        BarStyle::Continuite => color(pal.accent, 1.0),
        BarStyle::Graphite => color(pal.accent, 1.0),
        BarStyle::Island => color(pal.accent, 1.0),
        BarStyle::Convexe => color(pal.accent, 1.0),
    }
}

/// Une ligne d'espaces de travail pour le style : nom, fenêtres posées,
/// fenêtre demandant l'attention.
pub(in crate::ui) struct WsRow {
    pub name: String,
    pub id: i64,
    pub windows: u32,
    pub urgent: bool,
}

/// Tout ce qu'un style sait des espaces de l'écran de sa barre.
pub(in crate::ui) struct WsCtx {
    pub monitor: String,
    /// Espace actif de cet écran.
    pub current: String,
    /// Mode compact avec plus de six espaces : l'actif seul, puis « … ».
    pub folded: bool,
    pub rows: Vec<WsRow>,
}

pub(in crate::ui) fn ws_ctx(app: &App, bar: &BarSurface) -> WsCtx {
    let monitor = bar.monitor.clone();
    let current = app
        .monitors
        .iter()
        .find(|m| m.name == monitor)
        .map(|m| m.workspace.clone())
        .unwrap_or_default();
    let all: Vec<_> = app
        .workspaces
        .iter()
        .filter(|w| w.monitor == monitor)
        .collect();
    let folded = app.bar_compact(&monitor) && all.len() > 6;
    let rows = all
        .iter()
        .filter(|w| !folded || w.name == current)
        .map(|w| WsRow {
            name: w.name.clone(),
            id: w.id,
            windows: w.windows,
            urgent: app.bar_urgent(&monitor, &w.name),
        })
        .collect();
    WsCtx {
        monitor,
        current,
        folded,
        rows,
    }
}

/// Le groupe des espaces de travail : le mode choisi du sélecteur décide du
/// rendu des touches — « relief » et « rail » reprennent les rendus natifs des
/// styles (touche soulevée en Graphite, capsule glissante en Continuité),
/// « miniatures » et « icônes » montrent le contenu des espaces.
pub(in crate::ui) fn workspaces<'a>(
    app: &'a App,
    bar_id: iced::window::Id,
    bar: &'a BarSurface,
    alpha: f32,
) -> Element<'a, Msg> {
    let ctx = ws_ctx(app, bar);
    let selector = app.cfg.general.workspace_selector;
    match selector {
        crate::config::WsSelector::Relief => graphite::workspaces(app, bar_id, bar, &ctx, alpha),
        crate::config::WsSelector::Rail => continuite::workspaces(app, bar_id, bar, &ctx, alpha),
        crate::config::WsSelector::Miniatures => {
            selector::miniatures(app, bar_id, bar, &ctx, alpha)
        }
        crate::config::WsSelector::Icones => selector::icones(app, bar_id, bar, &ctx, alpha),
        crate::config::WsSelector::Numeros => {
            selector::compact(app, bar_id, bar, &ctx, alpha, false)
        }
        crate::config::WsSelector::Points => selector::compact(app, bar_id, bar, &ctx, alpha, true),
    }
}

/// Petit losange « fenêtre demandant l'attention », avec son libellé
/// accessible en info-bulle.
pub(in crate::ui) fn urgent_mark<'a>(
    app: &'a App,
    bar: &'a BarSurface,
    size: f32,
    c: Color,
) -> Option<Element<'a, Msg>> {
    let glyph = app.icon_named(URGENT_ICON, size, c)?;
    let pal = app.bar_palette();
    let bg = color(pal.bg, 0.96);
    let fg = color(pal.fg, 1.0);
    Some(
        tooltip(
            glyph,
            container(
                text(crate::i18n::tr("Fenêtre nécessitant votre attention"))
                    .font(app.font)
                    .size(crate::ui::tokens::ts(12.0)),
            )
            .padding([4, 8])
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: iced::Border::default()
                    .rounded(7)
                    .width(1)
                    .color(Color { a: 0.12, ..fg }),
                text_color: Some(fg),
                ..Default::default()
            }),
            if bar.position == BarPosition::Top {
                tooltip::Position::Bottom
            } else {
                tooltip::Position::Top
            },
        )
        .gap(5)
        .delay(Duration::from_millis(200))
        .into(),
    )
}

/// Couleur pratique : Rgba du thème → iced, alpha appliqué.
pub(in crate::ui) fn color(c: crate::config::Rgba, a: f32) -> Color {
    crate::ui::color(c, a)
}
