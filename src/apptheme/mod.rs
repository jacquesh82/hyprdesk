//! Thème des applications (paquet « Thème des applications ») : le thème de
//! hyprdesk appliqué à d'autres logiciels, en architecture « plug-n-play »
//! comme les widgets :
//!
//! - chaque application est un module (une CIBLE) qui expose
//!   `pub const TARGET: Target` : [`kitty`] ;
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
//! [`kitty`] (ou [`thunar`] pour une feuille GTK), un `pub mod <id>;` ci-dessous, une ligne dans [`TARGETS`], les
//! textes dans `locales/en.toml`, et l'outil de rechargement éventuel dans
//! `doctor::DEPS`.

pub mod kitty;
pub mod thunar;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::config::{Rgba, Theme};

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

/// Registre des applications qui peuvent prendre le thème.
pub const TARGETS: &[Target] = &[kitty::TARGET, thunar::TARGET];

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
    let mut out = Vec::new();
    for t in TARGETS {
        if enabled.contains(t.id) {
            if (t.installed)() {
                out.push((t.id.to_string(), (t.apply)(theme, opts)));
            }
        } else if let Err(e) = (t.clear)(opts) {
            out.push((t.id.to_string(), Err(e)));
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
/// `<conf>.avant-hyprdesk.bak`. `comment` précède la ligne. Rend vrai si
/// `conf` a été modifié.
pub(crate) fn include_once(
    conf: &Path,
    line: &str,
    already: &[&str],
    comment: &str,
) -> Result<bool, String> {
    let current = std::fs::read_to_string(conf).unwrap_or_default();
    if current
        .lines()
        .any(|l| l.trim() == line || already.contains(&l.trim()))
    {
        return Ok(false);
    }
    if let Some(e) = dangling(conf) {
        return Err(e);
    }
    if conf.is_file() {
        let bak = backup_path(conf);
        if !bak.exists() {
            std::fs::copy(conf, &bak).map_err(|e| fs_err(&bak, e))?;
        }
    }
    let mut next = current;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next += &format!("\n{comment}\n{line}\n");
    write_if_changed(conf, &next)
}

/// Comme [`include_once`], mais la ligne va en tête du fichier (feuilles
/// CSS de GTK : un `@import` placé après d'autres règles est ignoré).
pub(crate) fn include_first(
    conf: &Path,
    line: &str,
    already: &[&str],
    comment: &str,
) -> Result<bool, String> {
    if let Some(e) = dangling(conf) {
        return Err(e);
    }
    let current = std::fs::read_to_string(conf).unwrap_or_default();
    if current
        .lines()
        .any(|l| l.trim() == line || already.contains(&l.trim()))
    {
        return Ok(false);
    }
    if conf.is_file() {
        let bak = backup_path(conf);
        if !bak.exists() {
            std::fs::copy(conf, &bak).map_err(|e| fs_err(&bak, e))?;
        }
    }
    let next = if current.is_empty() {
        format!("{comment}\n{line}\n")
    } else {
        format!("{comment}\n{line}\n\n{current}")
    };
    write_if_changed(conf, &next)
}

/// Copie de sauvegarde d'une config avant sa première modification.
pub(crate) fn backup_path(conf: &Path) -> PathBuf {
    let mut name = conf.file_name().unwrap_or_default().to_os_string();
    name.push(".avant-hyprdesk.bak");
    conf.with_file_name(name)
}

/// `#rrggbb`.
pub(crate) fn hex(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// Mélange `a` vers `b` (t = 0 : a, t = 1 : b).
pub(crate) fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Rgba(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2), 0xff)
}

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
    }

    #[test]
    fn couleurs() {
        assert_eq!(
            mix(Rgba(0, 0, 0, 255), Rgba(200, 100, 50, 255), 0.5),
            Rgba(100, 50, 25, 255)
        );
        assert_eq!(hex(Rgba(0x17, 0x1b, 0x27, 0x80)), "#171b27");
        assert_eq!(
            backup_path(Path::new("/x/kitty.conf")),
            PathBuf::from("/x/kitty.conf.avant-hyprdesk.bak")
        );
    }

    #[test]
    fn lien_casse_signale() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-lien-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("kitty.conf");
        std::os::unix::fs::symlink(dir.join("absent/dotfiles/kitty.conf"), &conf).unwrap();
        let e = include_once(&conf, "include t.conf", &[], "# hd").unwrap_err();
        assert!(e.contains("kitty.conf") && e.contains("absent/dotfiles"), "{e}");
        let e = write_if_changed(&conf, "x").unwrap_err();
        assert!(e.contains("absent/dotfiles"), "{e}");
        // Dossier manquant : créé, sans erreur.
        assert!(write_if_changed(&dir.join("sous/dossier/t.conf"), "x").unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn inclusion_unique() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-incl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("app.conf");
        std::fs::write(&conf, "font_size 11").unwrap();
        assert!(include_once(&conf, "include t.conf", &["include ./t.conf"], "# hd").unwrap());
        assert!(!include_once(&conf, "include t.conf", &[], "# hd").unwrap());
        let s = std::fs::read_to_string(&conf).unwrap();
        assert_eq!(s, "font_size 11\n\n# hd\ninclude t.conf\n");
        assert_eq!(
            std::fs::read_to_string(backup_path(&conf)).unwrap(),
            "font_size 11"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
