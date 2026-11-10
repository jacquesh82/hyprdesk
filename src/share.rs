//! Partage d'un setup : export et import d'une archive `.tar.gz`.
//!
//! L'archive contient `hyprdesk.lua`, `hyprdesk/settings.lua`, les plugins
//! tiers (`hyprdesk/widgets/`), les fonds d'écran utilisés (`wallpapers/`) et,
//! sur demande, `monitors.lua` (propre aux écrans de la machine). Un
//! `manifest.json` décrit le contenu et la correspondance des fonds.
//!
//! L'import sauvegarde d'abord la configuration en place
//! (`hyprdesk/backups/`), recopie les fonds dans
//! `~/.local/share/hyprdesk/wallpapers/<archive>/` et réécrit leurs chemins
//! dans `settings.lua`, ajoute les plugins sans écraser ceux déjà là.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{Selector, WallSource};
use crate::settings::Settings;

/// Ce que contient (ou contiendrait) une archive.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub hyprdesk: String,
    pub created: String,
    pub files: Vec<String>,
    /// Fond d'écran : chemin d'origine → chemin dans l'archive.
    pub wallpapers: BTreeMap<String, String>,
    pub plugins: Vec<String>,
    pub screens: bool,
}

fn tmp_dir(tag: &str) -> Result<PathBuf, String> {
    let d = std::env::temp_dir().join(format!(
        "hyprdesk-{tag}-{}-{}",
        std::process::id(),
        crate::storage::revision()
    ));
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let st = Command::new("cp")
        .arg("-r")
        .arg(from)
        .arg(to)
        .status()
        .map_err(|e| e.to_string())?;
    st.success()
        .then_some(())
        .ok_or_else(|| format!("copie de {} impossible", from.display()))
}

fn is_image(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
        )
    })
}

/// Exporte le setup dans `out`. `screens` : inclure `monitors.lua`.
pub fn export(config_path: &Path, out: &Path, screens: bool) -> Result<Manifest, String> {
    let dir = config_path.parent().unwrap_or(Path::new("."));
    let stage = tmp_dir("export")?;
    let mut m = Manifest {
        hyprdesk: env!("CARGO_PKG_VERSION").into(),
        created: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        screens,
        ..Default::default()
    };
    std::fs::create_dir_all(stage.join("hyprdesk")).map_err(|e| e.to_string())?;
    if config_path.is_file() {
        std::fs::copy(config_path, stage.join("hyprdesk.lua")).map_err(|e| e.to_string())?;
        m.files.push("hyprdesk.lua".into());
    }
    let settings = crate::config::lua::settings_path(config_path);
    if settings.is_file() {
        std::fs::copy(&settings, stage.join("hyprdesk/settings.lua")).map_err(|e| e.to_string())?;
        m.files.push("hyprdesk/settings.lua".into());
        // Fonds d'écran des réglages : les images, pas seulement leurs chemins.
        let cfg = crate::config::lua::load_settings_only(config_path)?;
        let mut sources: Vec<PathBuf> = Vec::new();
        for w in &cfg.wallpapers {
            match &w.source {
                WallSource::Path(p) if p.is_file() => sources.push(p.clone()),
                WallSource::Rotate { dir, .. } => {
                    if let Ok(rd) = std::fs::read_dir(dir) {
                        let mut imgs: Vec<PathBuf> = rd
                            .flatten()
                            .map(|e| e.path())
                            .filter(|p| is_image(p))
                            .collect();
                        imgs.sort();
                        sources.extend(imgs.into_iter().take(60));
                    }
                }
                _ => {}
            }
        }
        let wdir = stage.join("wallpapers");
        for (i, src) in sources.iter().enumerate() {
            let name = format!(
                "{i:02}-{}",
                src.file_name().and_then(|n| n.to_str()).unwrap_or("fond")
            );
            std::fs::create_dir_all(&wdir).map_err(|e| e.to_string())?;
            std::fs::copy(src, wdir.join(&name)).map_err(|e| e.to_string())?;
            m.wallpapers
                .insert(src.display().to_string(), format!("wallpapers/{name}"));
        }
    }
    // Plugins tiers (pas ceux retirés).
    let widgets = dir.join("hyprdesk/widgets");
    if let Ok(rd) = std::fs::read_dir(&widgets) {
        std::fs::create_dir_all(stage.join("hyprdesk/widgets")).map_err(|e| e.to_string())?;
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            copy_tree(&e.path(), &stage.join("hyprdesk/widgets").join(&name))?;
            m.plugins.push(name);
        }
    }
    if screens {
        let mon = crate::screens::monitors_path(config_path);
        if mon.is_file() {
            std::fs::copy(&mon, stage.join("monitors.lua")).map_err(|e| e.to_string())?;
            m.files.push("monitors.lua".into());
        }
    }
    std::fs::write(
        stage.join("manifest.json"),
        serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let st = Command::new("tar")
        .arg("czf")
        .arg(out)
        .arg("-C")
        .arg(&stage)
        .arg(".")
        .status()
        .map_err(|e| format!("tar : {e}"))?;
    let _ = std::fs::remove_dir_all(&stage);
    st.success()
        .then_some(m)
        .ok_or_else(|| format!("écriture de {} impossible", out.display()))
}

/// Ce que fait (ou ferait, `dry_run`) un import.
#[derive(Debug, Default)]
pub struct Report {
    pub manifest: Manifest,
    pub backup: Option<PathBuf>,
    pub plugins_added: Vec<String>,
    pub plugins_kept: Vec<String>,
    pub wallpapers_dir: Option<PathBuf>,
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = &self.manifest;
        writeln!(f, "Setup hyprdesk {} du {}", m.hyprdesk, m.created)?;
        writeln!(f, "  fichiers : {}", m.files.join(", "))?;
        writeln!(f, "  fonds d'écran : {}", m.wallpapers.len())?;
        if let Some(d) = &self.wallpapers_dir {
            writeln!(f, "    copiés dans {}", d.display())?;
        }
        if !self.plugins_added.is_empty() {
            writeln!(f, "  plugins ajoutés : {}", self.plugins_added.join(", "))?;
        }
        if !self.plugins_kept.is_empty() {
            writeln!(
                f,
                "  plugins déjà présents (gardés) : {}",
                self.plugins_kept.join(", ")
            )?;
        }
        match &self.backup {
            Some(b) => write!(f, "  ancienne configuration sauvegardée : {}", b.display()),
            None => write!(f, "  (simulation : rien n'a été modifié)"),
        }
    }
}

/// Importe une archive. `screens` : appliquer aussi son `monitors.lua`.
pub fn import(
    bundle: &Path,
    config_path: &Path,
    screens: bool,
    dry_run: bool,
) -> Result<Report, String> {
    let stage = tmp_dir("import")?;
    let st = Command::new("tar")
        .arg("xzf")
        .arg(bundle)
        .arg("-C")
        .arg(&stage)
        .status()
        .map_err(|e| format!("tar : {e}"))?;
    if !st.success() {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(format!("{} : archive illisible", bundle.display()));
    }
    let result = import_from(&stage, bundle, config_path, screens, dry_run);
    let _ = std::fs::remove_dir_all(&stage);
    result
}

fn import_from(
    stage: &Path,
    bundle: &Path,
    config_path: &Path,
    screens: bool,
    dry_run: bool,
) -> Result<Report, String> {
    let manifest: Manifest = std::fs::read_to_string(stage.join("manifest.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .ok_or("pas de manifest.json : ce n'est pas un setup hyprdesk")?;
    let dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    // Vérifier que la configuration importée se charge, avant de toucher à quoi que ce soit.
    if stage.join("hyprdesk.lua").is_file() || stage.join("hyprdesk/settings.lua").is_file() {
        if !stage.join("hyprdesk.lua").is_file() {
            std::fs::write(stage.join("hyprdesk.lua"), "").map_err(|e| e.to_string())?;
        }
        crate::config::lua::load_file(&stage.join("hyprdesk.lua"))
            .map_err(|e| format!("configuration importée invalide : {e}"))?;
    }
    let user_widgets = dir.join("hyprdesk/widgets");
    let mut report = Report {
        manifest: manifest.clone(),
        ..Default::default()
    };
    for p in &manifest.plugins {
        if user_widgets.join(p).exists() {
            report.plugins_kept.push(p.clone());
        } else {
            report.plugins_added.push(p.clone());
        }
    }
    let name = bundle
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("setup")
        .trim_end_matches(".tar")
        .to_string();
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        });
    let wall_dir = data.join("hyprdesk/wallpapers").join(&name);
    if !manifest.wallpapers.is_empty() {
        report.wallpapers_dir = Some(wall_dir.clone());
    }
    if dry_run {
        return Ok(report);
    }
    // 1. Sauvegarde de l'existant.
    let backup = dir.join(format!(
        "hyprdesk/backups/avant-import-{}.tar.gz",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    export(config_path, &backup, true)?;
    report.backup = Some(backup);
    // 2. Fonds d'écran, puis réglages aux chemins réécrits.
    let mut map: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for (orig, inside) in &manifest.wallpapers {
        std::fs::create_dir_all(&wall_dir).map_err(|e| e.to_string())?;
        let target = wall_dir.join(Path::new(inside).file_name().unwrap_or_default());
        std::fs::copy(stage.join(inside), &target).map_err(|e| e.to_string())?;
        map.insert(PathBuf::from(orig), target);
    }
    if stage.join("hyprdesk/settings.lua").is_file() {
        let cfg = crate::config::lua::load_settings_only(&stage.join("hyprdesk.lua"))?;
        let mut s = Settings::from_config(&cfg);
        let walls = s.wallpapers.clone();
        for w in walls {
            let source = match &w.source {
                WallSource::Path(p) => map.get(p).cloned().map(WallSource::Path),
                WallSource::Rotate {
                    dir: d,
                    interval,
                    shuffle,
                } => {
                    // Diaporama : ses images importées sont dans le dossier de l'archive.
                    map.keys()
                        .any(|k| k.starts_with(d))
                        .then(|| WallSource::Rotate {
                            dir: wall_dir.clone(),
                            interval: *interval,
                            shuffle: *shuffle,
                        })
                }
                WallSource::Color(_) => None,
            };
            if let Some(src) = source {
                s.set_wallpaper(Selector { ..w.sel.clone() }, src, w.fit);
            }
        }
        s.save_versioned(config_path, crate::storage::revision())
            .map_err(|e| format!("settings.lua : {e}"))?;
    }
    if stage
        .join("hyprdesk.lua")
        .metadata()
        .is_ok_and(|m| m.len() > 0)
    {
        std::fs::copy(stage.join("hyprdesk.lua"), config_path).map_err(|e| e.to_string())?;
    }
    // 3. Plugins (sans écraser), écrans sur demande.
    for p in &report.plugins_added {
        std::fs::create_dir_all(&user_widgets).map_err(|e| e.to_string())?;
        copy_tree(
            &stage.join("hyprdesk/widgets").join(p),
            &user_widgets.join(p),
        )?;
    }
    if screens && stage.join("monitors.lua").is_file() {
        std::fs::copy(
            stage.join("monitors.lua"),
            crate::screens::monitors_path(config_path),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exporter_puis_importer_ailleurs() {
        let base = std::env::temp_dir().join(format!("hyprdesk-share-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        for d in [&a, &b] {
            std::fs::create_dir_all(d.join("hyprdesk/widgets")).unwrap();
        }
        // Machine A : un fond, un plugin, des réglages.
        let img = a.join("fond.png");
        std::fs::write(&img, b"PNG").unwrap();
        std::fs::write(a.join("hyprdesk.lua"), "hd.config({ fade_ms = 120 })").unwrap();
        std::fs::write(
            a.join("hyprdesk/settings.lua"),
            format!(
                "hd.config({{ workspaces = 4 }})\nhd.wallpaper({{ path = \"{}\" }})\n",
                img.display()
            ),
        )
        .unwrap();
        std::fs::write(
            a.join("hyprdesk/widgets/meteo.lua"),
            "return { render = function() return 'x' end }",
        )
        .unwrap();
        let bundle = base.join("setup.tar.gz");
        let m = export(&a.join("hyprdesk.lua"), &bundle, false).unwrap();
        assert_eq!(m.plugins, ["meteo.lua"]);
        assert_eq!(m.wallpapers.len(), 1);
        // Simulation sur B : rien ne bouge.
        let r = import(&bundle, &b.join("hyprdesk.lua"), false, true).unwrap();
        assert!(r.backup.is_none() && !b.join("hyprdesk.lua").exists());
        // Import sur B (XDG_DATA_HOME isolé).
        // SAFETY : test seul à lire cette variable dans ce processus.
        unsafe { std::env::set_var("XDG_DATA_HOME", base.join("data")) };
        let r = import(&bundle, &b.join("hyprdesk.lua"), false, false).unwrap();
        assert!(r.backup.as_ref().unwrap().is_file());
        assert_eq!(r.plugins_added, ["meteo.lua"]);
        let cfg = crate::config::lua::load_file(&b.join("hyprdesk.lua"))
            .unwrap()
            .config;
        assert_eq!(cfg.general.workspaces, Some(4));
        assert_eq!(cfg.general.fade_ms, 120);
        let WallSource::Path(p) = &cfg.wallpapers[0].source else {
            panic!("fond");
        };
        assert!(p.starts_with(base.join("data/hyprdesk/wallpapers")) && p.is_file());
        assert!(cfg.catalog.iter().any(|t| t.id == "meteo"));
        assert!(
            import(
                &base.join("rien.tar.gz"),
                &b.join("hyprdesk.lua"),
                false,
                true
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
