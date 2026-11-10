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
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
    base.join("hypr").join("hyprdesk.lua")
}

/// Config d'exemple, écrite au premier lancement si aucune n'existe.
pub const EXAMPLE: &str = include_str!("../../assets/hyprdesk.lua");

pub fn ensure_exists(path: &Path) -> std::io::Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, EXAMPLE)?;
    Ok(true)
}
