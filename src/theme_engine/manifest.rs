//! Manifeste `theme.toml` d'un pack (§4 de la spécification) : lecture TOML,
//! champs inconnus ignorés avec avertissement (compatibilité ascendante des
//! packs tiers), validation des limites et de l'identifiant.
//!
//! Un pack est des données : rien ici n'exécute quoi que ce soit du contenu.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
    /// Cibles déclarées du pack : surcharge d'une cible intégrée (gabarit)
    /// ou cible générique tierce (gabarit + sortie + rechargement borné).
    #[serde(default)]
    pub targets: BTreeMap<String, PackTarget>,
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
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Fonts {
    pub family: Option<String>,
    pub mono_family: Option<String>,
    pub size: Option<f64>,
}

/// Cible déclarée par le pack (§4) : surcharge ou cible générique.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct PackTarget {
    /// Gabarit du pack, chemin relatif au dossier du pack.
    pub template: Option<String>,
    /// Chemin de sortie (requis pour une cible générique ; `~/` accepté).
    pub output: Option<String>,
    /// Rechargement borné (§8) : none, signal:<processus>:<SIGUSR1|SIGUSR2|SIGHUP>,
    /// socket:kitty, gsettings, hyprctl, daemon.
    pub reload: Option<String>,
    /// Cible désactivée par le pack (défaut : active).
    pub enabled: Option<bool>,
}

impl PackTarget {
    /// Rechargement analysé, hors liste blanche refusé dès la validation.
    pub fn reload_kind(&self) -> std::result::Result<Option<ReloadSpec>, String> {
        let Some(text) = &self.reload else {
            return Ok(None);
        };
        let t = text.trim();
        Ok(Some(match t {
            "none" | "" => ReloadSpec::None,
            "gsettings" => ReloadSpec::Gsettings,
            "hyprctl" => ReloadSpec::Hyprctl,
            "daemon" => ReloadSpec::Daemon,
            "socket:kitty" => ReloadSpec::KittySocket,
            _ if t.starts_with("signal:") => {
                let rest = t.strip_prefix("signal:").unwrap_or("");
                let (proc, sig) = rest
                    .split_once(':')
                    .ok_or("signal attendu : signal:<processus>:<SIG>")?;
                let sig = match sig.trim() {
                    "SIGUSR1" | "USR1" => "USR1",
                    "SIGUSR2" | "USR2" => "USR2",
                    "SIGHUP" | "HUP" => "HUP",
                    other => return Err(format!("signal non accepté : {other}")),
                };
                let name = proc.trim();
                if name.is_empty()
                    || !name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    return Err(format!("nom de processus non accepté : {name}"));
                }
                ReloadSpec::Signal(name.to_string(), sig.to_string())
            }
            other => {
                return Err(format!(
                    "rechargement hors liste blanche : {other} (pack = des données, pas du shell)"
                ));
            }
        }))
    }
}

/// Rechargement analysé d'une cible de pack.
#[derive(Debug, Clone, PartialEq)]
pub enum ReloadSpec {
    None,
    Signal(String, String),
    Gsettings,
    Hyprctl,
    KittySocket,
    Daemon,
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
        // Identifiant vide : appelant qui ne connaît pas encore l'id (lecture
        // d'archive) — la correspondance est alors ignorée, la validation
        // du format reste entière.
        if !id.is_empty() && meta.id != id {
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
        // Options du pack et de ses variantes : clé en liste blanche, valeur
        // sur une ligne (un saut de ligne glisserait une directive hyprlang,
        // `exec` compris, dans le fragment ou dans `hyprctl keyword`).
        for (key, value) in self.hyprland.options.iter().chain(
            self.variant
                .values()
                .flat_map(|v| v.hyprland_options.iter()),
        ) {
            if !allowed_hyprland_key(key) {
                return Err(Error::Manifest(
                    id.to_string(),
                    format!("option Hyprland hors liste blanche : {key}"),
                ));
            }
            if !safe_hyprland_value(value) {
                return Err(Error::Manifest(
                    id.to_string(),
                    format!(
                        "valeur refusée pour l'option Hyprland {key} : une ligne, sans caractère de contrôle, {MAX_HYPR_VALUE} caractères au plus"
                    ),
                ));
            }
        }
        for (name, t) in &self.targets {
            if !valid_id(name)
                && !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(Error::Manifest(
                    id.to_string(),
                    format!("nom de cible non accepté : {name}"),
                ));
            }
            if let Some(out) = &t.output
                && (out.contains("..")
                    || (out.starts_with("~/")
                        && !out.starts_with("~/.config/")
                        && !out.starts_with("~/.local/share/")))
            {
                return Err(Error::Manifest(
                    id.to_string(),
                    format!("sortie hors liste blanche pour la cible {name} : {out}"),
                ));
            }
            if let Err(e) = t.reload_kind() {
                return Err(Error::Manifest(id.to_string(), e));
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

    /// Variante par défaut effective du pack : celle qu'il déclare, sinon
    /// déduite de son fond (`palette.bg` clair : `light`), sinon `dark`
    /// (MLG-254 : un pack clair sans `default_variant` passait en sombre et
    /// posait `prefer-dark`).
    pub fn default_variant(&self) -> &'static str {
        match self.meta.default_variant.as_deref() {
            Some("light") => "light",
            Some(_) => DEFAULT_VARIANT,
            None => match self.palette.get("bg").and_then(|c| super::color::parse(c)) {
                Some(bg) if super::color::luminance(bg) > 0.5 => "light",
                _ => DEFAULT_VARIANT,
            },
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

/// Longueur maximale d'une valeur d'option Hyprland déclarée par un pack.
pub const MAX_HYPR_VALUE: usize = 256;

/// Valeur d'option Hyprland acceptable : une seule ligne, sans caractère de
/// contrôle, bornée.
pub fn safe_hyprland_value(value: &str) -> bool {
    value.chars().count() <= MAX_HYPR_VALUE && !value.chars().any(char::is_control)
}

/// Chemin d'option Hyprland bien formé, rien d'autre : pas d'espace, de `=`
/// ni de saut de ligne qui ferait d'une clé admise une seconde directive.
pub fn valid_option_path(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '.' | '-'))
}

/// Liste blanche des options Hyprland que peut régler un pack (§8) :
/// décoration, espacement et bordures, animations, comportement du flou.
pub fn allowed_hyprland_key(key: &str) -> bool {
    if !valid_option_path(key) {
        return false;
    }
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
    fn variante_par_défaut_déduite_du_fond() {
        let clair = "[meta]\nid = \"xx\"\nname = \"X\"\nversion = \"1\"\n\
                     [palette]\nbg = \"#f3f4f8\"\nfg = \"#1f2330\"\naccent = \"#5b5bd6\"\n";
        assert_eq!(
            Manifest::parse("xx", clair).unwrap().0.default_variant(),
            "light"
        );
        let sombre = clair.replace("#f3f4f8", "#171b27");
        assert_eq!(
            Manifest::parse("xx", &sombre).unwrap().0.default_variant(),
            "dark"
        );
        // Déclarée : elle gagne sur le fond.
        let déclarée = clair.replace(
            "version = \"1\"\n",
            "version = \"1\"\ndefault_variant = \"dark\"\n",
        );
        assert_eq!(
            Manifest::parse("xx", &déclarée)
                .unwrap()
                .0
                .default_variant(),
            "dark"
        );
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

    #[test]
    fn rechargement_borné_des_cibles_de_pack() {
        // Accepté : signal borné, formes de la liste blanche.
        let m = format!(
            "{ARDOISE}\n[targets.waybar]\ntemplate = \"templates/waybar.css.tmpl\"\noutput = \"~/.config/waybar/hyprdesk.css\"\nreload = \"signal:waybar:USR1\"\n"
        );
        let (m, _) = Manifest::parse("ardoise", &m).unwrap();
        let waybar = &m.targets["waybar"];
        assert_eq!(
            waybar.output.as_deref(),
            Some("~/.config/waybar/hyprdesk.css")
        );
        assert_eq!(
            waybar.reload_kind().unwrap(),
            Some(super::ReloadSpec::Signal(
                "waybar".to_string(),
                "USR1".to_string()
            ))
        );
        // Refusé : commande arbitraire (un pack reste des données).
        for bad in [
            "pkill waybar",
            "signal:waybar:SIGKILL",
            "signal:../evil:USR1",
            "exec:rm -rf",
        ] {
            let m = format!(
                "{ARDOISE}\n[targets.waybar]\noutput = \"~/.config/w.css\"\nreload = \"{bad}\"\n"
            );
            assert!(
                Manifest::parse("ardoise", &m).is_err(),
                "rechargement accepté : {bad}"
            );
        }
        // Refusé : sortie qui sort du périmètre.
        let m = format!(
            "{ARDOISE}\n[targets.waybar]\ntemplate = \"t.tmpl\"\noutput = \"~/.ssh/evil\"\n"
        );
        assert!(
            Manifest::parse("ardoise", &m).is_err(),
            "sortie acceptée hors config"
        );
    }

    #[test]
    fn valeur_hyprland_sur_plusieurs_lignes_rejetée() {
        // MLG-224 : un saut de ligne glisserait `exec` dans le fragment.
        let m = ARDOISE.replace(
            "\"decoration:rounding\" = \"10\"",
            "\"decoration:rounding\" = \"8\\nexec = touch /tmp/x\"",
        );
        assert_ne!(m, ARDOISE);
        let err = Manifest::parse("ardoise", &m).unwrap_err();
        assert!(err.to_string().contains("valeur refusée"), "{err}");
        // La valeur ordinaire du fixture passe.
        assert!(Manifest::parse("ardoise", ARDOISE).is_ok());
    }

    #[test]
    fn options_hyprland_des_variantes_en_liste_blanche() {
        // MLG-224 : les variantes passaient sans contrôle des clés.
        let m =
            format!("{ARDOISE}\n[variant.dark.hyprland_options]\n\"exec\" = \"touch /tmp/x\"\n");
        assert!(Manifest::parse("ardoise", &m).is_err());
        let m = format!(
            "{ARDOISE}\n[variant.dark.hyprland_options]\n\"decoration:rounding\" = \"4\\nexec = x\"\n"
        );
        assert!(Manifest::parse("ardoise", &m).is_err());
        assert!(!allowed_hyprland_key("misc:x\nexec"));
        assert!(!allowed_hyprland_key("decoration:rounding = 1"));
        assert!(allowed_hyprland_key("decoration:blur:size"));
    }
}
