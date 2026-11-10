//! Partage d'un setup : export et import d'une archive `.tar.gz`.
//!
//! Deux portées (`Scope`) :
//!
//! - **tout le setup** : `hyprdesk.lua`, `hyprdesk/settings.lua`, les plugins
//!   tiers (`hyprdesk/widgets/`), les fonds d'écran utilisés (`wallpapers/`)
//!   et, sur demande, `monitors.lua` (propre aux écrans de la machine) ;
//! - **le thème seul** (`crate::theme`) : couleurs, effets, fonds d'écran,
//!   mode du bureau, voile du verrouillage, transitions — écrits dans un
//!   `hyprdesk/settings.lua` qui ne contient que cela.
//!
//! Un `manifest.json` décrit le contenu, sa portée (`kind`), le résumé du
//! thème et la correspondance des fonds.
//!
//! L'import sauvegarde d'abord la configuration en place
//! (`hyprdesk/backups/`), recopie les fonds dans
//! `~/.local/share/hyprdesk/wallpapers/<archive>/` et réécrit leurs chemins.
//! Tout le setup : remplace les réglages, ajoute les plugins sans écraser
//! ceux déjà là. Le thème seul (archive de thème, ou thème d'une archive
//! complète) : remplace le thème des réglages, rien d'autre.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{WallSource, WallpaperRule};
use crate::settings::Settings;
use crate::theme::Summary;

/// Ce que l'on exporte ou importe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    /// Tout le setup.
    #[default]
    All,
    /// Le thème seul.
    Theme,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::All => "setup",
            Scope::Theme => "theme",
        }
    }
}

/// Ce que contient (ou contiendrait) une archive.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub hyprdesk: String,
    pub created: String,
    /// `setup` (tout) ou `theme` (le thème seul) ; vide : ancienne archive complète.
    #[serde(default)]
    pub kind: String,
    /// Le thème de l'archive.
    #[serde(default)]
    pub theme: Option<Summary>,
    pub files: Vec<String>,
    /// Fond d'écran : chemin d'origine → chemin dans l'archive.
    pub wallpapers: BTreeMap<String, String>,
    pub plugins: Vec<String>,
    pub screens: bool,
}

impl Manifest {
    fn new(scope: Scope, screens: bool) -> Self {
        Manifest {
            hyprdesk: env!("CARGO_PKG_VERSION").into(),
            created: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            kind: scope.as_str().into(),
            screens,
            ..Default::default()
        }
    }

    pub fn scope(&self) -> Scope {
        if self.kind == "theme" {
            Scope::Theme
        } else {
            Scope::All
        }
    }
}

fn tmp_dir(tag: &str) -> Result<PathBuf, String> {
    let d = std::env::temp_dir().join(format!(
        "hyprdesk-{tag}-{p0}-{p1}",
        tag = tag,
        p0 = std::process::id(),
        p1 = crate::storage::revision()
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
        .ok_or_else(|| crate::tr!("copie de {p0} impossible", p0 = from.display()))
}

fn is_image(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
        )
    })
}

/// Copie dans l'archive les images des fonds (pas seulement leurs chemins).
fn stage_wallpapers(rules: &[WallpaperRule], stage: &Path, m: &mut Manifest) -> Result<(), String> {
    let mut sources: Vec<PathBuf> = Vec::new();
    for w in rules {
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
    sources.dedup();
    let wdir = stage.join("wallpapers");
    for src in &sources {
        if m.wallpapers.contains_key(&src.display().to_string()) {
            continue;
        }
        let name = format!(
            "{:02}-{}",
            m.wallpapers.len(),
            src.file_name().and_then(|n| n.to_str()).unwrap_or("fond")
        );
        std::fs::create_dir_all(&wdir).map_err(|e| e.to_string())?;
        std::fs::copy(src, wdir.join(&name)).map_err(|e| e.to_string())?;
        m.wallpapers.insert(
            src.display().to_string(),
            format!("wallpapers/{name}", name = name),
        );
    }
    Ok(())
}

/// Écrit le manifeste et ferme l'archive.
fn pack(stage: &Path, out: &Path, m: &Manifest) -> Result<(), String> {
    std::fs::write(
        stage.join("manifest.json"),
        serde_json::to_string_pretty(m).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let st = Command::new("tar")
        .arg("czf")
        .arg(out)
        .arg("-C")
        .arg(stage)
        .arg(".")
        .status()
        .map_err(|e| crate::tr!("tar : {e}", e = e))?;
    st.success()
        .then_some(())
        .ok_or_else(|| crate::tr!("écriture de {p0} impossible", p0 = out.display()))
}

/// Manifeste d'une archive, sans l'extraire.
pub fn read_manifest(bundle: &Path) -> Result<Manifest, String> {
    for entry in ["./manifest.json", "manifest.json"] {
        let out = Command::new("tar")
            .arg("xzOf")
            .arg(bundle)
            .arg(entry)
            .stderr(std::process::Stdio::null())
            .output()
            .map_err(|e| crate::tr!("tar : {e}", e = e))?;
        if out.status.success()
            && let Ok(m) = serde_json::from_slice::<Manifest>(&out.stdout)
        {
            return Ok(m);
        }
    }
    Err(crate::tr!(
        "{p0} : pas de manifest.json, ce n'est pas un setup hyprdesk",
        p0 = bundle.display()
    ))
}

/// Exporte dans `out` tout le setup ou le thème seul. `screens` (tout le
/// setup) : inclure `monitors.lua`.
pub fn export(
    config_path: &Path,
    out: &Path,
    scope: Scope,
    screens: bool,
) -> Result<Manifest, String> {
    match scope {
        Scope::All => export_all(config_path, out, screens),
        Scope::Theme => export_theme(config_path, out, None),
    }
}

/// Exporte le thème effectif ; `name` le renomme dans l'archive.
pub fn export_theme(
    config_path: &Path,
    out: &Path,
    name: Option<&str>,
) -> Result<Manifest, String> {
    let cfg = crate::theme::effective(config_path)?;
    let mut theme = Settings::theme_of(&cfg);
    if let Some(n) = name.map(str::trim).filter(|n| !n.is_empty()) {
        theme.theme_name = Some(n.to_string());
    }
    pack_theme(&theme, out)
}

/// Écrit l'archive d'un thème (réglages ne portant que le thème, couleurs
/// explicites) avec les images de ses fonds.
pub fn pack_theme(theme: &Settings, out: &Path) -> Result<Manifest, String> {
    let d = crate::config::Theme::default();
    let h = crate::settings::hex;
    let summary = Summary {
        name: theme.theme_name.clone().unwrap_or_else(|| d.name.clone()),
        fg: h(theme.fg.unwrap_or(d.fg)),
        bg: h(theme.bg.unwrap_or(d.bg)),
        accent: h(theme.accent.unwrap_or(d.accent)),
    };
    let stage = tmp_dir("export")?;
    let result = (|| {
        let mut m = Manifest::new(Scope::Theme, false);
        m.theme = Some(summary);
        std::fs::create_dir_all(stage.join("hyprdesk")).map_err(|e| e.to_string())?;
        std::fs::write(stage.join("hyprdesk/settings.lua"), theme.to_lua())
            .map_err(|e| e.to_string())?;
        m.files.push("hyprdesk/settings.lua".into());
        stage_wallpapers(&theme.wallpapers, &stage, &mut m)?;
        pack(&stage, out, &m)?;
        Ok(m)
    })();
    let _ = std::fs::remove_dir_all(&stage);
    result
}

fn export_all(config_path: &Path, out: &Path, screens: bool) -> Result<Manifest, String> {
    let dir = config_path.parent().unwrap_or(Path::new("."));
    let stage = tmp_dir("export")?;
    let result = (|| {
        let mut m = Manifest::new(Scope::All, screens);
        // Thème effectif (résumé) et fonds de toute la config ; une config en
        // erreur s'exporte quand même, telle quelle.
        let effective = crate::theme::effective(config_path).ok();
        m.theme = effective.as_ref().map(|c| Summary::of(&c.general.theme));
        std::fs::create_dir_all(stage.join("hyprdesk")).map_err(|e| e.to_string())?;
        if config_path.is_file() {
            std::fs::copy(config_path, stage.join("hyprdesk.lua")).map_err(|e| e.to_string())?;
            m.files.push("hyprdesk.lua".into());
        }
        let settings = crate::config::lua::settings_path(config_path);
        if settings.is_file() {
            std::fs::copy(&settings, stage.join("hyprdesk/settings.lua"))
                .map_err(|e| e.to_string())?;
            m.files.push("hyprdesk/settings.lua".into());
            let cfg = crate::config::lua::load_settings_only(config_path)?;
            stage_wallpapers(&cfg.wallpapers, &stage, &mut m)?;
        }
        if let Some(c) = &effective {
            stage_wallpapers(&c.wallpapers, &stage, &mut m)?;
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
        pack(&stage, out, &m)?;
        Ok(m)
    })();
    let _ = std::fs::remove_dir_all(&stage);
    result
}

/// Options d'un import.
#[derive(Debug, Clone, Copy, Default)]
pub struct ImportOpts {
    /// Portée demandée ; une archive de thème s'importe toujours en thème seul.
    pub scope: Scope,
    /// Tout le setup : appliquer aussi `monitors.lua`.
    pub screens: bool,
    /// Simulation : rien n'est modifié.
    pub dry_run: bool,
    /// Sauvegarder la configuration en place avant (sauf simulation).
    pub backup: bool,
}

/// Ce que fait (ou ferait, `dry_run`) un import.
#[derive(Debug, Default)]
pub struct Report {
    pub manifest: Manifest,
    /// Portée effective.
    pub scope: Scope,
    pub dry_run: bool,
    pub backup: Option<PathBuf>,
    pub plugins_added: Vec<String>,
    pub plugins_kept: Vec<String>,
    pub wallpapers_dir: Option<PathBuf>,
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = &self.manifest;
        let name = m.theme.as_ref().map(|t| t.name.as_str()).unwrap_or("");
        match self.scope {
            Scope::Theme => writeln!(
                f,
                "{}",
                crate::tr!(
                    "Thème « {name} » (hyprdesk {version} du {date}) — le reste de la configuration ne change pas",
                    name = name,
                    version = m.hyprdesk,
                    date = m.created
                )
            )?,
            Scope::All => {
                writeln!(
                    f,
                    "{}",
                    crate::tr!(
                        "Setup hyprdesk {version} du {date}",
                        version = m.hyprdesk,
                        date = m.created
                    )
                )?;
                if !name.is_empty() {
                    writeln!(f, "{}", crate::tr!("  thème : {name}", name = name))?;
                }
                writeln!(
                    f,
                    "{}",
                    crate::tr!("  fichiers : {list}", list = m.files.join(", "))
                )?;
            }
        }
        writeln!(
            f,
            "{}",
            crate::tr!("  fonds d'écran : {n}", n = m.wallpapers.len())
        )?;
        if let Some(d) = &self.wallpapers_dir {
            writeln!(
                f,
                "{}",
                crate::tr!("    copiés dans {dir}", dir = d.display())
            )?;
        }
        if !self.plugins_added.is_empty() {
            writeln!(
                f,
                "{}",
                crate::tr!(
                    "  plugins ajoutés : {list}",
                    list = self.plugins_added.join(", ")
                )
            )?;
        }
        if !self.plugins_kept.is_empty() {
            writeln!(
                f,
                "{}",
                crate::tr!(
                    "  plugins déjà présents (gardés) : {list}",
                    list = self.plugins_kept.join(", ")
                )
            )?;
        }
        match (&self.backup, self.dry_run) {
            (_, true) => write!(
                f,
                "{}",
                crate::i18n::tr("  (simulation : rien n'a été modifié)")
            ),
            (Some(b), _) => write!(
                f,
                "{}",
                crate::tr!(
                    "  ancienne configuration sauvegardée : {path}",
                    path = b.display()
                )
            ),
            (None, _) => write!(f, "{}", crate::i18n::tr("  appliqué")),
        }
    }
}

/// Importe une archive.
pub fn import(bundle: &Path, config_path: &Path, opts: ImportOpts) -> Result<Report, String> {
    let stage = tmp_dir("import")?;
    let st = Command::new("tar")
        .arg("xzf")
        .arg(bundle)
        .arg("-C")
        .arg(&stage)
        .status()
        .map_err(|e| crate::tr!("tar : {e}", e = e))?;
    if !st.success() {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(crate::tr!(
            "{p0} : archive illisible",
            p0 = bundle.display()
        ));
    }
    let result = import_from(&stage, bundle, config_path, opts);
    let _ = std::fs::remove_dir_all(&stage);
    result
}

/// Fonds aux chemins de cette machine. `drop_missing` : un fond qui n'est ni
/// dans l'archive ni présent ici est retiré (thème seul).
fn relocate(
    rules: Vec<WallpaperRule>,
    map: &BTreeMap<PathBuf, PathBuf>,
    wall_dir: &Path,
    drop_missing: bool,
) -> Vec<WallpaperRule> {
    rules
        .into_iter()
        .filter_map(|mut w| {
            match &w.source {
                WallSource::Path(p) => match map.get(p) {
                    Some(t) => w.source = WallSource::Path(t.clone()),
                    None if drop_missing && !p.is_file() => return None,
                    None => {}
                },
                WallSource::Rotate {
                    dir,
                    interval,
                    shuffle,
                } => {
                    // Diaporama : ses images importées sont dans le dossier de l'archive.
                    if map.keys().any(|k| k.starts_with(dir)) {
                        w.source = WallSource::Rotate {
                            dir: wall_dir.to_path_buf(),
                            interval: *interval,
                            shuffle: *shuffle,
                        };
                    } else if drop_missing && !dir.is_dir() {
                        return None;
                    }
                }
                WallSource::Color(_) => {}
            }
            Some(w)
        })
        .collect()
}

fn import_from(
    stage: &Path,
    bundle: &Path,
    config_path: &Path,
    opts: ImportOpts,
) -> Result<Report, String> {
    let manifest: Manifest = std::fs::read_to_string(stage.join("manifest.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .ok_or(crate::i18n::tr(
            "pas de manifest.json : ce n'est pas un setup hyprdesk",
        ))?;
    let scope = if manifest.scope() == Scope::Theme {
        Scope::Theme
    } else {
        opts.scope
    };
    let dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    // Vérifier que la configuration importée se charge, avant de toucher à quoi que ce soit.
    if !stage.join("hyprdesk.lua").is_file() {
        std::fs::write(stage.join("hyprdesk.lua"), "").map_err(|e| e.to_string())?;
    }
    let imported = crate::config::lua::load_file(&stage.join("hyprdesk.lua"))
        .map_err(|e| crate::tr!("configuration importée invalide : {e}", e = e))?
        .config;
    let user_widgets = dir.join("hyprdesk/widgets");
    let mut report = Report {
        manifest: manifest.clone(),
        scope,
        dry_run: opts.dry_run,
        ..Default::default()
    };
    if scope == Scope::All {
        for p in &manifest.plugins {
            if user_widgets.join(p).exists() {
                report.plugins_kept.push(p.clone());
            } else {
                report.plugins_added.push(p.clone());
            }
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
    if opts.dry_run {
        return Ok(report);
    }
    // 1. Sauvegarde de l'existant.
    if opts.backup {
        let backup = dir.join(format!(
            "hyprdesk/backups/avant-import-{p0}.tar.gz",
            p0 = chrono::Local::now().format("%Y%m%d-%H%M%S")
        ));
        export_all(config_path, &backup, true)?;
        report.backup = Some(backup);
    }
    // 2. Fonds d'écran, puis réglages aux chemins réécrits.
    let mut map: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for (orig, inside) in &manifest.wallpapers {
        std::fs::create_dir_all(&wall_dir).map_err(|e| e.to_string())?;
        let target = wall_dir.join(Path::new(inside).file_name().unwrap_or_default());
        std::fs::copy(stage.join(inside), &target).map_err(|e| e.to_string())?;
        map.insert(PathBuf::from(orig), target);
    }
    if scope == Scope::Theme {
        // Le thème effectif de l'archive remplace celui des réglages en place.
        let mut theme = Settings::theme_of(&imported);
        theme.wallpapers = relocate(theme.wallpapers, &map, &wall_dir, true);
        let mut s = Settings::from_config(&crate::config::lua::load_settings_only(config_path)?);
        s.set_theme(&theme);
        s.save_versioned(config_path, crate::storage::revision())
            .map_err(|e| crate::tr!("settings.lua : {e}", e = e))?;
        return Ok(report);
    }
    if stage.join("hyprdesk/settings.lua").is_file() {
        let cfg = crate::config::lua::load_settings_only(&stage.join("hyprdesk.lua"))?;
        let mut s = Settings::from_config(&cfg);
        s.wallpapers = relocate(std::mem::take(&mut s.wallpapers), &map, &wall_dir, false);
        s.save_versioned(config_path, crate::storage::revision())
            .map_err(|e| crate::tr!("settings.lua : {e}", e = e))?;
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
    if opts.screens && stage.join("monitors.lua").is_file() {
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

    /// `XDG_DATA_HOME` commun aux tests (variable du processus, tests en parallèle).
    fn data_home() -> PathBuf {
        let d = std::env::temp_dir().join(format!("hyprdesk-share-data-{}", std::process::id()));
        // SAFETY : seuls ces tests lisent cette variable, toujours à la même valeur.
        unsafe { std::env::set_var("XDG_DATA_HOME", &d) };
        d
    }

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
        let m = export(&a.join("hyprdesk.lua"), &bundle, Scope::All, false).unwrap();
        assert_eq!(m.plugins, ["meteo.lua"]);
        assert_eq!(m.wallpapers.len(), 1);
        // Simulation sur B : rien ne bouge.
        let opts = |dry_run| ImportOpts {
            dry_run,
            backup: true,
            ..Default::default()
        };
        let r = import(&bundle, &b.join("hyprdesk.lua"), opts(true)).unwrap();
        assert!(r.backup.is_none() && !b.join("hyprdesk.lua").exists());
        // Import sur B (XDG_DATA_HOME isolé).
        let data = data_home();
        let r = import(&bundle, &b.join("hyprdesk.lua"), opts(false)).unwrap();
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
        assert!(p.starts_with(data.join("hyprdesk/wallpapers")) && p.is_file());
        assert!(cfg.catalog.iter().any(|t| t.id == "meteo"));
        assert!(
            import(
                &base.join("rien.tar.gz"),
                &b.join("hyprdesk.lua"),
                opts(true)
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn exporter_le_theme_seul() {
        use crate::config::Place;
        let base = std::env::temp_dir().join(format!("hyprdesk-theme-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        for d in [&a, &b] {
            std::fs::create_dir_all(d.join("hyprdesk")).unwrap();
        }
        // Machine A : un thème, un fond, et des réglages qui ne sont pas du thème.
        let img = a.join("fond.png");
        std::fs::write(&img, b"PNG").unwrap();
        std::fs::write(
            a.join("hyprdesk.lua"),
            format!(
                "hd.config({{ workspaces = 7, theme = {{ name = \"Nuit\", accent = \"#ff0000\", \
                 effects = {{ bar = {{ opacity = 0.4, blur = false }} }} }} }})\n\
                 hd.wallpaper({{ path = \"{}\" }})\n",
                img.display()
            ),
        )
        .unwrap();
        let bundle = base.join("nuit.tar.gz");
        let m = export(&a.join("hyprdesk.lua"), &bundle, Scope::Theme, false).unwrap();
        assert_eq!(m.scope(), Scope::Theme);
        assert_eq!(m.theme.as_ref().unwrap().name, "Nuit");
        assert_eq!(m.wallpapers.len(), 1);
        assert!(m.plugins.is_empty() && !m.files.contains(&"hyprdesk.lua".to_string()));
        assert_eq!(read_manifest(&bundle).unwrap(), m);
        // Machine B : ses propres réglages ; l'import du thème n'y touche pas.
        std::fs::write(b.join("hyprdesk.lua"), "").unwrap();
        std::fs::write(
            b.join("hyprdesk/settings.lua"),
            "hd.config({ workspaces = 3 })\n",
        )
        .unwrap();
        let data = data_home();
        // Même en demandant « tout », une archive de thème n'importe que le thème.
        let r = import(&bundle, &b.join("hyprdesk.lua"), ImportOpts::default()).unwrap();
        assert_eq!(r.scope, Scope::Theme);
        let cfg = crate::config::lua::load_file(&b.join("hyprdesk.lua"))
            .unwrap()
            .config;
        assert_eq!(cfg.general.workspaces, Some(3), "le reste ne bouge pas");
        assert_eq!(cfg.general.theme.name, "Nuit");
        assert_eq!(
            cfg.general.theme.accent,
            crate::config::Rgba(0xff, 0, 0, 0xff)
        );
        let bar = cfg.general.theme.effect(Place::Bar);
        assert_eq!((bar.opacity, bar.blur), (0.4, false));
        let WallSource::Path(p) = &cfg.wallpapers[0].source else {
            panic!("fond");
        };
        assert!(p.starts_with(data.join("hyprdesk/wallpapers")) && p.is_file());
        // Le thème d'une archive complète s'importe seul aussi.
        let full = base.join("complet.tar.gz");
        export(&a.join("hyprdesk.lua"), &full, Scope::All, false).unwrap();
        std::fs::write(
            b.join("hyprdesk/settings.lua"),
            "hd.config({ workspaces = 5 })\n",
        )
        .unwrap();
        let r = import(
            &full,
            &b.join("hyprdesk.lua"),
            ImportOpts {
                scope: Scope::Theme,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.scope, Scope::Theme);
        let cfg = crate::config::lua::load_file(&b.join("hyprdesk.lua"))
            .unwrap()
            .config;
        assert_eq!(cfg.general.workspaces, Some(5));
        assert_eq!(cfg.general.theme.name, "Nuit");
        let _ = std::fs::remove_dir_all(&base);
    }
}
