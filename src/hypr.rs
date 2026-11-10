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

/// Fenêtre ouverte, pour le dock des applications ouvertes.
#[derive(Debug, Clone, PartialEq)]
pub struct Client {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace: String,
}

pub fn parse_clients(v: &Value) -> Vec<Client> {
    let Some(list) = v.as_array() else { return Vec::new() };
    list.iter()
        .filter(|c| c["mapped"].as_bool().unwrap_or(true) && !c["hidden"].as_bool().unwrap_or(false))
        .filter_map(|c| {
            let class = c["class"].as_str().filter(|s| !s.is_empty()).or(c["initialClass"].as_str())?.to_string();
            Some(Client {
                address: c["address"].as_str()?.to_string(),
                class,
                title: c["title"].as_str().unwrap_or("").to_string(),
                workspace: c["workspace"]["name"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect()
}

pub async fn clients_async() -> Vec<Client> {
    match tokio::task::spawn_blocking(|| ipc::query_json("clients")).await {
        Ok(Ok(v)) => parse_clients(&v),
        Ok(Err(e)) => {
            tracing::warn!("lecture des fenêtres : {e}");
            Vec::new()
        }
        Err(_) => Vec::new(),
    }
}

/// Donne le focus à une fenêtre (et donc à son workspace).
pub fn focus_window(address: &str) -> Result<(), ipc::IpcError> {
    let r = ipc::dispatch(&format!("hl.dsp.focus({{ window = \"address:{}\" }})", ipc::lua_escape(address)))?;
    if r.trim() != "ok" {
        tracing::warn!("focus {address} : {r}");
    }
    Ok(())
}

/// Ce que le flux d'événements apporte à l'interface.
#[derive(Debug, Clone)]
pub enum Update {
    Monitors(Vec<Monitor>),
    Clients(Vec<Client>),
    /// Hyprland a relu sa config : ses raccourcis d'exécution ont disparu.
    Reloaded,
}

/// Événement qui change la liste des fenêtres.
pub fn is_window_event(line: &str) -> bool {
    let name = line.split(">>").next().unwrap_or("");
    matches!(name, "openwindow" | "closewindow" | "movewindow" | "movewindowv2")
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

/// Flux d'états : écrans et fenêtres, d'abord en entier, puis à chaque changement pertinent.
/// Se reconnecte seul si Hyprland redémarre la socket.
pub fn stream() -> impl Stream<Item = Update> {
    iced::stream::channel(16, async move |mut out: iced::futures::channel::mpsc::Sender<Update>| {
        loop {
            let _ = out.send(Update::Monitors(monitors_async().await)).await;
            let _ = out.send(Update::Clients(clients_async().await)).await;
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
                            let _ = out.send(Update::Monitors(monitors_async().await)).await;
                        }
                        if line.starts_with("configreloaded>>") {
                            let _ = out.send(Update::Reloaded).await;
                        }
                        if is_window_event(&line) {
                            let _ = out.send(Update::Clients(clients_async().await)).await;
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

// ---------------------------------------------------------------------------
// Raccourcis : ceux de hyprdesk, et la submap qui suspend ceux de Hyprland
// pendant que le bureau est affiché.

/// Nom de la submap active pendant que le bureau est affiché.
pub const SUBMAP: &str = "hyprdesk";

/// Masque de modificateurs Hyprland (`modmask` de `j/binds`).
fn modifier_bit(m: &str) -> Option<u32> {
    Some(match m.to_ascii_uppercase().as_str() {
        "SHIFT" => 1,
        "CAPS" => 2,
        "CTRL" | "CONTROL" => 4,
        "ALT" | "MOD1" => 8,
        "MOD2" => 16,
        "MOD3" => 32,
        "SUPER" | "WIN" | "LOGO" | "MOD4" | "META" => 64,
        "MOD5" => 128,
        _ => return None,
    })
}

/// `"SUPER + SHIFT + D"` → (modmask, touche en minuscules).
pub fn parse_combo(s: &str) -> Option<(u32, String)> {
    let parts: Vec<&str> = s.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
    let (key, mods) = parts.split_last()?;
    if modifier_bit(key).is_some() {
        return None;
    }
    let mut mask = 0;
    for m in mods {
        mask |= modifier_bit(m)?;
    }
    Some((mask, key.to_lowercase()))
}

/// Raccourci déjà pris dans Hyprland (hors submap, hors ceux de hyprdesk).
pub fn conflicts_with(binds: &Value, combo: &str) -> Option<String> {
    let (mask, key) = parse_combo(combo)?;
    binds.as_array()?.iter().find_map(|b| {
        let same = b["modmask"].as_u64() == Some(mask as u64)
            && b["key"].as_str().is_some_and(|k| k.to_lowercase() == key)
            && b["submap"].as_str().unwrap_or("").is_empty()
            && !b["description"].as_str().unwrap_or("").starts_with("hyprdesk");
        same.then(|| b["dispatcher"].as_str().unwrap_or("?").replace("HL.Dispatcher(", "").replace(')', ""))
    })
}

/// Raccourcis de Hyprland marqués `submap_universal` : ils restent actifs
/// sur le bureau. Rendus lisibles (« SUPER + eacute → focus »).
pub fn universal_binds(binds: &Value) -> Vec<String> {
    let names = [(64, "SUPER"), (4, "CTRL"), (8, "ALT"), (1, "SHIFT")];
    binds
        .as_array()
        .map(|l| {
            l.iter()
                .filter(|b| matches!(&b["submap_universal"], Value::Bool(true)) || b["submap_universal"].as_str() == Some("true"))
                .filter(|b| !b["description"].as_str().unwrap_or("").starts_with("hyprdesk"))
                .map(|b| {
                    let mask = b["modmask"].as_u64().unwrap_or(0) as u32;
                    let mut parts: Vec<String> = names.iter().filter(|(bit, _)| mask & bit != 0).map(|(_, n)| n.to_string()).collect();
                    parts.push(b["key"].as_str().unwrap_or("?").to_string());
                    let what = b["dispatcher"].as_str().unwrap_or("").replace("HL.Dispatcher(", "").replace(')', "");
                    format!("{} → {what}", parts.join(" + "))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Lua qui (ré)installe les raccourcis de hyprdesk dans Hyprland. Ceux de la
/// submap rendent les raccourcis AVANT d'appeler hyprdesk : même démon
/// arrêté, on ne reste jamais bloqué dans la submap.
pub fn build_keys_lua(keys: &crate::config::Keys, exe: &str, skip: &[String]) -> String {
    let q = |s: &str| format!("\"{}\"", ipc::lua_escape(s));
    let cmd = |arg: &str| format!("{} {arg}", crate::launch::sh_quote(exe));
    let mut lua = String::from(
        "if __hyprdesk_binds then for _, b in ipairs(__hyprdesk_binds) do pcall(function() b:remove() end) end end\n\
         __hyprdesk_binds = {}\n\
         local function keep(b) table.insert(__hyprdesk_binds, b) end\n\
         local function leave(c) return function() hl.dispatch(hl.dsp.submap(\"reset\")) hl.dispatch(hl.dsp.exec_cmd(c)) end end\n",
    );
    let usable = |k: &String| !k.is_empty() && !skip.contains(k);
    if usable(&keys.toggle) {
        lua += &format!("keep(hl.bind({}, hl.dsp.exec_cmd({}), {{ description = \"hyprdesk : afficher le bureau\" }}))\n", q(&keys.toggle), q(&cmd("toggle")));
    }
    if usable(&keys.settings) {
        lua += &format!("keep(hl.bind({}, hl.dsp.exec_cmd({}), {{ description = \"hyprdesk : réglages\" }}))\n", q(&keys.settings), q(&cmd("settings")));
    }
    lua += &format!("hl.define_submap({}, function()\n", q(SUBMAP));
    for (k, desc) in [(&keys.toggle, "masquer le bureau"), (&keys.leave, "sortie de secours")] {
        if usable(k) {
            lua += &format!("  keep(hl.bind({}, leave({}), {{ description = \"hyprdesk : {desc}\" }}))\n", q(k), q(&cmd("hide")));
        }
    }
    lua += "end)\n";
    lua
}

/// Installe les raccourcis ; rend les conflits (raccourci, action déjà liée)
/// et les raccourcis universels de Hyprland.
pub fn install_keys(keys: &crate::config::Keys) -> Result<(Vec<(String, String)>, Vec<String>), ipc::IpcError> {
    let binds = ipc::query_json("binds")?;
    let mut conflicts = Vec::new();
    for k in [&keys.toggle, &keys.settings, &keys.leave] {
        if !k.is_empty()
            && let Some(what) = conflicts_with(&binds, k)
            && !conflicts.iter().any(|(c, _): &(String, String)| c == k)
        {
            conflicts.push((k.clone(), what));
        }
    }
    let skip: Vec<String> = conflicts.iter().map(|(k, _)| k.clone()).collect();
    let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "hyprdesk".into());
    let r = ipc::eval(&build_keys_lua(keys, &exe, &skip))?;
    if r.trim() != "ok" {
        tracing::warn!("installation des raccourcis : {r}");
    }
    let after = ipc::query_json("binds").unwrap_or(binds);
    Ok((conflicts, universal_binds(&after)))
}

/// Retire les raccourcis de hyprdesk (arrêt du démon).
pub fn remove_keys() {
    let lua = "if __hyprdesk_binds then for _, b in ipairs(__hyprdesk_binds) do pcall(function() b:remove() end) end end __hyprdesk_binds = nil";
    if let Err(e) = ipc::eval(lua) {
        tracing::warn!("retrait des raccourcis : {e}");
    }
}

/// Entre dans la submap du bureau, ou en sort (`None`).
pub fn set_submap(name: Option<&str>) {
    let target = format!("hl.dsp.submap(\"{}\")", ipc::lua_escape(name.unwrap_or("reset")));
    if let Err(e) = ipc::dispatch(&target) {
        tracing::warn!("submap : {e}");
    }
}

/// Submap courante (pour se remettre d'un arrêt brutal).
pub fn current_submap() -> String {
    ipc::repl("hl.get_current_submap()").unwrap_or_default().trim().to_string()
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
    fn raccourcis_et_conflits() {
        assert_eq!(parse_combo("SUPER + SHIFT + D"), Some((65, "d".into())));
        assert_eq!(parse_combo("super+Escape"), Some((64, "escape".into())));
        assert_eq!(parse_combo("F12"), Some((0, "f12".into())));
        assert_eq!(parse_combo("SUPER + HYPER + D"), None);
        assert_eq!(parse_combo("SUPER +"), None);
        let binds: Value = serde_json::from_str(
            r#"[{"modmask":64,"key":"Q","submap":"","description":"","dispatcher":"HL.Dispatcher(close)","submap_universal":"false"},
                {"modmask":64,"key":"D","submap":"","description":"hyprdesk : afficher le bureau","dispatcher":"HL.Dispatcher(exec_cmd)","submap_universal":"false"},
                {"modmask":64,"key":"eacute","submap":"","description":"","dispatcher":"HL.Dispatcher(focus)","submap_universal":"true"},
                {"modmask":64,"key":"D","submap":"hyprdesk","description":"","dispatcher":"HL.Dispatcher(exec_cmd)","submap_universal":"false"}]"#,
        )
        .unwrap();
        assert_eq!(conflicts_with(&binds, "SUPER + q").as_deref(), Some("close"));
        // Nos propres raccourcis et ceux d'une autre submap ne sont pas des conflits.
        assert_eq!(conflicts_with(&binds, "SUPER + D"), None);
        assert_eq!(universal_binds(&binds), vec!["SUPER + eacute → focus".to_string()]);
    }

    #[test]
    fn lua_des_raccourcis() {
        let k = crate::config::Keys { toggle: "SUPER + D".into(), settings: "SUPER + SHIFT + D".into(), leave: "SUPER + Escape".into() };
        let lua = build_keys_lua(&k, "/bin/hyprdesk", &["SUPER + SHIFT + D".to_string()]);
        assert!(lua.contains(r#"keep(hl.bind("SUPER + D", hl.dsp.exec_cmd("'/bin/hyprdesk' toggle")"#), "{lua}");
        assert!(!lua.contains("SHIFT"), "un raccourci en conflit n'est pas installé : {lua}");
        assert!(lua.contains(r#"hl.define_submap("hyprdesk", function()"#));
        assert!(lua.contains(r#"keep(hl.bind("SUPER + Escape", leave("'/bin/hyprdesk' hide")"#), "{lua}");
    }

    #[test]
    fn fenetres() {
        let v: Value = serde_json::from_str(
            r#"[{"address":"0x1","class":"kitty","title":"t","workspace":{"name":"2"},"mapped":true,"hidden":false},
                {"address":"0x2","class":"","initialClass":"steam","title":"","workspace":{"name":"3"}},
                {"address":"0x3","class":"x","workspace":{"name":"1"},"hidden":true}]"#,
        )
        .unwrap();
        let c = parse_clients(&v);
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].class.as_str(), c[0].workspace.as_str()), ("kitty", "2"));
        assert_eq!(c[1].class, "steam");
        assert!(is_window_event("closewindow>>0x1"));
        assert!(!is_window_event("activewindow>>a,b"));
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
