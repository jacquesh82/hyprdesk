//! Veille du système de fichiers : inotify (via `notify`), traduit en
//! actions d'index à appliquer en lots.
//!
//! La veille regarde exactement ce que le scan indexe : un watch inotify
//! non récursif par dossier admis, posé en parcourant chaque racine avec le
//! filtre du scan (`scan::parcours`). Rien n'est donc posé dans `.git`,
//! `target`, `node_modules`, `~/.cache`… (ni compté contre
//! `max_user_watches`, ni réveillé par un `cargo build`), et les événements
//! qui arrivent quand même (fichier exclu par extension dans un dossier
//! veillé, course à la création) passent par le même filtre avant d'agir
//! sur l'index ([`Perimetre`]). Les dossiers créés ou arrivés reçoivent
//! leurs watches, ceux qui disparaissent les rendent.
//!
//! inotify sur FUSE (ntfs-3g) et autres FS exotiques reste meilleur que
//! rien : si une racine refuse la veille, si `max_user_watches` est atteint
//! ou si le noyau signale un débordement de file (`IN_Q_OVERFLOW`), le
//! démon s'en remet à son rescan périodique — jamais de silence sur les
//! changements.

use std::collections::{BTreeSet, HashSet};
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

use notify::event::{Flag, ModifyKind};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::fileindex::scan::{self, Filtres};

/// Ce que la veille regarde : les racines et les filtres du scan.
#[derive(Debug, Clone)]
pub struct Perimetre {
    racines: Vec<PathBuf>,
    filtres: Arc<Filtres>,
}

impl Perimetre {
    pub fn new(racines: Vec<PathBuf>, filtres: Arc<Filtres>) -> Self {
        Self { racines, filtres }
    }

    /// Racine la plus précise qui contient `p` (les racines peuvent
    /// s'emboîter si l'utilisateur le veut).
    pub fn racine_de(&self, p: &Path) -> Option<&Path> {
        self.racines
            .iter()
            .filter(|r| p.starts_with(r))
            .max_by_key(|r| r.components().count())
            .map(PathBuf::as_path)
    }

    /// Le chemin serait-il indexé par le scan ? (`.gitignore` mis à part :
    /// il décide déjà où les watches sont posés.)
    pub fn admet_chemin(&self, p: &Path, est_fichier: Option<bool>) -> bool {
        self.racine_de(p)
            .is_some_and(|r| !self.filtres.ecarte_chemin(r, p, est_fichier))
    }

    /// L'action porte-t-elle sur un chemin indexable ? Le débordement est
    /// toujours admis.
    pub fn admet(&self, a: &Action) -> bool {
        match a {
            Action::Rescan => true,
            // disparu : nature inconnue, seuls noms, regex et sous-arbres
            // exclus jugent
            Action::Supprime(p) => self.admet_chemin(p, None),
            Action::SousArbre(p) => self.admet_chemin(p, Some(false)),
            Action::Maj(p) => {
                let fichier = std::fs::symlink_metadata(p).ok().map(|m| m.is_file());
                self.admet_chemin(p, fichier)
            }
        }
    }

    /// Ne garde que les actions sur des chemins indexables.
    pub fn filtrer(&self, actions: Vec<Action>) -> Vec<Action> {
        actions.into_iter().filter(|a| self.admet(a)).collect()
    }
}

/// Un veilleur par démon : émet des `Event` (ou erreurs de veille) sur un
/// canal std et tient un watch par dossier admis.
pub struct Veilleur {
    watcher: RecommendedWatcher,
    pub rx: mpsc::Receiver<Result<Event, notify::Error>>,
    perimetre: Perimetre,
    /// Dossiers sous watch, rangés : un sous-arbre y est contigu (ordre des
    /// composants), retrouvé d'un coup quand il disparaît.
    dossiers: BTreeSet<PathBuf>,
    /// Racines à la veille incomplète (refus, limite atteinte) : le démon
    /// les confie à son rescan périodique.
    incompletes: HashSet<PathBuf>,
    /// `max_user_watches` atteint : signalé une seule fois.
    sature: bool,
}

impl Veilleur {
    /// Pose les watches de chaque racine. Une racine refusée (absente, FS
    /// sans support, limite atteinte) n'empêche pas les autres ; elle est
    /// listée dans [`Veilleur::incompletes`].
    pub fn surveille(perimetre: Perimetre) -> notify::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(tx)?;
        let mut v = Self {
            watcher,
            rx,
            perimetre,
            dossiers: BTreeSet::new(),
            incompletes: HashSet::new(),
            sature: false,
        };
        for racine in v.perimetre.racines.clone() {
            v.couvrir(&racine);
        }
        Ok(v)
    }

    /// Le périmètre (filtre des événements).
    pub fn perimetre(&self) -> &Perimetre {
        &self.perimetre
    }

    /// Racines dont la veille est incomplète.
    pub fn incompletes(&self) -> &HashSet<PathBuf> {
        &self.incompletes
    }

    /// Entretient les watches pour un lot d'actions admises : les dossiers
    /// disparus rendent leurs watches, puis les dossiers apparus reçoivent
    /// les leurs. À appeler avant le sous-scan des dossiers apparus : un
    /// fichier créé pendant ce temps est vu par l'un ou par l'autre.
    /// Renvoie vrai si l'ensemble des racines incomplètes a changé.
    pub fn entretenir(&mut self, actions: &[Action]) -> bool {
        let avant = self.incompletes.len();
        // d'abord les départs : un dossier renommé garde son inode, et
        // inotify rendrait le même descripteur si on le reposait avant
        // d'avoir retiré l'ancien
        for a in actions {
            if let Action::Supprime(p) = a {
                self.oublier(p);
            }
        }
        for a in actions {
            match a {
                Action::SousArbre(p) => {
                    self.couvrir(p);
                }
                Action::Rescan => self.recouvrir(),
                _ => {}
            }
        }
        self.incompletes.len() != avant
    }

    /// Pose un watch non récursif sur chaque dossier admis du sous-arbre
    /// `dossier` (compris), parcouru avec le filtre du scan. Les dossiers
    /// déjà veillés sont laissés tels quels.
    fn couvrir(&mut self, dossier: &Path) {
        let Some(racine) = self.perimetre.racine_de(dossier).map(Path::to_path_buf) else {
            return;
        };
        if !dossier.is_dir() {
            if dossier == racine {
                self.refuser_racine(&racine, &notify::Error::path_not_found());
            }
            return;
        }
        let parcours = scan::parcours(dossier, self.perimetre.filtres.clone(), true).build();
        for res in parcours {
            // illisible (permissions) : le scan ne l'indexe pas non plus
            let Ok(e) = res else { continue };
            if !e.file_type().is_some_and(|t| t.is_dir()) {
                continue;
            }
            let p = e.into_path();
            if self.dossiers.contains(&p) {
                continue;
            }
            match self.watcher.watch(&p, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    self.dossiers.insert(p);
                }
                Err(err) if matches!(err.kind, notify::ErrorKind::MaxFilesWatch) => {
                    if !self.sature {
                        self.sature = true;
                        tracing::warn!(
                            "{}",
                            crate::tr!(
                                "limite inotify atteinte (fs.inotify.max_user_watches) après {n} dossiers : le reste de {p0} passe au rescan périodique",
                                n = self.dossiers.len(),
                                p0 = racine.display()
                            )
                        );
                    }
                    self.incompletes.insert(racine);
                    // tous les suivants échoueraient pareil
                    return;
                }
                // disparu entre le parcours et le watch : rien à veiller
                Err(err) if matches!(err.kind, notify::ErrorKind::PathNotFound) => {}
                Err(err) => {
                    if p == racine {
                        self.refuser_racine(&racine, &err);
                    } else {
                        tracing::debug!(
                            "{}",
                            crate::tr!(
                                "veille impossible sur {p0} ({e}) : rescan périodique",
                                p0 = p.display(),
                                e = err
                            )
                        );
                        self.incompletes.insert(racine.clone());
                    }
                }
            }
        }
    }

    fn refuser_racine(&mut self, racine: &Path, err: &notify::Error) {
        tracing::warn!(
            "{}",
            crate::tr!(
                "veille impossible sur {p0} ({e}) : rescan périodique",
                p0 = racine.display(),
                e = err
            )
        );
        self.incompletes.insert(racine.to_path_buf());
    }

    /// Retire les watches du sous-arbre `dossier` (compris). Un chemin qui
    /// n'était pas veillé (fichier) ne coûte qu'une recherche dans l'arbre.
    fn oublier(&mut self, dossier: &Path) {
        let sous: Vec<PathBuf> = self
            .dossiers
            .range::<Path, _>((Bound::Included(dossier), Bound::Unbounded))
            .take_while(|p| p.starts_with(dossier))
            .cloned()
            .collect();
        for p in sous {
            // déjà retiré par le noyau (dossier supprimé) : sans gravité
            let _ = self.watcher.unwatch(&p);
            self.dossiers.remove(&p);
        }
    }

    /// Après un débordement : des créations et suppressions de dossiers ont
    /// pu être perdues. Les watches des disparus sont rendus, les racines
    /// reparcourues (seuls les dossiers manquants reçoivent un watch).
    fn recouvrir(&mut self) {
        let disparus: Vec<PathBuf> = self
            .dossiers
            .iter()
            .filter(|p| !p.is_dir())
            .cloned()
            .collect();
        for p in disparus {
            let _ = self.watcher.unwatch(&p);
            self.dossiers.remove(&p);
        }
        for racine in self.perimetre.racines.clone() {
            self.couvrir(&racine);
        }
    }
}

/// Ce qu'un lot d'événements demande à l'index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    /// Créer ou mettre à jour cette entrée (stat fait à l'application).
    Maj(PathBuf),
    /// Entrée à retirer (fichier supprimé, source d'un renommage…).
    Supprime(PathBuf),
    /// Dossier apparu ou déplacé dans le volume : rescanner son sous-arbre
    /// (les contenus déplacés en bloc ne génèrent pas d'événement par
    /// fichier).
    SousArbre(PathBuf),
    /// Débordement inotify : tout reprendre, des changements ont pu être
    /// perdus.
    Rescan,
}

/// Traduit un événement notify en actions d'index. Les renommages arrivent
/// sans appariement fiable : on stat-ise chaque chemin — existant → maj,
/// disparu → suppression, dossier → sous-arbre.
pub fn actions(e: &Event) -> Vec<Action> {
    // débordement : le noyau a jeté des événements
    if e.attrs.flag() == Some(Flag::Rescan) {
        return vec![Action::Rescan];
    }
    let mut out = Vec::new();
    for p in &e.paths {
        match &e.kind {
            notify::EventKind::Remove(_) => out.push(Action::Supprime(p.clone())),
            notify::EventKind::Create(_) => {
                if est_dossier(p) {
                    out.push(Action::SousArbre(p.clone()));
                } else {
                    out.push(Action::Maj(p.clone()));
                }
            }
            notify::EventKind::Modify(ModifyKind::Name(_)) => {
                if est_dossier(p) {
                    out.push(Action::SousArbre(p.clone()));
                } else if p.exists() {
                    out.push(Action::Maj(p.clone()));
                } else {
                    out.push(Action::Supprime(p.clone()));
                }
            }
            notify::EventKind::Modify(_) => {
                if p.exists() {
                    out.push(Action::Maj(p.clone()));
                } else {
                    out.push(Action::Supprime(p.clone()));
                }
            }
            // accès, autres : sans effet sur l'index
            _ => {}
        }
    }
    out
}

/// Retire les doublons d'un lot en gardant le premier de chacun (un
/// renommage arrive en « From », « To » et « Both », plus le « MOVE_SELF »
/// du dossier lui-même : un seul sous-scan suffit).
pub fn dedoublonner(actions: Vec<Action>) -> Vec<Action> {
    let mut vues = HashSet::with_capacity(actions.len());
    actions
        .into_iter()
        .filter(|a| vues.insert(a.clone()))
        .collect()
}

fn est_dossier(p: &std::path::Path) -> bool {
    p.metadata().map(|m| m.is_dir()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    fn base_test(nom: &str) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("hyprdesk-watch-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("mkdir");
        base
    }

    fn filtres_par_defaut() -> Arc<Filtres> {
        Arc::new(Filtres {
            excludes: crate::fileindex::DEFAULT_EXCLUDES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            ..Default::default()
        })
    }

    fn cree(e: &Path) -> Event {
        Event::new(notify::EventKind::Create(notify::event::CreateKind::Any))
            .add_path(e.to_path_buf())
    }

    fn supprime(e: &Path) -> Event {
        Event::new(notify::EventKind::Remove(notify::event::RemoveKind::Any))
            .add_path(e.to_path_buf())
    }

    #[test]
    fn creation_fichier_evenement_et_classification() {
        let base = base_test("creation");
        let veilleur = Veilleur::surveille(Perimetre::new(
            vec![base.clone()],
            Arc::new(Filtres::default()),
        ))
        .expect("veilleur");
        // saturé (`max_user_watches` épuisé par un gros démon en place) :
        // l'événement ne viendra jamais, on ne teste rien ici
        if !veilleur.incompletes().is_empty() {
            let _ = fs::remove_dir_all(&base);
            return;
        }

        fs::write(base.join("nouveau.txt"), b"x").expect("write");

        // les événements arrivent vite ; on cherche celui du fichier
        let mut trouve = None;
        for _ in 0..50 {
            match veilleur.rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(e)) => {
                    if e.paths.iter().any(|p| p.ends_with("nouveau.txt")) {
                        trouve = Some(e);
                        break;
                    }
                }
                Ok(Err(_)) => continue,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            }
        }
        let e = trouve.expect("événement de création reçu");
        let act = veilleur.perimetre().filtrer(actions(&e));
        assert!(
            act.iter()
                .any(|a| matches!(a, Action::Maj(p) if p.ends_with("nouveau.txt"))),
            "classé Maj : {act:?}"
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn suppression_classee_supprime() {
        let e = supprime(Path::new("/tmp/volage.txt"));
        let act = actions(&e);
        assert!(
            act.iter()
                .any(|a| matches!(a, Action::Supprime(p) if p.ends_with("volage.txt")))
        );
    }

    #[test]
    fn debordement_reclame_un_rescan() {
        let e = Event::new(notify::EventKind::Other).set_flag(Flag::Rescan);
        assert_eq!(actions(&e), vec![Action::Rescan]);
        // le périmètre ne l'écarte jamais
        let per = Perimetre::new(vec![PathBuf::from("/nulle-part")], filtres_par_defaut());
        assert_eq!(per.filtrer(actions(&e)), vec![Action::Rescan]);
    }

    #[test]
    fn evenements_dans_git_et_target_ignores() {
        let base = base_test("filtre");
        let src = base.join("projet").join("src");
        let cible = base.join("projet").join("target").join("debug");
        fs::create_dir_all(&src).expect("mkdir");
        fs::create_dir_all(&cible).expect("mkdir");
        fs::write(src.join("main.rs"), b"x").expect("write");
        fs::write(cible.join("sortie.o"), b"x").expect("write");
        let per = Perimetre::new(vec![base.clone()], filtres_par_defaut());

        // fichier dans target/ : écarté, comme au scan
        assert!(
            per.filtrer(actions(&cree(&cible.join("sortie.o"))))
                .is_empty()
        );
        // dossier créé dans target/ : ni sous-scan ni watch
        assert!(per.filtrer(actions(&cree(&cible))).is_empty());
        // suppression dans .git (chemin disparu, nature inconnue) : écartée
        let git = base.join("projet").join(".git").join("objects").join("ab");
        assert!(per.filtrer(actions(&supprime(&git))).is_empty());
        // hors de toute racine : écarté
        assert!(
            per.filtrer(actions(&supprime(Path::new("/ailleurs/x"))))
                .is_empty()
        );
        // fichier ordinaire : gardé
        assert_eq!(
            per.filtrer(actions(&cree(&src.join("main.rs")))),
            vec![Action::Maj(src.join("main.rs"))]
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn snapshot_ignore_par_la_veille() {
        // snapshot hors de `~/.cache` (XDG_CACHE_HOME personnalisé), donc
        // sous aucun nom exclu : seul le sous-arbre exclu l'écarte
        let base = base_test("snapshot");
        let snap = base.join("cache-perso").join("hyprdesk").join("index");
        fs::create_dir_all(&snap).expect("mkdir");
        fs::write(snap.join("snapshot.tmp"), b"x").expect("write");
        let filtres = Filtres {
            chemins_exclus: vec![snap.clone()],
            ..(*filtres_par_defaut()).clone()
        };
        let per = Perimetre::new(vec![base.clone()], Arc::new(filtres));
        let renomme = Event::new(notify::EventKind::Modify(ModifyKind::Name(
            notify::event::RenameMode::Both,
        )))
        .add_path(snap.join("snapshot.tmp"))
        .add_path(snap.join("snapshot.bin"));
        assert!(
            per.filtrer(actions(&cree(&snap.join("snapshot.tmp"))))
                .is_empty()
        );
        assert!(per.filtrer(actions(&renomme)).is_empty());
        assert!(per.filtrer(actions(&cree(&snap))).is_empty());
        // le dossier parent reste veillé normalement
        assert!(
            !per.filtrer(actions(&cree(&base.join("cache-perso"))))
                .is_empty()
        );

        // et les filtres du démon portent le vrai dossier du snapshot
        let dir = crate::fileindex::store::snapshot_dir();
        let racine = dir.ancestors().nth(2).expect("ancêtre").to_path_buf();
        let demon = Perimetre::new(
            vec![racine],
            Arc::new(Filtres::depuis_config(&crate::fileindex::IndexConfig {
                roots: vec![],
                ..Default::default()
            })),
        );
        assert!(!demon.admet(&Action::Supprime(dir.join("snapshot.bin"))));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn pas_de_watch_dans_les_dossiers_exclus() {
        let base = base_test("exclus");
        for d in [
            "src/module",
            ".git/objects/ab",
            "target/debug/deps",
            "node_modules/paquet",
            ".cache/fontconfig",
        ] {
            fs::create_dir_all(base.join(d)).expect("mkdir");
        }
        let v = Veilleur::surveille(Perimetre::new(vec![base.clone()], filtres_par_defaut()))
            .expect("veilleur");
        if !v.incompletes().is_empty() {
            let _ = fs::remove_dir_all(&base);
            return; // limite inotify atteinte sur la machine de test
        }
        let veilles: HashSet<PathBuf> = v.dossiers.iter().cloned().collect();
        let attendus: HashSet<PathBuf> = [
            base.clone(),
            base.join("src"),
            base.join("src").join("module"),
        ]
        .into_iter()
        .collect();
        assert_eq!(veilles, attendus);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn watches_suivent_creations_et_suppressions() {
        let base = base_test("suivi");
        let mut v = Veilleur::surveille(Perimetre::new(vec![base.clone()], filtres_par_defaut()))
            .expect("veilleur");
        if !v.incompletes().is_empty() {
            let _ = fs::remove_dir_all(&base);
            return;
        }
        let neuf = base.join("neuf");
        fs::create_dir_all(neuf.join("sous")).expect("mkdir");
        fs::create_dir_all(neuf.join("target").join("x")).expect("mkdir");
        v.entretenir(&[Action::SousArbre(neuf.clone())]);
        assert!(v.dossiers.contains(&neuf));
        assert!(v.dossiers.contains(&neuf.join("sous")));
        assert!(!v.dossiers.contains(&neuf.join("target")));

        // voisin au nom proche : hors du sous-arbre à oublier
        let voisin = base.join("neuf-bis");
        fs::create_dir_all(&voisin).expect("mkdir");
        v.entretenir(&[Action::SousArbre(voisin.clone())]);

        fs::remove_dir_all(&neuf).expect("rm");
        v.entretenir(&[Action::Supprime(neuf.clone())]);
        assert!(!v.dossiers.iter().any(|p| p.starts_with(&neuf)));
        assert!(v.dossiers.contains(&voisin));
        assert!(v.dossiers.contains(&base));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn doublons_d_un_renommage_fusionnes() {
        let a = PathBuf::from("/tmp/a");
        let b = PathBuf::from("/tmp/b");
        let lot = vec![
            Action::Supprime(a.clone()),
            Action::SousArbre(b.clone()),
            Action::Supprime(a.clone()),
            Action::SousArbre(b.clone()),
        ];
        assert_eq!(
            dedoublonner(lot),
            vec![Action::Supprime(a), Action::SousArbre(b)]
        );
    }
}
