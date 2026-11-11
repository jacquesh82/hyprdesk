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
    MediaInfo {
        bus: bus.to_string(),
        title,
        artist,
        playing,
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
    Status {
        player: bus.trim_start_matches(MPRIS_PREFIX).to_string(),
        bus: bus.to_string(),
        playing: base.playing,
        title: base.title,
        artist: base.artist,
        album: text_of(&meta, "xesam:album"),
        art: text_of(&meta, "mpris:artUrl"),
        position: int(props.get("Position")).map(|us| us / 1_000_000),
        length: int(meta.get("mpris:length")).map(|us| us / 1_000_000),
    }
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
}
