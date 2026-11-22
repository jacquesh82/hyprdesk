//! Cible « kitty » : un fichier de couleurs (`~/.config/kitty/hyprdesk-theme.conf`
//! par défaut), inclus à la fin de la configuration de kitty pour l'emporter,
//! relu par SIGUSR1. Les trois se règlent (fichier, configuration à compléter
//! — vide : ne pas y toucher —, commande de rechargement).

use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use super::{Opt, Opts, Target, config_dir, in_path, opt, run_reload, write_if_changed};
use crate::config::Theme;
use crate::theme_engine;

const OPTIONS: &[Opt] = &[
    Opt {
        key: "file",
        label: "Fichier de couleurs",
        help: "Écrit par hyprdesk à chaque changement de thème.",
        default: || {
            config_dir("kitty")
                .join("hyprdesk-theme.conf")
                .display()
                .to_string()
        },
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
    PathBuf::from(if f.is_empty() {
        (OPTIONS[0].default)()
    } else {
        f
    })
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

fn apply(t: &Theme, o: &Opts) -> Result<String, String> {
    // Délégation moteur : fichier réglé, inclusion dans kitty.conf,
    // rechargement configuré (docs/THEME-ENGINE.md §8).
    let roots = theme_engine::paths::Roots::from_env();
    let resolved = theme_engine::resolve::from_theme(t);
    let mut target_opts = BTreeMap::new();
    target_opts.insert("kitty.file".to_string(), file(o).display().to_string());
    if let Some(c) = conf_path(o) {
        target_opts.insert("kitty.conf".to_string(), c.display().to_string());
    }
    let mut reload_commands = BTreeMap::new();
    let cmd = opt(o, "kitty", &OPTIONS[2]);
    if !cmd.is_empty() {
        reload_commands.insert("kitty".to_string(), cmd);
    }
    let options = theme_engine::apply::Options {
        only: vec!["kitty".to_string()],
        origin: "daemon".to_string(),
        check_installed: false,
        reload: true,
        target_opts,
        reload_commands,
        ..Default::default()
    };
    let report = theme_engine::apply::apply(&resolved, None, &roots, &options);
    if !report.errors.is_empty() {
        return Err(report.errors.join(" ; "));
    }
    Ok(crate::tr!(
        "kitty : thème « {name} » appliqué par le moteur de thème.",
        name = t.name
    ))
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

    /// Délégation moteur sur dossier XDG isolé (env modifié) :
    /// `cargo test -- --ignored --nocapture essai_kitty_moteur`.
    #[test]
    #[ignore]
    fn essai_kitty_moteur() {
        let tmp =
            std::env::temp_dir().join(format!("hyprdesk-kitty-moteur-{}", std::process::id()));
        let kdir = tmp.join("kitty");
        std::fs::create_dir_all(&kdir).unwrap();
        std::fs::write(kdir.join("kitty.conf"), "font_size 11\n").unwrap();
        let state = tmp.join("state");
        std::fs::create_dir_all(&state).unwrap();
        let cache = tmp.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        // SAFETY : essai lancé seul, à la main.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &tmp);
            std::env::set_var("XDG_STATE_HOME", &state);
            std::env::set_var("XDG_CACHE_HOME", &cache);
        }
        let mut o = Opts::new();
        o.insert("kitty.reload".into(), String::new());
        let r = apply(&Theme::default(), &o).unwrap();
        println!("{r}");
        let couleurs = std::fs::read_to_string(kdir.join("hyprdesk-theme.conf")).unwrap();
        assert!(couleurs.contains("foreground"), "{couleurs}");
        let kconf = std::fs::read_to_string(kdir.join("kitty.conf")).unwrap();
        assert!(kconf.contains("include hyprdesk-theme.conf"), "{kconf}");
        // Clear remet un fichier vide.
        clear(&o).unwrap();
        assert!(
            !std::fs::read_to_string(kdir.join("hyprdesk-theme.conf"))
                .unwrap()
                .contains("foreground")
        );
        // SAFETY : fin de l'essai, environnement de test rétabli.
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::remove_var("XDG_STATE_HOME");
            std::env::remove_var("XDG_CACHE_HOME");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
