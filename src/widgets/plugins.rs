//! Découverte des widgets plugins : des fichiers Lua dans un dossier de
//! widgets. Le premier dossier qui fournit un nom l'emporte (l'utilisateur
//! peut ainsi remplacer un plugin installé pour tout le système).
//!
//! Un plugin est `<dossier>/<nom>.lua` ou `<dossier>/<nom>/init.lua`, et rend
//! une table :
//!
//! ```lua
//! return {
//!     name = "Météo", description = "wttr.in", interval = 900, anchor = "top-left",
//!     options = { ville = "Paris" },                 -- valeurs par défaut
//!     render = function(opts) return hd.exec("curl -s 'wttr.in/" .. opts.ville .. "?format=3'") end,
//! }
//! ```

use std::path::{Path, PathBuf};

/// Dossiers de plugins, du plus prioritaire au moins prioritaire.
pub fn dirs(config_dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![config_dir.join("hyprdesk").join("widgets")];
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    out.push(std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share")).join("hyprdesk/widgets"));
    let sys = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    out.extend(sys.split(':').filter(|s| !s.is_empty()).map(|d| PathBuf::from(d).join("hyprdesk/widgets")));
    out
}

/// Un nom de plugin : minuscules, chiffres, `-` et `_`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Plugins trouvés : (nom, fichier), sans doublon, triés par nom.
pub fn discover(dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut found: Vec<(String, PathBuf)> = rd
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                let name = p.file_name()?.to_str()?.to_string();
                if p.is_dir() {
                    let init = p.join("init.lua");
                    return init.is_file().then_some((name, init));
                }
                let id = name.strip_suffix(".lua")?.to_string();
                Some((id, p))
            })
            .collect();
        found.sort();
        for (id, p) in found {
            if !out.iter().any(|(i, _)| i == &id) {
                out.push((id, p));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decouverte_et_priorite() {
        let base = std::env::temp_dir().join(format!("hyprdesk-plugins-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(a.join("meteo")).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("meteo/init.lua"), "return {}").unwrap();
        std::fs::write(a.join("note.lua"), "return {}").unwrap();
        std::fs::write(a.join("lisezmoi.txt"), "").unwrap();
        std::fs::write(b.join("note.lua"), "return {}").unwrap();
        std::fs::write(b.join("ip.lua"), "return {}").unwrap();
        let found = discover(&[a.clone(), b.clone(), base.join("absent")]);
        let ids: Vec<&str> = found.iter().map(|(i, _)| i.as_str()).collect();
        assert_eq!(ids, vec!["ip", "meteo", "note"]);
        assert_eq!(found[2].1, a.join("note.lua"), "le premier dossier l'emporte");
        assert!(valid_id("ma-meteo_2") && !valid_id("Météo") && !valid_id(""));
        let _ = std::fs::remove_dir_all(&base);
    }
}
