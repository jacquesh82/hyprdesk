//! Widget « media » : lecteur MPRIS sur le bus D-Bus de session.

use std::collections::HashMap;

use tokio::sync::OnceCell;
use zbus::zvariant::OwnedValue;

use super::{MediaInfo, Value};

static BUS: OnceCell<zbus::Connection> = OnceCell::const_new();

async fn bus() -> zbus::Result<&'static zbus::Connection> {
    BUS.get_or_try_init(zbus::Connection::session).await
}

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const PLAYER_IFACE: &str = "org.mpris.MediaPlayer2.Player";

async fn players(conn: &zbus::Connection) -> zbus::Result<Vec<String>> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await?;
    let names: Vec<String> = reply.body().deserialize()?;
    Ok(names
        .into_iter()
        .filter(|n| n.starts_with(MPRIS_PREFIX))
        .collect())
}

async fn player_props(
    conn: &zbus::Connection,
    bus: &str,
) -> zbus::Result<HashMap<String, OwnedValue>> {
    let reply = conn
        .call_method(
            Some(bus),
            "/org/mpris/MediaPlayer2",
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(PLAYER_IFACE,),
        )
        .await?;
    reply.body().deserialize()
}

fn media_from_props(bus: &str, props: &HashMap<String, OwnedValue>) -> MediaInfo {
    let playing = props
        .get("PlaybackStatus")
        .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
        .is_some_and(|s| s == "Playing");
    let meta: HashMap<String, OwnedValue> = props
        .get("Metadata")
        .and_then(|v| HashMap::<String, OwnedValue>::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default();
    let title = meta
        .get("xesam:title")
        .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default();
    let artist = meta
        .get("xesam:artist")
        .and_then(|v| Vec::<String>::try_from(v.try_clone().ok()?).ok())
        .map(|a| a.join(", "))
        .unwrap_or_default();
    // Pochette : un fichier local (`file://…`) ; une adresse web n'est pas
    // téléchargée.
    let art = meta
        .get("mpris:artUrl")
        .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
        .and_then(|u| {
            u.strip_prefix("file://")
                .map(|p| std::path::PathBuf::from(percent_decode(p)))
        })
        .filter(|p| p.is_file());
    let tint = art.as_deref().and_then(art_tint);
    MediaInfo {
        bus: bus.to_string(),
        title,
        artist,
        playing,
        art,
        tint,
        album: text_of(&meta, "xesam:album"),
        position: int(props.get("Position")).map(|us| us / 1_000_000),
        length: int(meta.get("mpris:length")).map(|us| us / 1_000_000),
    }
}

/// Nom lisible d'un lecteur d'après son nom de bus :
/// `org.mpris.MediaPlayer2.firefox.instance_1_23` donne « Firefox ».
pub fn player_name(bus: &str) -> String {
    let short = bus.trim_start_matches(MPRIS_PREFIX);
    let short = short.split('.').next().unwrap_or(short);
    let mut c = short.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Teinte moyenne d'une pochette, gardée par fichier (MLG-279).
fn art_tint(path: &std::path::Path) -> Option<crate::config::Rgba> {
    use std::sync::Mutex;
    static CACHE: Mutex<Option<(std::path::PathBuf, Option<crate::config::Rgba>)>> =
        Mutex::new(None);
    if let Ok(c) = CACHE.lock()
        && let Some((p, t)) = c.as_ref()
        && p == path
    {
        return *t;
    }
    let tint = ::image::open(path).ok().map(|img| {
        let small = img.thumbnail(8, 8).to_rgb8();
        let n = (small.width() * small.height()).max(1) as u64;
        let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
        for p in small.pixels() {
            r += u64::from(p[0]);
            g += u64::from(p[1]);
            b += u64::from(p[2]);
        }
        crate::config::Rgba((r / n) as u8, (g / n) as u8, (b / n) as u8, 255)
    });
    if let Ok(mut c) = CACHE.lock() {
        *c = Some((path.to_path_buf(), tint));
    }
    tint
}

/// `%20` et consorts d'une adresse `file://`.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Met en pause tous les lecteurs qui jouent (avant d'en lancer un autre :
/// jamais deux médias l'un par-dessus l'autre).
pub async fn pause_all() {
    let Ok(conn) = bus().await else {
        return;
    };
    let Ok(names) = players(conn).await else {
        return;
    };
    for name in names {
        let playing = player_props(conn, &name)
            .await
            .map(|p| media_from_props(&name, &p).playing)
            .unwrap_or(false);
        if playing {
            media_control(name, "Pause").await;
        }
    }
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
            let Ok(props) = player_props(conn, &name).await else {
                continue;
            };
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

/// État détaillé du lecteur (menu du plugin « lecteur », `hyprdesk media status`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// Nom court du lecteur (`spotify`, `firefox.instance…`).
    pub player: String,
    pub bus: String,
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Pochette (`mpris:artUrl`, souvent file:// ou https://).
    pub art: String,
    /// Position et durée, en secondes.
    pub position: Option<i64>,
    pub length: Option<i64>,
}

fn int(v: Option<&OwnedValue>) -> Option<i64> {
    let v = v?;
    i64::try_from(v.try_clone().ok()?)
        .ok()
        .or_else(|| u64::try_from(v.try_clone().ok()?).ok().map(|u| u as i64))
}

fn text_of(meta: &HashMap<String, OwnedValue>, k: &str) -> String {
    meta.get(k)
        .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default()
}

fn status_from_props(bus: &str, props: &HashMap<String, OwnedValue>) -> Status {
    let base = media_from_props(bus, props);
    let meta: HashMap<String, OwnedValue> = props
        .get("Metadata")
        .and_then(|v| HashMap::<String, OwnedValue>::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default();
    // Album, position et durée : ceux du widget, lus une fois.
    Status {
        player: bus.trim_start_matches(MPRIS_PREFIX).to_string(),
        bus: bus.to_string(),
        playing: base.playing,
        title: base.title,
        artist: base.artist,
        album: base.album,
        art: text_of(&meta, "mpris:artUrl"),
        position: base.position,
        length: base.length,
    }
}

/// Tous les lecteurs : la capture Panorama ne dépend pas du premier MPRIS.
pub async fn statuses() -> Vec<Status> {
    let Ok(conn) = bus().await else {
        return Vec::new();
    };
    let Ok(names) = players(conn).await else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for name in names {
        if let Ok(props) = player_props(conn, &name).await {
            out.push(status_from_props(&name, &props));
        }
    }
    out
}

/// Lecteur détaillé : celui qui joue en priorité, sinon le premier.
pub async fn status() -> Option<Status> {
    let conn = bus().await.ok()?;
    let mut best = None;
    for name in players(conn).await.ok()? {
        let Ok(props) = player_props(conn, &name).await else {
            continue;
        };
        let s = status_from_props(&name, &props);
        if s.playing {
            return Some(s);
        }
        best.get_or_insert(s);
    }
    best
}

/// Lignes « clé=valeur » (lues par le plugin Lua) ; retours à la ligne ôtés.
pub fn status_lines(s: &Status) -> String {
    let one = |v: &str| v.replace(['\n', '\r'], " ");
    let opt = |v: Option<i64>| v.map(|x| x.to_string()).unwrap_or_default();
    format!(
        "player={}\nstatus={}\ntitle={}\nartist={}\nalbum={}\nart={}\nposition={}\nlength={}\n",
        one(&s.player),
        if s.playing { "playing" } else { "paused" },
        one(&s.title),
        one(&s.artist),
        one(&s.album),
        one(&s.art),
        opt(s.position),
        opt(s.length),
    )
}

/// Met le lecteur au premier plan (`Raise` de l'interface racine MPRIS) :
/// clic sur le fond du widget média (HIG SYS-10). Un lecteur qui ne sait
/// pas le faire (`CanRaise` faux) est seulement journalisé.
pub async fn media_raise(bus_name: String) {
    let r = async {
        let conn = bus().await?;
        conn.call_method(
            Some(bus_name.as_str()),
            "/org/mpris/MediaPlayer2",
            Some("org.mpris.MediaPlayer2"),
            "Raise",
            &(),
        )
        .await?;
        Ok::<_, zbus::Error>(())
    };
    if let Err(e) = r.await {
        tracing::warn!("{}", crate::tr!("MPRIS Raise : {e}", e = e));
    }
}

/// Avance ou recule de `secs` secondes (Seek, en microsecondes).
pub async fn seek(bus_name: &str, secs: i64) -> Result<(), String> {
    let conn = bus().await.map_err(|e| e.to_string())?;
    conn.call_method(
        Some(bus_name),
        "/org/mpris/MediaPlayer2",
        Some(PLAYER_IFACE),
        "Seek",
        &(secs * 1_000_000,),
    )
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// `PlayPause`, `Next` ou `Previous` sur un lecteur.
pub async fn media_control(bus_name: String, method: &'static str) {
    let r = async {
        let conn = bus().await?;
        conn.call_method(
            Some(bus_name.as_str()),
            "/org/mpris/MediaPlayer2",
            Some(PLAYER_IFACE),
            method,
            &(),
        )
        .await?;
        Ok::<_, zbus::Error>(())
    };
    if let Err(e) = r.await {
        tracing::warn!(
            "{}",
            crate::tr!("MPRIS {method} : {e}", method = method, e = e)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lignes_d_etat() {
        let s = Status {
            player: "spotify".into(),
            bus: "org.mpris.MediaPlayer2.spotify".into(),
            playing: true,
            title: "Titre\nsur deux lignes".into(),
            artist: "A, B".into(),
            album: "Album".into(),
            art: "file:///tmp/a.png".into(),
            position: Some(83),
            length: None,
        };
        assert_eq!(
            status_lines(&s),
            "player=spotify\nstatus=playing\ntitle=Titre sur deux lignes\nartist=A, B\nalbum=Album\nart=file:///tmp/a.png\nposition=83\nlength=\n"
        );
    }

    /// Lecteur nommé d'après son bus, instance retirée (HIG SYS-15).
    #[test]
    fn nom_du_lecteur() {
        assert_eq!(player_name("org.mpris.MediaPlayer2.spotify"), "Spotify");
        assert_eq!(
            player_name("org.mpris.MediaPlayer2.firefox.instance_1_23"),
            "Firefox"
        );
        assert_eq!(player_name("org.mpris.MediaPlayer2."), "");
    }
}
