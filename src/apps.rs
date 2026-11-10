//! Applications par défaut : navigateur, terminal, fichiers, éditeur, images,
//! vidéo, audio, PDF, mail, assistant IA.
//!
//! Une valeur est l'identifiant d'une application installée (`brave-browser`,
//! entrée `.desktop`), ou une commande (`claude`, `foot`). Choisie depuis les
//! réglages, elle devient aussi la valeur du système : `xdg-mime default`
//! pour les types de fichiers du rôle (ce qu'utilise `xdg-open`), et
//! `xdg-terminals.list` pour le terminal. `hyprdesk open <rôle>` la lance.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::{IconDef, IconKind, Selector};
use crate::launch::{self, AppInfo};

/// Un rôle d'application par défaut.
pub struct Role {
    pub id: &'static str,
    pub label: &'static str,
    /// Types de fichiers du rôle ; le premier sert à lire la valeur du système.
    pub mimes: &'static [&'static str],
    /// Catégorie freedesktop qui désigne les candidats (terminal).
    pub category: Option<&'static str>,
    /// Une commande sans entrée `.desktop` s'ouvre dans le terminal.
    pub in_terminal: bool,
    /// Candidats connus en dehors des types de fichiers : (valeur, libellé).
    pub known: &'static [(&'static str, &'static str)],
}

pub const ROLES: &[Role] = &[
    Role {
        id: "browser",
        label: "Navigateur",
        mimes: &[
            "x-scheme-handler/https",
            "x-scheme-handler/http",
            "text/html",
        ],
        category: None,
        in_terminal: false,
        known: &[],
    },
    Role {
        id: "terminal",
        label: "Terminal",
        mimes: &[],
        category: Some("TerminalEmulator"),
        in_terminal: false,
        known: &[],
    },
    Role {
        id: "files",
        label: "Fichiers",
        mimes: &["inode/directory"],
        category: None,
        in_terminal: false,
        known: &[],
    },
    Role {
        id: "editor",
        label: "Éditeur de texte",
        mimes: &["text/plain"],
        category: None,
        in_terminal: true,
        known: &[],
    },
    Role {
        id: "image",
        label: "Images",
        mimes: &[
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/webp",
            "image/bmp",
            "image/tiff",
            "image/svg+xml",
        ],
        category: None,
        in_terminal: false,
        known: &[],
    },
    Role {
        id: "video",
        label: "Vidéo",
        mimes: &[
            "video/mp4",
            "video/webm",
            "video/x-matroska",
            "video/quicktime",
            "video/x-msvideo",
            "video/mpeg",
        ],
        category: None,
        in_terminal: false,
        known: &[],
    },
    Role {
        id: "audio",
        label: "Audio",
        mimes: &[
            "audio/mpeg",
            "audio/flac",
            "audio/ogg",
            "audio/x-wav",
            "audio/mp4",
            "audio/opus",
        ],
        category: None,
        in_terminal: false,
        known: &[],
    },
    Role {
        id: "pdf",
        label: "PDF",
        mimes: &["application/pdf"],
        category: None,
        in_terminal: false,
        known: &[],
    },
    Role {
        id: "mail",
        label: "Mail",
        mimes: &["x-scheme-handler/mailto"],
        category: None,
        in_terminal: false,
        known: &[],
    },
    Role {
        id: "ai",
        label: "Assistant IA",
        mimes: &[],
        category: None,
        in_terminal: true,
        known: &[
            ("claude", "Claude Code"),
            ("codex", "Codex"),
            ("zcode", "ZCode"),
            ("gemini", "Gemini CLI"),
            ("opencode", "opencode"),
            ("aider", "Aider"),
        ],
    },
];

pub fn role(id: &str) -> Option<&'static Role> {
    ROLES.iter().find(|r| r.id == id)
}

/// Commande disponible dans le PATH.
fn in_path(cmd: &str) -> bool {
    let first = cmd.split_whitespace().next().unwrap_or(cmd);
    std::env::var_os("PATH").is_some_and(|p| {
        std::env::split_paths(&p).any(|d| {
            let f = d.join(first);
            f.is_file()
        })
    })
}

/// Candidats d'un rôle : (valeur, libellé), sans doublon.
pub fn candidates(r: &Role, apps: &[AppInfo]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (value, label) in r.known {
        let desktop = apps.iter().find(|a| a.id.eq_ignore_ascii_case(value));
        if desktop.is_some() || in_path(value) {
            out.push((
                desktop.map_or(value.to_string(), |a| a.id.clone()),
                label.to_string(),
            ));
        }
    }
    for a in apps {
        let fits = r
            .mimes
            .first()
            .is_some_and(|m| a.mime_types.iter().any(|t| t == m))
            || r.category
                .is_some_and(|c| a.categories.iter().any(|x| x == c));
        if fits && !out.iter().any(|(v, _)| *v == a.id) {
            out.push((a.id.clone(), a.name.clone()));
        }
    }
    out
}

fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        })
}

/// Valeur du système pour un rôle : application de `xdg-mime`, ou premier
/// terminal de `xdg-terminals.list`.
pub fn system_default(r: &Role) -> Option<String> {
    if r.category == Some("TerminalEmulator") {
        let list = std::fs::read_to_string(config_home().join("xdg-terminals.list")).ok()?;
        return list
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.trim_end_matches(".desktop").to_string());
    }
    let mime = r.mimes.first()?;
    let out = std::process::Command::new("xdg-mime")
        .args(["query", "default", mime])
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!id.is_empty()).then(|| id.trim_end_matches(".desktop").to_string())
}

/// Rend `value` application du système pour ce rôle (s'il s'agit d'une
/// application installée ; une simple commande ne concerne que hyprdesk).
pub fn apply_system(r: &Role, value: &str) -> Result<bool, String> {
    if launch::find_desktop_file(value).is_none() {
        return Ok(false);
    }
    let desktop = format!("{value}.desktop");
    if r.category == Some("TerminalEmulator") {
        let p = config_home().join("xdg-terminals.list");
        let old = std::fs::read_to_string(&p).unwrap_or_default();
        let mut lines = vec![desktop.clone()];
        lines.extend(
            old.lines()
                .filter(|l| l.trim() != desktop && !l.trim().is_empty())
                .map(String::from),
        );
        std::fs::write(&p, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
        return Ok(true);
    }
    if r.mimes.is_empty() {
        return Ok(false);
    }
    let st = std::process::Command::new("xdg-mime")
        .arg("default")
        .arg(&desktop)
        .args(r.mimes)
        .status()
        .map_err(|e| format!("xdg-mime : {e}"))?;
    if !st.success() {
        return Err(format!("xdg-mime default {desktop} : échec"));
    }
    Ok(true)
}

/// Commande de terminal d'après une valeur du rôle terminal : application
/// installée (son `Exec`), sinon la valeur telle quelle (`foot {cmd}` permis).
pub fn terminal_command(value: &str) -> String {
    launch::find_desktop_file(value)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| launch::parse_desktop_entry(&s, ""))
        .and_then(|e| e.exec.split_whitespace().next().map(String::from))
        .unwrap_or_else(|| value.to_string())
}

/// Commande qui lance l'application d'un rôle, d'après la config ; à défaut
/// de réglage, celle du système.
pub fn command(r: &Role, apps: &BTreeMap<String, String>, terminal: &str) -> Option<String> {
    if r.id == "terminal" {
        let t = apps
            .get("terminal")
            .map_or(terminal.to_string(), |v| terminal_command(v));
        return Some(t.replace(" -e {cmd}", "").replace(" {cmd}", ""));
    }
    let value = apps.get(r.id).cloned().or_else(|| system_default(r))?;
    let kind = if launch::find_desktop_file(&value).is_some() {
        IconKind::App(value)
    } else {
        IconKind::Exec {
            cmd: value,
            terminal: r.in_terminal,
        }
    };
    let def = IconDef {
        origin: Default::default(),
        sel: Selector::default(),
        id: r.id.into(),
        label: None,
        kind,
        icon: None,
        pos: None,
    };
    let c = launch::resolve(&def, None, 16, terminal).command;
    (!c.is_empty()).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, mimes: &[&str], cats: &[&str]) -> AppInfo {
        AppInfo {
            id: id.into(),
            name: id.into(),
            icon: None,
            wm_class: None,
            mime_types: mimes.iter().map(|s| s.to_string()).collect(),
            categories: cats.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn candidats_par_role() {
        let apps = vec![
            app(
                "brave-browser",
                &["x-scheme-handler/https", "text/html"],
                &["WebBrowser"],
            ),
            app("mpv", &["video/mp4", "audio/mpeg"], &[]),
            app("kitty", &[], &["System", "TerminalEmulator"]),
            app("zcode", &[], &["Development"]),
        ];
        let ids = |r: &str| -> Vec<String> {
            candidates(role(r).unwrap(), &apps)
                .into_iter()
                .map(|c| c.0)
                .collect()
        };
        assert_eq!(ids("browser"), ["brave-browser"]);
        assert_eq!(ids("video"), ["mpv"]);
        assert_eq!(ids("audio"), ["mpv"]);
        assert_eq!(ids("terminal"), ["kitty"]);
        assert!(
            ids("ai").contains(&"zcode".to_string()),
            "application IA installée"
        );
        let mut seen: Vec<&str> = ROLES.iter().map(|r| r.id).collect();
        seen.dedup();
        assert_eq!(seen.len(), ROLES.len());
    }

    #[test]
    fn commande_du_terminal_et_d_une_commande_ia() {
        let mut apps = BTreeMap::new();
        assert_eq!(
            command(role("terminal").unwrap(), &apps, "foot {cmd}").as_deref(),
            Some("foot")
        );
        apps.insert("ai".to_string(), "claude-hyprdesk-test".to_string());
        // Pas d'entrée .desktop : la commande s'ouvre dans le terminal.
        assert_eq!(
            command(role("ai").unwrap(), &apps, "kitty").as_deref(),
            Some("kitty -e claude-hyprdesk-test")
        );
    }
}
