//! Protocole lanceur ↔ démon d'index et client de service.
//!
//! Une ligne JSON par requête sur le socket unix, une ligne JSON en
//! réponse (même gabarit que la socket de commande de hyprdesk). Le
//! lanceur (mode `:f`) et la CLI (`hyprdesk index query|status|rebuild`)
//! passent tous deux par `request`.

use std::io::{BufRead, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::fileindex::index::Hit;

/// Préfixe court du mode fichiers dans le lanceur.
pub const PREFIX: &str = ":f";
/// Préfixe long, plus lisible dans les raccourcis documentés.
pub const PREFIX_LONG: &str = ":fichiers";

/// Plafond servi au lanceur : au-delà, le classement du lanceur suffira.
pub const LIMIT_DEFAULT: usize = 100;

/// Requête envoyée au démon.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "arg")]
pub enum Request {
    /// Recherche par sous-chaîne (nom, ou chemin si « / ») ; `kinds`
    /// restreint la nature des résultats ([`KIND_FILE`], [`KIND_DIR`],
    /// [`KIND_LINK`]) — absent : toutes.
    Query {
        q: String,
        limit: usize,
        kinds: Option<Vec<u8>>,
        /// Catégorie demandée (code [`categorie_de`]) — absent : toutes.
        cat: Option<u8>,
    },
    /// État du démon (JSON libre).
    Status,
    /// Rescan complet demandé.
    Rebuild,
    /// Arrêt propre (sauvegarde puis sortie).
    Stop,
}

/// Réponse du démon.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "rep", content = "data")]
pub enum Reply {
    /// Résultats ; `scanning` présent tant que le premier scan n'a pas fini
    /// (progression en entrées vues).
    Hits {
        hits: Vec<Hit>,
        total: usize,
        elapsed_ms: u64,
        scanning: Option<u64>,
        /// Index en cours de chargement depuis son snapshot (démarrage du
        /// démon, ou reprise après déchargement) : aucun résultat servi,
        /// redemander dans un instant. Absent des réponses d'un démon
        /// antérieur : faux.
        #[serde(default)]
        loading: bool,
    },
    /// État (chaîne JSON, affichée ou analysée telle quelle).
    Status(String),
    /// Acquiescement simple.
    Ok,
    /// Erreur côté démon.
    Err(String),
}

/// Chemin du socket : sous le répertoire d'instance Hyprland (comme
/// `hyprdesk.sock`), avec repli hors session pour la CLI.
pub fn socket_path() -> PathBuf {
    if let Ok(p) = std::env::var("HYPRDESK_INDEX_SOCK")
        && !p.is_empty()
    {
        return PathBuf::from(p);
    }
    if let Ok(d) = hyprpanel_core::ipc::instance_dir() {
        return d.join("hyprdesk-index.sock");
    }
    crate::policy::private_runtime_or_unusable().join("hyprdesk-index.sock")
}

/// Le lanceur demande-t-il une recherche de fichiers ?
/// « :f mot », « :fichiers docs/mot » → « mot », « docs/mot ».
pub fn query(q: &str) -> Option<&str> {
    let rest = q.trim_start();
    let rest = rest
        .strip_prefix(PREFIX_LONG)
        .or_else(|| rest.strip_prefix(PREFIX))?;
    (rest.is_empty() || rest.starts_with(' ')).then(|| rest.trim())
}

/// Client bloquant : une requête, une réponse.
pub fn request(req: &Request) -> Result<Reply, String> {
    let path = socket_path();
    let s = UnixStream::connect(&path).map_err(|e| format!("{} : {e}", path.display()))?;
    s.set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|e| e.to_string())?;
    s.set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let ligne = serde_json::to_string(req).map_err(|e| e.to_string())?;
    (&s).write_all(ligne.as_bytes())
        .and_then(|_| (&s).write_all(b"\n"))
        .map_err(|e| e.to_string())?;
    let mut lecteur = std::io::BufReader::new(&s);
    let mut rep = String::new();
    lecteur.read_line(&mut rep).map_err(|e| e.to_string())?;
    serde_json::from_str(rep.trim()).map_err(|e| format!("réponse illisible : {e}"))
}

/// Un seul lancement à la fois dans ce processus : des frappes rapprochées
/// dans `:f` (et le démarrage avec hyprdesk) ne lancent pas chacune leur
/// démon — le second appel trouve la socket du premier.
static LANCEMENT: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Le démon tourne-t-il ? Sinon, le lance en arrière-plan (même binaire)
/// et attend la socket, avec les réglages résolus par hyprdesk (`index.*` —
/// racines vides : défauts du démon). Utilisé au démarrage de hyprdesk
/// (`index.autostart`), par le lanceur au premier `:f` et par la page
/// Réglages.
///
/// Le démon lie sa socket avant de charger son snapshot : il répond en
/// quelques millisecondes, puis annonce le chargement (`loading`) tant
/// que l'index n'est pas en mémoire.
pub fn ensure_daemon(config: &crate::fileindex::IndexConfig) -> Result<(), String> {
    if request(&Request::Status).is_ok() {
        return Ok(());
    }
    let _verrou = LANCEMENT.lock().unwrap_or_else(|e| e.into_inner());
    // lancé entre-temps par un autre appel
    if request(&Request::Status).is_ok() {
        return Ok(());
    }
    let mut enfant = commande(config)
        .spawn()
        .map_err(|e| format!("{} : {e}", crate::tr!("lancement du démon d'index")))?;
    // Le démon survit à hyprdesk (groupe à lui) ; tant qu'il est notre
    // enfant, sa fin est recueillie ici plutôt que de laisser un zombie.
    let pid = enfant.id();
    let attente = std::thread::Builder::new()
        .name("fileindex-reap".into())
        .spawn(move || {
            let _ = enfant.wait();
        });
    if let Err(e) = attente {
        tracing::warn!(
            "{}",
            crate::tr!("démon d'index (pid {pid}) : {e}", pid = pid, e = e)
        );
    }
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        if request(&Request::Status).is_ok() {
            return Ok(());
        }
    }
    Err(crate::tr!("le démon d'index ne répond pas").to_string())
}

/// Ligne de commande du démon : réglages en arguments, sorties fermées,
/// groupe de processus à lui, priorité basse (processeur et disque) — un
/// scan complet ne dispute pas la machine à la session.
fn commande(config: &crate::fileindex::IndexConfig) -> std::process::Command {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hyprdesk"));
    let mut cmd = Command::new(exe);
    cmd.args(["index", "daemon"]);
    if !config.roots.is_empty() {
        cmd.arg("--paths").arg(
            config
                .roots
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    if !config.extra_excludes.is_empty() {
        cmd.arg("--exclude").arg(config.extra_excludes.join(","));
    }
    if !config.exclude_regex.is_empty() {
        cmd.arg("--exclude-regex")
            .arg(config.exclude_regex.join(","));
    }
    if !config.exclude_ext.is_empty() {
        cmd.arg("--exclude-ext").arg(config.exclude_ext.join(","));
    }
    if !config.exclude_cat.is_empty() {
        cmd.arg("--exclude-cat").arg(config.exclude_cat.join(","));
    }
    if config.max_size > 0 {
        cmd.arg("--max-size").arg(config.max_size.to_string());
    }
    cmd.arg("--interval").arg(config.interval.to_string());
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    // SAFETY : entre fork et exec, seulement deux appels système sans
    // allocation ; un échec laisse la priorité normale.
    unsafe {
        cmd.pre_exec(|| {
            priorite_basse();
            Ok(())
        });
    }
    cmd
}

/// nice 10 et classe d'E/S « best effort » au plus bas (7) : le démon
/// reste servi sans attendre, mais cède au reste de la session.
fn priorite_basse() {
    // ioprio : classe << 13 | niveau ; IOPRIO_CLASS_BE = 2, qui = processus.
    const IOPRIO_WHO_PROCESS: libc::c_long = 1;
    const IOPRIO_BE_BAS: libc::c_long = (2 << 13) | 7;
    // SAFETY : appels sans pointeur, sur le processus courant.
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
        libc::syscall(
            libc::SYS_ioprio_set,
            IOPRIO_WHO_PROCESS,
            0 as libc::c_long,
            IOPRIO_BE_BAS,
        );
    }
}

/// Arrête proprement le démon s'il tourne (sauvegarde du snapshot).
pub fn stop() -> Result<(), String> {
    request(&Request::Stop).map(|_| ()).map_err(|e| {
        if e.contains("Connexion refusée") || e.contains("No such file") {
            crate::tr!("le démon d'index n'est pas lancé").to_string()
        } else {
            e
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_du_mode_fichiers() {
        assert_eq!(query(":f hypr"), Some("hypr"));
        assert_eq!(query(":fichiers docs/lisez"), Some("docs/lisez"));
        assert_eq!(query(":f"), Some(""));
        assert_eq!(query(":f   "), Some(""));
        assert_eq!(query(":fils de discussion"), None); // « :fils » ≠ « :f » seul
        assert_eq!(query("hypr"), None);
        assert_eq!(query(":clip x"), None);
    }

    #[test]
    fn aller_retour_serde_des_requetes() {
        for req in [
            Request::Query {
                q: "hypr".into(),
                limit: 10,
                kinds: None,
                cat: None,
            },
            Request::Query {
                q: "hypr".into(),
                limit: 10,
                kinds: Some(vec![crate::fileindex::index::KIND_DIR]),
                cat: Some(crate::fileindex::index::CAT_IMAGES),
            },
            Request::Status,
            Request::Rebuild,
            Request::Stop,
        ] {
            let s = serde_json::to_string(&req).expect("serde");
            let r: Request = serde_json::from_str(&s).expect("serde");
            assert_eq!(format!("{r:?}"), format!("{req:?}"));
        }
    }
}
