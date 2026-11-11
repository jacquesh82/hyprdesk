//! Modules de barre livrés après coup : ajoutés une fois à la barre.
//!
//! La barre réglée (`hd.bar` de settings.lua) liste ses modules : un plugin
//! intégré livré ensuite n'y entrerait jamais, et resterait introuvable. Au
//! démarrage du démon, chaque module de `OFFERED` pas encore proposé est
//! ajouté à droite (avant notifications, bureau et session) puis noté dans
//! `$XDG_STATE_HOME/hyprdesk/bar-offered` : retiré ensuite, il ne revient pas.
//! Ces modules se cachent d'eux-mêmes quand ils n'ont rien à montrer.

use std::path::{Path, PathBuf};

use crate::settings::{BarEntry, BarSettings, Settings};

/// Modules proposés (dans l'ordre où ils s'ajoutent).
pub const OFFERED: &[&str] = &[
    "brightness",
    "clavier",
    "nuit",
    "supports",
    "capture",
    "majs",
];

/// Modules devant lesquels les nouveaux se placent (le premier trouvé).
const BEFORE: &[&str] = &["notifications", "desktop", "session"];

fn state_path() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        })
        .join("hyprdesk")
        .join("bar-offered")
}

/// Ajoute à la barre les modules `ids` qui n'y sont pas ; vrai si elle a changé.
pub fn insert(bar: &mut BarSettings, ids: &[&str]) -> bool {
    let present = |b: &BarSettings, id: &str| {
        b.zones
            .iter()
            .chain(b.portrait.iter().flatten())
            .flatten()
            .any(|e| e.type_id == id)
    };
    let mut changed = false;
    for id in ids {
        if present(bar, id) {
            continue;
        }
        let right = &mut bar.zones[2];
        let at = right
            .iter()
            .position(|e| BEFORE.contains(&e.type_id.as_str()))
            .unwrap_or(right.len());
        right.insert(at, BarEntry::new(id));
        changed = true;
    }
    changed
}

/// Modules de `OFFERED` absents de la liste déjà proposée.
pub fn pending(offered: &str) -> Vec<&'static str> {
    let seen: Vec<&str> = offered.lines().map(str::trim).collect();
    OFFERED
        .iter()
        .copied()
        .filter(|id| !seen.contains(id))
        .collect()
}

/// Au démarrage du démon : propose les nouveaux modules à la barre réglée.
pub fn run(config_path: &Path) {
    let state = state_path();
    let todo = pending(&std::fs::read_to_string(&state).unwrap_or_default());
    if todo.is_empty() {
        return;
    }
    let cfg = match crate::config::lua::load_settings_only(config_path) {
        Ok(c) => c,
        // settings.lua illisible : on réessaiera au prochain démarrage.
        Err(_) => return,
    };
    let mut s = Settings::from_config(&cfg);
    if let Some(bar) = s.bar.as_mut()
        && insert(bar, &todo)
    {
        if let Err(e) = s.save(config_path) {
            tracing::warn!("{}", crate::tr!("barre : ajout des modules : {e}", e = e));
            return;
        }
        tracing::info!(
            "{}",
            crate::tr!(
                "barre : nouveaux modules ajoutés : {p0}",
                p0 = todo.join(", ")
            )
        );
    }
    let _ = std::fs::create_dir_all(state.parent().unwrap_or(Path::new(".")));
    let _ = std::fs::write(&state, OFFERED.join("\n") + "\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_proposes_une_fois() {
        let mut bar = BarSettings::default();
        bar.zones[2].retain(|e| !OFFERED.contains(&e.type_id.as_str()));
        assert!(insert(&mut bar, &["nuit", "majs"]));
        let right: Vec<&str> = bar.zones[2].iter().map(|e| e.type_id.as_str()).collect();
        let n = right.iter().position(|i| *i == "notifications").unwrap();
        assert_eq!(&right[n - 2..n], &["nuit", "majs"]);
        // Déjà là : rien ne change.
        assert!(!insert(&mut bar, &["nuit"]));
        // Ce qui a déjà été proposé ne l'est plus (même retiré depuis).
        assert_eq!(
            pending("brightness\nclavier\nnuit\n"),
            vec!["supports", "capture", "majs"]
        );
        assert!(pending(&OFFERED.join("\n")).is_empty());
    }

    #[test]
    fn sans_session_ni_bureau_en_bout_de_barre() {
        let mut bar = BarSettings::default();
        bar.zones[2] = vec![BarEntry::new("volume")];
        assert!(insert(&mut bar, &["clavier"]));
        assert_eq!(bar.zones[2].last().unwrap().type_id, "clavier");
    }
}
