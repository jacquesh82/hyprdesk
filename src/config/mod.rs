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

/// Bureau de départ : icônes, widgets et barre d'une première installation.
/// Écrit dans `settings.lua` (et non plus dans `hyprdesk.lua`) : tout s'y
/// règle dans les Réglages, rien n'est « fixé dans hyprdesk.lua ».
fn starter_lua() -> String {
    let home = hyprpanel_core::ipc::lua_escape(crate::i18n::tr("Dossier perso"));
    format!(
        r#"hd.config({{ workspaces = 5 }})
hd.icon({{ app = "kitty" }})
hd.icon({{ label = "{home}", path = "~", icon = "user-home" }})
hd.icon({{ label = "htop", exec = "htop", terminal = true, icon = "utilities-system-monitor" }})
hd.widget("clock", {{ id = "clock", anchor = "top-right", size = 72, card = false }})
hd.widget("calendar", {{ id = "calendar", anchor = "right" }})
hd.widget("system", {{ id = "system", anchor = "bottom-right" }})
hd.widget("media", {{ id = "media", anchor = "bottom-left", x = 48 }})
hd.bar({{
    position = "top", height = 36,
    left   = {{ "workspaces", "window" }},
    center = {{ {{ "clock", format = "%a %d %b  %H:%M" }} }},
    right  = {{ "tray", "media", "network", "bluetooth", "battery", "volume", "notifications", "desktop", "session", "controls" }},
}})
"#
    )
}

/// Écrit le bureau de départ dans `settings.lua`, sous la forme qu'écrivent
/// les Réglages.
pub fn write_starter_settings(path: &Path) -> std::io::Result<PathBuf> {
    let saved = starter_settings(path)?.save(path)?;
    forget_key_answers(path);
    Ok(saved)
}

/// Réglages du bureau de départ : la configuration par défaut de chaque
/// composant (`crate::defaults`).
pub fn starter_settings(path: &Path) -> std::io::Result<crate::settings::Settings> {
    let sp = lua::settings_path(path);
    let base = path.parent().unwrap_or(Path::new("."));
    let cfg = lua::load_chunks(
        &[(starter_lua(), sp.display().to_string(), Origin::Settings)],
        base,
    )
    .map_err(std::io::Error::other)?
    .config;
    Ok(crate::settings::Settings::from_config(&cfg))
}

/// Réponses aux conflits de raccourcis (une ligne par conflit répondu :
/// clé, combinaison, commande qui la tient) : ce qui est répondu n'est plus
/// demandé ; effacé à l'installation et à la remise à zéro (MLG-209).
pub fn keys_answers_path(path: &Path) -> PathBuf {
    lua::settings_path(path).with_file_name(".raccourcis-repondus")
}

/// Installation ou remise à zéro des raccourcis : les réponses aux conflits
/// s'effacent, la question se pose de nouveau.
pub fn forget_key_answers(path: &Path) {
    let _ = std::fs::remove_file(keys_answers_path(path));
    let _ = std::fs::remove_file(lua::settings_path(path).with_file_name(".raccourcis-a-revoir"));
}

pub fn example() -> &'static str {
    match crate::i18n::current() {
        crate::i18n::Lang::Fr => EXAMPLE,
        crate::i18n::Lang::En => EXAMPLE_EN,
    }
}

/// Première installation : l'exemple (`hyprdesk.lua`, tout en commentaires)
/// et le bureau de départ (`settings.lua`). Rien n'est recréé si l'un des
/// deux existe : `hyprdesk.lua` supprimé ne revient pas tant que les Réglages
/// sont là. `Ok(true)` : configuration créée.
pub fn ensure_exists(path: &Path) -> std::io::Result<bool> {
    if path.exists() || lua::settings_path(path).exists() {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, example())?;
    write_starter_settings(path)?;
    Ok(true)
}

#[cfg(test)]
mod reset_tests {
    use super::*;

    #[test]
    fn premiere_installation_et_hyprdesk_lua_facultatif() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-first-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = dir.join("hyprdesk.lua");
        // Première installation : l'exemple, sans ligne active, et le bureau de
        // départ dans les Réglages.
        assert!(ensure_exists(&cfg).unwrap());
        for ex in [EXAMPLE, EXAMPLE_EN] {
            assert!(
                ex.lines()
                    .all(|l| l.trim().is_empty() || l.trim_start().starts_with("--")),
                "exemple sans ligne active"
            );
        }
        let c = lua::load_file(&cfg).unwrap().config;
        assert!(c.keys_user.is_empty(), "rien de fixé dans hyprdesk.lua");
        let types: Vec<&str> = c
            .widgets
            .iter()
            .filter(|w| !w.in_bar)
            .map(|w| w.type_id.as_str())
            .collect();
        for t in ["clock", "calendar", "system", "media"] {
            assert!(types.contains(&t), "{t}");
        }
        assert!(
            c.widgets
                .iter()
                .filter(|w| !w.in_bar)
                .all(|w| w.origin == Origin::Settings)
        );
        assert_eq!(c.icons.len(), 3);
        assert!(c.bar.is_some());
        // hyprdesk.lua supprimé : pas recréé, et la configuration se charge.
        std::fs::remove_file(&cfg).unwrap();
        assert!(!ensure_exists(&cfg).unwrap());
        assert!(!cfg.exists());
        let again = lua::load_file(&cfg).unwrap().config;
        assert_eq!(again.icons.len(), 3);
        // Ni l'un ni l'autre : pas de configuration.
        std::fs::remove_file(lua::settings_path(&cfg)).unwrap();
        assert!(lua::load_file(&cfg).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
