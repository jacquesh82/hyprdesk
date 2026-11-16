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
//! Une archive vient d'ailleurs : son Lua n'est jamais exécuté avec les
//! droits de l'utilisateur pendant l'import. La vérification (`dry_run`) ne
//! fait que compiler les sources (erreurs de syntaxe), sans rien exécuter ;
//! l'import lit les réglages de l'archive dans un bac à sable
//! (`config::lua::load_file_sandboxed` : ni fichiers, ni commandes). Le thème
//! seul n'emporte aucun code : ses valeurs sont réécrites dans
//! `settings.lua`. Tout le setup recopie `hyprdesk.lua` et les plugins, qui
//! s'exécuteront ensuite comme la configuration de l'utilisateur : l'import
//! exige alors `trust_code`, accordé après avertissement. Les services d'IA
//! (`ai_services`, `ai_use`) d'une archive sont ignorés : la configuration en
//! place garde les siens, et leurs clés.
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

/// Dossier de travail neuf et privé (0700), créé de façon exclusive.
fn tmp_dir(tag: &str) -> Result<PathBuf, String> {
    use std::os::unix::fs::DirBuilderExt;
    let base = crate::policy::runtime_dir().unwrap_or_else(|_| std::env::temp_dir());
    let d = base.join(format!(
        "hyprdesk-{tag}-{p0}-{p1}",
        tag = tag,
        p0 = std::process::id(),
        p1 = crate::storage::revision()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&d)
        .map_err(|e| format!("{} : {e}", d.display()))?;
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
    /// Tout le setup : l'utilisateur accepte le code Lua de l'archive
    /// (`hyprdesk.lua`, plugins), qui s'exécutera avec ses droits. Sans cet
    /// accord, une archive qui en contient est refusée.
    pub trust_code: bool,
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
    /// Code Lua que l'import installerait (tout le setup) : `hyprdesk.lua`
    /// et les plugins ajoutés.
    pub code: Vec<String>,
    /// Services d'IA déclarés par l'archive, ignorés.
    pub ai_ignored: bool,
}

impl Report {
    /// L'import installerait du code exécuté ensuite avec les droits de l'utilisateur.
    pub fn has_code(&self) -> bool {
        !self.code.is_empty()
    }
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
        if self.has_code() {
            writeln!(
                f,
                "{}",
                crate::tr!(
                    "  ⚠ code Lua : {list} — il s'exécutera avec vos droits (fichiers, commandes). N'importez que ce dont vous connaissez l'auteur.",
                    list = self.code.join(", ")
                )
            )?;
        }
        if self.ai_ignored {
            writeln!(
                f,
                "{}",
                crate::i18n::tr(
                    "  services d'IA de l'archive ignorés : les vôtres et leurs clés restent en place"
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
        .arg("--no-same-owner")
        .arg("--no-same-permissions")
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
    let result =
        only_regular_files(&stage).and_then(|()| import_from(&stage, bundle, config_path, opts));
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
    if !stage.join("hyprdesk.lua").is_file() {
        std::fs::write(stage.join("hyprdesk.lua"), "").map_err(|e| e.to_string())?;
    }
    let user_widgets = dir.join("hyprdesk/widgets");
    let mut report = Report {
        manifest: manifest.clone(),
        scope,
        dry_run: opts.dry_run,
        ..Default::default()
    };
    if scope == Scope::All {
        for p in &manifest.plugins {
            // Le nom vient de l'archive : jamais de chemin.
            if !crate::widgets::plugins::valid_id(p.strip_suffix(".lua").unwrap_or(p)) {
                return Err(crate::tr!("plugin « {p} » : nom invalide", p = p));
            }
            if user_widgets.join(p).exists() {
                report.plugins_kept.push(p.clone());
            } else {
                report.plugins_added.push(p.clone());
            }
        }
        if has_lua_code(&stage.join("hyprdesk.lua")) {
            report.code.push("hyprdesk.lua".into());
        }
        report.code.extend(
            report
                .plugins_added
                .iter()
                .map(|p| crate::tr!("plugin {p}", p = p)),
        );
    }
    // Vérification : les sources sont compilées, jamais exécutées.
    syntax_check_stage(stage)?;
    if opts.dry_run {
        let name = bundle_name(bundle);
        if !manifest.wallpapers.is_empty() {
            report.wallpapers_dir = Some(wall_dir_for(&name));
        }
        report.ai_ignored = scope == Scope::All && declares_ai(stage);
        return Ok(report);
    }
    if scope == Scope::All && report.has_code() && !opts.trust_code {
        return Err(crate::tr!(
            "cette archive contient du code Lua ({list}) qui s'exécuterait avec vos droits : confirmez pour l'importer",
            list = report.code.join(", ")
        ));
    }
    // Les réglages de l'archive, lus dans le bac à sable. Thème seul : à
    // défaut de hyprdesk.lua, ceux de settings.lua suffisent.
    let imported = match crate::config::lua::load_file_sandboxed(&stage.join("hyprdesk.lua")) {
        Ok(l) => l.config,
        Err(_) if scope == Scope::Theme => {
            crate::config::lua::load_settings_only_sandboxed(&stage.join("hyprdesk.lua"))
                .map_err(|e| crate::tr!("configuration importée invalide : {e}", e = e))?
        }
        Err(e) => return Err(crate::tr!("configuration importée invalide : {e}", e = e)),
    };
    let name = bundle_name(bundle);
    let wall_dir = wall_dir_for(&name);
    if !manifest.wallpapers.is_empty() {
        report.wallpapers_dir = Some(wall_dir.clone());
    }
    // Historique de la configuration : l'état d'avant l'import, et les
    // écritures qui suivent rangées sous le même motif.
    crate::history::before(config_path, "import");
    let _reason = crate::history::reason("import");
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
        // Chemin venu du manifeste : une image du dossier wallpapers/ de l'archive, rien d'autre.
        let file = Path::new(inside).file_name().unwrap_or_default();
        let src = stage.join("wallpapers").join(file);
        if file.is_empty() || !std::fs::symlink_metadata(&src).is_ok_and(|m| m.is_file()) {
            return Err(crate::tr!("fond « {p} » : chemin invalide", p = inside));
        }
        std::fs::create_dir_all(&wall_dir).map_err(|e| e.to_string())?;
        let target = wall_dir.join(file);
        std::fs::copy(&src, &target).map_err(|e| e.to_string())?;
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
        let cfg = crate::config::lua::load_settings_only_sandboxed(&stage.join("hyprdesk.lua"))?;
        let mut s = Settings::from_config(&cfg);
        s.wallpapers = relocate(std::mem::take(&mut s.wallpapers), &map, &wall_dir, false);
        // Services d'IA : ceux en place, jamais ceux de l'archive (un
        // endpoint détourné recevrait la clé rangée sous le même nom).
        let mine = Settings::from_config(&crate::config::lua::load_settings_only(config_path)?);
        report.ai_ignored = s.ai_services.is_some() || !s.ai_use.is_empty();
        s.ai_services = mine.ai_services;
        s.ai_use = mine.ai_use;
        s.save_versioned(config_path, crate::storage::revision())
            .map_err(|e| crate::tr!("settings.lua : {e}", e = e))?;
    }
    if stage
        .join("hyprdesk.lua")
        .metadata()
        .is_ok_and(|m| m.len() > 0)
    {
        std::fs::copy(stage.join("hyprdesk.lua"), config_path).map_err(|e| e.to_string())?;
        crate::history::note(config_path);
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
        let monitors = crate::screens::monitors_path(config_path);
        std::fs::copy(stage.join("monitors.lua"), &monitors).map_err(|e| e.to_string())?;
        crate::history::note(&monitors);
    }
    Ok(report)
}

/// Une archive ne contient que des dossiers et des fichiers ordinaires : un
/// lien (vers un fichier de l'utilisateur) ou un fichier spécial est refusé.
fn only_regular_files(stage: &Path) -> Result<(), String> {
    let mut stack = vec![stage.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = std::fs::read_dir(&d).map_err(|e| e.to_string())?;
        for e in rd.flatten() {
            let t = e.file_type().map_err(|e| e.to_string())?;
            if t.is_dir() {
                stack.push(e.path());
            } else if !t.is_file() {
                return Err(crate::tr!(
                    "archive refusée : {p} n'est pas un fichier ordinaire",
                    p = e.path().strip_prefix(stage).unwrap_or(&e.path()).display()
                ));
            }
        }
    }
    Ok(())
}

fn bundle_name(bundle: &Path) -> String {
    bundle
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("setup")
        .trim_end_matches(".tar")
        .to_string()
}

fn wall_dir_for(name: &str) -> PathBuf {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        });
    data.join("hyprdesk/wallpapers").join(name)
}

/// `hyprdesk.lua` contient autre chose que des commentaires et des blancs.
fn has_lua_code(path: &Path) -> bool {
    let Ok(src) = std::fs::read_to_string(path) else {
        return false;
    };
    src.lines().any(|l| {
        let l = l.trim();
        !l.is_empty() && !l.starts_with("--")
    })
}

/// L'archive déclare des services d'IA (lecture du texte, sans exécution).
fn declares_ai(stage: &Path) -> bool {
    ["hyprdesk/settings.lua", "hyprdesk.lua"].iter().any(|f| {
        std::fs::read_to_string(stage.join(f)).is_ok_and(|s| {
            s.contains("ai_services") || s.contains("ai_use") || s.contains("ai_provider")
        })
    })
}

/// Compile (sans les exécuter) les sources Lua de l'archive.
fn syntax_check_stage(stage: &Path) -> Result<(), String> {
    let mut files = vec![
        stage.join("hyprdesk.lua"),
        stage.join("hyprdesk/settings.lua"),
    ];
    let mut stack = vec![stage.join("hyprdesk/widgets")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(p),
                Ok(t) if t.is_file() && p.extension().is_some_and(|x| x == "lua") => files.push(p),
                _ => {}
            }
        }
    }
    for f in files {
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        let name = f.strip_prefix(stage).unwrap_or(&f).display().to_string();
        crate::config::lua::syntax_check(&src, &name)
            .map_err(|e| crate::tr!("configuration importée invalide : {e}", e = e))?;
    }
    Ok(())
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
            trust_code: true,
            ..Default::default()
        };
        let r = import(&bundle, &b.join("hyprdesk.lua"), opts(true)).unwrap();
        assert!(r.backup.is_none() && !b.join("hyprdesk.lua").exists());
        assert_eq!(r.code.len(), 2, "hyprdesk.lua et le plugin : {:?}", r.code);
        // Sans accord, le code de l'archive n'est pas installé.
        let refused = ImportOpts {
            trust_code: false,
            ..opts(false)
        };
        assert!(import(&bundle, &b.join("hyprdesk.lua"), refused).is_err());
        assert!(!b.join("hyprdesk.lua").exists());
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

    /// Archive piégée : son Lua tente d'écrire un fichier témoin et de lancer
    /// une commande. Ni la vérification, ni le thème, ni l'import accordé
    /// (bac à sable) ne l'exécutent avec les droits de l'utilisateur.
    #[test]
    fn archive_piegee_sans_effet() {
        let base = std::env::temp_dir().join(format!("hyprdesk-piege-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (stage, b) = (base.join("stage"), base.join("b"));
        std::fs::create_dir_all(stage.join("hyprdesk/widgets/espion")).unwrap();
        std::fs::create_dir_all(b.join("hyprdesk")).unwrap();
        std::fs::write(b.join("hyprdesk.lua"), "").unwrap();
        let w1 = base.join("temoin-io");
        let w2 = base.join("temoin-exec");
        let w3 = base.join("temoin-plugin");
        let piege = |w: &Path| {
            format!(
                "local f = io.open({w:?}, 'w') if f then f:write('x') f:close() end
                 os.execute('touch {w}-os') hd.exec('touch {w}-hd') hd.spawn('touch {w}-sp')
",
                w = w.display().to_string()
            )
        };
        std::fs::write(
            stage.join("hyprdesk.lua"),
            format!(
                "{}hd.config({{ theme = {{ name = \"Piège\" }} }})\n",
                piege(&w1)
            ),
        )
        .unwrap();
        std::fs::write(
            stage.join("hyprdesk/settings.lua"),
            format!(
                "{}hd.config({{ workspaces = 2, ai_services = {{ {{ id = \"claude\", kind = \"anthropic\", endpoint = \"https://evil.example\" }} }} }})\n",
                piege(&w2)
            ),
        )
        .unwrap();
        std::fs::write(
            stage.join("hyprdesk/widgets/espion/init.lua"),
            format!(
                "{}return {{ render = function() return '' end }}",
                piege(&w3)
            ),
        )
        .unwrap();
        let manifest = Manifest {
            kind: "setup".into(),
            plugins: vec!["espion".into()],
            ..Manifest::new(Scope::All, false)
        };
        std::fs::write(
            stage.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        let bundle = base.join("piege.tar.gz");
        assert!(
            Command::new("tar")
                .arg("czf")
                .arg(&bundle)
                .arg("-C")
                .arg(&stage)
                .arg(".")
                .status()
                .unwrap()
                .success()
        );
        let cfg = b.join("hyprdesk.lua");
        let witnesses = || {
            [&w1, &w2, &w3].iter().any(|w| {
                ["", "-os", "-hd", "-sp"]
                    .iter()
                    .any(|s| Path::new(&format!("{}{s}", w.display())).exists())
            })
        };
        // Vérifier.
        let r = import(
            &bundle,
            &cfg,
            ImportOpts {
                dry_run: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(r.has_code() && r.ai_ignored);
        assert!(r.to_string().contains("espion"));
        // Thème seul.
        let _ = data_home();
        let _ = import(
            &bundle,
            &cfg,
            ImportOpts {
                scope: Scope::Theme,
                ..Default::default()
            },
        );
        // Tout, sans accord : refusé ; avec accord : réglages lus en bac à sable.
        assert!(import(&bundle, &cfg, ImportOpts::default()).is_err());
        let r = import(
            &bundle,
            &cfg,
            ImportOpts {
                trust_code: true,
                ..Default::default()
            },
        )
        .expect("le bac à sable laisse passer une config qui teste ses appels");
        assert!(r.ai_ignored);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(
            !witnesses(),
            "le Lua de l'archive a agi hors du bac à sable"
        );
        // Les services d'IA de l'archive ne remplacent pas ceux en place.
        let mine = crate::config::lua::load_settings_only(&cfg).unwrap();
        assert!(
            !mine
                .general
                .ai
                .services
                .iter()
                .any(|s| s.endpoint.contains("evil"))
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn lien_dans_l_archive_refuse() {
        let base = std::env::temp_dir().join(format!("hyprdesk-lien-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("s/hyprdesk")).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", base.join("s/hyprdesk.lua")).unwrap();
        assert!(only_regular_files(&base.join("s")).is_err());
        std::fs::remove_file(base.join("s/hyprdesk.lua")).unwrap();
        std::fs::write(base.join("s/hyprdesk.lua"), "").unwrap();
        assert!(only_regular_files(&base.join("s")).is_ok());
        let _ = std::fs::remove_dir_all(&base);
    }
}
