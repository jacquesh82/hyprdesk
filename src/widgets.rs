//! Sources de données des widgets. Horloge et calendrier se calculent à
//! l'affichage ; les autres sont échantillonnés hors du thread UI, et
//! seulement quand le bureau est visible.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use sysinfo::{Disks, System};
use tokio::sync::OnceCell;
use zbus::zvariant::OwnedValue;

use crate::config::SysItem;

#[derive(Debug, Clone, PartialEq)]
pub struct SysLine {
    pub label: String,
    pub value: String,
    /// Remplissage 0..1 pour une jauge, si la mesure s'y prête.
    pub ratio: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaInfo {
    /// Nom de bus (`org.mpris.MediaPlayer2.spotify`), pour les commandes.
    pub bus: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Sys(Vec<SysLine>),
    Media(Option<MediaInfo>),
    Error(String),
}

// ---------------------------------------------------------------------------
// Système

/// Un seul `System` partagé : l'usage CPU se calcule entre deux relevés.
static SYS: LazyLock<Option<Mutex<System>>> = LazyLock::new(|| System::new().ok().map(Mutex::new));

fn human_bytes(b: u64) -> String {
    const U: [&str; 5] = ["o", "Kio", "Mio", "Gio", "Tio"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i >= 3 { format!("{v:.1} {}", U[i]) } else { format!("{v:.0} {}", U[i]) }
}

fn human_duration(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, (secs % 86400) / 3600, (secs % 3600) / 60);
    if d > 0 { format!("{d} j {h} h") } else { format!("{h} h {m:02}") }
}

/// Relevé synchrone (à appeler dans `spawn_blocking`).
pub fn sample_system(items: &[SysItem]) -> Vec<SysLine> {
    let Some(sys) = SYS.as_ref() else {
        return vec![SysLine { label: "système".into(), value: "indisponible".into(), ratio: None }];
    };
    let mut sys = sys.lock().unwrap_or_else(|e| e.into_inner());
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let need_disks = items.iter().any(|i| matches!(i, SysItem::Disk(_)));
    let disks = need_disks.then(Disks::new_with_refreshed_list).and_then(Result::ok);
    items
        .iter()
        .map(|it| match it {
            SysItem::Cpu => {
                let u = sys.global_cpu_usage();
                SysLine { label: "CPU".into(), value: format!("{u:.0} %"), ratio: Some(u / 100.0) }
            }
            SysItem::Ram => {
                let (used, total) = (sys.used_memory(), sys.total_memory().max(1));
                SysLine {
                    label: "RAM".into(),
                    value: format!("{} / {}", human_bytes(used), human_bytes(total)),
                    ratio: Some(used as f32 / total as f32),
                }
            }
            SysItem::Load => {
                let value = System::load_average()
                    .map(|l| format!("{:.2} {:.2} {:.2}", l.one, l.five, l.fifteen))
                    .unwrap_or_else(|_| "?".into());
                SysLine { label: "Charge".into(), value, ratio: None }
            }
            SysItem::Uptime => SysLine { label: "Allumé".into(), value: System::uptime().map(human_duration).unwrap_or_else(|_| "?".into()), ratio: None },
            SysItem::Disk(mount) => {
                let d = disks.as_ref().and_then(|ds| ds.iter().find(|d| d.mount_point() == std::path::Path::new(mount)));
                match d {
                    Some(d) => {
                        let total = d.total_space().max(1);
                        let used = total.saturating_sub(d.available_space());
                        SysLine {
                            label: mount.clone(),
                            value: format!("{} / {}", human_bytes(used), human_bytes(total)),
                            ratio: Some(used as f32 / total as f32),
                        }
                    }
                    None => SysLine { label: mount.clone(), value: "non monté".into(), ratio: None },
                }
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Commande

pub async fn run_command(cmd: &str, timeout: Duration) -> Value {
    let child = tokio::process::Command::new("sh").arg("-c").arg(cmd).kill_on_drop(true).output();
    match tokio::time::timeout(timeout, child).await {
        Err(_) => Value::Error(format!("délai dépassé ({} s)", timeout.as_secs_f32())),
        Ok(Err(e)) => Value::Error(e.to_string()),
        Ok(Ok(o)) => {
            let out = String::from_utf8_lossy(&o.stdout);
            // Au plus 20 lignes : un widget n'est pas un terminal.
            let text: Vec<&str> = out.trim_end().lines().take(20).collect();
            if !o.status.success() && text.is_empty() {
                let err = String::from_utf8_lossy(&o.stderr);
                Value::Error(err.lines().next().unwrap_or("échec").to_string())
            } else {
                Value::Text(text.join("\n"))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Média (MPRIS sur D-Bus session)

static BUS: OnceCell<zbus::Connection> = OnceCell::const_new();

async fn bus() -> zbus::Result<&'static zbus::Connection> {
    BUS.get_or_try_init(zbus::Connection::session).await
}

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const PLAYER_IFACE: &str = "org.mpris.MediaPlayer2.Player";

async fn players(conn: &zbus::Connection) -> zbus::Result<Vec<String>> {
    let reply = conn
        .call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "ListNames", &())
        .await?;
    let names: Vec<String> = reply.body().deserialize()?;
    Ok(names.into_iter().filter(|n| n.starts_with(MPRIS_PREFIX)).collect())
}

async fn player_props(conn: &zbus::Connection, bus: &str) -> zbus::Result<HashMap<String, OwnedValue>> {
    let reply = conn
        .call_method(Some(bus), "/org/mpris/MediaPlayer2", Some("org.freedesktop.DBus.Properties"), "GetAll", &(PLAYER_IFACE,))
        .await?;
    reply.body().deserialize()
}

fn media_from_props(bus: &str, props: &HashMap<String, OwnedValue>) -> MediaInfo {
    let playing = props
        .get("PlaybackStatus")
        .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
        .is_some_and(|s| s == "Playing");
    let meta: HashMap<String, OwnedValue> =
        props.get("Metadata").and_then(|v| HashMap::<String, OwnedValue>::try_from(v.try_clone().ok()?).ok()).unwrap_or_default();
    let title = meta.get("xesam:title").and_then(|v| String::try_from(v.try_clone().ok()?).ok()).unwrap_or_default();
    let artist = meta
        .get("xesam:artist")
        .and_then(|v| Vec::<String>::try_from(v.try_clone().ok()?).ok())
        .map(|a| a.join(", "))
        .unwrap_or_default();
    MediaInfo { bus: bus.to_string(), title, artist, playing }
}

/// Lecteur à afficher : celui qui joue en priorité, filtré par `player`.
pub async fn media(player: &str) -> Value {
    let run = async {
        let conn = bus().await?;
        let mut best: Option<MediaInfo> = None;
        for name in players(conn).await? {
            let short = name.trim_start_matches(MPRIS_PREFIX).to_lowercase();
            if player != "*" && !short.contains(&player.to_lowercase()) {
                continue;
            }
            let Ok(props) = player_props(conn, &name).await else { continue };
            let info = media_from_props(&name, &props);
            if info.playing {
                return Ok::<_, zbus::Error>(Some(info));
            }
            best.get_or_insert(info);
        }
        Ok(best)
    };
    match run.await {
        Ok(m) => Value::Media(m),
        Err(e) => Value::Error(format!("MPRIS : {e}")),
    }
}

/// `PlayPause`, `Next` ou `Previous` sur un lecteur.
pub async fn media_control(bus_name: String, method: &'static str) {
    let r = async {
        let conn = bus().await?;
        conn.call_method(Some(bus_name.as_str()), "/org/mpris/MediaPlayer2", Some(PLAYER_IFACE), method, &()).await?;
        Ok::<_, zbus::Error>(())
    };
    if let Err(e) = r.await {
        tracing::warn!("MPRIS {method} : {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailles_et_durees() {
        assert_eq!(human_bytes(512), "512 o");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024 / 2), "1.5 Gio");
        assert_eq!(human_duration(90061), "1 j 1 h");
        assert_eq!(human_duration(3720), "1 h 02");
    }

    #[test]
    fn systeme_sans_planter() {
        let l = sample_system(&[SysItem::Ram, SysItem::Disk("/n/existe/pas".into())]);
        assert_eq!(l.len(), 2);
        assert_eq!(l[1].value, "non monté");
    }

    #[tokio::test]
    async fn commande_et_delai() {
        assert_eq!(run_command("printf 'a\\nb'", Duration::from_secs(5)).await, Value::Text("a\nb".into()));
        assert!(matches!(run_command("sleep 5", Duration::from_millis(100)).await, Value::Error(_)));
    }
}
