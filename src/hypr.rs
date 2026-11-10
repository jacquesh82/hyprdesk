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
    /// Zone réservée par les barres (gauche, haut, droite, bas), en pixels logiques.
    pub reserved: (f32, f32, f32, f32),
    /// Position de l'écran dans l'espace global (coordonnées logiques).
    pub x: f32,
    pub y: f32,
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

/// Va au workspace indiqué (nom ou numéro) ; `"+1"` / `"-1"` : relatif.
pub fn focus_workspace(target: &str) -> Result<(), ipc::IpcError> {
    let arg = match target.parse::<i64>() {
        Ok(n) if !target.starts_with('+') && !target.starts_with('-') => n.to_string(),
        _ => format!("\"{}\"", ipc::lua_escape(target)),
    };
    let r = ipc::dispatch(&format!("hl.dsp.focus({{ workspace = {arg} }})"))?;
    if r.trim() != "ok" {
        tracing::warn!("workspace {target} : {r}");
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
    let r = ipc::dispatch(&format!(
        "hl.dsp.focus({{ window = \"address:{}\" }})",
        ipc::lua_escape(address)
    ))?;
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
    Workspaces(Vec<Workspace>),
    ActiveWindow(Option<(String, String)>),
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
                name,
                width: (w / scale).round() as u32,
                height: (h / scale).round() as u32,
                workspace,
                focused: m["focused"].as_bool().unwrap_or(false),
                reserved: (r(0), r(1), r(2), r(3)),
                x: m["x"].as_f64().unwrap_or(0.0) as f32,
                y: m["y"].as_f64().unwrap_or(0.0) as f32,
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
    iced::stream::channel(
        16,
        async move |mut out: iced::futures::channel::mpsc::Sender<Update>| {
            loop {
                let _ = out.send(Update::Monitors(monitors_async().await)).await;
                let _ = out.send(Update::Clients(clients_async().await)).await;
                let _ = out.send(Update::Workspaces(workspaces_async().await)).await;
                let _ = out
                    .send(Update::ActiveWindow(active_window_async().await))
                    .await;
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
                            if is_window_event(&line)
                                || is_relevant(&line)
                                || line.starts_with("createworkspace")
                                || line.starts_with("destroyworkspace")
                            {
                                let _ =
                                    out.send(Update::Workspaces(workspaces_async().await)).await;
                            }
                            if let Some(p) = line.strip_prefix("activewindow>>") {
                                let _ =
                                    out.send(Update::ActiveWindow(parse_active_window(p))).await;
                            }
                        }
                        tracing::info!("socket d'événements fermée, reconnexion");
                    }
                    Err(e) => tracing::warn!("connexion à {} : {e}", path.display()),
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

/// Raccourci déjà pris dans Hyprland (hors submap, hors ceux de hyprdesk).
pub fn conflicts_with(binds: &Value, combo: &str) -> Option<String> {
    let (mask, key) = parse_combo(combo)?;
    binds.as_array()?.iter().find_map(|b| {
        let same = b["modmask"].as_u64() == Some(mask as u64)
            && b["key"].as_str().is_some_and(|k| k.to_lowercase() == key)
            && b["submap"].as_str().unwrap_or("").is_empty()
            && !b["description"]
                .as_str()
                .unwrap_or("")
                .starts_with("hyprdesk");
        same.then(|| {
            b["dispatcher"]
                .as_str()
                .unwrap_or("?")
                .replace("HL.Dispatcher(", "")
                .replace(')', "")
        })
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
    // Un raccourci par mode ; celui du mode en cours rend no-desktop.
    for (k, arg, desc) in [
        (&keys.toggle, "toggle", "full-desktop"),
        (&keys.launcher, "toggle-launcher", "launcher-only"),
        (&keys.hide, "hide", "no-desktop"),
    ] {
        if usable(k) {
            lua += &format!(
                "keep(hl.bind({}, hl.dsp.exec_cmd({}), {{ description = \"hyprdesk : {desc}\" }}))\n",
                q(k),
                q(&cmd(arg))
            );
        }
    }
    if usable(&keys.settings) {
        lua += &format!(
            "keep(hl.bind({}, hl.dsp.exec_cmd({}), {{ description = \"hyprdesk : réglages\" }}))\n",
            q(&keys.settings),
            q(&cmd("settings"))
        );
    }
    lua += &format!("hl.define_submap({}, function()\n", q(SUBMAP));
    // Dans la submap (full-desktop, raccourcis suspendus) : chaque raccourci
    // rend d'abord la main, puis change de mode.
    for (k, arg, desc) in [
        (&keys.toggle, "hide", "no-desktop"),
        (&keys.launcher, "launcher-only", "launcher-only"),
        (&keys.hide, "hide", "no-desktop"),
        (&keys.leave, "hide", "sortie de secours"),
    ] {
        if usable(k) {
            lua += &format!(
                "  keep(hl.bind({}, leave({}), {{ description = \"hyprdesk : {desc}\" }}))\n",
                q(k),
                q(&cmd(arg))
            );
        }
    }
    // Les réglages ne changent pas de mode : la submap reste active.
    if usable(&keys.settings) {
        lua += &format!(
            "  keep(hl.bind({}, hl.dsp.exec_cmd({}), {{ description = \"hyprdesk : réglages\" }}))\n",
            q(&keys.settings),
            q(&cmd("settings"))
        );
    }
    lua += "end)\n";
    // Barre et menus : flou derrière leur fond translucide, apparition en douceur.
    lua += "if __hyprdesk_rules then for _, r in ipairs(__hyprdesk_rules) do pcall(function() r:set_enabled(false) end) end end\n\
            __hyprdesk_rules = {}\n\
            pcall(function() table.insert(__hyprdesk_rules, hl.layer_rule({ match = { namespace = \"^hyprdesk-(bar|popover|settings|launcher|launcher-popup)$\" }, blur = true, ignore_alpha = 0.2, xray = false })) end)\n\
            pcall(function() table.insert(__hyprdesk_rules, hl.layer_rule({ match = { namespace = \"^hyprdesk-popover$\" }, animation = \"none\" })) end)\n";
    lua
}

/// Installe les raccourcis ; rend les conflits (raccourci, action déjà liée)
/// et les raccourcis universels de Hyprland.
pub type KeyReport = (Vec<(String, String)>, Vec<String>);

pub fn install_keys(keys: &crate::config::Keys) -> Result<KeyReport, ipc::IpcError> {
    let binds = ipc::query_json("binds")?;
    let mut conflicts = Vec::new();
    for k in [
        &keys.toggle,
        &keys.launcher,
        &keys.hide,
        &keys.settings,
        &keys.leave,
    ] {
        if !k.is_empty()
            && let Some(what) = conflicts_with(&binds, k)
            && !conflicts.iter().any(|(c, _): &(String, String)| c == k)
        {
            conflicts.push((k.clone(), what));
        }
    }
    let skip: Vec<String> = conflicts.iter().map(|(k, _)| k.clone()).collect();
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "hyprdesk".into());
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

/// Pose les règles de workspace ; une règle refusée par Hyprland (regex
/// invalide…) est signalée sans empêcher les autres.
pub fn install_rules(
    workspaces: Option<u8>,
    placement: &[Placement],
    rules: &[crate::config::AssignRule],
) -> Result<(), ipc::IpcError> {
    use crate::config::AppMatch;
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
        tracing::warn!("règles de workspace : {r}");
        notify(
            3,
            6000,
            &format!("hyprdesk : règles de workspace refusées — {}", r.trim()),
        );
    }
    Ok(())
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
        Ok(r) => tracing::warn!("marges du bureau : {}", r.trim()),
        Err(e) => tracing::warn!("marges du bureau : {e}"),
    }
}

/// Désactive les règles de workspace de hyprdesk (arrêt du démon).
pub fn remove_rules() {
    if let Err(e) = ipc::eval(&build_rules_lua(None, &[], &[])) {
        tracing::warn!("retrait des règles de workspace : {e}");
    }
}

/// Entre dans la submap du bureau, ou en sort (`None`).
pub fn set_submap(name: Option<&str>) {
    let target = format!(
        "hl.dsp.submap(\"{}\")",
        ipc::lua_escape(name.unwrap_or("reset"))
    );
    if let Err(e) = ipc::dispatch(&target) {
        tracing::warn!("submap : {e}");
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

/// Workspace spécial où attendent les fenêtres mises de côté.
pub const STASH: &str = "special:hyprdesk";

/// Fenêtre mise de côté : où la rendre.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stashed {
    pub address: String,
    pub workspace: String,
    pub monitor: String,
}

/// Déplace une fenêtre sans la suivre.
pub fn move_window(address: &str, workspace: &str) -> Result<(), ipc::IpcError> {
    let r = ipc::dispatch(&format!(
        "hl.dsp.window.move({{ workspace = \"{}\", follow = false, window = \"address:{}\" }})",
        ipc::lua_escape(workspace),
        ipc::lua_escape(address)
    ))?;
    if r.trim() != "ok" {
        return Err(ipc::IpcError::Io(std::io::Error::other(format!(
            "déplacement de {address} vers {workspace} : {r}"
        ))));
    }
    Ok(())
}

/// Referme le workspace spécial des fenêtres mises de côté s'il s'est ouvert :
/// quand la fenêtre qui avait le focus y part, Hyprland l'ouvre pour suivre
/// le focus, et les fenêtres réapparaissent par-dessus le bureau.
pub fn close_stash_view() {
    let Ok(v) = ipc::query_json("monitors") else {
        return;
    };
    let Some(list) = v.as_array() else { return };
    let focused = list
        .iter()
        .find(|m| m["focused"].as_bool() == Some(true))
        .and_then(|m| m["name"].as_str())
        .map(String::from);
    // Écran resté sur un workspace ORDINAIRE nommé « special:hyprdesk » (état
    // observé avec Hyprland 0.57) : on le ramène sur un workspace ordinaire.
    let stuck: Vec<String> = list
        .iter()
        .filter(|m| {
            m["activeWorkspace"]["name"].as_str() == Some(STASH)
                && m["specialWorkspace"]["name"].as_str() != Some(STASH)
        })
        .filter_map(|m| m["name"].as_str().map(String::from))
        .collect();
    if !stuck.is_empty() {
        let workspaces = ipc::query_json("workspaces").unwrap_or(Value::Null);
        for mon in &stuck {
            let target = regular_workspace(&v, &workspaces, mon);
            tracing::warn!(
                "{mon} affichait {STASH} comme workspace ordinaire : retour sur {target}"
            );
            let _ = ipc::dispatch(&format!(
                "hl.dsp.focus({{ monitor = \"{}\" }})",
                ipc::lua_escape(mon)
            ));
            let _ = focus_workspace(&target);
        }
        if let Some(f) = &focused
            && !stuck.contains(f)
        {
            let _ = ipc::dispatch(&format!(
                "hl.dsp.focus({{ monitor = \"{}\" }})",
                ipc::lua_escape(f)
            ));
        }
    }
    let open: Vec<String> = list
        .iter()
        .filter(|m| m["specialWorkspace"]["name"].as_str() == Some(STASH))
        .filter_map(|m| m["name"].as_str().map(String::from))
        .collect();
    if open.is_empty() {
        return;
    }
    let name = STASH.trim_start_matches("special:");
    for mon in &open {
        // toggle_special agit sur l'écran qui a le focus.
        if Some(mon) != focused.as_ref() {
            let _ = ipc::dispatch(&format!(
                "hl.dsp.focus({{ monitor = \"{}\" }})",
                ipc::lua_escape(mon)
            ));
        }
        if let Err(e) = ipc::dispatch(&format!(
            "hl.dsp.workspace.toggle_special(\"{}\")",
            ipc::lua_escape(name)
        )) {
            tracing::warn!("fermeture de {STASH} : {e}");
        }
    }
    if let Some(f) = focused
        && !open.contains(&f)
    {
        let _ = ipc::dispatch(&format!(
            "hl.dsp.focus({{ monitor = \"{}\" }})",
            ipc::lua_escape(&f)
        ));
    }
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
            tracing::warn!("restitution reportée : réponse clients invalide ({other:?})");
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
            tracing::warn!("restitution de {} : {e}", s.address);
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
            tracing::warn!("journal conservé : {e}");
            return;
        }
    };
    if !list.is_empty() {
        let pending = restore(&list);
        tracing::info!(
            "{} fenêtre(s) restituée(s), {} en attente",
            list.len() - pending.len(),
            pending.len()
        );
        if let Err(e) = save_stash(&pending) {
            tracing::warn!("journal : {e}");
        }
    }
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

/// Lance une commande dont la fenêtre s'ouvre flottante et centrée, de la
/// taille donnée : le bureau reste visible autour.
pub fn exec_floating(cmd: &str, w: u32, h: u32) -> Result<(), ipc::IpcError> {
    let r = ipc::dispatch(&format!(
        "hl.dsp.exec_cmd(\"{}\", {{ float = true, center = true, size = \"{w} {h}\" }})",
        ipc::lua_escape(cmd)
    ))?;
    if r.trim() != "ok" {
        tracing::warn!("exec « {cmd} » : réponse inattendue de Hyprland : {r}");
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
                            tracing::warn!("notification Hyprland : {e}");
                        }
                    }
                })
                .expect("thread des notifications");
            tx
        });
    if let Err(e) = NOTICES.try_send((icon, ms, msg.to_string())) {
        tracing::warn!("notification reportée : {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            conflicts_with(&binds, "SUPER + q").as_deref(),
            Some("close")
        );
        // Nos propres raccourcis et ceux d'une autre submap ne sont pas des conflits.
        assert_eq!(conflicts_with(&binds, "SUPER + D"), None);
        assert_eq!(
            universal_binds(&binds),
            vec!["SUPER + eacute → focus".to_string()]
        );
    }

    #[test]
    fn lua_des_raccourcis() {
        let k = crate::config::Keys {
            toggle: "SUPER + D".into(),
            launcher: "SUPER + ALT + D".into(),
            hide: "SUPER + CTRL + D".into(),
            settings: "SUPER + SHIFT + D".into(),
            leave: "SUPER + Escape".into(),
        };
        let lua = build_keys_lua(&k, "/bin/hyprdesk", &["SUPER + SHIFT + D".to_string()]);
        assert!(
            lua.contains(r#"keep(hl.bind("SUPER + D", hl.dsp.exec_cmd("'/bin/hyprdesk' toggle")"#),
            "{lua}"
        );
        assert!(
            !lua.contains("SHIFT"),
            "un raccourci en conflit n'est pas installé : {lua}"
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
        // Réglages : hors submap et dans la submap, sans la quitter.
        let lua = build_keys_lua(&k, "/bin/hyprdesk", &[]);
        assert_eq!(
            lua.matches(
                r#"hl.bind("SUPER + SHIFT + D", hl.dsp.exec_cmd("'/bin/hyprdesk' settings")"#
            )
            .count(),
            2,
            "{lua}"
        );
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
}
