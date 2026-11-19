//! Icônes du Panorama dessinées par SmartUI (MLG-172) : tuiles des sources
//! vidéo et raccourcis tactiles, dans le style SmartUI, par le fournisseur IA
//! de la fonction « smartui » (LYRA par défaut, `crate::ai_services`).
//!
//! `hyprdesk panorama icons [sources|music|shortcuts|all] [--force]` dessine les
//! icônes manquantes (toutes avec `--force`), les valide dans le thème
//! SmartUI et écrit leur chemin dans la config (`icon` de chaque source ou
//! raccourci), une à une : l'écran les montre au fur et à mesure. Le bouton
//! des Réglages lance cette même commande.

use std::path::{Path, PathBuf};

use super::config::Source;
use super::design::Shortcut;

/// Ce qu'on dessine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Sources,
    /// Services de musique (profil Musique).
    Music,
    Shortcuts,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Vec<Kind>> {
        Some(match s {
            // Sources du lecteur : vidéo et musique (bouton des Réglages).
            "sources" => vec![Kind::Sources, Kind::Music],
            "video" => vec![Kind::Sources],
            "music" | "musique" => vec![Kind::Music],
            "shortcuts" | "raccourcis" => vec![Kind::Shortcuts],
            "all" | "" => vec![Kind::Sources, Kind::Music, Kind::Shortcuts],
            _ => return None,
        })
    }
}

/// Clé SmartUI d'une source (par hôte : une même adresse garde son icône).
pub fn source_key(s: &Source) -> String {
    format!(
        "panorama-video-{}",
        crate::smartui::slug(&super::video::host(&s.url))
    )
}

/// Clé SmartUI d'un raccourci (par nom).
pub fn shortcut_key(s: &Shortcut) -> String {
    format!("panorama-shortcut-{}", crate::smartui::slug(&s.name))
}

/// L'icône vient déjà de SmartUI.
pub fn is_smartui(icon: &str) -> bool {
    !icon.is_empty() && crate::smartui::owns(Path::new(icon))
}

/// Dessine, valide et rend le chemin de l'icône `key` (bloquant : un appel
/// au fournisseur IA). `what` décrit l'objet au modèle ; `original` : icône
/// actuelle, s'il y en a une.
fn draw(
    cfg: &crate::config::Config,
    key: &str,
    name: &str,
    what: &str,
    original: Option<&Path>,
) -> Result<PathBuf, String> {
    crate::smartui::generate(cfg, key, what, name, original)?;
    crate::smartui::validate(key, cfg.general.icon_theme.as_deref())
}

/// Icône d'origine d'un raccourci (nom d'icône ou chemin), pour le modèle.
fn original(icon: &str, cfg: &crate::config::Config) -> Option<PathBuf> {
    if icon.is_empty() || is_smartui(icon) {
        return None;
    }
    crate::launch::lookup_icon(icon, cfg.general.icon_theme.as_deref(), 128)
}

/// Ligne de progression : « 3/7 Netflix ».
pub type Progress<'a> = &'a mut dyn FnMut(usize, usize, &str);

/// Dessine les icônes manquantes (toutes avec `force`) et les écrit dans la
/// config, une à une. Rend le nombre d'icônes dessinées et les erreurs.
pub fn run(
    config_path: &Path,
    kinds: &[Kind],
    force: bool,
    progress: Progress<'_>,
) -> Result<(usize, Vec<String>), String> {
    let cfg = crate::config::lua::load_file(config_path)
        .map_err(|e| e.to_string())?
        .config;
    // Travail à faire : (genre, indice, clé, nom, description, original).
    let mut todo: Vec<(Kind, usize, String, String, String, Option<PathBuf>)> = Vec::new();
    let p = &cfg.general.panorama;
    if kinds.contains(&Kind::Sources) {
        for (i, s) in p.sources.iter().enumerate() {
            if force || !is_smartui(&s.icon) {
                todo.push((
                    Kind::Sources,
                    i,
                    source_key(s),
                    s.name.clone(),
                    format!(
                        "streaming service {} ({})",
                        s.name,
                        super::video::host(&s.url)
                    ),
                    None,
                ));
            }
        }
    }
    if kinds.contains(&Kind::Music) {
        for (i, s) in p.music_sources.iter().enumerate() {
            if force || !is_smartui(&s.icon) {
                todo.push((
                    Kind::Music,
                    i,
                    source_key(s),
                    s.name.clone(),
                    format!(
                        "music streaming service {} ({})",
                        s.name,
                        super::video::host(&s.url)
                    ),
                    None,
                ));
            }
        }
    }
    if kinds.contains(&Kind::Shortcuts) {
        for (i, s) in p.design.shortcuts.iter().enumerate() {
            if force || !is_smartui(&s.icon) {
                todo.push((
                    Kind::Shortcuts,
                    i,
                    shortcut_key(s),
                    s.name.clone(),
                    format!("touch shortcut \"{}\" running `{}`", s.name, s.command),
                    original(&s.icon, &cfg),
                ));
            }
        }
    }
    let total = todo.len();
    let mut done = 0;
    let mut errors = Vec::new();
    for (n, (kind, i, key, name, what, orig)) in todo.into_iter().enumerate() {
        progress(n + 1, total, &name);
        match draw(&cfg, &key, &name, &what, orig.as_deref()) {
            Ok(path) => {
                set_icon(config_path, kind, i, &name, &path.to_string_lossy())?;
                done += 1;
            }
            Err(e) => errors.push(format!("{name} : {e}")),
        }
    }
    Ok((done, errors))
}

/// Écrit l'icône dans la config (relue : l'utilisateur a pu changer la
/// liste pendant la génération ; l'élément est retrouvé par son nom).
fn set_icon(
    config_path: &Path,
    kind: Kind,
    i: usize,
    name: &str,
    icon: &str,
) -> Result<(), String> {
    let cfg = crate::config::lua::load_file(config_path)
        .map_err(|e| e.to_string())?
        .config;
    let p = &cfg.general.panorama;
    let (key, json) = match kind {
        Kind::Sources => {
            let mut l = p.sources.clone();
            let Some(s) = pick(&mut l, i, |s| s.name == name) else {
                return Ok(());
            };
            s.icon = icon.to_string();
            ("panorama.sources", sources_json(&l))
        }
        Kind::Music => {
            let mut l = p.music_sources.clone();
            let Some(s) = pick(&mut l, i, |s| s.name == name) else {
                return Ok(());
            };
            s.icon = icon.to_string();
            ("panorama.music_sources", sources_json(&l))
        }
        Kind::Shortcuts => {
            let mut l = p.design.shortcuts.clone();
            let Some(s) = pick(&mut l, i, |s| s.name == name) else {
                return Ok(());
            };
            s.icon = icon.to_string();
            ("panorama.design.shortcuts", shortcuts_json(&l))
        }
    };
    crate::cli_config::run(config_path, &["set".into(), key.into(), json]).map(|_| ())
}

/// L'élément `i` s'il porte toujours ce nom, sinon le premier qui le porte.
fn pick<T>(l: &mut [T], i: usize, same: impl Fn(&T) -> bool) -> Option<&mut T> {
    if l.get(i).is_some_and(&same) {
        return l.get_mut(i);
    }
    l.iter_mut().find(|x| same(x))
}

/// Sources en JSON (clé `panorama.sources`).
pub fn sources_json(l: &[Source]) -> String {
    serde_json::Value::Array(
        l.iter()
            .map(|s| {
                let mut o = serde_json::json!({ "name": s.name, "url": s.url });
                if !s.icon.is_empty() {
                    o["icon"] = s.icon.clone().into();
                }
                o
            })
            .collect(),
    )
    .to_string()
}

/// Lit `panorama.sources` en JSON : [{name, url, icon?}].
pub fn sources_from_json(v: &str) -> Result<Vec<Source>, String> {
    let l: Vec<serde_json::Value> = serde_json::from_str(v).map_err(|e| e.to_string())?;
    l.iter()
        .map(|o| {
            let url = o["url"].as_str().unwrap_or("").trim().to_string();
            if !url.starts_with("https://") && !url.starts_with("http://") {
                return Err(crate::tr!(
                    "source « {url} » : l'adresse doit commencer par https://",
                    url = url
                ));
            }
            let name = o["name"].as_str().unwrap_or("").trim().to_string();
            Ok(Source {
                name: if name.is_empty() {
                    super::video::host(&url)
                } else {
                    name
                },
                url,
                icon: o["icon"].as_str().unwrap_or("").trim().to_string(),
            })
        })
        .collect()
}

fn shortcuts_json(l: &[Shortcut]) -> String {
    serde_json::Value::Array(
        l.iter()
            .map(|s| serde_json::json!({ "name": s.name, "icon": s.icon, "command": s.command }))
            .collect(),
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_des_sources() {
        let l = vec![
            Source {
                name: "Netflix".into(),
                url: "https://www.netflix.com".into(),
                icon: String::new(),
            },
            Source {
                name: "Free \"TV\"".into(),
                url: "https://tv.free.fr".into(),
                icon: "/x/free.svg".into(),
            },
        ];
        assert_eq!(sources_from_json(&sources_json(&l)).unwrap(), l);
        assert!(sources_from_json(r#"[{"name":"x","url":"file:///etc"}]"#).is_err());
        let l = sources_from_json(r#"[{"url":"https://www.youtube.com"}]"#).unwrap();
        assert_eq!(l[0].name, "www.youtube.com");
        assert_eq!(source_key(&l[0]), "panorama-video-www.youtube.com");
        assert_eq!(Kind::parse("all").unwrap().len(), 3);
        assert!(Kind::parse("x").is_none());
    }
}
