//! Accueil des fenêtres vidéo externes, indépendamment de leur écran.
//! Les fenêtres empruntées sont restaurées, jamais fermées.
use hyprpanel_core::ipc;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const TAG: &str = "hyprdesk-captured-video";

#[derive(Debug, Clone)]
pub struct Borrowed {
    touched: std::sync::Arc<std::sync::atomic::AtomicBool>,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    mutation: std::sync::Arc<std::sync::Mutex<()>>,
    origin: Option<Origin>,
    fallback_rect: std::sync::Arc<std::sync::Mutex<Option<(i32, i32, i32, i32)>>>,
    fullscreen_session: bool,
    exit_ready: bool,
    pub address: String,
    pub title: String,
    pub class: String,
    pub workspace: String,
    pub rect: (i32, i32, i32, i32),
    pub floating: bool,
    pub pinned: bool,
    pub fullscreen: i64,
    pub client_fullscreen: i64,
}

/// Dernier état fenêtré observé, avant toute capture ou mise en plein écran.
#[derive(Debug, Clone)]
struct Origin {
    class: String,
    workspace: String,
    rect: (i32, i32, i32, i32),
    floating: bool,
    pinned: bool,
    fullscreen: i64,
    client_fullscreen: i64,
}
impl Origin {
    fn read(c: &Value) -> Option<Self> {
        Some(Self {
            class: c["class"].as_str()?.into(),
            workspace: c["workspace"]["name"].as_str()?.into(),
            rect: (
                c["at"][0].as_i64()? as i32,
                c["at"][1].as_i64()? as i32,
                c["size"][0].as_i64()? as i32,
                c["size"][1].as_i64()? as i32,
            ),
            floating: c["floating"].as_bool().unwrap_or(false),
            pinned: c["pinned"].as_bool().unwrap_or(false),
            fullscreen: c["fullscreen"].as_i64().unwrap_or(0),
            client_fullscreen: c["fullscreenClient"].as_i64().unwrap_or(0),
        })
    }
}

fn browser(class: &str) -> bool {
    let c = class.to_lowercase();
    [
        "brave",
        "chromium",
        "chrome",
        "google-chrome",
        "vivaldi",
        "msedge",
        "microsoft-edge",
        "firefox",
        "org.mozilla.firefox",
    ]
    .iter()
    .any(|p| c == *p || c.starts_with(&format!("{p}-")) || c.starts_with(&format!("{p}.")))
}

fn pip(c: &Value) -> bool {
    let title = c["title"].as_str().unwrap_or("").to_lowercase();
    browser(c["class"].as_str().unwrap_or(""))
        && matches!(
            title.as_str(),
            "picture-in-picture"
                | "picture in picture"
                | "incrustation vidéo"
                | "image dans l’image"
                | "image dans l'image"
        )
}

fn full(c: &Value) -> bool {
    c["fullscreen"].as_i64() == Some(2) || c["fullscreenClient"].as_i64() == Some(2)
}

/// Au démarrage : une fenêtre capturée par une session précédente de
/// hyprdesk (marque encore posée) est libérée de ses marques et de son
/// ancrage, pour que la surveillance la reprenne comme une nouvelle (sinon
/// elle resterait cachée sous le Panorama, ni reprise ni rendue).
pub fn release_stale() -> usize {
    let Ok(v) = ipc::query_json("clients") else {
        return 0;
    };
    let stale: Vec<String> = v
        .as_array()
        .map(|l| {
            l.iter()
                .filter(|c| {
                    c["tags"]
                        .as_array()
                        .is_some_and(|t| t.iter().any(|t| t.as_str() == Some(TAG)))
                })
                .filter_map(|c| c["address"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for a in &stale {
        let _ = super::video::unanchor(a);
        for tag in [TAG, super::video::TAG] {
            let _ = ipc::dispatch(&format!(
                "hl.dsp.window.tag({{ tag = \"-{tag}\", window = \"address:{}\" }})",
                ipc::lua_escape(a)
            ));
        }
    }
    stale.len()
}

fn tagged(c: &Value) -> bool {
    c["tags"].as_array().is_some_and(|tags| {
        tags.iter()
            .any(|t| t.as_str() == Some(super::video::TAG) || t.as_str() == Some(TAG))
    })
}

/// Un navigateur plein écran est un candidat seulement si son titre correspond
/// à un média MPRIS. F11 sur une page ordinaire ne suffit pas.
pub fn eligible(c: &Value, media: &[crate::widgets::media::Status]) -> bool {
    if !c["mapped"].as_bool().unwrap_or(true) || c["hidden"].as_bool().unwrap_or(false) {
        return false;
    }
    if pip(c) {
        return true;
    }
    if !full(c) {
        return false;
    }
    if c["contentType"].as_str() == Some("video") {
        return true;
    }
    let class = c["class"].as_str().unwrap_or("").to_lowercase();
    if [
        "mpv",
        "vlc",
        "org.videolan.vlc",
        "io.github.celluloid_player.celluloid",
        "celluloid",
        "org.gnome.totem",
        "totem",
        "haruna",
        "org.kde.haruna",
    ]
    .contains(&class.as_str())
    {
        return true;
    }
    let title = c["title"].as_str().unwrap_or("").to_lowercase();
    browser(&class)
        && media.iter().any(|m| {
            let player = m.player.to_lowercase();
            let family = if class.contains("firefox") {
                "firefox"
            } else if class.contains("brave") {
                "brave"
            } else if class.contains("chromium") {
                "chromium"
            } else if class.contains("vivaldi") {
                "vivaldi"
            } else if class.contains("edge") {
                "edge"
            } else {
                "chrome"
            };
            m.playing
                && player.contains(family)
                && !m.title.trim().is_empty()
                && title.contains(&m.title.to_lowercase())
        })
}

#[derive(Debug, Default)]
pub struct Watch {
    /// Pas de reprise automatique après restitution, avant sortie du mode.
    seen: HashSet<String>,
    origins: HashMap<String, Origin>,
}
impl Watch {
    pub fn next(
        &mut self,
        clients: &Value,
        media: &[crate::widgets::media::Status],
        current: Option<&str>,
    ) -> Option<Borrowed> {
        let list = clients.as_array()?;
        self.origins
            .retain(|a, _| list.iter().any(|c| c["address"].as_str() == Some(a)));
        for c in list {
            let Some(a) = c["address"].as_str() else {
                continue;
            };
            if Some(a) != current
                && !full(c)
                && !pip(c)
                && !tagged(c)
                && c["mapped"].as_bool().unwrap_or(true)
                && !c["hidden"].as_bool().unwrap_or(false)
                && let Some(origin) = Origin::read(c)
            {
                self.origins.insert(a.into(), origin);
            }
        }
        self.seen.retain(|a| {
            list.iter()
                .any(|c| c["address"].as_str() == Some(a) && (full(c) || pip(c)))
        });
        let c = list
            .iter()
            .filter(|c| {
                let a = c["address"].as_str().unwrap_or("");
                !a.is_empty()
                    && Some(a) != current
                    && !self.seen.contains(a)
                    && !tagged(c)
                    && eligible(c, media)
            })
            .max_by_key(|c| (pip(c), c["focusHistoryID"].as_i64() == Some(0)))?;
        let mut b = Borrowed::from_client(c)?;
        b.origin = self
            .origins
            .get(&b.address)
            .filter(|o| o.class == b.class)
            .cloned();
        self.seen.insert(b.address.clone());
        Some(b)
    }
}
fn transfer_lua(address: &str, rect: (i32, i32, i32, i32), client: i64) -> String {
    let (x, y, w, h) = rect;
    format!(
        "return (function(...)\n{}\nend)(\"{}\",{x},{y},{w},{h},{client})",
        include_str!("../../assets/panorama-capture.lua"),
        ipc::lua_escape(address)
    )
}

impl Borrowed {
    /// Armé seulement après l'accusé de réussite du transfert atomique.
    pub fn confirm_transfer(&mut self) {
        self.exit_ready = self.fullscreen_session;
    }
    pub fn exited_fullscreen(&self, clients: &Value) -> bool {
        self.exit_ready
            && clients.as_array().is_some_and(|list| {
                list.iter().any(|c| {
                    c["address"].as_str() == Some(&self.address)
                        && matches!(c["fullscreenClient"].as_i64(), Some(0 | 1))
                        && matches!(c["fullscreen"].as_i64(), Some(0 | 1))
                })
            })
    }
    /// La sortie du plein écran rend l'état d'avant, jamais le plein écran
    /// qui avait déclenché la capture.
    pub fn return_before_fullscreen(&mut self) {
        self.exit_ready = false;
        self.fullscreen = 0;
        self.client_fullscreen = 0;
        if let Some(o) = &self.origin {
            self.workspace = o.workspace.clone();
            self.rect = o.rect;
            self.floating = o.floating;
            self.pinned = o.pinned;
            self.fullscreen = o.fullscreen;
            self.client_fullscreen = o.client_fullscreen;
        }
        // La géométrie rendue par le compositeur est plus récente qu'un relevé
        // périodique (redimensionnement juste avant le plein écran).
        if let Some(rect) = self.fallback_rect.lock().ok().and_then(|r| *r) {
            self.rect = rect;
        }
    }
    fn from_client(c: &Value) -> Option<Self> {
        Some(Self {
            origin: None,
            fallback_rect: Default::default(),
            fullscreen_session: full(c) && !pip(c),
            exit_ready: false,
            touched: Default::default(),
            cancelled: Default::default(),
            mutation: Default::default(),
            address: c["address"].as_str()?.into(),
            title: c["title"].as_str().unwrap_or("Vidéo").into(),
            class: c["class"].as_str().unwrap_or("").into(),
            workspace: c["workspace"]["name"].as_str()?.into(),
            rect: (
                c["at"][0].as_i64()? as i32,
                c["at"][1].as_i64()? as i32,
                c["size"][0].as_i64()? as i32,
                c["size"][1].as_i64()? as i32,
            ),
            floating: c["floating"].as_bool().unwrap_or(false),
            pinned: c["pinned"].as_bool().unwrap_or(false),
            fullscreen: c["fullscreen"].as_i64().unwrap_or(0),
            client_fullscreen: c["fullscreenClient"].as_i64().unwrap_or(0),
        })
    }
    fn dispatch(&self, command: String) -> Result<(), String> {
        let r = ipc::dispatch(&command).map_err(|e| e.to_string())?;
        if r.trim() == "ok" { Ok(()) } else { Err(r) }
    }
    fn fullscreen(&self, internal: i64, client: i64) -> Result<(), String> {
        self.dispatch(format!("hl.dsp.window.fullscreen_state({{ internal = {internal}, client = {client}, action = \"set\", window = \"address:{}\" }})", ipc::lua_escape(&self.address)))
    }
    fn tag(&self, tag: &str) -> Result<(), String> {
        self.dispatch(format!(
            "hl.dsp.window.tag({{ tag = \"{tag}\", window = \"address:{}\" }})",
            ipc::lua_escape(&self.address)
        ))
    }
    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn adopt(&self, rect: (i32, i32, i32, i32)) -> Result<(), String> {
        let _guard = self.mutation.lock().map_err(|e| e.to_string())?;
        if self.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("capture annulée".into());
        }
        // Revalidation avant toute mutation : la fenêtre peut avoir disparu.
        let clients = ipc::query_json("clients").map_err(|e| e.to_string())?;
        let c = clients
            .as_array()
            .and_then(|l| {
                l.iter()
                    .find(|c| c["address"].as_str() == Some(&self.address))
            })
            .ok_or("fenêtre vidéo disparue")?;
        if !(pip(c) || full(c)) {
            return Err("mode vidéo quitté".into());
        }
        self.touched
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.tag(&format!("+{TAG}"))?;
        super::video::prepare(&self.address)?;
        ipc::eval(crate::zones::runtime_lua()).map_err(|e| e.to_string())?;
        let code = transfer_lua(
            &self.address,
            rect,
            self.client_fullscreen.max(self.fullscreen),
        );
        let result = ipc::repl(&code).map_err(|e| e.to_string())?;
        let saved: Vec<i32> = result
            .trim()
            .trim_matches('"')
            .split(',')
            .filter_map(|n| n.trim().parse::<f64>().ok().map(|n| n.round() as i32))
            .collect();
        if saved.len() != 4 {
            return Err(format!("géométrie avant transfert illisible : {result}"));
        }
        *self.fallback_rect.lock().map_err(|e| e.to_string())? =
            Some((saved[0], saved[1], saved[2], saved[3]));
        super::video::anchor(&self.address, rect)
    }
    pub fn renew_anchor(&self, rect: (i32, i32, i32, i32)) -> Result<(), String> {
        let _guard = self.mutation.lock().map_err(|e| e.to_string())?;
        if self.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        super::video::anchor(&self.address, rect)
    }
    pub fn place(&self, rect: (i32, i32, i32, i32)) -> Result<(), String> {
        let _guard = self.mutation.lock().map_err(|e| e.to_string())?;
        if self.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        super::video::anchor(&self.address, rect)?;
        crate::hypr::place(&self.address, rect)
    }
    pub fn restore(&self) -> Result<(), String> {
        self.cancel();
        let _guard = self.mutation.lock().map_err(|e| e.to_string())?;
        if !self
            .touched
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }
        // Continuer la restitution même si l'une des commandes échoue.
        let results = [
            super::video::unanchor(&self.address),
            self.tag(&format!("-{}", super::video::TAG)),
            self.tag(&format!("-{TAG}")),
            self.fullscreen(0, 0),
            self.dispatch(format!(
                "hl.dsp.window.pin({{ action = \"{}\", window = \"address:{}\" }})",
                if self.pinned { "enable" } else { "disable" },
                ipc::lua_escape(&self.address)
            )),
            crate::hypr::move_window(&self.address, &self.workspace).map_err(|e| e.to_string()),
            crate::hypr::place(&self.address, self.rect),
            crate::hypr::set_floating(&self.address, self.floating),
            self.fullscreen(self.fullscreen, self.client_fullscreen),
        ];
        results.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn client(class: &str, title: &str, full: i64) -> Value {
        serde_json::json!({"address":"0x1", "class":class, "title":title, "fullscreen":full,
            "workspace":{"name":"2"}, "at":[1920,0], "size":[800,450], "monitor":3})
    }
    #[test]
    fn transfer_preserves_client_fullscreen_after_layout_changes() {
        let lua = mlua::Lua::new();
        lua.load(
            r#"
            client, internal = 2, 2
            hl = {dsp={window={}}, dispatch=function(f) return f() end,
                get_window=function() return {at={100,200},size={800,450}} end}
            hl.dsp.window.fullscreen_state = function(args) return function()
                internal, client = args.internal, args.client
                return {success=true}
            end end
            -- Le compositeur perd le mode client lors du passage en flottant
            -- ou du changement d'espace. Reproduire ces transitions, puis vérifier
            -- le résultat final de la transaction utilisée en production.
            for _, key in ipairs({'float','pin','resize','move'}) do
                hl.dsp.window[key] = function(args) return function()
                    client = 0
                    return {success=true}
                end end
            end
        "#,
        )
        .exec()
        .unwrap();
        lua.load(transfer_lua("0x1", (2560, 50, 800, 450), 2))
            .exec()
            .unwrap();
        assert_eq!(lua.globals().get::<i64>("internal").unwrap(), 0);
        assert_eq!(lua.globals().get::<i64>("client").unwrap(), 2);
        lua.load(transfer_lua("0x1", (2560, 50, 800, 450), 0))
            .exec()
            .unwrap();
        assert_eq!(
            lua.globals().get::<i64>("client").unwrap(),
            0,
            "PiP reste fenêtré"
        );
        lua.load("hl.dsp.window.float = function() return function() return {success=false,error='test failure'} end end").exec().unwrap();
        assert!(
            lua.load(transfer_lua("0x1", (0, 0, 800, 450), 2))
                .exec()
                .is_err()
        );
    }

    #[test]
    fn exit_is_armed_after_transfer_and_restores_pre_fullscreen_origin() {
        let mut watch = Watch::default();
        let mut normal = client("mpv", "Film", 0);
        normal["floating"] = true.into();
        normal["fullscreenClient"] = 0.into();
        assert!(
            watch
                .next(&serde_json::json!([normal]), &[], None)
                .is_none()
        );
        let mut fullscreen = normal.clone();
        fullscreen["fullscreen"] = 2.into();
        fullscreen["fullscreenClient"] = 2.into();
        fullscreen["at"] = serde_json::json!([1920, 0]);
        fullscreen["size"] = serde_json::json!([2560, 1440]);
        let mut b = watch
            .next(&serde_json::json!([fullscreen]), &[], None)
            .unwrap();
        let mut placed = normal.clone();
        placed["at"] = serde_json::json!([4000, 100]);
        placed["size"] = serde_json::json!([1000, 300]);
        assert!(
            !b.exited_fullscreen(&serde_json::json!([placed])),
            "transfert non confirmé"
        );
        b.confirm_transfer();
        assert!(!b.exited_fullscreen(&serde_json::json!([fullscreen])));
        assert!(
            !b.exited_fullscreen(&serde_json::json!([])),
            "pas de fausse sortie sur un relevé incomplet"
        );
        assert!(b.exited_fullscreen(&serde_json::json!([placed])));
        placed["fullscreenClient"] = 1.into();
        assert!(
            b.exited_fullscreen(&serde_json::json!([placed])),
            "Brave peut annoncer maximisé à la sortie"
        );
        b.return_before_fullscreen();
        assert_eq!(b.rect, (1920, 0, 800, 450));
        assert_eq!(b.workspace, "2");
        assert!(b.floating);
        assert_eq!((b.fullscreen, b.client_fullscreen), (0, 0));
    }
    #[test]
    fn pip_is_not_released_by_absence_of_fullscreen_and_fallback_uses_normal_geometry() {
        let mut pip = Borrowed::from_client(&client("firefox", "Picture-in-Picture", 0)).unwrap();
        pip.confirm_transfer();
        let mut c = client("firefox", "Picture-in-Picture", 0);
        c["fullscreenClient"] = 0.into();
        assert!(!pip.exited_fullscreen(&serde_json::json!([c])));
        let mut b = Borrowed::from_client(&client("mpv", "Film", 2)).unwrap();
        *b.fallback_rect.lock().unwrap() = Some((100, 200, 600, 400));
        b.return_before_fullscreen();
        assert_eq!(b.rect, (100, 200, 600, 400));
        assert_eq!((b.fullscreen, b.client_fullscreen), (0, 0));
    }

    #[test]
    fn pip_and_native_video_on_any_monitor() {
        assert!(eligible(&client("firefox", "Picture-in-Picture", 0), &[]));
        assert!(eligible(&client("chrome", "Picture in picture", 0), &[]));
        assert!(eligible(&client("mpv", "Film", 2), &[]));
        assert!(!eligible(&client("vlc", "Film", 1), &[]));
        assert!(!eligible(&client("kitty", "Picture-in-Picture", 2), &[]));
    }
    #[test]
    fn cancellation_before_adoption_never_touches_hyprland() {
        let b = Borrowed::from_client(&client("mpv", "Film", 2)).unwrap();
        b.cancel();
        assert!(b.adopt((0, 0, 800, 450)).is_err());
        assert!(b.restore().is_ok());
    }
    #[test]
    fn compositor_video_hint_supports_other_players() {
        let mut c = client("custom-player", "Film", 2);
        c["contentType"] = "video".into();
        assert!(eligible(&c, &[]));
        c["fullscreen"] = 0.into();
        assert!(!eligible(&c, &[]));
    }
    #[test]
    fn browser_requires_matching_media() {
        let c = client("brave-browser", "Film - YouTube", 2);
        assert!(!eligible(&c, &[]));
        let m = crate::widgets::media::Status {
            player: "brave.instance1".into(),
            title: "Film".into(),
            playing: true,
            ..Default::default()
        };
        assert!(eligible(&c, &[m.clone()]));
        assert!(!eligible(&client("firefox", "Film", 2), &[m]));
    }
    #[test]
    fn restoration_does_not_recapture_until_next_activation() {
        let mut w = Watch::default();
        let c = client("mpv", "Film", 2);
        assert!(w.next(&serde_json::json!([c]), &[], None).is_some());
        assert!(w.next(&serde_json::json!([c]), &[], None).is_none());
        w.next(&serde_json::json!([client("mpv", "Film", 0)]), &[], None);
        assert!(w.next(&serde_json::json!([c]), &[], None).is_some());
    }
}
