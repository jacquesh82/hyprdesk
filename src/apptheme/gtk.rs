//! Cible « GTK », déléguée au moteur de thème : le thème GTK de base
//! recoloré et installé (`hyprdesk-<pack>`, MLG-237), et un bloc de couleurs
//! nommées en fin des gtk.css de l'utilisateur (ses règles restent devant). Le rechargement passe par
//! `gsettings` (schéma clair/sombre), côté moteur.
//!
//! Clear : le bloc saute des deux feuilles — les règles de l'utilisateur
//! ne sont jamais touchées.

use std::path::PathBuf;

use super::{Opt, Opts, Target, config_dir, in_path, run_reload, write_if_changed};
use crate::config::Theme;
use crate::theme_engine;

const OPTIONS: &[Opt] = &[];

pub const TARGET: Target = Target {
    id: "gtk",
    label: "GTK 3 et 4",
    summary: "Thème GTK de base recoloré aux couleurs du pack (hyprdesk-<pack>, suivi à chaud \
              par les applications ouvertes, Brave et Firefox compris), couleurs nommées \
              gtk-3.0 et gtk-4.0 et schéma clair/sombre.",
    installed,
    options: OPTIONS,
    file,
    files,
    apply,
    clear,
};

fn file(_o: &Opts) -> PathBuf {
    config_dir("gtk-3.0").join("gtk.css")
}

fn files(_o: &Opts) -> Vec<PathBuf> {
    vec![
        config_dir("gtk-3.0").join("gtk.css"),
        config_dir("gtk-4.0").join("gtk.css"),
    ]
}

fn installed() -> bool {
    in_path("gsettings")
}

fn apply(t: &Theme, _o: &Opts) -> Result<String, String> {
    engine_apply(t, &["gtk"])?;
    Ok(
        crate::tr!("GTK : thème de base recoloré et sélectionné, couleurs nommées posées ; schéma clair/sombre réglé.")
            .to_string(),
    )
}

/// Clear : le bloc marqué saute des deux feuilles, le thème de base revient
/// et les thèmes recolorés sont retirés.
fn clear(_o: &Opts) -> Result<(), String> {
    let mut changé =
        !theme_engine::targets::remove_gtk_theme(&theme_engine::paths::Roots::from_env(), false)
            .is_empty();
    for path in files(&Opts::new()) {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        let sans = theme_engine::targets::remove_block(&current);
        if sans != current {
            changé |= write_if_changed(&path, &sans)?;
        }
    }
    if changé {
        run_reload("gsettings set org.gnome.desktop.interface color-scheme 'default'");
    }
    Ok(())
}

/// Délégation moteur partagée aux modules de cibles du démon.
pub(super) fn engine_apply(t: &Theme, only: &[&str]) -> Result<(), String> {
    let roots = theme_engine::paths::Roots::from_env();
    let (resolved, pack_dir) = theme_engine::bridge::resolution_for(&roots, t);
    let opts = theme_engine::apply::Options {
        only: only.iter().map(|s| (*s).to_string()).collect(),
        origin: "daemon".to_string(),
        check_installed: true,
        reload: true,
        ..Default::default()
    };
    let report = theme_engine::apply::apply(&resolved, pack_dir.as_deref(), &roots, &opts);
    if report.errors.is_empty() {
        Ok(())
    } else {
        Err(report.errors.join(" ; "))
    }
}
