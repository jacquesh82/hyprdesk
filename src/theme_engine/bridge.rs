//! Pont entre le démon et le moteur (MLG-231) : le moteur fait foi.
//!
//! Le pack actif et sa variante (`active.json`) sont la vérité ; les clés
//! `theme.*` de `settings.lua` (nom, couleurs, effets) en sont le miroir,
//! lu par toutes les surfaces de hyprdesk (barre, Réglages, bureau,
//! verrouillage, connexion…).
//!
//! - Réglages → moteur ([`adopt`]) : un thème réglé dans hyprdesk devient le
//!   pack qui a exactement cette apparence (préréglage, pack installé), ou à
//!   défaut le pack utilisateur [`CUSTOM_ID`], réécrit à chaque retouche.
//! - Moteur → Réglages ([`active`], [`to_theme`]) : un thème posé hors du
//!   démon (`hyprdesk-theme-engine use`, LYRA, `watch`) est recopié dans le
//!   miroir ; au démarrage, le moteur gagne.

use std::path::PathBuf;

use crate::config::{Effect, Place, Rgba, Theme};

use super::color;
use super::paths::Roots;
use super::resolve::{self, Resolved};
use super::state::{self, ActiveTheme};
use super::{Error, Result, bundled, packs};

/// Pack utilisateur qui porte un thème retouché dans hyprdesk.
pub const CUSTOM_ID: &str = "perso";

/// Thème actif du moteur, résolu.
#[derive(Debug, Clone)]
pub struct Active {
    pub active: ActiveTheme,
    pub resolved: Resolved,
    /// Dossier du pack (gabarits) ; `None` pour un pack embarqué.
    pub dir: Option<PathBuf>,
}

/// Résout le thème actif (`active.json`), s'il est posé et résoluble.
pub fn active(roots: &Roots) -> Option<Active> {
    let active = state::read_active(roots)?;
    resolve_pack(roots, &active.pack, &active.variant)
        .ok()
        .map(|(resolved, dir)| Active {
            active,
            resolved,
            dir,
        })
}

fn resolve_pack(roots: &Roots, id: &str, variant: &str) -> Result<(Resolved, Option<PathBuf>)> {
    let (all, _) = packs::discover(roots, &bundled::packs())?;
    let chain = packs::chain(&all, id)?;
    // Pack sans variantes : la sienne, quelle que soit celle notée dans
    // active.json (Clair noté `dark` avant MLG-254).
    let variant = match packs::find(&all, id) {
        Some(p) if p.manifest.variant.is_empty() => p.manifest.default_variant(),
        _ => variant,
    };
    let resolved = resolve::resolve(&chain, variant)?;
    let dir = packs::find(&all, id).and_then(|p| p.dir.clone());
    Ok((resolved, dir))
}

/// Ce que le démon garde d'une résolution : nom, couleurs, effets.
pub fn to_theme(r: &Resolved) -> Theme {
    let d = Theme::default();
    let slot = |name: &str, fallback: Rgba| r.palette.get(name).copied().unwrap_or(fallback);
    Theme {
        name: r.name.clone(),
        fg: slot("fg", d.fg),
        bg: slot("bg", d.bg),
        accent: slot("accent", d.accent),
        effects: Place::ALL
            .iter()
            .map(|p| {
                (
                    *p,
                    r.effects
                        .get(p)
                        .copied()
                        .unwrap_or_else(|| p.default_effect()),
                )
            })
            .collect(),
    }
}

/// Même apparence : couleurs et effet de chaque surface (le nom à part).
pub fn same_look(a: &Theme, b: &Theme) -> bool {
    a.fg == b.fg
        && a.bg == b.bg
        && a.accent == b.accent
        && Place::ALL.iter().all(|p| {
            let (x, y) = (a.effect(*p), b.effect(*p));
            x.blur == y.blur
                && (x.opacity - y.opacity).abs() < 1e-3
                && match (x.adaptive, y.adaptive) {
                    (None, None) => true,
                    (Some(a), Some(b)) => (a - b).abs() < 1e-3,
                    _ => false,
                }
        })
}

/// Même thème pour le miroir : apparence et nom.
pub fn same_theme(a: &Theme, b: &Theme) -> bool {
    a.name == b.name && same_look(a, b)
}

/// Une entrée du catalogue : un pack découvert, dans sa variante par défaut.
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub origin: &'static str,
    pub variant: String,
    pub theme: Theme,
    /// L'autre variante, quand le pack en déclare une d'apparence
    /// différente (MLG-254) : nom (`light`/`dark`) et thème.
    pub other: Option<(String, Theme)>,
}

/// Packs disponibles (embarqués, utilisateur, système) dans leur variante
/// par défaut, triés comme les découvre le moteur ; le pack
/// [`CUSTOM_ID`] n'y figure que s'il existe.
pub fn catalog(roots: &Roots) -> Vec<Entry> {
    let Ok((all, _)) = packs::discover(roots, &bundled::packs()) else {
        return Vec::new();
    };
    all.iter()
        .filter_map(|p| {
            let id = p.manifest.meta.id.clone();
            let variant = p.manifest.default_variant().to_string();
            let chain = packs::chain(&all, &id).ok()?;
            let resolved = resolve::resolve(&chain, &variant).ok()?;
            let theme = to_theme(&resolved);
            let autre = super::manifest::VARIANTS
                .iter()
                .find(|v| **v != variant)
                .map(|v| v.to_string());
            let other = autre.and_then(|v| {
                let t = to_theme(&resolve::resolve(&chain, &v).ok()?);
                (!same_look(&t, &theme)).then_some((v, t))
            });
            Some(Entry {
                id,
                origin: p.origin.as_str(),
                variant,
                theme,
                other,
            })
        })
        .collect()
}

/// Pose `theme` comme thème actif du moteur et publie sa palette.
///
/// Par ordre : le pack actif s'il a déjà ce thème (rien n'est réécrit) ;
/// sinon un pack découvert qui a cette apparence, dans l'une de ses
/// variantes (même nom d'abord) ; sinon le pack utilisateur [`CUSTOM_ID`],
/// réécrit avec ces couleurs et ces effets.
pub fn adopt(roots: &Roots, theme: &Theme, origin: &str) -> Result<Active> {
    if let Some(cur) = active(roots)
        && same_theme(&to_theme(&cur.resolved), theme)
    {
        publish(roots, &cur.resolved)?;
        return Ok(cur);
    }
    let (all, _) = packs::discover(roots, &bundled::packs())?;
    let mut trouvé: Option<(String, String, bool)> = None;
    for p in &all {
        let id = &p.manifest.meta.id;
        if id == CUSTOM_ID {
            continue;
        }
        let Ok(chain) = packs::chain(&all, id) else {
            continue;
        };
        // Variante par défaut du pack d'abord : un pack sans variantes a la
        // même apparence dans les deux, et Clair doit rester `light`.
        let défaut = p.manifest.default_variant();
        let variantes = std::iter::once(défaut).chain(
            super::manifest::VARIANTS
                .iter()
                .copied()
                .filter(|v| *v != défaut),
        );
        for variant in variantes {
            let Ok(r) = resolve::resolve(&chain, variant) else {
                continue;
            };
            let t = to_theme(&r);
            if same_look(&t, theme) {
                let même_nom = t.name == theme.name;
                if trouvé.as_ref().is_none_or(|(_, _, n)| !n && même_nom) {
                    trouvé = Some((id.clone(), variant.to_string(), même_nom));
                }
            }
        }
    }
    let (id, variant) = match trouvé {
        Some((id, variant, _)) => (id, variant),
        None => {
            write_custom(roots, theme)?;
            (CUSTOM_ID.to_string(), variant_of(theme).to_string())
        }
    };
    state::write_active(
        roots,
        &ActiveTheme {
            pack: id.clone(),
            variant: variant.clone(),
            origin: origin.to_string(),
        },
    )?;
    let (resolved, dir) = resolve_pack(roots, &id, &variant)?;
    publish(roots, &resolved)?;
    let _ = state::journal(
        roots,
        &format!("use pack={id} variante={variant} origine={origin}"),
    );
    Ok(Active {
        active: state::read_active(roots)
            .ok_or_else(|| Error::Missing("active.json".to_string()))?,
        resolved,
        dir,
    })
}

/// Pack qui porte exactement l'apparence de `theme` : le pack actif s'il
/// l'a, sinon le premier pack découvert qui l'a (même nom d'abord, variante
/// par défaut d'abord). Sans lecture d'active.json au-delà du pack actif :
/// sert à `genfiles` au moment même du changement de thème (MLG-255).
pub fn resolved_for_look(roots: &Roots, theme: &Theme) -> Option<Resolved> {
    if let Some(a) = active(roots)
        && same_look(&to_theme(&a.resolved), theme)
    {
        return Some(a.resolved);
    }
    let (all, _) = packs::discover(roots, &bundled::packs()).ok()?;
    let mut trouvé: Option<(Resolved, bool)> = None;
    for p in &all {
        let Ok(chain) = packs::chain(&all, &p.manifest.meta.id) else {
            continue;
        };
        let défaut = p.manifest.default_variant();
        let variantes = std::iter::once(défaut).chain(
            super::manifest::VARIANTS
                .iter()
                .copied()
                .filter(|v| *v != défaut),
        );
        for variant in variantes {
            let Ok(r) = resolve::resolve(&chain, variant) else {
                continue;
            };
            if same_look(&to_theme(&r), theme) {
                let même_nom = r.name == theme.name;
                if trouvé.as_ref().is_none_or(|(_, n)| !n && même_nom) {
                    trouvé = Some((r, même_nom));
                }
            }
        }
    }
    trouvé.map(|(r, _)| r)
}

/// Résolution à appliquer pour le thème du démon : celle du pack actif
/// (et son dossier, pour ses gabarits) quand il porte ce thème, sinon la
/// palette implicite calculée depuis le thème lui-même.
pub fn resolution_for(roots: &Roots, theme: &Theme) -> (Resolved, Option<PathBuf>) {
    match active(roots) {
        Some(a) if same_look(&to_theme(&a.resolved), theme) => (a.resolved, a.dir),
        _ => (resolve::from_theme(theme), None),
    }
}

/// Publie la palette : à chaque changement de thème, que des applications
/// soient cochées ou non (MLG-232).
fn publish(roots: &Roots, resolved: &Resolved) -> Result<u64> {
    state::publish_palette(roots, resolved)
}

/// Variante d'un thème retouché : claire si son fond est clair.
fn variant_of(theme: &Theme) -> &'static str {
    if color::luminance(theme.bg) > 0.5 {
        "light"
    } else {
        "dark"
    }
}

/// Écrit le pack utilisateur [`CUSTOM_ID`] : nom, couleurs et effets du
/// thème, rien d'autre (les slots dérivés sont recalculés par le moteur).
pub fn write_custom(roots: &Roots, theme: &Theme) -> Result<PathBuf> {
    let dir = roots.themes_dir().join(CUSTOM_ID);
    let name = if theme.name.trim().is_empty() {
        "Personnalisé".to_string()
    } else {
        theme.name.trim().to_string()
    };
    let mut meta = toml::Table::new();
    meta.insert("id".into(), CUSTOM_ID.into());
    meta.insert("name".into(), name.into());
    meta.insert("version".into(), "1".into());
    meta.insert(
        "description".into(),
        "Thème réglé dans hyprdesk (couleurs et effets).".into(),
    );
    meta.insert("default_variant".into(), variant_of(theme).into());
    let mut palette = toml::Table::new();
    for (slot, c) in [("bg", theme.bg), ("fg", theme.fg), ("accent", theme.accent)] {
        palette.insert(slot.into(), color::fmt(c, color::Fmt::Hex).into());
    }
    let mut effects = toml::Table::new();
    for p in Place::ALL.iter().copied() {
        let Effect {
            opacity,
            blur,
            adaptive,
        } = theme.effect(p);
        let mut e = toml::Table::new();
        let round = |v: f32| (f64::from(v) * 10_000.0).round() / 10_000.0;
        e.insert("opacity".into(), round(opacity).into());
        e.insert("blur".into(), blur.into());
        if let Some(max) = adaptive {
            e.insert("adaptive".into(), true.into());
            e.insert("opacity_max".into(), round(max).into());
        }
        effects.insert(p.as_str().into(), e.into());
    }
    let mut doc = toml::Table::new();
    doc.insert("meta".into(), meta.into());
    doc.insert("palette".into(), palette.into());
    doc.insert("effects".into(), effects.into());
    // Les couleurs choisies dans hyprdesk sont rendues telles quelles : le
    // contraste garanti (MLG-269) vaut pour les packs, pas pour un choix
    // explicite de l'utilisateur, que Réglages → Thème signale.
    let mut contrast = toml::Table::new();
    contrast.insert("enforce".into(), false.into());
    doc.insert("contrast".into(), contrast.into());
    let text =
        toml::to_string(&doc).map_err(|e| Error::Manifest(CUSTOM_ID.to_string(), e.to_string()))?;
    // Relu avant d'écrire : un pack invalide n'est jamais posé.
    super::manifest::Manifest::parse(CUSTOM_ID, &text)?;
    state::atomic_write(&dir.join(packs::MANIFEST_FILE), &text)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots(tag: &str) -> (PathBuf, Roots) {
        let root = std::env::temp_dir().join(format!("hd-bridge-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        (root.clone(), Roots::at(&root))
    }

    fn ardoise() -> Theme {
        let p = &crate::theme::PRESETS[0];
        Theme {
            name: p.name.to_string(),
            fg: p.fg,
            bg: p.bg,
            accent: p.accent,
            effects: p.effects(),
        }
    }

    #[test]
    fn préréglage_posé_comme_son_pack() {
        let (root, roots) = roots("preset");
        let a = adopt(&roots, &ardoise(), "user").unwrap();
        assert_eq!(a.active.pack, "ardoise");
        assert_eq!(a.resolved.id, "ardoise");
        // Palette publiée, au nom du vrai pack (plus « regle »).
        let doc = state::read_palette(&roots).unwrap();
        assert_eq!(doc.pack, "ardoise");
        assert!(!roots.themes_dir().join(CUSTOM_ID).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tous_les_préréglages_ont_leur_pack() {
        let (root, roots) = roots("presets");
        for p in crate::theme::PRESETS {
            let t = Theme {
                name: p.name.to_string(),
                fg: p.fg,
                bg: p.bg,
                accent: p.accent,
                effects: p.effects(),
            };
            let a = adopt(&roots, &t, "user").unwrap();
            assert_ne!(a.active.pack, CUSTOM_ID, "{}", p.name);
            assert!(same_theme(&to_theme(&a.resolved), &t), "{}", p.name);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn retouche_en_pack_perso_puis_aller_retour() {
        let (root, roots) = roots("perso");
        let mut t = ardoise();
        t.accent = Rgba(0x12, 0x34, 0x56, 0xff);
        t.name = "Mon thème".to_string();
        let a = adopt(&roots, &t, "user").unwrap();
        assert_eq!(a.active.pack, CUSTOM_ID);
        assert!(roots.themes_dir().join("perso/theme.toml").is_file());
        // Ce que le moteur rend est exactement ce qui a été réglé.
        assert!(same_theme(&to_theme(&a.resolved), &t));
        // Rejouer ne réécrit rien.
        let rev = state::read_palette(&roots).unwrap().revision;
        adopt(&roots, &t, "user").unwrap();
        assert_eq!(state::read_palette(&roots).unwrap().revision, rev);
        // Revenir à Ardoise repose le pack embarqué.
        assert_eq!(
            adopt(&roots, &ardoise(), "user").unwrap().active.pack,
            "ardoise"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn clair_reste_clair_et_copies_figées_ignorées() {
        // MLG-254 : Clair est posé en variante claire, même noté `dark`
        // dans active.json, et une copie 1.0.0 recopiée par `use` ne le
        // masque plus.
        let (root, roots) = roots("clair");
        let p = crate::theme::PRESETS
            .iter()
            .find(|p| p.name == "Clair")
            .unwrap();
        let t = Theme {
            name: p.name.to_string(),
            fg: p.fg,
            bg: p.bg,
            accent: p.accent,
            effects: p.effects(),
        };
        let copie = roots.themes_dir().join("clair");
        std::fs::create_dir_all(&copie).unwrap();
        std::fs::write(
            copie.join("theme.toml"),
            include_str!("packs/anciens/clair-1.0.0.toml"),
        )
        .unwrap();
        let a = adopt(&roots, &t, "user").unwrap();
        assert_eq!(
            (a.active.pack.as_str(), a.active.variant.as_str()),
            ("clair", "light")
        );
        assert_eq!(a.resolved.variant, "light");
        assert!(a.dir.is_none(), "le pack livré, pas la copie");
        let e = catalog(&roots)
            .into_iter()
            .find(|e| e.id == "clair")
            .unwrap();
        assert_eq!((e.origin, e.variant.as_str()), ("bundled", "light"));
        assert!(e.other.is_none());
        // Noté `dark` par une version d'avant : résolu clair quand même.
        state::write_active(
            &roots,
            &ActiveTheme {
                pack: "clair".to_string(),
                variant: "dark".to_string(),
                origin: "cli".to_string(),
            },
        )
        .unwrap();
        assert_eq!(active(&roots).unwrap().resolved.variant, "light");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn autre_variante_au_catalogue() {
        let (root, roots) = roots("variantes");
        let dir = roots.themes_dir().join("duo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("theme.toml"),
            "[meta]\nid = \"duo\"\nname = \"Duo\"\nversion = \"1\"\n\
             [palette]\nbg = \"#171b27\"\nfg = \"#f2f4fc\"\naccent = \"#a69cff\"\n\
             [variant.light]\npalette.bg = \"#f2f4fc\"\npalette.fg = \"#171b27\"\n",
        )
        .unwrap();
        let e = catalog(&roots).into_iter().find(|e| e.id == "duo").unwrap();
        assert_eq!(e.variant, "dark");
        let (v, clair) = e.other.clone().unwrap();
        assert_eq!(v, "light");
        // Choisir la version claire pose bien la variante claire.
        let a = adopt(&roots, &clair, "user").unwrap();
        assert_eq!(
            (a.active.pack.as_str(), a.active.variant.as_str()),
            ("duo", "light")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn thème_posé_hors_du_démon_relu() {
        let (root, roots) = roots("externe");
        state::write_active(
            &roots,
            &ActiveTheme {
                pack: "ambre".to_string(),
                variant: "dark".to_string(),
                origin: "cli".to_string(),
            },
        )
        .unwrap();
        let a = active(&roots).unwrap();
        let t = to_theme(&a.resolved);
        assert_eq!(t.name, "Ambre");
        assert_eq!(t.accent, Rgba(0xff, 0xb8, 0x6c, 0xff));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn thème_par_défaut_du_démon_est_le_pack_ardoise() {
        let (root, roots) = roots("defaut");
        assert!(same_theme(&Theme::default(), &ardoise()));
        assert_eq!(
            adopt(&roots, &Theme::default(), "daemon")
                .unwrap()
                .active
                .pack,
            "ardoise"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn catalogue_des_packs() {
        let (root, roots) = roots("catalogue");
        let c = catalog(&roots);
        assert!(c.iter().any(|e| e.id == "ardoise" && e.origin == "bundled"));
        assert_eq!(c.len(), crate::theme::PRESETS.len());
        let _ = std::fs::remove_dir_all(&root);
    }
}
