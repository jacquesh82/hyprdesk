//! Thème des applications (paquet « Thème des applications ») : le thème de
//! hyprdesk appliqué à d'autres logiciels. Chaque cible écrit son fichier de
//! couleurs, l'inclut une fois dans la config du logiciel (copie de
//! sauvegarde avant) et lui fait relire sa config.
//!
//! Cible intégrée : kitty (`~/.config/kitty/hyprdesk-theme.conf`, inclus à la
//! fin de kitty.conf pour l'emporter, relu par SIGUSR1). Couper une cible vide
//! son fichier : le thème d'avant reprend la main.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::config::{Rgba, Theme};

/// Application qui peut prendre le thème de hyprdesk.
pub struct Target {
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

pub const TARGETS: &[Target] = &[Target {
    id: "kitty",
    label: "Kitty",
    summary: "Terminal : texte, fond, sélection, curseur, liens, onglets, bordures, noirs et \
              blancs de la palette.",
    installed: kitty_installed,
    file: kitty_file,
    apply: kitty_apply,
    clear: kitty_clear,
}];

pub fn target(id: &str) -> Option<&'static Target> {
    TARGETS.iter().find(|t| t.id == id)
}

fn hex(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// Mélange `a` vers `b` (t = 0 : a, t = 1 : b).
fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Rgba(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2), 0xff)
}

// ---------------------------------------------------------------------------
// kitty

fn kitty_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"))
        .join("kitty")
}

fn kitty_file() -> PathBuf {
    kitty_dir().join("hyprdesk-theme.conf")
}

fn kitty_installed() -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("kitty").is_file()))
}

/// Couleurs de kitty d'après le thème.
pub fn kitty_conf(t: &Theme) -> String {
    let (fg, bg, ac) = (t.fg, t.bg, t.accent);
    let lines = [
        ("foreground", fg),
        ("background", bg),
        ("selection_foreground", bg),
        ("selection_background", ac),
        ("cursor", ac),
        ("cursor_text_color", bg),
        ("url_color", ac),
        ("active_border_color", ac),
        ("inactive_border_color", mix(bg, fg, 0.2)),
        ("bell_border_color", ac),
        ("active_tab_foreground", bg),
        ("active_tab_background", ac),
        ("inactive_tab_foreground", mix(fg, bg, 0.3)),
        ("inactive_tab_background", mix(bg, fg, 0.12)),
        ("tab_bar_background", bg),
        ("mark1_background", ac),
        ("color0", mix(bg, fg, 0.15)),
        ("color8", mix(bg, fg, 0.35)),
        ("color7", mix(fg, bg, 0.15)),
        ("color15", fg),
    ];
    let mut o = format!(
        "# Généré par hyprdesk (Réglages → Thème des applications) — thème « {} ».\n\
         # Réécrit à chaque changement de thème ; inclus à la fin de kitty.conf.\n\n",
        t.name
    );
    for (k, c) in lines {
        o += &format!("{k:<24} {}\n", hex(c));
    }
    o
}

const KITTY_INCLUDE: &str = "include hyprdesk-theme.conf";

/// Inclut le fichier de hyprdesk à la fin de kitty.conf (une fois ; copie de
/// sauvegarde avant la première modification).
fn kitty_include() -> Result<bool, String> {
    let conf = kitty_dir().join("kitty.conf");
    let current = std::fs::read_to_string(&conf).unwrap_or_default();
    if current
        .lines()
        .any(|l| l.trim() == KITTY_INCLUDE || l.trim() == "include ./hyprdesk-theme.conf")
    {
        return Ok(false);
    }
    if conf.is_file() {
        let bak = conf.with_extension("conf.avant-hyprdesk.bak");
        if !bak.exists() {
            std::fs::copy(&conf, &bak).map_err(|e| e.to_string())?;
        }
    }
    let mut next = current;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next +=
        "\n# Thème de hyprdesk (Réglages → Thème des applications) : en dernier, il l'emporte.\n";
    next += KITTY_INCLUDE;
    next.push('\n');
    std::fs::create_dir_all(kitty_dir()).map_err(|e| e.to_string())?;
    std::fs::write(&conf, next).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Fait relire leur config aux kitty ouverts (SIGUSR1).
fn kitty_reload() -> usize {
    std::process::Command::new("pkill")
        .args(["-USR1", "-x", "kitty"])
        .status()
        .ok()
        .filter(|s| s.success())
        .map_or(0, |_| 1)
}

fn kitty_apply(t: &Theme) -> Result<String, String> {
    let file = kitty_file();
    let conf = kitty_conf(t);
    let changed = std::fs::read_to_string(&file).ok().as_deref() != Some(conf.as_str());
    if changed {
        std::fs::create_dir_all(kitty_dir()).map_err(|e| e.to_string())?;
        std::fs::write(&file, &conf).map_err(|e| e.to_string())?;
    }
    let included = kitty_include()?;
    if changed || included {
        kitty_reload();
    }
    Ok(if included {
        crate::tr!(
            "kitty : thème « {name} » appliqué ; kitty.conf l'inclut désormais (copie : kitty.conf.avant-hyprdesk.bak).",
            name = t.name
        )
    } else {
        crate::tr!("kitty : thème « {name} » appliqué.", name = t.name)
    })
}

fn kitty_clear() -> Result<(), String> {
    let file = kitty_file();
    if !file.is_file() {
        return Ok(());
    }
    let empty = "# Thème de hyprdesk coupé (Réglages → Thème des applications).\n";
    if std::fs::read_to_string(&file).ok().as_deref() != Some(empty) {
        std::fs::write(&file, empty).map_err(|e| e.to_string())?;
        kitty_reload();
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couleurs_de_kitty() {
        let t = Theme::default();
        let c = kitty_conf(&t);
        assert!(c.contains("foreground               #f2f4fc"), "{c}");
        assert!(c.contains("background               #171b27"), "{c}");
        assert!(c.contains("selection_background     #a69cff"), "{c}");
        assert!(c.lines().filter(|l| l.starts_with("color")).count() == 4);
        assert_eq!(
            mix(Rgba(0, 0, 0, 255), Rgba(200, 100, 50, 255), 0.5),
            Rgba(100, 50, 25, 255)
        );
        assert!(target("kitty").is_some() && target("vim").is_none());
    }
}

#[cfg(test)]
mod essai_kitty {
    /// Sur une copie de la config de kitty du poste (dossier temporaire) :
    /// `cargo test -- --ignored --nocapture essai_inclusion_kitty`.
    #[test]
    #[ignore]
    fn essai_inclusion_kitty() {
        let tmp = std::env::temp_dir().join(format!("hyprdesk-kitty-{}", std::process::id()));
        let kdir = tmp.join("kitty");
        std::fs::create_dir_all(&kdir).unwrap();
        let home = std::env::var("HOME").unwrap();
        std::fs::copy(
            format!("{home}/.config/kitty/kitty.conf"),
            kdir.join("kitty.conf"),
        )
        .unwrap();
        // SAFETY : test lancé seul, à la main.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &tmp) };
        let t = crate::config::Theme::default();
        println!("1er : {:?}", super::kitty_apply(&t));
        println!("2e  : {:?}", super::kitty_apply(&t));
        let conf = std::fs::read_to_string(kdir.join("kitty.conf")).unwrap();
        println!(
            "inclusions : {}",
            conf.matches(super::KITTY_INCLUDE).count()
        );
        println!(
            "fin de kitty.conf :\n{}",
            conf.lines()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n")
        );
        println!(
            "copie : {}",
            kdir.join("kitty.conf.avant-hyprdesk.bak").is_file()
        );
        super::kitty_clear().unwrap();
        println!(
            "coupé : {:?}",
            std::fs::read_to_string(kdir.join("hyprdesk-theme.conf")).unwrap()
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
