//! Une seule instance de hyprdesk par session : un verrou `flock` exclusif
//! sur `$XDG_RUNTIME_DIR/hypr/<instance>/hyprdesk.lock`, tenu toute la vie du processus.
//!
//! Deux démons se disputeraient la socket de commande, les surfaces du bureau,
//! la submap et les fenêtres mises de côté. Le verrou est posé par le noyau :
//! pas de course au démarrage (service systemd et lancement manuel simultanés),
//! et il disparaît avec le processus, même tué brutalement.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Verrou tenu : le garder vivant aussi longtemps que le démon.
pub struct Guard {
    _file: File,
}

pub fn lock_path() -> PathBuf {
    hyprpanel_core::ipc::instance_dir()
        .or_else(|_| hyprpanel_core::ipc::runtime_dir())
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("hyprdesk.lock")
}

/// PID inscrit dans le verrou par l'instance qui le tient.
pub fn holder_pid(path: &Path) -> Option<u32> {
    let mut s = String::new();
    File::open(path).ok()?.read_to_string(&mut s).ok()?;
    s.trim().parse().ok()
}

/// Prend le verrou ; `Err(Some(pid))` si une autre instance le tient.
pub fn acquire(path: &Path) -> Result<Guard, Option<u32>> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| {
            tracing::error!(
                "{}",
                crate::tr!("verrou {p0} : {e}", p0 = path.display(), e = e)
            );
            None
        })?;
    match file.try_lock() {
        Ok(()) => {
            // Le PID sert aux messages (« déjà lancé, pid … ») et à `status`.
            let _ = file.set_len(0);
            let _ = file.seek(SeekFrom::Start(0));
            let _ = write!(file, "{}", std::process::id());
            let _ = file.flush();
            Ok(Guard { _file: file })
        }
        Err(TryLockError::WouldBlock) => Err(holder_pid(path)),
        Err(TryLockError::Error(e)) => {
            tracing::error!(
                "{}",
                crate::tr!("verrou {p0} : {e}", p0 = path.display(), e = e)
            );
            Err(None)
        }
    }
}

/// Attend que le verrou se libère (instance précédente en train de quitter).
pub fn acquire_within(path: &Path, timeout: Duration) -> Result<Guard, Option<u32>> {
    let start = Instant::now();
    loop {
        match acquire(path) {
            Ok(g) => return Ok(g),
            Err(pid) if start.elapsed() >= timeout => return Err(pid),
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_seule_instance() {
        let p = std::env::temp_dir().join(format!("hyprdesk-lock-test-{}", std::process::id()));
        let g = acquire(&p).expect("premier verrou");
        assert_eq!(holder_pid(&p), Some(std::process::id()));
        // Une seconde prise (autre description de fichier) échoue et donne le PID du détenteur.
        assert_eq!(acquire(&p).err(), Some(Some(std::process::id())));
        drop(g);
        // Libéré avec le détenteur : on le reprend.
        assert!(acquire_within(&p, Duration::from_secs(1)).is_ok());
        let _ = std::fs::remove_file(&p);
    }
}
