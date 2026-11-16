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
        "lecteur",
        include_str!("../../assets/widgets/integres/lecteur.lua"),
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
        "vkbd",
        include_str!("../../assets/widgets/integres/vkbd.lua"),
    ),
    (
        "mymfa",
        include_str!("../../assets/widgets/integres/mymfa.lua"),
    ),
    (
        "notifications",
        include_str!("../../assets/widgets/integres/notifications.lua"),
    ),
    (
        "session",
        include_str!("../../assets/widgets/integres/session.lua"),
    ),
    (
        "index",
        include_str!("../../assets/widgets/integres/index.lua"),
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

/// Plugin tiers mis en quarantaine, pas encore installé : rien n'en a été
/// chargé ni exécuté. L'utilisateur voit sa provenance (commit pour un dépôt
/// git), l'empreinte de son point d'entrée et ses premières lignes, puis
/// confirme ([`confirm`]) ou renonce ([`discard`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Pending {
    pub name: String,
    pub source: String,
    /// Dossier de quarantaine (hors des dossiers de plugins).
    pub quarantine: PathBuf,
    /// Le plugin dans la quarantaine : `nom.lua` ou dossier `nom/`.
    pub item: PathBuf,
    pub is_dir: bool,
    /// Commit installé (dépôt git).
    pub commit: Option<String>,
    /// SHA-256 du point d'entrée (`init.lua` ou le fichier).
    pub digest: String,
    /// Fichiers `.lua` du plugin.
    pub lua_files: usize,
    /// Début du point d'entrée, pour relecture.
    pub preview: String,
}

/// Sources git acceptées : chiffrées et authentifiées (https, ssh). `http://`,
/// `git://` et les transports locaux ou externes sont refusés.
fn git_source(source: &str) -> Option<Result<(), String>> {
    let looks_git =
        source.contains("://") || source.starts_with("git@") || source.ends_with(".git");
    if !looks_git {
        return None;
    }
    let ok = source.starts_with("https://")
        || source.starts_with("ssh://")
        || (source.starts_with("git@") && source.contains(':'));
    Some(if ok {
        Ok(())
    } else {
        Err(crate::tr!(
            "« {source} » : seuls les dépôts https:// ou ssh (git@…) sont acceptés",
            source = source
        ))
    })
}

fn quarantine_dir(name: &str) -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
        })
        .join("hyprdesk/quarantine");
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let d = base.join(format!("{name}-{}", crate::storage::revision()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn git(args: &[&std::ffi::OsStr]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        // Aucun transport en clair ni externe, même par un sous-module ou une redirection.
        .args([
            "-c",
            "protocol.allow=never",
            "-c",
            "protocol.https.allow=always",
            "-c",
            "protocol.ssh.allow=always",
        ])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| crate::tr!("git : {e}", e = e))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(crate::tr!(
            "git : {e}",
            e = String::from_utf8_lossy(&out.stderr).trim().to_string()
        ))
    }
}

fn sha256(path: &Path) -> Result<String, String> {
    let out = std::process::Command::new("sha256sum")
        .arg("--")
        .arg(path)
        .output()
        .map_err(|e| crate::tr!("sha256sum : {e}", e = e))?;
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .filter(|h| out.status.success() && h.len() == 64)
        .map(str::to_string)
        .ok_or_else(|| crate::tr!("empreinte de {p0} impossible", p0 = path.display()))
}

fn count_lua(dir: &Path) -> usize {
    let mut n = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "lua") {
                n += 1;
            }
        }
    }
    n
}

/// Prépare l'installation d'un plugin tiers depuis un fichier `.lua`, un
/// dossier contenant `init.lua`, ou un dépôt git (`https://…`, `git@…`,
/// suivi de `#<commit>` pour épingler une révision). Le plugin est copié en
/// quarantaine, hors des dossiers de plugins : il n'est pas chargé.
pub fn fetch(source: &str, dest: &Path) -> Result<Pending, String> {
    let source = source.trim();
    if !crate::policy::get().plugins_allowed() {
        return Err(crate::i18n::tr(
            "installation de plugins tiers interdite par la politique du poste",
        )
        .into());
    }
    let bad_name = |name: &str| {
        crate::tr!(
            "nom de plugin « {name} » invalide (minuscules, chiffres, - et _)",
            name = name
        )
    };
    if let Some(check) = git_source(source) {
        check?;
        let (url, rev) = match source.rsplit_once('#') {
            Some((u, r)) if !r.is_empty() => (u, Some(r)),
            _ => (source, None),
        };
        if rev.is_some_and(|r| !r.chars().all(|c| c.is_ascii_hexdigit()) || r.len() < 7) {
            return Err(crate::i18n::tr(
                "révision : un identifiant de commit (hexadécimal) attendu",
            )
            .into());
        }
        let name = url
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit(['/', ':'])
            .next()
            .unwrap_or_default()
            .trim_start_matches("hyprdesk-")
            .to_lowercase();
        if !valid_id(&name) {
            return Err(bad_name(&name));
        }
        if dest.join(&name).exists() {
            return Err(crate::tr!("« {name} » est déjà installé", name = name));
        }
        let q = quarantine_dir(&name)?;
        let item = q.join(&name);
        let os = |s: &str| std::ffi::OsString::from(s);
        let result = (|| {
            let mut clone = vec![os("clone"), os("--quiet")];
            if rev.is_none() {
                clone.extend([os("--depth"), os("1")]);
            }
            clone.extend([os("--"), os(url), item.clone().into_os_string()]);
            git(&clone.iter().map(|s| s.as_os_str()).collect::<Vec<_>>())?;
            let dir = item.as_os_str();
            if let Some(r) = rev {
                git(&[
                    os("-C").as_os_str(),
                    dir,
                    os("checkout").as_os_str(),
                    os("--quiet").as_os_str(),
                    os(r).as_os_str(),
                ])?;
            }
            let commit = git(&[
                os("-C").as_os_str(),
                dir,
                os("rev-parse").as_os_str(),
                os("HEAD").as_os_str(),
            ])?;
            if rev.is_some_and(|r| !commit.starts_with(&r.to_lowercase())) {
                return Err(crate::tr!(
                    "révision {r} introuvable",
                    r = rev.unwrap_or_default()
                ));
            }
            let init = item.join("init.lua");
            if !init.is_file() {
                return Err(crate::i18n::tr(
                    "dépôt sans init.lua à sa racine : ce n'est pas un plugin hyprdesk",
                )
                .into());
            }
            Ok(Pending {
                name: name.clone(),
                source: source.to_string(),
                quarantine: q.clone(),
                digest: sha256(&init)?,
                preview: preview(&init),
                lua_files: count_lua(&item),
                item: item.clone(),
                is_dir: true,
                commit: Some(commit),
            })
        })();
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&q);
        }
        return result;
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
        return Err(bad_name(&name));
    }
    let target_name = if is_dir {
        name.clone()
    } else {
        format!("{name}.lua")
    };
    if dest.join(&target_name).exists() {
        return Err(crate::tr!("« {name} » est déjà installé", name = name));
    }
    let q = quarantine_dir(&name)?;
    let item = q.join(&target_name);
    let result = (|| {
        if is_dir {
            let st = std::process::Command::new("cp")
                .arg("-r")
                .arg("--")
                .arg(&src)
                .arg(&item)
                .status()
                .map_err(|e| e.to_string())?;
            if !st.success() {
                return Err(crate::tr!("copie de {p0} impossible", p0 = src.display()));
            }
        } else {
            std::fs::copy(&src, &item).map_err(|e| e.to_string())?;
        }
        let entry = if is_dir {
            item.join("init.lua")
        } else {
            item.clone()
        };
        Ok(Pending {
            name: name.clone(),
            source: source.to_string(),
            quarantine: q.clone(),
            digest: sha256(&entry)?,
            preview: preview(&entry),
            lua_files: if is_dir { count_lua(&item) } else { 1 },
            item: item.clone(),
            is_dir,
            commit: None,
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&q);
    }
    result
}

fn preview(entry: &Path) -> String {
    std::fs::read_to_string(entry)
        .unwrap_or_default()
        .lines()
        .take(14)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Installe le plugin mis en quarantaine, après accord de l'utilisateur.
/// Rend son nom.
pub fn confirm(p: &Pending, dest: &Path) -> Result<String, String> {
    if !crate::policy::get().plugins_allowed() {
        return Err(crate::i18n::tr(
            "installation de plugins tiers interdite par la politique du poste",
        )
        .into());
    }
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let target = dest.join(p.item.file_name().unwrap_or_default());
    if target.exists() {
        return Err(crate::tr!("« {name} » est déjà installé", name = p.name));
    }
    // Le point d'entrée n'a pas changé depuis la relecture.
    let entry = if p.is_dir {
        p.item.join("init.lua")
    } else {
        p.item.clone()
    };
    if sha256(&entry)? != p.digest {
        discard(p);
        return Err(crate::i18n::tr(
            "le plugin a changé depuis sa relecture : installation annulée",
        )
        .into());
    }
    if std::fs::rename(&p.item, &target).is_err() {
        // Autre système de fichiers : copie.
        let st = std::process::Command::new("cp")
            .arg("-r")
            .arg("--")
            .arg(&p.item)
            .arg(&target)
            .status()
            .map_err(|e| e.to_string())?;
        if !st.success() {
            return Err(crate::tr!(
                "copie de {p0} impossible",
                p0 = p.item.display()
            ));
        }
    }
    discard(p);
    Ok(p.name.clone())
}

/// Renonce à un plugin en quarantaine.
pub fn discard(p: &Pending) {
    let _ = std::fs::remove_dir_all(&p.quarantine);
}

/// Prépare puis installe sans relecture : réservé aux tests.
#[cfg(test)]
pub fn install(source: &str, dest: &Path) -> Result<String, String> {
    let p = fetch(source, dest)?;
    confirm(&p, dest)
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

    #[test]
    fn quarantaine_avant_installation() {
        let base = std::env::temp_dir().join(format!("hyprdesk-quar-{}", std::process::id()));
        let (src, dest) = (base.join("src"), base.join("widgets"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("note.lua"), "return {}\n-- deux\n").unwrap();
        let s = src.join("note.lua").display().to_string();
        let p = fetch(&s, &dest).unwrap();
        // Rien dans le dossier des plugins tant que ce n'est pas confirmé.
        assert!(discover(std::slice::from_ref(&dest)).is_empty());
        assert_eq!(p.digest.len(), 64);
        assert!(p.preview.starts_with("return {}"));
        // Modifié après relecture : refusé.
        std::fs::write(&p.item, "os.execute('x')").unwrap();
        assert!(confirm(&p, &dest).is_err());
        assert!(!p.quarantine.exists() && !dest.join("note.lua").exists());
        let p = fetch(&s, &dest).unwrap();
        assert_eq!(confirm(&p, &dest).unwrap(), "note");
        assert!(dest.join("note.lua").is_file() && !p.quarantine.exists());
        // Transports refusés, sans rien cloner.
        for bad in [
            "http://exemple.org/x.git",
            "git://exemple.org/x.git",
            "file:///tmp/x.git",
            "ext::sh -c touch% /tmp/x",
        ] {
            assert!(fetch(bad, &dest).is_err(), "{bad}");
        }
        assert!(git_source("https://exemple.org/x.git").unwrap().is_ok());
        assert!(git_source("git@exemple.org:moi/x.git").unwrap().is_ok());
        assert!(fetch("https://exemple.org/x.git#pas-un-commit", &dest).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }
}
