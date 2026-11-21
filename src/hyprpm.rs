//! Plugins de Hyprland tenus par `hyprpm` (Réglages → Configuration de
//! Hyprland → Plugins Hyprland) : dépôts et plugins installés, activés ou
//! non, plugins chargés dans la session, et le `hyprpm reload` du démarrage
//! qui les recharge à chaque session.
//!
//! Activer ou couper un plugin passe par `hyprpm enable|disable` puis
//! `hyprpm reload` ; ajouter un dépôt ou mettre à jour compile les en-têtes
//! de Hyprland et demande les droits d'administration : ces deux-là
//! s'ouvrent dans un terminal.

use std::process::Command;

/// Plugin d'un dépôt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub enabled: bool,
}

/// Dépôt installé par `hyprpm add`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub name: String,
    pub author: String,
    pub plugins: Vec<Plugin>,
}

/// État relevé : dépôts, plugins chargés, `hyprpm reload` au démarrage.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// `None` : hyprpm absent.
    pub repos: Option<Result<Vec<Repo>, String>>,
    /// Noms des plugins chargés dans Hyprland (`hyprctl plugin list`).
    pub loaded: Vec<String>,
    /// Une commande de démarrage de la session lance `hyprpm reload`.
    pub at_start: bool,
}

/// Commande de démarrage qui recharge les plugins activés.
pub const RELOAD_AT_START: &str = "hyprpm reload -n";

/// Retire les séquences de couleur ANSI.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            if it.peek() == Some(&'[') {
                it.next();
                for d in it.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Lit la sortie de `hyprpm list`.
pub fn parse_list(out: &str) -> Vec<Repo> {
    let mut repos: Vec<Repo> = Vec::new();
    for l in strip_ansi(out).lines() {
        let t = l.trim_start_matches(|c: char| !c.is_alphanumeric());
        if let Some(rest) = t.strip_prefix("Repository ") {
            // « hyprland-plugins (by hyprwm): »
            let rest = rest.trim_end().trim_end_matches(':');
            let (name, author) = match rest.split_once(" (by ") {
                Some((n, a)) => (n.trim(), a.trim_end_matches(')').trim()),
                None => (rest.trim(), ""),
            };
            repos.push(Repo {
                name: name.into(),
                author: author.into(),
                plugins: Vec::new(),
            });
        } else if let Some(name) = t.strip_prefix("Plugin ")
            && let Some(r) = repos.last_mut()
        {
            r.plugins.push(Plugin {
                name: name.trim().into(),
                enabled: false,
            });
        } else if let Some(v) = t.strip_prefix("enabled:")
            && let Some(p) = repos.last_mut().and_then(|r| r.plugins.last_mut())
        {
            p.enabled = v.trim() == "true";
        }
    }
    repos
}

/// Plugins chargés dans Hyprland.
pub fn loaded() -> Vec<String> {
    let Ok(o) = Command::new("hyprctl")
        .args(["-j", "plugin", "list"])
        .output()
    else {
        return Vec::new();
    };
    serde_json::from_slice::<serde_json::Value>(&o.stdout)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|p| p["name"].as_str().map(str::to_string))
        .collect()
}

/// Relevé complet (bloquant : lance hyprpm et hyprctl).
pub fn read(config_path: &std::path::Path) -> State {
    let repos = crate::launch::executable("hyprpm").then(|| {
        Command::new("hyprpm")
            .arg("list")
            .output()
            .map_err(|e| e.to_string())
            .and_then(|o| {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                if o.status.success() || out.contains("Repository") {
                    Ok(parse_list(&out))
                } else {
                    Err(strip_ansi(&String::from_utf8_lossy(&o.stderr))
                        .trim()
                        .to_string())
                }
            })
    });
    let (ours, theirs) = crate::session_start::read(config_path);
    let at_start = ours
        .iter()
        .chain(theirs.iter())
        .any(|i| i.enabled && i.command.contains("hyprpm reload"));
    State {
        repos,
        loaded: loaded(),
        at_start,
    }
}

/// Active ou coupe un plugin, puis recharge (bloquant).
pub fn set_enabled(name: &str, on: bool) -> Result<(), String> {
    let run = |args: &[&str]| -> Result<(), String> {
        let o = Command::new("hyprpm")
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        if o.status.success() {
            Ok(())
        } else {
            let msg = strip_ansi(&String::from_utf8_lossy(&o.stderr));
            let msg = if msg.trim().is_empty() {
                strip_ansi(&String::from_utf8_lossy(&o.stdout))
            } else {
                msg
            };
            Err(msg.trim().lines().last().unwrap_or_default().to_string())
        }
    };
    run(&[if on { "enable" } else { "disable" }, name])?;
    run(&["reload", "-n"])
}

/// Ajoute `hyprpm reload -n` aux commandes de démarrage tenues par hyprdesk.
pub fn add_at_start(config_path: &std::path::Path) -> Result<(), String> {
    let (ours, _) = crate::session_start::read(config_path);
    let mut list: Vec<(bool, String)> = ours.into_iter().map(|i| (i.enabled, i.command)).collect();
    if !list.iter().any(|(_, c)| c.contains("hyprpm reload")) {
        list.push((true, RELOAD_AT_START.to_string()));
    }
    crate::session_start::save_ours(config_path, &list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liste_lue() {
        let out = "\u{1b}[0m→\u{1b}[0m Repository hyprland-plugins (by hyprwm):\n  │ Plugin borders-plus-plus\n  └─ enabled: \u{1b}[31mfalse\n\u{1b}[0m\n  │ Plugin hyprbars\n  └─ enabled: \u{1b}[32mtrue\n\u{1b}[0m\n";
        let r = parse_list(out);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].name, "hyprland-plugins");
        assert_eq!(r[0].author, "hyprwm");
        assert_eq!(
            r[0].plugins,
            vec![
                Plugin {
                    name: "borders-plus-plus".into(),
                    enabled: false
                },
                Plugin {
                    name: "hyprbars".into(),
                    enabled: true
                },
            ]
        );
    }
}
