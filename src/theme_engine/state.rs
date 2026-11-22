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
    // Lire, comparer, réécrire : le démon et le binaire peuvent publier en
    // même temps, le verrou garde la révision monotone.
    let _verrou = Verrou::prendre(&dir.join("palette.lock"))?;
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
    // Rien de neuf (couleurs, effets, polices, pack, variante) : le fichier
    // n'est pas réécrit, ses lecteurs (démon, clientes) ne sont pas réveillés.
    if let Some(o) = &old
        && !changed
        && o.pack == resolved.id
        && o.variant == resolved.variant
        && o.schema == SCHEMA
    {
        return Ok(o.revision);
    }
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

/// Verrou exclusif (`flock`) tenu le temps d'une section : relâché à la
/// fermeture du fichier.
struct Verrou(#[allow(dead_code)] std::fs::File);

impl Verrou {
    fn prendre(path: &std::path::Path) -> Result<Self> {
        let f = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            // SAFETY : descripteur valide, tenu par `f` pendant l'appel.
            if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        Ok(Self(f))
    }
}

/// Chemin réellement écrit : un lien symbolique final (dotfiles gérés par
/// stow, par exemple) est suivi jusqu'à sa cible, pour ne pas être remplacé
/// par un fichier ordinaire au renommage.
pub fn real_target(path: &std::path::Path) -> PathBuf {
    let mut current = path.to_path_buf();
    // Chaîne de liens bornée : un cycle ne boucle pas.
    for _ in 0..16 {
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.is_symlink() => match std::fs::read_link(&current) {
                Ok(next) => {
                    current = if next.is_absolute() {
                        next
                    } else {
                        current.parent().map(|p| p.join(&next)).unwrap_or(next)
                    };
                }
                Err(_) => break,
            },
            _ => break,
        }
    }
    current
}

/// Écriture atomique : fichier temporaire voisin (nom propre au processus),
/// données forcées sur le disque, puis renommage. Un lien final est suivi.
pub fn atomic_write(path: &std::path::Path, content: &str) -> Result<()> {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COMPTEUR: AtomicU64 = AtomicU64::new(0);

    let path = real_target(path);
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .ok_or_else(|| super::Error::Io(std::io::Error::other("chemin sans dossier")))?;
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "fichier".to_string()),
        std::process::id(),
        COMPTEUR.fetch_add(1, Ordering::Relaxed)
    ));
    let écrit = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        // Droits du fichier remplacé conservés (une config en 0600 le reste).
        if let Ok(meta) = std::fs::metadata(&path) {
            let _ = f.set_permissions(meta.permissions());
        }
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, &path)
    })();
    if let Err(e) = écrit {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
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

    #[test]
    fn lien_final_suivi_pas_remplacé() {
        // MLG-226 : un fichier en lien (stow) reste un lien, sa cible change.
        let root = std::env::temp_dir().join(format!("hd-state-lien-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("dotfiles")).unwrap();
        std::fs::create_dir_all(root.join("config")).unwrap();
        std::fs::write(root.join("dotfiles/gtk.css"), "avant").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("dotfiles/gtk.css"), root.join("config/gtk.css"))
            .unwrap();
        atomic_write(&root.join("config/gtk.css"), "après").unwrap();
        assert!(
            std::fs::symlink_metadata(root.join("config/gtk.css"))
                .unwrap()
                .is_symlink()
        );
        assert_eq!(
            std::fs::read_to_string(root.join("dotfiles/gtk.css")).unwrap(),
            "après"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn révision_monotone_sous_publications_concurrentes() {
        // MLG-229 : verrou autour de lire-comparer-réécrire.
        let root = std::env::temp_dir().join(format!("hd-state-conc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let fils: Vec<_> = (0..8u8)
            .map(|i| {
                let roots = roots.clone();
                std::thread::spawn(move || {
                    let mut r = resolved();
                    r.palette.insert("accent".to_string(), Rgba(i, 0, 0, 255));
                    publish_palette(&roots, &r).unwrap()
                })
            })
            .collect();
        let mut revs: Vec<u64> = fils.into_iter().map(|f| f.join().unwrap()).collect();
        revs.sort_unstable();
        // Huit palettes distinctes : huit révisions distinctes, 1 à 8.
        assert_eq!(revs, (1..=8).collect::<Vec<_>>());
        let _ = std::fs::remove_dir_all(&root);
    }
}
