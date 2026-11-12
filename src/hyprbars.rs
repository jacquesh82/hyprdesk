//! Hyprbars (plugin de Hyprland, dépôt hyprwm/hyprland-plugins) : barres de
//! titre des fenêtres, seulement en plein bureau.
//!
//! Option `hyprbars` (coupée par défaut) : en entrant en full-desktop,
//! hyprdesk charge le plugin (`hyprctl plugin load`) ; en sortant, il le
//! décharge. Le plugin est cherché là où `hyprpm` le construit et dans les
//! dossiers de plugins des distributions, ou au chemin `hyprbars_path`.

use std::path::{Path, PathBuf};

/// Emplacements connus de `hyprbars.so`.
fn candidates() -> Vec<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let mut v = Vec::new();
    // hyprpm : /var/cache/hyprpm/<utilisateur>/hyprland-plugins/hyprbars.so
    if let Ok(rd) = std::fs::read_dir("/var/cache/hyprpm") {
        for e in rd.flatten() {
            v.push(e.path().join("hyprland-plugins").join("hyprbars.so"));
        }
    }
    for d in [
        "/usr/lib/hyprland/plugins",
        "/usr/lib/hyprland-plugins",
        "/usr/lib64/hyprland/plugins",
        "/usr/local/lib/hyprland/plugins",
    ] {
        v.push(Path::new(d).join("hyprbars.so"));
        v.push(Path::new(d).join("libhyprbars.so"));
    }
    v.push(home.join(".local/share/hyprland/plugins/hyprbars.so"));
    v.push(home.join(".local/lib/hyprbars.so"));
    v
}

/// Chemin du plugin : celui réglé s'il existe, sinon le premier trouvé.
pub fn find(custom: &str) -> Option<PathBuf> {
    let custom = custom.trim();
    if !custom.is_empty() {
        let p = PathBuf::from(crate::apptheme::expand_home(custom));
        return p.is_file().then_some(p);
    }
    candidates().into_iter().find(|p| p.is_file())
}

fn hyprctl(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Le plugin est chargé dans Hyprland.
pub fn loaded() -> bool {
    hyprctl(&["plugin", "list"]).is_ok_and(|s| s.to_lowercase().contains("hyprbars"))
}

/// Charge le plugin (rien s'il l'est déjà).
pub fn load(path: &Path) -> Result<(), String> {
    if loaded() {
        return Ok(());
    }
    let r = hyprctl(&["plugin", "load", &path.display().to_string()])?;
    if r == "ok" || loaded() {
        Ok(())
    } else {
        Err(crate::tr!("Hyprbars : {r}", r = r))
    }
}

/// Décharge le plugin (rien s'il ne l'est pas).
pub fn unload(path: &Path) -> Result<(), String> {
    if !loaded() {
        return Ok(());
    }
    let r = hyprctl(&["plugin", "unload", &path.display().to_string()])?;
    if r == "ok" || !loaded() {
        Ok(())
    } else {
        Err(crate::tr!("Hyprbars : {r}", r = r))
    }
}
