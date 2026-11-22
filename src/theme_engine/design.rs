//! Jetons de conception du thème (MLG-267 à 273) : verre complet par
//! surface, contraste, typographie par rôle, états interactifs, mouvement
//! et surcharges par écran.
//!
//! Résolus avec la palette, dans le même ordre de priorité (variante, puis
//! pack, ancêtres, défauts du moteur), publiés dans `palette.json` pour les
//! applications tierces et suivis par les surfaces de hyprdesk. Rien ici ne
//! dépend d'iced : le binaire du moteur les calcule aussi.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::{Effect, Place, Rgba};

use super::color;
use super::manifest::{EffectSpec, Fonts, GlassLook, Manifest};

/// Verre d'une surface, hors opacité et flou (portés par [`Effect`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Glass {
    /// Teinte résolue (`#rrggbbaa`), mêlée au fond à `tint_strength`.
    pub tint: Option<String>,
    pub tint_strength: f32,
    /// Force (0..1) et largeur (px) de la réfraction au bord.
    pub refraction: f32,
    pub refraction_width: f32,
    /// Reflet (0..1), qui suit le pointeur si `pointer_response`.
    pub highlight: f32,
    pub pointer_response: bool,
    /// Opacités du filet (couleur du texte) et de l'ombre portée.
    pub border: f32,
    pub shadow: f32,
}

impl Default for Glass {
    fn default() -> Self {
        Self {
            tint: None,
            tint_strength: 0.0,
            refraction: 0.0,
            refraction_width: 10.0,
            highlight: 0.08,
            pointer_response: false,
            // Les surfaces gardent leur propre filet ; le verre en ajoute
            // un s'il est réglé.
            border: 0.0,
            shadow: 0.27,
        }
    }
}

impl Glass {
    /// Teinte décodée.
    pub fn tint_rgba(&self) -> Option<Rgba> {
        self.tint.as_deref().and_then(color::parse)
    }

    /// Verre sans effet de lumière : transparence réduite.
    pub fn reduced(&self) -> Self {
        Self {
            refraction: 0.0,
            highlight: 0.0,
            pointer_response: false,
            ..self.clone()
        }
    }
}

/// Cibles de contraste.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contrast {
    /// Texte (WCAG 1.4.3).
    pub text: f32,
    /// Repères visuels essentiels (WCAG 1.4.11).
    pub ui: f32,
    /// Densité minimale des surfaces lisibles (menus, lanceur, Réglages),
    /// appliquée à l'affichage : les réglages gardent leur valeur.
    pub min_opacity: f32,
    pub enforce: bool,
    pub reduced_transparency: bool,
}

impl Default for Contrast {
    fn default() -> Self {
        Self {
            text: 4.5,
            ui: 3.0,
            min_opacity: 0.5,
            enforce: true,
            reduced_transparency: false,
        }
    }
}

/// Opacité plancher d'une surface en transparence réduite.
pub const REDUCED_OPACITY: f32 = 0.96;

/// Typographie résolue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Typography {
    pub family: Option<String>,
    pub mono_family: Option<String>,
    pub caption: f32,
    pub body: f32,
    /// Libellés des commandes (barre, boutons) (MLG-282).
    #[serde(default = "ref_label")]
    pub label: f32,
    pub title: f32,
    pub display: f32,
    /// Mesures (horloges, monitoring) (MLG-282).
    #[serde(default = "ref_metric")]
    pub metric: f32,
    pub weight: u16,
    pub title_weight: u16,
    pub line_height: f32,
    pub tabular_numbers: bool,
}

/// Tailles de référence de hyprdesk : celles que l'interface emploie sans
/// thème (le rapport taille du thème / référence met tout un rôle à
/// l'échelle).
pub const REF_CAPTION: f32 = 11.0;
pub const REF_BODY: f32 = 13.0;
pub const REF_TITLE: f32 = 17.0;
pub const REF_DISPLAY: f32 = 34.0;
pub const REF_LABEL: f32 = 12.0;
pub const REF_METRIC: f32 = 34.0;

fn ref_label() -> f32 {
    REF_LABEL
}

fn ref_metric() -> f32 {
    REF_METRIC
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            family: None,
            mono_family: None,
            caption: REF_CAPTION,
            body: REF_BODY,
            label: REF_LABEL,
            title: REF_TITLE,
            display: REF_DISPLAY,
            metric: REF_METRIC,
            weight: 400,
            title_weight: 700,
            line_height: 1.3,
            tabular_numbers: false,
        }
    }
}

impl Typography {
    /// Taille du thème pour une taille écrite dans l'interface : le rôle est
    /// déduit de la taille de référence (légende, corps, titre, affichage),
    /// puis mis à l'échelle du rôle du thème.
    pub fn scaled(&self, reference: f32) -> f32 {
        let ratio = if reference < 12.0 {
            self.caption / REF_CAPTION
        } else if reference < 15.5 {
            self.body / REF_BODY
        } else if reference < 24.0 {
            self.title / REF_TITLE
        } else {
            self.display / REF_DISPLAY
        };
        (reference * ratio).max(6.0)
    }
}

/// États interactifs : part du texte mêlée au fond (voiles), anneau de
/// focus, opacité d'une commande désactivée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct States {
    pub hover: f32,
    pub pressed: f32,
    pub selected: f32,
    pub focus_width: f32,
    pub disabled: f32,
}

impl Default for States {
    fn default() -> Self {
        Self {
            // Voiles de hyprdesk sans thème.
            hover: 0.10,
            pressed: 0.16,
            selected: 0.22,
            focus_width: 2.0,
            disabled: 0.45,
        }
    }
}

/// Courbe de mouvement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    /// Ease-out cubique : la courbe de hyprdesk sans thème.
    CubicOut,
    Spring,
    Bezier([f32; 4]),
}

impl Easing {
    pub fn parse(text: &str) -> Option<Self> {
        let t = text.trim().to_ascii_lowercase();
        Some(match t.as_str() {
            "linear" => Easing::Linear,
            "ease-out-cubic" => Easing::CubicOut,
            "spring" => Easing::Spring,
            "ease" => Easing::Bezier([0.25, 0.1, 0.25, 1.0]),
            "ease-in" => Easing::Bezier([0.42, 0.0, 1.0, 1.0]),
            "ease-out" => Easing::Bezier([0.0, 0.0, 0.58, 1.0]),
            "ease-in-out" => Easing::Bezier([0.42, 0.0, 0.58, 1.0]),
            _ => {
                let inner = t.strip_prefix("cubic-bezier(")?.strip_suffix(')')?;
                let v: Vec<f32> = inner
                    .split(',')
                    .map(|x| x.trim().parse::<f32>())
                    .collect::<std::result::Result<_, _>>()
                    .ok()?;
                let [x1, y1, x2, y2] = <[f32; 4]>::try_from(v).ok()?;
                if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
                    return None;
                }
                Easing::Bezier([x1, y1, x2, y2])
            }
        })
    }

    /// Avancement mis en courbe (t ∈ 0..1).
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::CubicOut => 1.0 - (1.0 - t).powi(3),
            // Ressort amorti critique : arrivée douce, léger élan.
            Easing::Spring => 1.0 - (1.0 + 6.0 * t) * (-6.0 * t).exp(),
            Easing::Bezier([x1, y1, x2, y2]) => {
                // x(s) = t par dichotomie, puis y(s).
                let bez = |a: f32, b: f32, s: f32| {
                    let u = 1.0 - s;
                    3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
                };
                let (mut lo, mut hi) = (0.0f32, 1.0f32);
                for _ in 0..24 {
                    let mid = (lo + hi) / 2.0;
                    if bez(x1, x2, mid) < t {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                bez(y1, y2, (lo + hi) / 2.0)
            }
        }
    }
}

/// Mouvement des surfaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    pub duration: u32,
    pub easing: String,
    pub scale: f32,
    pub offset: f32,
    pub hover_highlight: f32,
    pub reduced: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            // Les valeurs de hyprdesk sans thème : un pack qui n'en déclare
            // pas ne change rien.
            duration: 180,
            easing: "ease-out-cubic".to_string(),
            scale: 0.975,
            offset: 6.0,
            hover_highlight: 0.12,
            reduced: false,
        }
    }
}

impl Motion {
    pub fn curve(&self) -> Easing {
        Easing::parse(&self.easing).unwrap_or(Easing::CubicOut)
    }

    /// Mouvement effectif : réduit, il n'a plus ni échelle ni déplacement,
    /// et sa durée tombe à un fondu court.
    pub fn effective(&self, reduced: bool) -> Self {
        if reduced || self.reduced {
            Self {
                duration: self.duration.min(90),
                easing: "linear".to_string(),
                scale: 1.0,
                offset: 0.0,
                hover_highlight: 0.0,
                reduced: true,
            }
        } else {
            self.clone()
        }
    }
}

/// Surface surchargée sur un écran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenPlace {
    pub opacity: f32,
    pub blur: bool,
    pub adaptive: Option<f32>,
    pub glass: Glass,
}

/// Surcharges d'un écran.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Screen {
    /// Surfaces surchargées (`bar`, `launcher`…).
    pub effects: BTreeMap<String, ScreenPlace>,
    pub text_scale: f32,
}

impl Screen {
    /// L'écran désigné par `selector` : nom de sortie (`DP-1`) ou
    /// définition (`2570x720`).
    pub fn matches(selector: &str, name: &str, size: Option<(u32, u32)>) -> bool {
        if selector == name {
            return true;
        }
        match (selector.split_once(['x', '×']), size) {
            (Some((w, h)), Some((sw, sh))) => {
                w.trim().parse() == Ok(sw) && h.trim().parse() == Ok(sh)
            }
            _ => false,
        }
    }
}

/// Jetons résolus d'un thème.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Design {
    /// Verre de chaque surface (`bar`, `menus`, `launcher`, `settings`,
    /// `widgets`).
    pub glass: BTreeMap<String, Glass>,
    pub contrast: Contrast,
    pub typography: Typography,
    pub states: States,
    pub motion: Motion,
    /// Sélecteur d'écran → surcharges.
    pub screens: BTreeMap<String, Screen>,
    /// Opacité du matériau de secours de chaque surface, quand elle ne
    /// voit pas ce qu'il y a derrière elle (MLG-282).
    #[serde(default)]
    pub fallback: BTreeMap<String, f32>,
    /// Dégradé de bord au défilement (MLG-282).
    #[serde(default = "yes")]
    pub scroll_edge: bool,
}

fn yes() -> bool {
    true
}

impl Default for Design {
    fn default() -> Self {
        Self {
            glass: Place::ALL
                .iter()
                .map(|p| (p.as_str().to_string(), Glass::default()))
                .collect(),
            contrast: Contrast::default(),
            typography: Typography::default(),
            states: States::default(),
            motion: Motion::default(),
            screens: BTreeMap::new(),
            fallback: BTreeMap::new(),
            scroll_edge: true,
        }
    }
}

impl Design {
    pub fn glass(&self, p: Place) -> Glass {
        self.glass.get(p.as_str()).cloned().unwrap_or_default()
    }

    /// Surcharges de l'écran `name` (définition `size`), la plus précise
    /// d'abord : nom de sortie, puis définition.
    pub fn screen(&self, name: &str, size: Option<(u32, u32)>) -> Option<&Screen> {
        self.screens.get(name).or_else(|| {
            self.screens
                .iter()
                .find(|(sel, _)| Screen::matches(sel, name, size))
                .map(|(_, s)| s)
        })
    }
}

/// Préréglages livrés : verre léger, teinté, dense, et les matériaux par
/// rôle (MLG-282) : `transparent` (le bureau reste visible), `glass_clear`
/// (verre très transparent, bord réfractif), `glass_regular` (verre
/// adaptatif des commandes), `content_surface` (surface de lecture stable).
pub fn builtin_presets() -> BTreeMap<String, EffectSpec> {
    let preset = |opacity: f64, look: GlassLook| EffectSpec {
        opacity: Some(opacity),
        blur: Some(true),
        glass: look,
        ..Default::default()
    };
    let material = |opacity: f64, blur: bool, adaptive: Option<f64>, look: GlassLook| EffectSpec {
        opacity: Some(opacity),
        blur: Some(blur),
        adaptive: adaptive.map(|_| true),
        opacity_max: adaptive,
        glass: look,
        ..Default::default()
    };
    let none = || GlassLook {
        highlight: Some(0.0),
        refraction: Some(0.0),
        border: Some(0.0),
        tint_strength: Some(0.0),
        ..Default::default()
    };
    BTreeMap::from([
        (
            "transparent".to_string(),
            material(0.0, false, None, none()),
        ),
        (
            "glass_clear".to_string(),
            material(
                0.35,
                true,
                None,
                GlassLook {
                    highlight: Some(0.16),
                    refraction: Some(0.35),
                    border: Some(0.12),
                    ..Default::default()
                },
            ),
        ),
        (
            "glass_regular".to_string(),
            material(
                0.72,
                true,
                Some(0.94),
                GlassLook {
                    tint: Some("accent".to_string()),
                    tint_strength: Some(0.04),
                    highlight: Some(0.10),
                    refraction: Some(0.2),
                    ..Default::default()
                },
            ),
        ),
        (
            "content_surface".to_string(),
            material(0.94, true, None, none()),
        ),
        (
            "light".to_string(),
            preset(
                0.55,
                GlassLook {
                    highlight: Some(0.16),
                    refraction: Some(0.30),
                    border: Some(0.14),
                    ..Default::default()
                },
            ),
        ),
        (
            "tinted".to_string(),
            preset(
                0.72,
                GlassLook {
                    tint: Some("accent".to_string()),
                    tint_strength: Some(0.14),
                    highlight: Some(0.10),
                    refraction: Some(0.15),
                    ..Default::default()
                },
            ),
        ),
        (
            "dense".to_string(),
            preset(
                0.92,
                GlassLook {
                    highlight: Some(0.05),
                    refraction: Some(0.0),
                    border: Some(0.08),
                    ..Default::default()
                },
            ),
        ),
    ])
}

/// Nom de slot acceptable (`accent`, `surface_raised`, couleur libre).
pub fn slot_name(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_lowercase())
        && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '_')
}

/// Applique un aspect partiel.
fn apply_look(
    g: &mut Glass,
    l: &GlassLook,
    palette: &BTreeMap<String, Rgba>,
    warn: &mut Vec<String>,
) {
    let unit = |v: f64| v.clamp(0.0, 1.0) as f32;
    if let Some(t) = &l.tint {
        match color::parse(t).or_else(|| palette.get(t.as_str()).copied()) {
            Some(c) => g.tint = Some(color::fmt(c, color::Fmt::Hex)),
            None => warn.push(format!("teinte du verre inconnue ignorée : {t}")),
        }
    }
    if let Some(v) = l.tint_strength {
        g.tint_strength = unit(v);
    }
    if let Some(v) = l.refraction {
        g.refraction = unit(v);
    }
    if let Some(v) = l.refraction_width {
        g.refraction_width = v.clamp(0.0, 64.0) as f32;
    }
    if let Some(v) = l.highlight {
        g.highlight = unit(v);
    }
    if let Some(v) = l.pointer_response {
        g.pointer_response = v;
    }
    if let Some(v) = l.border {
        g.border = unit(v);
    }
    if let Some(v) = l.shadow {
        g.shadow = unit(v);
    }
}

/// Applique un effet partiel (opacité, flou, adaptatif) : tout champ
/// absent garde la valeur en place.
fn apply_effect(cur: &mut Effect, e: &EffectSpec) {
    if let Some(o) = e.opacity {
        cur.opacity = o.clamp(0.0, 1.0) as f32;
    }
    if let Some(b) = e.blur {
        cur.blur = b;
    }
    // Adaptatif : déclaré ici, sinon hérité ; un plafond seul relève celui
    // d'un parent adaptatif.
    cur.adaptive = match e.adaptive {
        Some(false) => None,
        Some(true) => Some(e.opacity_max.unwrap_or(1.0).clamp(0.0, 1.0) as f32),
        None => cur
            .adaptive
            .map(|m| e.opacity_max.map_or(m, |x| x.clamp(0.0, 1.0) as f32)),
    };
}

/// Contexte de résolution : préréglages connus et palette résolue.
struct Ctx<'a> {
    presets: BTreeMap<String, EffectSpec>,
    palette: &'a BTreeMap<String, Rgba>,
}

impl Ctx<'_> {
    /// Préréglage puis champs propres de la surface.
    fn apply(
        &self,
        effect: &mut Effect,
        glass: &mut Glass,
        e: &EffectSpec,
        warn: &mut Vec<String>,
    ) {
        if let Some(name) = e.preset.as_ref().or(e.material.as_ref()) {
            match self.presets.get(name) {
                Some(p) => {
                    apply_effect(effect, p);
                    apply_look(glass, &p.glass, self.palette, warn);
                }
                None => warn.push(format!("préréglage de verre inconnu ignoré : {name}")),
            }
        }
        apply_effect(effect, e);
        apply_look(glass, &e.glass, self.palette, warn);
    }
}

/// Effets et verre par surface, options du flou, jetons : la chaîne
/// (ancêtres d'abord), puis les surcharges de variante.
pub fn resolve(
    chain: &[&Manifest],
    variant: &str,
    palette: &BTreeMap<String, Rgba>,
    effects: &mut BTreeMap<Place, Effect>,
    hyprland_options: &mut BTreeMap<String, String>,
    warnings: &mut Vec<String>,
) -> Design {
    let mut design = Design::default();
    let mut presets = builtin_presets();
    let mut base = Glass::default();
    for m in chain {
        if let Some(g) = &m.glass {
            presets.extend(g.presets.clone());
            if let Some(v) = g.scroll_edge {
                design.scroll_edge = v;
            }
            apply_look(&mut base, &g.look, palette, warnings);
            // Flou de Hyprland : posé seulement si le pack ne le règle pas
            // lui-même dans [hyprland.options].
            for (key, v) in [
                ("decoration:blur:size", g.blur_size),
                ("decoration:blur:passes", g.blur_passes),
            ] {
                if let Some(v) = v {
                    hyprland_options.insert(key.to_string(), v.to_string());
                }
            }
        }
    }
    for m in chain {
        for (key, v) in &m.hyprland.options {
            if key.starts_with("decoration:blur:") {
                hyprland_options.insert(key.clone(), v.clone());
            }
        }
        if let Some(v) = m.variant.get(variant) {
            for (key, val) in &v.hyprland_options {
                if key.starts_with("decoration:blur:") {
                    hyprland_options.insert(key.clone(), val.clone());
                }
            }
        }
    }
    let ctx = Ctx { presets, palette };
    let mut glass: BTreeMap<Place, Glass> = Place::ALL.iter().map(|p| (*p, base.clone())).collect();
    let specs = chain.iter().map(|m| &m.effects).chain(
        chain
            .iter()
            .filter_map(|m| m.variant.get(variant))
            .map(|v| &v.effects),
    );
    for spec in specs {
        for (key, e) in spec {
            match Place::parse(key) {
                Some(place) => {
                    let (eff, gl) = (
                        effects.get_mut(&place).expect("toutes les surfaces"),
                        glass.get_mut(&place).expect("toutes les surfaces"),
                    );
                    ctx.apply(eff, gl, e, warnings);
                    // Matériau de secours : l'opacité de son préréglage.
                    if let Some(f) = &e.fallback {
                        match ctx.presets.get(f).and_then(|p| p.opacity) {
                            Some(o) => {
                                design
                                    .fallback
                                    .insert(key.clone(), o.clamp(0.0, 1.0) as f32);
                            }
                            None => {
                                warnings.push(format!("matériau de secours inconnu ignoré : {f}"))
                            }
                        }
                    }
                }
                None => warnings.push(format!("surface d'effet inconnue ignorée : {key}")),
            }
        }
    }

    // Contraste, états, mouvement : champ par champ, le plus proche gagne.
    design.contrast = contrast_of(chain);
    let s = &mut design.states;
    for spec in chain.iter().filter_map(|m| m.states.as_ref()) {
        let unit = |v: f64| v.clamp(0.0, 1.0) as f32;
        if let Some(v) = spec.hover {
            s.hover = unit(v);
        }
        if let Some(v) = spec.pressed {
            s.pressed = unit(v);
        }
        if let Some(v) = spec.selected {
            s.selected = unit(v);
        }
        if let Some(v) = spec.focus_width {
            s.focus_width = v.clamp(0.0, 8.0) as f32;
        }
        if let Some(v) = spec.disabled {
            s.disabled = unit(v);
        }
    }
    let mo = &mut design.motion;
    for spec in chain.iter().filter_map(|m| m.motion.as_ref()) {
        if let Some(v) = spec.duration {
            mo.duration = v.min(2000);
        }
        if let Some(v) = &spec.easing {
            mo.easing = v.trim().to_ascii_lowercase();
        }
        if let Some(v) = spec.scale {
            mo.scale = v.clamp(0.5, 1.5) as f32;
        }
        if let Some(v) = spec.offset {
            mo.offset = v.clamp(-200.0, 200.0) as f32;
        }
        if let Some(v) = spec.hover_highlight {
            mo.hover_highlight = v.clamp(0.0, 1.0) as f32;
        }
        if let Some(v) = spec.reduced {
            mo.reduced = v;
        }
    }
    design.typography = typography(merged_fonts(chain).as_ref());

    // Surcharges par écran : depuis la surface résolue.
    for m in chain {
        for (sel, spec) in &m.screens {
            let screen = design.screens.entry(sel.clone()).or_insert(Screen {
                effects: BTreeMap::new(),
                text_scale: 1.0,
            });
            if let Some(v) = spec.text_scale {
                screen.text_scale = v.clamp(0.5, 3.0) as f32;
            }
            for (key, e) in &spec.effects {
                let Some(place) = Place::parse(key) else {
                    warnings.push(format!("surface d'effet inconnue ignorée : {key}"));
                    continue;
                };
                let cur = screen.effects.get(key).cloned();
                let (mut eff, mut gl) = match cur {
                    Some(sp) => (
                        Effect {
                            opacity: sp.opacity,
                            blur: sp.blur,
                            adaptive: sp.adaptive,
                        },
                        sp.glass,
                    ),
                    None => (effects[&place], glass[&place].clone()),
                };
                ctx.apply(&mut eff, &mut gl, e, warnings);
                screen.effects.insert(
                    key.clone(),
                    ScreenPlace {
                        opacity: eff.opacity,
                        blur: eff.blur,
                        adaptive: eff.adaptive,
                        glass: gl,
                    },
                );
            }
        }
    }
    design.glass = glass
        .into_iter()
        .map(|(p, g)| (p.as_str().to_string(), g))
        .collect();
    design
}

/// `[contrast]` de la chaîne, champ par champ.
pub fn contrast_of(chain: &[&Manifest]) -> Contrast {
    let mut c = Contrast::default();
    for spec in chain.iter().filter_map(|m| m.contrast.as_ref()) {
        if let Some(v) = spec.text {
            c.text = v.clamp(1.0, 21.0) as f32;
        }
        if let Some(v) = spec.ui {
            c.ui = v.clamp(1.0, 21.0) as f32;
        }
        if let Some(v) = spec.min_opacity {
            c.min_opacity = v.clamp(0.0, 1.0) as f32;
        }
        if let Some(v) = spec.enforce {
            c.enforce = v;
        }
        if let Some(v) = spec.reduced_transparency {
            c.reduced_transparency = v;
        }
    }
    c
}

/// `[fonts]` de la chaîne fusionnés champ par champ, le plus proche gagnant
/// (MLG-266 : l'ancêtre le plus lointain l'emportait, en bloc).
pub fn merged_fonts(chain: &[&Manifest]) -> Option<Fonts> {
    let mut out: Option<Fonts> = None;
    for f in chain.iter().filter_map(|m| m.fonts.as_ref()) {
        let o = out.get_or_insert_with(Fonts::default);
        if f.family.is_some() {
            o.family = f.family.clone();
        }
        if f.mono_family.is_some() {
            o.mono_family = f.mono_family.clone();
        }
        if f.size.is_some() {
            o.size = f.size;
        }
        o.sizes.extend(f.sizes.clone());
        if f.weight.is_some() {
            o.weight = f.weight.clone();
        }
        if f.title_weight.is_some() {
            o.title_weight = f.title_weight.clone();
        }
        if f.line_height.is_some() {
            o.line_height = f.line_height;
        }
        if f.tabular_numbers.is_some() {
            o.tabular_numbers = f.tabular_numbers;
        }
    }
    out
}

/// Typographie depuis `[fonts]` : `size` vaut pour le corps si `sizes.body`
/// manque ; les autres rôles suivent le corps dans les proportions de
/// hyprdesk.
pub fn typography(f: Option<&Fonts>) -> Typography {
    let mut t = Typography::default();
    let Some(f) = f else {
        return t;
    };
    t.family = f.family.clone();
    t.mono_family = f.mono_family.clone();
    let size = |v: f64| v.clamp(6.0, 96.0) as f32;
    if let Some(body) = f.sizes.get("body").copied().or(f.size) {
        let k = size(body) / REF_BODY;
        t.body = size(body);
        t.caption = REF_CAPTION * k;
        t.label = REF_LABEL * k;
        t.title = REF_TITLE * k;
        t.display = REF_DISPLAY * k;
        t.metric = REF_METRIC * k;
    }
    if let Some(v) = f.sizes.get("label") {
        t.label = size(*v);
    }
    if let Some(v) = f.sizes.get("metric") {
        t.metric = size(*v);
    }
    if let Some(v) = f.sizes.get("caption") {
        t.caption = size(*v);
    }
    if let Some(v) = f.sizes.get("title") {
        t.title = size(*v);
    }
    if let Some(v) = f.sizes.get("display") {
        t.display = size(*v);
    }
    if let Some(w) = f.weight.as_ref().and_then(|w| w.value()) {
        t.weight = w;
    }
    if let Some(w) = f.title_weight.as_ref().and_then(|w| w.value()) {
        t.title_weight = w;
    }
    if let Some(v) = f.line_height {
        t.line_height = v.clamp(0.8, 3.0) as f32;
    }
    if let Some(v) = f.tabular_numbers {
        t.tabular_numbers = v;
    }
    t
}

/// Slots tenus à la cible du texte et à celle des repères.
pub const TEXT_SLOTS: [&str; 5] = ["fg", "fg_muted", "success", "warning", "danger"];
pub const UI_SLOTS: [&str; 4] = ["accent", "action", "selection", "focus"];

/// Contraste garanti (MLG-269) : un slot sous sa cible contre le fond est
/// ajusté en luminosité, avec un avertissement.
pub fn enforce_contrast(
    palette: &mut BTreeMap<String, Rgba>,
    c: &Contrast,
    warnings: &mut Vec<String>,
) {
    if !c.enforce {
        return;
    }
    let Some(bg) = palette.get("bg").copied() else {
        return;
    };
    let targets = TEXT_SLOTS
        .iter()
        .map(|s| (*s, f64::from(c.text)))
        .chain(UI_SLOTS.iter().map(|s| (*s, f64::from(c.ui))));
    for (slot, min) in targets {
        let Some(v) = palette.get(slot).copied() else {
            continue;
        };
        // Couleur translucide : jugée posée sur le fond.
        let shown = color::mix(bg, Rgba(v.0, v.1, v.2, 255), f64::from(v.3) / 255.0);
        if color::contrast(shown, bg) + 1e-6 >= min {
            continue;
        }
        // Corrigée opaque : une teinte translucide sous la cible le resterait.
        let fixed = color::ensure_contrast(Rgba(v.0, v.1, v.2, 255), bg, min);
        warnings.push(format!(
            "slot {slot} ajusté pour le contraste ({} → {}, cible {min}:1)",
            color::fmt(v, color::Fmt::Hex),
            color::fmt(fixed, color::Fmt::Hex)
        ));
        palette.insert(slot.to_string(), fixed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courbes_de_mouvement() {
        assert_eq!(
            Easing::parse("ease-out"),
            Some(Easing::Bezier([0.0, 0.0, 0.58, 1.0]))
        );
        assert_eq!(
            Easing::parse("cubic-bezier(0.2, 0.8, 0.2, 1)"),
            Some(Easing::Bezier([0.2, 0.8, 0.2, 1.0]))
        );
        assert_eq!(Easing::parse("rebond"), None);
        assert_eq!(Easing::parse("cubic-bezier(2, 0, 0, 1)"), None);
        for e in [
            Easing::Linear,
            Easing::Spring,
            Easing::parse("ease-in-out").unwrap(),
        ] {
            assert!(e.apply(0.0).abs() < 0.01);
            assert!((e.apply(1.0) - 1.0).abs() < 0.02);
        }
        assert!(Easing::parse("ease-out").unwrap().apply(0.5) > 0.5);
    }

    #[test]
    fn rôles_de_texte_à_l_échelle() {
        let t = typography(Some(&Fonts {
            size: Some(15.0),
            sizes: BTreeMap::from([("display".to_string(), 40.0)]),
            ..Default::default()
        }));
        assert_eq!(t.body, 15.0);
        assert!((t.caption - REF_CAPTION * 15.0 / 13.0).abs() < 1e-4);
        assert_eq!(t.display, 40.0);
        assert!((t.scaled(13.0) - 15.0).abs() < 1e-4);
        assert!((t.scaled(34.0) - 40.0).abs() < 1e-4);
        assert_eq!(Typography::default().scaled(14.0), 14.0);
    }

    #[test]
    fn écran_par_nom_ou_définition() {
        assert!(Screen::matches("DP-1", "DP-1", None));
        assert!(Screen::matches("2570x720", "HDMI-A-1", Some((2570, 720))));
        assert!(!Screen::matches("2570x720", "HDMI-A-1", Some((2560, 1440))));
    }

    #[test]
    fn mouvement_réduit() {
        let m = Motion::default().effective(true);
        assert!(m.reduced && m.scale == 1.0 && m.offset == 0.0 && m.duration <= 90);
        assert_eq!(Motion::default().effective(false), Motion::default());
    }

    #[test]
    fn contraste_corrigé_sous_la_cible() {
        let mut p = BTreeMap::from([
            ("bg".to_string(), Rgba(0x10, 0x10, 0x10, 255)),
            ("fg".to_string(), Rgba(0x30, 0x30, 0x30, 255)),
            ("accent".to_string(), Rgba(0xff, 0xff, 0xff, 255)),
        ]);
        let mut w = Vec::new();
        enforce_contrast(&mut p, &Contrast::default(), &mut w);
        assert!(color::contrast(p["fg"], p["bg"]) >= 4.5);
        assert_eq!(p["accent"], Rgba(0xff, 0xff, 0xff, 255));
        assert_eq!(w.len(), 1, "{w:?}");
        let mut p2 = p.clone();
        p2.insert("fg".to_string(), Rgba(0x30, 0x30, 0x30, 255));
        enforce_contrast(
            &mut p2,
            &Contrast {
                enforce: false,
                ..Default::default()
            },
            &mut w,
        );
        assert_eq!(p2["fg"], Rgba(0x30, 0x30, 0x30, 255));
    }
}
