//! Le démon d'indexation : `hyprdesk index daemon`.
//!
//! Trois fils d'exécution autour d'un état partagé : le serveur de socket
//! (tokio), la veille inotify (lots appliqués au fil des événements) et la
//! maintenance (rescan périodique et demandé, sauvegarde du snapshot,
//! déchargement après 10 minutes d'inactivité — rechargé depuis le
//! snapshot en moins de deux secondes à la requête suivante).
//!
//! Pendant le premier scan, les requêtes renvoient la progression au lieu
//! de résultats ; le snapshot précédent reste servi s'il existe, le rescan
//! ne le remplace qu'à la fin. SIGTERM/SIGINT sauvegardent avant de sortir
//! (unité systemd, Ctrl+C).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::fileindex::index::{EntryMeta, Index, KIND_DIR, KIND_FILE, KIND_LINK, Volume};
use crate::fileindex::proto::{Reply, Request, socket_path};
use crate::fileindex::{IndexConfig, scan, store, watch};

/// Déchargement de l'index après autant de secondes sans requête.
const IDLE_UNLOAD_SECS: i64 = 600;
/// Sauvegarde du snapshot au plus souvent toutes ces secondes.
const SAVE_DEBOUNCE_SECS: i64 = 60;
/// Plafond de résultats par requête (le lanceur classe ensuite).
const MAX_HITS: usize = 200;
/// Plafond d'une ligne de requête (localement suffisant).
const MAX_LINE: usize = 64 * 1024;

/// État partagé du démon.
struct Etat {
    config: IndexConfig,
    /// Filtres d'indexation compilés une fois pour toutes (noms, regex,
    /// extensions, taille max).
    filtres: scan::Filtres,
    index: RwLock<Option<Index>>,
    /// Scan complet en cours (les requêtes annoncent la progression).
    scanning: AtomicBool,
    /// Entrées vues par le scan en cours.
    progress: Arc<AtomicUsize>,
    /// Époques (s) : dernier scan fini, dernière requête, dernière sauvegarde.
    dernier_scan: AtomicI64,
    dernier_usage: AtomicI64,
    derniere_sauvegarde: AtomicI64,
    /// Rescan complet demandé (Rebuild, snapshot absent, événements perdus).
    rescan: AtomicBool,
    /// Changements non encore sauvegardés (veille incrémentale).
    dirty: AtomicBool,
    /// Changements survenus pendant le déchargement : le snapshot est
    /// périmé, le chargement réclamera un rescan.
    perte_evenements: AtomicBool,
    /// Arrêt propre demandé (`Stop`).
    stop: AtomicBool,
    /// Racines effectivement sous veille inotify (les autres en rescan
    /// périodique).
    veilles: Mutex<HashSet<PathBuf>>,
}

impl Etat {
    fn new(config: IndexConfig) -> Self {
        let filtres = scan::Filtres {
            excludes: config.excludes().into_iter().collect(),
            regexes: scan::compile_regexes(&config.exclude_regex),
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
        };
        let maintenant = epoque();
        Self {
            config,
            filtres,
            index: RwLock::new(None),
            scanning: AtomicBool::new(false),
            progress: Arc::new(AtomicUsize::new(0)),
            dernier_scan: AtomicI64::new(0),
            dernier_usage: AtomicI64::new(maintenant),
            derniere_sauvegarde: AtomicI64::new(maintenant),
            rescan: AtomicBool::new(false),
            dirty: AtomicBool::new(false),
            perte_evenements: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            veilles: Mutex::new(HashSet::new()),
        }
    }
}

fn epoque() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Point d'entrée `hyprdesk index daemon` : bloque jusqu'à l'arrêt.
pub fn run(config: IndexConfig) -> anyhow::Result<()> {
    let etat = Arc::new(Etat::new(config));
    if !charger_snapshot(&etat) {
        etat.rescan.store(true, Ordering::SeqCst);
    }

    let veille_etat = etat.clone();
    std::thread::Builder::new()
        .name("fileindex-watch".into())
        .spawn(move || boucle_veille(&veille_etat))?;

    let maint_etat = etat.clone();
    std::thread::Builder::new()
        .name("fileindex-maint".into())
        .spawn(move || boucle_maintenance(&maint_etat))?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(servir(etat))
}

// ——————————————————————— chargement / scan ———————————————————————

/// Charge le snapshot s'il existe et couvre les racines configurées.
/// Renvoie vrai si un index est chargé.
fn charger_snapshot(etat: &Arc<Etat>) -> bool {
    match store::load() {
        Ok(idx) => {
            if racines_compatibles(&idx, &etat.config.roots) {
                let n = idx.len();
                *etat.index.write().unwrap_or_else(|e| e.into_inner()) = Some(idx);
                etat.dernier_scan.store(epoque(), Ordering::SeqCst);
                etat.derniere_sauvegarde.store(epoque(), Ordering::SeqCst);
                tracing::info!("{}", crate::tr!("index rechargé : {n} fichiers", n = n));
                true
            } else {
                // racines configurées ≠ snapshot : périmé, on reprend tout
                tracing::info!(
                    "{}",
                    crate::tr!("racines de l'index changées : rescan complet")
                );
                false
            }
        }
        Err(e) => {
            if !e.is_absent() {
                tracing::warn!("{}", crate::tr!("snapshot illisible : {e}", e = e));
            }
            false
        }
    }
}

/// Le snapshot couvre-t-il exactement les racines configurées ?
fn racines_compatibles(idx: &Index, racines: &[PathBuf]) -> bool {
    let a: HashSet<&str> = idx.volumes().iter().map(|v| v.root.as_str()).collect();
    let b: HashSet<&str> = racines.iter().map(|r| r.to_str().unwrap_or("")).collect();
    a == b
}

/// Rescan complet : reconstruit l'index et remplace l'ancien d'un coup.
fn scan_complet(etat: &Arc<Etat>) {
    if etat.scanning.swap(true, Ordering::SeqCst) {
        return; // un scan est déjà en route
    }
    let t0 = Instant::now();
    etat.progress.store(0, Ordering::Relaxed);
    let mut entries: Vec<(String, EntryMeta)> = Vec::new();
    let mut vols: Vec<Volume> = Vec::new();
    for (i, racine) in etat.config.roots.iter().enumerate() {
        if !racine.exists() {
            tracing::warn!(
                "{}",
                crate::tr!(
                    "racine d'index absente, ignorée : {p0}",
                    p0 = racine.display()
                )
            );
            continue;
        }
        let fs = scan::fs_type(racine);
        let (ents, st) = scan::scan_volume(
            racine,
            &etat.filtres,
            i.min(255) as u8,
            Some(etat.progress.clone()),
        );
        tracing::info!(
            "{}",
            crate::tr!(
                "volume {p0} ({fs}) : {n} entrées, {e} erreurs",
                p0 = racine.display(),
                fs = fs,
                n = ents.len(),
                e = st.errors
            )
        );
        entries.extend(ents);
        vols.push(Volume {
            root: racine.to_string_lossy().into_owned(),
            fs,
        });
    }
    let idx = Index::build(entries, vols);
    let n = idx.len();
    *etat.index.write().unwrap_or_else(|e| e.into_inner()) = Some(idx);
    etat.scanning.store(false, Ordering::SeqCst);
    etat.dernier_scan.store(epoque(), Ordering::SeqCst);
    etat.rescan.store(false, Ordering::SeqCst);
    etat.perte_evenements.store(false, Ordering::SeqCst);
    etat.dirty.store(false, Ordering::SeqCst);
    sauvegarder(etat);
    tracing::info!(
        "{}",
        crate::tr!(
            "index prêt : {n} fichiers en {s} s",
            n = n,
            s = format!("{:.1}", t0.elapsed().as_secs_f32())
        )
    );
}

fn sauvegarder(etat: &Arc<Etat>) {
    let g = etat.index.read().unwrap_or_else(|e| e.into_inner());
    let Some(idx) = g.as_ref() else { return };
    match store::save(idx) {
        Ok(()) => {
            etat.derniere_sauvegarde.store(epoque(), Ordering::SeqCst);
            etat.dirty.store(false, Ordering::SeqCst);
        }
        Err(e) => tracing::warn!("{}", crate::tr!("sauvegarde de l'index : {e}", e = e)),
    }
}

// ——————————————————————— veille incrémentale ———————————————————————

fn boucle_veille(etat: &Arc<Etat>) {
    let (veilleur, refuses) = match watch::Veilleur::surveille(&etat.config.roots) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "{}",
                crate::tr!("veille filesystem indisponible : {e}", e = e)
            );
            return;
        }
    };
    let surveillees: HashSet<PathBuf> = etat
        .config
        .roots
        .iter()
        .filter(|r| !refuses.contains(r))
        .cloned()
        .collect();
    *etat.veilles.lock().unwrap_or_else(|e| e.into_inner()) = surveillees;
    if !refuses.is_empty() {
        tracing::info!(
            "{}",
            crate::tr!(
                "{n} racine(s) sans veille inotify : rescan périodique",
                n = refuses.len()
            )
        );
    }

    loop {
        if etat.stop.load(Ordering::Relaxed) {
            return;
        }
        let mut lot = Vec::new();
        match veilleur.rx.recv_timeout(Duration::from_millis(400)) {
            Ok(Ok(e)) => {
                lot.extend(watch::actions(&e));
                while let Ok(Ok(e)) = veilleur.rx.try_recv() {
                    lot.extend(watch::actions(&e));
                }
            }
            Ok(Err(e)) => {
                // erreur ponctuelle du veilleur (watch raté…) : notée, la
                // rescan périodique couvrira
                tracing::warn!("{}", crate::tr!("veille filesystem : {e}", e = e));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        }
        appliquer_lot(etat, lot);
    }
}

/// Ce qu'il faut faire d'un lot d'actions, appliqué sous verrou.
struct Lot {
    maj: Vec<(String, EntryMeta)>,
    supprime: Vec<PathBuf>,
    change: bool,
}

fn appliquer_lot(etat: &Arc<Etat>, actions: Vec<watch::Action>) {
    if actions.is_empty() {
        return;
    }
    let mut lot = Lot {
        maj: Vec::new(),
        supprime: Vec::new(),
        change: false,
    };
    if actions.iter().any(|a| matches!(a, watch::Action::Rescan)) {
        // le noyau a jeté des événements : des changements ont pu échapper
        etat.perte_evenements.store(true, Ordering::SeqCst);
        etat.rescan.store(true, Ordering::SeqCst);
        return;
    }

    // hors verrou : stat des chemins et scans de sous-arbres (peut prendre
    // quelques secondes sur un gros déplacement) ; les requêtes continuent
    // d'être servies entre-temps
    let mut sous_scan: Vec<(String, EntryMeta)> = Vec::new();
    for a in actions {
        match a {
            watch::Action::Maj(p) => {
                if let Some((path, meta)) = meta_de(&etat.config, &p) {
                    lot.maj.push((path, meta));
                }
            }
            watch::Action::Supprime(p) => {
                lot.supprime.push(p);
            }
            watch::Action::SousArbre(p) => {
                if let Some(vol) = vol_de(&etat.config, &p) {
                    let (ents, _) = scan::scan_volume(&p, &etat.filtres, vol, None);
                    sous_scan.extend(ents);
                }
            }
            watch::Action::Rescan => unreachable!("traité au-dessus"),
        }
    }

    let mut g = etat.index.write().unwrap_or_else(|e| e.into_inner());
    let Some(idx) = g.as_mut() else {
        // index déchargé (inactivité) : le prochain chargement repartira du
        // snapshot, donc d'un rescan pour couvrir ce qui a changé ici
        etat.perte_evenements.store(true, Ordering::SeqCst);
        return;
    };
    for (path, meta) in sous_scan.drain(..) {
        idx.push(&path, meta);
        lot.change = true;
    }
    for (path, meta) in lot.maj.drain(..) {
        idx.push(&path, meta);
        lot.change = true;
    }
    for p in lot.supprime.drain(..) {
        if idx.remove_path(&p.to_string_lossy()) {
            lot.change = true;
        }
        // un dossier supprimé emporte ses enfants : purge en bloc
        if p.is_dir() || p.symlink_metadata().is_err() {
            let perdu = idx.remove_prefix(&p.to_string_lossy());
            if perdu > 0 {
                tracing::debug!(
                    "{}",
                    crate::tr!("{n} entrées purgées sous {p0}", n = perdu, p0 = p.display())
                );
                lot.change = true;
            }
        }
    }
    drop(g);
    if lot.change {
        if let Ok(mut g) = etat.index.write()
            && let Some(idx) = g.as_mut()
        {
            idx.compact();
        }
        etat.dirty.store(true, Ordering::SeqCst);
    }
}

/// (chemin, méta) d'un chemin actuel, avec son volume.
fn meta_de(config: &IndexConfig, p: &Path) -> Option<(String, EntryMeta)> {
    let m = std::fs::symlink_metadata(p).ok()?;
    let kind = if m.is_dir() {
        KIND_DIR
    } else if m.is_symlink() {
        KIND_LINK
    } else {
        KIND_FILE
    };
    let mtime = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let vol = vol_de(config, p)?;
    Some((
        p.to_string_lossy().into_owned(),
        EntryMeta {
            kind,
            size: m.len(),
            mtime,
            vol,
        },
    ))
}

/// Volume d'un chemin : position de la racine qui le contient (l'ordre des
/// racines définit les identifiants de volume au scan).
fn vol_de(config: &IndexConfig, p: &Path) -> Option<u8> {
    config
        .roots
        .iter()
        .position(|r| p.starts_with(r))
        .map(|i| i.min(255) as u8)
}

// ——————————————————————— maintenance ———————————————————————

fn boucle_maintenance(etat: &Arc<Etat>) {
    while !etat.stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(250));
        if etat.stop.load(Ordering::Relaxed) {
            break;
        }
        let maintenant = epoque();
        let scanning = etat.scanning.load(Ordering::Relaxed);

        // rescan demandé ou requis (premier lancement sans snapshot)
        if etat.rescan.load(Ordering::Relaxed) && !scanning {
            scan_complet(etat);
            continue;
        }
        // rescan périodique de sûreté (inotify a pu rater des choses)
        let dernier = etat.dernier_scan.load(Ordering::Relaxed);
        if !scanning && dernier > 0 && maintenant - dernier >= etat.config.interval as i64 {
            tracing::info!("{}", crate::tr!("rescan périodique de l'index"));
            scan_complet(etat);
            continue;
        }
        // sauvegarde débordée des changements incrémentaux
        if !scanning
            && etat.dirty.load(Ordering::Relaxed)
            && maintenant - etat.derniere_sauvegarde.load(Ordering::Relaxed) >= SAVE_DEBOUNCE_SECS
        {
            sauvegarder(etat);
        }
        // déchargement après inactivité : la RAM redevient libre, la
        // requête suivante rechargera le snapshot en moins de deux secondes
        if !scanning
            && maintenant - etat.dernier_usage.load(Ordering::Relaxed) >= IDLE_UNLOAD_SECS
            && etat
                .index
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .is_some()
        {
            sauvegarder(etat);
            *etat.index.write().unwrap_or_else(|e| e.into_inner()) = None;
            tracing::info!(
                "{}",
                crate::tr!("index déchargé (inactivité) — rechargement à la prochaine requête")
            );
        }
    }
    // arrêt propre (Stop) : dernière sauvegarde
    sauvegarder(etat);
    std::process::exit(0);
}

// ——————————————————————— serveur de socket ———————————————————————

async fn servir(etat: Arc<Etat>) -> anyhow::Result<()> {
    let path = socket_path();
    // un démon déjà en place ? on sonne avant de toucher à sa socket
    if std::os::unix::net::UnixStream::connect(&path).is_ok() {
        anyhow::bail!(
            "{}",
            crate::tr!("un démon d'index tourne déjà ({p0})", p0 = path.display())
        );
    }
    let _ = std::fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let listener = UnixListener::bind(&path)?;
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    tracing::info!(
        "{}",
        crate::tr!("index en écoute sur {p0}", p0 = path.display())
    );

    // SIGTERM (systemd) / SIGINT (Ctrl+C) : sauvegarde puis sortie
    let sig_etat = etat.clone();
    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        let Ok(mut term) = signal(SignalKind::terminate()) else {
            return;
        };
        let Ok(mut int) = signal(SignalKind::interrupt()) else {
            return;
        };
        tokio::select! {
            _ = term.recv() => {},
            _ = int.recv() => {},
        }
        tracing::info!("{}", crate::tr!("signal reçu : sauvegarde de l'index"));
        sauvegarder(&sig_etat);
        std::process::exit(0);
    });

    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let etat = etat.clone();
        tokio::spawn(async move {
            let _ = gerer(stream, &etat).await;
        });
    }
}

async fn gerer(stream: UnixStream, etat: &Arc<Etat>) -> std::io::Result<()> {
    let (r, mut w) = tokio::io::split(stream);
    let mut lecteur = BufReader::new(r);
    let mut ligne = String::new();
    if lecteur.read_line(&mut ligne).await? == 0 || ligne.len() > MAX_LINE {
        return Ok(());
    }
    let rep = match serde_json::from_str::<Request>(ligne.trim()) {
        Ok(req) => traiter(etat, req),
        Err(e) => Reply::Err(format!("{} : {e}", crate::tr!("requête illisible"))),
    };
    let mut out = serde_json::to_string(&rep).unwrap_or_default();
    out.push('\n');
    w.write_all(out.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

/// Traite une requête — tout est rapide sauf un premier scan, annoncé
/// plutôt que bloquant.
fn traiter(etat: &Arc<Etat>, req: Request) -> Reply {
    match req {
        Request::Query {
            q,
            limit,
            kinds,
            cat,
        } => {
            etat.dernier_usage.store(epoque(), Ordering::SeqCst);
            if etat.scanning.load(Ordering::Relaxed) {
                return en_scan(etat);
            }
            // index déchargé par l'inactivité : rechargement paresseux
            if etat
                .index
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .is_none()
                && (etat.perte_evenements.load(Ordering::Relaxed) || !charger_snapshot(etat))
            {
                etat.rescan.store(true, Ordering::SeqCst);
                return en_scan(etat);
            }
            let t0 = Instant::now();
            let g = etat.index.read().unwrap_or_else(|e| e.into_inner());
            let Some(idx) = g.as_ref() else {
                return en_scan(etat);
            };
            let hits = idx.search(&q, limit.clamp(1, MAX_HITS), kinds.as_deref(), cat);
            Reply::Hits {
                hits,
                total: idx.len(),
                elapsed_ms: t0.elapsed().as_millis() as u64,
                scanning: None,
            }
        }
        Request::Status => Reply::Status(statut_json(etat)),
        Request::Rebuild => {
            etat.rescan.store(true, Ordering::SeqCst);
            Reply::Ok
        }
        Request::Stop => {
            etat.stop.store(true, Ordering::SeqCst);
            Reply::Ok
        }
    }
}

/// Réponse « scan en cours » avec la progression.
fn en_scan(etat: &Arc<Etat>) -> Reply {
    Reply::Hits {
        hits: Vec::new(),
        total: 0,
        elapsed_ms: 0,
        scanning: Some(etat.progress.load(Ordering::Relaxed) as u64),
    }
}

/// État du démon en JSON libre (CLI `hyprdesk index status`, widget Lua).
fn statut_json(etat: &Arc<Etat>) -> String {
    let g = etat.index.read().unwrap_or_else(|e| e.into_inner());
    let (fichiers, volumes): (usize, Vec<&Volume>) = g
        .as_ref()
        .map(|i| (i.len(), i.volumes().iter().collect()))
        .unwrap_or_default();
    let phase = if etat.scanning.load(Ordering::Relaxed) {
        "scan"
    } else if fichiers > 0 {
        "pret"
    } else {
        "attente"
    };
    let veilles: Vec<String> = etat
        .veilles
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    let taille_snapshot = std::fs::metadata(store::snapshot_path())
        .map(|m| m.len())
        .unwrap_or(0);
    json!({
        "phase": phase,
        "progression": etat.progress.load(Ordering::Relaxed),
        "fichiers": fichiers,
        "volumes": volumes,
        "veilles": veilles,
        "dernier_scan": etat.dernier_scan.load(Ordering::Relaxed),
        "interval_s": etat.config.interval,
        "snapshot": store::snapshot_path().display().to_string(),
        "snapshot_size": taille_snapshot,
        "pid": std::process::id(),
    })
    .to_string()
}
