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

/// Genres proposés dans l'onglet Webradio : (étiquette Radio Browser, libellé).
pub const GENRES: &[(&str, &str)] = &[
    ("jazz", "Jazz"),
    ("rock", "Rock"),
    ("pop", "Pop"),
    ("classical", "Musique classique"),
    ("electronic", "Électro"),
    ("hip-hop", "Hip-hop"),
    ("chillout", "Chill"),
    ("news", "Info"),
];

/// Pays de la session (`fr_FR.UTF-8` → `FR`), pour les radios les plus
/// écoutées ; vide s'il n'est pas lisible.
pub fn local_country() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.is_empty() && v != "C" && v != "POSIX")
        .and_then(|v| {
            let cc = v
                .split(['.', '@'])
                .next()?
                .split('_')
                .nth(1)?
                .to_uppercase();
            (cc.len() == 2 && cc.chars().all(|c| c.is_ascii_alphabetic())).then_some(cc)
        })
        .unwrap_or_default()
}

/// Libellé d'une demande : `#tag` (genre), `@FR` (les plus écoutées d'un
/// pays, `@` seul : du monde), sinon le texte cherché.
pub fn query_label(q: &str) -> String {
    if let Some(tag) = q.strip_prefix('#') {
        GENRES
            .iter()
            .find(|(t, _)| *t == tag)
            .map(|(_, l)| crate::i18n::tr(l).to_string())
            .unwrap_or_else(|| tag.to_string())
    } else if let Some(cc) = q.strip_prefix('@') {
        if cc.is_empty() {
            crate::i18n::tr("Les plus écoutées").to_string()
        } else {
            crate::tr!("Les plus écoutées ({cc})", cc = cc.to_string())
        }
    } else {
        q.to_string()
    }
}

/// Recherche dans Radio Browser, les plus écoutées d'abord : `#tag` (genre),
/// `@FR` (un pays, `@` : le monde), sinon par nom, puis par genre s'il n'y a
/// rien. Les logos sont téléchargés (cache). Bloquant (requêtes HTTP).
pub fn search(query: &str) -> Result<Vec<Source>, String> {
    let q = query.trim();
    const ORDER: &str = "hidebroken=true&order=clickcount&reverse=true&limit=40";
    let mut out = if let Some(tag) = q.strip_prefix('#') {
        fetch(&format!(
            "stations/search?tag={}&tagExact=true&{ORDER}",
            encode(&tag.to_lowercase())
        ))?
    } else if let Some(cc) = q.strip_prefix('@') {
        let cc = cc.trim();
        if cc.is_empty() {
            fetch(&format!("stations/search?{ORDER}"))?
        } else {
            fetch(&format!(
                "stations/search?countrycode={}&{ORDER}",
                encode(&cc.to_uppercase())
            ))?
        }
    } else {
        let byname = fetch(&format!("stations/search?name={}&{ORDER}", encode(q)))?;
        if byname.is_empty() {
            fetch(&format!(
                "stations/search?tag={}&{ORDER}",
                encode(&q.to_lowercase())
            ))?
        } else {
            byname
        }
    };
    with_logos(&mut out);
    Ok(out.into_iter().map(|s| s.source).collect())
}

/// Logos des radios favorites sans icône, cherchés dans Radio Browser (par
/// flux, puis par nom) : (flux, chemin du logo). Bloquant.
pub fn favourite_logos(radios: &[Source]) -> Vec<(String, String)> {
    let mut want: Vec<Station> = Vec::new();
    for r in radios.iter().filter(|r| r.icon.is_empty()) {
        let by = |path: String| {
            fetch(&path)
                .unwrap_or_default()
                .into_iter()
                .find(|s| !s.favicon.is_empty())
        };
        let found = by(format!("stations/byurl?url={}", encode(&r.url)))
            .or_else(|| {
                by(format!(
                    "stations/search?name={}&nameExact=true&order=clickcount&reverse=true&limit=10",
                    encode(&r.name)
                ))
            })
            .or_else(|| {
                by(format!(
                    "stations/search?name={}&order=clickcount&reverse=true&limit=10",
                    encode(&r.name)
                ))
            });
        if let Some(f) = found {
            want.push(Station {
                source: r.clone(),
                favicon: f.favicon,
            });
        }
    }
    with_logos(&mut want);
    want.into_iter()
        .filter(|s| !s.source.icon.is_empty())
        .map(|s| (s.source.url, s.source.icon))
        .collect()
}

/// Station lue dans Radio Browser : la source et l'adresse de son logo.
struct Station {
    source: Source,
    favicon: String,
}

fn fetch(path: &str) -> Result<Vec<Station>, String> {
    let out = curl(
        &format!("https://all.api.radio-browser.info/json/{path}"),
        10,
    )?;
    parse(&String::from_utf8_lossy(&out))
}

fn curl(url: &str, secs: u32) -> Result<Vec<u8>, String> {
    let out = std::process::Command::new("curl")
        .args([
            "-sSfL",
            "--max-time",
            &secs.to_string(),
            "--max-filesize",
            "2000000",
            "-A",
            concat!("hyprdesk/", env!("CARGO_PKG_VERSION")),
            url,
        ])
        .output()
        .map_err(|e| format!("curl : {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "Radio Browser : {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}

/// Stations de Radio Browser (JSON) : nom, flux, logo ; sans doublon.
fn parse(json: &str) -> Result<Vec<Station>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out: Vec<Station> = Vec::new();
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
            || out
                .iter()
                .any(|o| o.source.url == url || o.source.name == name)
        {
            continue;
        }
        let favicon = s["favicon"]
            .as_str()
            .map(str::trim)
            .filter(|f| f.starts_with("https://") || f.starts_with("http://"))
            .unwrap_or("")
            .to_string();
        out.push(Station {
            source: Source {
                name,
                url,
                ..Default::default()
            },
            favicon,
        });
    }
    Ok(out)
}

/// Télécharge les logos (en parallèle, cache) et les pose en icône.
fn with_logos(list: &mut [Station]) {
    let found: Vec<Option<String>> = std::thread::scope(|sc| {
        let jobs: Vec<_> = list
            .iter()
            .map(|s| {
                let f = s.favicon.clone();
                sc.spawn(move || logo(&f))
            })
            .collect();
        jobs.into_iter().map(|j| j.join().ok().flatten()).collect()
    });
    for (s, p) in list.iter_mut().zip(found) {
        if let Some(p) = p {
            s.source.icon = p;
        }
    }
}

fn logo_dir() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
        })
        .join("hyprdesk/radio-logos")
}

/// Logo d'une station, en cache : SVG gardé tel quel, image ramenée à
/// 128 px en PNG. `None` : pas de logo, ou illisible (retenu, pas retenté).
fn logo(favicon: &str) -> Option<String> {
    use std::hash::{Hash, Hasher};
    if favicon.is_empty() {
        return None;
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    favicon.hash(&mut h);
    let dir = logo_dir();
    let key = format!("{:016x}", h.finish());
    let (png, svg, none) = (
        dir.join(format!("{key}.png")),
        dir.join(format!("{key}.svg")),
        dir.join(format!("{key}.none")),
    );
    for p in [&png, &svg] {
        if p.is_file() {
            return Some(p.display().to_string());
        }
    }
    if none.exists() {
        return None;
    }
    let _ = std::fs::create_dir_all(&dir);
    let bytes = curl(favicon, 6)
        .ok()
        .filter(|b| !b.is_empty())
        .or_else(|| curl(&wikimedia_original(favicon)?, 6).ok());
    let saved = bytes.and_then(|b| {
        let head = String::from_utf8_lossy(&b[..b.len().min(512)]).to_lowercase();
        if head.contains("<svg") {
            std::fs::write(&svg, &b).ok()?;
            return Some(svg.display().to_string());
        }
        let img = image::load_from_memory(&b).ok()?;
        img.thumbnail(128, 128).save(&png).ok()?;
        Some(png.display().to_string())
    });
    if saved.is_none() {
        let _ = std::fs::write(&none, b"");
    }
    saved
}

/// Vignette Wikimedia (`…/thumb/a/ab/Logo.svg/1024px-Logo.svg.png`) : son
/// original (`…/a/ab/Logo.svg`). Wikimedia refuse les tailles de vignette
/// non standard, que l'annuaire contient souvent.
fn wikimedia_original(url: &str) -> Option<String> {
    if !url.contains("upload.wikimedia.org/") {
        return None;
    }
    let (head, rest) = url.split_once("/thumb/")?;
    let (orig, _) = rest.rsplit_once('/')?;
    Some(format!("{head}/{orig}"))
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
        assert_eq!(
            l[0].source.url,
            "https://icecast.radiofrance.fr/fip-hifi.aac"
        );
        assert_eq!(l[0].favicon, "");
        let l =
            parse(r#"[{"name":"X","url":"https://x","favicon":"https://x/logo.png"}]"#).unwrap();
        assert_eq!(l[0].favicon, "https://x/logo.png");
    }

    /// Réseau : `cargo test -- --ignored annuaire_reel`.
    #[test]
    #[ignore]
    fn annuaire_reel() {
        for q in ["fip", "#jazz", "@FR"] {
            let l = search(q).unwrap();
            assert!(!l.is_empty(), "{q}");
            let logos = l
                .iter()
                .filter(|s| std::path::Path::new(&s.icon).is_file())
                .count();
            eprintln!("{q} : {} stations, {logos} logos", l.len());
            assert!(logos > 0, "{q} : aucun logo");
        }
        let found = favourite_logos(&default_radios());
        eprintln!("favoris : {found:?}");
        assert!(found.len() >= 4);
    }

    #[test]
    fn vignette_wikimedia() {
        assert_eq!(
            wikimedia_original(
                "https://upload.wikimedia.org/wikipedia/fr/thumb/d/d5/FIP_logo_2005.svg/1024px-FIP_logo_2005.svg.png"
            )
            .as_deref(),
            Some("https://upload.wikimedia.org/wikipedia/fr/d/d5/FIP_logo_2005.svg")
        );
        assert!(wikimedia_original("https://x/thumb/a/b.png").is_none());
    }

    #[test]
    fn libelles_des_demandes() {
        assert_eq!(query_label("fip"), "fip");
        assert_eq!(query_label("#jazz"), "Jazz");
        assert_eq!(query_label("#inconnu"), "inconnu");
        assert!(query_label("@FR").contains("FR"));
    }
}
