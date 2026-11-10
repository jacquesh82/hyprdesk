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
//! Ajouter une application : un fichier `<id>.rs` sur le modèle de
//! [`kitty`], un `pub mod <id>;` ci-dessous, une ligne dans [`TARGETS`], les
//! textes dans `locales/en.toml`, et l'outil de rechargement éventuel dans
//! `doctor::DEPS`.

pub mod kitty;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::config::{Rgba, Theme};

/// Application qui peut prendre le thème de hyprdesk.
pub struct Target {
    /// Clé enregistrée dans `general.app_themes` : ne plus la changer.
    pub id: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    /// Le logiciel est présent.
    pub installed: fn() -> bool,
    /// Fichier de couleurs écrit par hyprdesk.
    pub file: fn() -> PathBuf,
    /// Écrit et applique le thème ; rend un compte rendu.
    pub apply: fn(&Theme) -> Result<String, String>,
    /// Retire le thème (fichier vidé, logiciel rechargé).
    pub clear: fn() -> Result<(), String>,
}

/// Registre des applications qui peuvent prendre le thème.
pub const TARGETS: &[Target] = &[kitty::TARGET];

pub fn target(id: &str) -> Option<&'static Target> {
    TARGETS.iter().find(|t| t.id == id)
}

/// Applique le thème aux cibles choisies et le retire des autres (celles où
/// hyprdesk l'avait posé). Rend un compte rendu par cible.
pub fn sync(enabled: &BTreeSet<String>, theme: &Theme) -> Vec<(String, Result<String, String>)> {
    let mut out = Vec::new();
    for t in TARGETS {
        if enabled.contains(t.id) {
            if (t.installed)() {
                out.push((t.id.to_string(), (t.apply)(theme)));
            }
        } else if let Err(e) = (t.clear)() {
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

/// L'exécutable `bin` est dans le `PATH`.
pub(crate) fn in_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
}

/// Écrit `content` dans `file` s'il diffère (dossier créé au besoin). Rend
/// vrai si le fichier a changé.
pub(crate) fn write_if_changed(file: &Path, content: &str) -> Result<bool, String> {
    if std::fs::read_to_string(file).ok().as_deref() == Some(content) {
        return Ok(false);
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, content).map_err(|e| e.to_string())?;
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
    if conf.is_file() {
        let bak = backup_path(conf);
        if !bak.exists() {
            std::fs::copy(conf, &bak).map_err(|e| e.to_string())?;
        }
    }
    let mut next = current;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next += &format!("\n{comment}\n{line}\n");
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
