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
    /// Écran éteint par DPMS (`dpmsStatus` à faux) : rien n'y est visible.
    pub dpms_off: bool,
    /// Taille logique (rotation et échelle appliquées).
    pub width: u32,
    pub height: u32,
    /// Workspace affiché : le spécial s'il est ouvert, sinon l'actif.
    pub workspace: String,
    /// Workspace normal de l'écran, sous un spécial ouvert par-dessus (vide
    /// sinon ou s'il est inconnu).
    pub base: String,
    pub focused: bool,
    /// Zone réservée par les barres (gauche, haut, droite, bas), en pixels logiques.
    pub reserved: (f32, f32, f32, f32),
    /// Position de l'écran dans l'espace global (coordonnées logiques).
    pub x: f32,
    pub y: f32,
    /// Échelle de Hyprland : pixels physiques = taille logique × échelle.
    pub scale: f32,
}

/// Position globale du curseur, demandée à Hyprland au moment du clic (pas de
/// suivi continu du pointeur : chaque mouvement coûterait un rendu complet).
pub fn cursor_pos() -> Option<(f32, f32)> {
    let v = ipc::query_json("cursorpos").ok()?;
    Some((v["x"].as_f64()? as f32, v["y"].as_f64()? as f32))
}

/// Workspace, pour le module « workspaces » de la barre.
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub monitor: String,
    pub windows: u32,
    /// Une fenêtre y est en plein écran (`hasfullscreen`).
    pub fullscreen: bool,
}

pub fn parse_workspaces(v: &Value) -> Vec<Workspace> {
    let mut l: Vec<Workspace> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|w| {
                    Some(Workspace {
                        id: w["id"].as_i64()?,
                        name: w["name"].as_str()?.to_string(),
                        monitor: w["monitor"].as_str().unwrap_or("").to_string(),
                        windows: w["windows"].as_u64().unwrap_or(0) as u32,
                        fullscreen: w["hasfullscreen"].as_bool().unwrap_or(false),
                    })
                })
                .filter(|w| !w.name.starts_with("special:"))
                .collect()
        })
        .unwrap_or_default();
    l.sort_by_key(|w| w.id);
    l
}

pub async fn workspaces_async() -> Vec<Workspace> {
    tokio::task::spawn_blocking(|| {
        ipc::query_json("workspaces")
            .map(|v| parse_workspaces(&v))
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

/// Fenêtre active : (classe, titre), depuis `j/activewindow`.
pub async fn active_window_async() -> Option<(String, String)> {
    tokio::task::spawn_blocking(|| {
        let v = ipc::query_json("activewindow").ok()?;
        Some((
            v["class"].as_str()?.to_string(),
            v["title"].as_str().unwrap_or("").to_string(),
        ))
    })
    .await
    .ok()
    .flatten()
}

/// `activewindow>>classe,titre` (le titre peut contenir des virgules).
pub fn parse_active_window(payload: &str) -> Option<(String, String)> {
    let (class, title) = payload.split_once(',')?;
    (!class.is_empty()).then(|| (class.to_string(), title.to_string()))
}

/// Argument `workspace` d'un dispatcher : relatif (`"+1"`) ou nom inconnu
/// cité tel quel, id sinon. Le dispatcher résout son argument par id, pas
/// par nom : un espace créé numériquement puis renommé (« code » pour
/// l'id 2) ne répond pas à son nom — Hyprland fabrique alors un clone vide
/// du même nom. Le nom est donc résolu en id avant l'appel.
fn workspace_dispatch_arg(target: &str) -> String {
    if target.starts_with('+') || target.starts_with('-') {
        return format!("\"{}\"", ipc::lua_escape(target));
    }
    match target.parse::<i64>() {
        Ok(n) => n.to_string(),
        Err(_) => match workspace_id_by_name(target) {
            Some(id) => id.to_string(),
            None => format!("\"{}\"", ipc::lua_escape(target)),
        },
    }
}

/// Id d'un espace d'après son nom. Les espaces nommés dès leur création
/// (id non numérique) ne passent pas dans la liste : leur nom reste la seule
/// clé, l'appelant le garde en repli.
fn workspace_id_by_name(name: &str) -> Option<i64> {
    let v = ipc::query_json("workspaces").ok()?;
    parse_workspaces(&v)
        .into_iter()
        .find(|w| w.name == name)
        .map(|w| w.id)
}

/// Va au workspace indiqué (nom ou numéro) ; `"+1"` / `"-1"` : relatif.
pub fn focus_workspace(target: &str) -> Result<(), ipc::IpcError> {
    let arg = workspace_dispatch_arg(target);
    let r = ipc::dispatch(&format!("hl.dsp.focus({{ workspace = {arg} }})"))?;
    if r.trim() != "ok" {
        tracing::warn!(
            "{}",
            crate::tr!("workspace {target} : {r}", target = target, r = r)
        );
    }
    Ok(())
}

pub fn focus_monitor(name: &str) {
    let _ = ipc::dispatch(&format!(
        "hl.dsp.focus({{ monitor = \"{}\" }})",
        ipc::lua_escape(name)
    ));
}

/// Fenêtre ouverte, pour le dock des applications ouvertes.
#[derive(Debug, Clone, PartialEq)]
pub struct Client {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace: String,
    /// Écran où elle se trouve (la barre y lit les urgences).
    pub monitor: String,
}

pub fn parse_clients(v: &Value) -> Vec<Client> {
    let Some(list) = v.as_array() else {
        return Vec::new();
    };
    list.iter()
        .filter(|c| {
            c["mapped"].as_bool().unwrap_or(true) && !c["hidden"].as_bool().unwrap_or(false)
        })
        .filter_map(|c| {
            let class = c["class"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or(c["initialClass"].as_str())?
                .to_string();
            Some(Client {
                address: c["address"].as_str()?.to_string(),
                class,
                title: c["title"].as_str().unwrap_or("").to_string(),
                workspace: c["workspace"]["name"].as_str().unwrap_or("").to_string(),
                monitor: c["monitor"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect()
}

/// Workspace des fenêtres réduites (bouton de Hyprbars, `hyprdesk window
/// minimize`) : le dock les montre, les rappeler les ramène sur le
/// workspace affiché.
pub const MINIMIZED: &str = "special:hyprdesk-reduites";

/// Réduit la fenêtre active : mise dans `MINIMIZED`, sans la suivre.
pub fn minimize_active() -> Result<String, String> {
    let v = ipc::query_json("activewindow").map_err(|e| e.to_string())?;
    let address = v["address"]
        .as_str()
        .filter(|a| !a.is_empty())
        .ok_or_else(|| crate::i18n::tr("aucune fenêtre active").to_string())?;
    if v["workspace"]["name"]
        .as_str()
        .is_some_and(|w| w.starts_with("special:"))
    {
        return Err(crate::i18n::tr("fenêtre déjà dans un workspace spécial").to_string());
    }
    move_window(address, MINIMIZED).map_err(|e| e.to_string())?;
    Ok(address.to_string())
}

/// Donne le focus à une fenêtre (et donc à son workspace) ; réduite, elle
/// revient d'abord sur le workspace affiché.
pub fn focus_window(address: &str) -> Result<(), ipc::IpcError> {
    let minimized = ipc::query_json("clients").ok().is_some_and(|v| {
        v.as_array().is_some_and(|l| {
            l.iter().any(|c| {
                c["address"].as_str() == Some(address)
                    && c["workspace"]["name"].as_str() == Some(MINIMIZED)
            })
        })
    });
    if minimized
        && let Some(ws) = ipc::query_json("activeworkspace")
            .ok()
            .and_then(|v| v["name"].as_str().map(str::to_string))
    {
        move_window(address, &ws)?;
    }
    let r = ipc::dispatch(&format!(
        "hl.dsp.focus({{ window = \"address:{}\" }})",
        ipc::lua_escape(address)
    ))?;
    if r.trim() != "ok" {
        tracing::warn!(
            "{}",
            crate::tr!("focus {address} : {r}", address = address, r = r)
        );
    }
    Ok(())
}

/// Ferme une fenêtre (comme son bouton de fermeture : l'application peut s'y opposer).
pub fn close_window(address: &str) -> Result<(), ipc::IpcError> {
    let r = ipc::dispatch(&format!(
        "hl.dsp.window.close({{ window = \"address:{}\" }})",
        ipc::lua_escape(address)
    ))?;
    if r.trim() != "ok" {
        tracing::warn!(
            "{}",
            crate::tr!("fermeture {address} : {r}", address = address, r = r)
        );
    }
    Ok(())
}

/// La commande Lua qui tue une fenêtre désignée par son adresse (HIG ENT-10) :
/// `kill`, le jumeau brutal de `close` — même désignation, même échappement.
/// Hyprland 0.57.2 : `hl.dsp.window.kill({ window = … })`, à côté de `close`
/// et `signal`.
pub fn kill_window_lua(address: &str) -> String {
    format!(
        "hl.dsp.window.kill({{ window = \"address:{}\" }})",
        ipc::lua_escape(address)
    )
}

/// Tue une fenêtre, même si son application s'y refuse : « Forcer à quitter ».
pub fn kill_window(address: &str) -> Result<(), ipc::IpcError> {
    let r = ipc::dispatch(&kill_window_lua(address))?;
    if r.trim() != "ok" {
        tracing::warn!(
            "{}",
            crate::tr!("destruction {address} : {r}", address = address, r = r)
        );
    }
    Ok(())
}

/// Tue des fenêtres une à une (« Forcer à quitter », HIG ENT-10). Chacune est
/// relue juste avant : une fenêtre déjà disparue — son processus, partagé
/// avec la précédente, vient d'être tué — n'est plus visée, jamais une autre
/// à sa place.
pub fn kill_windows(addresses: &[String]) {
    for a in addresses {
        let alive = ipc::query_json("clients")
            .map(|v| parse_clients(&v))
            .is_ok_and(|l| l.iter().any(|c| &c.address == a));
        if alive && let Err(e) = kill_window(a) {
            tracing::warn!(
                "{}",
                crate::tr!("destruction {address} : {r}", address = a, r = e)
            );
        }
    }
}

/// Ce que le flux d'événements apporte à l'interface.
#[derive(Debug, Clone)]
pub enum Update {
    Monitors(Vec<Monitor>),
    /// Fenêtres, et leurs géométries (minimaps) tirées de la même lecture.
    Clients(Vec<Client>, Vec<WindowGeo>),
    Workspaces(Vec<Workspace>),
    ActiveWindow(Option<(String, String)>),
    /// Une fenêtre est marquée urgente (`urgent>>adresse`) : la barre place
    /// son losange sur son workspace tant qu'il n'est pas redevenu visible.
    Urgent(String),
    /// Hyprland a relu sa config : ses raccourcis d'exécution ont disparu.
    Reloaded,
}

/// Événement qui change la liste des fenêtres.
pub fn is_window_event(line: &str) -> bool {
    let name = line.split(">>").next().unwrap_or("");
    matches!(
        name,
        "openwindow" | "closewindow" | "movewindow" | "movewindowv2"
    )
}

/// Relectures qu'une rafale d'événements impose : chaque état n'est relu
/// qu'une fois, même si Hyprland envoie l'événement en v1 et en v2, ou
/// plusieurs événements d'un coup (MLG-135).
#[derive(Debug, Default, PartialEq)]
pub struct Pending {
    pub monitors: bool,
    pub clients: bool,
    pub workspaces: bool,
    pub reloaded: bool,
    /// Dernière fenêtre active annoncée (`activewindow>>classe,titre`).
    pub active: Option<Option<(String, String)>>,
    pub urgent: Vec<String>,
}

impl Pending {
    /// Ajoute une ligne du socket d'événements.
    pub fn note(&mut self, line: &str) {
        if is_relevant(line) {
            self.monitors = true;
            self.workspaces = true;
        }
        if line.starts_with("configreloaded>>") {
            self.reloaded = true;
        }
        if is_window_event(line) {
            self.clients = true;
            self.workspaces = true;
        }
        let name = line.split(">>").next().unwrap_or("");
        // Plein écran : `hasfullscreen` des workspaces (pause du fond vidéo).
        if matches!(
            name,
            "createworkspace"
                | "createworkspacev2"
                | "destroyworkspace"
                | "destroyworkspacev2"
                | "fullscreen"
        ) {
            self.workspaces = true;
        }
        // Le mode du plein écran se lit dans les fenêtres (HIG PAT-01).
        if name == "fullscreen" {
            self.clients = true;
        }
        if let Some(p) = line.strip_prefix("activewindow>>") {
            self.active = Some(parse_active_window(p));
        }
        if let Some(p) = line.strip_prefix("urgent>>") {
            self.urgent.push(p.trim().to_string());
            self.clients = true;
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Pending::default()
    }
}

/// Fenêtres et géométries d'une seule lecture de `clients`.
pub async fn clients_and_geos_async() -> (Vec<Client>, Vec<WindowGeo>) {
    match tokio::task::spawn_blocking(|| ipc::query_json("clients")).await {
        Ok(Ok(v)) => (parse_clients(&v), parse_window_geometries(&v)),
        Ok(Err(e)) => {
            tracing::warn!("{}", crate::tr!("lecture des fenêtres : {e}", e = e));
            (Vec::new(), Vec::new())
        }
        Err(_) => (Vec::new(), Vec::new()),
    }
}

impl Monitor {
    /// Workspace dont l'écran suit le mode du bureau : le normal, même quand
    /// un spécial (scratchpad) est ouvert par-dessus ; vide s'il est inconnu.
    pub fn mode_workspace(&self) -> &str {
        if self.workspace.starts_with("special:") {
            &self.base
        } else {
            &self.workspace
        }
    }
}

pub fn parse_monitors(v: &Value) -> Vec<Monitor> {
    let Some(list) = v.as_array() else {
        return Vec::new();
    };
    list.iter()
        .filter(|m| !m["disabled"].as_bool().unwrap_or(false))
        .filter_map(|m| {
            let name = m["name"].as_str()?.to_string();
            let (w, h) = (m["width"].as_u64()? as f64, m["height"].as_u64()? as f64);
            let scale = m["scale"].as_f64().filter(|s| *s > 0.0).unwrap_or(1.0);
            // transform impair = rotation de 90° ou 270° : largeur et hauteur s'échangent.
            let rotated = m["transform"].as_u64().unwrap_or(0) % 2 == 1;
            let (w, h) = if rotated { (h, w) } else { (w, h) };
            // Le workspace des fenêtres mises de côté n'est jamais « celui de
            // l'écran » : sinon on y mettrait de côté des fenêtres déjà de côté,
            // avec lui pour adresse de retour, et elles n'en reviendraient plus.
            let special = m["specialWorkspace"]["name"].as_str().unwrap_or("").trim();
            let active = m["activeWorkspace"]["name"].as_str().unwrap_or("").trim();
            let workspace = if !special.is_empty() && special != STASH {
                special.to_string()
            } else if active != STASH {
                active.to_string()
            } else {
                String::new()
            };
            let r = |i: usize| m["reserved"][i].as_f64().unwrap_or(0.0) as f32;
            Some(Monitor {
                dpms_off: !m["dpmsStatus"].as_bool().unwrap_or(true),
                name,
                width: (w / scale).round() as u32,
                height: (h / scale).round() as u32,
                workspace,
                base: if active != STASH {
                    active.to_string()
                } else {
                    String::new()
                },
                focused: m["focused"].as_bool().unwrap_or(false),
                reserved: (r(0), r(1), r(2), r(3)),
                x: m["x"].as_f64().unwrap_or(0.0) as f32,
                y: m["y"].as_f64().unwrap_or(0.0) as f32,
                scale: scale as f32,
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
            tracing::warn!("{}", crate::tr!("lecture des écrans : {e}", e = e));
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

/// Attente des lignes suivantes d'une rafale d'événements : assez pour
/// réunir v1 et v2 et les événements d'un même geste, trop peu pour se voir.
const BURST: Duration = Duration::from_millis(12);

/// Flux d'états : écrans et fenêtres, d'abord en entier, puis à chaque changement pertinent.
/// Se reconnecte seul si Hyprland redémarre la socket.
pub fn stream() -> impl Stream<Item = Update> {
    iced::stream::channel(
        16,
        async move |mut out: iced::futures::channel::mpsc::Sender<Update>| {
            loop {
                let _ = out.send(Update::Monitors(monitors_async().await)).await;
                let (clients, geos) = clients_and_geos_async().await;
                let _ = out.send(Update::Clients(clients, geos)).await;
                let _ = out.send(Update::Workspaces(workspaces_async().await)).await;
                let _ = out
                    .send(Update::ActiveWindow(active_window_async().await))
                    .await;
                let path = match ipc::event_socket_path() {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!("{}", crate::tr!("socket d'événements : {e}", e = e));
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                };
                match UnixStream::connect(&path).await {
                    Ok(stream) => {
                        let mut lines = BufReader::new(stream).lines();
                        // Une rafale (v1 et v2 du même changement, ou plusieurs
                        // changements d'un geste) est lue en entier, puis chaque
                        // état n'est relu qu'une fois.
                        while let Ok(Some(line)) = lines.next_line().await {
                            let mut p = Pending::default();
                            p.note(&line);
                            while let Ok(Ok(Some(more))) =
                                tokio::time::timeout(BURST, lines.next_line()).await
                            {
                                p.note(&more);
                            }
                            if p.is_empty() {
                                continue;
                            }
                            if p.monitors {
                                let _ = out.send(Update::Monitors(monitors_async().await)).await;
                            }
                            if p.reloaded {
                                let _ = out.send(Update::Reloaded).await;
                            }
                            for addr in p.urgent {
                                let _ = out.send(Update::Urgent(addr)).await;
                            }
                            if p.clients {
                                let (clients, geos) = clients_and_geos_async().await;
                                let _ = out.send(Update::Clients(clients, geos)).await;
                            }
                            if p.workspaces {
                                let _ =
                                    out.send(Update::Workspaces(workspaces_async().await)).await;
                            }
                            if let Some(w) = p.active {
                                let _ = out.send(Update::ActiveWindow(w)).await;
                            }
                        }
                        tracing::info!("{}", crate::tr!("socket d'événements fermée, reconnexion"));
                    }
                    Err(e) => tracing::warn!(
                        "{}",
                        crate::tr!("connexion à {p0} : {e}", p0 = path.display(), e = e)
                    ),
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        },
    )
}

// ---------------------------------------------------------------------------
// Raccourcis : ceux de hyprdesk, et la submap qui suspend ceux de Hyprland
// pendant que le bureau est affiché.

/// Nom de la submap active pendant que le bureau est affiché.
pub const SUBMAP: &str = "hyprdesk";

/// Submap vide, le temps de capturer un raccourci dans les Réglages : la
/// combinaison arrive au tiroir au lieu de déclencher l'action en place.
pub const CAPTURE_SUBMAP: &str = "hyprdesk-capture";

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

/// Modificateurs à l'écrit, dans l'ordre de la HIG : CTRL, ALT, SHIFT, SUPER
/// (HIG ENT-04).
pub const MODIFIER_ORDER: [(u32, &str); 4] = [(4, "CTRL"), (8, "ALT"), (1, "SHIFT"), (64, "SUPER")];

/// Rang d'écriture d'un modificateur d'une combinaison ; ce qui n'en est pas
/// un vient en dernier, dans son ordre d'écriture.
pub fn modifier_rank(name: &str) -> usize {
    modifier_bit(name)
        .and_then(|bit| MODIFIER_ORDER.iter().position(|(b, _)| *b == bit))
        .unwrap_or(MODIFIER_ORDER.len())
}

/// `"SUPER + SHIFT + D"` → (modmask, touche en minuscules).
pub fn parse_combo(s: &str) -> Option<(u32, String)> {
    let parts: Vec<&str> = s
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
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

/// Raccourcis de Hyprland marqués `submap_universal` : ils restent actifs
/// sur le bureau. Rendus lisibles (« SUPER + eacute → focus »).
pub fn universal_binds(binds: &Value) -> Vec<String> {
    let names = MODIFIER_ORDER;
    binds
        .as_array()
        .map(|l| {
            l.iter()
                .filter(|b| {
                    matches!(&b["submap_universal"], Value::Bool(true))
                        || b["submap_universal"].as_str() == Some("true")
                })
                .filter(|b| {
                    !b["description"]
                        .as_str()
                        .unwrap_or("")
                        .starts_with("hyprdesk")
                })
                .map(|b| {
                    let mask = b["modmask"].as_u64().unwrap_or(0) as u32;
                    let mut parts: Vec<String> = names
                        .iter()
                        .filter(|(bit, _)| mask & bit != 0)
                        .map(|(_, n)| n.to_string())
                        .collect();
                    parts.push(b["key"].as_str().unwrap_or("?").to_string());
                    let what = b["dispatcher"]
                        .as_str()
                        .unwrap_or("")
                        .replace("HL.Dispatcher(", "")
                        .replace(')', "");
                    format!("{} → {what}", parts.join(" + "))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Lua qui (ré)installe les raccourcis de hyprdesk dans Hyprland. Ceux de la
/// submap rendent les raccourcis AVANT d'appeler hyprdesk : même démon
/// arrêté, on ne reste jamais bloqué dans la submap.
/// Raccourci de verrouillage que `scripts/install.sh` pose dans hyprland.lua.
pub const LOCK_KEY_DEFAULT: &str = "SUPER + L";

/// Touches multimédia : (touche, commande hyprdesk, répétée tant qu'appuyée).
pub const MEDIA_KEYS: &[(&str, &str, bool)] = &[
    ("XF86AudioRaiseVolume", "osd volume +5", true),
    ("XF86AudioLowerVolume", "osd volume -5", true),
    ("XF86AudioMute", "osd volume toggle", false),
    ("XF86AudioMicMute", "osd mic toggle", false),
    ("XF86MonBrightnessUp", "osd brightness +5", true),
    ("XF86MonBrightnessDown", "osd brightness -5", true),
    ("XF86KbdBrightnessUp", "osd kbd +10", false),
    ("XF86KbdBrightnessDown", "osd kbd -10", false),
    ("XF86AudioPlay", "media play-pause", false),
    ("XF86AudioPause", "media play-pause", false),
    ("XF86AudioNext", "media next", false),
    ("XF86AudioPrev", "media previous", false),
    ("Print", "capture zone", false),
    ("SHIFT + Print", "capture ecran", false),
    ("ALT + Print", "capture fenetre", false),
];

pub fn build_keys_lua(
    keys: &crate::config::Keys,
    blurred: &[&str],
    exe: &str,
    plan: &crate::keymap::Plan,
    ws_mod: &str,
) -> String {
    let q = |s: &str| format!("\"{}\"", ipc::lua_escape(s));
    let cmd = |arg: &str| format!("{} {arg}", crate::launch::sh_quote(exe));
    let mut lua = String::from(
        "if __hyprdesk_binds then for _, b in ipairs(__hyprdesk_binds) do pcall(function() b:remove() end) end end\n\
         __hyprdesk_binds = {}\n\
         local function keep(b) table.insert(__hyprdesk_binds, b) end\n\
         local function leave(c) return function() hl.dispatch(hl.dsp.submap(\"reset\")) hl.dispatch(hl.dsp.exec_cmd(c)) end end\n",
    );
    // Combinaisons prises à hyprland.lua (keys.take, verrouillage réglé,
    // lanceur unifié) : déliées avant de poser celles de hyprdesk. Une
    // combinaison rendue : Hyprland relit sa config pour retrouver celle
    // d'origine, puis hyprdesk se réinstalle.
    let taken: Vec<String> = plan.take.iter().map(|k| q(k)).collect();
    lua += &format!(
        "local taken = {{ {} }}
         local now = {{}} for _, k in ipairs(taken) do now[k] = true end
         if __hyprdesk_lock_key then __hyprdesk_taken = __hyprdesk_taken or {{ [__hyprdesk_lock_key] = true }} __hyprdesk_lock_key = nil end
         if __hyprdesk_taken then
             for k in pairs(__hyprdesk_taken) do
                 if not now[k] then __hyprdesk_taken = nil hl.dispatch(hl.dsp.exec_cmd(\"hyprctl reload\")) break end
             end
         end
         for _, k in ipairs(taken) do pcall(function() hl.unbind(k) end) end
         __hyprdesk_taken = now
",
        taken.join(", ")
    );
    let bind = |lua: &mut String, indent: &str, k: &str, action: String, desc: &str| {
        *lua += &format!(
            "{indent}keep(hl.bind({}, {action}, {{ description = \"hyprdesk : {desc}\" }}))\n",
            q(k)
        );
    };
    let exec = |arg: &str| format!("hl.dsp.exec_cmd({})", q(&cmd(arg)));
    let switch_back = crate::switcher::with_shift(&keys.switch).unwrap_or_default();
    // Hors plein bureau. Un raccourci par mode ; celui du mode en cours rend no-desktop.
    for (id, k, arg, desc) in [
        ("toggle", &keys.toggle, "cycle", "changer de mode"),
        (
            "launcher",
            &keys.launcher,
            "toggle-launcher",
            "launcher-only",
        ),
        ("hide", &keys.hide, "hide", "no-desktop"),
        ("switch", &keys.switch, "switch", "fenêtres"),
        (
            "switch-back",
            &switch_back,
            "switch prev",
            "fenêtres, en arrière",
        ),
        ("search", &keys.search, "search", "lanceur d'applications"),
        ("overview", &keys.overview, "overview", "vue d'ensemble"),
        ("assistant", &keys.assistant, "assistant", "LYRA"),
        // Un raccourci par surface du shell (HIG ENT-02) : barre, dock,
        // centre de notifications, volet des Réglages.
        (
            "focus-bar",
            &keys.focus_bar,
            "focus bar",
            "clavier à la barre",
        ),
        (
            "focus-dock",
            &keys.focus_dock,
            "focus dock",
            "clavier au dock",
        ),
        (
            "focus-notifications",
            &keys.focus_notifications,
            "focus notifications",
            "clavier au centre de notifications",
        ),
        (
            "focus-settings",
            &keys.focus_settings,
            "focus settings",
            "clavier aux réglages",
        ),
        // Forcer une application à quitter (HIG ENT-10), partout.
        (
            "force-quit",
            &keys.force_quit,
            "force-quit",
            "forcer à quitter",
        ),
        // Accès clavier complet (HIG ENT-01), partout.
        (
            "keyboard-access",
            &keys.keyboard_access,
            "keyboard-access",
            "accès clavier complet",
        ),
        ("settings", &keys.settings, "toggle-settings", "réglages"),
        ("lock", &keys.lock, "lock", "verrouiller"),
    ] {
        if plan.outside(id, k) {
            bind(&mut lua, "", k, exec(arg), desc);
        }
    }
    // Touches multimédia : actives écran verrouillé (`locked`), là où
    // hyprland.lua ne les lie pas déjà.
    let media = |lua: &mut String, indent: &str, inside: bool| {
        for (k, arg, repeating) in MEDIA_KEYS {
            let ok = if inside {
                plan.desktop("media", k)
            } else {
                plan.outside("media", k)
            };
            if keys.media && ok {
                *lua += &format!(
                    "{indent}keep(hl.bind({}, hl.dsp.exec_cmd({}), {{ locked = true, repeating = {repeating}, description = \"hyprdesk : touche multimédia\" }}))\n",
                    q(k),
                    q(&cmd(arg))
                );
            }
        }
    };
    media(&mut lua, "", false);
    // Essentiels de fenêtres, de workspaces et d'écrans (MLG-109) : là où
    // hyprland.lua n'en a pas, sur une combinaison libre.
    let essential = |lua: &mut String, indent: &str, combo: &str, l: &crate::essentials::Line| {
        let desc = if l.detail.is_empty() {
            format!("hyprdesk : {}", l.label)
        } else {
            format!("hyprdesk : {} ({})", l.label, l.detail)
        };
        let opts = if l.opts.is_empty() {
            String::new()
        } else {
            format!(", {}", l.opts)
        };
        *lua += &format!(
            "{indent}keep(hl.bind({}, {}, {{ description = {}{opts} }}))\n",
            q(combo),
            l.lua,
            q(&desc)
        );
    };
    for l in plan.essentials.iter().filter(|l| l.state.installs()) {
        essential(&mut lua, "", &l.combo, l);
    }
    // Dans la submap (full-desktop), les raccourcis de Hyprland sont
    // suspendus, sauf ceux marqués submap_universal.
    lua += &format!("hl.define_submap({}, function()\n", q(SUBMAP));
    let leave = |arg: &str| format!("leave({})", q(&cmd(arg)));
    // Chaque raccourci de mode rend d'abord la main, puis change de mode.
    for (id, k, arg, desc) in [
        ("toggle", &keys.toggle, "hide", "no-desktop"),
        ("launcher", &keys.launcher, "launcher-only", "launcher-only"),
        ("hide", &keys.hide, "hide", "no-desktop"),
        ("leave", &keys.leave, "hide", "sortie de secours"),
    ] {
        if plan.desktop(id, k) {
            bind(&mut lua, "  ", k, leave(arg), desc);
        }
    }
    // Réglages, verrouillage, lanceur, fenêtres : la submap reste active.
    // Verrouillage non réglé : la combinaison de hyprland.lua (posée par
    // l'installation), suspendue avec les autres dans la submap. Lanceur :
    // celui du plein bureau s'il est réglé.
    let submap_lock = if keys.lock.is_empty() {
        LOCK_KEY_DEFAULT.to_string()
    } else {
        keys.lock.clone()
    };
    let (search_id, desk_search) = if keys.search_desktop.trim().is_empty() {
        ("search", &keys.search)
    } else {
        ("search-desktop", &keys.search_desktop)
    };
    for (id, k, arg, desc) in [
        ("settings", &keys.settings, "toggle-settings", "réglages"),
        ("lock", &submap_lock, "lock", "verrouiller"),
        (search_id, desk_search, "search", "lanceur d'applications"),
        ("switch", &keys.switch, "switch", "fenêtres"),
        (
            "switch-back",
            &switch_back,
            "switch prev",
            "fenêtres, en arrière",
        ),
        ("overview", &keys.overview, "overview", "vue d'ensemble"),
        ("assistant", &keys.assistant, "assistant", "LYRA"),
        // Le clavier va à chaque surface, bureau affiché aussi (HIG ENT-02).
        (
            "focus-bar",
            &keys.focus_bar,
            "focus bar",
            "clavier à la barre",
        ),
        (
            "focus-dock",
            &keys.focus_dock,
            "focus dock",
            "clavier au dock",
        ),
        (
            "focus-notifications",
            &keys.focus_notifications,
            "focus notifications",
            "clavier au centre de notifications",
        ),
        (
            "focus-settings",
            &keys.focus_settings,
            "focus settings",
            "clavier aux réglages",
        ),
        // Forcer une application à quitter, bureau affiché aussi (HIG ENT-10).
        (
            "force-quit",
            &keys.force_quit,
            "force-quit",
            "forcer à quitter",
        ),
        // Accès clavier complet (HIG ENT-01), partout.
        (
            "keyboard-access",
            &keys.keyboard_access,
            "keyboard-access",
            "accès clavier complet",
        ),
    ] {
        if plan.desktop(id, k) {
            bind(&mut lua, "  ", k, exec(arg), desc);
        }
    }
    // Changer de workspace reste possible bureau affiché : la rangée des
    // chiffres (codes physiques 10 à 19, quelle que soit la disposition du
    // clavier) avec le modificateur des raccourcis de workspace de
    // hyprland.lua ; le bureau suit (fenêtres du nouveau workspace mises de côté).
    // Les flèches gauche et droite, avec le même modificateur : workspace
    // précédent et suivant.
    if !ws_mod.is_empty() {
        for n in 1..=10 {
            lua += &format!(
                "  keep(hl.bind({}, hl.dsp.focus({{ workspace = {n} }}), {{ description = \"hyprdesk : workspace {n}\" }}))\n",
                q(&format!("{ws_mod} + code:{}", 9 + n))
            );
        }
        for (key, rel, desc) in [
            ("Left", "-1", "workspace précédent"),
            ("Right", "+1", "workspace suivant"),
        ] {
            lua += &format!(
                "  keep(hl.bind({}, hl.dsp.focus({{ workspace = \"{rel}\" }}), {{ description = \"hyprdesk : {desc}\" }}))\n",
                q(&format!("{ws_mod} + {key}"))
            );
        }
    }
    // Essentiels qui ont un sens bureau affiché (le bureau suit).
    for l in plan.essentials.iter().filter(|l| l.desktop) {
        essential(&mut lua, "  ", &l.desktop_combo, l);
    }
    // Bureau affiché : celles de hyprland.lua sont suspendues, hyprdesk les
    // tient, sauf celles marquées submap_universal.
    media(&mut lua, "  ", true);
    lua += "end)\n";
    // Capture d'un raccourci (Réglages → Raccourcis) : aucune liaison.
    lua += &format!("hl.define_submap({}, function() end)\n", q(CAPTURE_SUBMAP));
    // Surfaces floutées selon le thème (barre, menus, lanceur, réglages),
    // menus sans animation d'apparition.
    lua += "if __hyprdesk_rules then for _, r in ipairs(__hyprdesk_rules) do pcall(function() r:set_enabled(false) end) end end\n\
            __hyprdesk_rules = {}\n";
    if !blurred.is_empty() {
        let names: Vec<&str> = blurred
            .iter()
            .map(|n| n.trim_start_matches("hyprdesk-"))
            .collect();
        lua += &format!(
            "pcall(function() table.insert(__hyprdesk_rules, hl.layer_rule({{ match = {{ namespace = \"^hyprdesk-({})$\" }}, blur = true, ignore_alpha = 0.2, xray = false }})) end)\n",
            names.join("|")
        );
    }
    lua += "pcall(function() table.insert(__hyprdesk_rules, hl.layer_rule({ match = { namespace = \"^hyprdesk-(popover|launcher|launcher-popup)$\" }, animation = \"none\" })) end)\n";
    lua
}

/// Installe les raccourcis ; rend ce qui les gêne (`crate::keymap`).
pub type KeyReport = crate::keymap::Report;

pub fn install_keys(
    keys: &crate::config::Keys,
    unified: bool,
    blurred: &[&str],
    source: &[crate::keymap::SourceBind],
    layouts: &[String],
) -> Result<KeyReport, ipc::IpcError> {
    let binds = ipc::query_json("binds")?;
    let (mut report, mut plan) = plan_keys(keys, unified, &binds, source, layouts);
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "hyprdesk".into());
    let ws_mod = workspace_modifier(&binds);
    let r = ipc::eval(&build_keys_lua(keys, blurred, &exe, &plan, &ws_mod))?;
    if r.trim() != "ok" {
        tracing::warn!("{}", crate::tr!("installation des raccourcis : {r}", r = r));
    }
    let after = ipc::query_json("binds").unwrap_or(binds);
    report.universal = universal_binds(&after);
    report.essentials = std::mem::take(&mut plan.essentials);
    report.hyprland = crate::essentials::inventory(&after, source);
    Ok(report)
}

/// Analyse des raccourcis de hyprdesk, et des essentiels à poser : leurs
/// combinaisons prises à hyprland.lua s'ajoutent à celles à délier.
fn plan_keys(
    keys: &crate::config::Keys,
    unified: bool,
    binds: &Value,
    source: &[crate::keymap::SourceBind],
    layouts: &[String],
) -> (KeyReport, crate::keymap::Plan) {
    let (report, mut plan) = crate::keymap::analyse(keys, unified, binds, source);
    let mut layouts = layouts.to_vec();
    if let Some(l) = crate::tiling::current().get("general.layout") {
        layouts.push(l.clone());
    }
    plan.essentials = crate::essentials::plan(keys, binds, source, &layouts);
    for c in crate::essentials::taken_combos(&plan.essentials) {
        if !plan.take.iter().any(|t| t == &c) {
            plan.take.push(c);
        }
    }
    (report, plan)
}

/// `hyprdesk keys` : analyse sans rien installer.
pub fn key_report(
    keys: &crate::config::Keys,
    unified: bool,
    source: &[crate::keymap::SourceBind],
    layouts: &[String],
) -> Result<KeyReport, ipc::IpcError> {
    let binds = ipc::query_json("binds")?;
    let (mut report, mut plan) = plan_keys(keys, unified, &binds, source, layouts);
    report.universal = universal_binds(&binds);
    report.essentials = std::mem::take(&mut plan.essentials);
    report.hyprland = crate::essentials::inventory(&binds, source);
    Ok(report)
}

/// Touches de la rangée des chiffres (AZERTY, QWERTY, QWERTZ…).
const NUMBER_ROW: &[&str] = &[
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "0",
    "ampersand",
    "eacute",
    "quotedbl",
    "apostrophe",
    "parenleft",
    "minus",
    "egrave",
    "underscore",
    "ccedilla",
    "agrave",
    "exclam",
    "at",
    "numbersign",
    "dollar",
    "percent",
    "asciicircum",
    "asterisk",
    "parenright",
    "section",
    "plus",
];

/// Code physique d'une touche : `code:N`, ou une touche de la rangée des
/// chiffres en QWERTY ou en AZERTY (`1` et `ampersand` → 10). Hyprland
/// compare la touche sans modificateur : sur la même touche physique, deux
/// raccourcis se déclenchent ensemble.
pub fn physical_key(key: &str) -> Option<u32> {
    if let Some(c) = key.strip_prefix("code:") {
        return c.parse().ok();
    }
    // Touche à gauche du 1 : ` en QWERTY, ² en AZERTY.
    if ["grave", "twosuperior"]
        .iter()
        .any(|k| k.eq_ignore_ascii_case(key))
    {
        return Some(49);
    }
    NUMBER_ROW[..20]
        .iter()
        .position(|k| k.eq_ignore_ascii_case(key))
        .map(|i| 10 + (i % 10) as u32)
}

/// Code physique d'un raccourci de `j/binds` : posé par code (`key` vide,
/// `keycode`), ou sur une touche de la rangée des chiffres.
pub fn bind_code(b: &Value) -> Option<u32> {
    match b["keycode"].as_u64() {
        Some(c) if c > 0 => Some(c as u32),
        _ => physical_key(b["key"].as_str()?),
    }
}

/// Touche d'un raccourci de `j/binds`, en minuscules : `code:N` s'il est
/// posé par code.
pub fn bind_key(b: &Value) -> Option<String> {
    match (b["key"].as_str().unwrap_or(""), b["keycode"].as_u64()) {
        ("", Some(c)) if c > 0 => Some(format!("code:{c}")),
        ("", _) => None,
        (k, _) => Some(k.to_lowercase()),
    }
}

/// Modificateur des raccourcis de workspace de hyprland.lua : celui des
/// raccourcis « focus » posés sur la rangée des chiffres (SUPER à défaut).
pub fn workspace_modifier(binds: &Value) -> String {
    let mask = binds
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["submap"].as_str().unwrap_or("").is_empty())
        .filter(|b| {
            b["dispatcher"]
                .as_str()
                .is_some_and(|d| d.contains("focus") || d == "workspace")
        })
        .filter(|b| bind_code(b).is_some_and(|c| (10..=19).contains(&c)))
        .filter_map(|b| b["modmask"].as_u64())
        .next()
        .unwrap_or(64);
    modmask_name(mask)
}

/// Masque de modificateurs de Hyprland → « ALT + SUPER ».
pub fn modmask_name(mask: u64) -> String {
    let mut parts = Vec::new();
    for (bit, name) in MODIFIER_ORDER {
        if mask & u64::from(bit) != 0 {
            parts.push(name);
        }
    }
    parts.join(" + ")
}

/// Retire les raccourcis de hyprdesk (arrêt du démon).
pub fn remove_keys() {
    let lua = "if __hyprdesk_binds then for _, b in ipairs(__hyprdesk_binds) do pcall(function() b:remove() end) end end __hyprdesk_binds = nil";
    if let Err(e) = ipc::eval(lua) {
        tracing::warn!("{}", crate::tr!("retrait des raccourcis : {e}", e = e));
    }
}

// ---------------------------------------------------------------------------
// Workspaces permanents et affectation des applications : règles Hyprland
// posées à chaud, comme les raccourcis. Une relecture de la config Hyprland
// les efface ; hyprdesk les repose (`Update::Reloaded`).

/// Expression régulière de classe pour une affectation. Une application
/// installée est reconnue à son id `.desktop` comme à son `StartupWMClass`,
/// tels quels ou en minuscules (Wayland et XWayland ne s'accordent pas).
pub fn class_regex(app: &crate::config::AppMatch, wm_class: Option<&str>) -> String {
    use crate::config::AppMatch;
    match app {
        AppMatch::Class(re) => re.clone(),
        AppMatch::App(id) => {
            let mut names: Vec<String> = Vec::new();
            for n in [Some(id.as_str()), wm_class].into_iter().flatten() {
                for v in [n.to_string(), n.to_lowercase()] {
                    if !names.contains(&v) {
                        names.push(v);
                    }
                }
            }
            let alts: Vec<String> = names.iter().map(|n| regex_escape(n)).collect();
            format!("^({})$", alts.join("|"))
        }
    }
}

fn regex_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        if r"\.^$|?*+()[]{}".contains(c) {
            o.push('\\');
        }
        o.push(c);
    }
    o
}

/// Affectation prête à poser : classe (regex), workspace, sans y aller.
pub type Assign = (String, String, bool);

/// Workspaces nommés par une règle active vers un écran absent (hors ceux que
/// hyprdesk place déjà) ; les sélecteurs (`m[…]`, `r[…]`…) sont ignorés.
pub fn orphan_workspaces(rules: &Value, live: &[String], placement: &[Placement]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in rules.as_array().into_iter().flatten() {
        if r["enabled"].as_bool() == Some(false) {
            continue;
        }
        let (Some(ws), Some(mon)) = (r["workspaceString"].as_str(), r["monitor"].as_str()) else {
            continue;
        };
        let ws = ws.strip_prefix("name:").unwrap_or(ws);
        let plain =
            !ws.is_empty() && !ws.contains('[') && !ws.starts_with("special") && !ws.contains(' ');
        if !plain
            || mon.is_empty()
            || mon.starts_with("desc:")
            || live.iter().any(|m| m == mon)
            || placement.iter().any(|(w, _, _)| w == ws)
            || out.iter().any(|w| w == ws)
        {
            continue;
        }
        out.push(ws.to_string());
    }
    out
}

/// Lua qui remplace les règles de workspace de hyprdesk : les anciennes sont
/// désactivées (Hyprland ne sait pas les retirer), les nouvelles posées.
/// Écran d'un workspace : (workspace, écran, workspace par défaut de l'écran).
pub type Placement = (String, String, bool);

pub fn build_rules_lua(
    workspaces: Option<u8>,
    placement: &[Placement],
    assigns: &[Assign],
) -> String {
    let q = |s: &str| format!("\"{}\"", ipc::lua_escape(s));
    let mut lua = String::from(
        "if __hyprdesk_ws then for _, r in ipairs(__hyprdesk_ws) do pcall(function() r:set_enabled(false) end) end end\n\
         __hyprdesk_ws = {}\n\
         local errs = {}\n\
         local function keep(f) local ok, r = pcall(f) if ok then table.insert(__hyprdesk_ws, r) else table.insert(errs, tostring(r)) end end\n",
    );
    // Workspaces permanents (1..N) et workspaces placés sur un écran : un
    // permanent placé est créé, ou déplacé, aussitôt sur son écran.
    let n = workspaces.unwrap_or(0) as usize;
    let mut names: Vec<String> = (1..=n).map(|i| i.to_string()).collect();
    for (ws, _, _) in placement {
        if !names.contains(ws) {
            names.push(ws.clone());
        }
    }
    for ws in &names {
        let mut f = vec![format!("workspace = {}", q(ws))];
        if ws.parse::<usize>().is_ok_and(|i| (1..=n).contains(&i)) {
            f.push("persistent = true".into());
        }
        if let Some((_, mon, default)) = placement.iter().find(|(w, _, _)| w == ws) {
            f.push(format!("monitor = {}", q(mon)));
            if *default {
                f.push("default = true".into());
            }
        }
        lua += &format!(
            "keep(function() return hl.workspace_rule({{ {} }}) end)\n",
            f.join(", ")
        );
    }
    for (i, (class, ws, silent)) in assigns.iter().enumerate() {
        let target = if *silent {
            format!("{ws} silent")
        } else {
            ws.clone()
        };
        lua += &format!(
            "keep(function() return hl.window_rule({{ name = \"hyprdesk-assign-{}\", match = {{ class = {} }}, workspace = {} }}) end)\n",
            i + 1,
            q(class),
            q(&target)
        );
    }
    lua += "if #errs > 0 then error(table.concat(errs, \" ; \"), 0) end\n";
    lua
}

/// Place sur `primary` les workspaces permanents (1..=n) qu'aucun placement
/// ne rattache à un écran ; ceux que l'utilisateur a placés gardent le leur.
pub fn place_permanent(placement: &mut Vec<Placement>, n: u8, primary: &str) {
    for i in 1..=n {
        let ws = i.to_string();
        if !placement.iter().any(|(w, _, _)| *w == ws) {
            placement.push((ws, primary.to_string(), false));
        }
    }
}

/// Pose les règles de workspace ; une règle refusée par Hyprland (regex
/// invalide…) est signalée sans empêcher les autres.
pub fn install_rules(
    workspaces: Option<u8>,
    placement: &[Placement],
    rules: &[crate::config::AssignRule],
    primary: Option<&str>,
    // Écrans branchés depuis la dernière pose : ramenés sur leur workspace.
    added: &[String],
) -> Result<(), ipc::IpcError> {
    use crate::config::AppMatch;
    // Écrans relus au moment de poser : deux poses en vol (démarrage, écran
    // branché) ne peuvent pas laisser un placement vers un écran parti.
    let live: Vec<String> = monitors()
        .map(|l| l.into_iter().map(|m| m.name).collect())
        .unwrap_or_default();
    let mut placement: Vec<Placement> = placement
        .iter()
        .filter(|(_, mon, _)| live.is_empty() || live.contains(mon))
        .cloned()
        .collect();
    // Règles d'une autre source (hyprland.lua d'une époque à plusieurs
    // écrans…) vers un écran absent : Hyprland ne crée pas ces workspaces,
    // la barre ne les montre pas. Ils vont sur l'écran principal.
    let target = live
        .iter()
        .find(|m| Some(m.as_str()) == primary)
        .or(live.first())
        .filter(|_| primary.is_some())
        .cloned();
    if let Some(target) = &target
        && let Ok(v) = ipc::query_json("workspacerules")
    {
        for ws in orphan_workspaces(&v, &live, &placement) {
            tracing::info!(
                "{}",
                crate::tr!(
                    "workspace {ws} : écran de sa règle absent, placé sur {m}",
                    ws = ws,
                    m = target
                )
            );
            placement.push((ws, target.clone(), false));
        }
    }
    // Workspaces permanents sans écran choisi : sur l'écran principal. Sans
    // écran dans sa règle, un permanent va sur l'écran qui a le focus quand
    // la règle est posée — et hyprdesk la repose à chaque relecture de la
    // config : 1 à N partaient tous sur un écran secondaire, le principal
    // n'en gardait aucun (un workspace neuf, 6, y était créé).
    if let Some(target) = &target {
        place_permanent(&mut placement, workspaces.unwrap_or(0), target);
    }
    let placement = &placement[..];
    let assigns: Vec<Assign> = rules
        .iter()
        .map(|r| {
            let wm_class = match &r.app {
                AppMatch::App(id) => crate::launch::find_desktop_file(id)
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .and_then(|s| crate::launch::parse_desktop_entry(&s, ""))
                    .and_then(|e| e.wm_class),
                AppMatch::Class(_) => None,
            };
            (
                class_regex(&r.app, wm_class.as_deref()),
                r.workspace.clone(),
                r.silent,
            )
        })
        .collect();
    let r = ipc::eval(&build_rules_lua(workspaces, placement, &assigns))?;
    if r.trim() != "ok" {
        tracing::warn!("{}", crate::tr!("règles de workspace : {r}", r = r));
        notify(
            3,
            6000,
            &crate::tr!(
                "hyprdesk : règles de workspace refusées — {p0}",
                p0 = r.trim()
            ),
        );
    }
    if !added.is_empty()
        && let Ok(mons) = monitors()
    {
        rehome(&rehome_plan(
            placement,
            workspaces.unwrap_or(0) as usize,
            added,
            &mons,
        ));
    }
    Ok(())
}

/// Écran branché à ramener sur son workspace par défaut.
#[derive(Debug, Clone, PartialEq)]
pub struct Rehome {
    pub monitor: String,
    pub default: String,
    /// Workspace où Hyprland l'a mis.
    pub current: String,
    /// Ses fenêtres suivent l'écran (workspace qu'aucune règle ne déclare).
    pub take_windows: bool,
}

/// Écrans branchés (`added`) que Hyprland a mis sur un workspace hors de leur
/// liste (MLG-158). Hyprland choisit le workspace d'un écran qui arrive avant
/// que hyprdesk ne repose sa règle par défaut (retirée quand l'écran est
/// parti) : il prend alors le premier numéro libre et non lié à un autre
/// écran — 11 quand 1 à 6 existent et que 7 à 10 sont liés au principal
/// (écran dédié). Les fenêtres d'un tel workspace suivent l'écran sur son
/// workspace (le premier, vide, disparaît) ; celles d'un workspace déclaré
/// ailleurs restent chez lui.
pub fn rehome_plan(
    placement: &[Placement],
    permanent: usize,
    added: &[String],
    monitors: &[Monitor],
) -> Vec<Rehome> {
    let declared = |ws: &str| {
        placement.iter().any(|(w, _, _)| w == ws)
            || ws
                .parse::<usize>()
                .is_ok_and(|i| (1..=permanent).contains(&i))
    };
    monitors
        .iter()
        .filter(|m| added.contains(&m.name))
        .filter_map(|m| {
            let (default, _, _) = placement.iter().find(|(_, mon, d)| *d && mon == &m.name)?;
            let cur = m.base.as_str();
            if cur.is_empty()
                || cur.starts_with("special")
                || placement
                    .iter()
                    .any(|(w, mon, _)| w == cur && mon == &m.name)
            {
                return None;
            }
            Some(Rehome {
                monitor: m.name.clone(),
                default: default.clone(),
                current: cur.to_string(),
                take_windows: !declared(cur),
            })
        })
        .collect()
}

/// Applique `rehome_plan`, puis rend le focus à la fenêtre (ou à l'écran)
/// qui l'avait.
fn rehome(plan: &[Rehome]) {
    if plan.is_empty() {
        return;
    }
    let before = ipc::query_json("activewindow").ok().and_then(|v| {
        v["address"]
            .as_str()
            .filter(|a| !a.is_empty())
            .map(String::from)
    });
    let focused = monitors()
        .ok()
        .and_then(|l| l.into_iter().find(|m| m.focused).map(|m| m.name));
    // Fenêtres cachées comprises (avalées par un terminal…) : toutes suivent.
    let clients = ipc::query_json("clients").ok();
    let run = |lua: String| match ipc::dispatch(&lua) {
        Ok(r) if r.trim() == "ok" => {}
        Ok(r) => tracing::warn!("{}", crate::tr!("retour d'écran : {r}", r = r.trim())),
        Err(e) => tracing::warn!("{}", crate::tr!("retour d'écran : {r}", r = e)),
    };
    for p in plan {
        tracing::info!(
            "{}",
            crate::tr!(
                "écran {m} rebranché sur le workspace {cur} : ramené sur {ws}",
                m = p.monitor,
                cur = p.current,
                ws = p.default
            )
        );
        if p.take_windows
            && let Some(v) = &clients
        {
            for a in v
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| c["workspace"]["name"].as_str() == Some(p.current.as_str()))
                .filter_map(|c| c["address"].as_str())
            {
                if let Err(e) = move_window(a, &p.default) {
                    tracing::warn!("{}", e);
                }
            }
        }
        run(format!(
            "hl.dsp.focus({{ monitor = \"{}\" }})",
            ipc::lua_escape(&p.monitor)
        ));
        run(format!(
            "hl.dsp.focus({{ workspace = {} }})",
            workspace_dispatch_arg(&p.default)
        ));
    }
    match (before, focused) {
        (Some(a), _) => run(format!(
            "hl.dsp.focus({{ window = \"address:{}\" }})",
            ipc::lua_escape(&a)
        )),
        (None, Some(m)) => run(format!(
            "hl.dsp.focus({{ monitor = \"{}\" }})",
            ipc::lua_escape(&m)
        )),
        _ => {}
    }
}

/// Marges extérieures de Hyprland (`general:gaps_out`) : haut, droite, bas, gauche.
pub fn base_gaps_out() -> [i32; 4] {
    let css = ipc::query_json("getoption general:gaps_out")
        .ok()
        .and_then(|v| v["css"].as_str().map(String::from))
        .unwrap_or_default();
    let n: Vec<i32> = css
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    match n.as_slice() {
        [a] => [*a; 4],
        [v, h] => [*v, *h, *v, *h],
        [t, h, b] => [*t, *h, *b, *h],
        [t, r, b, l, ..] => [*t, *r, *b, *l],
        _ => [0; 4],
    }
}

/// Lua des marges « bureau avec les fenêtres » : par écran, les fenêtres en
/// mosaïque laissent libre la place des contrôles du bureau.
/// `margins` : (écran, [haut, droite, bas, gauche]) en plus de `base`.
pub fn build_gaps_lua(margins: &[(String, [i32; 4])], base: [i32; 4]) -> String {
    let mut lua = String::from(
        "if __hyprdesk_gaps then for _, r in ipairs(__hyprdesk_gaps) do pcall(function() r:set_enabled(false) end) end end\n\
         __hyprdesk_gaps = {}\n",
    );
    for (mon, m) in margins {
        lua += &format!(
            "table.insert(__hyprdesk_gaps, hl.workspace_rule({{ workspace = \"m[{}]\", gaps_out = {{ top = {}, right = {}, bottom = {}, left = {} }} }}))\n",
            ipc::lua_escape(mon),
            base[0] + m[0],
            base[1] + m[1],
            base[2] + m[2],
            base[3] + m[3],
        );
    }
    lua
}

/// Pose (ou retire, liste vide) les marges des contrôles du bureau.
pub fn set_desktop_gaps(margins: &[(String, [i32; 4])]) {
    let base = if margins.is_empty() {
        [0; 4]
    } else {
        base_gaps_out()
    };
    match ipc::eval(&build_gaps_lua(margins, base)) {
        Ok(r) if r.trim() == "ok" => {}
        Ok(r) => tracing::warn!("{}", crate::tr!("marges du bureau : {p0}", p0 = r.trim())),
        Err(e) => tracing::warn!("{}", crate::tr!("marges du bureau : {e}", e = e)),
    }
}

/// Fenêtre en plein écran ou agrandie, telle que Hyprland la tient
/// (`internal`) et telle que l'application la croit (`client`) :
/// 0 normale, 1 agrandie, 2 plein écran.
#[derive(Debug, Clone, PartialEq)]
pub struct FsWindow {
    pub address: String,
    pub monitor: String,
    pub internal: i64,
    pub client: i64,
}

/// Fenêtres en plein écran ou agrandies (écran nommé, pas numéroté).
pub fn fullscreen_windows() -> Vec<FsWindow> {
    let (Ok(clients), Ok(monitors)) = (ipc::query_json("clients"), ipc::query_json("monitors"))
    else {
        return Vec::new();
    };
    let name_of = |id: i64| {
        monitors
            .as_array()
            .and_then(|l| l.iter().find(|m| m["id"].as_i64() == Some(id)))
            .and_then(|m| m["name"].as_str())
            .unwrap_or("")
            .to_string()
    };
    clients
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|c| {
                    let internal = c["fullscreen"].as_i64().unwrap_or(0);
                    let client = c["fullscreenClient"].as_i64().unwrap_or(0);
                    (internal != 0 || client != 0).then(|| FsWindow {
                        address: c["address"].as_str().unwrap_or("").to_string(),
                        monitor: name_of(c["monitor"].as_i64().unwrap_or(-1)),
                        internal,
                        client,
                    })
                })
                .filter(|w| !w.address.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Ce que les tiroirs (réglages, LYRA) imposent au plein écran : sur leur
/// écran, une fenêtre en plein écran passe agrandie pour Hyprland — elle
/// laisse le tiroir visible, dans les marges qu'il réserve — en restant en
/// plein écran pour l'application (vidéo sans interface). `held` : fenêtres
/// déjà ainsi retenues. Rend les états à poser (adresse, interne, client),
/// les fenêtres retenues en plus et celles relâchées.
/// États à poser (adresse, interne, client), retenues en plus, relâchées.
pub type DrawerFullscreenPlan = (Vec<(String, i64, i64)>, Vec<String>, Vec<String>);

pub fn plan_drawer_fullscreen(
    windows: &[FsWindow],
    drawers: &[String],
    held: &std::collections::HashSet<String>,
) -> DrawerFullscreenPlan {
    let (mut set, mut added, mut dropped) = (Vec::new(), Vec::new(), Vec::new());
    for w in windows {
        let on_drawer = drawers.contains(&w.monitor);
        let kept = w.internal == 1 && w.client == 2;
        if held.contains(&w.address) {
            if !on_drawer {
                // Tiroir fermé (ou parti sur un autre écran) : plein écran rendu.
                if kept {
                    set.push((w.address.clone(), 2, 2));
                }
                dropped.push(w.address.clone());
            } else if w.internal == 2 {
                // Plein écran basculé (Super+F) sur une fenêtre retenue : Hyprland
                // la remet en plein écran au lieu d'en sortir ; c'est une sortie.
                set.push((w.address.clone(), 0, 0));
                dropped.push(w.address.clone());
            } else if !kept {
                // Sortie du plein écran par l'application ou l'utilisateur.
                dropped.push(w.address.clone());
            }
        } else if on_drawer && w.internal == 2 {
            set.push((w.address.clone(), 1, w.client.max(1)));
            added.push(w.address.clone());
        }
    }
    // Retenues disparues (fenêtre fermée, plein écran quitté) : relâchées.
    for a in held {
        if !windows.iter().any(|w| &w.address == a) {
            dropped.push(a.clone());
        }
    }
    (set, added, dropped)
}

/// Applique `plan_drawer_fullscreen` à l'état courant de Hyprland.
pub fn fit_drawer_fullscreen(
    drawers: &[String],
    held: &std::collections::HashSet<String>,
) -> (Vec<String>, Vec<String>) {
    let (set, added, dropped) = plan_drawer_fullscreen(&fullscreen_windows(), drawers, held);
    for (address, internal, client) in set {
        if let Err(e) = ipc::dispatch(&format!(
            "hl.dsp.window.fullscreen_state({{ internal = {internal}, client = {client}, action = \"set\", window = \"address:{}\" }})",
            ipc::lua_escape(&address)
        )) {
            tracing::warn!(
                "{}",
                crate::tr!("plein écran près d'un tiroir : {e}", e = e)
            );
        }
    }
    (added, dropped)
}

/// Écran vu par le recadrage des fenêtres flottantes près d'un tiroir.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawerScreen {
    /// Workspace affiché.
    pub workspace: String,
    /// Écran, en coordonnées globales.
    pub rect: Rect,
    /// Largeur du tiroir à droite (réglages, LYRA) ; 0 : aucun.
    pub right: i32,
}

/// Fenêtres flottantes retenues hors d'un tiroir : adresse → (place
/// d'origine, place donnée par hyprdesk).
pub type FloatHeld = std::collections::HashMap<String, (Rect, Rect)>;

/// Plan du recadrage des fenêtres flottantes : `gaps_out` ne pousse que la
/// mosaïque, une fenêtre flottante (ouverte depuis le bureau, plein bureau)
/// resterait sous le tiroir. Sur l'écran d'un tiroir, celle qui déborde sur
/// lui est décalée vers la gauche, rétrécie si elle ne tient pas ; le tiroir
/// refermé, elle revient à sa place si personne ne l'a déplacée entre-temps.
/// `gap` : marge extérieure de Hyprland (gauche, droite). Rend les places à
/// poser et les fenêtres retenues après coup.
pub fn plan_drawer_floats(
    windows: &[Geometry],
    screens: &[DrawerScreen],
    gap: (i32, i32),
    held: &FloatHeld,
    skip_class: &str,
) -> (Vec<(String, Rect)>, FloatHeld) {
    let mut moves = Vec::new();
    let mut next = FloatHeld::new();
    for w in windows {
        if !w.floating || w.class == skip_class || w.workspace.starts_with("special:") {
            continue;
        }
        let Some(s) = screens.iter().find(|s| s.workspace == w.workspace) else {
            continue;
        };
        let (x, y, ww, wh) = w.rect;
        let prior = held.get(&w.address);
        if s.right > 0 {
            let left = s.rect.0 + gap.0;
            let limit = s.rect.0 + s.rect.2 - s.right - gap.1;
            if x + ww <= limit {
                // Déjà hors du tiroir : retenue seulement si c'est notre place.
                if let Some(&(orig, placed)) = prior.filter(|(_, p)| *p == w.rect) {
                    next.insert(w.address.clone(), (orig, placed));
                }
                continue;
            }
            let nw = ww.min(limit - left).max(1);
            let nx = (limit - nw).max(left);
            let placed = (nx, y, nw, wh);
            let orig = prior.map_or(w.rect, |&(o, _)| o);
            moves.push((w.address.clone(), placed));
            next.insert(w.address.clone(), (orig, placed));
        } else if let Some(&(orig, placed)) = prior {
            // Tiroir refermé : retour à la place d'origine, sauf fenêtre déplacée.
            if placed == w.rect {
                moves.push((w.address.clone(), orig));
            }
        }
    }
    (moves, next)
}

/// Pose le plan de [`plan_drawer_floats`] d'après l'état de Hyprland.
pub fn fit_drawer_floats(screens: &[DrawerScreen], held: &FloatHeld) -> FloatHeld {
    let Ok(clients) = ipc::query_json("clients") else {
        return held.clone();
    };
    let g = base_gaps_out();
    let (moves, next) = plan_drawer_floats(
        &geometries(&clients),
        screens,
        (g[3], g[1]),
        held,
        crate::assistant::LYRA_APP_ID,
    );
    for (address, rect) in moves {
        if let Err(e) = place(&address, rect) {
            tracing::warn!(
                "{}",
                crate::tr!("fenêtre flottante près d'un tiroir : {e}", e = e)
            );
        }
    }
    next
}

/// Désactive les règles de workspace de hyprdesk (arrêt du démon).
pub fn remove_rules() {
    if let Err(e) = ipc::eval(&build_rules_lua(None, &[], &[])) {
        tracing::warn!(
            "{}",
            crate::tr!("retrait des règles de workspace : {e}", e = e)
        );
    }
}

/// Entre dans la submap du bureau, ou en sort (`None`).
/// Ferme la session Hyprland (les fenêtres avec) : pour qu'un réglage lu au
/// démarrage seulement (carte de rendu, `AQ_DRM_DEVICES`) prenne effet.
pub fn exit_session() -> Result<(), String> {
    let r = ipc::dispatch("hl.dsp.exit()").map_err(|e| e.to_string())?;
    match r.trim() {
        "ok" | "" => Ok(()),
        other => Err(other.to_string()),
    }
}

pub fn set_submap(name: Option<&str>) {
    let target = format!(
        "hl.dsp.submap(\"{}\")",
        ipc::lua_escape(name.unwrap_or("reset"))
    );
    if let Err(e) = ipc::dispatch(&target) {
        tracing::warn!("{}", crate::tr!("submap : {e}", e = e));
    }
}

/// Submap courante (pour se remettre d'un arrêt brutal).
pub fn current_submap() -> String {
    ipc::repl("hl.get_current_submap()")
        .unwrap_or_default()
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// Fenêtres mises de côté pendant que le bureau est affiché (comme « afficher
// le bureau » ailleurs) : elles attendent dans un workspace spécial, et le
// bureau, resté sous les fenêtres, contient celles qu'on ouvre depuis lui.

/// Version minimale de Hyprland (API Lua de 0.57). En 0.56.1, rendre à Lua
/// l'objet d'un raccourci (`hl.bind`, par `hyprctl eval`) fait planter
/// Hyprland : hyprdesk ne lui envoie rien en dessous.
pub const MIN_HYPRLAND: (u32, u32) = (0, 57);

/// Version de Hyprland (`v0.57.2` → (0, 57, 2)).
pub fn parse_version(tag: &str) -> Option<(u32, u32, u32)> {
    let mut it = tag.trim().trim_start_matches('v').split(['.', '-', '+']);
    let mut n = || it.next().and_then(|p| p.parse::<u32>().ok());
    Some((n()?, n()?, n().unwrap_or(0)))
}

/// Version de Hyprland en marche (None : injoignable ou illisible).
pub fn running_version() -> Option<(u32, u32, u32)> {
    let v = ipc::query_json("version").ok()?;
    v["tag"]
        .as_str()
        .and_then(parse_version)
        .or_else(|| v["version"].as_str().and_then(parse_version))
}

/// Workspace spécial où attendent les fenêtres mises de côté.
pub const STASH: &str = "special:hyprdesk";

/// Workspace spécial où les versions d'avant MLG-55 rangeaient les fenêtres
/// du bureau masqué ; vidé au prochain affichage du bureau.
pub const DESK: &str = "special:hyprdesk-bureau";

/// Parmi ces adresses, les fenêtres flottantes (liste `clients` de Hyprland).
pub fn floating_among(clients: &Value, addresses: &[String]) -> Vec<String> {
    clients
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["floating"].as_bool() == Some(true))
        .filter_map(|c| c["address"].as_str())
        .filter(|a| addresses.iter().any(|x| x == a))
        .map(String::from)
        .collect()
}

/// Fenêtres d'un workspace (par nom).
pub fn windows_on(clients: &Value, workspace: &str) -> Vec<String> {
    clients
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["workspace"]["name"].as_str() == Some(workspace))
        .filter_map(|c| c["address"].as_str().map(String::from))
        .collect()
}

/// Workspace ordinaire de l'écran qui a le focus.
pub fn focused_workspace() -> String {
    let monitors = ipc::query_json("monitors").unwrap_or(Value::Null);
    let workspaces = ipc::query_json("workspaces").unwrap_or(Value::Null);
    let focused = monitors
        .as_array()
        .and_then(|l| l.iter().find(|m| m["focused"].as_bool() == Some(true)))
        .and_then(|m| m["name"].as_str())
        .unwrap_or("")
        .to_string();
    regular_workspace(&monitors, &workspaces, &focused)
}

/// Rectangle d'une fenêtre, en coordonnées logiques globales : x, y, largeur, hauteur.
pub type Rect = (i32, i32, i32, i32);

/// Fenêtre ordinaire (affichée) d'un workspace, avec sa géométrie.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    pub address: String,
    pub workspace: String,
    /// Classe de la fenêtre (application).
    pub class: String,
    pub floating: bool,
    pub rect: Rect,
}

/// Fenêtres affichées (mappées, non cachées) d'après `clients`.
pub fn geometries(clients: &Value) -> Vec<Geometry> {
    clients
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| {
            c["mapped"].as_bool().unwrap_or(true) && !c["hidden"].as_bool().unwrap_or(false)
        })
        .filter_map(|c| {
            let n = |v: &Value, i: usize| v[i].as_i64().unwrap_or(0) as i32;
            Some(Geometry {
                address: c["address"].as_str()?.to_string(),
                workspace: c["workspace"]["name"].as_str()?.to_string(),
                class: c["class"].as_str().unwrap_or_default().to_string(),
                floating: c["floating"].as_bool().unwrap_or(false),
                rect: (
                    n(&c["at"], 0),
                    n(&c["at"], 1),
                    n(&c["size"], 0),
                    n(&c["size"], 1),
                ),
            })
        })
        .collect()
}

/// Fenêtre d'un workspace, avec titre et géométrie : pour l'écran « Espaces
/// de travail » et les miniatures du sélecteur.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowGeo {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace: String,
    pub floating: bool,
    pub rect: Rect,
    /// Vrai plein écran (mode 2 de Hyprland), pas la fenêtre agrandie (mode
    /// 1), sous laquelle les calques `top` restent visibles (HIG PAT-01).
    pub fullscreen: bool,
}

/// Fenêtres d'après `clients` : titres et géométrie en coordonnées globales.
/// Les fenêtres des workspaces spéciaux (`special:`) sont écartées.
pub fn parse_window_geometries(v: &Value) -> Vec<WindowGeo> {
    let mut l: Vec<WindowGeo> = v
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| {
            c["mapped"].as_bool().unwrap_or(true) && !c["hidden"].as_bool().unwrap_or(false)
        })
        .filter_map(|c| {
            let class = c["class"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or(c["initialClass"].as_str())?
                .to_string();
            let n = |v: &Value, i: usize| v[i].as_i64().unwrap_or(0) as i32;
            Some(WindowGeo {
                address: c["address"].as_str()?.to_string(),
                class,
                title: c["title"].as_str().unwrap_or("").to_string(),
                workspace: c["workspace"]["name"].as_str()?.to_string(),
                floating: c["floating"].as_bool().unwrap_or(false),
                rect: (
                    n(&c["at"], 0),
                    n(&c["at"], 1),
                    n(&c["size"], 0),
                    n(&c["size"], 1),
                ),
                fullscreen: c["fullscreen"].as_i64().unwrap_or(0) & 2 != 0,
            })
        })
        .filter(|w| !w.workspace.starts_with("special:"))
        .collect();
    l.sort_by(|a, b| {
        a.workspace
            .cmp(&b.workspace)
            .then(a.address.cmp(&b.address))
    });
    l
}

pub async fn window_geometries_async() -> Vec<WindowGeo> {
    match tokio::task::spawn_blocking(|| ipc::query_json("clients")).await {
        Ok(Ok(v)) => parse_window_geometries(&v),
        Ok(Err(e)) => {
            tracing::warn!("{}", crate::tr!("lecture des fenêtres : {e}", e = e));
            Vec::new()
        }
        Err(_) => Vec::new(),
    }
}

/// Plein bureau : `n` fenêtres flottantes organisées dans la zone utile d'un
/// écran. Une seule : centrée, 60 % × 70 % ; sinon une grille (colonnes ≈ √n),
/// la dernière rangée centrée.
pub fn arrange(n: usize, area: Rect, gap: i32) -> Vec<Rect> {
    let (x, y, w, h) = area;
    if n == 0 || w <= 0 || h <= 0 {
        return Vec::new();
    }
    if n == 1 {
        let (ww, hh) = ((w as f32 * 0.6) as i32, (h as f32 * 0.7) as i32);
        return vec![(x + (w - ww) / 2, y + (h - hh) / 2, ww, hh)];
    }
    let cols = (n as f32).sqrt().ceil() as usize;
    let rows = n.div_ceil(cols);
    let cw = (w - gap * (cols as i32 - 1)) / cols as i32;
    let ch = (h - gap * (rows as i32 - 1)) / rows as i32;
    (0..n)
        .map(|i| {
            let (r, c) = (i / cols, i % cols);
            // Dernière rangée incomplète : centrée.
            let in_row = if r == rows - 1 { n - r * cols } else { cols };
            let shift = (cols - in_row) as i32 * (cw + gap) / 2;
            (
                x + shift + c as i32 * (cw + gap),
                y + r as i32 * (ch + gap),
                cw,
                ch,
            )
        })
        .collect()
}

fn on_window(address: &str, cmd: &str) -> Result<(), String> {
    let r =
        ipc::dispatch(&cmd.replace("{w}", &ipc::lua_escape(address))).map_err(|e| e.to_string())?;
    if r.trim() == "ok" {
        Ok(())
    } else {
        Err(crate::tr!(
            "{address} : {r}",
            address = address.to_string(),
            r = r.trim().to_string()
        ))
    }
}

/// Passe une fenêtre en flottant ou en mosaïque.
pub fn set_floating(address: &str, floating: bool) -> Result<(), String> {
    on_window(
        address,
        if floating {
            "hl.dsp.window.float({ action = \"enable\", window = \"address:{w}\" })"
        } else {
            "hl.dsp.window.float({ action = \"disable\", window = \"address:{w}\" })"
        },
    )
}

/// Place une fenêtre flottante (position puis taille).
pub fn place(address: &str, rect: Rect) -> Result<(), String> {
    let (x, y, w, h) = rect;
    on_window(
        address,
        &format!(
            "hl.dsp.window.resize({{ x = {w}, y = {h}, relative = false, window = \"address:{{w}}\" }})"
        ),
    )?;
    on_window(
        address,
        &format!(
            "hl.dsp.window.move({{ x = {x}, y = {y}, relative = false, window = \"address:{{w}}\" }})"
        ),
    )
}

/// Plein bureau : parmi ces fenêtres, celles en mosaïque passent en
/// flottant, à la taille donnée et centrées. Rend celles qui ont changé
/// (les déjà flottantes gardent leur place).
pub fn float_windows(addresses: &[String], size: (u32, u32)) -> Result<Vec<String>, String> {
    let clients = ipc::query_json("clients").map_err(|e| e.to_string())?;
    let tiled: Vec<String> = geometries(&clients)
        .into_iter()
        .filter(|g| !g.floating && addresses.contains(&g.address))
        .map(|g| g.address)
        .collect();
    let mut errors = Vec::new();
    for a in &tiled {
        let steps = set_floating(a, true)
            .and_then(|()| {
                on_window(
                    a,
                    &format!(
                        "hl.dsp.window.resize({{ x = {}, y = {}, relative = false, window = \"address:{{w}}\" }})",
                        size.0, size.1
                    ),
                )
            })
            .and_then(|()| on_window(a, "hl.dsp.window.center({ window = \"address:{w}\" })"));
        if let Err(e) = steps {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(tiled)
    } else {
        Err(errors.join(" ; "))
    }
}

/// Sortie du plein bureau : ces fenêtres repassent en mosaïque (une fenêtre
/// fermée entre-temps est ignorée).
pub fn tile_windows(addresses: &[String]) -> Result<(), String> {
    if addresses.is_empty() {
        return Ok(());
    }
    let clients = ipc::query_json("clients").map_err(|e| e.to_string())?;
    let errors: Vec<String> = floating_among(&clients, addresses)
        .iter()
        .filter_map(|a| set_floating(a, false).err())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(" ; "))
    }
}

/// Bureau affiché (ou hyprdesk qui s'arrête) : les fenêtres de `DESK`
/// reviennent sur le workspace ordinaire de l'écran qui a le focus.
pub fn unpark_desk_windows() -> Result<(), String> {
    let clients = ipc::query_json("clients").map_err(|e| e.to_string())?;
    let parked = windows_on(&clients, DESK);
    if parked.is_empty() {
        return Ok(());
    }
    let target = focused_workspace();
    let errors: Vec<String> = parked
        .iter()
        .filter_map(|a| move_window(a, &target).err().map(|e| e.to_string()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(" ; "))
    }
}

/// Fenêtre mise de côté : où la rendre.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stashed {
    pub address: String,
    pub workspace: String,
    pub monitor: String,
}

/// Déplace une fenêtre sans la suivre.
pub fn move_window(address: &str, workspace: &str) -> Result<(), ipc::IpcError> {
    let ws = workspace_dispatch_arg(workspace);
    let r = ipc::dispatch(&format!(
        "hl.dsp.window.move({{ workspace = {ws}, follow = false, window = \"address:{}\" }})",
        ipc::lua_escape(address)
    ))?;
    if r.trim() != "ok" {
        return Err(ipc::IpcError::Io(std::io::Error::other(crate::tr!(
            "déplacement de {address} vers {workspace} : {r}",
            address = address,
            workspace = workspace,
            r = r
        ))));
    }
    Ok(())
}

/// Renomme un workspace pour la session (Hyprland ne le garde pas au
/// redémarrage). Le dispatcher typé de Hyprland 0.57 : `hl.dsp.workspace.rename`
/// (le passage brut `renameworkspace` via `hl.dsp.global` ne s'exécute pas).
pub fn rename_workspace(id: i64, name: &str) -> Result<(), ipc::IpcError> {
    let r = ipc::dispatch(&format!(
        "hl.dsp.workspace.rename({{ workspace = {}, name = \"{}\" }})",
        id,
        ipc::lua_escape(name)
    ))?;
    if r.trim() != "ok" {
        return Err(ipc::IpcError::Io(std::io::Error::other(crate::tr!(
            "renommage du workspace {id} : {r}",
            id = id,
            r = r
        ))));
    }
    Ok(())
}

/// Fichier qui garde la liste, pour rendre les fenêtres même après un arrêt brutal.
fn stash_path() -> Option<std::path::PathBuf> {
    ipc::instance_dir()
        .ok()
        .map(|d| d.join("hyprdesk-stash.json"))
}

pub fn save_stash(list: &[Stashed]) -> std::io::Result<()> {
    let p = stash_path().ok_or_else(|| std::io::Error::other("instance Hyprland indisponible"))?;
    if list.is_empty() {
        std::fs::remove_file(&p).or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(e)
            }
        })
    } else {
        let contents = serde_json::to_vec(list).map_err(std::io::Error::other)?;
        crate::storage::write(&p, &contents, crate::storage::revision())
    }
}

pub fn load_stash() -> std::io::Result<Vec<Stashed>> {
    let p = stash_path().ok_or_else(|| std::io::Error::other("instance Hyprland indisponible"))?;
    match std::fs::read(&p) {
        Ok(s) => serde_json::from_slice(&s).map_err(std::io::Error::other),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Rend les fenêtres ; seules les entrées non résolues restent dans le journal.
pub fn restore(list: &[Stashed]) -> Vec<Stashed> {
    let list = repair_homes(list, || {
        Ok((ipc::query_json("monitors")?, ipc::query_json("workspaces")?))
    });
    restore_using(
        &list,
        || ipc::query_json("clients").map_err(|e| e.to_string()),
        |s| move_window(&s.address, &s.workspace).map_err(|e| e.to_string()),
    )
}

/// Entrées dont l'adresse de retour est le workspace de mise de côté lui-même
/// (journaux écrits par les versions précédentes) : retour sur un workspace
/// ordinaire de leur écran, sinon elles resteraient cachées pour toujours.
fn repair_homes(
    list: &[Stashed],
    query: impl FnOnce() -> Result<(Value, Value), ipc::IpcError>,
) -> Vec<Stashed> {
    if !list
        .iter()
        .any(|s| s.workspace == STASH || s.workspace.is_empty())
    {
        return list.to_vec();
    }
    let (monitors, workspaces) = query().unwrap_or((Value::Null, Value::Null));
    list.iter()
        .cloned()
        .map(|mut s| {
            if s.workspace == STASH || s.workspace.is_empty() {
                s.workspace = regular_workspace(&monitors, &workspaces, &s.monitor);
            }
            s
        })
        .collect()
}

/// Workspace ordinaire d'un écran : celui qu'il affiche, sinon le plus petit
/// des siens, sinon le 1.
pub fn regular_workspace(monitors: &Value, workspaces: &Value, monitor: &str) -> String {
    let ordinary = |w: &Value| {
        w["id"].as_i64().is_some_and(|id| id >= 1)
            && w["name"]
                .as_str()
                .is_some_and(|n| !n.starts_with("special:"))
    };
    let shown = monitors.as_array().and_then(|l| {
        l.iter()
            .find(|m| m["name"].as_str() == Some(monitor))
            .map(|m| &m["activeWorkspace"])
            .filter(|w| ordinary(w))
            .and_then(|w| w["name"].as_str())
    });
    let own = || {
        workspaces.as_array().and_then(|l| {
            l.iter()
                .filter(|w| w["monitor"].as_str() == Some(monitor) && ordinary(w))
                .min_by_key(|w| w["id"].as_i64())
                .and_then(|w| w["name"].as_str())
        })
    };
    shown.or_else(own).unwrap_or("1").to_string()
}

fn restore_using(
    list: &[Stashed],
    mut query: impl FnMut() -> Result<Value, String>,
    mut move_back: impl FnMut(&Stashed) -> Result<(), String>,
) -> Vec<Stashed> {
    if list.is_empty() {
        return Vec::new();
    }
    let clients = match query() {
        Ok(Value::Array(a)) => a,
        other => {
            tracing::warn!(
                "{}",
                crate::tr!(
                    "restitution reportée : réponse clients invalide ({other})",
                    other = format!("{other:?}")
                )
            );
            return list.to_vec();
        }
    };
    let stashed = |s: &Stashed, clients: &[Value]| {
        clients.iter().any(|c| {
            c["address"].as_str() == Some(&s.address)
                && c["workspace"]["name"].as_str() == Some(STASH)
        })
    };
    let targets: Vec<_> = list.iter().filter(|s| stashed(s, &clients)).collect();
    if targets.is_empty() {
        return Vec::new();
    }
    for s in &targets {
        if let Err(e) = move_back(s) {
            tracing::warn!(
                "{}",
                crate::tr!("restitution de {p0} : {e}", p0 = s.address, e = e)
            );
        }
    }
    match query() {
        Ok(Value::Array(after)) => targets
            .into_iter()
            .filter(|s| stashed(s, &after))
            .cloned()
            .collect(),
        _ => list.to_vec(),
    }
}

/// Au démarrage : rend les fenêtres d'une session précédente interrompue.
pub fn restore_leftover_stash() {
    let list = match load_stash() {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!("{}", crate::tr!("journal conservé : {e}", e = e));
            return;
        }
    };
    if !list.is_empty() {
        let pending = restore(&list);
        tracing::info!(
            "{}",
            crate::tr!(
                "{p0} fenêtre(s) restituée(s), {p1} en attente",
                p0 = list.len() - pending.len(),
                p1 = pending.len()
            )
        );
        if let Err(e) = save_stash(&pending) {
            tracing::warn!("{}", crate::tr!("journal : {e}", e = e));
        }
    }
}

/// Commande lancée avec les variables du thème (`HYPRDESK_*`, MLG-235) :
/// l'application est un enfant de Hyprland, pas du démon.
fn themed(cmd: &str) -> String {
    format!("{}{cmd}", crate::theme_engine::env::launch_prefix())
}

/// Lance une commande par Hyprland (`hl.dsp.exec_cmd`) : le processus est un
/// enfant du compositeur, pas de hyprdesk, et les règles d'exec s'appliquent.
/// Lance sur un workspace donné (celui de l'écran où l'on a cliqué).
pub fn exec_on(cmd: &str, workspace: &str) -> Result<(), ipc::IpcError> {
    let cmd = &themed(cmd);
    let r = ipc::dispatch(&format!(
        "hl.dsp.exec_cmd(\"{}\", {{ workspace = \"{}\" }})",
        ipc::lua_escape(cmd),
        ipc::lua_escape(workspace)
    ))?;
    if r.trim() != "ok" {
        tracing::warn!(
            "{}",
            crate::tr!(
                "exec « {cmd} » : réponse inattendue de Hyprland : {r}",
                cmd = cmd,
                r = r
            )
        );
    }
    Ok(())
}

pub fn exec(cmd: &str) -> Result<(), ipc::IpcError> {
    let cmd = &themed(cmd);
    let r = ipc::dispatch(&format!("hl.dsp.exec_cmd(\"{}\")", ipc::lua_escape(cmd)))?;
    if r.trim() != "ok" {
        tracing::warn!(
            "{}",
            crate::tr!(
                "exec « {cmd} » : réponse inattendue de Hyprland : {r}",
                cmd = cmd,
                r = r
            )
        );
    }
    Ok(())
}

/// Lance une commande dont la fenêtre s'ouvre flottante et centrée, de la
/// taille donnée : le bureau reste visible autour.
/// Lance flottant et centré ; `workspace` : celui où la fenêtre s'ouvre
/// (sinon celui qui a le focus).
pub fn exec_floating(
    cmd: &str,
    w: u32,
    h: u32,
    workspace: Option<&str>,
) -> Result<(), ipc::IpcError> {
    let ws = workspace
        .map(|w| format!("workspace = \"{}\", ", ipc::lua_escape(w)))
        .unwrap_or_default();
    let cmd = &themed(cmd);
    let r = ipc::dispatch(&format!(
        "hl.dsp.exec_cmd(\"{}\", {{ {ws}float = true, center = true, size = \"{w} {h}\" }})",
        ipc::lua_escape(cmd)
    ))?;
    if r.trim() != "ok" {
        tracing::warn!(
            "{}",
            crate::tr!(
                "exec « {cmd} » : réponse inattendue de Hyprland : {r}",
                cmd = cmd,
                r = r
            )
        );
    }
    Ok(())
}

/// Notification Hyprland (icône 0 avertissement, 1 info, 3 ok, 4 erreur).
pub fn notify(icon: u8, ms: u32, msg: &str) {
    type Notice = (u8, u32, String);
    static NOTICES: std::sync::LazyLock<std::sync::mpsc::SyncSender<Notice>> =
        std::sync::LazyLock::new(|| {
            let (tx, rx) = std::sync::mpsc::sync_channel::<Notice>(16);
            std::thread::Builder::new()
                .name("hyprdesk-notifications".into())
                .spawn(move || {
                    while let Ok((icon, ms, msg)) = rx.recv() {
                        if let Err(e) = ipc::notify(icon, ms, "0", &msg) {
                            tracing::warn!("{}", crate::tr!("notification Hyprland : {e}", e = e));
                        }
                    }
                })
                .expect("thread des notifications");
            tx
        });
    if let Err(e) = NOTICES.try_send((icon, ms, msg.to_string())) {
        tracing::warn!("{}", crate::tr!("notification reportée : {e}", e = e));
    }
}

#[cfg(test)]
mod tests {
    /// Vrai plein écran (mode 2, ou 3 : agrandie puis plein écran) ; une
    /// fenêtre seulement agrandie (mode 1) n'en est pas un (HIG PAT-01).
    #[test]
    fn plein_ecran_distingue_de_la_fenetre_agrandie() {
        let v = serde_json::json!([
            {"address": "0x1", "class": "mpv", "workspace": {"name": "1"}, "fullscreen": 2},
            {"address": "0x2", "class": "foot", "workspace": {"name": "1"}, "fullscreen": 1},
            {"address": "0x3", "class": "kitty", "workspace": {"name": "2"}, "fullscreen": 3},
            {"address": "0x4", "class": "zed", "workspace": {"name": "2"}}
        ]);
        let full: Vec<bool> = super::parse_window_geometries(&v)
            .iter()
            .map(|g| g.fullscreen)
            .collect();
        assert_eq!(full, [true, false, true, false]);
    }

    fn float_win(address: &str, ws: &str, rect: Rect, floating: bool) -> Geometry {
        Geometry {
            address: address.into(),
            workspace: ws.into(),
            class: "brave".into(),
            floating,
            rect,
        }
    }

    #[test]
    fn flottantes_ramenees_hors_du_tiroir_puis_rendues() {
        // DP-1 : 2560 de large, tiroir des réglages de 936 à droite.
        let open = [DrawerScreen {
            workspace: "1".into(),
            rect: (0, 0, 2560, 1440),
            right: 936,
        }];
        let shut = [DrawerScreen {
            right: 0,
            ..open[0].clone()
        }];
        let wins = [
            float_win("a", "1", (400, 60, 1380, 950), true), // déborde sur le tiroir
            float_win("b", "1", (100, 100, 800, 600), true), // tient déjà
            float_win("c", "1", (0, 0, 1600, 1400), false),  // mosaïque : gaps_out
            float_win("d", "2", (1800, 0, 600, 600), true),  // autre workspace
            float_win("e", "1", (300, 0, 2400, 900), true),  // plus large que la place
        ];
        let (moves, held) = plan_drawer_floats(&wins, &open, (7, 7), &FloatHeld::new(), "lyra");
        // limite : 2560 - 936 - 7 = 1617 ; a recule, e rétrécit à 1610 depuis 7.
        assert_eq!(
            moves,
            vec![
                ("a".to_string(), (237, 60, 1380, 950)),
                ("e".to_string(), (7, 0, 1610, 900))
            ]
        );
        assert_eq!(held.len(), 2);
        // Tiroir refermé : a est restée où on l'a mise, elle revient ; e a été
        // déplacée à la main, elle reste.
        let after = [
            float_win("a", "1", (237, 60, 1380, 950), true),
            float_win("e", "1", (50, 50, 1610, 900), true),
        ];
        let (moves, held) = plan_drawer_floats(&after, &shut, (7, 7), &held, "lyra");
        assert_eq!(moves, vec![("a".to_string(), (400, 60, 1380, 950))]);
        assert!(held.is_empty());
    }

    #[test]
    fn fenetre_de_lyra_jamais_recadree() {
        let open = [DrawerScreen {
            workspace: "1".into(),
            rect: (0, 0, 2560, 1440),
            right: 600,
        }];
        let mut w = float_win("l", "1", (1960, 0, 600, 1440), true);
        w.class = "lyra".into();
        let (moves, _) = plan_drawer_floats(&[w], &open, (0, 0), &FloatHeld::new(), "lyra");
        assert!(moves.is_empty());
    }

    #[test]
    fn rafale_relue_une_fois() {
        let mut p = Pending::default();
        for l in [
            "workspace>>2",
            "workspacev2>>2,2",
            "focusedmon>>DP-1,2",
            "focusedmonv2>>DP-1,2",
            "activewindow>>kitty,~",
            "activewindowv2>>abc",
            "activewindow>>firefox,Accueil",
        ] {
            p.note(l);
        }
        assert!(p.monitors && p.workspaces);
        assert!(!p.clients && !p.reloaded);
        assert_eq!(
            p.active,
            Some(Some(("firefox".into(), "Accueil".into()))),
            "la dernière fenêtre active de la rafale"
        );
        let mut q = Pending::default();
        q.note("fullscreen>>1");
        assert!(q.workspaces && !q.monitors);
        let mut r = Pending::default();
        r.note("windowtitlev2>>abc,titre");
        assert!(r.is_empty());
    }

    use super::*;

    #[test]
    fn fenetres_du_bureau() {
        let clients = serde_json::json!([
            { "address": "0x1", "floating": true, "workspace": { "name": "2" } },
            { "address": "0x2", "floating": false, "workspace": { "name": "2" } },
            { "address": "0x3", "floating": true, "workspace": { "name": DESK } },
            { "address": "0x4", "floating": true, "workspace": { "name": "2" } },
        ]);
        // Seules les flottantes ouvertes sur le bureau partent avec lui.
        let ours = vec!["0x1".to_string(), "0x2".to_string()];
        assert_eq!(floating_among(&clients, &ours), vec!["0x1".to_string()]);
        assert_eq!(windows_on(&clients, DESK), vec!["0x3".to_string()]);
        assert!(DESK.starts_with("special:") && DESK != STASH);
    }

    #[test]
    fn version_de_hyprland() {
        assert_eq!(parse_version("v0.57.2"), Some((0, 57, 2)));
        assert_eq!(parse_version("0.56.1"), Some((0, 56, 1)));
        assert_eq!(parse_version("v0.58.0-12-gabc"), Some((0, 58, 0)));
        assert_eq!(parse_version("v1.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("inconnue"), None);
        assert!((0, 56) < MIN_HYPRLAND && (0, 57) >= MIN_HYPRLAND);
    }

    /// Plein bureau : une fenêtre centrée, plusieurs en grille, sans débordement.
    #[test]
    fn disposition_flottante() {
        let area = (100, 50, 1000, 600);
        assert_eq!(arrange(0, area, 10), vec![]);
        assert_eq!(arrange(1, area, 10), vec![(300, 140, 600, 420)]);
        let two = arrange(2, area, 10);
        assert_eq!(two, vec![(100, 50, 495, 600), (605, 50, 495, 600)]);
        // Trois : deux colonnes, la dernière rangée (une fenêtre) centrée.
        let three = arrange(3, area, 10);
        assert_eq!(three.len(), 3);
        assert_eq!(three[2].0, 100 + (495 + 10) / 2);
        for n in 1..=12 {
            for (x, y, w, h) in arrange(n, area, 10) {
                assert!(
                    x >= 100 && y >= 50 && x + w <= 1100 && y + h <= 650,
                    "{n} : {x},{y},{w},{h}"
                );
                assert!(w > 0 && h > 0);
            }
        }
        let g = geometries(&serde_json::json!([
            { "address": "0x1", "workspace": { "name": "2" }, "floating": true, "at": [10, 20], "size": [300, 200] },
            { "address": "0x2", "workspace": { "name": "2" }, "floating": false, "at": [0, 0], "size": [1, 1], "hidden": true },
        ]));
        assert_eq!(
            g,
            vec![Geometry {
                address: "0x1".into(),
                workspace: "2".into(),
                class: String::new(),
                floating: true,
                rect: (10, 20, 300, 200)
            }]
        );
    }

    #[test]
    fn marges_du_bureau_avec_les_fenetres() {
        let lua = build_gaps_lua(&[("DP-1".into(), [0, 290, 98, 130])], [5, 5, 5, 5]);
        assert!(lua.contains(
            r#"hl.workspace_rule({ workspace = "m[DP-1]", gaps_out = { top = 5, right = 295, bottom = 103, left = 135 } })"#
        ));
        // Retrait : seulement la désactivation des anciennes règles.
        let off = build_gaps_lua(&[], [0; 4]);
        assert!(off.contains("set_enabled(false)") && !off.contains("hl.workspace_rule"));
    }

    fn fs(address: &str, monitor: &str, internal: i64, client: i64) -> FsWindow {
        FsWindow {
            address: address.into(),
            monitor: monitor.into(),
            internal,
            client,
        }
    }

    #[test]
    fn plein_ecran_hors_des_tiroirs() {
        use std::collections::HashSet;
        let drawers = vec!["DP-1".to_string()];
        let none = HashSet::new();
        // Plein écran sur l'écran du tiroir : agrandi pour Hyprland, plein
        // écran pour l'application ; ailleurs et agrandi : intact.
        let w = [
            fs("0xa", "DP-1", 2, 2),
            fs("0xb", "DP-2", 2, 2),
            fs("0xc", "DP-1", 1, 0),
        ];
        let (set, added, dropped) = plan_drawer_fullscreen(&w, &drawers, &none);
        assert_eq!(set, vec![("0xa".to_string(), 1, 2)]);
        assert_eq!(added, vec!["0xa".to_string()]);
        assert!(dropped.is_empty());

        // Retenue et toujours sous le tiroir : rien à faire.
        let held: HashSet<String> = ["0xa".to_string()].into();
        let (set, added, dropped) =
            plan_drawer_fullscreen(&[fs("0xa", "DP-1", 1, 2)], &drawers, &held);
        assert!(set.is_empty() && added.is_empty() && dropped.is_empty());

        // Tiroir refermé : plein écran rendu.
        let (set, _, dropped) = plan_drawer_fullscreen(&[fs("0xa", "DP-1", 1, 2)], &[], &held);
        assert_eq!(set, vec![("0xa".to_string(), 2, 2)]);
        assert_eq!(dropped, vec!["0xa".to_string()]);

        // Super+F sur la fenêtre retenue (Hyprland la remet en plein écran) :
        // c'est une sortie du plein écran.
        let (set, added, dropped) =
            plan_drawer_fullscreen(&[fs("0xa", "DP-1", 2, 2)], &drawers, &held);
        assert_eq!(set, vec![("0xa".to_string(), 0, 0)]);
        assert!(added.is_empty());
        assert_eq!(dropped, vec!["0xa".to_string()]);

        // Sortie par l'application, ou fenêtre fermée : relâchée, sans retour.
        let (set, _, dropped) = plan_drawer_fullscreen(&[fs("0xa", "DP-1", 1, 0)], &[], &held);
        assert!(set.is_empty());
        assert_eq!(dropped, vec!["0xa".to_string()]);
        let (set, _, dropped) = plan_drawer_fullscreen(&[], &drawers, &held);
        assert!(set.is_empty());
        assert_eq!(dropped, vec!["0xa".to_string()]);
    }

    /// État relevé chez l'utilisateur : HDMI-A-2 affiche un workspace ORDINAIRE
    /// nommé « special:hyprdesk », le journal y renvoie trois fenêtres.
    fn etat_bloque() -> (Value, Value) {
        let monitors = serde_json::json!([
            { "name": "HDMI-A-2", "width": 2560, "height": 1440, "focused": false,
              "activeWorkspace": { "name": "special:hyprdesk", "type": "normal" },
              "specialWorkspace": { "name": "special:hyprdesk", "type": "special" } },
            { "name": "DP-1", "width": 1440, "height": 2560, "focused": true,
              "activeWorkspace": { "id": 6, "name": "6" },
              "specialWorkspace": { "name": "" } }
        ]);
        let workspaces = serde_json::json!([
            { "id": 6, "name": "6", "monitor": "DP-1" },
            { "id": null, "name": "special:hyprdesk", "monitor": "HDMI-A-2" },
            { "id": 3, "name": "3", "monitor": "HDMI-A-2" },
            { "id": 1, "name": "1", "monitor": "HDMI-A-2" }
        ]);
        (monitors, workspaces)
    }

    #[test]
    fn le_workspace_de_cote_n_est_jamais_celui_de_l_ecran() {
        let (monitors, _) = etat_bloque();
        let list = parse_monitors(&monitors);
        assert_eq!(list[0].workspace, "");
        assert_eq!(list[1].workspace, "6");
        // Un autre workspace spécial ouvert reste, lui, celui de l'écran.
        let magic = serde_json::json!([{ "name": "A", "width": 10, "height": 10,
            "activeWorkspace": { "id": 2, "name": "2" },
            "specialWorkspace": { "name": "special:magic" } }]);
        assert_eq!(parse_monitors(&magic)[0].workspace, "special:magic");
    }

    #[test]
    fn adresse_de_retour_invalide_reparee() {
        let (monitors, workspaces) = etat_bloque();
        assert_eq!(regular_workspace(&monitors, &workspaces, "HDMI-A-2"), "1");
        assert_eq!(regular_workspace(&monitors, &workspaces, "DP-1"), "6");
        assert_eq!(regular_workspace(&Value::Null, &Value::Null, "X"), "1");
        let stuck = |a: &str, ws: &str, m: &str| Stashed {
            address: a.into(),
            workspace: ws.into(),
            monitor: m.into(),
        };
        let list = vec![
            stuck("0x1", STASH, "HDMI-A-2"),
            stuck("0x2", "4", "HDMI-A-2"),
            stuck("0x3", "", "DP-1"),
        ];
        let fixed = repair_homes(&list, || Ok(etat_bloque()));
        let homes: Vec<&str> = fixed.iter().map(|s| s.workspace.as_str()).collect();
        assert_eq!(homes, ["1", "4", "6"]);
        // Journal sain : aucune requête.
        let sane = vec![stuck("0x2", "4", "HDMI-A-2")];
        assert_eq!(repair_homes(&sane, || panic!("requête inutile")), sane);
    }

    #[test]
    fn regles_de_workspace() {
        use crate::config::AppMatch;
        assert_eq!(
            class_regex(&AppMatch::App("org.telegram.desktop".into()), None),
            r"^(org\.telegram\.desktop)$"
        );
        assert_eq!(
            class_regex(&AppMatch::App("code".into()), Some("Code")),
            "^(code|Code)$"
        );
        assert_eq!(
            class_regex(&AppMatch::Class("^(a|b)$".into()), None),
            "^(a|b)$"
        );
        let lua = build_rules_lua(
            Some(3),
            &[
                ("1".into(), "HDMI-A-2".into(), true),
                ("6".into(), "DP-1".into(), true),
            ],
            &[
                ("^(firefox)$".into(), "2".into(), false),
                ("^(k\"itty)$".into(), "4".into(), true),
            ],
        );
        assert_eq!(lua.matches("persistent = true").count(), 3);
        assert!(lua.contains(r#"workspace = "3", persistent = true }"#));
        assert!(lua.contains(
            r#"workspace = "1", persistent = true, monitor = "HDMI-A-2", default = true"#
        ));
        // Placé mais pas permanent : la règle s'applique à sa création.
        assert!(lua.contains(r#"{ workspace = "6", monitor = "DP-1", default = true }"#));
        assert!(lua.contains(r#"class = "^(firefox)$" }, workspace = "2" }"#));
        assert!(lua.contains(r#"class = "^(k\"itty)$" }, workspace = "4 silent" }"#));
        // Retrait : seulement la désactivation des anciennes règles.
        let off = build_rules_lua(None, &[], &[]);
        assert!(!off.contains("hl.workspace_rule") && !off.contains("hl.window_rule"));
        assert!(off.contains("set_enabled(false)"));
    }

    /// Les permanents sans écran choisi vont sur le principal, dans leur
    /// règle : poser les règles le focus sur un écran secondaire (relecture
    /// de la config) ne les y déplace plus.
    #[test]
    fn permanents_sur_l_ecran_principal() {
        let mut placement: Vec<Placement> = vec![("2".into(), "DP-2".into(), true)];
        place_permanent(&mut placement, 3, "DP-1");
        assert_eq!(
            placement,
            vec![
                ("2".into(), "DP-2".into(), true),
                ("1".into(), "DP-1".into(), false),
                ("3".into(), "DP-1".into(), false),
            ]
        );
        let lua = build_rules_lua(Some(3), &placement, &[]);
        assert!(lua.contains(r#"workspace = "1", persistent = true, monitor = "DP-1" }"#));
        assert!(
            lua.contains(
                r#"workspace = "2", persistent = true, monitor = "DP-2", default = true }"#
            )
        );
        let mut none = Vec::new();
        place_permanent(&mut none, 0, "DP-1");
        assert!(none.is_empty());
    }

    #[test]
    fn workspaces_d_un_ecran_absent() {
        let rules = serde_json::json!([
            { "workspaceString": "1", "monitor": "HDMI-A-1", "persistent": true },
            { "workspaceString": "2", "monitor": "HDMI-A-1", "enabled": false },
            { "workspaceString": "3", "monitor": "HDMI-A-1" },
            { "workspaceString": "3", "monitor": "HDMI-A-1" },
            { "workspaceString": "4", "monitor": "eDP-1" },
            { "workspaceString": "5" },
            { "workspaceString": "m[HDMI-A-1]", "monitor": "HDMI-A-1" },
            { "workspaceString": "6", "monitor": "desc:Dell U2720" },
            { "workspaceString": "7", "monitor": "DP-3" },
        ]);
        let live = vec!["eDP-1".to_string()];
        let placed = vec![("7".to_string(), "eDP-1".to_string(), false)];
        assert_eq!(orphan_workspaces(&rules, &live, &placed), vec!["1", "3"]);
    }

    #[test]
    fn restoration_retains_failures_and_verifies_the_destination() {
        let list = vec![Stashed {
            address: "0x1".into(),
            workspace: "1".into(),
            monitor: "DP-1".into(),
        }];
        let still_hidden = serde_json::json!([{ "address":"0x1", "workspace":{"name":STASH} }]);
        assert_eq!(
            restore_using(
                &list,
                || Ok(Value::String("bad reply".into())),
                |_| panic!("aucun déplacement sans état valide")
            ),
            list
        );
        assert_eq!(
            restore_using(&list, || Ok(still_hidden.clone()), |_| Ok(())),
            list,
            "un accusé IPC seul ne prouve pas la restitution"
        );
        let mut queries = [
            still_hidden,
            serde_json::json!([{ "address":"0x1", "workspace":{"name":"1"} }]),
        ]
        .into_iter();
        assert!(restore_using(&list, || Ok(queries.next().unwrap()), |_| Ok(())).is_empty());
        assert!(
            restore_using(
                &list,
                || Ok(serde_json::json!([{ "address":"0x1", "workspace":{"name":"7"} }])),
                |_| panic!("respecter un déplacement manuel")
            )
            .is_empty()
        );
    }

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
        assert_eq!(
            (
                m[0].width,
                m[0].height,
                m[0].workspace.as_str(),
                m[0].focused
            ),
            (2560, 1440, "2", true)
        );
        assert_eq!((m[1].width, m[1].height), (1152, 2048));
        assert_eq!(m[1].workspace, "special:magic");
        // Le mode du bureau suit le workspace normal, sous le scratchpad.
        assert_eq!(m[1].mode_workspace(), "6");
        assert_eq!(m[0].mode_workspace(), "2");
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
        // Nos propres raccourcis et ceux d'une autre submap ne sont pas des conflits.
        let f = crate::keymap::foreign(&binds);
        assert!(
            f.iter()
                .any(|f| f.combo == (64, "q".into()) && f.what == "close")
        );
        assert!(!f.iter().any(|f| f.combo == (64, "d".into())));
        assert_eq!(
            universal_binds(&binds),
            vec!["SUPER + eacute → focus".to_string()]
        );
    }

    #[test]
    fn touches_multimedia() {
        let k = crate::config::Keys::default();
        let raise = r#"hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("'/bin/hyprdesk' osd volume +5"), { locked = true, repeating = true"#;
        // Déjà liée par hyprland.lua : seulement dans la submap.
        let lua = build_keys_lua(
            &k,
            &[],
            "/bin/hyprdesk",
            &crate::keymap::Plan::skipping(&["XF86AudioRaiseVolume"]),
            "SUPER",
        );
        let (main, submap) = lua.split_once("hl.define_submap").unwrap();
        assert!(!main.contains(raise) && submap.contains(raise), "{lua}");
        assert!(main.contains(
            r#"hl.bind("XF86MonBrightnessUp", hl.dsp.exec_cmd("'/bin/hyprdesk' osd brightness +5")"#
        ));
        assert!(submap.contains(r#"hl.bind("XF86AudioPlay", hl.dsp.exec_cmd("'/bin/hyprdesk' media play-pause"), { locked = true, repeating = false"#));
        // Coupées : aucune.
        let off = crate::config::Keys { media: false, ..k };
        assert!(
            !build_keys_lua(&off, &[], "/bin/hyprdesk", &Default::default(), "SUPER")
                .contains("XF86")
        );
        for (key, _, _) in MEDIA_KEYS {
            assert!(parse_combo(key).is_some(), "{key}");
        }
    }

    #[test]
    fn lua_des_raccourcis() {
        let k = crate::config::Keys {
            toggle: "SUPER + D".into(),
            launcher: "SUPER + ALT + D".into(),
            hide: "SUPER + CTRL + D".into(),
            settings: "SUPER + SHIFT + D".into(),
            lock: "SUPER + L".into(),
            leave: "SUPER + Escape".into(),
            switch: "ALT + Tab".into(),
            search: "SUPER + Space".into(),
            overview: "SUPER + Tab".into(),
            media: true,
            ..Default::default()
        };
        let mut plan = crate::keymap::Plan::skipping(&["SUPER + SHIFT + D"]);
        plan.take = vec!["SUPER + L".into()];
        let lua = build_keys_lua(&k, &["hyprdesk-bar"], "/bin/hyprdesk", &plan, "SUPER");
        assert!(
            lua.contains(r#"keep(hl.bind("SUPER + D", hl.dsp.exec_cmd("'/bin/hyprdesk' cycle")"#),
            "{lua}"
        );
        // Sélecteur de fenêtres, et en arrière avec Maj.
        assert!(lua.contains(r#"hl.bind("ALT + Tab", hl.dsp.exec_cmd("'/bin/hyprdesk' switch")"#));
        assert!(lua.contains(
            r#"hl.bind("ALT + SHIFT + Tab", hl.dsp.exec_cmd("'/bin/hyprdesk' switch prev")"#
        ));
        // En conflit hors de la submap : pas installé là, mais bien dans la
        // submap, où les raccourcis de Hyprland sont suspendus.
        let (main, submap) = lua.split_once("hl.define_submap").unwrap();
        assert!(
            !main.contains("SUPER + SHIFT + D"),
            "un raccourci en conflit n'est pas installé : {lua}"
        );
        assert!(
            submap.contains(
                r#"hl.bind("SUPER + SHIFT + D", hl.dsp.exec_cmd("'/bin/hyprdesk' toggle-settings")"#
            ),
            "{lua}"
        );
        // Verrouiller : hors de la submap et dans la submap (sans la quitter).
        assert!(
            main.contains(r#"keep(hl.bind("SUPER + L", hl.dsp.exec_cmd("'/bin/hyprdesk' lock")"#),
            "{lua}"
        );
        assert!(
            submap.contains(r#"keep(hl.bind("SUPER + L", hl.dsp.exec_cmd("'/bin/hyprdesk' lock")"#),
            "{lua}"
        );
        assert!(
            lua.contains(r#"namespace = "^hyprdesk-(bar)$" }, blur = true"#),
            "seules les couches floutées par le thème : {lua}"
        );
        assert!(lua.contains(r#"hl.define_submap("hyprdesk", function()"#));
        assert!(
            lua.contains(r#"keep(hl.bind("SUPER + Escape", leave("'/bin/hyprdesk' hide")"#),
            "{lua}"
        );
        // Un raccourci par mode, partout ; dans la submap aussi.
        assert!(lua.contains(
            r#"hl.bind("SUPER + ALT + D", hl.dsp.exec_cmd("'/bin/hyprdesk' toggle-launcher")"#
        ));
        assert!(
            lua.contains(r#"hl.bind("SUPER + CTRL + D", hl.dsp.exec_cmd("'/bin/hyprdesk' hide")"#)
        );
        assert!(
            lua.contains(r#"hl.bind("SUPER + ALT + D", leave("'/bin/hyprdesk' launcher-only")"#)
        );
        assert!(lua.contains(r#"hl.bind("SUPER + D", leave("'/bin/hyprdesk' hide")"#));
        // Verrouillage réglé : la combinaison de hyprland.lua est libérée, puis liée.
        assert!(
            lua.contains(r#"local taken = { "SUPER + L" }"#)
                && lua.contains("hl.unbind(k)")
                && lua.contains(r#"hl.bind("SUPER + L", hl.dsp.exec_cmd("'/bin/hyprdesk' lock")"#),
            "{lua}"
        );
        // Non réglé : rien de lié ; s'il l'était, Hyprland relit sa config.
        let mut none = k.clone();
        none.lock.clear();
        let lua = build_keys_lua(&none, &[], "/bin/hyprdesk", &Default::default(), "SUPER");
        assert!(lua.contains("local taken = {  }") && lua.contains("hyprctl reload"));
        let (main, submap) = lua.split_once("hl.define_submap").unwrap();
        assert!(!main.contains("' lock\")"), "{lua}");
        // … sauf dans la submap, où celui de hyprland.lua est suspendu.
        assert!(
            submap.contains(r#"hl.bind("SUPER + L", hl.dsp.exec_cmd("'/bin/hyprdesk' lock")"#),
            "{lua}"
        );
        // Réglages : hors submap et dans la submap, sans la quitter.
        let lua = build_keys_lua(&k, &[], "/bin/hyprdesk", &Default::default(), "SUPER");
        assert!(!lua.contains("blur = true"), "aucun flou demandé : {lua}");
        assert_eq!(
            lua.matches(
                r#"hl.bind("SUPER + SHIFT + D", hl.dsp.exec_cmd("'/bin/hyprdesk' toggle-settings")"#
            )
            .count(),
            2,
            "{lua}"
        );
    }

    #[test]
    fn raccourcis_de_focus() {
        // HIG ENT-02 : un raccourci par surface du shell, posé hors de la
        // submap comme dedans (la submap reste active).
        let k = crate::config::Keys::default();
        let lua = build_keys_lua(&k, &[], "/bin/hyprdesk", &Default::default(), "SUPER");
        let (main, submap) = lua.split_once("hl.define_submap").unwrap();
        for (combo, arg) in [
            ("CTRL + F2", "focus bar"),
            ("CTRL + F3", "focus dock"),
            ("CTRL + F8", "focus notifications"),
            ("CTRL + F6", "focus settings"),
        ] {
            let bind = format!(r#"hl.bind("{combo}", hl.dsp.exec_cmd("'/bin/hyprdesk' {arg}")"#);
            assert!(main.contains(&bind) && submap.contains(&bind), "{lua}");
        }
    }

    #[test]
    fn raccourci_forcer_a_quitter() {
        // HIG ENT-10 : le modèle Option-Command-Esc, partout — hors de la
        // submap comme dedans. Coupé (`none`) : plus aucune liaison.
        let k = crate::config::Keys::default();
        let lua = build_keys_lua(&k, &[], "/bin/hyprdesk", &Default::default(), "SUPER");
        let (main, submap) = lua.split_once("hl.define_submap").unwrap();
        let bind =
            r#"hl.bind("ALT + SUPER + Escape", hl.dsp.exec_cmd("'/bin/hyprdesk' force-quit")"#;
        assert!(main.contains(bind) && submap.contains(bind), "{lua}");
        let mut k = k;
        k.force_quit.clear();
        let lua = build_keys_lua(&k, &[], "/bin/hyprdesk", &Default::default(), "SUPER");
        assert!(!lua.contains("force-quit"), "{lua}");
    }

    #[test]
    fn raccourci_acces_clavier() {
        // HIG ENT-01 : Control-F1, hors de la submap comme dedans.
        let k = crate::config::Keys::default();
        let lua = build_keys_lua(&k, &[], "/bin/hyprdesk", &Default::default(), "SUPER");
        let (main, submap) = lua.split_once("hl.define_submap").unwrap();
        let bind = r#"hl.bind("CTRL + F1", hl.dsp.exec_cmd("'/bin/hyprdesk' keyboard-access")"#;
        assert!(main.contains(bind) && submap.contains(bind), "{lua}");
        let mut k = k;
        k.keyboard_access.clear();
        let lua = build_keys_lua(&k, &[], "/bin/hyprdesk", &Default::default(), "SUPER");
        assert!(!lua.contains("keyboard-access"), "{lua}");
    }

    #[test]
    fn lua_de_destruction() {
        // HIG ENT-10 : la fenêtre désignée par son adresse, échappée comme
        // pour `close` ; jamais la fenêtre active par défaut.
        assert_eq!(
            kill_window_lua("0x55a1b2c3d4e5"),
            r#"hl.dsp.window.kill({ window = "address:0x55a1b2c3d4e5" })"#
        );
        let l = kill_window_lua("a\"b");
        assert!(
            l.starts_with("hl.dsp.window.kill({ window = \"address:a"),
            "{l}"
        );
        assert!(l.ends_with("\" })") && !l.contains(r#"address:a"b"#), "{l}");
    }

    #[test]
    fn workspaces_et_fenetre_active() {
        let v: Value = serde_json::from_str(
            r#"[{"id":6,"name":"6","monitor":"DP-1","windows":3},{"id":1,"name":"1","monitor":"HDMI-A-2","windows":0},
                {"id":-98,"name":"special:hyprdesk","monitor":"HDMI-A-2","windows":2}]"#,
        )
        .unwrap();
        let w = parse_workspaces(&v);
        assert_eq!(w.iter().map(|w| w.id).collect::<Vec<_>>(), vec![1, 6]);
        assert_eq!(
            parse_active_window("kitty,vim a, b"),
            Some(("kitty".into(), "vim a, b".into()))
        );
        assert_eq!(parse_active_window(","), None);
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
        assert_eq!(
            (c[0].class.as_str(), c[0].workspace.as_str()),
            ("kitty", "2")
        );
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
        let c = ipc::build_dispatch(&format!(
            "hl.dsp.exec_cmd(\"{}\")",
            ipc::lua_escape("xdg-open \"/a b\"")
        ));
        assert_eq!(c, r#"/dispatch hl.dsp.exec_cmd("xdg-open \"/a b\"")"#);
    }

    /// Bureau affiché : la rangée des chiffres change de workspace, avec le
    /// modificateur des raccourcis de workspace de hyprland.lua.
    #[test]
    fn workspaces_bureau_affiche() {
        let binds = serde_json::json!([
            { "submap": "", "dispatcher": "HL.Dispatcher(exec_cmd)", "key": "ampersand", "modmask": 1 },
            { "submap": "", "dispatcher": "HL.Dispatcher(focus)", "key": "eacute", "modmask": 64 },
            { "submap": "", "dispatcher": "HL.Dispatcher(move)", "key": "eacute", "modmask": 72 },
        ]);
        assert_eq!(workspace_modifier(&binds), "SUPER");
        let alt = serde_json::json!([
            { "submap": "", "dispatcher": "HL.Dispatcher(focus)", "key": "code:11", "modmask": 72 },
        ]);
        assert_eq!(workspace_modifier(&alt), "ALT + SUPER");
        assert_eq!(workspace_modifier(&serde_json::json!([])), "SUPER");
        let k = crate::config::Keys::default();
        let lua = build_keys_lua(&k, &[], "/bin/hyprdesk", &Default::default(), "SUPER");
        let sub = &lua[lua.find("hl.define_submap").unwrap()..];
        assert!(sub.contains(r#"hl.bind("SUPER + code:10", hl.dsp.focus({ workspace = 1 })"#));
        assert!(sub.contains(r#"hl.bind("SUPER + code:19", hl.dsp.focus({ workspace = 10 })"#));
        assert!(sub.contains(r#"hl.bind("SUPER + Left", hl.dsp.focus({ workspace = "-1" })"#));
        assert!(sub.contains(r#"hl.bind("SUPER + Right", hl.dsp.focus({ workspace = "+1" })"#));
        assert!(sub.contains("' search\""), "{sub}");
    }

    /// MLG-158 : HDMI-A-2, dédié au 6, rebranché sur 11 (1 à 6 existants,
    /// 7 à 10 liés au DP-1) revient sur 6 avec les fenêtres du 11.
    #[test]
    fn ecran_rebranche_ramene_sur_son_workspace() {
        let mon = |name: &str, ws: &str| {
            parse_monitors(&serde_json::json!([{
                "name": name, "width": 1920, "height": 1080,
                "activeWorkspace": { "name": ws }, "specialWorkspace": { "name": "" }
            }]))
            .remove(0)
        };
        let mut placement: Vec<Placement> = (1..=10)
            .filter(|i| *i != 6)
            .map(|i| (i.to_string(), "DP-1".to_string(), i == 1))
            .collect();
        placement.push(("6".into(), "HDMI-A-2".into(), true));
        let added = vec!["HDMI-A-2".to_string()];
        let mons = [mon("DP-1", "3"), mon("HDMI-A-2", "11")];
        assert_eq!(
            rehome_plan(&placement, 6, &added, &mons),
            vec![Rehome {
                monitor: "HDMI-A-2".into(),
                default: "6".into(),
                current: "11".into(),
                take_windows: true,
            }]
        );
        // Déjà sur un workspace de sa liste, ou écran non branché : rien.
        assert!(rehome_plan(&placement, 6, &added, &[mon("HDMI-A-2", "6")]).is_empty());
        assert!(rehome_plan(&placement, 6, &[], &mons).is_empty());
        // Sur un workspace déclaré ailleurs (le 3) : ses fenêtres restent à lui.
        let p = rehome_plan(&placement, 6, &added, &[mon("HDMI-A-2", "3")]);
        assert!(!p[0].take_windows);
        // Écran sans workspace par défaut : laissé à Hyprland.
        let added = vec!["DP-2".to_string()];
        assert!(rehome_plan(&placement, 6, &added, &[mon("DP-2", "11")]).is_empty());
    }
}
