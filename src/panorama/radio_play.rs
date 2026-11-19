//! Lecture des webradios sans fenêtre (MLG-172) : mpv joue le flux, le
//! Panorama montre la barre « flux vocal » à la place d'une page de
//! navigateur. mpv se pilote par sa socket (pause, arrêt) ; le rythme se lit
//! sur la sortie son (moniteur PipeWire), en niveaux par bande.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// Lecteur en cours.
static PLAYER: Mutex<Option<Child>> = Mutex::new(None);

fn socket() -> PathBuf {
    crate::policy::runtime_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("hyprdesk-radio.sock")
}

/// L'adresse est un flux audio (pas une page) : extension ou serveur de
/// diffusion connus.
pub fn is_stream(url: &str) -> bool {
    let u = url.to_lowercase();
    let path = u.split(['?', '#']).next().unwrap_or(&u);
    [".mp3", ".aac", ".ogg", ".opus", ".m3u", ".m3u8", ".pls", ".flac"]
        .iter()
        .any(|e| path.ends_with(e))
        || ["icecast", "shoutcast", "/stream", "/live", "radioho.st", "infomaniak.ch"]
            .iter()
            .any(|k| u.contains(k))
}

/// Joue le flux (arrête le précédent). Bloquant le temps du lancement.
pub fn play(url: &str) -> Result<(), String> {
    stop();
    let sock = socket();
    let _ = std::fs::remove_file(&sock);
    let child = Command::new("mpv")
        .args([
            "--no-video",
            "--no-terminal",
            "--really-quiet",
            "--cache=yes",
            "--audio-client-name=hyprdesk-radio",
        ])
        .arg(format!("--input-ipc-server={}", sock.display()))
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("mpv : {e}"))?;
    if let Ok(mut p) = PLAYER.lock() {
        *p = Some(child);
    }
    Ok(())
}

/// Radio en cours, gardée le temps de la session : un redémarrage de
/// hyprdesk (mise à jour) la reprend.
fn resume_file() -> PathBuf {
    socket().with_file_name("hyprdesk-radio.json")
}

/// Retient la radio en cours (nom, flux), ou l'oublie (`None`).
pub fn remember(radio: Option<(&str, &str)>) {
    match radio {
        Some((name, url)) => {
            let _ = std::fs::write(
                resume_file(),
                serde_json::json!({ "name": name, "url": url }).to_string(),
            );
        }
        None => {
            let _ = std::fs::remove_file(resume_file());
        }
    }
}

/// Radio à reprendre au démarrage : (nom, flux).
pub fn to_resume() -> Option<(String, String)> {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(resume_file()).ok()?).ok()?;
    Some((v["name"].as_str()?.to_string(), v["url"].as_str()?.to_string()))
}

/// Arrête la radio en cours.
pub fn stop() {
    if let Ok(mut p) = PLAYER.lock()
        && let Some(mut c) = p.take()
    {
        let _ = c.kill();
        let _ = c.wait();
    }
}

/// Une radio joue (le lecteur est vivant).
pub fn playing() -> bool {
    PLAYER
        .lock()
        .ok()
        .and_then(|mut p| p.as_mut().map(|c| matches!(c.try_wait(), Ok(None))))
        .unwrap_or(false)
}

fn send(cmd: &str) -> Option<String> {
    let mut s = std::os::unix::net::UnixStream::connect(socket()).ok()?;
    s.set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .ok()?;
    writeln!(s, "{cmd}").ok()?;
    let mut line = String::new();
    std::io::BufReader::new(s).read_line(&mut line).ok()?;
    Some(line)
}

/// Lecture / pause ; rend l'état après coup (`true` : en pause).
pub fn toggle_pause() -> Option<bool> {
    send(r#"{"command":["cycle","pause"]}"#)?;
    let r = send(r#"{"command":["get_property","pause"]}"#)?;
    serde_json::from_str::<serde_json::Value>(&r).ok()?["data"].as_bool()
}

/// Niveaux par image (environ 30 par seconde) : global, graves, médiums,
/// aigus, de 0 à 1.
pub type Levels = [f32; 4];

/// Niveaux de ce qui sort des haut-parleurs (moniteur de la sortie par
/// défaut), pour la barre « flux vocal ».
pub fn levels() -> impl iced::futures::Stream<Item = Levels> {
    use iced::futures::SinkExt;
    iced::stream::channel(4, async move |mut tx| {
        let child = Command::new("pw-record")
            .args([
                "--raw",
                "--rate",
                "16000",
                "--channels",
                "1",
                "--format",
                "s16",
                "-P",
                "{ stream.capture.sink = true }",
                "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = child else {
            return std::future::pending().await;
        };
        let out = child.stdout.take();
        struct Kill(Child);
        impl Drop for Kill {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let _guard = Kill(child);
        let (ltx, mut lrx) = tokio::sync::mpsc::unbounded_channel::<Levels>();
        if let Some(mut out) = out {
            std::thread::spawn(move || {
                use std::io::Read;
                // 1/30 s à 16 kHz : 533 échantillons.
                let mut buf = vec![0u8; 1066];
                let mut f = Bands::default();
                while out.read_exact(&mut buf).is_ok() {
                    let l = f.feed(&buf);
                    if ltx.send(l).is_err() {
                        break;
                    }
                }
            });
        }
        while let Some(l) = lrx.recv().await {
            let _ = tx.send(l).await;
        }
        std::future::pending().await
    })
}

/// Séparation grossière en bandes (filtres du premier ordre) et niveaux
/// lissés (montée rapide, descente lente).
#[derive(Default)]
struct Bands {
    lp: f32,
    lp2: f32,
    out: Levels,
    peak: f32,
}

impl Bands {
    fn feed(&mut self, raw: &[u8]) -> Levels {
        let (mut all, mut low, mut mid, mut high) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        let n = (raw.len() / 2).max(1) as f32;
        for c in raw.chunks_exact(2) {
            let x = i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0;
            // Passe-bas ~200 Hz et ~2 kHz à 16 kHz.
            self.lp += 0.075 * (x - self.lp);
            self.lp2 += 0.55 * (x - self.lp2);
            let (l, m, h) = (self.lp, self.lp2 - self.lp, x - self.lp2);
            all += x * x;
            low += l * l;
            mid += m * m;
            high += h * h;
        }
        let rms = |v: f32| (v / n).sqrt();
        let raw_levels = [rms(all), rms(low) * 1.6, rms(mid) * 2.2, rms(high) * 4.0];
        // Normalisation douce : le plus fort récent vaut 1.
        let top = raw_levels.iter().copied().fold(0.0f32, f32::max);
        self.peak = (self.peak * 0.995).max(top).max(0.02);
        for (o, v) in self.out.iter_mut().zip(raw_levels) {
            let v = (v / self.peak).clamp(0.0, 1.0);
            *o = if v > *o { *o + (v - *o) * 0.6 } else { *o + (v - *o) * 0.12 };
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flux_et_niveaux() {
        assert!(is_stream("https://icecast.radiofrance.fr/fip-hifi.aac"));
        assert!(!is_stream("https://tunein.com"));
        assert!(!is_stream("https://open.spotify.com"));
        let mut b = Bands::default();
        let silence = vec![0u8; 1066];
        assert_eq!(b.feed(&silence), [0.0; 4]);
        let loud: Vec<u8> = (0..533)
            .flat_map(|i| (((i % 40) as i16 - 20) * 1500).to_le_bytes())
            .collect();
        let l = b.feed(&loud);
        assert!(l[0] > 0.3, "{l:?}");
    }
}
