//! Démarrage automatique XDG (MLG-15) : les entrées de `~/.config/autostart`
//! et de `/etc/xdg/autostart` (XDG_CONFIG_DIRS), lancées une fois par session.
//!
//! Réglage `autostart` (coupé par défaut : beaucoup de configurations lancent
//! déjà leurs applets depuis hyprland.lua). Activé, le démon lance au
//! démarrage les entrées retenues par la spécification — l'entrée de
//! l'utilisateur masque celle du système, `Hidden`, `OnlyShowIn`/`NotShowIn`
//! (d'après XDG_CURRENT_DESKTOP), `X-GNOME-Autostart-enabled`, `TryExec` —
//! sauf celles dont le programme tourne déjà. Un marqueur dans le dossier de
//! l'instance Hyprland évite de relancer à chaque redémarrage du démon.
//! Désactiver une entrée suit la spécification : `Hidden=true` dans la copie
//! de l'utilisateur (`~/.config/autostart/<id>.desktop`).

use std::path::{Path, PathBuf};

/// Pourquoi une entrée est lancée ou non.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    /// Sera lancée.
    Run,
    /// Désactivée (`Hidden=true` ou `X-GNOME-Autostart-enabled=false`).
    Disabled,
    /// Réservée à d'autres bureaux (OnlyShowIn / NotShowIn).
    OtherDesktop,
    /// Programme absent (TryExec ou Exec).
    Missing,
    /// Le programme tourne déjà.
    Running,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Nom du fichier sans `.desktop`.
    pub id: String,
    pub name: String,
    /// Ligne Exec, codes de champ retirés.
    pub exec: String,
    pub terminal: bool,
    pub path: PathBuf,
    /// Fichier du dossier de l'utilisateur.
    pub user: bool,
    pub status: Status,
}

/// Clés propres au démarrage automatique, lues dans `[Desktop Entry]`.
#[derive(Debug, Default, PartialEq)]
struct Keys {
    hidden: bool,
    gnome_enabled: bool,
    only: Vec<String>,
    not: Vec<String>,
}

fn keys(src: &str) -> Keys {
    let mut k = Keys {
        gnome_enabled: true,
        ..Default::default()
    };
    let mut in_section = false;
    let list = |v: &str| -> Vec<String> {
        v.split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    };
    for line in src.lines().map(str::trim) {
        if line.starts_with('[') {
            in_section = line == "[Desktop Entry]";
            continue;
        }
        let Some((key, v)) = line.split_once('=').filter(|_| in_section) else {
            continue;
        };
        match key.trim() {
            "Hidden" => k.hidden = v.trim() == "true",
            "X-GNOME-Autostart-enabled" => k.gnome_enabled = v.trim() != "false",
            "OnlyShowIn" => k.only = list(v),
            "NotShowIn" => k.not = list(v),
            _ => {}
        }
    }
    k
}

/// Bureaux en cours (XDG_CURRENT_DESKTOP, séparés par « : »).
pub fn current_desktops() -> Vec<String> {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// L'entrée vaut-elle pour ces bureaux ?
fn shown_in(only: &[String], not: &[String], desktops: &[String]) -> bool {
    let has = |l: &[String]| {
        l.iter()
            .any(|d| desktops.iter().any(|c| c.eq_ignore_ascii_case(d)))
    };
    (only.is_empty() || has(only)) && !has(not)
}

/// Dossiers d'autostart, du plus prioritaire (utilisateur) au moins prioritaire.
fn dirs() -> Vec<(PathBuf, bool)> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let user = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let mut out = vec![(user.join("autostart"), true)];
    let sys = std::env::var("XDG_CONFIG_DIRS").unwrap_or_else(|_| "/etc/xdg".into());
    out.extend(
        sys.split(':')
            .filter(|s| !s.is_empty())
            .map(|d| (Path::new(d).join("autostart"), false)),
    );
    out
}

/// Classe une entrée (sans regarder les processus en cours).
fn classify(src: &str, desktops: &[String]) -> Option<(crate::launch::DesktopEntry, Status)> {
    let lang = std::env::var("LANG").unwrap_or_default();
    let e = crate::launch::parse_desktop_entry(src, &lang)?;
    let k = keys(src);
    let status = if k.hidden || !k.gnome_enabled {
        Status::Disabled
    } else if !shown_in(&k.only, &k.not, desktops) {
        Status::OtherDesktop
    } else if !e.available() {
        Status::Missing
    } else {
        Status::Run
    };
    Some((e, status))
}

/// Programme en cours d'exécution (même nom de commande) ?
fn running(program: &str) -> bool {
    let base = program.rsplit('/').next().unwrap_or(program);
    // /proc/<pid>/comm est tronqué à 15 caractères.
    let short: String = base.chars().take(15).collect();
    let Ok(rd) = std::fs::read_dir("/proc") else {
        return false;
    };
    rd.flatten()
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.parse::<u32>().is_ok())
        })
        .any(|e| {
            let p = e.path();
            if std::fs::read_to_string(p.join("comm")).is_ok_and(|c| c.trim() == short) {
                return true;
            }
            // Scripts (python, sh…) : le script est le premier argument.
            std::fs::read(p.join("cmdline")).is_ok_and(|c| {
                c.split(|b| *b == 0)
                    .take(2)
                    .any(|a| String::from_utf8_lossy(a).rsplit('/').next() == Some(base))
            })
        })
}

/// Entrées d'autostart, l'utilisateur masquant le système, triées par nom.
pub fn entries() -> Vec<Entry> {
    let desktops = current_desktops();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (dir, user) in dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "desktop"))
            .collect();
        files.sort();
        for path in files {
            let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(String::from) else {
                continue;
            };
            if !seen.insert(id.clone()) {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Some((e, mut status)) = classify(&src, &desktops) else {
                continue;
            };
            if status == Status::Run
                && crate::launch::exec_program(&e.exec).is_some_and(|p| running(&p))
            {
                status = Status::Running;
            }
            out.push(Entry {
                name: if e.name.is_empty() {
                    id.clone()
                } else {
                    e.name.clone()
                },
                id,
                exec: crate::launch::strip_field_codes(&e.exec),
                terminal: e.terminal,
                path,
                user,
                status,
            });
        }
    }
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}

/// `entries`, relevé récent (l'écran Réglages se redessine souvent).
pub fn entries_cached() -> Vec<Entry> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: Mutex<Option<(Instant, Vec<Entry>)>> = Mutex::new(None);
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    match &*c {
        Some((at, v)) if at.elapsed() < Duration::from_secs(3) => v.clone(),
        _ => {
            let v = entries();
            *c = Some((Instant::now(), v.clone()));
            v
        }
    }
}

/// Marqueur « déjà lancé dans cette session Hyprland ».
fn marker() -> PathBuf {
    hyprpanel_core::ipc::instance_dir()
        .or_else(|_| hyprpanel_core::ipc::runtime_dir())
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("hyprdesk-autostart.done")
}

/// Lance les entrées retenues, une fois par session ; noms lancés.
pub fn launch_all(terminal: &str) -> Vec<String> {
    let m = marker();
    if m.exists() {
        return Vec::new();
    }
    let _ = std::fs::write(&m, "");
    let mut launched = Vec::new();
    for e in entries().into_iter().filter(|e| e.status == Status::Run) {
        let cmd = if e.terminal {
            crate::launch::in_terminal(terminal, &e.exec)
        } else {
            e.exec.clone()
        };
        // `cmd &` : le shell rend la main aussitôt, l'application ne reste
        // pas enfant du démon (elle survit à son redémarrage).
        let ok = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("{cmd} >/dev/null 2>&1 &"))
            .stdin(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            launched.push(e.name);
        }
    }
    launched
}

/// Contenu du fichier avec `Hidden` posé (dans `[Desktop Entry]`).
pub fn with_hidden(src: &str, hidden: bool) -> String {
    let mut out = Vec::new();
    let mut in_section = false;
    let mut done = false;
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            if in_section && !done {
                out.push(format!("Hidden={hidden}"));
                done = true;
            }
            in_section = t == "[Desktop Entry]";
        } else if in_section && t.split_once('=').is_some_and(|(k, _)| k.trim() == "Hidden") {
            if !done {
                out.push(format!("Hidden={hidden}"));
                done = true;
            }
            continue;
        }
        out.push(line.to_string());
    }
    if in_section && !done {
        out.push(format!("Hidden={hidden}"));
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// Active ou désactive une entrée : copie de l'utilisateur avec `Hidden`.
pub fn set_enabled(e: &Entry, enabled: bool) -> std::io::Result<()> {
    let src = std::fs::read_to_string(&e.path)?;
    let user_dir = dirs().remove(0).0;
    std::fs::create_dir_all(&user_dir)?;
    std::fs::write(
        user_dir.join(format!("{}.desktop", e.id)),
        with_hidden(&src, !enabled),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(l: &[&str]) -> Vec<String> {
        l.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn bureaux() {
        let here = d(&["Hyprland"]);
        assert!(shown_in(&[], &[], &here));
        assert!(!shown_in(&d(&["KDE", "GNOME"]), &[], &here));
        assert!(shown_in(&d(&["KDE", "Hyprland"]), &[], &here));
        assert!(!shown_in(&[], &d(&["hyprland"]), &here));
        assert!(shown_in(&[], &d(&["GNOME"]), &here));
    }

    #[test]
    fn cles_lues() {
        let k = keys(
            "[Desktop Entry]\nName=A\nHidden=true\nOnlyShowIn=KDE;XFCE;\nX-GNOME-Autostart-enabled=false\n[Desktop Action x]\nHidden=false\n",
        );
        assert!(k.hidden);
        assert!(!k.gnome_enabled);
        assert_eq!(k.only, d(&["KDE", "XFCE"]));
        assert!(keys("[Desktop Entry]\nName=A\n").gnome_enabled);
    }

    #[test]
    fn classement() {
        let here = d(&["Hyprland"]);
        let st = |src: &str| classify(src, &here).map(|(_, s)| s);
        assert_eq!(
            st("[Desktop Entry]\nName=Sh\nExec=sh -c true\n"),
            Some(Status::Run)
        );
        assert_eq!(
            st("[Desktop Entry]\nName=Sh\nExec=sh\nHidden=true\n"),
            Some(Status::Disabled)
        );
        assert_eq!(
            st("[Desktop Entry]\nName=Sh\nExec=sh\nOnlyShowIn=KDE;\n"),
            Some(Status::OtherDesktop)
        );
        assert_eq!(
            st("[Desktop Entry]\nName=X\nExec=programme-absent-hyprdesk\n"),
            Some(Status::Missing)
        );
        assert_eq!(st("[Desktop Entry]\nName=Vide\n"), None);
    }

    /// Relevé de la machine (lecture seule) : `cargo test releve -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn releve() {
        for e in entries() {
            println!("{:?}\t{}\t{}", e.status, e.id, e.exec);
        }
    }

    #[test]
    fn hidden_pose_et_retire() {
        let src = "[Desktop Entry]\nName=A\nExec=a\n[Desktop Action b]\nName=B\n";
        let off = with_hidden(src, true);
        assert_eq!(
            off,
            "[Desktop Entry]\nName=A\nExec=a\nHidden=true\n[Desktop Action b]\nName=B\n"
        );
        let on = with_hidden(&off, false);
        assert!(on.contains("Hidden=false") && !on.contains("Hidden=true"));
        assert_eq!(on.matches("Hidden=").count(), 1);
        assert_eq!(
            with_hidden("[Desktop Entry]\nName=A", true),
            "[Desktop Entry]\nName=A\nHidden=true\n"
        );
    }
}
