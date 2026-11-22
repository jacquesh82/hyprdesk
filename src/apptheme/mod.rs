//! Thème des applications (paquet « Thème des applications ») : le thème de
//! hyprdesk appliqué à d'autres logiciels, en architecture « plug-n-play »
//! comme les widgets :
//!
//! - chaque application est un module (une CIBLE) qui expose
//!   `pub const TARGET: Target` : [`kitty`], [`gtk`], [`qt`] ;
//! - le registre [`TARGETS`] les liste ; la page de réglages, la config Lua et
//!   `hyprdesk config` le parcourent, sans rien connaître de chaque cible ;
//! - les outils communs (dossier de config, `PATH`, écriture idempotente,
//!   inclusion unique avec copie de sauvegarde, couleurs) vivent ici.
//!
//! Contrat d'une cible : écrire son fichier de couleurs, l'inclure une fois
//! dans la config du logiciel (copie de sauvegarde avant) et lui faire relire
//! sa config. Couper une cible vide son fichier sans le supprimer (l'inclusion
//! reste valide) : le thème d'avant reprend la main.
//!
//! Chaque cible déclare ses options (fichiers, rechargement) : le paquet est
//! un plugin configurable, réglé dans Réglages → Thème des applications ou
//! Réglages → Plugins → Configurer ; les valeurs vivent dans
//! `plugin_options["apptheme"]` (clés `<cible>.<option>`).
//!
//! Ajouter une application : un fichier `<id>.rs` sur le modèle de
//! [`kitty`], un `pub mod <id>;` ci-dessous, une ligne dans [`TARGETS`], les
//! textes dans `locales/en.toml`, et l'outil de rechargement éventuel dans
//! `doctor::DEPS`.
//!
//! Cible retirée : `thunar` (MLG-251), que le thème GTK de base de la cible
//! [`gtk`] couvre déjà. Son identifiant reste admis à la lecture de la config
//! ([`RETIRED`]) et ce qu'elle avait posé est retiré au passage suivant.

pub mod gtk;
pub mod kitty;
pub mod qt;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::config::Theme;

/// Identifiant du paquet, clé de ses options dans `plugin_options`.
pub const PLUGIN: &str = "apptheme";

/// Réglage d'une cible : clé (`file`, `conf`…), libellé, aide, valeur par défaut.
pub struct Opt {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub default: fn() -> String,
}

/// Valeurs réglées : `<cible>.<option>` → texte (absente : valeur par défaut).
pub type Opts = std::collections::BTreeMap<String, String>;

/// Options réglées du paquet, d'après la config.
pub fn options(g: &crate::config::General) -> Opts {
    g.plugin_options
        .get(PLUGIN)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.display())).collect())
        .unwrap_or_default()
}

/// Valeur d'une option de `target` : réglée, sinon par défaut ; `~/` développé.
pub fn opt(o: &Opts, target: &str, opt: &Opt) -> String {
    let v = o
        .get(&format!("{target}.{}", opt.key))
        .cloned()
        .unwrap_or_else(opt.default);
    expand_home(v.trim())
}

/// `~/…` → dossier personnel.
pub fn expand_home(p: &str) -> String {
    match p.strip_prefix("~/") {
        Some(rest) => format!("{}/{rest}", std::env::var("HOME").unwrap_or_default()),
        None => p.to_string(),
    }
}

/// Application qui peut prendre le thème de hyprdesk.
pub struct Target {
    /// Clé enregistrée dans `general.app_themes` : ne plus la changer.
    pub id: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    /// Le logiciel est présent.
    pub installed: fn() -> bool,
    /// Ses réglages (fichiers, rechargement).
    pub options: &'static [Opt],
    /// Fichier de couleurs écrit par hyprdesk.
    pub file: fn(&Opts) -> PathBuf,
    /// Fichiers en jeu, pour un diagnostic (couleurs, configuration…).
    pub files: fn(&Opts) -> Vec<PathBuf>,
    /// Écrit et applique le thème ; rend un compte rendu.
    pub apply: fn(&Theme, &Opts) -> Result<String, String>,
    /// Retire le thème (fichier vidé, logiciel rechargé).
    pub clear: fn(&Opts) -> Result<(), String>,
}

/// Options des cibles pour le moteur : `cible.clé` → valeur effective, et
/// commandes de rechargement par cible (`:` quand elle est vidée).
pub fn engine_options(o: &Opts) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let (mut target_opts, mut reload_commands) = (BTreeMap::new(), BTreeMap::new());
    for t in TARGETS {
        for op in t.options {
            let v = opt(o, t.id, op);
            if op.key == "reload" {
                let cmd = if v.is_empty() { ":".to_string() } else { v };
                reload_commands.insert(t.id.to_string(), cmd);
            } else if !v.is_empty() {
                target_opts.insert(format!("{}.{}", t.id, op.key), v);
            }
        }
    }
    (target_opts, reload_commands)
}

/// Registre des applications qui peuvent prendre le thème.
pub const TARGETS: &[Target] = &[kitty::TARGET, gtk::TARGET, qt::TARGET];

/// Cibles retirées : encore admises dans `app_themes` (config d'avant), sans
/// effet.
pub const RETIRED: &[&str] = &crate::theme_engine::targets::RETIRED_IDS;

pub fn target(id: &str) -> Option<&'static Target> {
    TARGETS.iter().find(|t| t.id == id)
}

/// Applique le thème aux cibles choisies et le retire des autres (celles où
/// hyprdesk l'avait posé). Rend un compte rendu par cible.
pub fn sync(
    enabled: &BTreeSet<String>,
    theme: &Theme,
    opts: &Opts,
) -> Vec<(String, Result<String, String>)> {
    // Délégation au moteur de thème (docs/THEME-ENGINE.md §16) : une seule
    // application transactionnelle pour toutes les cibles activées, palette
    // implicite depuis le thème, options conservées (kitty.file…). Les
    // cibles désactivées reprennent leur clear. Le fragment `hyprland` du
    // moteur reste hors du démon : les couleurs de fenêtres sont portées
    // par appearance.lua.
    let mut out = Vec::new();
    // Ce que la cible thunar avait posé (MLG-251) : sans effet une fois fait.
    let roots = crate::theme_engine::paths::Roots::from_env();
    let anciens: BTreeMap<String, String> = opts
        .iter()
        .filter(|(k, v)| k.starts_with("thunar.") && !v.trim().is_empty())
        .map(|(k, v)| (k.clone(), expand_home(v.trim())))
        .collect();
    crate::theme_engine::targets::retire_thunar(&roots, &anciens);
    for t in TARGETS {
        if !enabled.contains(t.id)
            && let Err(e) = (t.clear)(opts)
        {
            out.push((t.id.to_string(), Err(e)));
        }
    }
    let mut activées = Vec::new();
    for t in TARGETS {
        if enabled.contains(t.id) {
            if (t.installed)() {
                activées.push(t.id.to_string());
            } else {
                out.push((
                    t.id.to_string(),
                    Ok(crate::tr!("{a} : non installé", a = t.label)),
                ));
            }
        }
    }
    if activées.is_empty() {
        return out;
    }
    let (resolved, pack_dir) = crate::theme_engine::bridge::resolution_for(&roots, theme);
    // Options effectives de chaque cible, `~/` développé et défauts compris
    // (le moteur ne connaît pas ces défauts : sans eux, l'inclusion dans
    // kitty.conf n'était plus posée). Une commande de rechargement vidée
    // veut dire « aucune » ; elle passe par `sh -c` côté moteur, c'est un
    // réglage du poste, jamais un contenu de pack.
    let (target_opts, reload_commands) = engine_options(opts);
    let options = crate::theme_engine::apply::Options {
        only: activées.clone(),
        origin: "daemon".to_string(),
        check_installed: false,
        reload: true,
        target_opts,
        reload_commands,
        ..Default::default()
    };
    let report =
        crate::theme_engine::apply::apply(&resolved, pack_dir.as_deref(), &roots, &options);
    for id in &activées {
        match report.targets.iter().find(|tr| tr.id == *id) {
            Some(tr) if tr.status == "ok" => {
                out.push((
                    id.clone(),
                    Ok(crate::tr!("thème « {n} » appliqué", n = theme.name)),
                ));
            }
            Some(tr) if tr.status == "failed" => out.push((
                id.clone(),
                Err(if report.errors.is_empty() {
                    tr.warnings.join(" ; ")
                } else {
                    report.errors.join(" ; ")
                }),
            )),
            Some(_) => out.push((id.clone(), Ok(crate::tr!("aucun changement").to_string()))),
            None => out.push((id.clone(), Ok(crate::tr!("aucun changement").to_string()))),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Outils communs aux cibles

/// `$XDG_CONFIG_HOME/<app>`, ou `~/.config/<app>`.
pub(crate) fn config_dir(app: &str) -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"))
        .join(app)
}

/// Lance la commande de rechargement d'une cible (vide : rien), sans attendre
/// son résultat.
pub(crate) fn run_reload(cmd: &str) {
    if cmd.trim().is_empty() {
        return;
    }
    let _ = std::process::Command::new("sh").args(["-c", cmd]).status();
}

/// Invite pour LYRA quand une cible échoue : l'erreur, les fichiers en jeu,
/// et ce qu'on attend d'elle.
pub fn lyra_prompt(t: &Target, opts: &Opts, error: &str) -> String {
    let files: Vec<String> = (t.files)(opts)
        .iter()
        .map(|f| {
            let link = std::fs::read_link(f)
                .map(|to| crate::tr!(" (lien vers {t})", t = to.display()))
                .unwrap_or_default();
            format!("- {}{link}", f.display())
        })
        .collect();
    crate::tr!(
        "Dans Réglages → Thème des applications, hyprdesk n'arrive pas à appliquer son thème à {app} :\n{error}\n\nFichiers en jeu :\n{files}\n\nRegarde ces fichiers (liens symboliques et leurs cibles compris), explique-moi la cause, puis propose la correction la plus simple : rétablir la cible d'un lien, remplacer le lien par un fichier, ou indiquer d'autres chemins dans Réglages → Thème des applications → Réglages de {app}. Demande-moi avant de modifier quoi que ce soit.",
        app = t.label,
        error = error,
        files = files.join("\n")
    )
}

/// L'exécutable `bin` est dans le `PATH`.
pub(crate) fn in_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
}

/// Erreur d'un fichier : son chemin, puis la cause.
fn fs_err(path: &Path, e: std::io::Error) -> String {
    format!("{} : {e}", path.display())
}

/// Lien symbolique dont la cible n'existe pas (dotfiles copiés sans leur
/// dépôt…) : écrire à travers échouerait sur « os error 2 ».
fn dangling(path: &Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.file_type().is_symlink() || path.exists() {
        return None;
    }
    let target = std::fs::read_link(path).unwrap_or_default();
    Some(crate::tr!(
        "{p} est un lien vers {t}, qui n'existe pas : rétablissez la cible ou retirez le lien",
        p = path.display(),
        t = target.display()
    ))
}

/// Écrit `content` dans `file` s'il diffère (dossier créé au besoin). Rend
/// vrai si le fichier a changé.
pub(crate) fn write_if_changed(file: &Path, content: &str) -> Result<bool, String> {
    if std::fs::read_to_string(file).ok().as_deref() == Some(content) {
        return Ok(false);
    }
    if let Some(dir) = file.parent() {
        if let Some(e) = dangling(dir) {
            return Err(e);
        }
        std::fs::create_dir_all(dir).map_err(|e| fs_err(dir, e))?;
    }
    if let Some(e) = dangling(file) {
        return Err(e);
    }
    std::fs::write(file, content).map_err(|e| fs_err(file, e))?;
    Ok(true)
}

/// Ajoute `line` à la fin de `conf`, une seule fois : rien si l'une des
/// lignes de `already` (ou `line`) y figure déjà. Avant la première
/// modification d'un fichier existant, copie de sauvegarde
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registre() {
        assert!(target("kitty").is_some() && target("vim").is_none());
        let mut ids: Vec<&str> = TARGETS.iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), TARGETS.len(), "identifiants de cible en double");
        // La migration moteur apporte GTK et Qt au registre.
        assert!(target("gtk").is_some() && target("qt").is_some());
    }

    #[test]
    fn lien_casse_signale() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-lien-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("kitty.conf");
        std::os::unix::fs::symlink(dir.join("absent/dotfiles/kitty.conf"), &conf).unwrap();
        let e = write_if_changed(&conf, "x").unwrap_err();
        assert!(e.contains("absent/dotfiles"), "{e}");
        // Dossier manquant : créé, sans erreur.
        assert!(write_if_changed(&dir.join("sous/dossier/t.conf"), "x").unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// La délégation moteur parcourt toutes les cibles activées et publie la
    /// palette (état dans les racines d'essai via l'environnement) :
    /// `cargo test -- --ignored --nocapture essai_sync_moteur`.
    #[test]
    #[ignore]
    fn essai_sync_moteur() {
        let tmp = std::env::temp_dir().join(format!("hyprdesk-sync-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("kitty")).unwrap();
        std::fs::create_dir_all(tmp.join("state")).unwrap();
        std::fs::create_dir_all(tmp.join("cache")).unwrap();
        // SAFETY : essai lancé seul, à la main.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &tmp);
            std::env::set_var("XDG_STATE_HOME", tmp.join("state"));
            std::env::set_var("XDG_CACHE_HOME", tmp.join("cache"));
        }
        let mut enabled = BTreeSet::new();
        enabled.insert("kitty".to_string());
        let mut opts = Opts::new();
        opts.insert("kitty.reload".into(), String::new());
        let résultats = sync(&enabled, &Theme::default(), &opts);
        for (id, r) in &résultats {
            println!("{id} : {r:?}");
        }
        assert!(
            résultats.iter().all(|(id, r)| id == "kitty" && r.is_ok()),
            "{résultats:?}"
        );
        let couleurs = std::fs::read_to_string(tmp.join("kitty/hyprdesk-theme.conf")).unwrap();
        assert!(couleurs.contains("foreground"), "{couleurs}");
        // SAFETY : fin de l'essai.
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::remove_var("XDG_STATE_HOME");
            std::env::remove_var("XDG_CACHE_HOME");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn options_effectives_pour_le_moteur() {
        // Rien de réglé : défauts développés, inclusion dans kitty.conf.
        let (opts, reloads) = engine_options(&Opts::new());
        assert!(opts["kitty.conf"].ends_with("kitty/kitty.conf"), "{opts:?}");
        assert!(!opts["kitty.file"].starts_with('~'));
        assert_eq!(reloads["kitty"], "pkill -USR1 -x kitty");
        // `~/` développé ; rechargement vidé : aucun.
        let mut o = Opts::new();
        o.insert("kitty.file".to_string(), "~/x/couleurs.conf".to_string());
        o.insert("kitty.reload".to_string(), String::new());
        o.insert("kitty.conf".to_string(), String::new());
        let (opts, reloads) = engine_options(&o);
        assert!(!opts["kitty.file"].starts_with('~'), "{opts:?}");
        assert!(!opts.contains_key("kitty.conf"));
        assert_eq!(reloads["kitty"], ":");
    }
}
