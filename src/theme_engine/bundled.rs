//! Packs embarqués : les préréglages de `crate::theme`, convertis en packs
//! (mêmes couleurs, mêmes effets de surface). Visibles par `list` avec
//! `origin = "bundled"` ; installés dans le dossier utilisateur à la
//! sélection (`use`), sans écraser une modification locale éventuelle.

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
        assert_eq!(encre.manifest.effects["bar"].opacity, 0.94);
        assert_eq!(encre.manifest.effects["bar"].blur, Some(false));
        let ardoise = packs
            .iter()
            .find(|p| p.manifest.meta.id == "ardoise")
            .unwrap();
        assert_eq!(ardoise.manifest.effects["widgets"].blur, Some(false));
        assert_eq!(ardoise.manifest.effects["bar"].blur, Some(true));
    }

    #[test]
    fn texte_du_manifeste_disponible() {
        assert!(manifest_text("ardoise").is_some());
        assert!(manifest_text("inconnu").is_none());
    }
}
