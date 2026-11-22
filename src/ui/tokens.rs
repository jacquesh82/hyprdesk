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
static USER_INCREASE_CONTRAST: AtomicBool = AtomicBool::new(false);
/// Échelle du texte de l'utilisateur (bits d'un f32, 1,0 au départ).
static USER_TEXT_SCALE: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0x3f80_0000);
static RAW_PALETTE: RwLock<Option<std::collections::BTreeMap<String, Rgba>>> = RwLock::new(None);
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
/// Renvoie `true` si le contraste renforcé a changé (la palette a été
/// reposée : le thème iced est à refaire).
pub fn set_user(
    reduced_transparency: bool,
    reduced_motion: bool,
    increase_contrast: bool,
    text_scale: f32,
) -> bool {
    USER_REDUCED_TRANSPARENCY.store(reduced_transparency, Ordering::Relaxed);
    USER_REDUCED_MOTION.store(reduced_motion, Ordering::Relaxed);
    USER_TEXT_SCALE.store(text_scale.clamp(0.75, 2.0).to_bits(), Ordering::Relaxed);
    let changed =
        USER_INCREASE_CONTRAST.swap(increase_contrast, Ordering::Relaxed) != increase_contrast;
    if changed
        && let Ok(raw) = RAW_PALETTE.read()
        && let Some(p) = raw.clone()
    {
        super::tone::set(adjusted(p));
    }
    changed
}

/// Contraste renforcé (MLG-282) : réglage de l'utilisateur.
pub fn increase_contrast() -> bool {
    USER_INCREASE_CONTRAST.load(Ordering::Relaxed)
}

/// Palette du thème, telle que le moteur la résout ; les surfaces la
/// reçoivent corrigée au contraste renforcé s'il est demandé.
pub fn set_palette(p: std::collections::BTreeMap<String, Rgba>) {
    if let Ok(mut raw) = RAW_PALETTE.write() {
        *raw = Some(p.clone());
    }
    super::tone::set(adjusted(p));
}

fn adjusted(
    mut p: std::collections::BTreeMap<String, Rgba>,
) -> std::collections::BTreeMap<String, Rgba> {
    if increase_contrast() {
        let mut w = Vec::new();
        design::enforce_contrast(
            &mut p,
            &design::Contrast {
                text: 7.0,
                ui: 4.5,
                enforce: true,
                ..Default::default()
            },
            &mut w,
        );
        if let Some(fg) = p.get("fg").copied() {
            let line = crate::theme_engine::color::with_alpha(fg, 0.45);
            p.insert("border".into(), line);
            p.insert("separator".into(), line);
        }
    }
    p
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
    (reference * ratio * SCREEN_TEXT.with(std::cell::Cell::get) * user_text_scale()).max(6.0)
}

/// Échelle du texte réglée par l'utilisateur.
fn user_text_scale() -> f32 {
    f32::from_bits(USER_TEXT_SCALE.load(Ordering::Relaxed))
}

/// Taille d'un libellé de commande (barre, boutons) : rôle `label` du
/// thème (MLG-282).
pub fn ts_label(reference: f32) -> f32 {
    role(reference, |t| t.label / design::REF_LABEL)
}

/// Taille d'une mesure (horloge, monitoring) : rôle `metric` du thème.
pub fn ts_metric(reference: f32) -> f32 {
    role(reference, |t| t.metric / design::REF_METRIC)
}

fn role(reference: f32, ratio: impl Fn(&design::Typography) -> f32) -> f32 {
    let r = ratio(&design().typography);
    (reference * r * SCREEN_TEXT.with(std::cell::Cell::get) * user_text_scale()).max(6.0)
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
    let mut g = if reduced_transparency() {
        g.reduced()
    } else {
        g
    };
    // Qualité des effets du lanceur (MLG-278).
    if p == Place::Launcher {
        match super::apps_grid::shape().quality {
            crate::config::EffectsQuality::Low => g = g.reduced(),
            crate::config::EffectsQuality::Medium => g.pointer_response = false,
            crate::config::EffectsQuality::High => {}
        }
    }
    if increase_contrast() {
        g.border = g.border.max(0.3);
    }
    g
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
        } else if increase_contrast() && p != Place::Content {
            // Contraste renforcé : surfaces plus denses.
            e.opacity = e.opacity.max(0.9);
        }
        e
    }
}

/// Couleur moyenne du fond d'écran affiché, et part qu'en prend le verre
/// (`wallpaper.glass_tint`, MLG-280).
static WALL_COLOR: RwLock<Option<Rgba>> = RwLock::new(None);
static WALL_TINT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Couleur moyenne du dernier fond décodé.
pub fn set_wall_color(c: Rgba) {
    if let Ok(mut w) = WALL_COLOR.write() {
        *w = Some(c);
    }
}

/// Part du fond dans la teinte du verre.
pub fn set_wall_tint(strength: f32) {
    WALL_TINT.store(strength.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

/// Fond d'une surface teinté par son verre (`tint`, `tint_strength`).
pub fn tinted(bg: Rgba, p: Place) -> Rgba {
    let bg = match (
        f32::from_bits(WALL_TINT.load(Ordering::Relaxed)),
        WALL_COLOR.read().ok().and_then(|c| *c),
    ) {
        (k, Some(c)) if k > 0.0 => {
            let m = crate::theme_engine::color::mix(bg, c, f64::from(k));
            Rgba(m.0, m.1, m.2, bg.3)
        }
        _ => bg,
    };
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

/// Couleur de l'anneau de focus clavier : le slot `focus` du pack s'il le
/// distingue de l'accent (MLG-282), sinon `fallback`.
pub fn focus(fallback: Color) -> Color {
    match (super::tone::get("focus"), super::tone::get("accent")) {
        (Some(f), Some(acc)) if f != acc => {
            Color::from_rgba8(f.0, f.1, f.2, f32::from(f.3) / 255.0)
        }
        _ => fallback,
    }
}

/// Surface de contenu (MLG-282) : un fond de lecture stable sous les
/// résultats, formulaires et graphiques d'une surface de verre, à
/// l'opacité du rôle `content` du thème (aucune sans thème).
pub fn content<'a, M: 'a>(
    el: impl Into<iced::Element<'a, M>>,
    theme: &Theme,
    radius: f32,
) -> iced::Element<'a, M> {
    let e = theme.shown(Place::Content);
    if e.opacity <= 0.001 {
        return el.into();
    }
    let bg = tinted(theme.bg, Place::Content);
    let fill = Color::from_rgba8(bg.0, bg.1, bg.2, e.opacity);
    let line = glass(Place::Content).border.max(0.06);
    let fg = theme.fg;
    iced::widget::container(el)
        .padding(6)
        .style(move |_| iced::widget::container::Style {
            background: Some(fill.into()),
            border: iced::Border::default()
                .rounded(radius)
                .width(1)
                .color(Color::from_rgba8(fg.0, fg.1, fg.2, line)),
            ..Default::default()
        })
        .into()
}

/// Dégradé de bord au défilement (MLG-282) : la liste s'estompe sous les
/// bords haut et bas de sa surface (`edge`, couleur du fond), si le thème
/// le veut (oui par défaut).
pub fn scroll_edges<'a, M: 'a>(
    list: impl Into<iced::Element<'a, M>>,
    edge: Color,
) -> iced::Element<'a, M> {
    if !design().scroll_edge {
        return list.into();
    }
    let fade = |top: bool| {
        let (from, to) = (edge, Color { a: 0.0, ..edge });
        let g = iced::gradient::Linear::new(iced::Radians(if top {
            std::f32::consts::PI
        } else {
            0.0
        }))
        .add_stop(0.0, from)
        .add_stop(1.0, to);
        iced::widget::container(iced::widget::Space::new())
            .width(iced::Length::Fill)
            .height(14)
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Gradient(g.into())),
                ..Default::default()
            })
    };
    iced::widget::stack![
        list.into(),
        iced::widget::column![
            fade(true),
            iced::widget::Space::new().height(iced::Length::Fill),
            fade(false)
        ]
        .height(iced::Length::Fill)
    ]
    .into()
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
    let ring = focus(sel);
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
                a: ring.a * 0.6,
                ..ring
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
    /// Fond propre au mode du thème (`wallpaper.light_image`,
    /// `wallpaper.dark_image`, MLG-280), s'il est réglé et lisible.
    pub(super) fn mode_wallpaper(&self) -> Option<std::path::PathBuf> {
        let g = &self.cfg.general;
        let light = crate::theme_engine::color::luminance(g.theme.bg) > 0.4;
        let key = if light {
            "wallpaper.light_image"
        } else {
            "wallpaper.dark_image"
        };
        let p = crate::design_options::choice(g, key);
        let p = std::path::PathBuf::from(p.trim());
        (!p.as_os_str().is_empty() && p.is_file()).then_some(p)
    }

    /// Fond peint sous le lanceur en plein bureau (MLG-275) : rien d'autre
    /// que le bureau n'est derrière lui, sa lentille réfracte le fond de
    /// l'écran actif. Ailleurs (fenêtres derrière), `None`.
    pub(super) fn launcher_under(&self) -> Option<iced::Element<'_, super::Msg>> {
        if !super::lens::available() || self.mode() != super::DesktopMode::FullDesktop {
            return None;
        }
        let m = self.monitors.iter().find(|m| m.focused)?;
        self.wall_backdrop(&m.name, iced::Point::ORIGIN)
    }

    /// Teinte de la lentille du lanceur : son fond à l'opacité du thème.
    pub(super) fn launcher_tint(&self, bg: iced::Color) -> iced::Color {
        let o = self
            .cfg
            .general
            .theme
            .shown(crate::config::Place::Launcher)
            .opacity;
        iced::Color { a: o, ..bg }
    }

    /// Fond d'écran de `monitor`, à peindre sous une surface posée sur
    /// cet écran en `origin` (coin de la surface, pixels logiques) : la
    /// lentille de la barre ou du dock a ainsi le vrai fond à réfracter
    /// (MLG-275). `None` : fond vidéo ou pas encore affiché.
    pub(super) fn wall_backdrop(
        &self,
        monitor: &str,
        origin: iced::Point,
    ) -> Option<iced::Element<'_, super::Msg>> {
        let shown = self.shown.get(monitor)?;
        let wall = match &shown.current {
            super::Wall::Color(c) => super::glass::Backing::Color(super::color(*c, 1.0)),
            super::Wall::Image(key) => {
                if self.video_walls.contains_key(key) {
                    return None;
                }
                let h = self.walls.get(key)?;
                let fit = if key.4 {
                    iced::ContentFit::Cover
                } else {
                    match key.3 {
                        crate::config::Fit::Cover => iced::ContentFit::Cover,
                        crate::config::Fit::Contain => iced::ContentFit::Contain,
                        crate::config::Fit::Fill => iced::ContentFit::Fill,
                        crate::config::Fit::Center => iced::ContentFit::None,
                    }
                };
                super::glass::Backing::Image(h.clone(), fit)
            }
        };
        Some(iced::Element::new(super::glass::Backdrop {
            wall,
            screen: self.monitor_size(monitor),
            origin,
        }))
    }

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
        set_user(true, true, false, 1.0);
        assert!(t.shown(Place::Bar).opacity >= REDUCED_OPACITY);
        assert_eq!(glass(Place::Bar).highlight, 0.0);
        assert!(motion().reduced);
        set_user(false, false, false, 1.0);
        set(Design::default());
        assert_eq!(ts(13.0), 13.0);
    }
}
