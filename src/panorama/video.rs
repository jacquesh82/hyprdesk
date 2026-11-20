//! Widget Vidéo du Panorama : les services (Free TV, Netflix, Disney+…)
//! s'ouvrent en fenêtre d'application d'un navigateur Chromium (`--app=`,
//! Widevine pour les services protégés), avec le profil habituel de
//! l'utilisateur, donc déjà connecté. hyprdesk retrouve la fenêtre (son
//! app_id `<navigateur>-<hôte>…`), la rend flottante, l'épingle, lui retire
//! bordure et ombre, et la pose exactement dans la zone du widget ; la
//! surface du Panorama y laisse un trou (image et saisie). Son plein écran
//! reste dans la zone (runtime des zones, MLG-172).

use serde_json::Value;

/// Navigateurs Chromium connus : (commande, préfixe de l'app_id de leurs
/// fenêtres d'application sous Wayland).
pub const BROWSERS: &[(&str, &str)] = &[
    ("brave", "brave"),
    ("brave-browser", "brave"),
    ("chromium", "chromium"),
    ("google-chrome-stable", "chrome"),
    ("google-chrome", "chrome"),
    ("vivaldi-stable", "vivaldi"),
    ("microsoft-edge-stable", "msedge"),
];

/// Étiquette posée sur la fenêtre (règle de Hyprland : sans bordure ni ombre).
pub const TAG: &str = "hyprdesk-pip";

fn in_path(cmd: &str) -> bool {
    if cmd.contains('/') {
        return std::path::Path::new(cmd).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file()))
}

/// Préfixe d'app_id d'une commande de navigateur.
pub fn prefix_of(cmd: &str) -> String {
    let base = cmd.rsplit('/').next().unwrap_or(cmd);
    BROWSERS
        .iter()
        .find(|(c, _)| *c == base)
        .map(|(_, p)| p.to_string())
        .unwrap_or_else(|| base.to_string())
}

/// Navigateur à utiliser : celui configuré, sinon le premier Chromium
/// installé ; (commande, préfixe d'app_id).
pub fn browser(configured: &str) -> Option<(String, String)> {
    let c = configured.trim();
    if !c.is_empty() {
        return Some((c.to_string(), prefix_of(c)));
    }
    BROWSERS
        .iter()
        .find(|(cmd, _)| in_path(cmd))
        .map(|(cmd, p)| (cmd.to_string(), p.to_string()))
}

/// Hôte d'une adresse (`https://tv.free.fr/x` → `tv.free.fr`).
pub fn host(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .to_lowercase()
}

/// La fenêtre de classe `class` est l'application ouverte pour `url`.
pub fn matches(class: &str, prefix: &str, url: &str) -> bool {
    let c = class.to_lowercase();
    let h = host(url);
    !h.is_empty() && (c.starts_with(&format!("{prefix}-{h}")) || c.contains(&h))
}

/// Fenêtre apparue depuis le lancement (absente de `known`) et qui
/// correspond à la source.
pub fn find_new(clients: &Value, known: &[String], prefix: &str, url: &str) -> Option<String> {
    clients.as_array()?.iter().find_map(|c| {
        let a = c["address"].as_str()?;
        let class = c["class"].as_str().unwrap_or("");
        let initial = c["initialClass"].as_str().unwrap_or("");
        (!known.iter().any(|k| k == a)
            && (matches(class, prefix, url) || matches(initial, prefix, url)))
        .then(|| a.to_string())
    })
}

/// Lance la source (détaché). Bloquant le temps du `spawn`.
pub fn launch(cmd: &str, url: &str) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let mut child = std::process::Command::new(cmd)
        .arg(format!("--app={url}"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("{cmd} : {e}"))?;
    // Le lanceur rend souvent la main aussitôt (script, instance existante).
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Règle de Hyprland des fenêtres étiquetées : sans bordure, arrondi, ombre ni flou.
pub fn rule_lua() -> String {
    format!(
        "if not __hyprdesk_player_rule then __hyprdesk_player_rule = hl.window_rule({{ name = \"{TAG}\", match = {{ tag = \"{TAG}\" }}, \
         border_size = 0, rounding = 0, no_shadow = true, no_blur = true, no_anim = true, no_xdg_drags = true, sync_fullscreen = false }}) end"
    )
}

/// Poser la règle avant toute transition de plein écran/flottant : autrement
/// Hyprland peut notifier une sortie de plein écran au navigateur.
pub fn prepare(address: &str) -> Result<(), String> {
    hyprpanel_core::ipc::eval(&rule_lua()).map_err(|e| e.to_string())?;
    hyprpanel_core::ipc::eval(include_str!("../../assets/panorama-player.lua"))
        .map_err(|e| e.to_string())?;
    hyprpanel_core::ipc::dispatch(&format!(
        "hl.dsp.window.tag({{ tag = \"+{TAG}\", window = \"address:{}\" }})",
        hyprpanel_core::ipc::lua_escape(address)
    ))
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn anchor(address: &str, rect: (i32, i32, i32, i32)) -> Result<(), String> {
    let (x, y, w, h) = rect;
    hyprpanel_core::ipc::eval(&format!(
        "if __hyprdesk_player then __hyprdesk_player.frames[\"{}\"] = {{ x={x}, y={y}, w={w}, h={h}, until_time=os.time()+3 }} end",
        hyprpanel_core::ipc::lua_escape(address)
    )).map(|_| ()).map_err(|e| e.to_string())
}

pub fn unanchor(address: &str) -> Result<(), String> {
    hyprpanel_core::ipc::eval(&format!(
        "if __hyprdesk_player then __hyprdesk_player.frames[\"{}\"] = nil end",
        hyprpanel_core::ipc::lua_escape(address)
    ))
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Flottante, épinglée (visible quel que soit l'espace de travail de
/// l'écran), étiquetée (sans bordure ni ombre), puis posée sur `rect`
/// (coordonnées globales) s'il est déjà connu.
pub fn adopt(address: &str, rect: Option<(i32, i32, i32, i32)>) -> Result<(), String> {
    prepare(address)?;
    let a = hyprpanel_core::ipc::lua_escape(address);
    // Plein écran (bouton du lecteur, F11) : l'application se croit en plein
    // écran mais reste dans son cadre (MLG-172, `assets/zones.lua`).
    hyprpanel_core::ipc::eval(crate::zones::runtime_lua()).map_err(|e| e.to_string())?;
    crate::hypr::set_floating(address, true)?;
    for cmd in [
        format!("hl.dsp.window.pin({{ action = \"enable\", window = \"address:{a}\" }})"),
        format!("hl.dsp.window.tag({{ tag = \"+{TAG}\", window = \"address:{a}\" }})"),
    ] {
        let _ = hyprpanel_core::ipc::dispatch(&cmd);
    }
    match rect {
        Some(r) => {
            anchor(address, r)?;
            crate::hypr::place(address, r)
        }
        None => Ok(()),
    }
}

/// Envoie une touche à la fenêtre du lecteur (sans lui donner le clavier) :
/// `f` met la vidéo seule en plein écran, dans sa zone (MLG-172).
pub fn send_key(address: &str, key: &str) -> Result<(), String> {
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("touche invalide : {key}"));
    }
    hyprpanel_core::ipc::eval(&format!(
        "hl.dispatch(hl.dsp.send_shortcut({{ mods = \"\", key = \"{key}\", window = \"address:{}\" }}))",
        hyprpanel_core::ipc::lua_escape(address)
    ))
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Fenêtres étiquetées du lecteur (adresse, classe), d'un hyprdesk
/// précédent par exemple.
pub fn tagged() -> Vec<(String, String)> {
    let Ok(v) = hyprpanel_core::ipc::query_json("clients") else {
        return Vec::new();
    };
    v.as_array()
        .map(|l| {
            l.iter()
                .filter(|c| {
                    c["tags"].to_string().contains(TAG)
                        && !c["tags"].to_string().contains(super::capture::TAG)
                })
                .filter_map(|c| {
                    Some((
                        c["address"].as_str()?.to_string(),
                        c["class"].as_str().unwrap_or("").to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Ferme la fenêtre.
pub fn close(address: &str) {
    let _ = unanchor(address);
    let _ = hyprpanel_core::ipc::dispatch(&format!(
        "hl.dsp.window.close({{ window = \"address:{}\" }})",
        hyprpanel_core::ipc::lua_escape(address)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadre_corrige_la_geometrie_et_libere_les_fenetres() {
        let lua = mlua::Lua::new();
        lua.load(
            r#"
            now = 100
            os.time = function() return now end
            moves, sizes, timers = 0, 0, 0
            window = { at = { 10, 20 }, size = { 800, 450 }, tags = { "hyprdesk-pip" } }
            hl = {
                get_window = function() return window end,
                timer = function(fn, options) tick = fn; timers = timers + 1; return {} end,
                dispatch = function(cmd)
                    if cmd.kind == "move" then moves = moves + 1; window.at = {cmd.x, cmd.y}
                    else sizes = sizes + 1; window.size = {cmd.x, cmd.y} end
                end,
                dsp = { window = {
                    move = function(t) t.kind = "move"; return t end,
                    resize = function(t) t.kind = "resize"; return t end,
                } },
            }
        "#,
        )
        .exec()
        .unwrap();
        let runtime = include_str!("../../assets/panorama-player.lua");
        lua.load(runtime).exec().unwrap();
        lua.load(runtime).exec().unwrap();
        lua.load(
            r#"
            assert(timers == 1)
            __hyprdesk_player.frames.a = {x=10,y=20,w=800,h=450,until_time=103}
            tick(); assert(moves == 0 and sizes == 0)
            window.at = {100,200}; window.size = {900,500}
            tick(); assert(moves == 1 and sizes == 1)
            tick(); assert(moves == 1 and sizes == 1)
            window.tags = {}; window.at = {40,50}
            tick(); assert(moves == 1 and __hyprdesk_player.frames.a == nil)
            window.tags = {"hyprdesk-pip"}
            __hyprdesk_player.frames.a = {x=10,y=20,w=800,h=450,until_time=103}
            now = 104
            tick(); assert(moves == 1 and __hyprdesk_player.frames.a == nil)
        "#,
        )
        .exec()
        .unwrap();
    }

    #[test]
    fn hote_et_correspondance() {
        assert_eq!(host("https://tv.free.fr"), "tv.free.fr");
        assert_eq!(
            host("https://www.Netflix.com/browse?x=1"),
            "www.netflix.com"
        );
        assert!(matches(
            "brave-tv.free.fr__-Default",
            "brave",
            "https://tv.free.fr"
        ));
        assert!(matches(
            "chrome-www.netflix.com__-Default",
            "chrome",
            "https://www.netflix.com"
        ));
        assert!(!matches("brave-browser", "brave", "https://tv.free.fr"));
        assert!(!matches("anything", "brave", ""));
    }

    #[test]
    fn nouvelle_fenetre_retrouvee() {
        let v: Value = serde_json::from_str(
            r#"[{"address":"0x1","class":"brave-browser"},
                {"address":"0x2","class":"brave-tv.free.fr__-Default"},
                {"address":"0x3","class":"brave-tv.free.fr__-Default"}]"#,
        )
        .unwrap();
        // 0x2 existait déjà avant le lancement : c'est 0x3.
        assert_eq!(
            find_new(
                &v,
                &["0x1".into(), "0x2".into()],
                "brave",
                "https://tv.free.fr"
            ),
            Some("0x3".into())
        );
    }

    #[test]
    fn navigateur() {
        assert_eq!(prefix_of("/usr/bin/google-chrome-stable"), "chrome");
        assert_eq!(prefix_of("brave"), "brave");
        assert_eq!(
            browser("chromium"),
            Some(("chromium".into(), "chromium".into()))
        );
        assert!(rule_lua().contains(r#"match = { tag = "hyprdesk-pip" }"#));
    }
}
