//! État transmis par le démon à l'écran de verrouillage : valeurs déjà
//! résolues (fond, couleurs, police, options), dans un JSON de la session.
//!
//! Le verrou n'exécute ainsi aucun Lua : un plugin ne peut ni le bloquer ni y
//! injecter du code. Sans ce fichier (démon arrêté), des valeurs par défaut
//! sûres s'appliquent.

use std::path::{Path, PathBuf};

use crate::config::{Config, Rgba, WallSource};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LockState {
    /// Image de fond (déjà choisie : celle du verrouillage, sinon celle du bureau).
    pub image: Option<PathBuf>,
    /// Couleur de fond, sans image ou sous l'image.
    pub background: [u8; 4],
    pub fg: [u8; 4],
    pub accent: [u8; 4],
    pub font: Option<String>,
    pub blur: bool,
    pub dim: f32,
    pub show_user: bool,
    pub show_battery: bool,
    pub show_media: bool,
}

impl Default for LockState {
    fn default() -> Self {
        let g = crate::config::General::default();
        let rgba = |c: Rgba| [c.0, c.1, c.2, c.3];
        LockState {
            image: None,
            background: rgba(g.background),
            fg: rgba(g.theme.fg),
            accent: rgba(g.theme.accent),
            font: None,
            blur: g.lock.blur,
            dim: g.lock.dim,
            show_user: g.lock.show_user,
            show_battery: g.lock.show_battery,
            show_media: g.lock.show_media,
        }
    }
}

/// `$XDG_RUNTIME_DIR/hyprdesk/lock.json` (propre à l'utilisateur et à la session).
pub fn path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("hyprdesk-{}", unsafe { libc::getuid() }))
        });
    base.join("hyprdesk").join("lock.json")
}

/// État du verrouillage d'après la config, pour l'écran principal et le
/// workspace qu'il affiche.
pub fn from_config(cfg: &Config, monitor: &str, workspace: &str) -> LockState {
    let g = &cfg.general;
    let l = &g.lock;
    let rgba = |c: Rgba| [c.0, c.1, c.2, c.3];
    let mut background = rgba(g.background);
    let desktop = crate::config::resolve::resolve(cfg, monitor, workspace);
    let image = l.image.clone().filter(|p| p.is_file()).or_else(|| {
        match desktop.wallpaper.map(|w| &w.source) {
            Some(WallSource::Path(p)) => Some(p.clone()),
            Some(WallSource::Rotate { dir, .. }) => first_image(dir),
            Some(WallSource::Color(c)) => {
                background = rgba(*c);
                None
            }
            None => None,
        }
    });
    LockState {
        image,
        background,
        fg: rgba(g.theme.fg),
        accent: rgba(g.theme.accent),
        font: g.font.clone(),
        blur: l.blur,
        dim: l.dim,
        show_user: l.show_user,
        show_battery: l.show_battery,
        show_media: l.show_media,
    }
}

fn first_image(dir: &Path) -> Option<PathBuf> {
    let mut l: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                matches!(e.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp")
            })
        })
        .collect();
    l.sort();
    l.into_iter().next()
}

/// Écrit l'état (atomique, lisible par l'utilisateur seul).
pub fn write(s: &LockState) -> std::io::Result<()> {
    let p = path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    let json = serde_json::to_vec_pretty(s).map_err(std::io::Error::other)?;
    if std::fs::read(&p).ok().as_deref() == Some(json.as_slice()) {
        return Ok(());
    }
    crate::storage::write(&p, &json, crate::storage::revision())
}

/// Lit l'état ; valeurs par défaut s'il manque ou s'il est illisible.
pub fn read() -> LockState {
    std::fs::read(path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etat_du_verrou() {
        let mut cfg = Config::default();
        cfg.general.lock.dim = 0.6;
        cfg.wallpapers.push(crate::config::WallpaperRule {
            origin: Default::default(),
            sel: Default::default(),
            source: WallSource::Color(Rgba(1, 2, 3, 255)),
            fit: Default::default(),
        });
        let s = from_config(&cfg, "DP-1", "1");
        assert_eq!(s.image, None);
        assert_eq!(s.background, [1, 2, 3, 255]);
        assert_eq!(s.dim, 0.6);
        // Aller-retour JSON, champs manquants complétés.
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<LockState>(&json).unwrap(), s);
        let partial: LockState = serde_json::from_str(r#"{ "blur": false }"#).unwrap();
        assert!(!partial.blur && partial.show_user);
    }
}
