//! Emplacements du moteur (§13 de la spécification) : racines XDG lues dans
//! l'environnement, paramétrables pour les essais (aucune mutation de
//! variables globales, les essais passent des racines explicites).

use std::path::{Path, PathBuf};

/// Racines XDG du moteur : config, données, état, cache, données système.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    /// `$XDG_CONFIG_HOME` ou `~/.config`.
    pub config: PathBuf,
    /// `$XDG_DATA_HOME` ou `~/.local/share`.
    pub data: PathBuf,
    /// `$XDG_STATE_HOME` ou `~/.local/state`.
    pub state: PathBuf,
    /// `$XDG_CACHE_HOME` ou `~/.cache`.
    pub cache: PathBuf,
    /// `$XDG_DATA_DIRS` (défaut : /usr/local/share:/usr/share).
    pub data_system: Vec<PathBuf>,
}

impl Roots {
    /// Racines de la session courante.
    pub fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let dir = |var: &str, fallback: &str| {
            std::env::var(var)
                .ok()
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| Path::new(&home).join(fallback))
        };
        Self {
            config: dir("XDG_CONFIG_HOME", ".config"),
            data: dir("XDG_DATA_HOME", ".local/share"),
            state: dir("XDG_STATE_HOME", ".local/state"),
            cache: dir("XDG_CACHE_HOME", ".cache"),
            data_system: std::env::var("XDG_DATA_DIRS")
                .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string())
                .split(':')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .collect(),
        }
    }

    /// Racines sous un dossier racine d'essai : `root/config`, `root/data`,
    /// `root/state`, `root/cache`, aucun dossier système.
    pub fn at(root: &Path) -> Self {
        Self {
            config: root.join("config"),
            data: root.join("data"),
            state: root.join("state"),
            cache: root.join("cache"),
            data_system: Vec::new(),
        }
    }

    /// Packs de l'utilisateur : `hyprdesk/themes/`.
    pub fn themes_dir(&self) -> PathBuf {
        self.data.join("hyprdesk").join("themes")
    }

    /// Dossiers système de packs : `<data_dir>/hyprdesk/themes/`.
    pub fn system_themes_dirs(&self) -> Vec<PathBuf> {
        self.data_system
            .iter()
            .map(|d| d.join("hyprdesk").join("themes"))
            .collect()
    }

    /// État du moteur : `hyprdesk/theme-engine/` (palette publiée, dernier
    /// rapport, journal).
    pub fn state_dir(&self) -> PathBuf {
        self.state.join("hyprdesk").join("theme-engine")
    }

    /// Cache du moteur : `hyprdesk/theme-engine/` (staging, builds LYRA).
    pub fn cache_dir(&self) -> PathBuf {
        self.cache.join("hyprdesk").join("theme-engine")
    }

    /// Staging de génération.
    pub fn staging_dir(&self) -> PathBuf {
        self.cache_dir().join("staging")
    }

    /// Fragment Hyprland généré : `hypr/theme/hyprland.conf`.
    pub fn hypr_theme_dir(&self) -> PathBuf {
        self.config.join("hypr").join("theme")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn racines_sous_un_dossier_essai() {
        let tmp = std::env::temp_dir().join("hd-roots-test");
        let roots = Roots::at(&tmp);
        assert_eq!(roots.themes_dir(), tmp.join("data/hyprdesk/themes"));
        assert_eq!(roots.state_dir(), tmp.join("state/hyprdesk/theme-engine"));
        assert_eq!(
            roots.hypr_theme_dir(),
            tmp.join("config/hypr/theme/hyprland.conf")
                .parent()
                .unwrap()
        );
        assert!(roots.system_themes_dirs().is_empty());
    }
}
