//! hyprdesk, racine de session : il lance et supervise des composants
//! (`hd.component`) — démarrage avec le démon, relance après un plantage
//! (délai croissant), arrêt avec le démon. Les composants sont ses enfants.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::watch;

use crate::config::ComponentDef;

/// État d'un composant, lu par l'écran Réglages.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    pub pid: Option<u32>,
    pub restarts: u32,
    pub last_exit: Option<String>,
    /// Supervision abandonnée (trop de plantages rapprochés).
    pub given_up: bool,
}

struct Handle {
    task: tokio::task::JoinHandle<()>,
    def: ComponentDef,
    stop: watch::Sender<bool>,
    status: Arc<Mutex<Status>>,
}

static COMPONENTS: LazyLock<Mutex<HashMap<String, Handle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static OPERATIONS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Plus de `MAX_FAILS` arrêts en moins de `FAIL_WINDOW` : on abandonne.
const MAX_FAILS: usize = 5;
const FAIL_WINDOW: Duration = Duration::from_secs(60);

fn backoff(n: u32) -> Duration {
    Duration::from_secs([1, 2, 5, 10, 30][(n as usize).min(4)])
}

fn signal_group(pid: u32, signal: &str) {
    let _ = std::process::Command::new("kill")
        .args([signal, "--", &format!("-{pid}")])
        .stderr(std::process::Stdio::null())
        .status();
}

fn terminate(pid: u32) {
    signal_group(pid, "-TERM");
}

async fn supervise(def: ComponentDef, mut stop: watch::Receiver<bool>, status: Arc<Mutex<Status>>) {
    let mut fails: Vec<Instant> = Vec::new();
    loop {
        if *stop.borrow() {
            return;
        }
        // `exec` : le shell laisse la place au programme, qu'on arrête directement.
        let child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(format!("exec {}", def.exec))
            .kill_on_drop(true)
            .process_group(0)
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("composant {} : {e}", def.name);
                if let Ok(mut s) = status.lock() {
                    s.last_exit = Some(e.to_string());
                    s.given_up = true;
                }
                return;
            }
        };
        let pid = child.id();
        tracing::info!("composant {} lancé (pid {})", def.name, pid.unwrap_or(0));
        if let Ok(mut s) = status.lock() {
            s.pid = pid;
        }
        tokio::select! {
            r = child.wait() => {
                let why = match r {
                    Ok(st) => st.to_string(),
                    Err(e) => e.to_string(),
                };
                tracing::warn!("composant {} arrêté : {why}", def.name);
                if let Ok(mut s) = status.lock() {
                    s.pid = None;
                    s.last_exit = Some(why);
                }
            }
            _ = stop.changed() => {
                if let Some(p) = pid {
                    terminate(p);
                }
                if tokio::time::timeout(Duration::from_secs(3), child.wait()).await.is_err() {
                    let _ = child.kill().await;
                }
                if let Ok(mut s) = status.lock() {
                    s.pid = None;
                }
                if let Some(p) = pid { signal_group(p,"-KILL"); }
                tracing::info!("composant {} arrêté", def.name);
                return;
            }
        }
        if let Some(p) = pid {
            signal_group(p, "-KILL");
        }
        if !def.restart || *stop.borrow() {
            return;
        }
        let now = Instant::now();
        fails.retain(|t| now.duration_since(*t) < FAIL_WINDOW);
        fails.push(now);
        if fails.len() > MAX_FAILS {
            tracing::error!(
                "composant {} : trop de plantages, supervision abandonnée",
                def.name
            );
            crate::hypr::notify(
                4,
                10000,
                &format!(
                    "hyprdesk : le composant « {} » plante en boucle, il n'est plus relancé",
                    def.name
                ),
            );
            if let Ok(mut s) = status.lock() {
                s.given_up = true;
            }
            return;
        }
        let n = status.lock().map(|mut s| {
            s.restarts += 1;
            s.restarts
        });
        tokio::select! {
            _ = tokio::time::sleep(backoff(n.unwrap_or(1).saturating_sub(1))) => {}
            _ = stop.changed() => return,
        }
    }
}

/// Aligne les composants lancés sur la config : arrête les retirés ou
/// modifiés, lance les nouveaux. À appeler dans le runtime tokio.
pub async fn sync(defs: &[ComponentDef]) {
    let _operation = OPERATIONS.lock().await;
    let stale = {
        let mut map = COMPONENTS.lock().unwrap_or_else(|e| e.into_inner());
        let names: Vec<_> = map
            .iter()
            .filter(|(n, h)| !defs.iter().any(|d| &d.name == *n && *d == h.def))
            .map(|(n, _)| n.clone())
            .collect();
        names
            .into_iter()
            .filter_map(|n| map.remove(&n))
            .collect::<Vec<_>>()
    };
    for h in &stale {
        let _ = h.stop.send(true);
    }
    for h in stale {
        let _ = h.task.await;
    }
    for d in defs {
        sync_one(d.clone());
    }
}

/// Le remplaçant démarre seulement après l'arrêt confirmé de son prédécesseur.
pub async fn restart(name: &str) {
    let _operation = OPERATIONS.lock().await;
    let handle = COMPONENTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(name);
    if let Some(h) = handle {
        let _ = h.stop.send(true);
        let _ = h.task.await;
        sync_one(h.def);
    }
}

fn sync_one(def: ComponentDef) {
    let mut map = COMPONENTS.lock().unwrap_or_else(|e| e.into_inner());
    if map.contains_key(&def.name) {
        return;
    }
    let (tx, rx) = watch::channel(false);
    let status = Arc::new(Mutex::new(Status::default()));
    let task = tokio::spawn(supervise(def.clone(), rx, status.clone()));
    map.insert(
        def.name.clone(),
        Handle {
            task,
            def,
            stop: tx,
            status,
        },
    );
}

pub async fn stop_all() {
    let _operation = OPERATIONS.lock().await;
    let handles: Vec<_> = COMPONENTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .drain()
        .map(|(_, h)| h)
        .collect();
    for h in &handles {
        let _ = h.stop.send(true);
    }
    for h in handles {
        let _ = h.task.await;
    }
}

/// État de tous les composants, triés par nom.
pub fn snapshot() -> Vec<(ComponentDef, Status)> {
    let Ok(map) = COMPONENTS.lock() else {
        return Vec::new();
    };
    let mut v: Vec<_> = map
        .values()
        .map(|h| {
            (
                h.def.clone(),
                h.status.lock().map(|s| s.clone()).unwrap_or_default(),
            )
        })
        .collect();
    v.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn supervision_relance_et_arret() {
        let defs = vec![ComponentDef {
            name: "test-sleep".into(),
            exec: "sleep 30".into(),
            restart: true,
        }];
        sync(&defs).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        let snap = snapshot();
        let pid = snap
            .iter()
            .find(|(d, _)| d.name == "test-sleep")
            .and_then(|(_, s)| s.pid)
            .expect("lancé");
        // Tué de l'extérieur : relancé avec un autre PID.
        terminate(pid);
        tokio::time::sleep(Duration::from_millis(1600)).await;
        let snap = snapshot();
        let (_, st) = snap.iter().find(|(d, _)| d.name == "test-sleep").unwrap();
        assert_eq!(st.restarts, 1);
        assert!(st.pid.is_some_and(|p| p != pid));
        // Retiré de la config : arrêté.
        sync(&[]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(snapshot().is_empty());
        // Un composant qui ignore TERM ne doit jamais chevaucher son remplaçant.
        let slow = ComponentDef {
            name: "slow-stop".into(),
            exec: r#"sh -c 'trap "" TERM; exec sleep 30'"#.into(),
            restart: false,
        };
        sync(&[slow]).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let old = snapshot()[0].1.pid.unwrap();
        restart("slow-stop").await;
        assert!(!std::path::Path::new(&format!("/proc/{old}")).exists());
        stop_all().await;
        assert!(snapshot().is_empty());
    }
}
