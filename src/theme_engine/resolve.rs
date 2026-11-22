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
use super::design::{self, Design};
use super::manifest::{Cursors, Fonts, Named, PackTarget, VARIANTS};
use super::packs::Pack;
use super::{Error, Result};

/// Slots dont la présence est exigée dans la chaîne (directement ou hérités).
pub const REQUIRED: [&str; 3] = ["bg", "fg", "accent"];

/// Noms de rôles (MLG-282) → slot : `text_primary` est `fg`, etc. Les deux
/// noms sont présents dans la palette résolue.
pub const ALIASES: [(&str, &str); 4] = [
    ("text_primary", "fg"),
    ("text_secondary", "fg_muted"),
    ("separator", "border"),
    ("error", "danger"),
];

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
    /// Verre, contraste, typographie, états, mouvement, écrans (MLG-267 à
    /// 273).
    pub design: Design,
    /// Fond d'écran du pack, variante comprise (MLG-266).
    pub wallpaper: Option<PackWallpaper>,
    /// Forme et disposition (MLG-276) : `<section>.<clé>` → valeur, la
    /// plus proche déclarée gagnant.
    pub shape: BTreeMap<String, String>,
    /// Avertissements non bloquants (parents manquants, clés d'effet
    /// inconnues).
    pub warnings: Vec<String>,
}

/// Fond d'écran déclaré par le pack, chemin absolu dans son dossier.
#[derive(Debug, Clone, PartialEq)]
pub struct PackWallpaper {
    /// Image, ou dossier d'images pour un diaporama.
    pub path: PathBuf,
    pub slideshow: bool,
    pub interval: u64,
    pub shuffle: bool,
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
    // Rôles du brief (MLG-282) : un nom de rôle donne le slot qu'il
    // désigne, s'il manque.
    for (alias, slot) in ALIASES {
        if let Some(c) = palette.get(alias).copied() {
            palette.entry(slot.to_string()).or_insert(c);
        }
    }
    for slot in REQUIRED {
        if !palette.contains_key(slot) {
            return Err(Error::Slot(slot.to_string()));
        }
    }
    let manifests: Vec<&super::manifest::Manifest> = chain.iter().map(|p| &p.manifest).collect();
    // Contraste garanti (MLG-269) : slots déclarés d'abord, dérivés ensuite
    // (calculés à la cible).
    let contrast = design::contrast_of(&manifests);
    design::enforce_contrast(&mut palette, &contrast, &mut warnings);
    fill_derived(&mut palette, &contrast);

    // Options Hyprland : chaîne, puis surcharge de variante.
    let mut hyprland_options: BTreeMap<String, String> = BTreeMap::new();
    for pack in chain {
        hyprland_options.extend(pack.manifest.hyprland.options.clone());
        if let Some(v) = pack.manifest.variant.get(variant) {
            hyprland_options.extend(v.hyprland_options.clone());
        }
    }

    // Effets et verre : défauts du moteur, préréglages, fusion de la chaîne,
    // puis la variante ; jetons de conception.
    let mut effects: BTreeMap<Place, Effect> = Place::ALL
        .iter()
        .map(|p| (*p, p.default_effect()))
        .collect();
    let design = design::resolve(
        &manifests,
        variant,
        &palette,
        &mut effects,
        &mut hyprland_options,
        &mut warnings,
    );

    // Typographie fusionnée champ par champ ; icônes, curseurs, sons : la
    // plus proche déclarée gagne (MLG-266 : l'ancêtre le plus lointain
    // l'emportait).
    let fonts = design::merged_fonts(&manifests);
    let icons = chain.iter().rev().find_map(|p| p.manifest.icons.clone());
    let cursors = chain.iter().rev().find_map(|p| p.manifest.cursors.clone());
    let sounds = chain.iter().rev().find_map(|p| p.manifest.sounds.clone());
    let wallpaper = wallpaper(chain, variant, &mut warnings);
    let shape = chain
        .iter()
        .flat_map(|p| p.manifest.shape.clone())
        .collect();

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
        design,
        wallpaper,
        shape,
        warnings,
    })
}

/// Fond du pack : le plus proche qui en déclare un, la variante l'emportant
/// sur la déclaration générale ; chemin lu dans le dossier du pack qui le
/// déclare.
fn wallpaper(chain: &[Pack], variant: &str, warnings: &mut Vec<String>) -> Option<PackWallpaper> {
    let mut found: Option<(&super::manifest::WallpaperSpec, &Pack)> = None;
    for pack in chain {
        if let Some(w) = &pack.manifest.wallpaper {
            found = Some((w, pack));
        }
    }
    for pack in chain {
        if let Some(w) = pack
            .manifest
            .variant
            .get(variant)
            .and_then(|v| v.wallpaper.as_ref())
        {
            found = Some((w, pack));
        }
    }
    let (spec, pack) = found?;
    let rel = spec.image.as_ref().or(spec.dir.as_ref())?;
    let Some(dir) = &pack.dir else {
        warnings.push(format!(
            "fond d'écran de {} ignoré : pack embarqué, sans dossier",
            pack.manifest.meta.id
        ));
        return None;
    };
    let path = dir.join(rel);
    if !path.exists() {
        warnings.push(format!("fond d'écran introuvable : {}", path.display()));
        return None;
    }
    let slideshow = spec.mode.as_deref() == Some("slideshow") || path.is_dir();
    Some(PackWallpaper {
        path,
        slideshow,
        interval: spec.interval.unwrap_or(1800).max(10),
        shuffle: spec.shuffle.unwrap_or(false),
    })
}

/// Slots dérivés (§5) : calculés seulement quand ils ne sont pas posés
/// explicitement (palette, variante, héritage).
fn fill_derived(p: &mut BTreeMap<String, Rgba>, c: &design::Contrast) {
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
        color::ensure_contrast(
            color::from_oklch(color::Oklch { l, ..o }),
            surface,
            f64::from(c.text),
        )
    });
    p.entry("border".to_string())
        .or_insert(color::with_alpha(fg, 0.18));
    let ink = |on: Rgba| {
        let blanc = Rgba(255, 255, 255, 255);
        let noir = Rgba(0, 0, 0, 255);
        if color::contrast(blanc, on) >= color::contrast(noir, on) {
            blanc
        } else {
            noir
        }
    };
    p.entry("accent_fg".to_string()).or_insert(ink(accent));
    // Action (commandes) et sélection (MLG-267) : l'accent s'ils manquent,
    // chacun avec son encre.
    for (slot, on) in [("action", "action_fg"), ("selection", "selection_fg")] {
        let c = *p.entry(slot.to_string()).or_insert(accent);
        p.entry(on.to_string()).or_insert(ink(c));
    }
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
            .or_insert_with(|| color::ensure_contrast(color::from_oklch(o), bg, f64::from(c.text)));
    }
    p.entry("background".to_string())
        .or_insert(color::darken(bg, 4.0));
    // Focus clavier (MLG-282) : l'accent, distinct de la sélection quand le
    // pack les sépare.
    p.entry("focus".to_string()).or_insert(accent);
    for (alias, slot) in ALIASES {
        if let Some(c) = p.get(slot).copied() {
            p.entry(alias.to_string()).or_insert(c);
        }
    }
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
    fill_derived(&mut palette, &design::Contrast::default());
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
        design: Design::default(),
        wallpaper: None,
        shape: BTreeMap::new(),
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
        let parents: Vec<_> = r.warnings.iter().filter(|w| w.contains("parent")).collect();
        assert_eq!(parents.len(), 1, "{:?}", r.warnings);
        assert!(parents[0].contains("fantôme"));
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
    fn transparence_adaptative_déclarée_héritée_coupée() {
        // MLG-261 : `adaptive` + `opacity_max` ; une variante qui ne dit
        // rien en hérite, une variante peut la couper.
        let (manifest, _) = Manifest::parse(
            "base",
            "[meta]\nid = \"base\"\nname = \"base\"\nversion = \"1\"\n[palette]\nbg = \"#000000\"\nfg = \"#ffffff\"\naccent = \"#111111\"\n[effects.bar]\nopacity = 0.6\nadaptive = true\nopacity_max = 0.9\n[effects.widgets]\nopacity = 0.8\nadaptive = true\n[variant.light.effects.bar]\nopacity = 0.5\n[variant.light.effects.widgets]\nopacity = 0.8\nadaptive = false\n",
        )
        .unwrap();
        let base = use_manifest(manifest);
        let sombre = resolve(std::slice::from_ref(&base), "dark").unwrap();
        assert_eq!(sombre.effects[&Place::Bar].adaptive, Some(0.9));
        assert_eq!(sombre.effects[&Place::Widgets].adaptive, Some(1.0));
        assert_eq!(sombre.effects[&Place::Menus].adaptive, None);
        let clair = resolve(&[base], "light").unwrap();
        assert_eq!(clair.effects[&Place::Bar].adaptive, Some(0.9));
        assert!((clair.effects[&Place::Bar].opacity - 0.5).abs() < 1e-6);
        assert_eq!(clair.effects[&Place::Widgets].adaptive, None);
        // Le thème réglé qui en découle garde l'adaptatif (pack perso du
        // pont, aller-retour par le manifeste).
        let t = super::super::bridge::to_theme(&sombre);
        assert_eq!(t.effect(Place::Bar).adaptive, Some(0.9));
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
            assert!(color::contrast(m, r.palette["surface"]) >= 4.5);
        }
        // Thème sombre : 65 % de la luminosité de fg, relevé jusqu'à la
        // cible du texte (MLG-269).
        let r = from_adjusted(
            Rgba(0xf2, 0xf4, 0xfc, 0xff),
            Rgba(0x17, 0x1b, 0x27, 0xff),
            Rgba(0xa6, 0x9c, 0xff, 0xff),
            Rgba(0x17, 0x1b, 0x27, 0xff),
        );
        let o = color::to_oklch(Rgba(0xf2, 0xf4, 0xfc, 0xff));
        let attendu = color::ensure_contrast(
            color::from_oklch(color::Oklch { l: o.l * 0.65, ..o }),
            r.palette["surface"],
            4.5,
        );
        assert_eq!(r.palette["fg_muted"], attendu);
    }

    fn manifest(text: &str) -> Pack {
        let (m, w) = Manifest::parse("essai", text).unwrap();
        assert!(w.is_empty(), "{w:?}");
        use_manifest(m)
    }

    const BASE: &str = "[meta]\nid = \"essai\"\nname = \"Essai\"\nversion = \"1\"\n[palette]\nbg = \"#171b27\"\nfg = \"#f2f4fc\"\naccent = \"#a69cff\"\n";

    #[test]
    fn verre_préréglages_et_surcharges() {
        let p = manifest(&format!(
            "{BASE}[glass]\nhighlight = 0.2\nblur_size = 12\n[glass.presets.brume]\nopacity = 0.6\nrefraction = 0.5\n\
             [effects.launcher]\npreset = \"dense\"\nrefraction = 0.4\n[effects.bar]\npreset = \"brume\"\n\
             [effects.menus]\npreset = \"tinted\"\n[variant.dark.effects.widgets]\nblur = true\n"
        ));
        let r = resolve(&[p], "dark").unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        let l = r.effects[&Place::Launcher];
        assert!((l.opacity - 0.92).abs() < 1e-6);
        let g = r.design.glass(Place::Launcher);
        assert!(
            (g.refraction - 0.4).abs() < 1e-6,
            "surcharge après le préréglage"
        );
        assert!((g.highlight - 0.05).abs() < 1e-6, "valeur du préréglage");
        assert!((r.effects[&Place::Bar].opacity - 0.6).abs() < 1e-6);
        assert!(
            (r.design.glass(Place::Bar).highlight - 0.2).abs() < 1e-6,
            "verre commun"
        );
        // Teinte d'un préréglage : le slot résolu.
        let t = r.design.glass(Place::Menus).tint_rgba().unwrap();
        assert_eq!(t, r.palette["accent"]);
        // MLG-266 : une variante qui ne donne que le flou garde l'opacité.
        assert!(
            (r.effects[&Place::Widgets].opacity - Place::Widgets.default_effect().opacity).abs()
                < 1e-6
        );
        assert!(r.effects[&Place::Widgets].blur);
        assert_eq!(r.hyprland_options["decoration:blur:size"], "12");
    }

    #[test]
    fn jetons_contraste_typo_états_mouvement_écrans() {
        let p = manifest(&format!(
            "{BASE}[contrast]\ntext = 7\nreduced_transparency = true\n[fonts]\nsize = 15\nweight = \"medium\"\ntabular_numbers = true\n\
             [fonts.sizes]\ntitle = 20\n[states]\nhover = 0.1\n[motion]\nduration = 240\neasing = \"cubic-bezier(0.2, 0.8, 0.2, 1)\"\n\
             [screens.\"2570x720\"]\ntext_scale = 0.85\n[screens.\"2570x720\".effects.bar]\nopacity = 0.5\n"
        ));
        let r = resolve(&[p], "dark").unwrap();
        let d = &r.design;
        assert_eq!(d.contrast.text, 7.0);
        assert!(d.contrast.reduced_transparency);
        assert!(color::contrast(r.palette["fg_muted"], r.palette["surface"]) >= 7.0 - 1e-6);
        assert_eq!(
            (d.typography.body, d.typography.title, d.typography.weight),
            (15.0, 20.0, 500)
        );
        assert!(d.typography.tabular_numbers);
        assert_eq!(d.states.hover, 0.1);
        assert_eq!(d.motion.duration, 240);
        let s = d.screen("HDMI-A-1", Some((2570, 720))).unwrap();
        assert_eq!(s.text_scale, 0.85);
        assert_eq!(s.effects["bar"].opacity, 0.5);
        assert!(d.screen("DP-1", Some((1920, 1080))).is_none());
    }

    #[test]
    fn polices_et_icônes_du_plus_proche() {
        let parent = manifest(&format!(
            "{BASE}[fonts]\nfamily = \"Parent\"\nsize = 12\n[icons]\ntheme = \"Papirus\"\n"
        ));
        let enfant = manifest(
            "[meta]\nid = \"essai\"\nname = \"E\"\nversion = \"1\"\n[fonts]\nsize = 14\n[icons]\ntheme = \"Adwaita\"\n",
        );
        let r = resolve(&[parent, enfant], "dark").unwrap();
        let f = r.fonts.unwrap();
        assert_eq!(f.family.as_deref(), Some("Parent"), "champ hérité");
        assert_eq!(f.size, Some(14.0), "le plus proche gagne");
        assert_eq!(r.icons.unwrap().theme.as_deref(), Some("Adwaita"));
    }

    #[test]
    fn fond_du_pack_selon_la_variante() {
        let dir = std::env::temp_dir().join(format!("hd-fond-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("fonds")).unwrap();
        std::fs::write(dir.join("fonds/nuit.jpg"), b"x").unwrap();
        std::fs::write(dir.join("fonds/jour.jpg"), b"x").unwrap();
        let mut p = manifest(&format!(
            "{BASE}[wallpaper]\nimage = \"fonds/nuit.jpg\"\n[variant.light.wallpaper]\nimage = \"fonds/jour.jpg\"\n"
        ));
        p.dir = Some(dir.clone());
        let nuit = resolve(std::slice::from_ref(&p), "dark")
            .unwrap()
            .wallpaper
            .unwrap();
        assert_eq!(nuit.path, dir.join("fonds/nuit.jpg"));
        assert!(!nuit.slideshow);
        let jour = resolve(std::slice::from_ref(&p), "light")
            .unwrap()
            .wallpaper
            .unwrap();
        assert_eq!(jour.path, dir.join("fonds/jour.jpg"));
        let dossier = manifest(&format!(
            "{BASE}[wallpaper]\ndir = \"fonds\"\nmode = \"slideshow\"\ninterval = 600\n"
        ));
        let dossier = Pack {
            dir: Some(dir.clone()),
            ..dossier
        };
        let w = resolve(&[dossier], "dark").unwrap().wallpaper.unwrap();
        assert!(w.slideshow && w.interval == 600);
        assert!(
            Manifest::parse(
                "essai",
                &format!("{BASE}[wallpaper]\nimage = \"../x.jpg\"\n")
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn matériaux_rôles_et_secours() {
        let p = manifest(&format!(
            "{BASE}text_secondary = \"#c0c4d0\"\nfocus = \"#2dd4bf\"\n\
             [glass]\nscroll_edge = false\n\
             [effects.bar]\nmaterial = \"transparent\"\n\
             [effects.launcher]\nmaterial = \"glass_regular\"\nfallback = \"content_surface\"\n\
             [effects.content]\nmaterial = \"content_surface\"\n\
             [fonts.sizes]\nlabel = 14\nmetric = 40\n"
        ));
        let r = resolve(&[p], "dark").unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(r.effects[&Place::Bar].opacity, 0.0);
        assert!(r.effects[&Place::Launcher].adaptive.is_some());
        assert!((r.effects[&Place::Content].opacity - 0.94).abs() < 1e-6);
        assert!((r.design.fallback["launcher"] - 0.94).abs() < 1e-6);
        assert!(!r.design.scroll_edge);
        // Rôles : nom du brief et slot, les deux présents.
        assert_eq!(r.palette["fg_muted"], r.palette["text_secondary"]);
        assert_eq!(r.palette["text_primary"], r.palette["fg"]);
        assert_eq!(r.palette["error"], r.palette["danger"]);
        assert_eq!(r.palette["focus"], Rgba(0x2d, 0xd4, 0xbf, 0xff));
        assert_eq!(
            (r.design.typography.label, r.design.typography.metric),
            (14.0, 40.0)
        );
    }
}
