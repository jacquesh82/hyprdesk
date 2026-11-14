//! Index de fichiers pour le lanceur — le « $MFT » de hyprdesk.
//!
//! Aucun FS Linux n'a de table centrale des fichiers : ce module la
//! reconstitue à la manière de plocate — un index inversé de trigrammes sur
//! les noms, construit par un parcours parallèle des volumes (ext4, btrfs,
//! xfs, NTFS et FAT montés…), tenu à jour par inotify (`notify`) et rescan
//! périodique, persisté en snapshot compressé.
//!
//! Le démon (`hyprdesk index daemon`, même binaire) sert les requêtes sur un
//! socket unix ; le lanceur interroge ce socket depuis son mode `:f`
//! (prototype : le mode `:clip`). Les réglages (`index.paths`,
//! `index.exclude`, `index.interval`) suivent le motif des réglages de
//! hyprdesk et sont passés au démon par le lanceur au démarrage.

// Le cœur (index + store) est testable d'abord ; les modules suivants
// arrivent avec le démon.
pub mod daemon;
pub mod index;
pub mod proto;
pub mod scan;
pub mod store;
pub mod watch;

use std::path::PathBuf;

/// Réglages de l'index, résolus par hyprdesk (`hd.config`) et passés au
/// démon en ligne de commande ; les défauts s'appliquent si absents.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexConfig {
    /// Racines à indexer : une racine = un volume (on ne traverse pas les
    /// montages).
    pub roots: Vec<PathBuf>,
    /// Noms de dossiers à exclure, en plus des exclusions par défaut.
    pub extra_excludes: Vec<String>,
    /// Expressions régulières exclues, appliquées au chemin complet.
    pub exclude_regex: Vec<String>,
    /// Extensions exclues (sans le point, ex. « iso », « vdi »).
    pub exclude_ext: Vec<String>,
    /// Catégories exclues (noms : documents, images, vidéos…).
    pub exclude_cat: Vec<String>,
    /// Taille maximale indexée, en Mio (0 : illimité).
    pub max_size: u64,
    /// Secondes entre deux rescans de sûreté (inotify peut rater des
    /// événements sur FUSE/NTFS ou saturer `max_user_watches`).
    pub interval: u64,
}

/// Rescan de sûreté toutes les six heures par défaut.
pub const DEFAULT_INTERVAL_SECS: u64 = 21_600;

/// Exclusions par défaut : artéfacts de build, caches et déchets.
pub const DEFAULT_EXCLUDES: &[&str] = &[
    ".git",
    ".cache",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "lost+found",
    ".Trash",
];

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            roots: default_roots(),
            extra_excludes: Vec::new(),
            exclude_regex: Vec::new(),
            exclude_ext: Vec::new(),
            exclude_cat: Vec::new(),
            max_size: 0,
            interval: DEFAULT_INTERVAL_SECS,
        }
    }
}

impl IndexConfig {
    /// Liste d'exclusion complète : défauts + réglage utilisateur.
    pub fn excludes(&self) -> Vec<String> {
        let mut v: Vec<String> = DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect();
        // « .Trash-1000 » créé par gvfs sur les clés USB : préfixe couvert.
        v.push(".Trash-".to_string());
        v.extend(
            self.extra_excludes
                .iter()
                .filter(|s| !s.is_empty())
                .cloned(),
        );
        v
    }
}

/// Racines par défaut : le home et les volumes amovibles montés
/// (clés USB, disques Windows NTFS… sous /media et /run/media/$USER).
pub fn default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = home_dir() {
        roots.push(home);
    }
    for base in media_bases() {
        if let Ok(entries) = std::fs::read_dir(&base) {
            for e in entries.flatten() {
                if e.file_type().is_ok_and(|t| t.is_dir()) {
                    roots.push(e.path());
                }
            }
        }
    }
    roots
}

/// « ~/Documents » → chemin absolu (le « ~ » ne s'expand pas tout seul).
pub fn expand_root(s: &str) -> PathBuf {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix('~')
        && let Some(home) = home_dir()
    {
        let rel = rest.strip_prefix('/').unwrap_or(rest);
        if rel.is_empty() {
            return home;
        }
        return home.join(rel);
    }
    PathBuf::from(t)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// Taille lisible : « 980 o », « 1,2 Mo », « 3,4 Go ».
pub fn fmt_taille(octets: u64) -> String {
    const UNITES: [&str; 5] = ["o", "Ko", "Mo", "Go", "To"];
    let mut v = octets as f64;
    let mut u = 0usize;
    while v >= 1024.0 && u < UNITES.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{octets} o")
    } else {
        format!("{v:.1} {}", UNITES[u]).replace('.', ",")
    }
}

/// Dossiers où udisks/gvfs montent les volumes : /media et
/// /run/media/<utilisateur>.
fn media_bases() -> Vec<PathBuf> {
    let mut v = vec![PathBuf::from("/media")];
    if let Some(user) = std::env::var_os("USER").filter(|u| !u.is_empty()) {
        let mut p = PathBuf::from("/run/media");
        p.push(user);
        v.push(p);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusion_par_defaut_et_supplementaires() {
        let c = IndexConfig {
            roots: vec![],
            extra_excludes: vec!["dist".into(), ".git".into(), String::new()],
            exclude_regex: vec!["\\.bak$".into()],
            exclude_ext: vec!["iso".into()],
            exclude_cat: vec!["vidéos".into()],
            max_size: 512,
            interval: 1,
        };
        let ex = c.excludes();
        assert!(ex.contains(&".git".to_string()));
        assert!(ex.contains(&"node_modules".to_string()));
        assert!(ex.contains(&".Trash-".to_string()));
        assert!(ex.contains(&"dist".to_string()));
        // le vide n'apporte rien
        assert!(!ex.iter().any(|s| s.is_empty()));
    }

    #[test]
    fn expansion_du_tilde() {
        let home = std::env::var("HOME").unwrap_or_default();
        assert_eq!(
            expand_root("~/Documents"),
            PathBuf::from(&home).join("Documents")
        );
        assert_eq!(expand_root("~"), PathBuf::from(&home));
        assert_eq!(expand_root("/media/cle"), PathBuf::from("/media/cle"));
    }

    #[test]
    fn racines_par_defaut_contiennent_le_home() {
        let roots = default_roots();
        if let Some(home) = home_dir() {
            assert!(roots.contains(&home));
        }
    }

    #[test]
    fn tailles_lisibles() {
        assert_eq!(fmt_taille(980), "980 o");
        assert_eq!(fmt_taille(1024), "1,0 Ko");
        assert_eq!(fmt_taille(1_234_567), "1,2 Mo");
        assert_eq!(fmt_taille(3_500_000_000), "3,3 Go");
    }
}
