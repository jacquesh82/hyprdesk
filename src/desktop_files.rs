//! Bureau type gestionnaire de fichiers (MLG-19) : le contenu du dossier
//! Bureau XDG (`xdg-user-dir DESKTOP`) devient des icônes du bureau, avec
//! une Corbeille. Réglage `desktop_files`, coupé par défaut.
//!
//! Les entrées sont ajoutées à `cfg.icons` au chargement de la config : la
//! grille, la disposition mémorisée et le lancement existants s'en
//! chargent. Un raccourci `.desktop` devient un lanceur (son Exec, son
//! icône, son nom) ; un fichier ou un dossier s'ouvre avec xdg-open. Les
//! fichiers cachés sont écartés, les dossiers viennent d'abord.

use std::path::{Path, PathBuf};

use crate::config::{Config, IconDef, IconKind, Origin, Selector};

/// Préfixe des identifiants (disposition mémorisée).
const ID: &str = "desk:";
/// Au-delà, le bureau deviendrait illisible : les premiers seulement.
const MAX: usize = 120;

/// Dossier Bureau, s'il existe et n'est pas le dossier personnel lui-même
/// (certains systèmes règlent DESKTOP=$HOME : tout le dossier serait affiché).
pub fn desktop_dir() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let dir = std::process::Command::new("xdg-user-dir")
        .arg("DESKTOP")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("Desktop"));
    (dir.is_dir() && dir != home).then_some(dir)
}

/// Icône d'un lanceur `.desktop` du Bureau, s'il en est un.
fn launcher(path: &Path, id: String) -> Option<IconDef> {
    let src = std::fs::read_to_string(path).ok()?;
    let lang = std::env::var("LANG").unwrap_or_default();
    let e = crate::launch::parse_desktop_entry(&src, &lang)?;
    if !e.is_app {
        return None;
    }
    Some(IconDef {
        origin: Origin::User,
        sel: Selector::default(),
        id,
        label: (!e.name.is_empty()).then(|| e.name.clone()),
        kind: IconKind::Exec {
            cmd: crate::launch::strip_field_codes(&e.exec),
            terminal: e.terminal,
        },
        icon: e.icon.clone(),
        pos: None,
    })
}

/// Icônes du dossier : dossiers d'abord, puis par nom (sans casse).
pub fn icons_in(dir: &Path) -> Vec<IconDef> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<(bool, String, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            (!name.starts_with('.')).then(|| (!e.path().is_dir(), name, e.path()))
        })
        .collect();
    entries.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });
    entries
        .into_iter()
        .take(MAX)
        .map(|(_, name, path)| {
            let id = format!("{ID}{name}");
            if name.ends_with(".desktop")
                && let Some(l) = launcher(&path, id.clone())
            {
                return l;
            }
            IconDef {
                origin: Origin::User,
                sel: Selector::default(),
                id,
                label: Some(name),
                kind: IconKind::Path(path),
                icon: None,
                pos: None,
            }
        })
        .collect()
}

/// Dossier Bureau, relevé une fois (xdg-user-dir est un processus).
pub fn desktop_dir_cached() -> Option<PathBuf> {
    static DIR: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    DIR.get_or_init(desktop_dir).clone()
}

fn trash_files() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .map(|d| d.join("Trash/files"))
}

/// Empreinte du Bureau et de la Corbeille (dates de modification des
/// dossiers) : elle change quand un fichier y est ajouté, retiré, renommé.
pub type Stamp = (Option<std::time::SystemTime>, Option<std::time::SystemTime>);

pub fn stamp() -> Stamp {
    let m = |p: Option<PathBuf>| p.and_then(|p| std::fs::metadata(p).ok()?.modified().ok());
    (m(desktop_dir_cached()), m(trash_files()))
}

/// La corbeille contient-elle quelque chose ? (`$XDG_DATA_HOME/Trash/files`)
pub fn trash_full() -> bool {
    trash_files()
        .and_then(|d| std::fs::read_dir(d).ok())
        .is_some_and(|mut rd| rd.next().is_some())
}

/// Icône de la Corbeille (ouverte par le gestionnaire de fichiers).
pub fn trash_icon(full: bool) -> IconDef {
    IconDef {
        origin: Origin::User,
        sel: Selector::default(),
        id: format!("{ID}trash"),
        label: Some(crate::i18n::tr("Corbeille").to_string()),
        kind: IconKind::Exec {
            cmd: "gio open trash:///".into(),
            terminal: false,
        },
        icon: Some(
            if full {
                "user-trash-full"
            } else {
                "user-trash"
            }
            .into(),
        ),
        pos: None,
    }
}

/// Ajoute le Bureau et la Corbeille aux icônes, si le réglage est actif
/// (sans doublon d'identifiant avec une icône déclarée).
pub fn augment(cfg: &mut Config) {
    if !cfg.general.desktop_files {
        return;
    }
    let mut add = desktop_dir_cached()
        .map(|d| icons_in(&d))
        .unwrap_or_default();
    add.push(trash_icon(trash_full()));
    for i in add {
        if !cfg.icons.iter().any(|c| c.id == i.id) {
            cfg.icons.push(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dossier_lu() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-bureau-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Projets")).unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        std::fs::write(dir.join(".cache"), "x").unwrap();
        std::fs::write(
            dir.join("outil.desktop"),
            "[Desktop Entry]\nType=Application\nName=Outil\nExec=outil %U\nIcon=outil-icon\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("lien.desktop"),
            "[Desktop Entry]\nType=Link\nName=L\nURL=x\n",
        )
        .unwrap();
        let icons = icons_in(&dir);
        let ids: Vec<&str> = icons.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "desk:Projets",
                "desk:lien.desktop",
                "desk:notes.txt",
                "desk:outil.desktop"
            ]
        );
        let outil = icons.iter().find(|i| i.id == "desk:outil.desktop").unwrap();
        assert_eq!(
            outil.kind,
            IconKind::Exec {
                cmd: "outil".into(),
                terminal: false
            }
        );
        assert_eq!(outil.label.as_deref(), Some("Outil"));
        assert!(matches!(icons[0].kind, IconKind::Path(_)));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reglage_coupe_rien_ajoute() {
        let mut cfg = Config::default();
        augment(&mut cfg);
        assert!(cfg.icons.is_empty());
    }
}
