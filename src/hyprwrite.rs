//! Écriture vérifiée des fichiers de configuration de Hyprland (MLG-137) :
//! plusieurs fichiers d'un coup, `hyprctl reload`, puis comparaison des
//! erreurs de Hyprland avant et après ; une erreur nouvelle remet tous les
//! fichiers comme avant. Généralise `binds_file::write_checked`.
//!
//! Hors session Hyprland (tests, greeter), les fichiers sont seulement
//! écrits.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Un fichier à écrire (`None` : à supprimer).
pub struct Change {
    pub path: PathBuf,
    pub content: Option<String>,
}

/// Session Hyprland à vérifier ; jamais pendant les tests (ils ne doivent
/// pas recharger le Hyprland de la session qui les lance).
fn in_session() -> bool {
    !cfg!(test) && std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

fn put(path: &Path, content: Option<&str>) -> std::io::Result<()> {
    match content {
        Some(c) => crate::storage::write(path, c.as_bytes(), crate::storage::revision()),
        None => match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    }
}

fn reload() {
    let _ = std::process::Command::new("hyprctl").arg("reload").output();
    std::thread::sleep(std::time::Duration::from_millis(300));
}

/// Écrit `changes` (rien si tout est déjà identique) ; recharge Hyprland et
/// revient en arrière si une erreur de config nouvelle apparaît. Renvoie le
/// nombre de fichiers changés.
pub fn write_checked(changes: &[Change]) -> Result<usize, String> {
    let olds: Vec<Option<String>> = changes
        .iter()
        .map(|c| std::fs::read_to_string(&c.path).ok())
        .collect();
    let todo: Vec<usize> = (0..changes.len())
        .filter(|&i| olds[i].as_deref() != changes[i].content.as_deref())
        .collect();
    if todo.is_empty() {
        return Ok(0);
    }
    let session = in_session();
    let before: BTreeSet<String> = if session {
        crate::hyprcheck::config_errors().into_iter().collect()
    } else {
        BTreeSet::new()
    };
    let mut written: Vec<usize> = Vec::new();
    for &i in &todo {
        if let Err(e) = put(&changes[i].path, changes[i].content.as_deref()) {
            for &j in &written {
                let _ = put(&changes[j].path, olds[j].as_deref());
            }
            return Err(crate::tr!(
                "écriture de {p0} : {e}",
                p0 = changes[i].path.display(),
                e = e
            ));
        }
        written.push(i);
    }
    if !session {
        return Ok(todo.len());
    }
    reload();
    let fresh: Vec<String> = crate::hyprcheck::config_errors()
        .into_iter()
        .filter(|e| !before.contains(e))
        .collect();
    if !fresh.is_empty() {
        for &i in &todo {
            let _ = put(&changes[i].path, olds[i].as_deref());
        }
        reload();
        return Err(crate::tr!(
            "configuration laissée telle quelle : Hyprland signalait {e}",
            e = fresh.join(" ; ")
        ));
    }
    Ok(todo.len())
}

/// Source de hyprland.lua avec `require("<module>")` dans le bloc des
/// fichiers générés de hyprdesk, en fin de fichier (ils gagnent sur ce qui
/// précède) ; `None` s'il y est déjà.
pub fn with_require(src: &str, module: &str) -> Option<String> {
    let line = format!("require(\"{module}\")");
    if src.lines().any(|l| l.trim_start().starts_with(&line)) {
        return None;
    }
    let mut out = src.trim_end().to_string();
    if !src.contains(BLOCK) {
        out.push_str("\n\n");
        out.push_str(BLOCK);
    }
    out.push('\n');
    out.push_str(&line);
    out.push('\n');
    Some(out)
}

/// Titre du bloc des `require` de hyprdesk.
pub const BLOCK: &str = "-- hyprdesk : fichiers générés (Réglages), chargés en dernier";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_en_fin_une_seule_fois() {
        let src = "require(\"monitors\")\nhl.config({})\n";
        let a = with_require(src, "appearance").unwrap();
        assert!(a.ends_with("require(\"appearance\")\n"));
        assert!(a.contains(BLOCK));
        let b = with_require(&a, "system").unwrap();
        assert_eq!(b.matches(BLOCK).count(), 1);
        assert!(b.ends_with("require(\"appearance\")\nrequire(\"system\")\n"));
        assert!(with_require(&b, "system").is_none());
    }

    #[test]
    fn ecriture_hors_session() {
        let d = std::env::temp_dir().join(format!("hyprdesk-hyprwrite-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let a = d.join("a.lua");
        let b = d.join("b.lua");
        std::fs::write(&b, "vieux").unwrap();
        let n = write_checked(&[
            Change {
                path: a.clone(),
                content: Some("neuf".into()),
            },
            Change {
                path: b.clone(),
                content: None,
            },
        ])
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "neuf");
        assert!(!b.exists());
        assert_eq!(
            write_checked(&[Change {
                path: a,
                content: Some("neuf".into())
            }])
            .unwrap(),
            0
        );
    }
}
