//! Jetons de conception du thème côté surfaces (MLG-267 à 273).
//!
//! Le moteur résout le verre, le contraste, la typographie, les états et le
//! mouvement du pack actif ; ils sont posés ici à chaque réponse du moteur
//! (comme la palette de [`super::tone`]) et lus par les surfaces au moment
//! de se dessiner. Les réglages de l'utilisateur (transparence réduite,
//! mouvement réduit) s'y ajoutent ; l'écran en cours de dessin donne ses
//! surcharges (`[screens]` du pack).

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use iced::Color;

use crate::config::{Effect, Place, Rgba, Theme};
use crate::theme_engine::design::{self, Design, Glass, Motion, REDUCED_OPACITY, States};

static DESIGN: RwLock<Option<Arc<Design>>> = RwLock::new(None);
/// Rapports d'échelle des rôles de texte (légende, corps, titre,
/// affichage), lus à chaque texte : copie légère du jeton.
static TYPE: RwLock<TypeScale> = RwLock::new(TypeScale::IDENTITY);
/// Courbe et durée de référence du mouvement, lues à chaque image.
static CURVE: RwLock<(design::Easing, u32, bool)> =
    RwLock::new((design::Easing::CubicOut, 180, false));
static USER_REDUCED_TRANSPARENCY: AtomicBool = AtomicBool::new(false);
static USER_REDUCED_MOTION: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Écran de la surface en cours de dessin : nom, définition.
    static SCREEN: RefCell<Option<(String, (u32, u32))>> = const { RefCell::new(None) };
    /// Échelle du texte de cet écran (`[screens].text_scale`).
    static SCREEN_TEXT: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TypeScale {
    caption: f32,
    body: f32,
    title: f32,
    display: f32,
}

impl TypeScale {
    const IDENTITY: Self = Self {
        caption: 1.0,
        body: 1.0,
        title: 1.0,
        display: 1.0,
    };
}

/// Pose les jetons du pack actif.
pub fn set(d: Design) {
    let t = &d.typography;
    let scale = TypeScale {
        caption: t.caption / design::REF_CAPTION,
        body: t.body / design::REF_BODY,
        title: t.title / design::REF_TITLE,
        display: t.display / design::REF_DISPLAY,
    };
    if let Ok(mut w) = TYPE.write() {
        *w = scale;
    }
    if let Ok(mut w) = CURVE.write() {
        *w = (d.motion.curve(), d.motion.duration, d.motion.reduced);
    }
    if let Ok(mut w) = DESIGN.write() {
        *w = Some(Arc::new(d));
    }
}

/// Avancement mis en courbe du thème (MLG-271) ; linéaire en mouvement
/// réduit.
pub fn ease(t: f32) -> f32 {
    let (curve, _, pack_reduced) =
        CURVE
            .read()
            .map(|c| *c)
            .unwrap_or((design::Easing::CubicOut, 180, false));
    if pack_reduced || USER_REDUCED_MOTION.load(Ordering::Relaxed) {
        t.clamp(0.0, 1.0)
    } else {
        curve.apply(t)
    }
}

/// Durée d'une animation de hyprdesk (`millis` sans thème) à la durée de
/// référence du thème ; en mouvement réduit, un fondu court.
pub fn duration(millis: u64) -> std::time::Duration {
    let (_, reference, pack_reduced) =
        CURVE
            .read()
            .map(|c| *c)
            .unwrap_or((design::Easing::CubicOut, 180, false));
    let ms = millis * u64::from(reference) / 180;
    let ms = if pack_reduced || USER_REDUCED_MOTION.load(Ordering::Relaxed) {
        ms.min(90)
    } else {
        ms
    };
    std::time::Duration::from_millis(ms)
}

/// Jetons en place (défauts du moteur avant la première réponse).
pub fn design() -> Arc<Design> {
    DESIGN
        .read()
        .ok()
        .and_then(|d| d.clone())
        .unwrap_or_else(|| Arc::new(Design::default()))
}

/// Réglages de l'utilisateur (Réglages → Thème → Accessibilité).
pub fn set_user(reduced_transparency: bool, reduced_motion: bool) {
    USER_REDUCED_TRANSPARENCY.store(reduced_transparency, Ordering::Relaxed);
    USER_REDUCED_MOTION.store(reduced_motion, Ordering::Relaxed);
}

/// Transparence réduite : réglage de l'utilisateur ou du pack.
pub fn reduced_transparency() -> bool {
    USER_REDUCED_TRANSPARENCY.load(Ordering::Relaxed) || design().contrast.reduced_transparency
}

/// Écran de la surface qui va se dessiner (`None` : surface sans écran
/// propre, surcharges ignorées).
pub fn enter_screen(screen: Option<(&str, (u32, u32))>) {
    let scale = screen
        .and_then(|(name, size)| design().screen(name, Some(size)).map(|s| s.text_scale))
        .unwrap_or(1.0);
    SCREEN_TEXT.with(|c| c.set(scale));
    SCREEN.with(|c| *c.borrow_mut() = screen.map(|(n, s)| (n.to_string(), s)));
}

fn with_screen<T>(f: impl FnOnce(&Design, Option<&design::Screen>) -> T) -> T {
    let d = design();
    SCREEN.with(|c| {
        let cur = c.borrow();
        let s = cur.as_ref().and_then(|(n, size)| d.screen(n, Some(*size)));
        f(&d, s)
    })
}

/// Taille de texte du thème pour une taille écrite dans l'interface : le
/// rôle se déduit de la taille (légende, corps, titre, affichage), puis
/// suit le rôle du thème et l'échelle de l'écran (MLG-270, MLG-272). Sans
/// typographie dans le pack, la taille est rendue telle quelle.
pub fn ts(reference: f32) -> f32 {
    let t = TYPE.read().map(|t| *t).unwrap_or(TypeScale::IDENTITY);
    let ratio = if reference < 12.0 {
        t.caption
    } else if reference < 15.5 {
        t.body
    } else if reference < 24.0 {
        t.title
    } else {
        t.display
    };
    (reference * ratio * SCREEN_TEXT.with(std::cell::Cell::get)).max(6.0)
}

/// Interligne du thème.
pub fn line_height() -> iced::widget::text::LineHeight {
    iced::widget::text::LineHeight::Relative(design().typography.line_height)
}

/// Graisse iced depuis une graisse numérique.
pub fn weight(w: u16) -> iced::font::Weight {
    use iced::font::Weight::*;
    match w {
        0..=149 => Thin,
        150..=249 => ExtraLight,
        250..=349 => Light,
        350..=449 => Normal,
        450..=549 => Medium,
        550..=649 => Semibold,
        650..=749 => Bold,
        750..=849 => ExtraBold,
        _ => Black,
    }
}

/// États interactifs du thème.
pub fn states() -> States {
    design().states.clone()
}

/// Mouvement effectif (mouvement réduit compris).
pub fn motion() -> Motion {
    let d = design();
    d.motion
        .effective(USER_REDUCED_MOTION.load(Ordering::Relaxed))
}

/// Verre d'une surface sur l'écran en cours de dessin, transparence réduite
/// comprise.
pub fn glass(p: Place) -> Glass {
    let g = with_screen(|d, s| {
        s.and_then(|s| s.effects.get(p.as_str()))
            .map(|sp| sp.glass.clone())
            .unwrap_or_else(|| d.glass(p))
    });
    if reduced_transparency() {
        g.reduced()
    } else {
        g
    }
}

/// Effet affiché d'une surface : celui du thème, surchargé par l'écran en
/// cours de dessin, relevé à la densité minimale des surfaces lisibles et
/// presque opaque en transparence réduite (MLG-269, MLG-272).
pub trait Shown {
    fn shown(&self, p: Place) -> Effect;
}

impl Shown for Theme {
    fn shown(&self, p: Place) -> Effect {
        let base = self.effect(p);
        let (mut e, min) = with_screen(|d, s| {
            let e = s
                .and_then(|s| s.effects.get(p.as_str()))
                .map(|sp| Effect {
                    opacity: sp.opacity,
                    blur: sp.blur,
                    adaptive: sp.adaptive,
                })
                .unwrap_or(base);
            (e, d.contrast.min_opacity)
        });
        if matches!(p, Place::Menus | Place::Launcher | Place::Settings) {
            e.opacity = e.opacity.max(min);
        }
        if reduced_transparency() {
            e.opacity = e.opacity.max(REDUCED_OPACITY);
            e.adaptive = None;
        }
        e
    }
}

/// Fond d'une surface teinté par son verre (`tint`, `tint_strength`).
pub fn tinted(bg: Rgba, p: Place) -> Rgba {
    let g = glass(p);
    match g.tint_rgba() {
        Some(t) if g.tint_strength > 0.0 => {
            let m = crate::theme_engine::color::mix(bg, t, f64::from(g.tint_strength));
            Rgba(m.0, m.1, m.2, bg.3)
        }
        _ => bg,
    }
}

/// Couleur de sélection : le slot `selection` du pack s'il en déclare un
/// distinct de l'accent (MLG-267), sinon la couleur de focus de la surface.
pub fn selection(fallback: Color) -> Color {
    match (super::tone::get("selection"), super::tone::get("accent")) {
        (Some(sel), Some(acc)) if sel != acc => {
            Color::from_rgba8(sel.0, sel.1, sel.2, f32::from(sel.3) / 255.0)
        }
        _ => fallback,
    }
}

/// Élément de liste (résultat, tuile, entrée de menu) : voile de
/// sélection, d'appui ou de survol, anneau de focus clavier sur
/// l'élément choisi au clavier, texte atténué s'il est désactivé
/// (MLG-273).
pub fn item(
    accent: Color,
    fg: Color,
    selected: bool,
    status: iced::widget::button::Status,
    radius: f32,
) -> iced::widget::button::Style {
    use iced::widget::button::Status;
    let s = states();
    let sel = selection(accent);
    let veil = if selected {
        Some(s.selected)
    } else {
        match status {
            Status::Pressed => Some(s.pressed),
            Status::Hovered => Some(s.hover),
            _ => None,
        }
    };
    iced::widget::button::Style {
        background: veil.map(|a| {
            Color {
                a: sel.a * a,
                ..sel
            }
            .into()
        }),
        border: iced::Border::default()
            .rounded(radius)
            .width(if selected { s.focus_width } else { 0.0 })
            .color(Color {
                a: sel.a * 0.6,
                ..sel
            }),
        text_color: if status == Status::Disabled {
            disabled(fg)
        } else {
            fg
        },
        ..Default::default()
    }
}

/// Voile d'un état : la couleur du texte mêlée au fond, à la part du thème.
pub fn veil(fg: Color, part: f32) -> Color {
    Color {
        a: fg.a * part.clamp(0.0, 1.0),
        ..fg
    }
}

/// Couleur d'une commande désactivée.
pub fn disabled(c: Color) -> Color {
    Color {
        a: c.a * states().disabled,
        ..c
    }
}

impl super::App {
    /// Police des chiffres qui défilent (horloges, monitoring) : la police
    /// de l'interface avec ses chiffres tabulaires si le thème les veut
    /// (MLG-270, MLG-274 : variante OpenType `tnum`, transmise par la copie
    /// embarquée d'iced_graphics).
    pub(super) fn numeric_font(&self, weight: iced::font::Weight) -> iced::Font {
        let font = self.ui_font(weight);
        if !design().typography.tabular_numbers {
            return font;
        }
        iced::Font {
            family: iced::font::Family::Name(super::intern_font(&tabular_name(self.ui_family))),
            ..font
        }
    }

    /// Écran d'une surface (bureau, barre, dock) : nom et définition
    /// logique, pour ses surcharges `[screens]`.
    pub(super) fn surface_screen(&self, id: iced::window::Id) -> Option<(&str, (u32, u32))> {
        let name = self
            .surfaces
            .get(&id)
            .or_else(|| self.bar_surfaces.get(&id).map(|s| &s.monitor))
            .or_else(|| self.launcher_surfaces.get(&id).map(|s| &s.monitor))?;
        let m = self.monitors.iter().find(|m| &m.name == name)?;
        Some((m.name.as_str(), (m.width, m.height)))
    }
}

/// Nom de famille qui demande les chiffres tabulaires.
pub fn tabular_name(family: &str) -> String {
    format!("{family}{}", iced::advanced::graphics::text::TABULAR_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chiffres_tabulaires_transmis_au_moteur_de_texte() {
        use iced::advanced::graphics::text::{cosmic_text, to_attributes};
        let name: &'static str = Box::leak(tabular_name("Mindlog").into_boxed_str());
        let attrs = to_attributes(iced::Font::with_name(name));
        assert_eq!(attrs.family, cosmic_text::Family::Name("Mindlog"));
        assert!(
            attrs
                .font_features
                .features
                .iter()
                .any(|f| f.tag.as_bytes() == b"tnum" && f.value == 1)
        );
        let plain = to_attributes(iced::Font::with_name("Mindlog"));
        assert!(plain.font_features.features.is_empty());
    }

    /// Les jetons sont globaux : un seul essai les pose, dans l'ordre.
    #[test]
    fn jetons_posés_lus_par_les_surfaces() {
        let mut d = Design::default();
        d.typography.body = 15.6;
        d.contrast.min_opacity = 0.8;
        let g = Glass {
            tint: Some("#ff0000ff".into()),
            tint_strength: 0.5,
            ..Glass::default()
        };
        d.glass.insert("bar".into(), g);
        d.screens.insert(
            "2570x720".into(),
            design::Screen {
                effects: Default::default(),
                text_scale: 0.5,
            },
        );
        set(d);
        assert!((ts(13.0) - 15.6).abs() < 1e-4);
        assert!((ts(11.0) - 11.0).abs() < 1e-4, "légende inchangée");
        enter_screen(Some(("HDMI-A-1", (2570, 720))));
        assert!((ts(13.0) - 7.8).abs() < 1e-4);
        enter_screen(None);
        let t = Theme::default();
        let launcher = t.effect(Place::Launcher);
        assert!(t.shown(Place::Launcher).opacity >= launcher.opacity.max(0.8) - 1e-6);
        assert_eq!(t.shown(Place::Bar), t.effect(Place::Bar));
        let teinte = tinted(Rgba(0, 0, 0, 200), Place::Bar);
        assert!(teinte.0 > teinte.1 + 40 && teinte.3 == 200, "{teinte:?}");
        set_user(true, true);
        assert!(t.shown(Place::Bar).opacity >= REDUCED_OPACITY);
        assert_eq!(glass(Place::Bar).highlight, 0.0);
        assert!(motion().reduced);
        set_user(false, false);
        set(Design::default());
        assert_eq!(ts(13.0), 13.0);
    }
}
