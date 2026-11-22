//! Cible « Qt » : schéma de couleurs qt5/6ct généré par le moteur
//! (QPalette en #aarrggbb), sélectionné dans qt5/6ct — vaut pour les
//! applications lancées ensuite. Aucune option : le fichier est écrit par le
//! moteur et retiré par clear.

use std::path::PathBuf;

use super::{Opt, Opts, Target, config_dir, in_path};
use crate::config::Theme;

const OPTIONS: &[Opt] = &[];

/// Fichier écrit pour la palette implicite du démon (« thème réglé »).
fn fichier() -> PathBuf {
    config_dir("qt6ct")
        .join("colors")
        .join("hyprdesk-regle.conf")
}

pub const TARGET: Target = Target {
    id: "qt",
    label: "Qt",
    summary: "Schéma de couleurs qt5/6ct (QPalette) ; vaut pour les applications lancées ensuite.",
    installed,
    options: OPTIONS,
    file,
    files,
    apply,
    clear,
};

fn file(_o: &Opts) -> PathBuf {
    fichier()
}

fn files(_o: &Opts) -> Vec<PathBuf> {
    vec![fichier()]
}

fn installed() -> bool {
    in_path("qt6ct") || config_dir("qt6ct").is_dir()
}

fn apply(t: &Theme, _o: &Opts) -> Result<String, String> {
    super::gtk::engine_apply(t, &["qt"])?;
    Ok(
        crate::tr!("Qt : schéma de couleurs qt5/6ct posé ; vaut au relancement des applications.")
            .to_string(),
    )
}

/// Clear : le fichier du moteur saute (jamais un fichier de l'utilisateur).
fn clear(_o: &Opts) -> Result<(), String> {
    let path = fichier();
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
