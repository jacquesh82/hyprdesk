//! Cible « kitty » : un fichier de couleurs (`~/.config/kitty/hyprdesk-theme.conf`
//! par défaut), inclus à la fin de la configuration de kitty pour l'emporter,
//! relu par SIGUSR1. Les trois se règlent (fichier, configuration à compléter
//! — vide : ne pas y toucher —, commande de rechargement).

use std::path::{Path, PathBuf};

use super::{
    Opt, Opts, Target, config_dir, hex, in_path, include_once, mix, opt, run_reload,
    write_if_changed,
};
use crate::config::Theme;

const OPTIONS: &[Opt] = &[
    Opt {
        key: "file",
        label: "Fichier de couleurs",
        help: "Écrit par hyprdesk à chaque changement de thème.",
        default: || config_dir("kitty").join("hyprdesk-theme.conf").display().to_string(),
    },
    Opt {
        key: "conf",
        label: "Configuration qui l'inclut",
        help: "hyprdesk y ajoute une fois la ligne include. Vide : ne pas la modifier (à inclure soi-même).",
        default: || config_dir("kitty").join("kitty.conf").display().to_string(),
    },
    Opt {
        key: "reload",
        label: "Commande de rechargement",
        help: "Fait relire leur configuration aux kitty ouverts. Vide : aucune.",
        default: || "pkill -USR1 -x kitty".into(),
    },
];

pub const TARGET: Target = Target {
    id: "kitty",
    label: "Kitty",
    summary: "Terminal : texte, fond, sélection, curseur, liens, onglets, bordures, noirs et \
              blancs de la palette.",
    installed,
    options: OPTIONS,
    file,
    files,
    apply,
    clear,
};

fn file(o: &Opts) -> PathBuf {
    let f = opt(o, "kitty", &OPTIONS[0]);
    // Vide : le fichier par défaut (il en faut un).
    PathBuf::from(if f.is_empty() { (OPTIONS[0].default)() } else { f })
}

fn conf_path(o: &Opts) -> Option<PathBuf> {
    Some(opt(o, "kitty", &OPTIONS[1]))
        .filter(|c| !c.is_empty())
        .map(PathBuf::from)
}

fn files(o: &Opts) -> Vec<PathBuf> {
    std::iter::once(file(o)).chain(conf_path(o)).collect()
}

fn installed() -> bool {
    in_path("kitty")
}

/// Ligne d'inclusion : le nom seul si le fichier est à côté de la
/// configuration, sinon son chemin complet.
fn include_line(file: &Path, conf: &Path) -> String {
    match (file.parent(), conf.parent(), file.file_name()) {
        (Some(a), Some(b), Some(name)) if a == b => format!("include {}", name.to_string_lossy()),
        _ => format!("include {}", file.display()),
    }
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

fn apply(t: &Theme, o: &Opts) -> Result<String, String> {
    let file = file(o);
    let changed = write_if_changed(&file, &conf(t))?;
    let Some(kconf) = conf_path(o) else {
        if changed {
            run_reload(&opt(o, "kitty", &OPTIONS[2]));
        }
        return Ok(crate::tr!(
            "kitty : thème « {name} » écrit dans {f} ; à inclure dans votre configuration ({line}).",
            name = t.name,
            f = file.display(),
            line = format!("include {}", file.display())
        ));
    };
    let line = include_line(&file, &kconf);
    let name = file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let included = include_once(
        &kconf,
        &line,
        &[
            &format!("include {name}"),
            &format!("include ./{name}"),
            &format!("include {}", file.display()),
        ],
        "# Thème de hyprdesk (Réglages → Thème des applications) : en dernier, il l'emporte.",
    )?;
    if changed || included {
        run_reload(&opt(o, "kitty", &OPTIONS[2]));
    }
    Ok(if included {
        crate::tr!(
            "kitty : thème « {name} » appliqué ; {c} l'inclut désormais (copie : {c}.avant-hyprdesk.bak).",
            name = t.name,
            c = kconf.display()
        )
    } else {
        crate::tr!("kitty : thème « {name} » appliqué.", name = t.name)
    })
}

fn clear(o: &Opts) -> Result<(), String> {
    let file = file(o);
    if !file.is_file() {
        return Ok(());
    }
    let empty = "# Thème de hyprdesk coupé (Réglages → Thème des applications).\n";
    if write_if_changed(&file, empty)? {
        run_reload(&opt(o, "kitty", &OPTIONS[2]));
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

    #[test]
    fn options_reglables() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-kopt-{}", std::process::id()));
        let mut o = Opts::new();
        o.insert("kitty.file".into(), dir.join("couleurs.conf").display().to_string());
        o.insert("kitty.conf".into(), dir.join("kitty.conf").display().to_string());
        o.insert("kitty.reload".into(), String::new());
        let r = apply(&Theme::default(), &o).unwrap();
        assert!(r.contains("kitty.conf"), "{r}");
        let k = std::fs::read_to_string(dir.join("kitty.conf")).unwrap();
        assert!(k.contains("include couleurs.conf"), "{k}");
        assert!(std::fs::read_to_string(dir.join("couleurs.conf")).unwrap().contains("foreground"));
        // Configuration vide : on n'y touche pas, le fichier est quand même écrit.
        o.insert("kitty.conf".into(), String::new());
        o.insert("kitty.file".into(), dir.join("autre.conf").display().to_string());
        let r = apply(&Theme::default(), &o).unwrap();
        assert!(r.contains("à inclure"), "{r}");
        assert!(!std::fs::read_to_string(dir.join("kitty.conf")).unwrap().contains("autre.conf"));
        assert_eq!(
            include_line(Path::new("/a/x.conf"), Path::new("/b/kitty.conf")),
            "include /a/x.conf"
        );
        let _ = std::fs::remove_dir_all(&dir);
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
        println!("1er : {:?}", apply(&t, &Opts::new()));
        println!("2e  : {:?}", apply(&t, &Opts::new()));
        let kconf = std::fs::read_to_string(kdir.join("kitty.conf")).unwrap();
        println!("inclusions : {}", kconf.matches("include hyprdesk-theme.conf").count());
        let tail: Vec<&str> = kconf.lines().rev().take(3).collect();
        println!(
            "fin de kitty.conf :\n{}",
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        );
        println!(
            "copie : {}",
            kdir.join("kitty.conf.avant-hyprdesk.bak").is_file()
        );
        clear(&Opts::new()).unwrap();
        println!(
            "coupé : {:?}",
            std::fs::read_to_string(kdir.join("hyprdesk-theme.conf")).unwrap()
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
