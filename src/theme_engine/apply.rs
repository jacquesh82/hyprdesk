//! Application transactionnelle (§9-§10 de la spécification) : résoudre →
//! générer → valider → committer → recharger → rapporter.
//!
//! Les rendus vivent en mémoire (fichiers de quelques Ko : le staging disque
//! n'apporte rien), chaque écriture est atomique, chaque cible garde ses
//! contenus précédents pour un retour arrière ciblé en cas d'échec d'écriture.
//! Le mode `dry-run` n'écrit rien ; le mode `hot` ne touche Hyprland qu'en
//! `hyprctl keyword`, sans écrire son fragment.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use super::paths::Roots;
use super::resolve::Resolved;
use super::state;
use super::targets::{self, Op, Planned};
use super::{Error, Result};

/// Mode d'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Fichiers générés + rechargement, état persistant.
    #[default]
    Persist,
    /// Hyprland en `hyprctl keyword` seulement (aperçu), autres cibles en
    /// persist.
    Hot,
    /// Résolution et génération seulement, aucune écriture.
    DryRun,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "persist" => Self::Persist,
            "hot" => Self::Hot,
            "dry-run" => Self::DryRun,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Persist => "persist",
            Self::Hot => "hot",
            Self::DryRun => "dry-run",
        }
    }
}

/// Options d'une application.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub mode: Mode,
    /// Cibles visées (vide : toutes).
    pub only: Vec<String>,
    /// Origine tracée dans le rapport et l'Historique (`user`, `lyra`,
    /// `daemon`, `cli`).
    pub origin: String,
    /// Vérifier la présence des logiciels cibles.
    pub check_installed: bool,
    /// Exécuter les rechargements (faux dans les essais).
    pub reload: bool,
    /// Options par cible (`kitty.file`, `thunar.conf`…) : les réglages du
    /// démon passent ici, clés conservées depuis `plugin_options`.
    pub target_opts: BTreeMap<String, String>,
    /// Rechargement par cible : commande configurée (elle passe par `sh -c`,
    /// c'est un réglage du poste, jamais un contenu de pack).
    pub reload_commands: BTreeMap<String, String>,
}

/// Sortie d'un fichier planifié.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FileOutcome {
    pub path: String,
    pub result: String,
}

/// Rapport d'une cible.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TargetReport {
    pub id: String,
    /// `ok`, `skipped`, `failed`.
    pub status: String,
    pub files: Vec<FileOutcome>,
    pub reload: String,
    pub warnings: Vec<String>,
}

/// Rapport d'application complet (§9, annexe D) : écrit dans
/// `last-applied.json`, journalisé, porteur de la révision publiée.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub pack: String,
    pub variant: String,
    pub mode: String,
    pub origin: String,
    pub started: String,
    pub duration_ms: u64,
    pub revision: u64,
    pub targets: Vec<TargetReport>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub rolled_back: Vec<String>,
}

impl Report {
    /// JSON lisible (écrit dans l'état, affiché par `--json`).
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| Error::Manifest("rapport".to_string(), e.to_string()))
    }

    /// Résumé d'une ligne par cible, pour la sortie humaine.
    pub fn summary(&self) -> String {
        let mut out = format!(
            "pack {} ({}), mode {} : {}\n",
            self.pack,
            self.variant,
            self.mode,
            if self.errors.is_empty() {
                "ok"
            } else {
                "échec partiel"
            }
        );
        for t in &self.targets {
            out.push_str(&format!("  {:<10} {:<8}", t.id, t.status));
            for f in &t.files {
                out.push_str(&format!("  {} ({})", f.path, f.result));
            }
            if t.reload != "aucun" {
                out.push_str(&format!("  reload : {}", t.reload));
            }
            out.push('\n');
        }
        for w in &self.warnings {
            out.push_str(&format!("  avertissement : {w}\n"));
        }
        for e in &self.errors {
            out.push_str(&format!("  erreur : {e}\n"));
        }
        out
    }
}

/// Applique une palette résolue à toutes les cibles visées. Renvoie toujours
/// un rapport : les échecs y sont décrits, jamais muets.
pub fn apply(
    resolved: &Resolved,
    pack_dir: Option<&Path>,
    roots: &Roots,
    opts: &Options,
) -> Report {
    let started = Instant::now();
    let mut report = Report {
        pack: resolved.id.clone(),
        variant: resolved.variant.clone(),
        mode: opts.mode.as_str().to_string(),
        origin: opts.origin.clone(),
        started: chrono::Utc::now().to_rfc3339(),
        duration_ms: 0,
        revision: 0,
        targets: Vec::new(),
        warnings: resolved.warnings.clone(),
        errors: Vec::new(),
        rolled_back: Vec::new(),
    };

    for target in targets::all(resolved) {
        if !opts.only.is_empty() && !opts.only.iter().any(|o| o == &target.id) {
            continue;
        }
        if opts.check_installed && !(target.installed)() {
            report.targets.push(TargetReport {
                id: target.id.clone(),
                status: "skipped".to_string(),
                files: Vec::new(),
                reload: "aucun".to_string(),
                warnings: vec!["logiciel non installé".to_string()],
            });
            continue;
        }
        let planned =
            match targets::plan_opts(&target, resolved, pack_dir, roots, &opts.target_opts) {
                Ok(p) => p,
                Err(e) => {
                    report.errors.push(format!("{} : {e}", target.id));
                    report.targets.push(TargetReport {
                        id: target.id.to_string(),
                        status: "failed".to_string(),
                        files: Vec::new(),
                        reload: "aucun".to_string(),
                        warnings: Vec::new(),
                    });
                    continue;
                }
            };
        // Mode `hot` : Hyprland reçoit ses options en direct, sans fichier
        // écrit (aperçu non persistant) ; les autres cibles persistent.
        let hot_hyprland = opts.mode == Mode::Hot && target.id == "hyprland";
        let (target_report, rolled) = if hot_hyprland {
            (
                TargetReport {
                    id: String::new(),
                    status: "ok".to_string(),
                    files: planned
                        .iter()
                        .map(|p| FileOutcome {
                            path: p.path.display().to_string(),
                            result: "non écrit (hot)".to_string(),
                        })
                        .collect(),
                    reload: "aucun".to_string(),
                    warnings: Vec::new(),
                },
                Vec::new(),
            )
        } else {
            let après = planned.clone();
            let r = commit_target(planned, opts.mode, réglages_du_poste());
            if r.0.status == "ok" && opts.mode != Mode::DryRun {
                targets::after_commit(&target.id, &après, roots);
            }
            r
        };
        if !rolled.is_empty() || target_report.status == "failed" {
            report.errors.push(format!(
                "{} : échec, retour arrière{}",
                target.id,
                target_report
                    .warnings
                    .first()
                    .map(|w| format!(" ({w})"))
                    .unwrap_or_default()
            ));
            report.rolled_back.extend(rolled);
        }
        let reload_text =
            if target_report.status != "ok" || opts.mode == Mode::DryRun || !opts.reload {
                "aucun".to_string()
            } else if hot_hyprland {
                hot_apply(resolved, pack_dir)
            } else {
                match reload_for(&target, opts) {
                    Ok(r) => match targets::run_reload(r, resolved) {
                        Ok(Some(text)) => text,
                        Ok(None) => "aucun".to_string(),
                        Err(e) => format!("échec : {e}"),
                    },
                    Err(e) => format!("échec : {e}"),
                }
            };
        report.targets.push(TargetReport {
            id: target.id.to_string(),
            reload: reload_text,
            ..target_report
        });
    }

    // Avertissement d'inclusion hyprland manquante (hors dry-run, si visée).
    if opts.mode != Mode::DryRun
        && (opts.only.is_empty() || opts.only.iter().any(|o| o == "hyprland"))
        && let Some(w) = targets::include_missing_warning(roots, "hyprland.conf")
    {
        report.warnings.push(w);
    }

    // État : palette publiée, rapport, journal (hors dry-run).
    if opts.mode != Mode::DryRun {
        match state::publish_palette(roots, resolved) {
            Ok(rev) => report.revision = rev,
            Err(e) => report.warnings.push(format!("palette.json : {e}")),
        }
        if let Ok(json) = report.to_json()
            && let Err(e) = state::write_last_applied(roots, &json)
        {
            report.warnings.push(format!("last-applied.json : {e}"));
        }
        if let Err(e) = state::journal(
            roots,
            &format!(
                "apply pack={} variante={} origine={}",
                resolved.id, resolved.variant, opts.origin
            ),
        ) {
            report.warnings.push(format!("journal : {e}"));
        }
    }

    report.duration_ms = started.elapsed().as_millis() as u64;
    report
}

/// Rechargement d'une cible, par priorité : commande configurée sur le
/// poste (réglage du démon), rechargement borné déclaré par le pack, puis
/// rechargement intégré.
fn reload_for(target: &targets::Target, opts: &Options) -> Result<targets::Reload> {
    if let Some(cmd) = opts
        .reload_commands
        .get(&target.id)
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
    {
        return Ok(targets::Reload::Command(cmd.to_string()));
    }
    if let Some(decl) = target.reload.as_deref() {
        return targets::reload_generic(decl);
    }
    Ok(targets::reload_of(&target.id))
}

/// Libellé d'une opération planifiée dans le rapport.
fn label_of(p: &Planned) -> String {
    match &p.setting {
        Some((schema, key)) if p.op == Op::Setting => format!("{schema}/{key}"),
        _ => p.path.display().to_string(),
    }
}

/// Exécute les opérations d'une cible : écritures atomiques et réglages
/// posés, état précédent retenu pour un retour arrière complet de la cible
/// (fichiers restaurés ou supprimés, réglages remis à leur valeur).
fn commit_target(
    planned: Vec<Planned>,
    mode: Mode,
    réglages: &dyn Réglages,
) -> (TargetReport, Vec<String>) {
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    // Fichiers : contenu d'avant (None : fichier créé par cette passe).
    let mut fichiers_avant: BTreeMap<PathBuf, Option<String>> = BTreeMap::new();
    // Réglages posés : (schéma, clé, valeur d'avant), dans l'ordre.
    let mut réglages_avant: Vec<(String, String, String)> = Vec::new();
    let mut failed: Option<String> = None;

    for p in planned {
        let label = label_of(&p);
        if mode == Mode::DryRun {
            files.push(FileOutcome {
                path: label,
                result: "généré (dry-run)".to_string(),
            });
            continue;
        }
        if p.op == Op::Setting {
            let Some((schema, key)) = p.setting.clone() else {
                continue;
            };
            let actuelle = match réglages.lire(&schema, &key) {
                Ok(v) => v,
                Err(e) => {
                    failed = Some(format!("{label} : lecture impossible ({e})"));
                    break;
                }
            };
            if normalise_gsettings(&actuelle) == normalise_gsettings(&p.content) {
                files.push(FileOutcome {
                    path: label,
                    result: "inchangé".to_string(),
                });
                continue;
            }
            if let Err(e) = réglages.poser(&schema, &key, &p.content) {
                failed = Some(format!("{label} : {e}"));
                break;
            }
            réglages_avant.push((schema, key, actuelle));
            files.push(FileOutcome {
                path: label,
                result: "réglage posé".to_string(),
            });
            continue;
        }
        let existed = p.path.is_file();
        let old = existed.then(|| targets::read_or_empty(&p.path));
        let current = old.clone().unwrap_or_default();
        let new_content: String = match p.op {
            Op::Write => p.content.clone(),
            Op::UpsertBlock => targets::upsert_block(&current, &p.content),
            Op::AppendSource => {
                // Une ligne source par cible : reconnue par le tronçon
                // « theme/<fragment> » de son chemin, stable partout.
                let needle = needle_for(&p.content);
                targets::append_source_once(&current, &needle, &p.content)
            }
            Op::Setting => unreachable!("réglage traité plus haut"),
        };
        if existed && new_content == current {
            files.push(FileOutcome {
                path: label,
                result: "inchangé".to_string(),
            });
            continue;
        }
        if let Err(e) = commit_file(&p.path, &new_content) {
            // Écriture atomique : le fichier en échec est resté tel quel.
            failed = Some(format!("{label} : {e}"));
            break;
        }
        // Seuls les fichiers réellement écrits sont à remettre en cas
        // d'échec plus loin ; le premier état connu fait foi.
        fichiers_avant.entry(p.path.clone()).or_insert(old);
        files.push(FileOutcome {
            path: label,
            result: match p.op {
                Op::Write => "écrit",
                Op::UpsertBlock => "bloc mis à jour",
                Op::AppendSource => "source ajoutée",
                Op::Setting => "réglage posé",
            }
            .to_string(),
        });
    }

    let mut rolled_back = Vec::new();
    let status = if let Some(f) = failed {
        // Retour arrière de toute la cible : fichiers remis (ou supprimés
        // s'ils ont été créés), réglages remis à leur valeur, du dernier au
        // premier.
        for (path, old) in &fichiers_avant {
            let restauré = match old {
                Some(content) => {
                    targets::read_or_empty(path) == *content
                        || super::state::atomic_write(path, content).is_ok()
                }
                None => !path.exists() || std::fs::remove_file(path).is_ok(),
            };
            if restauré {
                rolled_back.push(path.display().to_string());
            } else {
                warnings.push(format!("retour arrière impossible : {}", path.display()));
            }
        }
        for (schema, key, avant) in réglages_avant.iter().rev() {
            match réglages.poser(schema, key, avant) {
                Ok(()) => rolled_back.push(format!("{schema}/{key}")),
                Err(e) => {
                    warnings.push(format!("retour arrière impossible : {schema}/{key} ({e})"))
                }
            }
        }
        warnings.insert(0, f);
        "failed"
    } else {
        "ok"
    };
    (
        TargetReport {
            id: String::new(),
            status: status.to_string(),
            files,
            reload: "aucun".to_string(),
            warnings,
        },
        rolled_back,
    )
}

/// Valeur gsettings comparable : `'Adwaita'` (sortie de `get`) et `Adwaita`
/// (valeur à poser) sont la même.
fn normalise_gsettings(v: &str) -> String {
    let v = v.trim();
    v.strip_prefix('\'')
        .and_then(|x| x.strip_suffix('\''))
        .unwrap_or(v)
        .to_string()
}

/// Aiguille d'une ligne `source =` : les deux derniers tronçons du chemin du
/// fragment (`theme/hyprlock.conf`), stables quel que soit le dossier HOME.
fn needle_for(line: &str) -> String {
    let src = line.trim().strip_prefix("source = ").unwrap_or(line.trim());
    let p = Path::new(src);
    let name = p.file_name().unwrap_or_default().to_string_lossy();
    match p.parent().and_then(|d| d.file_name()) {
        Some(dir) => format!("{}/{name}", dir.to_string_lossy()),
        None => name.to_string(),
    }
}

/// Magasin de réglages de bureau : gsettings en vrai, un double en essai
/// (les essais ne touchent jamais au dconf de la session).
trait Réglages {
    fn lire(&self, schema: &str, clé: &str) -> std::result::Result<String, String>;
    fn poser(&self, schema: &str, clé: &str, valeur: &str) -> std::result::Result<(), String>;
}

/// Réglages par l'outil `gsettings`.
#[cfg_attr(test, allow(dead_code))]
struct Gsettings;

/// Réglages de bureau utilisés par [`apply`] : gsettings ; dans les essais,
/// un magasin en mémoire propre au fil — un essai ne pose jamais un réglage
/// de la session qui le lance (le thème GTK de la session avait été changé
/// par un essai).
fn réglages_du_poste() -> &'static dyn Réglages {
    #[cfg(test)]
    {
        &EssaiRéglages
    }
    #[cfg(not(test))]
    {
        &Gsettings
    }
}

#[cfg(test)]
thread_local! {
    static ESSAI_RÉGLAGES: std::cell::RefCell<BTreeMap<String, String>> =
        const { std::cell::RefCell::new(BTreeMap::new()) };
}

/// Magasin de réglages des essais (voir [`réglages_du_poste`]).
#[cfg(test)]
struct EssaiRéglages;

#[cfg(test)]
impl Réglages for EssaiRéglages {
    fn lire(&self, schema: &str, clé: &str) -> std::result::Result<String, String> {
        Ok(ESSAI_RÉGLAGES.with(|m| {
            m.borrow()
                .get(&format!("{schema} {clé}"))
                .cloned()
                .unwrap_or_default()
        }))
    }
    fn poser(&self, schema: &str, clé: &str, valeur: &str) -> std::result::Result<(), String> {
        ESSAI_RÉGLAGES.with(|m| {
            m.borrow_mut()
                .insert(format!("{schema} {clé}"), valeur.to_string())
        });
        Ok(())
    }
}

impl Réglages for Gsettings {
    fn lire(&self, schema: &str, clé: &str) -> std::result::Result<String, String> {
        valeur_gsettings(schema, clé)
    }
    fn poser(&self, schema: &str, clé: &str, valeur: &str) -> std::result::Result<(), String> {
        poser_gsettings(schema, clé, valeur)
    }
}

/// Valeur actuelle d'un réglage gsettings.
#[cfg_attr(test, allow(dead_code))]
fn valeur_gsettings(schema: &str, clé: &str) -> std::result::Result<String, String> {
    match std::process::Command::new("gsettings")
        .args(["get", schema, clé])
        .output()
    {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).trim().to_string()),
        Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Pose un réglage gsettings.
#[cfg_attr(test, allow(dead_code))]
fn poser_gsettings(schema: &str, clé: &str, valeur: &str) -> std::result::Result<(), String> {
    match std::process::Command::new("gsettings")
        .args(["set", schema, clé, valeur])
        .output()
    {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Sauvegarde une fois puis écriture atomique. Le périmètre des sorties de
/// pack est vérifié à la planification sur le chemin réel
/// (`targets::safe_output_path`) ; un lien final (dotfiles) est suivi, pas
/// remplacé.
fn commit_file(path: &Path, content: &str) -> Result<()> {
    targets::backup_once(&super::state::real_target(path))?;
    super::state::atomic_write(path, content)
}

/// Mode `hot` : `hyprctl keyword` pour chaque ligne du fragment, sans
/// écrire de fichier.
fn hot_apply(resolved: &Resolved, pack_dir: Option<&Path>) -> String {
    // Les tests ne touchent pas à la session Hyprland qui les lance.
    if cfg!(test) {
        return "aucun".to_string();
    }
    match targets::hot_keywords(resolved, pack_dir) {
        Ok(keywords) => {
            let mut ok = 0;
            let mut errs = Vec::new();
            for (key, value) in keywords {
                match std::process::Command::new("hyprctl")
                    .args(["keyword", &key, &value])
                    .output()
                {
                    Ok(o) if o.status.success() => ok += 1,
                    Ok(o) => errs.push(format!(
                        "{key} : {}",
                        String::from_utf8_lossy(&o.stderr).trim()
                    )),
                    Err(e) => errs.push(format!("{key} : {e}")),
                }
            }
            if errs.is_empty() {
                format!("hot : {ok} options")
            } else {
                format!("hot : {ok} options, échecs : {}", errs.join(" ; "))
            }
        }
        Err(e) => format!("échec : {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::resolve;
    use super::*;

    fn ardoise() -> Resolved {
        resolve::from_theme(&crate::config::Theme::default())
    }

    fn opts(mode: Mode) -> Options {
        Options {
            mode,
            only: Vec::new(),
            origin: "cli".to_string(),
            check_installed: false,
            reload: false,
            target_opts: BTreeMap::new(),
            reload_commands: BTreeMap::new(),
        }
    }

    #[test]
    fn application_persist_puis_inchangée() {
        let root = std::env::temp_dir().join(format!("hd-apply-1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let r = ardoise();
        let first = apply(&r, None, &roots, &opts(Mode::Persist));
        assert!(first.errors.is_empty(), "{:?}", first.errors);
        assert_eq!(first.revision, 1);
        let kitty = first.targets.iter().find(|t| t.id == "kitty").unwrap();
        assert_eq!(kitty.status, "ok");
        assert!(
            kitty
                .files
                .iter()
                .any(|f| f.result == "écrit" && f.path.contains("hyprdesk-theme.conf")),
            "{:?}",
            kitty.files
        );
        // Deuxième passe : tout inchangé, révision stable.
        let second = apply(&r, None, &roots, &opts(Mode::Persist));
        assert_eq!(second.revision, 1);
        for t in &second.targets {
            assert!(
                t.files.iter().all(|f| f.result == "inchangé"),
                "{} : {:?}",
                t.id,
                t.files
            );
        }
        // Les fichiers existent aux bons endroits.
        assert!(roots.config.join("kitty/hyprdesk-theme.conf").is_file());
        assert!(roots.config.join("gtk-3.0/gtk.css").is_file());
        assert!(roots.config.join("gtk-4.0/gtk.css").is_file());
        // Cible thunar retirée (MLG-251) : plus de feuille dédiée.
        assert!(!roots.config.join("gtk-3.0/hyprdesk-thunar.css").exists());
        assert!(roots.hypr_theme_dir().join("hyprland.conf").is_file());
        assert!(roots.hypr_theme_dir().join("hyprlock.conf").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dry_run_n_écrit_rien() {
        let root = std::env::temp_dir().join(format!("hd-apply-2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let report = apply(&ardoise(), None, &roots, &opts(Mode::DryRun));
        assert!(report.errors.is_empty());
        assert_eq!(report.revision, 0, "pas de publication en dry-run");
        assert!(!roots.config.join("kitty").exists());
        assert!(!roots.state_dir().join("palette.json").exists());
        assert!(
            report
                .targets
                .iter()
                .all(|t| t.files.iter().all(|f| f.result.contains("dry-run")))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn contenu_utilisateur_préservé_avec_sauvegarde() {
        let root = std::env::temp_dir().join(format!("hd-apply-3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        std::fs::create_dir_all(roots.config.join("gtk-3.0")).unwrap();
        std::fs::write(roots.config.join("gtk-3.0/gtk.css"), "/* mes règles */\n").unwrap();
        let report = apply(&ardoise(), None, &roots, &opts(Mode::Persist));
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let gtk = std::fs::read_to_string(roots.config.join("gtk-3.0/gtk.css")).unwrap();
        // Les règles de l'utilisateur restent devant, le bloc du thème à la
        // fin.
        assert!(gtk.contains("/* mes règles */"), "{gtk}");
        assert!(gtk.contains(targets::BLOCK_BEGIN));
        // Sauvegarde une fois, au premier passage.
        assert!(targets::backup_path(&roots.config.join("gtk-3.0/gtk.css")).is_file());
        std::fs::write(
            targets::backup_path(&roots.config.join("gtk-3.0/gtk.css")),
            "/* première version trouvée */",
        )
        .unwrap();
        let mut second_opts = opts(Mode::Persist);
        second_opts.only = vec!["gtk".to_string()];
        apply(&ardoise(), None, &roots, &second_opts);
        assert_eq!(
            std::fs::read_to_string(targets::backup_path(&roots.config.join("gtk-3.0/gtk.css")))
                .unwrap(),
            "/* première version trouvée */"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn échec_décriture_avec_retour_arrière_ciblé() {
        let root = std::env::temp_dir().join(format!("hd-apply-5-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        // gtk-4.0/gtk.css est un DOSSIER : l'écriture échoue, gtk-3.0 doit
        // être remis en ordre. Seule la cible gtk, pour que rien d'autre ne
        // touche gtk-3.0/gtk.css.
        std::fs::create_dir_all(roots.config.join("gtk-4.0/gtk.css")).unwrap();
        let mut o = opts(Mode::Persist);
        o.only = vec!["gtk".to_string()];
        let report = apply(&ardoise(), None, &roots, &o);
        assert!(
            report
                .targets
                .iter()
                .any(|t| t.id == "gtk" && t.status == "failed"),
            "{:?}",
            report.targets
        );
        assert!(!report.rolled_back.is_empty());
        assert!(
            !roots.config.join("gtk-3.0/gtk.css").exists(),
            "fichier créé puis annulé : {}",
            roots.config.join("gtk-3.0/gtk.css").display()
        );
        // Le dossier de test n'a pas été touché par l'annulation.
        assert!(roots.config.join("gtk-4.0/gtk.css").is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cibles_filtrées() {
        let root = std::env::temp_dir().join(format!("hd-apply-6-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let mut o = opts(Mode::Persist);
        o.only = vec!["kitty".to_string()];
        let report = apply(&ardoise(), None, &roots, &o);
        assert_eq!(report.targets.len(), 1);
        assert_eq!(report.targets[0].id, "kitty");
        assert!(!roots.config.join("gtk-3.0").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cible_absente_marquée_skipped() {
        let root = std::env::temp_dir().join(format!("hd-apply-7-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let mut o = opts(Mode::Persist);
        o.only = vec!["hyprland".to_string()];
        o.check_installed = true;
        // hyprctl n'existe pas dans le PATH de ce fil d'essai (le PATH du
        // processus n'est pas touché : les autres essais lancent tar, sh…).
        targets::TEST_PATH.with(|p| *p.borrow_mut() = Some("/rien".to_string()));
        let report = apply(&ardoise(), None, &roots, &o);
        targets::TEST_PATH.with(|p| *p.borrow_mut() = None);
        assert_eq!(report.targets[0].status, "skipped");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn palette_publiée_relue_par_la_création_cliente() {
        // Fin de chaîne : ce que le moteur publie, la crate cliente
        // `hyprdesk-theme-client` le lit sans dépendre du démon.
        let root = std::env::temp_dir().join(format!("hd-apply-9-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let report = apply(&ardoise(), None, &roots, &opts(Mode::Persist));
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let client =
            hyprdesk_theme_client::ThemeClient::from_path(roots.state_dir().join("palette.json"));
        let pal = client.palette().unwrap();
        assert_eq!(pal.pack, "regle");
        assert_eq!(pal.revision, report.revision);
        let accent = pal.slot("accent").unwrap();
        assert_eq!(
            (accent.r, accent.g, accent.b),
            (
                crate::config::Theme::default().accent.0,
                crate::config::Theme::default().accent.1,
                crate::config::Theme::default().accent.2
            )
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rapport_json_et_résumé() {
        let root = std::env::temp_dir().join(format!("hd-apply-8-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let report = apply(&ardoise(), None, &roots, &opts(Mode::Persist));
        let json = report.to_json().unwrap();
        assert!(json.contains("\"pack\": \"regle\""), "{json}");
        assert!(report.summary().contains("kitty"));
        let _ = std::fs::remove_dir_all(&root);
    }

    use std::cell::RefCell;

    /// Double de gsettings : valeurs en mémoire, une clé qui refuse.
    struct Faux {
        valeurs: RefCell<BTreeMap<String, String>>,
        refuse: &'static str,
    }

    impl Réglages for Faux {
        fn lire(&self, s: &str, k: &str) -> std::result::Result<String, String> {
            Ok(self
                .valeurs
                .borrow()
                .get(&format!("{s}/{k}"))
                .cloned()
                .unwrap_or_default())
        }
        fn poser(&self, s: &str, k: &str, v: &str) -> std::result::Result<(), String> {
            if k == self.refuse {
                return Err("refusé".to_string());
            }
            self.valeurs
                .borrow_mut()
                .insert(format!("{s}/{k}"), v.to_string());
            Ok(())
        }
    }

    fn réglage(clé: &str, valeur: &str) -> Planned {
        Planned {
            path: PathBuf::new(),
            content: valeur.to_string(),
            op: Op::Setting,
            setting: Some(("org.gnome.desktop.interface".to_string(), clé.to_string())),
        }
    }

    #[test]
    fn retour_arrière_des_réglages_sur_leur_propre_clé() {
        // MLG-225 : chaque ancienne valeur était reposée sur la clé en échec.
        let faux = Faux {
            valeurs: RefCell::new(BTreeMap::from([
                (
                    "org.gnome.desktop.interface/cursor-theme".to_string(),
                    "'Adwaita'".to_string(),
                ),
                (
                    "org.gnome.desktop.interface/cursor-size".to_string(),
                    "24".to_string(),
                ),
            ])),
            refuse: "icon-theme",
        };
        let (rapport, annulés) = commit_target(
            vec![
                réglage("cursor-theme", "Bibata"),
                réglage("cursor-size", "32"),
                réglage("icon-theme", "Papirus"),
            ],
            Mode::Persist,
            &faux,
        );
        assert_eq!(rapport.status, "failed");
        let v = faux.valeurs.borrow();
        assert_eq!(v["org.gnome.desktop.interface/cursor-theme"], "'Adwaita'");
        assert_eq!(v["org.gnome.desktop.interface/cursor-size"], "24");
        assert!(!v.contains_key("org.gnome.desktop.interface/icon-theme"));
        assert_eq!(annulés.len(), 2, "{annulés:?}");
        // Rapport lisible : schéma/clé, pas un chemin vide.
        assert!(
            rapport
                .files
                .iter()
                .all(|f| f.path.contains("org.gnome.desktop.interface/"))
        );
    }

    #[test]
    fn réglage_déjà_en_place_inchangé() {
        let faux = Faux {
            valeurs: RefCell::new(BTreeMap::from([(
                "org.gnome.desktop.interface/icon-theme".to_string(),
                "'Papirus'".to_string(),
            )])),
            refuse: "",
        };
        let (rapport, _) =
            commit_target(vec![réglage("icon-theme", "Papirus")], Mode::Persist, &faux);
        assert_eq!(rapport.files[0].result, "inchangé");
    }

    #[test]
    fn échec_de_fichier_remet_aussi_les_réglages() {
        let root = std::env::temp_dir().join(format!("hd-apply-10-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("dossier.conf")).unwrap();
        let faux = Faux {
            valeurs: RefCell::new(BTreeMap::new()),
            refuse: "",
        };
        let (rapport, _) = commit_target(
            vec![
                réglage("cursor-theme", "Bibata"),
                Planned {
                    path: root.join("dossier.conf"),
                    content: "x".to_string(),
                    op: Op::Write,
                    setting: None,
                },
            ],
            Mode::Persist,
            &faux,
        );
        assert_eq!(rapport.status, "failed");
        assert_eq!(
            faux.valeurs.borrow()["org.gnome.desktop.interface/cursor-theme"],
            ""
        );
        assert!(root.join("dossier.conf").is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn options_du_démon_honorées() {
        // MLG-223 : kitty.file et kitty.conf étaient ignorés par apply.
        let root = std::env::temp_dir().join(format!("hd-apply-11-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let mut o = opts(Mode::Persist);
        o.only = vec!["kitty".to_string()];
        o.target_opts.insert(
            "kitty.file".to_string(),
            root.join("perso/couleurs.conf").display().to_string(),
        );
        o.target_opts.insert(
            "kitty.conf".to_string(),
            root.join("perso/kitty.conf").display().to_string(),
        );
        let report = apply(&ardoise(), None, &roots, &o);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(root.join("perso/couleurs.conf").is_file());
        assert!(!roots.config.join("kitty/hyprdesk-theme.conf").exists());
        let conf = std::fs::read_to_string(root.join("perso/kitty.conf")).unwrap();
        assert!(conf.contains("include couleurs.conf"), "{conf}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rechargement_du_poste_puis_du_pack_puis_intégré() {
        // MLG-223 : reload_commands et le rechargement déclaré d'un pack
        // n'étaient jamais exécutés.
        let mut o = opts(Mode::Persist);
        o.reload_commands
            .insert("kitty".to_string(), "kitty @ load-config".to_string());
        let kitty = targets::find("kitty").unwrap();
        assert_eq!(
            reload_for(&kitty, &o).unwrap(),
            targets::Reload::Command("kitty @ load-config".to_string())
        );
        let mut générique = kitty.clone();
        générique.id = "waybar".to_string();
        générique.reload = Some("signal:waybar:USR2".to_string());
        assert_eq!(
            reload_for(&générique, &o).unwrap(),
            targets::Reload::Signal("waybar".to_string(), "USR2".to_string())
        );
        assert_eq!(
            reload_for(&targets::find("gtk").unwrap(), &o).unwrap(),
            targets::Reload::GsettingsScheme
        );
    }

    #[test]
    fn hot_n_écrit_pas_le_fragment_hyprland() {
        // MLG-224 : le mode hot persistait hyprland.conf.
        let root = std::env::temp_dir().join(format!("hd-apply-12-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let mut o = opts(Mode::Hot);
        o.only = vec!["hyprland".to_string(), "kitty".to_string()];
        let report = apply(&ardoise(), None, &roots, &o);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(!roots.hypr_theme_dir().join("hyprland.conf").exists());
        // Les autres cibles persistent.
        assert!(roots.config.join("kitty/hyprdesk-theme.conf").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
