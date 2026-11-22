//! Cible « thunar » : une feuille GTK 3 (`~/.config/gtk-3.0/hyprdesk-thunar.css`
//! par défaut) dont toutes les règles sont limitées à la classe `.thunar` que
//! Thunar pose sur ses fenêtres : les autres applications GTK ne changent pas.
//! gtk.css l'importe (`@import`, en tête : GTK ignore un import placé après
//! d'autres règles). GTK ne relit pas gtk.css à chaud : le thème vaut pour les
//! fenêtres ouvertes ensuite (ou après `thunar -q`). Les trois se règlent
//! (fichier, feuille qui l'importe — vide : ne pas y toucher —, commande de
//! rechargement, vide par défaut : `thunar -q` fermerait les fenêtres).

use std::path::PathBuf;

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
            config_dir("gtk-3.0")
                .join("hyprdesk-thunar.css")
                .display()
                .to_string()
        },
    },
    Opt {
        key: "conf",
        label: "Feuille GTK qui l'importe",
        help: "hyprdesk y ajoute une fois la ligne @import, en tête. Vide : ne pas la modifier (à importer soi-même).",
        default: || config_dir("gtk-3.0").join("gtk.css").display().to_string(),
    },
    Opt {
        key: "reload",
        label: "Commande de rechargement",
        help: "GTK ne relit pas ses feuilles à chaud : vide, le thème vaut pour les prochaines fenêtres ; « thunar -q » ferme Thunar pour qu'il reparte à jour.",
        default: String::new,
    },
];

pub const TARGET: Target = Target {
    id: "thunar",
    label: "Thunar",
    summary: "Gestionnaire de fichiers : fond, texte, sélection, panneau latéral, barres, \
              champs. Seul Thunar change, pas les autres applications GTK.",
    installed,
    options: OPTIONS,
    file,
    files,
    apply,
    clear,
};

fn file(o: &Opts) -> PathBuf {
    let f = opt(o, "thunar", &OPTIONS[0]);
    PathBuf::from(if f.is_empty() {
        (OPTIONS[0].default)()
    } else {
        f
    })
}

fn conf_path(o: &Opts) -> Option<PathBuf> {
    Some(opt(o, "thunar", &OPTIONS[1]))
        .filter(|c| !c.is_empty())
        .map(PathBuf::from)
}

fn files(o: &Opts) -> Vec<PathBuf> {
    std::iter::once(file(o)).chain(conf_path(o)).collect()
}

fn installed() -> bool {
    in_path("thunar")
}

fn apply(t: &Theme, o: &Opts) -> Result<String, String> {
    // Délégation moteur : feuille limitée à .thunar, import en tête de
    // gtk.css réglé, rechargement configuré (docs/THEME-ENGINE.md §8).
    let roots = theme_engine::paths::Roots::from_env();
    let resolved = theme_engine::resolve::from_theme(t);
    let mut target_opts = BTreeMap::new();
    target_opts.insert("thunar.file".to_string(), file(o).display().to_string());
    if let Some(c) = conf_path(o) {
        target_opts.insert("thunar.conf".to_string(), c.display().to_string());
    }
    let mut reload_commands = BTreeMap::new();
    let cmd = opt(o, "thunar", &OPTIONS[2]);
    if !cmd.is_empty() {
        reload_commands.insert("thunar".to_string(), cmd);
    }
    let options = theme_engine::apply::Options {
        only: vec!["thunar".to_string()],
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
        "Thunar : thème « {name} » appliqué par le moteur de thème (prochaines fenêtres).",
        name = t.name
    ))
}

fn clear(o: &Opts) -> Result<(), String> {
    let file = file(o);
    if !file.is_file() {
        return Ok(());
    }
    // Feuille vidée, pas supprimée : l'import de gtk.css reste valide.
    let empty = "/* Thème de Thunar par hyprdesk coupé (Réglages → Thème des applications). */\n";
    if write_if_changed(&file, empty)? {
        run_reload(&opt(o, "thunar", &OPTIONS[2]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Délégation moteur sur dossier XDG isolé (env modifié) :
    /// `cargo test -- --ignored --nocapture essai_thunar_moteur`.
    #[test]
    #[ignore]
    fn essai_thunar_moteur() {
        let tmp =
            std::env::temp_dir().join(format!("hyprdesk-thunar-moteur-{}", std::process::id()));
        let gtk = tmp.join("gtk-3.0");
        std::fs::create_dir_all(&gtk).unwrap();
        std::fs::write(gtk.join("gtk.css"), "@define-color accent_color #9580ff;\n").unwrap();
        let state = tmp.join("state");
        std::fs::create_dir_all(&state).unwrap();
        // SAFETY : essai lancé seul, à la main.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &tmp);
            std::env::set_var("XDG_STATE_HOME", &state);
            std::env::set_var("XDG_CACHE_HOME", tmp.join("cache"));
        }
        let mut o = Opts::new();
        o.insert("thunar.reload".into(), String::new());
        apply(&Theme::default(), &o).unwrap();
        let css = std::fs::read_to_string(gtk.join("gtk.css")).unwrap();
        // Import en tête, règles limitées à .thunar, règles de l'utilisateur gardées.
        let première = css
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with("/*"))
            .unwrap();
        assert_eq!(première, "@import url(\"hyprdesk-thunar.css\");", "{css}");
        assert!(css.contains("@define-color accent_color #9580ff;"), "{css}");
        let feuille = std::fs::read_to_string(gtk.join("hyprdesk-thunar.css")).unwrap();
        assert!(feuille.contains(".thunar"), "{feuille}");
        // Clear : le bloc et l'import ne disparaissent pas de gtk.css (ils
        // appartiennent aux cibles gtk et thunar du moteur), mais la feuille
        // thunar se vide.
        clear(&o).unwrap();
        assert!(
            !std::fs::read_to_string(gtk.join("hyprdesk-thunar.css"))
                .unwrap()
                .contains(".thunar")
        );
        // SAFETY : fin de l'essai.
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::remove_var("XDG_STATE_HOME");
            std::env::remove_var("XDG_CACHE_HOME");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
