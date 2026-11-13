//! Configuration Lua : modèle, chargement, résolution par bureau, surveillance.

pub mod lua;
pub mod model;
pub mod resolve;

use std::path::{Path, PathBuf};

pub use model::*;

/// `$HYPRDESK_CONFIG`, sinon `$XDG_CONFIG_HOME/hypr/hyprdesk.lua` (à côté de `hyprland.lua`).
pub fn default_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HYPRDESK_CONFIG") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    base.join("hypr").join("hyprdesk.lua")
}

/// Config d'exemple, écrite au premier lancement si aucune n'existe
/// (commentaires dans la langue de l'interface).
pub const EXAMPLE: &str = include_str!("../../assets/hyprdesk.lua");
pub const EXAMPLE_EN: &str = include_str!("../../assets/hyprdesk.en.lua");

/// Remet la configuration par défaut : `hyprdesk.lua` (la configuration
/// d'exemple) et `hyprdesk/settings.lua` (vidé : plus aucun réglage fait à la
/// souris), après une copie des deux dans `hyprdesk/sauvegardes/<date>/`.
/// Les fichiers de Hyprland (inputs.lua, monitors.lua, autostart.lua), les
/// thèmes et les widgets ne bougent pas. Rend le dossier de la copie.
pub fn reset_to_defaults(path: &Path) -> std::io::Result<PathBuf> {
    let settings = lua::settings_path(path);
    let dir = path.parent().unwrap_or(Path::new("."));
    let backup = dir
        .join("hyprdesk")
        .join("sauvegardes")
        .join(chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
    std::fs::create_dir_all(&backup)?;
    for f in [path, settings.as_path()] {
        if f.is_file() {
            std::fs::copy(f, backup.join(f.file_name().unwrap_or_default()))?;
        }
    }
    std::fs::write(path, example())?;
    if settings.is_file() {
        std::fs::remove_file(&settings)?;
    }
    Ok(backup)
}

pub fn example() -> &'static str {
    match crate::i18n::current() {
        crate::i18n::Lang::Fr => EXAMPLE,
        crate::i18n::Lang::En => EXAMPLE_EN,
    }
}

pub fn ensure_exists(path: &Path) -> std::io::Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, example())?;
    Ok(true)
}

#[cfg(test)]
mod reset_tests {
    use super::*;

    #[test]
    fn remise_par_defaut_avec_copie() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-reset-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("hyprdesk")).unwrap();
        let cfg = dir.join("hyprdesk.lua");
        std::fs::write(&cfg, "-- ma config").unwrap();
        std::fs::write(
            dir.join("hyprdesk/settings.lua"),
            "hd.config({ show_on = \"focused\" })",
        )
        .unwrap();
        std::fs::write(dir.join("monitors.lua"), "-- écrans").unwrap();
        let backup = reset_to_defaults(&cfg).unwrap();
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), example());
        assert!(!dir.join("hyprdesk/settings.lua").exists());
        assert_eq!(
            std::fs::read_to_string(backup.join("hyprdesk.lua")).unwrap(),
            "-- ma config"
        );
        assert!(backup.join("settings.lua").is_file());
        assert_eq!(
            std::fs::read_to_string(dir.join("monitors.lua")).unwrap(),
            "-- écrans"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
