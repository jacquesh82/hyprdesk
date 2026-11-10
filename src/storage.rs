//! Écritures atomiques et ordonnées des fichiers d'état.
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

static NEXT: AtomicU64 = AtomicU64::new(1);
static WRITES: LazyLock<Mutex<HashMap<PathBuf, u64>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn revision() -> u64 {
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub fn write(path: &Path, contents: &[u8], revision: u64) -> io::Result<()> {
    let mut writes = WRITES.lock().map_err(|_| io::Error::other("verrou d'enregistrement indisponible"))?;
    if writes.get(path).is_some_and(|last| *last > revision) {
        return Ok(());
    }
    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".hyprdesk-{}-{revision}.tmp", std::process::id()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        std::fs::File::open(parent)?.sync_all()?;
        writes.insert(path.to_path_buf(), revision);
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_writes_keep_the_latest_requested_value() {
        let path = std::env::temp_dir().join(format!("hyprdesk-storage-{}.lua", revision()));
        let revisions: Vec<_> = (0..32).map(|_| revision()).collect();
        let last = *revisions.last().unwrap();
        let threads: Vec<_> = revisions.into_iter().rev().map(|r| {
            let path = path.clone();
            std::thread::spawn(move || write(&path, r.to_string().as_bytes(), r).unwrap())
        }).collect();
        for t in threads { t.join().unwrap(); }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), last.to_string());
        std::fs::remove_file(path).unwrap();
    }
}
