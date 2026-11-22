//! Résolution (§5 et §6 de la spécification) : variante + chaîne d'héritage →
//! palette complète, effets par surface, options Hyprland.
//!
//! Ordre de priorité : surcharge de variante > palette du pack > ancêtres
//! (le plus proche d'abord) > défaut dérivé. Les slots requis (`bg`, `fg`,
//! `accent`) doivent exister quelque part dans la chaîne ; les dérivés sont
//! calculés en dernier et ne remplacent jamais une valeur explicite.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::{Effect, Place, Rgba};

use super::color;
use super::manifest::{Cursors, Fonts, Named, PackTarget, VARIANTS};
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
    /// Typographie déclarée (cible `fonts`).
    pub fonts: Option<Fonts>,
    /// Thème d'icônes (cible `icons`).
    pub icons: Option<Named>,
    /// Thème de curseur et taille (cible `cursors`).
    pub cursors: Option<Cursors>,
    /// Thème de sons (cible `sounds`).
    pub sounds: Option<Named>,
    /// Cibles déclarées par la chaîne de packs : surcharges et cibles
    /// génériques tierces, fusionnées dans l'ordre (le pack gagne).
    pub pack_targets: BTreeMap<String, PackTarget>,
    /// Dossiers des packs de la chaîne qui en ont un, le pack d'abord puis
    /// ses ancêtres (le plus proche d'abord) : gabarits de convention
    /// hérités (`templates/<id>.tmpl`).
    pub template_dirs: Vec<PathBuf>,
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

    // Typographie, icônes, curseurs, sons : la dernière déclarée gagne.
    let fonts = chain.iter().find_map(|p| p.manifest.fonts.clone());
    let icons = chain.iter().find_map(|p| p.manifest.icons.clone());
    let cursors = chain.iter().find_map(|p| p.manifest.cursors.clone());
    let sounds = chain.iter().find_map(|p| p.manifest.sounds.clone());

    // Cibles déclarées : fusion de la chaîne champ par champ (le dernier
    // pack gagne). Un gabarit garde le dossier du pack qui l'a déclaré : un
    // enfant qui ne redéclare que `reload` lit toujours celui du parent.
    let mut pack_targets: BTreeMap<String, PackTarget> = BTreeMap::new();
    for pack in chain {
        for (id, decl) in &pack.manifest.targets {
            let e = pack_targets.entry(id.clone()).or_default();
            if let Some(t) = &decl.template {
                e.template = Some(t.clone());
                e.template_base = pack.dir.clone();
            }
            if decl.output.is_some() {
                e.output = decl.output.clone();
            }
            if decl.reload.is_some() {
                e.reload = decl.reload.clone();
            }
            if decl.enabled.is_some() {
                e.enabled = decl.enabled;
            }
        }
    }
    let template_dirs = chain.iter().rev().filter_map(|p| p.dir.clone()).collect();

    Ok(Resolved {
        id: top.manifest.meta.id.clone(),
        name: top.manifest.meta.name.clone(),
        version: top.manifest.meta.version.clone(),
        variant: variant.to_string(),
        palette,
        effects,
        hyprland_options,
        fonts,
        icons,
        cursors,
        sounds,
        pack_targets,
        template_dirs,
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
    // Texte secondaire : 35 % du chemin de fg vers l'extrême du côté du
    // fond (vers le noir sur un thème sombre, comme avant ; vers le blanc
    // sur un thème clair, où il partait vers le noir — MLG-250), puis
    // lisible sur surface (3:1, plancher de `verify`).
    p.entry("fg_muted".to_string()).or_insert({
        let o = color::to_oklch(fg);
        let l = if color::luminance(bg) > color::luminance(fg) {
            o.l + (1.0 - o.l) * 0.35
        } else {
            o.l * 0.65
        };
        color::ensure_contrast(color::from_oklch(color::Oklch { l, ..o }), surface, 3.0)
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
    // États (ok, attention, danger) : teintes fixes, luminosité ajustée pour
    // rester lisibles en texte sur le fond du thème (4,5:1), clair comme
    // sombre.
    for (slot, o) in [
        (
            "success",
            color::Oklch {
                l: 0.72,
                c: 0.17,
                h: 145.0,
            },
        ),
        (
            "warning",
            color::Oklch {
                l: 0.80,
                c: 0.16,
                h: 75.0,
            },
        ),
        (
            "danger",
            color::Oklch {
                l: 0.63,
                c: 0.21,
                h: 25.0,
            },
        ),
    ] {
        p.entry(slot.to_string())
            .or_insert_with(|| color::ensure_contrast(color::from_oklch(o), bg, 4.5));
    }
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
        icons: None,
        cursors: None,
        sounds: None,
        pack_targets: BTreeMap::new(),
        template_dirs: Vec::new(),
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

    #[test]
    fn états_lisibles_en_clair_comme_en_sombre() {
        // MLG-234 : le jaune d'attention était illisible sur un fond clair.
        use crate::config::Rgba;
        for bg in [
            Rgba(0xf3, 0xf4, 0xf8, 0xff),
            Rgba(0x17, 0x1b, 0x27, 0xff),
            Rgba(0, 0, 0, 0xff),
        ] {
            let r = from_adjusted(
                Rgba(0x1f, 0x23, 0x30, 0xff),
                bg,
                Rgba(0x5b, 0x5b, 0xd6, 0xff),
                bg,
            );
            for (slot, teinte) in [("success", 145.0), ("warning", 75.0), ("danger", 25.0)] {
                let c = r.palette[slot];
                let ratio = color::contrast(c, bg);
                assert!(ratio >= 4.5, "{slot} sur {bg:?} : {ratio:.2}");
                // MLG-250 : la teinte et la couleur restent (le noir passait
                // aussi le 4,5:1).
                let o = color::to_oklch(c);
                let écart = ((o.h - teinte + 180.0).rem_euclid(360.0) - 180.0).abs();
                assert!(o.c > 0.08, "{slot} sur {bg:?} : chroma {:.3} ({c:?})", o.c);
                assert!(
                    écart < 25.0,
                    "{slot} sur {bg:?} : teinte {:.0} ({c:?})",
                    o.h
                );
                assert!(o.l > 0.35, "{slot} sur {bg:?} : presque noir ({c:?})");
            }
        }
    }

    #[test]
    fn texte_secondaire_entre_texte_et_fond() {
        // MLG-250 : sur un thème clair, fg_muted était plus foncé que fg.
        use crate::config::Rgba;
        for (fg, bg) in [
            (Rgba(0x1f, 0x23, 0x30, 0xff), Rgba(0xf3, 0xf4, 0xf8, 0xff)),
            (Rgba(0xf2, 0xf4, 0xfc, 0xff), Rgba(0x17, 0x1b, 0x27, 0xff)),
        ] {
            let r = from_adjusted(fg, bg, Rgba(0x5b, 0x5b, 0xd6, 0xff), bg);
            let l = |c| color::luminance(c);
            let m = r.palette["fg_muted"];
            let (lo, hi) = if l(fg) < l(bg) {
                (l(fg), l(bg))
            } else {
                (l(bg), l(fg))
            };
            assert!(
                l(m) > lo && l(m) < hi,
                "fg_muted {m:?} hors de [{fg:?}, {bg:?}]"
            );
            assert!(color::contrast(m, r.palette["surface"]) >= 3.0);
        }
        // Thème sombre : inchangé (65 % de la luminosité de fg).
        let r = from_adjusted(
            Rgba(0xf2, 0xf4, 0xfc, 0xff),
            Rgba(0x17, 0x1b, 0x27, 0xff),
            Rgba(0xa6, 0x9c, 0xff, 0xff),
            Rgba(0x17, 0x1b, 0x27, 0xff),
        );
        let o = color::to_oklch(Rgba(0xf2, 0xf4, 0xfc, 0xff));
        let attendu = color::from_oklch(color::Oklch { l: o.l * 0.65, ..o });
        assert_eq!(r.palette["fg_muted"], attendu);
    }
}
