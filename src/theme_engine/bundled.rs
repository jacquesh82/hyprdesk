//! Packs embarqués : les préréglages de `crate::theme`, convertis en packs
//! (mêmes couleurs, mêmes effets de surface). Visibles par `list` avec
//! `origin = "bundled"`, lus depuis le binaire : ils ne sont plus recopiés
//! dans le dossier utilisateur à la sélection (MLG-254), et une copie restée
//! telle quelle d'une version livrée est ignorée ([`unmodified_copy`]) pour
//! ne pas masquer les corrections suivantes.

use super::manifest::Manifest;
use super::packs::{Origin, Pack};

/// Packs embarqués : identifiant et texte du manifeste.
const PACKS: &[(&str, &str)] = &[
    ("ardoise", include_str!("packs/ardoise.toml")),
    ("verre", include_str!("packs/verre.toml")),
    ("encre", include_str!("packs/encre.toml")),
    ("foret", include_str!("packs/foret.toml")),
    ("ambre", include_str!("packs/ambre.toml")),
    ("clair", include_str!("packs/clair.toml")),
    (
        "contraste-eleve",
        include_str!("packs/contraste-eleve.toml"),
    ),
];

/// Versions livrées précédentes de chaque pack : une copie qui leur est
/// identique (recopiée par `use` avant MLG-254) n'est pas « à vous ».
const ANCIENS: &[(&str, &str)] = &[
    ("ardoise", include_str!("packs/anciens/ardoise-1.0.0.toml")),
    ("verre", include_str!("packs/anciens/verre-1.0.0.toml")),
    ("verre", include_str!("packs/anciens/verre-1.1.0.toml")),
    ("encre", include_str!("packs/anciens/encre-1.0.0.toml")),
    ("foret", include_str!("packs/anciens/foret-1.0.0.toml")),
    ("ambre", include_str!("packs/anciens/ambre-1.0.0.toml")),
    ("clair", include_str!("packs/anciens/clair-1.0.0.toml")),
    (
        "contraste-eleve",
        include_str!("packs/anciens/contraste-eleve-1.0.0.toml"),
    ),
];

/// `text` est le manifeste livré du pack `id`, actuel ou d'une version
/// précédente, sans modification (fins de ligne mises à part).
pub fn unmodified_copy(id: &str, text: &str) -> bool {
    let text = text.trim_end();
    PACKS
        .iter()
        .chain(ANCIENS)
        .any(|(i, t)| *i == id && t.trim_end() == text)
}

/// Packs embarqués chargés et validés (un manifeste invalide embarqué serait
/// une erreur de programme : il est ignoré avec la trace en commentaire).
pub fn packs() -> Vec<Pack> {
    PACKS
        .iter()
        .filter_map(|(id, text)| match Manifest::parse(id, text) {
            Ok((manifest, _)) => Some(Pack {
                origin: Origin::Bundled,
                dir: None,
                manifest,
            }),
            Err(e) => {
                eprintln!("pack embarqué invalide ({id}) : {e}");
                None
            }
        })
        .collect()
}

/// Texte du manifeste embarqué, pour installer le pack dans le dossier
/// utilisateur à la sélection.
pub fn manifest_text(id: &str) -> Option<&'static str> {
    PACKS.iter().find(|(i, _)| *i == id).map(|(_, text)| *text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_sept_préréglages_sont_des_packs_valides() {
        let packs = packs();
        assert_eq!(packs.len(), PACKS.len());
        for p in &packs {
            assert_eq!(p.origin, Origin::Bundled);
            assert!(p.dir.is_none());
        }
        let ardoise = packs
            .iter()
            .find(|p| p.manifest.meta.id == "ardoise")
            .unwrap();
        // Mêmes valeurs que le préréglage d'origine (src/theme.rs).
        let preset = &crate::theme::PRESETS[0];
        assert_eq!(
            ardoise.manifest.palette["fg"],
            format!("#{:02x}{:02x}{:02x}", preset.fg.0, preset.fg.1, preset.fg.2)
        );
        assert_eq!(
            ardoise.manifest.palette["bg"],
            format!("#{:02x}{:02x}{:02x}", preset.bg.0, preset.bg.1, preset.bg.2)
        );
        assert_eq!(
            ardoise.manifest.palette["accent"],
            format!(
                "#{:02x}{:02x}{:02x}",
                preset.accent.0, preset.accent.1, preset.accent.2
            )
        );
    }

    #[test]
    fn effets_repris_du_préréglage() {
        let packs = packs();
        let encre = packs
            .iter()
            .find(|p| p.manifest.meta.id == "encre")
            .unwrap();
        assert_eq!(encre.manifest.effects["bar"].opacity, Some(0.94));
        assert_eq!(encre.manifest.effects["bar"].blur, Some(false));
        let ardoise = packs
            .iter()
            .find(|p| p.manifest.meta.id == "ardoise")
            .unwrap();
        assert_eq!(ardoise.manifest.effects["widgets"].blur, Some(false));
        assert_eq!(ardoise.manifest.effects["bar"].blur, Some(true));
    }

    #[test]
    fn barre_et_widgets_des_packs_livrés_se_densifient() {
        // HIG FON-01 : les packs livrés ne disent rien de `adaptive` pour la
        // barre et les widgets ; ils héritent du défaut, qui les densifie
        // selon le fond d'écran, plafond 0,95.
        for p in packs() {
            for variant in ["dark", "light"] {
                let r = super::super::resolve::resolve(std::slice::from_ref(&p), variant).unwrap();
                let id = &p.manifest.meta.id;
                assert_eq!(
                    r.effects[&crate::config::Place::Bar].adaptive,
                    Some(0.95),
                    "{id} {variant}"
                );
                assert_eq!(
                    r.effects[&crate::config::Place::Widgets].adaptive,
                    Some(0.95),
                    "{id} {variant}"
                );
            }
        }
    }

    #[test]
    fn packs_livrés_complets_sans_avertissement() {
        // MLG-254 : métadonnées complètes, variante déclarée et cohérente
        // avec le fond, aucun champ inconnu.
        for (id, text) in PACKS {
            let (m, warnings) = Manifest::parse(id, text).unwrap();
            assert!(warnings.is_empty(), "{id} : {warnings:?}");
            assert!(!m.meta.authors.is_empty(), "{id}");
            assert!(
                m.meta.description.is_some() && m.meta.license.is_some(),
                "{id}"
            );
            assert!(m.meta.min_engine.is_some(), "{id}");
            let déclarée = m.meta.default_variant.clone().unwrap_or_default();
            let bg = super::super::color::parse(&m.palette["bg"]).unwrap();
            let déduite = if super::super::color::luminance(bg) > 0.5 {
                "light"
            } else {
                "dark"
            };
            assert_eq!(déclarée, déduite, "{id}");
            for surface in ["bar", "menus", "launcher", "settings", "widgets"] {
                assert!(m.effects.contains_key(surface), "{id} : {surface}");
            }
        }
    }

    #[test]
    fn packs_livrés_au_contraste_sans_correction() {
        // MLG-269 : aucun slot d'un pack livré ne manque sa cible.
        for (id, text) in PACKS {
            let (manifest, _) = Manifest::parse(id, text).unwrap();
            let pack = super::super::packs::Pack {
                origin: super::super::packs::Origin::Bundled,
                dir: None,
                manifest,
            };
            for v in ["dark", "light"] {
                let r = super::super::resolve::resolve(std::slice::from_ref(&pack), v).unwrap();
                assert!(r.warnings.is_empty(), "{id} {v} : {:?}", r.warnings);
            }
        }
    }

    #[test]
    fn copie_inchangée_reconnue() {
        assert!(unmodified_copy(
            "clair",
            include_str!("packs/anciens/clair-1.0.0.toml")
        ));
        assert!(unmodified_copy("clair", manifest_text("clair").unwrap()));
        let retouchée = manifest_text("clair")
            .unwrap()
            .replace("#5b5bd6", "#ff0000");
        assert!(!unmodified_copy("clair", &retouchée));
        assert!(!unmodified_copy("verre", manifest_text("clair").unwrap()));
    }

    #[test]
    fn texte_du_manifeste_disponible() {
        assert!(manifest_text("ardoise").is_some());
        assert!(manifest_text("inconnu").is_none());
    }
}
