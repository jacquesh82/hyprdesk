//! Application transactionnelle (§9-§10 de la spécification) : résoudre →
//! générer → valider → committer → recharger → rapporter.
//!
//! Les rendus vivent en mémoire (fichiers de quelques Ko : le staging disque
//! n'apporte rien), chaque écriture est atomique, chaque cible garde ses
//! contenus précédents pour un retour arrière ciblé en cas d'échec d'écriture.
//! Le mode `dry-run` n'écrit rien ; le mode `hot` ne touche Hyprland qu'en
//! `hyprctl keyword`, sans persister.

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
        let planned = match targets::plan(&target, resolved, pack_dir, roots) {
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
        let (target_report, rolled) = commit_target(planned, opts.mode);
        if !rolled.is_empty() {
            report
                .errors
                .push(format!("{} : écriture en échec, retour arrière", target.id));
            report.rolled_back.extend(rolled);
        }
        let reload_text =
            if target_report.status != "ok" || opts.mode == Mode::DryRun || !opts.reload {
                "aucun".to_string()
            } else if opts.mode == Mode::Hot && target.id == "hyprland" {
                hot_apply(resolved, pack_dir)
            } else {
                match targets::run_reload(targets::reload_of(&target.id), &resolved.variant) {
                    Ok(Some(text)) => text,
                    Ok(None) => "aucun".to_string(),
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

/// Exécute les opérations d'une cible : écritures atomiques, contenu
/// précédent retenu pour un retour arrière ciblé.
fn commit_target(planned: Vec<Planned>, mode: Mode) -> (TargetReport, Vec<String>) {
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    let mut previous: BTreeMap<PathBuf, Option<String>> = BTreeMap::new();
    let mut rolled_back = Vec::new();
    let mut status = "ok".to_string();
    let mut failed: Option<String> = None;

    'ops: for p in planned {
        let path_text = p.path.display().to_string();
        if mode == Mode::DryRun {
            files.push(FileOutcome {
                path: path_text,
                result: "généré (dry-run)".to_string(),
            });
            continue;
        }
        if !previous.contains_key(&p.path) {
            let existed = p.path.is_file();
            previous.insert(
                p.path.clone(),
                existed.then(|| targets::read_or_empty(&p.path)),
            );
        }
        let old = previous.get(&p.path).cloned().flatten();
        let existed = old.is_some() || p.path.is_file();
        let new_content: String = match p.op {
            Op::Write => p.content.clone(),
            Op::UpsertBlock => targets::upsert_block(&targets::read_or_empty(&p.path), &p.content),
            Op::ImportFirst => targets::include_first(&targets::read_or_empty(&p.path), &p.content),
            Op::AppendSource => {
                // Une ligne source par cible : reconnue par le tronçon
                // « theme/<fragment> » de son chemin, stable partout.
                let needle = needle_for(&p.content);
                targets::append_source_once(&targets::read_or_empty(&p.path), &needle, &p.content)
            }
        };
        if new_content == old.unwrap_or_default() && existed {
            files.push(FileOutcome {
                path: path_text,
                result: "inchangé".to_string(),
            });
            continue;
        }
        if let Err(e) = commit_file(&p.path, &new_content) {
            // Retour arrière de cette cible : restauration de tous les
            // contenus précédents, suppression des fichiers créés.
            failed = Some(format!("{} : {e}", path_text));
            status = "failed".to_string();
            for (path, old) in &previous {
                rolled_back.push(path.display().to_string());
                let _ = match old {
                    Some(content) => super::state::atomic_write(path, content).is_ok(),
                    None => std::fs::remove_file(path).is_ok(),
                };
            }
            let _ = failed;
            break 'ops;
        }
        files.push(FileOutcome {
            path: path_text,
            result: match p.op {
                Op::Write => "écrit".to_string(),
                Op::UpsertBlock => "bloc mis à jour".to_string(),
                Op::ImportFirst => "import en tête".to_string(),
                Op::AppendSource => "source ajoutée".to_string(),
            },
        });
    }
    if let Some(f) = failed {
        warnings.push(f);
    }
    (
        TargetReport {
            id: String::new(),
            status,
            files,
            reload: "aucun".to_string(),
            warnings,
        },
        rolled_back,
    )
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

/// Sauvegarde une fois puis écriture atomique ; la sortie ne doit traverser
/// aucun lien symbolique (§13, cibles génériques surtout).
fn commit_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        let mut ancêtre = Some(parent);
        while let Some(a) = ancêtre {
            if let Ok(meta) = std::fs::symlink_metadata(a)
                && meta.is_symlink()
            {
                return Err(Error::Template(format!(
                    "sortie refusée : traverserait le lien symbolique {}",
                    a.display()
                )));
            }
            ancêtre = a.parent();
        }
    }
    targets::backup_once(path)?;
    super::state::atomic_write(path, content)
}

/// Mode `hot` : `hyprctl keyword` pour chaque ligne du fragment, sans
/// écrire de fichier.
fn hot_apply(resolved: &Resolved, pack_dir: Option<&Path>) -> String {
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
        assert!(roots.config.join("gtk-3.0/hyprdesk-thunar.css").is_file());
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
        // Les règles de l'utilisateur restent : l'import de Thunar devant,
        // le bloc du thème à la fin.
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
    fn import_thunar_en_tête_de_gtk_css() {
        let root = std::env::temp_dir().join(format!("hd-apply-4-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let report = apply(&ardoise(), None, &roots, &opts(Mode::Persist));
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let gtk = std::fs::read_to_string(roots.config.join("gtk-3.0/gtk.css")).unwrap();
        let first_rule = gtk
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with("/*"))
            .unwrap();
        assert_eq!(first_rule, "@import url(\"hyprdesk-thunar.css\");", "{gtk}");
        // Import relatif : le chemin ne porte pas le dossier.
        assert!(!gtk.contains("gtk-3.0"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn échec_décriture_avec_retour_arrière_ciblé() {
        let root = std::env::temp_dir().join(format!("hd-apply-5-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        // gtk-4.0/gtk.css est un DOSSIER : l'écriture échoue, gtk-3.0 doit
        // être remis en ordre. Seule la cible gtk : thunar s'exécute après et
        // recréerait gtk-3.0/gtk.css (import en tête), ce qui masquerait
        // l'annulation.
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
        // hyprctl n'existe pas dans l'environnement d'essai (PATH réduit).
        let saved = std::env::var("PATH").ok();
        // SAFETY : essai unitaire mono-thread sur son propre PATH.
        unsafe { std::env::set_var("PATH", "/rien") };
        let report = apply(&ardoise(), None, &roots, &o);
        match saved {
            Some(p) => unsafe { std::env::set_var("PATH", p) },
            None => unsafe { std::env::remove_var("PATH") },
        }
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
}
