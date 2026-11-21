//! Côte à côte du Panorama : une fenêtre ouverte ramenée dans la zone
//! d'accueil, à côté du média. Hyprland fait le travail dans le compositeur
//! (un seul bloc Lua : plein écran levé, flottante, épinglée, posée), puis
//! le cadre du lecteur (`assets/panorama-player.lua`) la tient dans sa zone
//! tant que hyprdesk renouvelle le bail. Rendue telle qu'elle était à la
//! sortie (espace de travail, place, mosaïque ou flottante, plein écran) ;
//! jamais fermée.
use hyprpanel_core::ipc;
use serde_json::Value;

/// Étiquette propre aux fenêtres invitées.
pub const TAG: &str = "hyprdesk-guest";

#[derive(Debug, Clone, PartialEq)]
pub struct Guest {
    pub address: String,
    pub title: String,
    pub class: String,
    workspace: String,
    rect: (i32, i32, i32, i32),
    floating: bool,
    pinned: bool,
    fullscreen: i64,
    client_fullscreen: i64,
}

/// Bloc exécuté par Hyprland : sortie du plein écran, flottante, épinglée
/// (visible quel que soit l'espace de travail de l'écran), puis posée.
fn adopt_lua(address: &str, rect: (i32, i32, i32, i32)) -> String {
    let (x, y, w, h) = rect;
    format!(
        r#"return (function()
local window = "address:{a}"
local function dispatch(command)
    local result = hl.dispatch(command)
    if type(result) == "table" and result.success == false then
        error(result.error or "fenêtre non ramenée")
    end
end
dispatch(hl.dsp.window.fullscreen_state({{internal=0, client=0, action="set", window=window}}))
dispatch(hl.dsp.window.float({{action="enable", window=window}}))
dispatch(hl.dsp.window.pin({{action="enable", window=window}}))
dispatch(hl.dsp.window.resize({{x={w}, y={h}, relative=false, window=window}}))
dispatch(hl.dsp.window.move({{x={x}, y={y}, relative=false, window=window}}))
return "ok"
end)()"#,
        a = ipc::lua_escape(address)
    )
}

impl Guest {
    /// État de la fenêtre `address` dans la liste des clients de Hyprland.
    pub fn read(clients: &Value, address: &str) -> Option<Guest> {
        let c = clients
            .as_array()?
            .iter()
            .find(|c| c["address"].as_str() == Some(address))?;
        Some(Guest {
            address: address.to_string(),
            title: c["title"].as_str().unwrap_or("").into(),
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

    /// Ramène la fenêtre `address` sur `rect` (coordonnées globales).
    pub fn take(address: &str, rect: (i32, i32, i32, i32)) -> Result<Guest, String> {
        let clients = ipc::query_json("clients").map_err(|e| e.to_string())?;
        let g = Self::read(&clients, address)
            .ok_or_else(|| crate::i18n::tr("fenêtre disparue").to_string())?;
        if let Err(e) = g.adopt(rect) {
            let _ = g.restore();
            return Err(e);
        }
        Ok(g)
    }

    fn tag(&self, tag: &str) -> Result<(), String> {
        ipc::dispatch(&format!(
            "hl.dsp.window.tag({{ tag = \"{tag}\", window = \"address:{}\" }})",
            ipc::lua_escape(&self.address)
        ))
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    fn adopt(&self, rect: (i32, i32, i32, i32)) -> Result<(), String> {
        // Étiquette du lecteur (cadre, sans bordure ni ombre), plus celle
        // des fenêtres empruntées : un hyprdesk relancé la libère au lieu de
        // la prendre pour un lecteur orphelin (qui serait fermé).
        super::video::prepare(&self.address)?;
        self.tag(&format!("+{}", super::capture::TAG))?;
        self.tag(&format!("+{TAG}"))?;
        // Plein écran demandé par l'application : il reste dans la zone.
        ipc::eval(crate::zones::runtime_lua()).map_err(|e| e.to_string())?;
        let r = ipc::repl(&adopt_lua(&self.address, rect)).map_err(|e| e.to_string())?;
        if r.trim().trim_matches('"') != "ok" {
            return Err(r.trim().to_string());
        }
        super::video::anchor(&self.address, rect)
    }

    /// Nouvelle place (zones inversées, écran tourné).
    pub fn place(&self, rect: (i32, i32, i32, i32)) -> Result<(), String> {
        super::video::anchor(&self.address, rect)?;
        crate::hypr::place(&self.address, rect)
    }

    /// Renouvelle le bail du cadre.
    pub fn renew(&self, rect: (i32, i32, i32, i32)) -> Result<(), String> {
        super::video::anchor(&self.address, rect)
    }

    /// Rend la fenêtre telle qu'elle était ; va au bout même si une étape
    /// échoue (la fenêtre a pu être fermée entre-temps).
    pub fn restore(&self) -> Result<(), String> {
        let a = ipc::lua_escape(&self.address);
        let results = [
            super::video::unanchor(&self.address),
            self.tag(&format!("-{}", super::video::TAG)),
            self.tag(&format!("-{}", super::capture::TAG)),
            self.tag(&format!("-{TAG}")),
            ipc::dispatch(&format!(
                "hl.dsp.window.pin({{ action = \"{}\", window = \"address:{a}\" }})",
                if self.pinned { "enable" } else { "disable" },
            ))
            .map(|_| ())
            .map_err(|e| e.to_string()),
            crate::hypr::move_window(&self.address, &self.workspace).map_err(|e| e.to_string()),
            crate::hypr::place(&self.address, self.rect),
            crate::hypr::set_floating(&self.address, self.floating),
            ipc::dispatch(&format!(
                "hl.dsp.window.fullscreen_state({{ internal = {}, client = {}, action = \"set\", window = \"address:{a}\" }})",
                self.fullscreen, self.client_fullscreen
            ))
            .map(|_| ())
            .map_err(|e| e.to_string()),
        ];
        results.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_l_etat_a_rendre() {
        let clients = serde_json::json!([
            {"address": "0x1", "class": "kitty", "title": "zsh",
             "workspace": {"name": "3"}, "at": [10, 20], "size": [800, 600],
             "floating": false, "pinned": false, "fullscreen": 1, "fullscreenClient": 0},
        ]);
        let g = Guest::read(&clients, "0x1").unwrap();
        assert_eq!(g.class, "kitty");
        assert_eq!(g.workspace, "3");
        assert_eq!(g.rect, (10, 20, 800, 600));
        assert!(!g.floating);
        assert_eq!(g.fullscreen, 1);
        assert!(Guest::read(&clients, "0x2").is_none());
    }

    #[test]
    fn le_bloc_lua_s_execute_dans_l_ordre() {
        let lua = mlua::Lua::new();
        lua.load(
            r#"
            calls = {}
            local function rec(kind) return function(t) t.kind = kind; return t end end
            hl = {
                dispatch = function(cmd) table.insert(calls, cmd.kind) end,
                dsp = { window = {
                    fullscreen_state = rec("fullscreen"), float = rec("float"),
                    pin = rec("pin"), resize = rec("resize"), move = rec("move"),
                } },
            }
            "#,
        )
        .exec()
        .unwrap();
        let code = adopt_lua("0x1", (1, 2, 3, 4));
        let body = code.strip_prefix("return ").unwrap();
        let r: String = lua.load(body).eval().unwrap();
        assert_eq!(r, "ok");
        let calls: Vec<String> = lua.globals().get("calls").unwrap();
        assert_eq!(calls, ["fullscreen", "float", "pin", "resize", "move"]);
    }

    #[test]
    fn le_bloc_lua_pose_la_fenetre() {
        let lua = adopt_lua("0xab", (100, 200, 640, 360));
        assert!(lua.contains("address:0xab"));
        assert!(lua.contains("x=640, y=360"));
        assert!(lua.contains("x=100, y=200"));
        // Plein écran levé avant le passage en flottant.
        assert!(lua.find("fullscreen_state") < lua.find("float("));
    }
}
