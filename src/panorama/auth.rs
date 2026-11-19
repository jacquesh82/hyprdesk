//! Plateformes visibles dans le Panorama (MLG-172) : celles qui marchent
//! sans compte, et celles où la personne est connectée.
//!
//! La connexion se lit dans le profil du navigateur des sources : présence
//! d'un cookie de session connu pour le domaine (noms seulement, jamais les
//! valeurs ; base copiée puis ouverte en lecture seule). Une plateforme sans
//! cookie reconnaissable reste visible ; chaque source peut être forcée
//! (`show = "on" | "off"`) dans les Réglages.

use std::collections::HashSet;
use std::path::PathBuf;

use super::config::Source;

/// Plateformes à compte : (domaine, cookies de session ; vide : connexion
/// non lisible).
const ACCOUNTS: &[(&str, &[&str])] = &[
    ("netflix.com", &["NetflixId", "SecureNetflixId"]),
    ("spotify.com", &["sp_dc"]),
    ("deezer.com", &["arl"]),
    ("primevideo.com", &["at-main-av", "x-main-av"]),
    ("hbomax.com", &["st"]),
    ("max.com", &["st"]),
    ("canalplus.com", &["p_pass_token", "passToken"]),
    ("disneyplus.com", &[]),
    ("qobuz.com", &[]),
    ("tv.free.fr", &[]),
    ("tunein.com", &[]),
];

/// La source demande un compte (d'après son domaine).
pub fn needs_account(url: &str) -> bool {
    rule(url).is_some()
}

fn rule(url: &str) -> Option<&'static (&'static str, &'static [&'static str])> {
    let host = super::video::host(url);
    ACCOUNTS
        .iter()
        .find(|(d, _)| host == *d || host.ends_with(&format!(".{d}")))
}

/// Base de cookies du profil par défaut du navigateur (commande).
fn cookie_db(browser: &str) -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let cfg = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let dir = match super::video::prefix_of(browser).as_str() {
        "brave" => "BraveSoftware/Brave-Browser",
        "chromium" => "chromium",
        "chrome" => "google-chrome",
        "vivaldi" => "vivaldi",
        "msedge" => "microsoft-edge",
        _ => return None,
    };
    let base = cfg.join(dir).join("Default");
    [base.join("Network/Cookies"), base.join("Cookies")]
        .into_iter()
        .find(|p| p.is_file())
}

/// Cookies présents (domaine, nom) dans le profil du navigateur. Bloquant.
pub fn cookies(browser: &str) -> Option<HashSet<(String, String)>> {
    let db = cookie_db(browser)?;
    // Copie : le navigateur garde sa base ouverte (et son journal WAL).
    let tmp = std::env::temp_dir().join(format!("hyprdesk-cookies-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).ok()?;
    let copy = tmp.join("Cookies");
    std::fs::copy(&db, &copy).ok()?;
    for ext in ["-wal", "-journal"] {
        let side = PathBuf::from(format!("{}{ext}", db.display()));
        if side.is_file() {
            let _ = std::fs::copy(&side, PathBuf::from(format!("{}{ext}", copy.display())));
        }
    }
    let read = || -> rusqlite::Result<HashSet<(String, String)>> {
        let conn = rusqlite::Connection::open_with_flags(
            &copy,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let mut st = conn.prepare("SELECT host_key, name FROM cookies")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect()
    };
    let out = read().ok();
    let _ = std::fs::remove_dir_all(&tmp);
    out
}

/// Connexion lue pour la source : `Some(true)` connecté, `Some(false)` pas
/// connecté, `None` : sans compte, ou connexion non lisible.
pub fn signed_in(cookies: &HashSet<(String, String)>, url: &str) -> Option<bool> {
    let (domain, names) = rule(url)?;
    if names.is_empty() {
        return None;
    }
    Some(cookies.iter().any(|(host, name)| {
        let h = host.trim_start_matches('.');
        (h == *domain || h.ends_with(&format!(".{domain}"))) && names.contains(&name.as_str())
    }))
}

/// La source s'affiche dans le Panorama : forcée, ou sans compte, ou
/// connectée (ou connexion non lisible).
pub fn visible(s: &Source, signed: Option<bool>) -> bool {
    match s.show.as_str() {
        "on" => true,
        "off" => false,
        _ => !needs_account(&s.url) || signed != Some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connexions_lues() {
        let c: HashSet<(String, String)> = [
            (".netflix.com", "NetflixId"),
            (".spotify.com", "sp_t"),
            (".youtube.com", "SAPISID"),
        ]
        .iter()
        .map(|(h, n)| (h.to_string(), n.to_string()))
        .collect();
        assert_eq!(signed_in(&c, "https://www.netflix.com"), Some(true));
        assert_eq!(signed_in(&c, "https://open.spotify.com"), Some(false));
        assert_eq!(signed_in(&c, "https://www.disneyplus.com"), None);
        assert_eq!(signed_in(&c, "https://www.youtube.com"), None);
        let mut s = Source {
            url: "https://open.spotify.com".into(),
            ..Default::default()
        };
        assert!(!visible(&s, Some(false)));
        assert!(visible(&s, None));
        s.show = "on".into();
        assert!(visible(&s, Some(false)));
        s.url = "https://www.youtube.com".into();
        s.show = "off".into();
        assert!(!visible(&s, None));
    }
}
