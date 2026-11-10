//! Hyprland : état des écrans (requête `j/monitors`) et flux d'événements
//! (`.socket2.sock`). L'IPC bas niveau vient de `hyprpanel-core`.
//!
//! Plutôt que de rejouer chaque événement sur un état local (source de
//! dérive), tout événement qui touche aux écrans ou aux workspaces déclenche
//! une relecture complète de `j/monitors` : c'est quelques centaines d'octets.

use std::time::Duration;

use hyprpanel_core::ipc;
use iced::futures::{SinkExt, Stream};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    pub name: String,
    /// Taille logique (rotation et échelle appliquées).
    pub width: u32,
    pub height: u32,
    /// Workspace affiché : le spécial s'il est ouvert, sinon l'actif.
    pub workspace: String,
    pub focused: bool,
}

pub fn parse_monitors(v: &Value) -> Vec<Monitor> {
    let Some(list) = v.as_array() else { return Vec::new() };
    list.iter()
        .filter(|m| !m["disabled"].as_bool().unwrap_or(false))
        .filter_map(|m| {
            let name = m["name"].as_str()?.to_string();
            let (w, h) = (m["width"].as_u64()? as f64, m["height"].as_u64()? as f64);
            let scale = m["scale"].as_f64().filter(|s| *s > 0.0).unwrap_or(1.0);
            // transform impair = rotation de 90° ou 270° : largeur et hauteur s'échangent.
            let rotated = m["transform"].as_u64().unwrap_or(0) % 2 == 1;
            let (w, h) = if rotated { (h, w) } else { (w, h) };
            let special = m["specialWorkspace"]["name"].as_str().unwrap_or("");
            let workspace = if !special.is_empty() {
                special.to_string()
            } else {
                m["activeWorkspace"]["name"].as_str().unwrap_or("").to_string()
            };
            Some(Monitor {
                name,
                width: (w / scale).round() as u32,
                height: (h / scale).round() as u32,
                workspace,
                focused: m["focused"].as_bool().unwrap_or(false),
            })
        })
        .collect()
}

pub fn monitors() -> Result<Vec<Monitor>, ipc::IpcError> {
    Ok(parse_monitors(&ipc::query_json("monitors")?))
}

pub async fn monitors_async() -> Vec<Monitor> {
    match tokio::task::spawn_blocking(monitors).await {
        Ok(Ok(m)) => m,
        Ok(Err(e)) => {
            tracing::warn!("lecture des écrans : {e}");
            Vec::new()
        }
        Err(_) => Vec::new(),
    }
}

/// L'événement socket2 impose-t-il de relire les écrans ?
pub fn is_relevant(line: &str) -> bool {
    let name = line.split(">>").next().unwrap_or("");
    matches!(
        name,
        "workspace"
            | "workspacev2"
            | "focusedmon"
            | "focusedmonv2"
            | "activespecial"
            | "activespecialv2"
            | "monitoradded"
            | "monitoraddedv2"
            | "monitorremoved"
            | "monitorremovedv2"
            | "moveworkspace"
            | "moveworkspacev2"
            | "renameworkspace"
            | "configreloaded"
    )
}

/// Flux d'états d'écrans : un premier état, puis un à chaque changement pertinent.
/// Se reconnecte seul si Hyprland redémarre la socket.
pub fn monitor_stream() -> impl Stream<Item = Vec<Monitor>> {
    iced::stream::channel(16, async move |mut out: iced::futures::channel::mpsc::Sender<Vec<Monitor>>| {
        loop {
            let _ = out.send(monitors_async().await).await;
            let path = match ipc::event_socket_path() {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("socket d'événements : {e}");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };
            match UnixStream::connect(&path).await {
                Ok(stream) => {
                    let mut lines = BufReader::new(stream).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if is_relevant(&line) {
                            let _ = out.send(monitors_async().await).await;
                        }
                    }
                    tracing::info!("socket d'événements fermée, reconnexion");
                }
                Err(e) => tracing::warn!("connexion à {} : {e}", path.display()),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    })
}

/// Lance une commande par Hyprland (`hl.dsp.exec_cmd`) : le processus est un
/// enfant du compositeur, pas de hyprdesk, et les règles d'exec s'appliquent.
pub fn exec(cmd: &str) -> Result<(), ipc::IpcError> {
    let r = ipc::dispatch(&format!("hl.dsp.exec_cmd(\"{}\")", ipc::lua_escape(cmd)))?;
    if r.trim() != "ok" {
        tracing::warn!("exec « {cmd} » : réponse inattendue de Hyprland : {r}");
    }
    Ok(())
}

/// Notification Hyprland (icône 0 avertissement, 1 info, 3 ok, 4 erreur).
pub fn notify(icon: u8, ms: u32, msg: &str) {
    if let Err(e) = ipc::notify(icon, ms, "0", msg) {
        tracing::warn!("notification Hyprland : {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecrans_rotation_echelle_special() {
        let v: Value = serde_json::from_str(
            r#"[
              {"name":"HDMI-A-2","width":2560,"height":1440,"scale":1,"transform":0,"focused":true,
               "activeWorkspace":{"id":2,"name":"2"},"specialWorkspace":{"id":0,"name":""}},
              {"name":"DP-1","width":2560,"height":1440,"scale":1.25,"transform":1,"focused":false,
               "activeWorkspace":{"id":6,"name":"6"},"specialWorkspace":{"id":-98,"name":"special:magic"}},
              {"name":"OFF","width":1,"height":1,"disabled":true,"activeWorkspace":{"name":"9"}}
            ]"#,
        )
        .unwrap();
        let m = parse_monitors(&v);
        assert_eq!(m.len(), 2);
        assert_eq!((m[0].width, m[0].height, m[0].workspace.as_str(), m[0].focused), (2560, 1440, "2", true));
        assert_eq!((m[1].width, m[1].height), (1152, 2048));
        assert_eq!(m[1].workspace, "special:magic");
    }

    #[test]
    fn evenements_pertinents() {
        assert!(is_relevant("workspacev2>>3,3"));
        assert!(is_relevant("focusedmon>>DP-1,6"));
        assert!(is_relevant("activespecial>>,HDMI-A-2"));
        assert!(!is_relevant("activewindow>>kitty,~"));
        assert!(!is_relevant("openwindow>>abc,1,kitty,t"));
    }

    #[test]
    fn exec_echappe_la_commande() {
        let c = ipc::build_dispatch(&format!("hl.dsp.exec_cmd(\"{}\")", ipc::lua_escape("xdg-open \"/a b\"")));
        assert_eq!(c, r#"/dispatch hl.dsp.exec_cmd("xdg-open \"/a b\"")"#);
    }
}
