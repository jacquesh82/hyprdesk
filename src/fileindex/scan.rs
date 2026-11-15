//! Parcours des volumes : un parcours parallèle par racine.
//!
//! Chaque racine configurée (home, volumes montés — clés USB, partitions
//! Windows NTFS…) est un volume : on ne traverse pas les frontières de
//! montage (`same_file_system`), les squashfs de snap et autres boucles
//! restent dehors. Le moteur est `ignore::WalkParallel` (celui de
//! ripgrep/fd) : multithreadé, respecte `.gitignore`/`.ignore`, et les
//! exclusions par nom (`.git`, `node_modules`, `target`…) sont posées en
//! filtre d'entrée pour ne pas descendre du tout.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ignore::{DirEntry, WalkBuilder, WalkState};
use regex::RegexSet;

use crate::fileindex::IndexConfig;
use crate::fileindex::index::{EntryMeta, KIND_DIR, KIND_FILE, KIND_LINK};

/// Filtres d'indexation appliqués pendant le parcours et à la veille :
/// noms exclus (composants exacts), expressions régulières sur le chemin
/// complet, sous-arbres exclus par chemin, extensions écartées et taille
/// maximale (fichiers seulement).
#[derive(Debug, Clone, Default)]
pub struct Filtres {
    pub excludes: std::collections::HashSet<String>,
    pub regexes: RegexSet,
    pub ext_excl: std::collections::HashSet<String>,
    /// Catégories exclues (codes, fichiers seulement).
    pub cat_excl: std::collections::HashSet<u8>,
    /// Octets (0 : illimité).
    pub max_size: u64,
    /// Sous-arbres exclus par chemin absolu : le dossier du snapshot, dont
    /// l'écriture ne doit ni s'indexer ni réveiller la veille.
    pub chemins_exclus: Vec<PathBuf>,
}

impl Filtres {
    /// Filtres d'un démon : réglages de l'utilisateur compilés une fois,
    /// plus le dossier du snapshot (`$XDG_CACHE_HOME` peut sortir de
    /// `~/.cache`, exclu par nom, et retomber dans une racine surveillée).
    pub fn depuis_config(config: &IndexConfig) -> Self {
        Self {
            excludes: config.excludes().into_iter().collect(),
            regexes: compile_regexes(&config.exclude_regex),
            ext_excl: config
                .exclude_ext
                .iter()
                .map(|x| x.trim().trim_start_matches('.').to_lowercase())
                .filter(|x| !x.is_empty())
                .collect(),
            max_size: config.max_size.saturating_mul(1024 * 1024),
            cat_excl: config
                .exclude_cat
                .iter()
                .filter_map(|n| crate::fileindex::index::categorie_code(n))
                .collect(),
            chemins_exclus: vec![crate::fileindex::store::snapshot_dir()],
        }
    }

    /// Même jugement que le parcours, pour un chemin arrivé par la veille.
    /// Le parcours ne descend pas dans un dossier écarté : chaque composant
    /// entre la racine et `p` passe donc au filtre de nom et de regex (un
    /// ancêtre écarté écarte tout son sous-arbre). Extension et catégorie
    /// ne jugent que le dernier composant, et seulement si c'est un fichier
    /// (`est_fichier` : `None` pour un chemin disparu, nature inconnue).
    /// Hors de `racine` : écarté.
    pub fn ecarte_chemin(&self, racine: &Path, p: &Path, est_fichier: Option<bool>) -> bool {
        if self.chemins_exclus.iter().any(|x| p.starts_with(x)) {
            return true;
        }
        let Ok(rel) = p.strip_prefix(racine) else {
            return true;
        };
        let mut courant = racine.to_path_buf();
        for c in rel.components() {
            let Component::Normal(nom) = c else {
                return true; // « .. » ou autre bizarrerie : pas pour nous
            };
            courant.push(nom);
            if exclu(nom, &self.excludes) || self.regex_ecarte(&courant) {
                return true;
            }
        }
        if est_fichier == Some(true)
            && let Some(nom) = p.file_name().and_then(|n| n.to_str())
        {
            return self.ext_ou_cat_ecartee(nom);
        }
        false
    }

    /// Fichier au-delà de la taille maximale (non indexé).
    pub fn trop_gros(&self, kind: u8, taille: u64) -> bool {
        self.max_size > 0 && kind == KIND_FILE && taille > self.max_size
    }

    fn regex_ecarte(&self, p: &Path) -> bool {
        !self.regexes.is_empty() && self.regexes.is_match(p.to_string_lossy().as_ref())
    }

    /// Extension ou catégorie exclue, jugée sur le nom d'un fichier.
    fn ext_ou_cat_ecartee(&self, nom: &str) -> bool {
        if !self.ext_excl.is_empty()
            && Path::new(nom)
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| self.ext_excl.contains(&x.to_lowercase()))
        {
            return true;
        }
        // catégorie déduite du nom au vol
        !self.cat_excl.is_empty() && self.cat_excl.contains(&super::index::categorie_de(nom))
    }
}

/// Parcours d'un sous-arbre avec les options et le filtre d'entrée du scan
/// (cachés inclus, `.gitignore` respecté, pas de traversée de montage ni de
/// lien). `dossiers_seuls` : ne livre que les dossiers (pose des watches de
/// la veille), sans stat des fichiers.
pub fn parcours(root: &Path, filtres: Arc<Filtres>, dossiers_seuls: bool) -> WalkBuilder {
    let mut builder = WalkBuilder::new(root);
    // fichiers cachés inclus (tout ce qui n'est pas exclu sert la
    // recherche) ; les caches massifs sont dans la liste d'exclusions
    builder
        .hidden(false)
        .git_ignore(true)
        .ignore(true)
        .parents(true)
        .same_file_system(true)
        .follow_links(false)
        .filter_entry(move |e: &DirEntry| {
            if e.depth() == 0 {
                return true;
            }
            if dossiers_seuls && !e.file_type().is_some_and(|t| t.is_dir()) {
                return false;
            }
            !ecarte(e, &filtres)
        });
    builder
}

/// Compile les motifs utilisateur ; une regex invalide est signalée puis
/// écartée (elle ne bloque jamais tout l'index).
pub fn compile_regexes(motifs: &[String]) -> RegexSet {
    let mut valides = Vec::new();
    for m in motifs {
        if m.trim().is_empty() {
            continue;
        }
        match regex::Regex::new(m) {
            Ok(_) => valides.push(m.clone()),
            Err(e) => tracing::warn!(
                "{}",
                crate::tr!(
                    "exclusion ignorée, expression régulière invalide : {m} ({e})",
                    m = m,
                    e = e
                )
            ),
        }
    }
    RegexSet::new(valides).unwrap_or_else(|_| RegexSet::empty())
}

/// Bilan d'un parcours de volume.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ScanStats {
    /// Entrées indexées (fichiers, dossiers, liens).
    pub scanned: usize,
    /// Entrées illisibles (permissions, E/S) : comptées, jamais fatales.
    pub errors: usize,
}

/// Parcourt un volume et renvoie ses entrées `(chemin, méta)`, toutes
/// estampillées du volume `vol`. `progress`, s'il existe, compte les
/// entrées vues (barre d'état du démon pendant le scan initial).
pub fn scan_volume(
    root: &Path,
    filtres: &Filtres,
    vol: u8,
    progress: Option<Arc<AtomicUsize>>,
) -> (Vec<(String, EntryMeta)>, ScanStats) {
    let out: Arc<Mutex<Vec<(String, EntryMeta)>>> = Arc::default();
    let scanned = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let filtres = Arc::new(filtres.clone());

    parcours(root, filtres.clone(), false)
        .build_parallel()
        .run(|| {
            // état par fil : compteurs partagés en Arc, collecte via le mutex
            // commun ; `run` attend une closure FnMut par fil de travail
            let out = out.clone();
            let scanned = scanned.clone();
            let errors = errors.clone();
            let prog = progress.clone();
            let filtres = filtres.clone();
            Box::new(move |res: Result<DirEntry, ignore::Error>| match res {
                Ok(e) => {
                    scanned.fetch_add(1, Ordering::Relaxed);
                    if let Some(p) = &prog {
                        p.fetch_add(1, Ordering::Relaxed);
                    }
                    if let Some(entree) = vers_entree(&e, vol, &filtres)
                        && let Ok(mut g) = out.lock()
                    {
                        g.push(entree);
                    }
                    WalkState::Continue
                }
                Err(_) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                    WalkState::Continue
                }
            }) as Box<dyn FnMut(Result<DirEntry, ignore::Error>) -> WalkState + Send>
        });

    let entries = Arc::try_unwrap(out)
        .expect("le parcours est fini, plus de lecteur")
        .into_inner()
        .expect("plus de lecteur");
    let stats = ScanStats {
        scanned: scanned.load(Ordering::Relaxed),
        errors: errors.load(Ordering::Relaxed),
    };
    (entries, stats)
}

/// DirEntry → entrée indexée (None : rien d'utile à retenir, ou fichier
/// trop gros pour la limite de taille).
fn vers_entree(e: &DirEntry, vol: u8, filtres: &Filtres) -> Option<(String, EntryMeta)> {
    let ft = e.file_type()?;
    let meta = e.metadata().ok()?;
    let kind = if ft.is_dir() {
        KIND_DIR
    } else if ft.is_symlink() {
        KIND_LINK
    } else {
        KIND_FILE
    };
    if filtres.trop_gros(kind, meta.len()) {
        return None;
    }
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Some((
        e.path().to_string_lossy().into_owned(),
        EntryMeta {
            kind,
            size: meta.len(),
            mtime,
            vol,
        },
    ))
}

/// L'entrée est-elle écartée ? Par nom de composant (exclusion exacte),
/// par regex sur le chemin complet, par sous-arbre exclu, ou — fichiers
/// seulement — par extension ou catégorie exclue.
fn ecarte(e: &DirEntry, filtres: &Filtres) -> bool {
    if exclu(e.file_name(), &filtres.excludes) || filtres.regex_ecarte(e.path()) {
        return true;
    }
    if filtres.chemins_exclus.iter().any(|x| e.path() == x) {
        return true; // le parcours n'y descend pas : l'égalité suffit
    }
    e.file_type().is_some_and(|t| t.is_file())
        && e.file_name()
            .to_str()
            .is_some_and(|n| filtres.ext_ou_cat_ecartee(n))
}

/// Le nom d'entrée est-il exclu ? (composant exact, insensible à la casse)
fn exclu(nom: &std::ffi::OsStr, excl: &HashSet<String>) -> bool {
    nom.to_str()
        .map(|n| excl.contains(&n.to_lowercase()))
        .unwrap_or(false)
}

/// Type de système de fichiers d'un point de montage, via
/// `/proc/self/mounts` (le plus précis : `fuseblk` pour ntfs-3g, `ntfs3`
/// pour le pilote noyau, `exfat`…) avec repli sur `statfs`.
pub fn fs_type(root: &Path) -> String {
    fstype_de_mounts(root)
        .or_else(|| fstype_de_statfs(root))
        .unwrap_or_else(|| "autre".to_string())
}

/// Cherche dans `/proc/self/mounts` le point de montage le plus long qui
/// contient `root`, et renvoie son type.
fn fstype_de_mounts(root: &Path) -> Option<String> {
    let contenu = std::fs::read_to_string("/proc/self/mounts").ok()?;
    let cible = root.to_string_lossy();
    let mut meilleur: Option<(usize, String)> = None;
    for ligne in contenu.lines() {
        let mut champs = ligne.split_whitespace();
        let (Some(_dev), Some(mp), Some(fs)) = (champs.next(), champs.next(), champs.next()) else {
            continue;
        };
        let mp = deechapper(mp);
        if (cible == mp || cible.starts_with(&format!("{mp}/")))
            && meilleur
                .as_ref()
                .map(|(l, _)| mp.len() > *l)
                .unwrap_or(true)
        {
            meilleur = Some((mp.len(), fs.to_string()));
        }
    }
    meilleur.map(|(_, fs)| fs)
}

/// `\040` (espace), `\011`, `\012`, `\134` — encodage de /proc/mounts.
fn deechapper(s: &str) -> String {
    s.replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

/// Repli : magie `statfs` pour les types courants.
fn fstype_de_statfs(root: &Path) -> Option<String> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = CString::new(root.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    Some(
        match st.f_type {
            0xef53 => "ext4", // EXT2/3/4 partagent la magie
            0x9123683e => "btrfs",
            0x58465342 => "xfs",
            0x5346544e => "ntfs", // pilote noyau ntfs3
            0x4d44 => "vfat",
            0x2011bab => "exfat",
            0x65735546 => "fuse", // ntfs-3g, sshfs… (précisé par /proc/mounts)
            0x6969 => "nfs",
            0xff534d42 => "cifs",
            0x958458f2 => "tmpfs",
            _ => "autre",
        }
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dechappement_des_monts() {
        assert_eq!(deechapper("/mnt/un\\040deux"), "/mnt/un deux");
        assert_eq!(deechapper("/a\\134b"), "/a\\b");
    }

    #[test]
    fn type_de_fs_non_vide() {
        // /tmp est monté d'une façon ou d'une autre sur toute distribution
        assert!(!fs_type(Path::new("/tmp")).is_empty());
    }

    #[test]
    fn parcours_d_un_petit_arbre() {
        let base = std::env::temp_dir().join(format!("hyprdesk-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let docs = base.join("docs");
        std::fs::create_dir_all(&docs).expect("mkdir");
        std::fs::write(docs.join("a.txt"), b"hello").expect("write");
        std::fs::write(base.join(".fichier_cache"), b"x").expect("write");
        std::fs::create_dir_all(base.join("node_modules")).expect("mkdir");
        std::fs::write(base.join("node_modules").join("gros.js"), b"x").expect("write");

        let filtres = Filtres {
            excludes: crate::fileindex::DEFAULT_EXCLUDES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            ..Default::default()
        };
        let (entries, stats) = scan_volume(&base, &filtres, 0, None);

        let chemins: HashSet<String> = entries.into_iter().map(|(p, _)| p).collect();
        assert!(chemins.contains(&base.to_string_lossy().to_string()));
        assert!(chemins.contains(&docs.to_string_lossy().to_string()));
        assert!(chemins.contains(&docs.join("a.txt").to_string_lossy().to_string()));
        // fichiers cachés inclus, exclusions évitées
        assert!(chemins.contains(&base.join(".fichier_cache").to_string_lossy().to_string()));
        assert!(!chemins.iter().any(|p| p.contains("node_modules")));
        assert!(stats.scanned >= 4);
        assert_eq!(stats.errors, 0);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn filtres_regex_extensions_et_taille() {
        let base = std::env::temp_dir().join(format!("hyprdesk-filtres-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("mkdir");
        std::fs::write(base.join("garde.txt"), b"x").expect("write");
        std::fs::write(base.join("vieux.bak"), b"x").expect("write");
        std::fs::write(base.join("disque.iso"), b"x").expect("write");
        std::fs::write(base.join("gros.bin"), vec![0u8; 64]).expect("write");

        let filtres = Filtres {
            regexes: compile_regexes(&["\\.bak$".to_string()]),
            ext_excl: ["iso".to_string()].into_iter().collect(),
            max_size: 16,
            ..Default::default()
        };
        let (entries, _) = scan_volume(&base, &filtres, 0, None);
        let chemins: HashSet<String> = entries.into_iter().map(|(p, _)| p).collect();

        assert!(chemins.contains(&base.join("garde.txt").to_string_lossy().to_string()));
        // regex sur le chemin complet
        assert!(!chemins.iter().any(|p| p.ends_with("vieux.bak")));
        // extension exclue (fichiers seulement)
        assert!(!chemins.iter().any(|p| p.ends_with("disque.iso")));
        // trop gros : non indexé
        assert!(!chemins.iter().any(|p| p.ends_with("gros.bin")));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn filtre_de_chemin_comme_le_parcours() {
        let filtres = Filtres {
            excludes: crate::fileindex::DEFAULT_EXCLUDES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            regexes: compile_regexes(&["/build$".to_string()]),
            ext_excl: ["iso".to_string()].into_iter().collect(),
            chemins_exclus: vec![PathBuf::from("/home/u/.local/cache/hyprdesk/index")],
            ..Default::default()
        };
        let r = Path::new("/home/u");
        let ecarte = |p: &str, f: Option<bool>| filtres.ecarte_chemin(r, Path::new(p), f);
        // ancêtre exclu par nom : tout le sous-arbre
        assert!(ecarte("/home/u/projet/.git/objects/ab/cd", Some(true)));
        assert!(ecarte("/home/u/projet/target/debug/deps", None));
        assert!(ecarte("/home/u/projet/target", None));
        // ancêtre écarté par regex (le fichier seul ne correspond pas)
        assert!(ecarte("/home/u/projet/build/sortie.o", Some(true)));
        // extension : fichiers seulement
        assert!(ecarte("/home/u/disque.iso", Some(true)));
        assert!(!ecarte("/home/u/dossier.iso", Some(false)));
        // sous-arbre exclu par chemin (snapshot)
        assert!(ecarte(
            "/home/u/.local/cache/hyprdesk/index/snapshot.tmp",
            Some(true)
        ));
        assert!(!ecarte("/home/u/.local/cache/autre", Some(true)));
        // hors racine
        assert!(ecarte("/ailleurs/a.txt", Some(true)));
        // admis
        assert!(!ecarte("/home/u/projet/src/main.rs", Some(true)));
        assert!(!ecarte("/home/u", None));
    }

    #[test]
    fn filtres_du_demon_excluent_le_snapshot() {
        let f = Filtres::depuis_config(&IndexConfig {
            roots: vec![],
            ..Default::default()
        });
        let dir = crate::fileindex::store::snapshot_dir();
        assert!(f.chemins_exclus.contains(&dir));
        assert!(f.ecarte_chemin(Path::new("/"), &dir.join("snapshot.tmp"), Some(true)));
        assert!(f.excludes.contains(".git"));
    }

    #[test]
    fn parcours_exclut_le_dossier_du_snapshot() {
        let base = std::env::temp_dir().join(format!("hyprdesk-snapexcl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let snap = base.join("cache").join("hyprdesk").join("index");
        std::fs::create_dir_all(&snap).expect("mkdir");
        std::fs::write(snap.join("snapshot.bin"), b"x").expect("write");
        std::fs::write(base.join("garde.txt"), b"x").expect("write");
        let filtres = Filtres {
            chemins_exclus: vec![snap.clone()],
            ..Default::default()
        };
        let (entries, _) = scan_volume(&base, &filtres, 0, None);
        let chemins: HashSet<String> = entries.into_iter().map(|(p, _)| p).collect();
        assert!(chemins.contains(&base.join("garde.txt").to_string_lossy().to_string()));
        assert!(!chemins.iter().any(|p| p.contains("snapshot")));
        assert!(!chemins.contains(&snap.to_string_lossy().to_string()));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn regex_invalide_ecartee_sans_bloquer() {
        let set = compile_regexes(&["([(".to_string(), "ok".to_string()]);
        assert!(set.is_match("c'est ok"));
        assert_eq!(set.len(), 1);
        assert_eq!(compile_regexes(&["  ".to_string()]).len(), 0);
    }
}
