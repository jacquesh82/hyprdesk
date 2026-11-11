//! Commandes bornées : délai, mémoire et terminaison de tout le groupe enfant.
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Mémoire gardée par flux : les 64 derniers Kio (une sortie plus longue
/// tourne, la commande n'est jamais interrompue pour sa taille).
pub const KEEP: usize = 64 * 1024;

/// Fin d'un flux : un tampon tournant de `cap` octets.
#[derive(Debug, Default)]
pub struct Tail {
    buf: std::collections::VecDeque<u8>,
    cap: usize,
    /// Octets oubliés au début.
    pub dropped: u64,
}

impl Tail {
    pub fn new(cap: usize) -> Self {
        Tail {
            buf: std::collections::VecDeque::with_capacity(cap.min(8192)),
            cap,
            dropped: 0,
        }
    }

    pub fn push(&mut self, bytes: &[u8]) {
        let bytes = if bytes.len() > self.cap {
            self.dropped += (bytes.len() - self.cap) as u64;
            &bytes[bytes.len() - self.cap..]
        } else {
            bytes
        };
        let over = (self.buf.len() + bytes.len()).saturating_sub(self.cap);
        if over > 0 {
            self.buf.drain(..over);
            self.dropped += over as u64;
        }
        self.buf.extend(bytes);
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buf.into()
    }
}

/// Journal à taille bornée, à la façon de logrotate : au-delà de `max`
/// octets, `f.log` devient `f.log.1` (`.1` → `.2`…), `keep` anciens gardés.
pub struct RotatingLog {
    path: PathBuf,
    max: u64,
    keep: u32,
    file: Option<std::fs::File>,
    size: u64,
}

impl RotatingLog {
    pub fn open(path: &Path, max: u64, keep: u32) -> Self {
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok();
        let size = file
            .as_ref()
            .and_then(|f| f.metadata().ok())
            .map(|m| m.len())
            .unwrap_or(0);
        RotatingLog {
            path: path.to_path_buf(),
            max: max.max(1),
            keep,
            file,
            size,
        }
    }

    fn numbered(&self, n: u32) -> PathBuf {
        let mut s = self.path.clone().into_os_string();
        s.push(format!(".{n}"));
        PathBuf::from(s)
    }

    fn rotate(&mut self) {
        self.file = None;
        if self.keep == 0 {
            let _ = std::fs::remove_file(&self.path);
        } else {
            let _ = std::fs::remove_file(self.numbered(self.keep));
            for n in (1..self.keep).rev() {
                let _ = std::fs::rename(self.numbered(n), self.numbered(n + 1));
            }
            let _ = std::fs::rename(&self.path, self.numbered(1));
        }
        self.file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .ok();
        self.size = 0;
    }

    pub fn write(&mut self, bytes: &[u8]) {
        use std::io::Write;
        if self.size > 0 && self.size + bytes.len() as u64 > self.max {
            self.rotate();
        }
        if let Some(f) = self.file.as_mut()
            && f.write_all(bytes).is_ok()
        {
            self.size += bytes.len() as u64;
        }
    }
}

type Shared = std::sync::Arc<std::sync::Mutex<Option<RotatingLog>>>;

async fn read_tail(mut reader: impl AsyncRead + Unpin, log: Shared) -> io::Result<Vec<u8>> {
    let mut tail = Tail::new(KEEP);
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        tail.push(&chunk[..n]);
        if let Some(l) = log.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            l.write(&chunk[..n]);
        }
    }
    Ok(tail.into_vec())
}

/// Commande bornée en temps ; sorties : leurs 64 derniers Kio.
pub async fn output(
    command: &mut tokio::process::Command,
    timeout: Duration,
) -> io::Result<Output> {
    run(command, timeout, None).await
}

/// Comme `output`, et toute la sortie (standard et d'erreur) va aussi dans
/// un journal tournant (`log`).
pub async fn output_logged(
    command: &mut tokio::process::Command,
    timeout: Duration,
    log: RotatingLog,
) -> io::Result<Output> {
    run(command, timeout, Some(log)).await
}

async fn run(
    command: &mut tokio::process::Command,
    timeout: Duration,
    log: Option<RotatingLog>,
) -> io::Result<Output> {
    let log: Shared = std::sync::Arc::new(std::sync::Mutex::new(log));
    let mut child = command
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let pid = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("stdout indisponible"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("stderr indisponible"))?;
    let result = tokio::time::timeout(timeout, async {
        let (stdout, stderr, status) = tokio::try_join!(
            read_tail(stdout, log.clone()),
            read_tail(stderr, log.clone()),
            child.wait()
        )?;
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    })
    .await
    .unwrap_or_else(|_| {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            crate::i18n::tr("délai de commande dépassé"),
        ))
    });
    if result.is_err() {
        if let Some(pid) = pid {
            let _ = tokio::process::Command::new("kill")
                .args(["-KILL", "--", &format!("-{pid}")])
                .stderr(Stdio::null())
                .status()
                .await;
        }
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn limits_cover_pipes_and_runaway_output() {
        let start = std::time::Instant::now();
        let r = output(
            tokio::process::Command::new("sh").args(["-c", "sleep 30 | cat"]),
            Duration::from_millis(100),
        )
        .await;
        assert_eq!(r.unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(start.elapsed() < Duration::from_secs(2));
        // Sortie sans fin : le délai l'arrête, la mémoire reste bornée.
        let r = output(
            tokio::process::Command::new("sh").args(["-c", "yes"]),
            Duration::from_millis(300),
        )
        .await;
        assert_eq!(r.unwrap_err().kind(), io::ErrorKind::TimedOut);
    }

    /// Sortie longue : la commande va au bout, on en garde la fin.
    #[tokio::test]
    async fn long_output_keeps_its_end() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-proc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let log = RotatingLog::open(&dir.join("t.log"), 100_000, 2);
        let o = output_logged(
            tokio::process::Command::new("sh")
                .args(["-c", "head -c 200000 /dev/zero | tr '\\0' a; echo FIN"]),
            Duration::from_secs(10),
            log,
        )
        .await
        .unwrap();
        assert!(o.status.success());
        assert_eq!(o.stdout.len(), KEEP);
        assert!(o.stdout.ends_with(b"aFIN\n"));
        // Journal : 200 004 octets en fichiers de 100 000 au plus, 2 anciens gardés.
        let size = |n: &str| std::fs::metadata(dir.join(n)).map(|m| m.len()).unwrap_or(0);
        assert!(size("t.log") <= 100_000 && size("t.log.1") <= 100_000);
        assert!(size("t.log") + size("t.log.1") + size("t.log.2") >= 200_004);
        assert_eq!(size("t.log.3"), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tampon_tournant() {
        let mut t = Tail::new(4);
        t.push(b"ab");
        t.push(b"cdef");
        assert_eq!(t.dropped, 2);
        t.push(b"0123456789");
        assert_eq!(t.into_vec(), b"6789");
    }
}
