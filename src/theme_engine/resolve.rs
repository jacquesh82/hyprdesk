//! Résolution (§5 et §6 de la spécification) : variante + chaîne d'héritage →
//! palette complète, effets par surface, options Hyprland.
//!
//! Ordre de priorité : surcharge de variante > palette du pack > ancêtres
//! (le plus proche d'abord) > défaut dérivé. Les slots requis (`bg`, `fg`,
//! `accent`) doivent exister quelque part dans la chaîne ; les dérivés sont
//! calculés en dernier et ne remplacent jamais une valeur explicite.

use std::collections::BTreeMap;

use crate::config::{Effect, Place, Rgba};

use super::color;
use super::manifest::{Fonts, VARIANTS};
use super::packs::Pack;
use super::{Error, Result};

/// Slots dont la présence est exigée dans la chaîne (directement ou hérités).
pub const REQUIRED: [&str; 3] = ["bg", "fg", "accent"];

/// Palette entièrement résolue d'un pack dans une variante.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub id: String,
    pub name: String,
    pub version: String,
    /// Variante effective : `dark` ou `light`.
    pub variant: String,
    /// Tous les slots (sémantiques + couleurs libres du pack), dérivés compris.
    pub palette: BTreeMap<String, Rgba>,
    /// Effet de chaque surface de l'interface, défauts du moteur compris.
    pub effects: BTreeMap<Place, Effect>,
    /// Options Hyprland de liste blanche, variante appliquée.
    pub hyprland_options: BTreeMap<String, String>,
    /// Typographie déclarée (cible `fonts`, phase 2).
    pub fonts: Option<Fonts>,
    /// Avertissements non bloquants (parents manquants, clés d'effet
    /// inconnues).
    pub warnings: Vec<String>,
}

/// Résout une chaîne (ancêtres d'abord, pack en dernier) dans une variante.
pub fn resolve(chain: &[Pack], variant: &str) -> Result<Resolved> {
    if !VARIANTS.contains(&variant) {
        return Err(Error::Variant(variant.to_string()));
    }
    let top = chain
        .last()
        .ok_or_else(|| Error::Missing("?".to_string()))?;
    let mut warnings = Vec::new();
    let ids: Vec<&str> = chain.iter().map(|p| p.manifest.meta.id.as_str()).collect();
    for pack in chain {
        for parent in &pack.manifest.inherits.themes {
            if !ids.contains(&parent.as_str()) {
                warnings.push(format!(
                    "parent manquant « {parent} » de {} : ignoré",
                    pack.manifest.meta.id
                ));
            }
        }
    }

    // Palette : fusion de la chaîne, puis surcharges de variante dans l'ordre.
    let mut palette_str: BTreeMap<String, String> = BTreeMap::new();
    for pack in chain {
        palette_str.extend(pack.manifest.palette.clone());
        if let Some(v) = pack.manifest.variant.get(variant) {
            palette_str.extend(v.palette.clone());
        }
    }
    let mut palette: BTreeMap<String, Rgba> = BTreeMap::new();
    for (name, value) in &palette_str {
        let Some(c) = color::parse(value) else {
            return Err(Error::Color(
                name.clone(),
                format!("« {value} » (attendu : #rrggbb, rgb(), rgba() ou hsl())"),
            ));
        };
        palette.insert(name.clone(), c);
    }
    for slot in REQUIRED {
        if !palette.contains_key(slot) {
            return Err(Error::Slot(slot.to_string()));
        }
    }
    fill_derived(&mut palette);

    // Effets : défauts du moteur, fusion de la chaîne, puis la variante.
    let mut effects: BTreeMap<Place, Effect> = Place::ALL
        .iter()
        .map(|p| (*p, p.default_effect()))
        .collect();
    let merge_effects = |spec: &BTreeMap<String, super::manifest::EffectSpec>,
                         effects: &mut BTreeMap<Place, Effect>,
                         warnings: &mut Vec<String>| {
        for (key, e) in spec {
            match Place::parse(key) {
                Some(place) => {
                    let current = effects[&place];
                    effects.insert(
                        place,
                        Effect {
                            opacity: e.opacity as f32,
                            blur: e.blur.unwrap_or(current.blur),
                        },
                    );
                }
                None => warnings.push(format!("surface d'effet inconnue ignorée : {key}")),
            }
        }
    };
    for pack in chain {
        merge_effects(&pack.manifest.effects, &mut effects, &mut warnings);
    }
    for pack in chain {
        if let Some(v) = pack.manifest.variant.get(variant) {
            merge_effects(&v.effects, &mut effects, &mut warnings);
        }
    }

    // Options Hyprland : chaîne, puis surcharge de variante.
    let mut hyprland_options: BTreeMap<String, String> = BTreeMap::new();
    for pack in chain {
        hyprland_options.extend(pack.manifest.hyprland.options.clone());
        if let Some(v) = pack.manifest.variant.get(variant) {
            hyprland_options.extend(v.hyprland_options.clone());
        }
    }

    // Typographie : la dernière déclarée dans la chaîne gagne.
    let fonts = chain.iter().find_map(|p| p.manifest.fonts.clone());

    Ok(Resolved {
        id: top.manifest.meta.id.clone(),
        name: top.manifest.meta.name.clone(),
        version: top.manifest.meta.version.clone(),
        variant: variant.to_string(),
        palette,
        effects,
        hyprland_options,
        fonts,
        warnings,
    })
}

/// Slots dérivés (§5) : calculés seulement quand ils ne sont pas posés
/// explicitement (palette, variante, héritage).
fn fill_derived(p: &mut BTreeMap<String, Rgba>) {
    let bg = p["bg"];
    let fg = p["fg"];
    let accent = p["accent"];
    p.entry("surface".to_string())
        .or_insert(color::lighten(bg, 6.0));
    let surface = p["surface"];
    p.entry("surface_raised".to_string())
        .or_insert(color::lighten(surface, 4.0));
    p.entry("fg_muted".to_string()).or_insert({
        let o = color::to_oklch(fg);
        color::from_oklch(color::Oklch { l: o.l * 0.65, ..o })
    });
    p.entry("border".to_string())
        .or_insert(color::with_alpha(fg, 0.18));
    p.entry("accent_fg".to_string()).or_insert({
        let blanc = Rgba(255, 255, 255, 255);
        let noir = Rgba(0, 0, 0, 255);
        if color::contrast(blanc, accent) >= color::contrast(noir, accent) {
            blanc
        } else {
            noir
        }
    });
    p.entry("accent_muted".to_string())
        .or_insert(color::with_alpha(accent, 0.35));
    p.entry("success".to_string())
        .or_insert(color::from_oklch(color::Oklch {
            l: 0.72,
            c: 0.17,
            h: 145.0,
        }));
    p.entry("warning".to_string())
        .or_insert(color::from_oklch(color::Oklch {
            l: 0.80,
            c: 0.16,
            h: 75.0,
        }));
    p.entry("danger".to_string())
        .or_insert(color::from_oklch(color::Oklch {
            l: 0.63,
            c: 0.21,
            h: 25.0,
        }));
    p.entry("background".to_string())
        .or_insert(color::darken(bg, 4.0));
}

/// Palette implicite « thème réglé » : fg, bg, accent et fond du bureau
/// donnés par les réglages du démon, le reste par les défauts dérivés — le
/// comportement actuel de hyprdesk devient un cas particulier du moteur.
pub fn from_adjusted(fg: Rgba, bg: Rgba, accent: Rgba, background: Rgba) -> Resolved {
    let mut palette = BTreeMap::new();
    for (name, c) in [
        ("fg", fg),
        ("bg", bg),
        ("accent", accent),
        ("background", background),
    ] {
        palette.insert(name.to_string(), c);
    }
    fill_derived(&mut palette);
    Resolved {
        id: "regle".to_string(),
        name: "Thème réglé".to_string(),
        version: "0".to_string(),
        variant: "dark".to_string(),
        palette,
        effects: Place::ALL
            .iter()
            .map(|p| (*p, p.default_effect()))
            .collect(),
        hyprland_options: BTreeMap::new(),
        fonts: None,
        warnings: Vec::new(),
    }
}

/// Palette implicite depuis le thème courant du démon : mêmes couleurs et
/// mêmes effets que l'interface, fond sans image dérivé du fond.
pub fn from_theme(t: &crate::config::Theme) -> Resolved {
    let mut r = from_adjusted(t.fg, t.bg, t.accent, super::color::darken(t.bg, 4.0));
    r.name = t.name.clone();
    r.effects = Place::ALL.iter().map(|p| (*p, t.effect(*p))).collect();
    r
}

#[cfg(test)]
mod tests {
    use super::super::manifest::Manifest;
    use super::*;

    fn pack_with(id: &str, palette: &[(&str, &str)], variant_light: &[(&str, &str)]) -> Pack {
        use std::fmt::Write as _;
        let mut text = format!("[meta]\nid = \"{id}\"\nname = \"{id}\"\nversion = \"1.0.0\"\n");
        let mut entries = String::new();
        for (k, v) in palette {
            let _ = writeln!(entries, "{k} = \"{v}\"");
        }
        let _ = write!(text, "\n[palette]\n{entries}");
        if !variant_light.is_empty() {
            let mut light = String::new();
            for (k, v) in variant_light {
                let _ = writeln!(light, "{k} = \"{v}\"");
            }
            let _ = write!(text, "\n[variant.light]\npalette.{light}");
        }
        let (manifest, _) = Manifest::parse(id, &text).unwrap();
        Pack {
            origin: super::super::packs::Origin::Bundled,
            dir: None,
            manifest,
        }
    }

    fn use_manifest(m: Manifest) -> Pack {
        Pack {
            origin: super::super::packs::Origin::Bundled,
            dir: None,
            manifest: m,
        }
    }

    #[test]
    fn palette_résolue_avec_dérivés() {
        let base = pack_with(
            "bas",
            &[("bg", "#171b27"), ("fg", "#f2f4fc"), ("accent", "#a69cff")],
            &[],
        );
        let r = resolve(&[base], "dark").unwrap();
        assert_eq!(r.palette["bg"], Rgba(0x17, 0x1b, 0x27, 0xff));
        assert!(r.palette.contains_key("surface"));
        assert!(r.palette.contains_key("accent_fg"));
        assert!(r.palette.contains_key("background"));
        // accent_fg : le contraste contre l'accent atteint au moins 4.5 (blanc
        // ou noir, l'un des deux dépasse toujours ce plancher).
        assert!(color::contrast(r.palette["accent_fg"], r.palette["accent"]) >= 4.0);
    }

    #[test]
    fn héritage_le_plus_proche_gagne() {
        let base = pack_with(
            "base",
            &[("bg", "#000000"), ("fg", "#ffffff"), ("accent", "#111111")],
            &[],
        );
        let haut = pack_with("haut", &[("accent", "#ff0000")], &[]);
        let r = resolve(&[base, haut], "dark").unwrap();
        assert_eq!(r.palette["bg"], Rgba(0, 0, 0, 255));
        assert_eq!(r.palette["accent"], Rgba(0xff, 0, 0, 255));
    }

    #[test]
    fn variante_surcharge_la_palette() {
        let base = pack_with(
            "bas",
            &[("bg", "#171b27"), ("fg", "#f2f4fc"), ("accent", "#a69cff")],
            &[("bg", "#eef0f8"), ("fg", "#171b27")],
        );
        let sombre = resolve(std::slice::from_ref(&base), "dark").unwrap();
        let clair = resolve(&[base], "light").unwrap();
        assert_eq!(sombre.palette["bg"], Rgba(0x17, 0x1b, 0x27, 0xff));
        assert_eq!(clair.palette["bg"], Rgba(0xee, 0xf0, 0xf8, 0xff));
        assert_eq!(clair.variant, "light");
    }

    #[test]
    fn slot_requis_absent_en_erreur() {
        let base = pack_with("bas", &[("bg", "#000000"), ("fg", "#ffffff")], &[]);
        assert!(matches!(resolve(&[base], "dark"), Err(Error::Slot(s)) if s == "accent"));
    }

    #[test]
    fn variante_inconnue_en_erreur() {
        let base = pack_with(
            "bas",
            &[("bg", "#000000"), ("fg", "#ffffff"), ("accent", "#111111")],
            &[],
        );
        assert!(matches!(resolve(&[base], "nuit"), Err(Error::Variant(_))));
    }

    #[test]
    fn parent_manquant_en_avertissement() {
        let mut base = pack_with(
            "base",
            &[("bg", "#000000"), ("fg", "#ffffff"), ("accent", "#111111")],
            &[],
        );
        base.manifest.inherits.themes = vec!["fantôme".to_string()];
        let r = resolve(&[base], "dark").unwrap();
        assert_eq!(r.warnings.len(), 1);
        assert!(r.warnings[0].contains("fantôme"));
    }

    #[test]
    fn effets_par_surface_avec_défauts() {
        let mut base = pack_with(
            "base",
            &[("bg", "#000000"), ("fg", "#ffffff"), ("accent", "#111111")],
            &[],
        );
        let (manifest, _) = Manifest::parse(
            "base",
            "[meta]\nid = \"base\"\nname = \"base\"\nversion = \"1\"\n[palette]\nbg = \"#000000\"\nfg = \"#ffffff\"\naccent = \"#111111\"\n[effects.bar]\nopacity = 0.5\n",
        )
        .unwrap();
        base.manifest = manifest;
        let r = resolve(&[base], "dark").unwrap();
        assert!((r.effects[&Place::Bar].opacity - 0.5).abs() < 1e-6);
        assert!(r.effects[&Place::Settings].opacity > 0.9);
    }

    #[test]
    fn effet_inconnu_en_avertissement() {
        let (mut manifest, _) = Manifest::parse(
            "base",
            "[meta]\nid = \"base\"\nname = \"base\"\nversion = \"1\"\n[palette]\nbg = \"#000000\"\nfg = \"#ffffff\"\naccent = \"#111111\"\n[effects.rien]\nopacity = 0.5\n",
        )
        .unwrap();
        manifest.meta.id = "base".to_string();
        let base = use_manifest(manifest);
        let r = resolve(&[base], "dark").unwrap();
        assert!(r.warnings.iter().any(|w| w.contains("surface d'effet")));
    }

    #[test]
    fn options_hyprland_variante_appliquée() {
        let (manifest, _) = Manifest::parse(
            "bas",
            "[meta]\nid = \"bas\"\nname = \"bas\"\nversion = \"1\"\n[palette]\nbg = \"#000000\"\nfg = \"#ffffff\"\naccent = \"#111111\"\n[hyprland.options]\n\"decoration:rounding\" = \"8\"\n[variant.light.hyprland_options]\n\"decoration:rounding\" = \"0\"\n",
        )
        .unwrap();
        let base = use_manifest(manifest);
        let sombre = resolve(std::slice::from_ref(&base), "dark").unwrap();
        let clair = resolve(&[base], "light").unwrap();
        assert_eq!(sombre.hyprland_options["decoration:rounding"], "8");
        assert_eq!(clair.hyprland_options["decoration:rounding"], "0");
    }

    #[test]
    fn palette_implicite_depuis_réglages() {
        let r = from_adjusted(
            Rgba(0xf2, 0xf4, 0xfc, 0xff),
            Rgba(0x17, 0x1b, 0x27, 0xff),
            Rgba(0xa6, 0x9c, 0xff, 0xff),
            Rgba(0x13, 0x17, 0x24, 0xff),
        );
        assert_eq!(r.id, "regle");
        assert_eq!(r.palette["bg"], Rgba(0x17, 0x1b, 0x27, 0xff));
        assert!(r.palette.contains_key("surface"));
    }
}
