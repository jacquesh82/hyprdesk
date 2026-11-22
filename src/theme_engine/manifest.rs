//! Manifeste `theme.toml` d'un pack (§4 de la spécification) : lecture TOML,
//! champs inconnus ignorés avec avertissement (compatibilité ascendante des
//! packs tiers), validation des limites et de l'identifiant.
//!
//! Un pack est des données : rien ici n'exécute quoi que ce soit du contenu.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{Error, Result};

/// Taille maximale d'un manifeste (§14 : 64 Kio).
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;

/// Clés de premier niveau connues ; toute autre produit un avertissement.
const KNOWN_TOP: &[&str] = &[
    "meta",
    "inherits",
    "palette",
    "variant",
    "effects",
    "hyprland",
    "fonts",
    "icons",
    "cursors",
    "sounds",
    "wallpaper",
    "targets",
];

/// Cartouche du pack.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Meta {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub screenshot: Option<String>,
    #[serde(default)]
    pub default_variant: Option<String>,
    #[serde(default)]
    pub min_engine: Option<String>,
}

/// Racine du manifeste : tout est optionnel sauf `meta`, les valeurs manquantes
/// sont des défauts du moteur ou de l'héritage.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Manifest {
    pub meta: Meta,
    /// Packs parents, ordre = priorité croissante (le dernier gagne).
    #[serde(default)]
    pub inherits: Inherits,
    /// Slots nommés → couleur (slots sémantiques du §5 et couleurs libres).
    #[serde(default)]
    pub palette: BTreeMap<String, String>,
    /// Variantes plates (surcharges), noms réservés `dark` et `light`.
    #[serde(default)]
    pub variant: BTreeMap<String, VariantSpec>,
    /// Effets par surface (`bar`, `menus`, `launcher`, `settings`, `widgets`).
    #[serde(default)]
    pub effects: BTreeMap<String, EffectSpec>,
    /// Options Hyprland en liste blanche (§8).
    #[serde(default)]
    pub hyprland: HyprlandSpec,
    /// Typographie (cible `fonts`, phase 2).
    #[serde(default)]
    pub fonts: Option<Fonts>,
    /// Thème d'icônes (cible `icons`, phase 2).
    #[serde(default)]
    pub icons: Option<Named>,
    /// Thème de curseur (cible `cursors`, phase 2).
    #[serde(default)]
    pub cursors: Option<Cursors>,
    /// Thème de sons (cible `sounds`, phase 2).
    #[serde(default)]
    pub sounds: Option<Named>,
}

/// `[inherits] themes = [...]`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Inherits {
    pub themes: Vec<String>,
}

/// Surcharges d'une variante : palette, effets, options Hyprland.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct VariantSpec {
    pub palette: BTreeMap<String, String>,
    pub effects: BTreeMap<String, EffectSpec>,
    pub hyprland_options: BTreeMap<String, String>,
}

/// Effet d'une surface : opacité du fond (0..1) et flou derrière (absent :
/// la valeur en place du moteur est conservée).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(default)]
pub struct EffectSpec {
    pub opacity: f64,
    pub blur: Option<bool>,
}

impl Default for EffectSpec {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            blur: None,
        }
    }
}

/// `[hyprland] options = { ... }` : chemins hyprlang → valeurs.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct HyprlandSpec {
    pub options: BTreeMap<String, String>,
}

/// `[fonts]`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Fonts {
    pub family: Option<String>,
    pub mono_family: Option<String>,
    pub size: Option<f64>,
}

/// Thème nommé générique (icônes, sons).
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Named {
    pub theme: Option<String>,
}

/// `[cursors]`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Cursors {
    pub theme: Option<String>,
    pub size: Option<u16>,
}

/// Variante résolue : les deux noms réservés.
pub const VARIANTS: [&str; 2] = ["dark", "light"];

/// Variante par défaut quand le pack n'en déclare pas.
pub const DEFAULT_VARIANT: &str = "dark";

impl Manifest {
    /// Lit un manifeste : limite de taille, champs inconnus en avertissement,
    /// validation de `meta`. `id` sert aux messages d'erreur.
    pub fn parse(id: &str, text: &str) -> Result<(Self, Vec<String>)> {
        if text.len() > MAX_MANIFEST_BYTES {
            return Err(Error::Manifest(
                id.to_string(),
                format!(
                    "manifeste trop grand ({} > {} octets)",
                    text.len(),
                    MAX_MANIFEST_BYTES
                ),
            ));
        }
        let value: toml::Value =
            toml::from_str(text).map_err(|e| Error::Manifest(id.to_string(), e.to_string()))?;
        let mut warnings = Vec::new();
        if let Some(table) = value.as_table() {
            for key in table.keys() {
                if !KNOWN_TOP.contains(&key.as_str()) {
                    warnings.push(format!("champ inconnu ignoré : {key}"));
                }
            }
        }
        let manifest: Manifest = Manifest::deserialize(value)
            .map_err(|e| Error::Manifest(id.to_string(), e.to_string()))?;
        manifest.validate(id)?;
        Ok((manifest, warnings))
    }

    /// Validation : identifiant borné, nom et version présents, variante par
    /// défaut connue, options Hyprland en liste blanche, slots décodables.
    pub fn validate(&self, id: &str) -> Result<()> {
        let meta = &self.meta;
        if meta.id != id {
            return Err(Error::Manifest(
                id.to_string(),
                format!(
                    "l'identifiant du manifeste « {} » ne correspond pas au pack",
                    meta.id
                ),
            ));
        }
        if !valid_id(&meta.id) {
            return Err(Error::Manifest(
                id.to_string(),
                "identifiant attendu : 2 à 32 caractères [a-z0-9-]".to_string(),
            ));
        }
        if meta.name.trim().is_empty() || meta.version.trim().is_empty() {
            return Err(Error::Manifest(
                id.to_string(),
                "nom et version requis".to_string(),
            ));
        }
        if let Some(v) = &meta.default_variant
            && !VARIANTS.contains(&v.as_str())
        {
            return Err(Error::Manifest(
                id.to_string(),
                format!("variante par défaut inconnue : {v}"),
            ));
        }
        for key in self.hyprland.options.keys() {
            if !allowed_hyprland_key(key) {
                return Err(Error::Manifest(
                    id.to_string(),
                    format!("option Hyprland hors liste blanche : {key}"),
                ));
            }
        }
        for (key, value) in self
            .palette
            .iter()
            .chain(self.variant.values().flat_map(|v| v.palette.iter()))
        {
            if crate::theme_engine::color::parse(value).is_none() {
                return Err(Error::Color(
                    key.clone(),
                    format!("« {value} » (attendu : #rrggbb, rgb(), rgba() ou hsl())"),
                ));
            }
        }
        Ok(())
    }

    /// Variante par défaut effective du pack.
    pub fn default_variant(&self) -> &'static str {
        match self.meta.default_variant.as_deref() {
            Some("light") => "light",
            _ => DEFAULT_VARIANT,
        }
    }
}

/// Identifiant de pack : 2 à 32 caractères `[a-z0-9-]`.
pub fn valid_id(id: &str) -> bool {
    (2..=32).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
}

/// Liste blanche des options Hyprland que peut régler un pack (§8) :
/// décoration, espacement et bordures, animations, comportement du flou.
pub fn allowed_hyprland_key(key: &str) -> bool {
    key.starts_with("decoration:")
        || key.starts_with("general:gaps_")
        || key.starts_with("general:border")
        || key.starts_with("animation:")
        || key.starts_with("misc:")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARDOISE: &str = r##"
[meta]
id = "ardoise"
name = "Ardoise"
version = "1.0.0"
default_variant = "dark"

[inherits]
themes = ["hyprdesk-base"]

[palette]
bg = "#171b27"
fg = "#f2f4fc"
accent = "#a69cff"

[variant.light]
palette.fg = "#171b27"
palette.bg = "#eef0f8"

[effects.bar]
opacity = 0.72
blur = true

[hyprland.options]
"decoration:rounding" = "10"
"##;

    #[test]
    fn manifeste_complet_lu() {
        let (m, warnings) = Manifest::parse("ardoise", ARDOISE).unwrap();
        assert_eq!(m.meta.id, "ardoise");
        assert_eq!(m.inherits.themes, vec!["hyprdesk-base"]);
        assert_eq!(m.palette["accent"], "#a69cff");
        assert_eq!(m.variant["light"].palette["fg"], "#171b27");
        assert_eq!(m.effects["bar"].blur, Some(true));
        assert_eq!(m.hyprland.options["decoration:rounding"], "10");
        assert!(warnings.is_empty());
        assert_eq!(m.default_variant(), "dark");
    }

    #[test]
    fn champ_inconnu_averti_et_ignoré() {
        let (m, warnings) =
            Manifest::parse("ardoise", &format!("{ARDOISE}\n[futur]\ncle = 1\n")).unwrap();
        assert_eq!(warnings, vec!["champ inconnu ignoré : futur"]);
        assert_eq!(m.meta.id, "ardoise");
    }

    #[test]
    fn identifiants_rejetés_si_invalides() {
        for id in [
            "a",
            "MAJUSCULE",
            "-tiroit",
            "avec_espace",
            "trop-long-qui-depasse-la-limite-des-trente-deux-caracteres",
        ] {
            let m = ARDOISE.replacen("ardoise", id, 2);
            assert!(
                Manifest::parse(id, &m).is_err(),
                "identifiant accepté : {id}"
            );
        }
    }

    #[test]
    fn option_hyprland_hors_liste_blanche_rejetée() {
        let m =
            format!("{ARDOISE}\n[hyprland.options]\n\"bind:SUPER+Q\" = \"exec quelque chose\"\n");
        assert!(Manifest::parse("ardoise", &m).is_err());
    }

    #[test]
    fn couleur_invalide_rejetée() {
        let m = ARDOISE.replace("accent = \"#a69cff\"", "accent = \"rouge\"");
        assert!(Manifest::parse("ardoise", &m).is_err());
    }

    #[test]
    fn manifeste_vide_reffusé_sans_meta() {
        assert!(Manifest::parse("x", "").is_err());
    }
}
