//! Palette par intention (§17.3 de la spécification) : LYRA traduit une
//! demande en intent structuré à vocabulaire fermé, le moteur calcule la
//! palette en OKLCH avec contraste garanti. Résultat déterministe : deux
//! fois la même intention donnent la même palette.

use serde_json::Value;

use crate::config::Rgba;

use super::color::{self, Oklch};
use super::resolve;
use super::{Error, Result};

/// Teintes nommées de la liste close : nom → degrés de teinte OKLCH.
/// `graphite` est presque neutre (chroma réduit).
pub const HUES: &[(&str, f64)] = &[
    ("iris", 285.0),
    ("sauge", 130.0),
    ("ambre", 65.0),
    ("rubis", 15.0),
    ("océan", 235.0),
    ("mousse", 105.0),
    ("graphite", 255.0),
];

/// Ambiance : bornes de chroma et de luminosité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Calme,
    Vif,
    Contraste,
}

/// Plancher WCAG des paires essentielles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contrast {
    Aa,
    Aaa,
}

impl Contrast {
    pub fn min(self) -> f64 {
        match self {
            Self::Aa => 4.5,
            Self::Aaa => 7.0,
        }
    }
}

/// Intention structurée, telle que LYRA la produit.
#[derive(Debug, Clone, PartialEq)]
pub struct Intent {
    pub name: String,
    pub variant: String,
    /// Teinte : nom de la liste close, ou degrés libres.
    pub hue_degrees: f64,
    /// `graphite` : chroma réduit partout.
    pub graphite: bool,
    pub mood: Mood,
    pub contrast: Contrast,
    /// Accent imposé (hex) ; absent : dérivée de la teinte.
    pub accent: Option<Rgba>,
    /// Parent d'héritage du pack construit (défaut : hyprdesk-base).
    pub inherit: Option<String>,
}

impl Intent {
    /// Analyse un intent JSON (`build --intent fichier.json`) : chaque champ
    /// hors vocabulaire est une erreur explicite — LYRA est corrigée, pas
    /// contournée.
    pub fn parse(value: &Value) -> Result<Self> {
        let obj = value.as_object().ok_or_else(|| {
            Error::Manifest("intent".to_string(), "objet JSON attendu".to_string())
        })?;
        let get = |k: &str| obj.get(k);
        let name = get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Manifest("intent".to_string(), "name requis".to_string()))?
            .trim()
            .to_string();
        if name.is_empty() {
            return Err(Error::Manifest(
                "intent".to_string(),
                "name vide".to_string(),
            ));
        }
        let variant = get("variant")
            .and_then(Value::as_str)
            .unwrap_or("dark")
            .trim()
            .to_string();
        if variant != "dark" && variant != "light" {
            return Err(Error::Manifest(
                "intent".to_string(),
                format!("variante inconnue : {variant}"),
            ));
        }
        let (hue_degrees, graphite) = match get("hue").and_then(Value::as_str) {
            Some(h) => {
                let h = h.trim().to_lowercase();
                if let Some(deg) = HUES.iter().find(|(n, _)| *n == h) {
                    (deg.1, h == "graphite")
                } else if let Ok(deg) = h.trim_end_matches("deg").parse::<f64>() {
                    (deg.rem_euclid(360.0), false)
                } else {
                    return Err(Error::Manifest(
                        "intent".to_string(),
                        format!(
                            "teinte inconnue : {h} (liste close : {})",
                            HUES.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
                        ),
                    ));
                }
            }
            None => (HUES[0].1, false),
        };
        let mood = match get("mood").and_then(Value::as_str).unwrap_or("calme") {
            "calme" | "calm" => Mood::Calme,
            "vif" | "vivid" => Mood::Vif,
            "contraste" | "contrast" => Mood::Contraste,
            other => {
                return Err(Error::Manifest(
                    "intent".to_string(),
                    format!("ambiance inconnue : {other} (calme, vif ou contraste)"),
                ));
            }
        };
        let contrast = match get("contrast").and_then(Value::as_str).unwrap_or("aa") {
            "aa" => Contrast::Aa,
            "aaa" => Contrast::Aaa,
            other => {
                return Err(Error::Manifest(
                    "intent".to_string(),
                    format!("plancher inconnu : {other} (aa ou aaa)"),
                ));
            }
        };
        let accent = match get("accent") {
            None => None,
            Some(Value::String(s)) if s == "auto" || s.is_empty() => None,
            Some(Value::String(s)) => Some(color::parse(s).ok_or_else(|| {
                Error::Manifest("intent".to_string(), format!("accent invalide : {s}"))
            })?),
            Some(_) => {
                return Err(Error::Manifest(
                    "intent".to_string(),
                    "accent : \"auto\" ou une couleur".to_string(),
                ));
            }
        };
        let inherit = get("inherit")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some("hyprdesk-base".to_string()));
        Ok(Intent {
            name,
            variant,
            hue_degrees,
            graphite,
            mood,
            contrast,
            accent,
            inherit,
        })
    }

    /// Construit la palette résolue de l'intention : quatre slots posés
    /// d'après les bornes de l'ambiance, le reste dérivé, puis contraste
    /// garanti sur les paires essentielles.
    pub fn build(&self) -> resolve::Resolved {
        let (bg_l, bg_c, fg_l, accent_l, accent_c) = match (self.mood, self.variant.as_str()) {
            (Mood::Calme, "dark") => (0.16, 0.030, 0.93, 0.72, 0.10),
            (Mood::Vif, "dark") => (0.19, 0.045, 0.93, 0.72, 0.17),
            (Mood::Contraste, "dark") => (0.12, 0.035, 0.96, 0.74, 0.14),
            (Mood::Calme, "light") => (0.92, 0.022, 0.25, 0.55, 0.10),
            (Mood::Vif, "light") => (0.90, 0.030, 0.24, 0.55, 0.17),
            (Mood::Contraste, "light") => (0.96, 0.018, 0.22, 0.52, 0.14),
            _ => (0.16, 0.03, 0.93, 0.72, 0.10),
        };
        let bg_c = if self.graphite { bg_c * 0.35 } else { bg_c };
        let fg_c = if self.graphite { 0.005 } else { 0.015 };
        let bg = color::from_oklch(Oklch {
            l: bg_l,
            c: bg_c,
            h: self.hue_degrees,
        });
        let fg = color::from_oklch(Oklch {
            l: fg_l,
            c: fg_c,
            h: self.hue_degrees,
        });
        let accent = self.accent.unwrap_or_else(|| {
            color::from_oklch(Oklch {
                l: accent_l,
                c: if self.graphite {
                    accent_c * 0.35
                } else {
                    accent_c
                },
                h: self.hue_degrees,
            })
        });
        let background = color::darken(bg, 4.0);
        let mut r = resolve::from_adjusted(fg, bg, accent, background);
        r.id = pack_id(&self.name);
        r.name = format!("LYRA — {}", self.name);
        // Contraste garanti : le fg et son atténué, l'accent_fg, sur leurs
        // fonds — jamais en dessous du plancher choisi.
        let min = self.contrast.min();
        r.palette
            .insert("fg".to_string(), color::ensure_contrast(fg, bg, min));
        r.palette.insert(
            "background".to_string(),
            color::ensure_contrast(background, r.palette["fg"], min),
        );
        r.palette.insert(
            "fg_muted".to_string(),
            color::ensure_contrast(r.palette["fg_muted"], r.palette["surface"], min.min(3.0)),
        );
        r.palette.insert(
            "accent_fg".to_string(),
            color::ensure_contrast(r.palette["accent_fg"], accent, min),
        );
        r
    }

    /// Manifeste `theme.toml` du pack construit : les couleurs posées du
    /// côté de la variante demandée, description d'origine.
    pub fn manifest(&self) -> String {
        let r = self.build();
        let hex = |slot: &str| color::fmt(r.palette[slot], color::Fmt::Hex);
        let variant_section = match self.variant.as_str() {
            "dark" => String::new(),
            other => format!(
                "\n[variant.{other}]\npalette.bg = \"{}\"\npalette.fg = \"{}\"\npalette.accent = \"{}\"\n",
                hex("bg"),
                hex("fg"),
                hex("accent")
            ),
        };
        let inherit = self
            .inherit
            .clone()
            .unwrap_or_else(|| "hyprdesk-base".to_string());
        format!(
            "[meta]\nid = \"{}\"\nname = \"LYRA — {}\"\nversion = \"0.1.0\"\ndescription = \"Palette construite par intention ({} — {} — contraste {}), appliquée après aperçu et confirmation.\"\nlicense = \"MIT\"\n\n[inherits]\nthemes = [\"{inherit}\"]\n\n[palette]\nbg = \"{}\"\nfg = \"{}\"\naccent = \"{}\"\n{variant_section}",
            pack_id(&self.name),
            self.name,
            self.variant,
            hue_name(self.hue_degrees),
            match self.contrast {
                Contrast::Aa => "aa",
                Contrast::Aaa => "aaa",
            },
            hex("bg"),
            hex("fg"),
            hex("accent")
        )
    }
}

/// Identifiant de pack depuis un nom libre : `lyra-` + normalisé.
pub fn pack_id(name: &str) -> String {
    let norm: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let norm = norm.trim_matches('-').to_string();
    let norm = norm
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if norm.is_empty() {
        "lyra-theme".to_string()
    } else {
        format!("lyra-{norm}")
    }
}

/// Nom de teinte le plus proche d'un degré (pour la description du pack).
fn hue_name(degrees: f64) -> String {
    HUES.iter()
        .min_by(|a, b| {
            let da = (a.1 - degrees).abs();
            let db = (b.1 - degrees).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(n, _)| (*n).to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(json: &str) -> Intent {
        Intent::parse(&serde_json::from_str(json).unwrap()).unwrap()
    }

    #[test]
    fn vocabulaire_fermé() {
        let i = intent(
            r#"{"name": "Sauge", "variant": "dark", "hue": "sauge", "mood": "calme", "contrast": "aa"}"#,
        );
        assert!((i.hue_degrees - 130.0).abs() < 1.0);
        assert_eq!(i.mood, Mood::Calme);
        assert_eq!(i.build().id, "lyra-sauge");
        for bad in [
            r#"{"name": "x", "hue": "pourpre"}"#,
            r#"{"name": "x", "mood": "dingue"}"#,
            r#"{"name": "x", "contrast": "zz"}"#,
            r#"{"name": "x", "variant": "nuit"}"#,
            r#"{"name": "x", "accent": "rouge"}"#,
            r#"{"variant": "dark"}"#,
            r#"{"name": "  "}"#,
        ] {
            assert!(
                Intent::parse(&serde_json::from_str(bad).unwrap()).is_err(),
                "accepté : {bad}"
            );
        }
    }

    #[test]
    fn contraste_garanti_sur_les_paires() {
        for (variant, hue) in [("dark", "sauge"), ("dark", "graphite"), ("light", "ambre")] {
            let i = intent(&format!(
                r#"{{"name": "t", "variant": "{variant}", "hue": "{hue}", "mood": "vif", "contrast": "aa"}}"#
            ));
            let r = i.build();
            let ratio = |a: &str, b: &str| color::contrast(r.palette[a], r.palette[b]);
            assert!(
                ratio("fg", "bg") >= 4.5,
                "{variant}/{hue} : {}",
                ratio("fg", "bg")
            );
            assert!(ratio("accent_fg", "accent") >= 4.5);
            assert!(ratio("fg_muted", "surface") >= 3.0);
            assert!(ratio("fg", "background") >= 4.5);
        }
    }

    #[test]
    fn déterministe() {
        let i = intent(r#"{"name": "Océan", "variant": "dark", "hue": "océan", "mood": "calme"}"#);
        let a = i.build();
        let b = i.build();
        assert_eq!(a.palette, b.palette);
    }

    #[test]
    fn manifeste_construit_parse_et_installe() {
        let i = intent(
            r#"{"name": "Mousse claire", "variant": "light", "hue": "mousse", "mood": "calme", "contrast": "aaa"}"#,
        );
        let text = i.manifest();
        let (m, _) = super::super::manifest::Manifest::parse("", &text).unwrap();
        assert_eq!(m.meta.id, "lyra-mousse-claire");
        assert!(text.contains("[variant.light]"));
        // Accent imposé : respecté.
        let i2 = intent(
            r##"{"name": "Rubis", "variant": "dark", "hue": "rubis", "accent": "#ff0000"}"##,
        );
        assert_eq!(i2.build().palette["accent"], Rgba(255, 0, 0, 255));
    }
}
