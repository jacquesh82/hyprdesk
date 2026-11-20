//! Module Hue : lumières Philips Hue par l'API locale du pont (v1, HTTP sur le réseau
//! local) : découverte du pont, appairage (bouton du pont), pièces et zones,
//! allumage et luminosité. Les requêtes passent par `curl`, comme la
//! webradio : pas de client HTTP embarqué.
//!
//! Le pont et la clé d'appairage sont gardés dans
//! `$XDG_STATE_HOME/hyprdesk/hue.json` (droits 0600), pas dans la
//! configuration partagée : la clé ouvre les lumières de la maison.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Pièces masquées dans la tuile (option `hue.hidden` : identifiants
/// séparés par des virgules).
pub fn hidden(cfg: &crate::config::Config) -> Vec<String> {
    super::option(cfg, "hue", "hidden")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Pont appairé : adresse et clé (`username` de l'API v1).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Bridge {
    pub ip: String,
    pub user: String,
}

/// Pièce ou zone du pont, et ses lampes.
#[derive(Debug, Clone, PartialEq)]
pub struct Room {
    pub id: String,
    pub name: String,
    /// Au moins une lampe allumée.
    pub on: bool,
    /// Luminosité de la dernière commande, 1 à 254.
    pub bri: u8,
    /// `Room`, `Zone`…
    pub kind: String,
    pub lights: Vec<Light>,
}

/// Lampe du pont.
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    pub id: String,
    pub name: String,
    pub on: bool,
    /// 1 à 254 (lampes sans variateur : 254).
    pub bri: u8,
    /// Joignable par le pont (sinon : hors tension, hors portée).
    pub reachable: bool,
}

pub fn state_path() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        })
        .join("hyprdesk")
        .join("hue.json")
}

/// Pont enregistré (aucun : pas encore appairé).
pub fn load() -> Option<Bridge> {
    let s = std::fs::read_to_string(state_path()).ok()?;
    serde_json::from_str::<Bridge>(&s)
        .ok()
        .filter(|b| !b.ip.is_empty() && !b.user.is_empty())
}

pub fn save(b: &Bridge) -> Result<(), String> {
    let p = state_path();
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let body = serde_json::to_string_pretty(b).map_err(|e| e.to_string())?;
    std::fs::write(&p, body).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Oublie le pont.
pub fn forget() {
    let _ = std::fs::remove_file(state_path());
}

/// Requête au pont (ou au service de découverte) : `method` GET, POST ou PUT.
fn curl(method: &str, url: &str, body: Option<&str>) -> Result<serde_json::Value, String> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-sS", "--max-time", "5", "-X", method, url]);
    if let Some(b) = body {
        cmd.args(["-H", "Content-Type: application/json", "--data-binary", b]);
    }
    let out = cmd.output().map_err(|e| format!("curl : {e}"))?;
    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if e.is_empty() {
            crate::i18n::tr("pont Hue injoignable").to_string()
        } else {
            e
        });
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}

/// Ponts du réseau local, d'après le service de découverte de Philips.
pub fn discover() -> Result<Vec<String>, String> {
    curl("GET", "https://discovery.meethue.com/", None).map(|v| parse_discovery(&v))
}

pub fn parse_discovery(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|b| b["internalipaddress"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Appairage : à lancer dans les 30 s qui suivent l'appui sur le bouton du
/// pont. Rend la clé.
pub fn pair(ip: &str) -> Result<String, String> {
    let host = std::env::var("HOSTNAME")
        .ok()
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "desktop".into());
    let body = serde_json::json!({ "devicetype": format!("hyprdesk#{}", host.chars().take(19).collect::<String>()) });
    let v = curl("POST", &format!("http://{ip}/api"), Some(&body.to_string()))?;
    parse_pair(&v)
}

pub fn parse_pair(v: &serde_json::Value) -> Result<String, String> {
    let first = &v[0];
    if let Some(u) = first["success"]["username"].as_str() {
        return Ok(u.to_string());
    }
    match first["error"]["type"].as_i64() {
        Some(101) => Err(crate::i18n::tr(
            "Appuyez sur le bouton du pont Hue, puis relancez l'appairage dans les 30 secondes.",
        )
        .to_string()),
        _ => Err(first["error"]["description"]
            .as_str()
            .unwrap_or("réponse inattendue du pont")
            .to_string()),
    }
}

/// Pièces et zones, par nom, avec leurs lampes.
pub fn rooms(b: &Bridge) -> Result<Vec<Room>, String> {
    let groups = curl(
        "GET",
        &format!("http://{}/api/{}/groups", b.ip, b.user),
        None,
    )?;
    let lights = curl(
        "GET",
        &format!("http://{}/api/{}/lights", b.ip, b.user),
        None,
    )?;
    parse_rooms(&groups, &lights)
}

pub fn parse_rooms(v: &serde_json::Value, lights: &serde_json::Value) -> Result<Vec<Room>, String> {
    for r in [v, lights] {
        if let Some(e) = r[0]["error"]["description"].as_str() {
            return Err(e.to_string());
        }
    }
    let Some(map) = v.as_object() else {
        return Err("réponse inattendue du pont".into());
    };
    let light = |id: &str| -> Option<Light> {
        let l = &lights[id];
        l.is_object().then(|| Light {
            id: id.to_string(),
            name: l["name"].as_str().unwrap_or(id).to_string(),
            on: l["state"]["on"].as_bool().unwrap_or(false),
            bri: l["state"]["bri"].as_u64().unwrap_or(254).clamp(1, 254) as u8,
            reachable: l["state"]["reachable"].as_bool().unwrap_or(true),
        })
    };
    let mut list: Vec<Room> = map
        .iter()
        .filter(|(_, g)| matches!(g["type"].as_str(), Some("Room" | "Zone")))
        .map(|(id, g)| Room {
            id: id.clone(),
            name: g["name"].as_str().unwrap_or(id).to_string(),
            on: g["state"]["any_on"].as_bool().unwrap_or(false),
            bri: g["action"]["bri"].as_u64().unwrap_or(254).clamp(1, 254) as u8,
            kind: g["type"].as_str().unwrap_or_default().to_string(),
            lights: {
                let mut l: Vec<Light> = g["lights"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|i| light(i.as_str()?)).collect())
                    .unwrap_or_default();
                l.sort_by(|a, b| a.name.cmp(&b.name));
                l
            },
        })
        .collect();
    // Les pièces d'abord, puis les zones ; chacune par nom.
    list.sort_by(|a, b| (a.kind != "Room", &a.name).cmp(&(b.kind != "Room", &b.name)));
    Ok(list)
}

/// Allume, éteint ou règle la luminosité (1 à 254) d'une pièce.
pub fn set(b: &Bridge, room: &str, on: Option<bool>, bri: Option<u8>) -> Result<(), String> {
    let mut body = serde_json::Map::new();
    if let Some(on) = on {
        body.insert("on".into(), on.into());
    }
    if let Some(bri) = bri {
        body.insert("on".into(), true.into());
        body.insert("bri".into(), bri.clamp(1, 254).into());
    }
    let v = curl(
        "PUT",
        &format!("http://{}/api/{}/groups/{room}/action", b.ip, b.user),
        Some(&serde_json::Value::Object(body).to_string()),
    )?;
    match v[0]["error"]["description"].as_str() {
        Some(e) => Err(e.to_string()),
        None => Ok(()),
    }
}

/// Allume, éteint ou règle la luminosité (1 à 254) d'une lampe.
pub fn set_light(b: &Bridge, light: &str, on: Option<bool>, bri: Option<u8>) -> Result<(), String> {
    let mut body = serde_json::Map::new();
    if let Some(on) = on {
        body.insert("on".into(), on.into());
    }
    if let Some(bri) = bri {
        body.insert("on".into(), true.into());
        body.insert("bri".into(), bri.clamp(1, 254).into());
    }
    let v = curl(
        "PUT",
        &format!("http://{}/api/{}/lights/{light}/state", b.ip, b.user),
        Some(&serde_json::Value::Object(body).to_string()),
    )?;
    match v[0]["error"]["description"].as_str() {
        Some(e) => Err(e.to_string()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decouverte() {
        let v = json!([{"id":"001788fffe","internalipaddress":"192.168.1.20","port":443}]);
        assert_eq!(parse_discovery(&v), vec!["192.168.1.20".to_string()]);
        assert!(parse_discovery(&json!({})).is_empty());
    }

    #[test]
    fn appairage() {
        let ok = json!([{"success":{"username":"abc123"}}]);
        assert_eq!(parse_pair(&ok).unwrap(), "abc123");
        let wait =
            json!([{"error":{"type":101,"address":"","description":"link button not pressed"}}]);
        assert!(parse_pair(&wait).unwrap_err().contains("bouton"));
    }

    #[test]
    fn pieces() {
        let v = json!({
            "1": {"name":"Salon","type":"Room","lights":["11","10"],"state":{"any_on":true},"action":{"bri":200}},
            "2": {"name":"Entrée","type":"Zone","lights":[],"state":{"any_on":false},"action":{"bri":0}},
            "3": {"name":"Groupe","type":"LightGroup","state":{"any_on":true},"action":{}},
            "4": {"name":"Cuisine","type":"Room","lights":["12","99"],"state":{"any_on":false},"action":{"bri":80}}
        });
        let lights = json!({
            "10": {"name":"Lampadaire","state":{"on":true,"bri":120,"reachable":true}},
            "11": {"name":"Applique","state":{"on":false,"reachable":false}},
            "12": {"name":"Plafond","state":{"on":false,"bri":254,"reachable":true}}
        });
        let r = parse_rooms(&v, &lights).unwrap();
        let names: Vec<&str> = r.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Cuisine", "Salon", "Entrée"]);
        assert!(r[1].on && r[1].bri == 200);
        assert_eq!(r[2].bri, 1);
        // Lampes de la pièce, par nom ; une lampe inconnue du pont est omise.
        let salon: Vec<&str> = r[1].lights.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(salon, ["Applique", "Lampadaire"]);
        assert!(!r[1].lights[0].reachable && r[1].lights[1].bri == 120);
        assert_eq!(r[0].lights.len(), 1);
        let err = json!([{"error":{"type":1,"description":"unauthorized user"}}]);
        assert_eq!(parse_rooms(&err, &lights).unwrap_err(), "unauthorized user");
    }
}
