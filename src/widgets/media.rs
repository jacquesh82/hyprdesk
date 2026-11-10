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

