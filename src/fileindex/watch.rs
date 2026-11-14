//! Veille du système de fichiers : inotify (via `notify`), traduit en
//! actions d'index à appliquer en lots.
//!
//! inotify sur FUSE (ntfs-3g) et autres FS exotiques reste meilleur que
//! rien : si une racine refuse la veille ou si le noyau signale un
//! débordement de file (`IN_Q_OVERFLOW`), le démon repasse sur son rescan
//! périodique — jamais de silence sur les changements.

use std::path::PathBuf;
use std::sync::mpsc;

use notify::event::{Flag, ModifyKind};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

/// Un veilleur par démon : émet des `Event` (ou erreurs de veille) sur un
/// canal std.
pub struct Veilleur {
    _watcher: RecommendedWatcher,
    pub rx: mpsc::Receiver<Result<Event, notify::Error>>,
}

impl Veilleur {
    /// Regarde chaque racine récursivement. Renvoie aussi les racines qui
    /// ont refusé (limite `max_user_watches`, FS sans support) : le démon
    /// les confie à son rescan périodique.
    pub fn surveille(racines: &[PathBuf]) -> notify::Result<(Self, Vec<PathBuf>)> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        let mut refuses = Vec::new();
        for racine in racines {
            if let Err(e) = watcher.watch(racine, RecursiveMode::Recursive) {
                tracing::warn!(
                    "{}",
                    crate::tr!(
                        "veille impossible sur {p0} ({e}) : rescan périodique",
                        p0 = racine.display(),
                        e = e
                    )
                );
                refuses.push(racine.clone());
            }
        }
        Ok((
            Self {
                _watcher: watcher,
                rx,
            },
            refuses,
        ))
    }
}

/// Ce qu'un lot d'événements demande à l'index.
#[derive(Debug, Clone, PartialEq)]
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

fn est_dossier(p: &std::path::Path) -> bool {
    p.metadata().map(|m| m.is_dir()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn creation_fichier_evenement_et_classification() {
        let base = std::env::temp_dir().join(format!("hyprdesk-watch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("mkdir");

        let (veilleur, refuses) =
            Veilleur::surveille(std::slice::from_ref(&base)).expect("veilleur");
        // saturé (`max_user_watches` épuisé par un gros démon en place) :
        // l'événement ne viendra jamais, on ne teste rien ici
        if !refuses.is_empty() {
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
        let act = actions(&e);
        assert!(
            act.iter()
                .any(|a| matches!(a, Action::Maj(p) if p.ends_with("nouveau.txt"))),
            "classé Maj : {act:?}"
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn suppression_classee_supprime() {
        let e = Event::new(notify::EventKind::Remove(notify::event::RemoveKind::File))
            .add_path(Path::new("/tmp/volage.txt").to_path_buf());
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
    }
}
