//! Cible « kitty » : `~/.config/kitty/hyprdesk-theme.conf`, inclus à la fin
//! de kitty.conf pour l'emporter, relu par SIGUSR1 (`pkill`).

use std::path::PathBuf;

use super::{Target, config_dir, hex, in_path, include_once, mix, write_if_changed};
use crate::config::Theme;

pub const TARGET: Target = Target {
    id: "kitty",
    label: "Kitty",
    summary: "Terminal : texte, fond, sélection, curseur, liens, onglets, bordures, noirs et \
              blancs de la palette.",
    installed,
    file,
    apply,
    clear,
};

const INCLUDE: &str = "include hyprdesk-theme.conf";

fn dir() -> PathBuf {
    config_dir("kitty")
}

fn file() -> PathBuf {
    dir().join("hyprdesk-theme.conf")
}

fn installed() -> bool {
    in_path("kitty")
}

/// Couleurs de kitty d'après le thème.
pub fn conf(t: &Theme) -> String {
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

/// Fait relire leur config aux kitty ouverts (SIGUSR1).
fn reload() {
    let _ = std::process::Command::new("pkill")
        .args(["-USR1", "-x", "kitty"])
        .status();
}

fn apply(t: &Theme) -> Result<String, String> {
    let changed = write_if_changed(&file(), &conf(t))?;
    let included = include_once(
        &dir().join("kitty.conf"),
        INCLUDE,
        &["include ./hyprdesk-theme.conf"],
        "# Thème de hyprdesk (Réglages → Thème des applications) : en dernier, il l'emporte.",
    )?;
    if changed || included {
        reload();
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

fn clear() -> Result<(), String> {
    let file = file();
    if !file.is_file() {
        return Ok(());
    }
    let empty = "# Thème de hyprdesk coupé (Réglages → Thème des applications).\n";
    if write_if_changed(&file, empty)? {
        reload();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couleurs_de_kitty() {
        let c = conf(&Theme::default());
        assert!(c.contains("foreground               #f2f4fc"), "{c}");
        assert!(c.contains("background               #171b27"), "{c}");
        assert!(c.contains("selection_background     #a69cff"), "{c}");
        assert!(c.lines().filter(|l| l.starts_with("color")).count() == 4);
    }

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
        let t = Theme::default();
        println!("1er : {:?}", apply(&t));
        println!("2e  : {:?}", apply(&t));
        let kconf = std::fs::read_to_string(kdir.join("kitty.conf")).unwrap();
        println!("inclusions : {}", kconf.matches(INCLUDE).count());
        let tail: Vec<&str> = kconf.lines().rev().take(3).collect();
        println!(
            "fin de kitty.conf :\n{}",
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        );
        println!(
            "copie : {}",
            kdir.join("kitty.conf.avant-hyprdesk.bak").is_file()
        );
        clear().unwrap();
        println!(
            "coupé : {:?}",
            std::fs::read_to_string(kdir.join("hyprdesk-theme.conf")).unwrap()
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
