//! Commandes bornées : délai, mémoire et terminaison de tout le groupe enfant.
use std::io;
use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

async fn read_limited(reader: impl AsyncRead + Unpin) -> io::Result<Vec<u8>> {
    const LIMIT: u64 = 64 * 1024;
    let mut bytes = Vec::new();
    reader.take(LIMIT + 1).read_to_end(&mut bytes).await?;
    if bytes.len() > LIMIT as usize {
        return Err(io::Error::other("sortie trop longue (64 Kio maximum)"));
    }
    Ok(bytes)
}

pub async fn output(
    command: &mut tokio::process::Command,
    timeout: Duration,
) -> io::Result<Output> {
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
        let (stdout, stderr, status) =
            tokio::try_join!(read_limited(stdout), read_limited(stderr), child.wait())?;
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
            "délai de commande dépassé",
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
        let r = output(
            tokio::process::Command::new("sh").args(["-c", "yes"]),
            Duration::from_secs(2),
        )
        .await;
        assert!(r.unwrap_err().to_string().contains("64 Kio"));
    }
}
