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
//!
//! Au repos, aucun fil ne se réveille pour rien : la veille dort sur son
//! canal d'événements, la maintenance sur une `Condvar` jusqu'à sa
//! prochaine échéance (sauvegarde, déchargement, rescan périodique) ou
//! jusqu'à ce qu'on la sonne (Rebuild, Stop, index rechargé, changements
//! à sauvegarder).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
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
/// Plancher du rescan périodique : `index.interval = 0` ne doit pas
/// enchaîner les scans complets.
const MIN_INTERVAL_SECS: u64 = 60;
/// Attente maximale de la maintenance : l'horloge monotone de la `Condvar`
/// s'arrête pendant une mise en veille, l'échéance (en temps réel) est
/// donc revérifiée au moins aussi souvent.
const MAX_ATTENTE_SECS: u64 = 900;
/// Actions distinctes gardées pendant le déchargement ; au-delà, un rescan
/// complet coûte moins que de les rejouer.
const MAX_EN_ATTENTE: usize = 20_000;

/// État partagé du démon.
struct Etat {
    config: IndexConfig,
    /// Filtres d'indexation compilés une fois pour toutes (noms, regex,
    /// extensions, taille max, dossier du snapshot), partagés par le scan
    /// et la veille.
    filtres: Arc<scan::Filtres>,
    index: RwLock<Option<Index>>,
    /// Scan complet en cours (les requêtes annoncent la progression).
    scanning: AtomicBool,
    /// Entrées vues par le scan en cours.
    progress: Arc<AtomicUsize>,
    /// Époques (s) : dernier scan fini, dernière requête, dernière tentative
    /// de sauvegarde (réussie ou non : un échec réessaie au délai suivant).
    dernier_scan: AtomicI64,
    dernier_usage: AtomicI64,
    derniere_sauvegarde: AtomicI64,
    /// Rescan complet demandé (Rebuild, snapshot absent, événements perdus).
    rescan: AtomicBool,
    /// Changements non encore sauvegardés (veille incrémentale).
    dirty: AtomicBool,
    /// Changements perdus (débordement inotify, file d'attente pleine,
    /// snapshot non écrit au déchargement) : le snapshot est périmé, le
    /// chargement réclamera un rescan.
    perte_evenements: AtomicBool,
    /// Actions de la veille arrivées pendant le déchargement (sans
    /// doublons), rejouées au rechargement du snapshot. Verrou pris avant
    /// celui de l'index, des deux côtés : aucune action ne se glisse entre
    /// le rechargement et la reprise de la file.
    attente: Mutex<HashSet<watch::Action>>,
    /// Arrêt propre demandé (`Stop`).
    stop: AtomicBool,
    /// Racines entièrement sous veille inotify (les autres en rescan
    /// périodique).
    veilles: Mutex<HashSet<PathBuf>>,
    /// Sonnette de la maintenance : vrai si elle a été sonnée depuis sa
    /// dernière attente.
    sonnette: Mutex<bool>,
    reveil: Condvar,
}

impl Etat {
    fn new(config: IndexConfig) -> Self {
        let filtres = Arc::new(scan::Filtres::depuis_config(&config));
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
            attente: Mutex::new(HashSet::new()),
            stop: AtomicBool::new(false),
            veilles: Mutex::new(HashSet::new()),
            sonnette: Mutex::new(false),
            reveil: Condvar::new(),
        }
    }

    /// Réveille la maintenance (demande à traiter, échéance changée).
    fn reveiller(&self) {
        *self.sonnette.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.reveil.notify_one();
    }

    /// Attend d'être sonné, ou au plus `delai`.
    fn attendre(&self, delai: Duration) {
        let fin = Instant::now() + delai;
        let mut sonne = self.sonnette.lock().unwrap_or_else(|e| e.into_inner());
        while !*sonne {
            let reste = fin.saturating_duration_since(Instant::now());
            if reste.is_zero() {
                break;
            }
            sonne = self
                .reveil
                .wait_timeout(sonne, reste)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        *sonne = false;
    }

    /// Secondes entre deux rescans de sûreté.
    fn intervalle(&self) -> i64 {
        self.config.interval.max(MIN_INTERVAL_SECS) as i64
    }

    fn index_charge(&self) -> bool {
        self.index
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
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
                let attente = {
                    let mut file = etat.attente.lock().unwrap_or_else(|e| e.into_inner());
                    *etat.index.write().unwrap_or_else(|e| e.into_inner()) = Some(idx);
                    std::mem::take(&mut *file)
                };
                etat.dernier_scan.store(epoque(), Ordering::SeqCst);
                etat.derniere_sauvegarde.store(epoque(), Ordering::SeqCst);
                tracing::info!("{}", crate::tr!("index rechargé : {n} fichiers", n = n));
                // ce qui a changé pendant le déchargement
                appliquer_lot(etat, attente.into_iter().collect());
                // nouvelle échéance : le déchargement après inactivité
                etat.reveiller();
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
    {
        // l'index neuf couvre tout ce qui attendait (même ordre de verrous
        // que la veille : file, puis index)
        let mut file = etat.attente.lock().unwrap_or_else(|e| e.into_inner());
        *etat.index.write().unwrap_or_else(|e| e.into_inner()) = Some(idx);
        file.clear();
    }
    etat.scanning.store(false, Ordering::SeqCst);
    etat.dernier_scan.store(epoque(), Ordering::SeqCst);
    etat.rescan.store(false, Ordering::SeqCst);
    etat.perte_evenements.store(false, Ordering::SeqCst);
    // à sauvegarder ; si l'écriture échoue, la maintenance réessaiera
    etat.dirty.store(true, Ordering::SeqCst);
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

/// Sauvegarde l'index chargé ; vrai si le snapshot est à jour (écrit, ou
/// rien à écrire faute d'index).
fn sauvegarder(etat: &Arc<Etat>) -> bool {
    let g = etat.index.read().unwrap_or_else(|e| e.into_inner());
    let Some(idx) = g.as_ref() else { return true };
    sauvegarder_index(etat, idx)
}

/// Écrit `idx` (verrou tenu par l'appelant : la veille ne peut rien y
/// changer pendant l'écriture, `dirty` reste juste).
fn sauvegarder_index(etat: &Etat, idx: &Index) -> bool {
    // tentative datée même en échec : nouvel essai au délai suivant, pas
    // en boucle
    etat.derniere_sauvegarde.store(epoque(), Ordering::SeqCst);
    match store::save(idx) {
        Ok(()) => {
            etat.dirty.store(false, Ordering::SeqCst);
            true
        }
        Err(e) => {
            tracing::warn!("{}", crate::tr!("sauvegarde de l'index : {e}", e = e));
            false
        }
    }
}

// ——————————————————————— veille incrémentale ———————————————————————

fn boucle_veille(etat: &Arc<Etat>) {
    let perimetre = watch::Perimetre::new(etat.config.roots.clone(), etat.filtres.clone());
    let mut veilleur = match watch::Veilleur::surveille(perimetre) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "{}",
                crate::tr!("veille filesystem indisponible : {e}", e = e)
            );
            return;
        }
    };
    publier_veilles(etat, &veilleur);
    let n = veilleur.incompletes().len();
    if n > 0 {
        tracing::info!(
            "{}",
            crate::tr!(
                "{n} racine(s) sans veille inotify : rescan périodique",
                n = n
            )
        );
    }

    // attente bloquante : aucun réveil sans événement. Le canal ne se ferme
    // qu'avec le veilleur, c'est-à-dire jamais avant la sortie du processus
    // (Stop et les signaux passent par `process::exit`).
    while let Ok(premier) = veilleur.rx.recv() {
        let mut lot = Vec::new();
        let mut recu = Some(premier);
        while let Some(r) = recu {
            match r {
                // seuls les chemins indexables comptent : le reste n'agit
                // ni sur l'index, ni sur `dirty`, ni sur `perte_evenements`
                Ok(e) => lot.extend(veilleur.perimetre().filtrer(watch::actions(&e))),
                // erreur ponctuelle du veilleur : notée, le rescan
                // périodique couvrira
                Err(e) => tracing::warn!("{}", crate::tr!("veille filesystem : {e}", e = e)),
            }
            recu = veilleur.rx.try_recv().ok();
        }
        if lot.is_empty() {
            continue;
        }
        let lot = watch::dedoublonner(lot);
        // watches d'abord (dossiers apparus, disparus), sous-scan ensuite
        if veilleur.entretenir(&lot) {
            publier_veilles(etat, &veilleur);
        }
        appliquer_lot(etat, lot);
    }
}

/// Publie dans l'état (statut) les racines entièrement veillées.
fn publier_veilles(etat: &Etat, veilleur: &watch::Veilleur) {
    let veillees: HashSet<PathBuf> = etat
        .config
        .roots
        .iter()
        .filter(|r| !veilleur.incompletes().contains(*r))
        .cloned()
        .collect();
    *etat.veilles.lock().unwrap_or_else(|e| e.into_inner()) = veillees;
}

/// Index déchargé (inactivité) : garde les actions pour le rechargement
/// plutôt que de condamner le snapshot à un rescan complet. Renvoie vrai si
/// les actions sont prises en charge ici (file d'attente ou perte).
fn mettre_en_attente(etat: &Etat, actions: &[watch::Action]) -> bool {
    let mut file = etat.attente.lock().unwrap_or_else(|e| e.into_inner());
    if etat.index_charge() {
        return false;
    }
    if !etat.perte_evenements.load(Ordering::SeqCst) {
        file.extend(actions.iter().cloned());
        if file.len() > MAX_EN_ATTENTE {
            // trop de changements : le rescan les couvrira tous
            file.clear();
            etat.perte_evenements.store(true, Ordering::SeqCst);
        }
    }
    true
}

fn appliquer_lot(etat: &Arc<Etat>, actions: Vec<watch::Action>) {
    if actions.is_empty() {
        return;
    }
    if actions.iter().any(|a| matches!(a, watch::Action::Rescan)) {
        // le noyau a jeté des événements : des changements ont pu échapper
        etat.perte_evenements.store(true, Ordering::SeqCst);
        etat.rescan.store(true, Ordering::SeqCst);
        etat.reveiller();
        return;
    }
    if mettre_en_attente(etat, &actions) {
        return;
    }

    // hors verrou : stat des chemins et scans de sous-arbres (peut prendre
    // quelques secondes sur un gros déplacement) ; les requêtes continuent
    // d'être servies entre-temps
    let mut maj: Vec<(String, EntryMeta)> = Vec::new();
    let mut supprime: Vec<PathBuf> = Vec::new();
    for a in actions {
        match a {
            watch::Action::Maj(p) => match meta_de(&etat.config, &p) {
                // devenu trop gros : sort de l'index, comme au scan
                Some((_, meta)) if etat.filtres.trop_gros(meta.kind, meta.size) => supprime.push(p),
                Some(entree) => maj.push(entree),
                None => {}
            },
            watch::Action::Supprime(p) => supprime.push(p),
            watch::Action::SousArbre(p) => {
                if let Some(vol) = vol_de(&etat.config, &p) {
                    let (ents, _) = scan::scan_volume(&p, &etat.filtres, vol, None);
                    maj.extend(ents);
                }
            }
            watch::Action::Rescan => unreachable!("traité au-dessus"),
        }
    }

    let mut g = etat.index.write().unwrap_or_else(|e| e.into_inner());
    let Some(idx) = g.as_mut() else {
        // déchargé entre-temps
        etat.perte_evenements.store(true, Ordering::SeqCst);
        return;
    };
    let mut change = false;
    // suppressions d'abord : un chemin supprimé puis recréé dans le même
    // lot a été stat-isé après coup, son ajout doit passer en dernier
    for p in supprime {
        let p = p.to_string_lossy();
        // fichier : une recherche ; dossier : ses descendants avec lui
        let n = idx.remove(&p);
        if n > 1 {
            tracing::debug!(
                "{}",
                crate::tr!("{n} entrées purgées sous {p0}", n = n - 1, p0 = p)
            );
        }
        change |= n > 0;
    }
    for (path, meta) in maj {
        idx.push(&path, meta);
        change = true;
    }
    if change {
        idx.compact();
        // sous le verrou : une sauvegarde concurrente ne peut pas effacer
        // ce `dirty` sans avoir écrit ces changements
        if !etat.dirty.swap(true, Ordering::SeqCst) {
            etat.reveiller(); // nouvelle échéance : la sauvegarde
        }
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
    while !etat.stop.load(Ordering::SeqCst) {
        let maintenant = epoque();

        // rescan demandé ou requis (premier lancement sans snapshot)
        if etat.rescan.load(Ordering::SeqCst) {
            scan_complet(etat);
            continue;
        }
        // rescan périodique de sûreté (inotify a pu rater des choses)
        let dernier = etat.dernier_scan.load(Ordering::SeqCst);
        if dernier > 0 && maintenant - dernier >= etat.intervalle() {
            tracing::info!("{}", crate::tr!("rescan périodique de l'index"));
            scan_complet(etat);
            continue;
        }
        // sauvegarde débordée des changements incrémentaux
        if etat.dirty.load(Ordering::SeqCst)
            && maintenant - etat.derniere_sauvegarde.load(Ordering::SeqCst) >= SAVE_DEBOUNCE_SECS
        {
            sauvegarder(etat);
        }
        // déchargement après inactivité : la RAM redevient libre, la
        // requête suivante rechargera le snapshot en moins de deux secondes
        if maintenant - etat.dernier_usage.load(Ordering::SeqCst) >= IDLE_UNLOAD_SECS {
            decharger(etat);
        }

        etat.attendre(prochaine_echeance(etat, epoque()));
    }
    // arrêt propre (Stop) : dernière sauvegarde
    sauvegarder(etat);
    std::process::exit(0);
}

/// Décharge l'index, sauvegardé d'abord s'il a changé. Tout se passe sous
/// le verrou d'écriture : aucun lot de la veille ne peut se glisser entre
/// l'écriture du snapshot et le déchargement.
fn decharger(etat: &Etat) {
    let mut g = etat.index.write().unwrap_or_else(|e| e.into_inner());
    let Some(idx) = g.as_ref() else { return };
    let a_jour = !etat.dirty.load(Ordering::SeqCst) || sauvegarder_index(etat, idx);
    *g = None;
    drop(g);
    if !a_jour {
        // snapshot périmé : le prochain chargement repartira d'un rescan
        etat.perte_evenements.store(true, Ordering::SeqCst);
    }
    etat.dirty.store(false, Ordering::SeqCst);
    tracing::info!(
        "{}",
        crate::tr!("index déchargé (inactivité) — rechargement à la prochaine requête")
    );
}

/// Délai jusqu'à la prochaine échéance réelle : rescan périodique,
/// sauvegarde de changements en attente, déchargement d'un index chargé.
/// Plafonné à [`MAX_ATTENTE_SECS`] (l'attente suit l'horloge monotone,
/// arrêtée pendant une mise en veille de la machine).
fn prochaine_echeance(etat: &Etat, maintenant: i64) -> Duration {
    let mut echeance = maintenant + MAX_ATTENTE_SECS as i64;
    let dernier = etat.dernier_scan.load(Ordering::SeqCst);
    if dernier > 0 {
        echeance = echeance.min(dernier + etat.intervalle());
    }
    if etat.index_charge() {
        if etat.dirty.load(Ordering::SeqCst) {
            echeance =
                echeance.min(etat.derniere_sauvegarde.load(Ordering::SeqCst) + SAVE_DEBOUNCE_SECS);
        }
        echeance = echeance.min(etat.dernier_usage.load(Ordering::SeqCst) + IDLE_UNLOAD_SECS);
    }
    Duration::from_secs(echeance.saturating_sub(maintenant).max(0) as u64)
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
            if !etat.index_charge()
                && (etat.perte_evenements.load(Ordering::Relaxed) || !charger_snapshot(etat))
            {
                etat.rescan.store(true, Ordering::SeqCst);
                etat.reveiller();
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
            etat.reveiller();
            Reply::Ok
        }
        Request::Stop => {
            etat.stop.store(true, Ordering::SeqCst);
            etat.reveiller();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn etat_test() -> Etat {
        Etat::new(IndexConfig {
            roots: vec![PathBuf::from("/tmp/hyprdesk-demon-test")],
            ..Default::default()
        })
    }

    fn maj(i: usize) -> watch::Action {
        watch::Action::Maj(PathBuf::from(format!("/tmp/hyprdesk-demon-test/f{i}")))
    }

    #[test]
    fn dechargement_met_en_attente_sans_perte() {
        let etat = etat_test();
        // index déchargé : gardé pour le rechargement, pas de rescan
        assert!(mettre_en_attente(&etat, &[maj(1), maj(2), maj(1)]));
        assert_eq!(etat.attente.lock().unwrap().len(), 2);
        assert!(!etat.perte_evenements.load(Ordering::SeqCst));
        // index chargé : rien à mettre de côté
        *etat.index.write().unwrap() = Some(Index::new());
        assert!(!mettre_en_attente(&etat, &[maj(3)]));
        assert_eq!(etat.attente.lock().unwrap().len(), 2);
    }

    #[test]
    fn file_d_attente_pleine_reclame_un_rescan() {
        let etat = etat_test();
        let lot: Vec<watch::Action> = (0..=MAX_EN_ATTENTE).map(maj).collect();
        assert!(mettre_en_attente(&etat, &lot));
        assert!(etat.perte_evenements.load(Ordering::SeqCst));
        assert!(etat.attente.lock().unwrap().is_empty());
    }

    #[test]
    fn echeances_sans_reveil_inutile() {
        let etat = etat_test();
        let t = epoque();
        // rien de prévu (ni scan fini, ni index chargé) : attente plafonnée,
        // même avec `dirty` (rien à écrire sans index)
        etat.dirty.store(true, Ordering::SeqCst);
        assert_eq!(
            prochaine_echeance(&etat, t),
            Duration::from_secs(MAX_ATTENTE_SECS)
        );
        // index chargé, changements en attente : la sauvegarde d'abord
        *etat.index.write().unwrap() = Some(Index::new());
        etat.derniere_sauvegarde.store(t, Ordering::SeqCst);
        etat.dernier_usage.store(t, Ordering::SeqCst);
        assert_eq!(
            prochaine_echeance(&etat, t),
            Duration::from_secs(SAVE_DEBOUNCE_SECS as u64)
        );
        // propre : le déchargement après inactivité
        etat.dirty.store(false, Ordering::SeqCst);
        assert_eq!(
            prochaine_echeance(&etat, t),
            Duration::from_secs(IDLE_UNLOAD_SECS as u64)
        );
        // échéance dépassée : tout de suite
        etat.dernier_usage
            .store(t - 2 * IDLE_UNLOAD_SECS, Ordering::SeqCst);
        assert_eq!(prochaine_echeance(&etat, t), Duration::ZERO);
    }

    #[test]
    fn intervalle_nul_borne() {
        let etat = Etat::new(IndexConfig {
            roots: vec![],
            interval: 0,
            ..Default::default()
        });
        assert_eq!(etat.intervalle(), MIN_INTERVAL_SECS as i64);
    }

    #[test]
    fn sonnette_reveille_l_attente() {
        let etat = Arc::new(etat_test());
        let e2 = etat.clone();
        let t0 = Instant::now();
        let fil = std::thread::spawn(move || e2.attendre(Duration::from_secs(30)));
        std::thread::sleep(Duration::from_millis(50));
        etat.reveiller();
        fil.join().expect("fil");
        assert!(t0.elapsed() < Duration::from_secs(10));
        // sonnée avant l'attente : rendue tout de suite
        etat.reveiller();
        let t1 = Instant::now();
        etat.attendre(Duration::from_secs(30));
        assert!(t1.elapsed() < Duration::from_secs(10));
    }
}
