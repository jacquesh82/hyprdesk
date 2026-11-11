//! Captures et enregistrement de l'écran (MLG-14) : hyprdesk orchestre
//! hyprshot (captures) et wf-recorder (vidéo) plutôt que de les refaire.
//!
//! `hyprdesk capture <zone|fenetre|ecran>` : capture enregistrée dans
//! Images/Captures et copiée dans le presse-papiers (hyprshot notifie).
//! `hyprdesk capture enregistrer [zone]` : démarre l'enregistrement (écran
//! actif, ou zone choisie avec slurp) dans Vidéos/Enregistrements, ou
//! l'arrête s'il tourne ; à l'arrêt, une notification propose « Ouvrir ».
//! L'enregistrement en cours est décrit dans un fichier d'état du dossier
//! d'exécution, que le plugin de barre `capture` lit pour son indicateur.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Ce que l'on capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shot {
    Region,
    Window,
    Output,
}

impl Shot {
    /// Mode de hyprshot.
    pub fn mode(self) -> &'static str {
        match self {
            Shot::Region => "region",
            Shot::Window => "window",
            Shot::Output => "output",
        }
    }
}

/// Action demandée en ligne de commande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Shot(Shot),
    /// Enregistrer (ou arrêter) : zone choisie, sinon l'écran actif.
    Record {
        region: bool,
    },
    Stop,
}

pub const USAGE: &str = "hyprdesk capture <zone|fenetre|ecran|enregistrer [zone]|arreter>";

pub fn parse(args: &[String]) -> Result<Action, String> {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    Ok(match a.as_slice() {
        ["zone" | "region"] => Action::Shot(Shot::Region),
        ["fenetre" | "window"] => Action::Shot(Shot::Window),
        ["ecran" | "output" | "screen"] => Action::Shot(Shot::Output),
        ["enregistrer" | "record"] => Action::Record { region: false },
        ["enregistrer" | "record", "zone" | "region"] => Action::Record { region: true },
        ["arreter" | "stop"] => Action::Stop,
        _ => return Err(USAGE.into()),
    })
}

/// Dossier XDG de l'utilisateur (`xdg-user-dir PICTURES`), sinon `~/<repli>`.
fn user_dir(kind: &str, fallback: &str) -> PathBuf {
    Command::new("xdg-user-dir")
        .arg(kind)
        .stderr(Stdio::null())
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|d| !d.is_empty() && Path::new(d).is_absolute())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback)
        })
}

pub fn shots_dir() -> PathBuf {
    user_dir("PICTURES", "Pictures").join("Captures")
}

pub fn videos_dir() -> PathBuf {
    user_dir("VIDEOS", "Videos").join("Enregistrements")
}

/// Fichier d'état de l'enregistrement en cours.
pub fn state_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("hyprdesk-recording")
}

/// Enregistrement en cours : processus, fichier, début (secondes Unix).
#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    pub pid: u32,
    pub file: PathBuf,
    pub started: u64,
}

impl Recording {
    /// Trois lignes : pid, début, fichier (lu aussi par le plugin Lua).
    pub fn to_state(&self) -> String {
        format!("{}\n{}\n{}\n", self.pid, self.started, self.file.display())
    }

    pub fn from_state(s: &str) -> Option<Self> {
        let mut l = s.lines();
        Some(Recording {
            pid: l.next()?.trim().parse().ok()?,
            started: l.next()?.trim().parse().ok()?,
            file: PathBuf::from(l.next()?.trim()).to_path_buf(),
        })
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Nom de fichier horodaté (`2026-11-11_02-30-05.mp4`).
pub fn stamp_name(ext: &str) -> String {
    let out = Command::new("date")
        .arg("+%Y-%m-%d_%H-%M-%S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| now().to_string());
    format!("{out}.{ext}")
}

fn alive(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|c| c.trim() == "wf-recorder")
}

/// Enregistrement en cours, d'après le fichier d'état (et le processus).
pub fn current() -> Option<Recording> {
    let r = Recording::from_state(&std::fs::read_to_string(state_path()).ok()?)?;
    if alive(r.pid) {
        Some(r)
    } else {
        let _ = std::fs::remove_file(state_path());
        None
    }
}

/// Arguments de wf-recorder.
pub fn record_args(file: &Path, geometry: Option<&str>, output: Option<&str>) -> Vec<String> {
    let mut a = vec!["-f".to_string(), file.display().to_string()];
    if let Some(g) = geometry {
        a.extend(["-g".into(), g.into()]);
    } else if let Some(o) = output {
        a.extend(["-o".into(), o.into()]);
    }
    a
}

/// Écran actif (connecteur), d'après Hyprland.
fn focused_output() -> Option<String> {
    let out = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    v.as_array()?
        .iter()
        .find(|m| m["focused"].as_bool() == Some(true))
        .and_then(|m| m["name"].as_str().map(String::from))
}

fn notify(summary: &str, body: &str) {
    let _ = Command::new("notify-send")
        .args(["-a", "hyprdesk", summary, body])
        .status();
}

/// Fichier le plus récent d'un dossier, modifié après `since`.
pub fn newest_since(dir: &std::path::Path, since: std::time::SystemTime) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let t = e.metadata().ok()?.modified().ok()?;
            (t >= since && e.path().is_file()).then(|| (t, e.path()))
        })
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// Capture d'écran. hyprshot reste silencieux : la notification vient
/// d'ici (« Capture enregistrée », avec « Ouvrir ») — celle de hyprshot
/// n'arrivait pas toujours jusqu'au centre de notifications.
pub fn shot(s: Shot) -> std::io::Result<()> {
    let dir = shots_dir();
    std::fs::create_dir_all(&dir)?;
    let since = std::time::SystemTime::now() - Duration::from_secs(1);
    let status = Command::new("hyprshot")
        .args(["-z", "--silent", "-m", s.mode(), "-o"])
        .arg(&dir)
        .status()?;
    if !status.success() {
        // Zone abandonnée (Échap) : rien à signaler.
        return Err(std::io::Error::other(format!("hyprshot : {status}")));
    }
    let Some(file) = newest_since(&dir, since) else {
        return Ok(());
    };
    let out = Command::new("notify-send")
        .args(["-a", "hyprdesk", "-i"])
        .arg(&file)
        .arg("-A")
        .arg(format!("open={}", crate::i18n::tr("Ouvrir")))
        .arg(crate::i18n::tr("Capture enregistrée"))
        .arg(crate::tr!(
            "{f} — copiée dans le presse-papiers",
            f = file.display()
        ))
        .output()?;
    if String::from_utf8_lossy(&out.stdout).trim() == "open" {
        let _ = Command::new("xdg-open").arg(&file).spawn();
    }
    Ok(())
}

/// Démarre un enregistrement (détaché : il survit à cette commande).
pub fn start(region: bool) -> std::io::Result<Recording> {
    let geometry = if region {
        let out = Command::new("slurp").output()?;
        if !out.status.success() {
            return Err(std::io::Error::other(crate::i18n::tr("zone non choisie")));
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    };
    let dir = videos_dir();
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(stamp_name("mp4"));
    let output = focused_output();
    let child = {
        use std::os::unix::process::CommandExt;
        Command::new("wf-recorder")
            .args(record_args(&file, geometry.as_deref(), output.as_deref()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()?
    };
    let r = Recording {
        pid: child.id(),
        file,
        started: now(),
    };
    std::fs::write(state_path(), r.to_state())?;
    Ok(r)
}

/// Arrête l'enregistrement (SIGINT : wf-recorder termine proprement le
/// fichier), puis propose de l'ouvrir.
pub fn stop(r: &Recording) -> std::io::Result<()> {
    let _ = Command::new("kill")
        .args(["-INT", &r.pid.to_string()])
        .status();
    for _ in 0..50 {
        if !alive(r.pid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = std::fs::remove_file(state_path());
    // Notification avec action : notify-send attend la réponse.
    let out = Command::new("notify-send")
        .args(["-a", "hyprdesk", "-A"])
        .arg(format!("open={}", crate::i18n::tr("Ouvrir")))
        .arg(crate::i18n::tr("Enregistrement terminé"))
        .arg(r.file.display().to_string())
        .output()?;
    if String::from_utf8_lossy(&out.stdout).trim() == "open" {
        let _ = Command::new("xdg-open").arg(&r.file).spawn();
    }
    Ok(())
}

/// Exécute une commande `hyprdesk capture …`.
pub fn run(a: Action) -> std::io::Result<()> {
    match (a, current()) {
        (Action::Shot(s), _) => shot(s),
        (Action::Record { .. } | Action::Stop, Some(r)) => stop(&r),
        (Action::Record { region }, None) => match start(region) {
            Ok(_) => Ok(()),
            Err(e) => {
                notify(crate::i18n::tr("Enregistrement impossible"), &e.to_string());
                Err(e)
            }
        },
        (Action::Stop, None) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn commandes() {
        assert_eq!(parse(&args(&["zone"])), Ok(Action::Shot(Shot::Region)));
        assert_eq!(parse(&args(&["window"])), Ok(Action::Shot(Shot::Window)));
        assert_eq!(parse(&args(&["ecran"])), Ok(Action::Shot(Shot::Output)));
        assert_eq!(
            parse(&args(&["enregistrer"])),
            Ok(Action::Record { region: false })
        );
        assert_eq!(
            parse(&args(&["enregistrer", "zone"])),
            Ok(Action::Record { region: true })
        );
        assert_eq!(parse(&args(&["arreter"])), Ok(Action::Stop));
        assert!(parse(&args(&[])).is_err());
        assert!(parse(&args(&["zone", "x"])).is_err());
    }

    #[test]
    fn etat_aller_retour() {
        let r = Recording {
            pid: 4242,
            file: PathBuf::from("/home/u/Vidéos/Enregistrements/a b.mp4"),
            started: 1_794_000_000,
        };
        assert_eq!(Recording::from_state(&r.to_state()), Some(r));
        assert_eq!(Recording::from_state("x\n1\n/f"), None);
        assert_eq!(Recording::from_state("1\n2"), None);
    }

    #[test]
    fn arguments_de_wf_recorder() {
        let f = Path::new("/v/a.mp4");
        assert_eq!(record_args(f, None, None), ["-f", "/v/a.mp4"]);
        assert_eq!(
            record_args(f, Some("10,20 300x200"), Some("DP-1")),
            ["-f", "/v/a.mp4", "-g", "10,20 300x200"]
        );
        assert_eq!(
            record_args(f, None, Some("DP-1")),
            ["-f", "/v/a.mp4", "-o", "DP-1"]
        );
    }
}

#[cfg(test)]
mod tests_newest {
    use super::*;

    #[test]
    fn capture_la_plus_recente() {
        let d = std::env::temp_dir().join(format!("hyprdesk-shots-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let since = std::time::SystemTime::now() - Duration::from_secs(1);
        std::fs::write(d.join("a.png"), b"a").unwrap();
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(d.join("b.png"), b"b").unwrap();
        assert_eq!(newest_since(&d, since), Some(d.join("b.png")));
        let later = std::time::SystemTime::now() + Duration::from_secs(60);
        assert_eq!(newest_since(&d, later), None);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
