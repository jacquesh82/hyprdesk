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

/// Plugins livrés avec hyprdesk : chargés si aucun fichier du même nom ne les remplace.
pub const BUNDLED: &[(&str, &str)] = &[
    (
        "reseau",
        include_str!("../../assets/widgets/integres/reseau.lua"),
    ),
    (
        "bluetooth",
        include_str!("../../assets/widgets/integres/bluetooth.lua"),
    ),
    (
        "batterie",
        include_str!("../../assets/widgets/integres/batterie.lua"),
    ),
    (
        "capture",
        include_str!("../../assets/widgets/integres/capture.lua"),
    ),
    (
        "clavier",
        include_str!("../../assets/widgets/integres/clavier.lua"),
    ),
    (
        "majs",
        include_str!("../../assets/widgets/integres/majs.lua"),
    ),
    (
        "supports",
        include_str!("../../assets/widgets/integres/supports.lua"),
    ),
    (
        "nuit",
        include_str!("../../assets/widgets/integres/nuit.lua"),
    ),
    (
        "notifications",
        include_str!("../../assets/widgets/integres/notifications.lua"),
    ),
    (
        "session",
        include_str!("../../assets/widgets/integres/session.lua"),
    ),
];

/// Source d'un plugin : un fichier, ou un plugin livré.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    File(PathBuf),
    Bundled(&'static str),
}

/// Plugins disponibles : fichiers des dossiers, puis plugins livrés non remplacés.
pub fn all(dirs: &[PathBuf]) -> Vec<(String, Source)> {
    let mut out: Vec<(String, Source)> = discover(dirs)
        .into_iter()
        .map(|(id, p)| (id, Source::File(p)))
        .collect();
    for (id, src) in BUNDLED {
        if !out.iter().any(|(i, _)| i == id) {
            out.push((id.to_string(), Source::Bundled(src)));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Dossiers de plugins, du plus prioritaire au moins prioritaire.
pub fn dirs(config_dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![config_dir.join("hyprdesk").join("widgets")];
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    out.push(
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("hyprdesk/widgets"),
    );
    let sys =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    out.extend(
        sys.split(':')
            .filter(|s| !s.is_empty())
            .map(|d| PathBuf::from(d).join("hyprdesk/widgets")),
    );
    out
}

/// Un nom de plugin : minuscules, chiffres, `-` et `_`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Plugins trouvés : (nom, fichier), sans doublon, triés par nom.
pub fn discover(dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
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

/// Installe un plugin tiers dans `dest` (dossier des plugins de
/// l'utilisateur), depuis un fichier `.lua`, un dossier contenant `init.lua`,
/// ou un dépôt git (`https://…`, `git@…`). Rend le nom du plugin installé.
pub fn install(source: &str, dest: &Path) -> Result<String, String> {
    let source = source.trim();
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let is_git = source.starts_with("https://")
        || source.starts_with("http://")
        || source.starts_with("git@")
        || source.ends_with(".git");
    if is_git {
        let name = source
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit(['/', ':'])
            .next()
            .unwrap_or_default()
            .trim_start_matches("hyprdesk-")
            .to_lowercase();
        if !valid_id(&name) {
            return Err(crate::tr!(
                "nom de plugin « {name} » invalide (minuscules, chiffres, - et _)",
                name = name
            ));
        }
        let target = dest.join(&name);
        if target.exists() {
            return Err(crate::tr!("« {name} » est déjà installé", name = name));
        }
        let st = std::process::Command::new("git")
            .args(["clone", "--depth", "1", "--quiet", source])
            .arg(&target)
            .status()
            .map_err(|e| crate::tr!("git : {e}", e = e))?;
        if !st.success() || !target.join("init.lua").is_file() {
            let _ = std::fs::remove_dir_all(&target);
            return Err(crate::i18n::tr(
                "dépôt sans init.lua à sa racine : ce n'est pas un plugin hyprdesk",
            )
            .into());
        }
        return Ok(name);
    }
    let src = crate::config::lua::expand_path(source, Path::new("/"));
    let (name, is_dir) = if src.is_dir() {
        if !src.join("init.lua").is_file() {
            return Err(crate::tr!("{p0} : pas de init.lua", p0 = src.display()));
        }
        (
            src.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string(),
            true,
        )
    } else if src.extension().is_some_and(|e| e == "lua") && src.is_file() {
        (
            src.file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string(),
            false,
        )
    } else {
        return Err(crate::tr!(
            "« {source} » : ni un fichier .lua, ni un dossier de plugin, ni un dépôt git",
            source = source
        ));
    };
    if !valid_id(&name) {
        return Err(crate::tr!(
            "nom de plugin « {name} » invalide (minuscules, chiffres, - et _)",
            name = name
        ));
    }
    let target = if is_dir {
        dest.join(&name)
    } else {
        dest.join(format!("{name}.lua", name = name))
    };
    if target.exists() {
        return Err(crate::tr!("« {name} » est déjà installé", name = name));
    }
    if is_dir {
        let st = std::process::Command::new("cp")
            .arg("-r")
            .arg(&src)
            .arg(&target)
            .status()
            .map_err(|e| e.to_string())?;
        if !st.success() {
            return Err(crate::tr!("copie de {p0} impossible", p0 = src.display()));
        }
    } else {
        std::fs::copy(&src, &target).map_err(|e| e.to_string())?;
    }
    Ok(name)
}

/// Retire un plugin de l'utilisateur sans le détruire : il part dans
/// `<dossier>/.retired/`, d'où on peut le remettre.
pub fn retire(file: &Path, user_dir: &Path) -> Result<PathBuf, String> {
    // Fichier `nom.lua`, ou dossier `nom/` (dont `init.lua` est le point d'entrée).
    let item = if file.file_name().is_some_and(|n| n == "init.lua") {
        file.parent().unwrap_or(file).to_path_buf()
    } else {
        file.to_path_buf()
    };
    if item.parent() != Some(user_dir) {
        return Err(
            crate::i18n::tr("seuls les plugins installés par l'utilisateur se retirent").into(),
        );
    }
    let bin = user_dir.join(".retired");
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let mut target = bin.join(item.file_name().unwrap_or_default());
    if target.exists() {
        target = bin.join(format!(
            "{}.{}",
            item.file_name().unwrap_or_default().to_string_lossy(),
            crate::storage::revision()
        ));
    }
    std::fs::rename(&item, &target).map_err(|e| e.to_string())?;
    Ok(target)
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
        assert_eq!(
            found[2].1,
            a.join("note.lua"),
            "le premier dossier l'emporte"
        );
        assert!(valid_id("ma-meteo_2") && !valid_id("Météo") && !valid_id(""));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn installer_puis_retirer_un_plugin_tiers() {
        let base = std::env::temp_dir().join(format!("hyprdesk-plug-{}", std::process::id()));
        let (src, dest) = (base.join("src"), base.join("widgets"));
        std::fs::create_dir_all(src.join("vpn")).unwrap();
        std::fs::write(
            src.join("meteo.lua"),
            "return { render = function() return 'x' end }",
        )
        .unwrap();
        std::fs::write(src.join("vpn/init.lua"), "return {}").unwrap();
        std::fs::write(src.join("Mauvais Nom.lua"), "return {}").unwrap();
        let s = |p: PathBuf| p.display().to_string();
        assert_eq!(install(&s(src.join("meteo.lua")), &dest).unwrap(), "meteo");
        assert_eq!(install(&s(src.join("vpn")), &dest).unwrap(), "vpn");
        assert!(
            install(&s(src.join("meteo.lua")), &dest).is_err(),
            "déjà installé"
        );
        assert!(install(&s(src.join("Mauvais Nom.lua")), &dest).is_err());
        assert!(install("/nulle/part.txt", &dest).is_err());
        let found: Vec<String> = discover(std::slice::from_ref(&dest))
            .into_iter()
            .map(|p| p.0)
            .collect();
        assert_eq!(found, ["meteo", "vpn"]);
        // Retirer : déplacé dans .retired/, plus découvert.
        retire(&dest.join("vpn/init.lua"), &dest).unwrap();
        retire(&dest.join("meteo.lua"), &dest).unwrap();
        assert!(discover(std::slice::from_ref(&dest)).is_empty());
        assert!(dest.join(".retired/vpn/init.lua").is_file());
        assert!(
            retire(&src.join("meteo.lua"), &dest).is_err(),
            "hors du dossier de l'utilisateur"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
