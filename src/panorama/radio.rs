//! Webradio et recherche de musique du Panorama (MLG-172).
//!
//! - Onglet Webradio du profil Musique : radios favorites, et recherche dans
//!   l'annuaire libre Radio Browser (sans clé ni compte), ou un service web
//!   (Radio Garden, radio.net, TuneIn) ouvert dans le lecteur.
//! - Recherche sur la plateforme de musique active (Spotify, Deezer, YouTube
//!   Music…) : l'adresse de sa page de résultats, que LYRA ouvre d'une
//!   demande (« mets du jazz calme »).
//!
//! Un flux de radio s'ouvre comme une source : dans le lecteur du Panorama
//! (une seule source à la fois, commandes MPRIS du navigateur).

use super::config::Source;

/// Services de webradio : (identifiant, libellé, page web ; `None` : annuaire
/// Radio Browser, affiché dans l'onglet).
pub const SERVICES: &[(&str, &str, Option<&str>)] = &[
    ("radio-browser", "Radio Browser", None),
    ("radio-garden", "Radio Garden", Some("https://radio.garden")),
    ("radio-net", "radio.net", Some("https://www.radio.net")),
    ("tunein", "TuneIn", Some("https://tunein.com")),
];

pub const DEFAULT_SERVICE: &str = "radio-browser";

/// Service par identifiant.
pub fn service(id: &str) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    SERVICES.iter().copied().find(|s| s.0 == id)
}

/// Radios favorites par défaut (flux publics).
pub fn default_radios() -> Vec<Source> {
    [
        ("FIP", "https://icecast.radiofrance.fr/fip-hifi.aac"),
        (
            "France Inter",
            "https://icecast.radiofrance.fr/franceinter-hifi.aac",
        ),
        (
            "franceinfo",
            "https://icecast.radiofrance.fr/franceinfo-hifi.aac",
        ),
        (
            "France Culture",
            "https://icecast.radiofrance.fr/franceculture-hifi.aac",
        ),
        (
            "France Musique",
            "https://icecast.radiofrance.fr/francemusique-hifi.aac",
        ),
        (
            "Radio Nova",
            "https://novazz.ice.infomaniak.ch/novazz-128.mp3",
        ),
        (
            "TSF Jazz",
            "https://tsfjazz.ice.infomaniak.ch/tsfjazz-high.mp3",
        ),
    ]
    .iter()
    .map(|(n, u)| Source {
        name: n.to_string(),
        url: u.to_string(),
        ..Default::default()
    })
    .collect()
}

/// Encodage d'un paramètre d'adresse.
pub fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.trim().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Page de résultats de `query` sur la plateforme de `source_url` (hôte
/// connu) ; `None` : plateforme sans recherche connue.
pub fn search_url(source_url: &str, query: &str) -> Option<String> {
    let q = encode(query);
    let host = super::video::host(source_url);
    Some(match host.as_str() {
        h if h.ends_with("spotify.com") => format!("https://open.spotify.com/search/{q}"),
        h if h.ends_with("deezer.com") => format!("https://www.deezer.com/search/{q}"),
        "music.youtube.com" => format!("https://music.youtube.com/search?q={q}"),
        h if h.ends_with("youtube.com") => {
            format!("https://www.youtube.com/results?search_query={q}")
        }
        h if h.ends_with("qobuz.com") => format!("https://play.qobuz.com/search?q={q}"),
        h if h.ends_with("radiofrance.fr") => {
            format!("https://www.radiofrance.fr/recherche?term={q}")
        }
        h if h.ends_with("soundcloud.com") => format!("https://soundcloud.com/search?q={q}"),
        _ => return None,
    })
}

/// Recherche dans Radio Browser : par nom, puis par genre (tag) s'il n'y a
/// rien ; les plus écoutées d'abord. Bloquant (requête HTTP, 10 s au plus).
pub fn search(query: &str) -> Result<Vec<Source>, String> {
    let q = encode(query);
    let mut out = fetch(&format!(
        "stations/search?name={q}&hidebroken=true&order=clickcount&reverse=true&limit=30"
    ))?;
    if out.is_empty() {
        out = fetch(&format!(
            "stations/search?tag={}&hidebroken=true&order=clickcount&reverse=true&limit=30",
            encode(&query.to_lowercase())
        ))?;
    }
    Ok(out)
}

fn fetch(path: &str) -> Result<Vec<Source>, String> {
    let out = std::process::Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "10",
            "-A",
            concat!("hyprdesk/", env!("CARGO_PKG_VERSION")),
            &format!("https://all.api.radio-browser.info/json/{path}"),
        ])
        .output()
        .map_err(|e| format!("curl : {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "Radio Browser : {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

/// Stations de Radio Browser (JSON) : nom, flux, logo ; sans doublon.
fn parse(json: &str) -> Result<Vec<Source>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out: Vec<Source> = Vec::new();
    for s in v.as_array().into_iter().flatten() {
        let url = s["url_resolved"]
            .as_str()
            .filter(|u| !u.is_empty())
            .or(s["url"].as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let name = s["name"].as_str().unwrap_or("").trim().to_string();
        if !(url.starts_with("https://") || url.starts_with("http://"))
            || name.is_empty()
            || out.iter().any(|o| o.url == url || o.name == name)
        {
            continue;
        }
        out.push(Source {
            name,
            url,
            ..Default::default()
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresses_de_recherche() {
        assert_eq!(
            search_url("https://music.youtube.com", "jazz calme").unwrap(),
            "https://music.youtube.com/search?q=jazz%20calme"
        );
        assert_eq!(
            search_url("https://open.spotify.com", "Été 85").unwrap(),
            "https://open.spotify.com/search/%C3%89t%C3%A9%2085"
        );
        assert!(search_url("https://example.org", "x").is_none());
        assert!(service("radio-garden").is_some() && service("x").is_none());
    }

    #[test]
    fn stations_lues() {
        let l = parse(
            r#"[{"name":"FIP","url":"http://x","url_resolved":"https://icecast.radiofrance.fr/fip-hifi.aac"},
                {"name":"FIP","url_resolved":"https://autre"},
                {"name":"Vide","url_resolved":"ftp://x"}]"#,
        )
        .unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].url, "https://icecast.radiofrance.fr/fip-hifi.aac");
    }
}
