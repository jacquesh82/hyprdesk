//! État du moteur (§13) : thème actif, palette publiée (`palette.json`),
//! dernier rapport, journal. Tout est écrit par renommage atomique ; la
//! révision ne bouge que si le contenu de la palette change.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Result;
use super::paths::Roots;
use super::resolve::Resolved;

/// Thème actif, tel que posé par `use` ou par les réglages du démon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveTheme {
    pub pack: String,
    pub variant: String,
    /// Origine de la pose : `user`, `lyra`, `daemon`, `cli`.
    pub origin: String,
}

/// Palette publiée pour les applications tierces (§9.1) : la source de vérité
/// lue par `hyprdesk-theme-client` et le portail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaletteDoc {
    pub schema: u32,
    pub pack: String,
    pub variant: String,
    pub revision: u64,
    /// Tous les slots (dérivés compris) en hexadécimal.
    pub colors: BTreeMap<String, String>,
    /// Effet de chaque surface (opacité, flou).
    pub effects: BTreeMap<String, EffectDoc>,
    /// Typographie déclarée, si le pack en porte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fonts: Option<super::manifest::Fonts>,
    pub updated: String,
}

/// Effet publié d'une surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectDoc {
    pub opacity: f64,
    pub blur: bool,
}

/// Version du schéma de `palette.json`.
pub const SCHEMA: u32 = 1;

/// Thème actif lu, s'il est posé.
pub fn read_active(roots: &Roots) -> Option<ActiveTheme> {
    let text = std::fs::read_to_string(roots.state_dir().join("active.json")).ok()?;
    serde_json::from_str(&text).ok()
}

/// Pose le thème actif (écriture atomique).
pub fn write_active(roots: &Roots, active: &ActiveTheme) -> Result<()> {
    let dir = ensure_state_dir(roots)?;
    let json = serde_json::to_string_pretty(active)?;
    atomic_write(&dir.join("active.json"), &json)
}

/// Publie la palette résolue ; renvoie la révision nouvelle. La révision
/// n'augmente que si les couleurs, effets ou polices changent — les clients
/// comparent la révision avant de redessiner.
pub fn publish_palette(roots: &Roots, resolved: &Resolved) -> Result<u64> {
    let dir = ensure_state_dir(roots)?;
    let path = dir.join("palette.json");
    let colors: BTreeMap<String, String> = resolved
        .palette
        .iter()
        .map(|(k, c)| (k.clone(), super::color::fmt(*c, super::color::Fmt::Hex)))
        .collect();
    let effects: BTreeMap<String, EffectDoc> = resolved
        .effects
        .iter()
        .map(|(p, e)| {
            (
                p.as_str().to_string(),
                EffectDoc {
                    opacity: (f64::from(e.opacity) * 1_000_000.0).round() / 1_000_000.0,
                    blur: e.blur,
                },
            )
        })
        .collect();
    let old: Option<PaletteDoc> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok());
    let changed = match &old {
        Some(o) => o.colors != colors || o.effects != effects || o.fonts != resolved.fonts,
        None => true,
    };
    let revision = match old {
        Some(o) if !changed => o.revision,
        Some(o) => o.revision + 1,
        None => 1,
    };
    let doc = PaletteDoc {
        schema: SCHEMA,
        pack: resolved.id.clone(),
        variant: resolved.variant.clone(),
        revision,
        colors,
        effects,
        fonts: resolved.fonts.clone(),
        updated: chrono::Utc::now().to_rfc3339(),
    };
    atomic_write(&path, &serde_json::to_string_pretty(&doc)?)?;
    Ok(revision)
}

/// Palette publiée lue (pour les tests et le CLI `palette`).
pub fn read_palette(roots: &Roots) -> Option<PaletteDoc> {
    let text = std::fs::read_to_string(roots.state_dir().join("palette.json")).ok()?;
    serde_json::from_str(&text).ok()
}

/// Écrit le dernier rapport d'application.
pub fn write_last_applied(roots: &Roots, json: &str) -> Result<()> {
    let dir = ensure_state_dir(roots)?;
    atomic_write(&dir.join("last-applied.json"), json)
}

/// Dernier rapport lu (CLI `verify`, retour arrière de l'Historique).
pub fn read_last_applied(roots: &Roots) -> Option<String> {
    std::fs::read_to_string(roots.state_dir().join("last-applied.json")).ok()
}

/// Ajoute une ligne horodatée au journal.
pub fn journal(roots: &Roots, line: &str) -> Result<()> {
    let dir = ensure_state_dir(roots)?;
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("journal.log"))?;
    writeln!(f, "{} {line}", chrono::Utc::now().to_rfc3339())?;
    Ok(())
}

/// Dossier d'état créé au besoin (0700).
pub fn ensure_state_dir(roots: &Roots) -> Result<PathBuf> {
    let dir = roots.state_dir();
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(dir)
}

/// Écriture atomique : fichier temporaire voisin puis renommage.
pub fn atomic_write(path: &std::path::Path, content: &str) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| super::Error::Io(std::io::Error::other("chemin sans dossier")))?;
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "fichier".to_string())
    ));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Rgba, Theme};

    fn resolved() -> Resolved {
        super::super::resolve::from_adjusted(
            Rgba(0xf2, 0xf4, 0xfc, 0xff),
            Rgba(0x17, 0x1b, 0x27, 0xff),
            Rgba(0xa6, 0x9c, 0xff, 0xff),
            Rgba(0x13, 0x17, 0x24, 0xff),
        )
    }

    #[test]
    fn révision_stable_si_rien_ne_change() {
        let root = std::env::temp_dir().join(format!("hd-state-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let r = resolved();
        let rev1 = publish_palette(&roots, &r).unwrap();
        assert_eq!(rev1, 1);
        let rev2 = publish_palette(&roots, &r).unwrap();
        assert_eq!(rev2, 1);
        // Un slot qui change fait avancer la révision.
        let mut autre = r.clone();
        autre
            .palette
            .insert("accent".to_string(), Rgba(0xff, 0, 0, 255));
        let rev3 = publish_palette(&roots, &autre).unwrap();
        assert_eq!(rev3, 2);
        let doc = read_palette(&roots).unwrap();
        assert_eq!(doc.colors["accent"], "#ff0000");
        assert_eq!(doc.pack, "regle");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn actif_et_journal() {
        let root = std::env::temp_dir().join(format!("hd-state-act-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        assert!(read_active(&roots).is_none());
        write_active(
            &roots,
            &ActiveTheme {
                pack: "ardoise".to_string(),
                variant: "dark".to_string(),
                origin: "cli".to_string(),
            },
        )
        .unwrap();
        let a = read_active(&roots).unwrap();
        assert_eq!(a.pack, "ardoise");
        journal(&roots, "essai").unwrap();
        let j = std::fs::read_to_string(roots.state_dir().join("journal.log")).unwrap();
        assert!(j.contains("essai"), "{j}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn écriture_atomique_sans_fichier_partiel() {
        let root = std::env::temp_dir().join(format!("hd-state-atom-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        atomic_write(&roots.state_dir().join("imbr/deep/f.txt"), "ok").unwrap();
        assert_eq!(
            std::fs::read_to_string(roots.state_dir().join("imbr/deep/f.txt")).unwrap(),
            "ok"
        );
        // Aucun temporaire laissé derrière.
        let leftovers: Vec<_> = std::fs::read_dir(roots.state_dir().join("imbr/deep"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn thème_défaut_utilisé_dans_la_publication() {
        let root = std::env::temp_dir().join(format!("hd-state-def-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        publish_palette(
            &roots,
            &super::super::resolve::from_theme(&Theme::default()),
        )
        .unwrap();
        let doc = read_palette(&roots).unwrap();
        assert_eq!(doc.colors["fg"], "#f2f4fc");
        let _ = std::fs::remove_dir_all(&root);
    }
}
