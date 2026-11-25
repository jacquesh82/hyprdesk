//! Modules de barre livrés après coup : ajoutés une fois à la barre.
//!
//! La barre réglée (`hd.bar` de settings.lua) liste ses modules : un plugin
//! intégré livré ensuite n'y entrerait jamais, et resterait introuvable. Au
//! démarrage du démon, chaque module de `OFFERED` pas encore proposé est
//! ajouté à droite (avant notifications, bureau et session) puis noté dans
//! `$XDG_STATE_HOME/hyprdesk/bar-offered` : retiré ensuite, il ne revient pas.
//! Ces modules se cachent d'eux-mêmes quand ils n'ont rien à montrer.
//!
//! Une fois aussi, une barre Continuité restée à l'ancienne pose par défaut
//! (horloge au centre) prend celle de macOS (`macos_layout`), notée dans
//! `bar-macos` du même dossier d'état ; et les menus du shell, dessinés
//! d'office en tête de barre avant d'être un module, entrent dans chaque
//! barre comme module `menus` (`menus_module`), noté dans `bar-menus`.

use std::path::{Path, PathBuf};

use crate::settings::{BarEntry, BarSettings, Settings};

/// Modules proposés (dans l'ordre où ils s'ajoutent).
pub const OFFERED: &[&str] = &[
    "brightness",
    "keyboard",
    "nightlight",
    "removable",
    "capture",
    "updates",
    "mymfa",
    "gpu",
    "voice",
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

/// Premier démarrage du démon sur ce compte : ni `first-start` ni
/// `bar-offered` (écrit à chaque démarrage par `run`) dans le dossier d'état.
/// À appeler avant `run` ; le marqueur `first-start` est posé aussitôt.
pub fn first_start() -> bool {
    let dir = state_path();
    let dir = dir.parent().unwrap_or(Path::new("."));
    let marker = dir.join("first-start");
    let first = !marker.exists() && !dir.join("bar-offered").exists();
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(&marker, "");
    first
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
    // Ancien nom d'un module déjà proposé : il compte sous son nom actuel.
    let seen: Vec<&str> = offered
        .lines()
        .map(|l| crate::widgets::plugins::canonical(l.trim()))
        .collect();
    OFFERED
        .iter()
        .copied()
        .filter(|id| !seen.contains(id))
        .collect()
}

/// Ancienne pose par défaut de Continuité (horloge au centre), hors modules
/// proposés : une barre restée telle quelle prend la pose macOS.
const OLD_DEFAULT: [&[&str]; 3] = [
    &["workspaces", "window"],
    &["clock"],
    &[
        "tray",
        "media",
        "network",
        "bluetooth",
        "battery",
        "volume",
        "notifications",
        "desktop",
        "session",
        "controls",
    ],
];

/// Barre Continuité restée à l'ancienne pose par défaut : elle passe une fois
/// à celle de macOS (menus et fenêtre à gauche, horloge en bout à droite).
/// Ses modules gardent leurs options ; ceux qu'elle n'avait pas n'entrent
/// pas. Session (au menu hyprdesk) et les numéros d'espaces (au menu Espaces)
/// en sortent. Vrai si elle a changé.
pub fn macos_layout(bar: &mut BarSettings) -> bool {
    if bar.style != crate::config::BarStyle::Continuite {
        return false;
    }
    let ids = |z: &[BarEntry]| -> Vec<String> {
        z.iter()
            .map(|e| e.type_id.clone())
            .filter(|id| !OFFERED.contains(&id.as_str()))
            .collect()
    };
    if (0..3).any(|i| ids(&bar.zones[i]) != OLD_DEFAULT[i]) {
        return false;
    }
    let old: Vec<BarEntry> = bar.zones.iter().flatten().cloned().collect();
    let mut zones = crate::config::BarStyle::Continuite.preset_settings();
    for zone in zones.iter_mut() {
        *zone = zone
            .iter()
            .filter_map(|e| old.iter().find(|o| o.type_id == e.type_id).cloned())
            .collect();
    }
    // Modules proposés absents du préréglage (myMFA, GPU…) : à droite, avant
    // le bureau, comme `insert` les place. Session, passée au menu
    // hyprdesk, ne revient pas.
    for e in old {
        if OFFERED.contains(&e.type_id.as_str())
            && !zones.iter().flatten().any(|z| z.type_id == e.type_id)
        {
            let right = &mut zones[2];
            let at = right
                .iter()
                .position(|z| BEFORE.contains(&z.type_id.as_str()))
                .unwrap_or(right.len());
            right.insert(at, e);
        }
    }
    bar.zones = zones;
    true
}

fn macos_state() -> PathBuf {
    state_path().with_file_name("bar-macos")
}

/// Les menus du shell, dessinés d'office en tête de barre jusqu'ici,
/// deviennent le module `menus` : en tête de la zone gauche, comme avant.
/// Une barre Continuité qui laisse son centre vide et garde la fenêtre
/// active à gauche la met au centre, la pose de macOS. Vrai si elle a
/// changé.
pub fn menus_module(bar: &mut BarSettings) -> bool {
    let present = bar
        .zones
        .iter()
        .chain(bar.portrait.iter().flatten())
        .flatten()
        .any(|e| e.type_id == "menus");
    let mut changed = false;
    if !present {
        bar.zones[0].insert(0, BarEntry::new("menus"));
        changed = true;
    }
    if bar.style == crate::config::BarStyle::Continuite
        && bar.zones[1].is_empty()
        && let Some(at) = bar.zones[0].iter().position(|e| e.type_id == "window")
    {
        let window = bar.zones[0].remove(at);
        bar.zones[1].push(window);
        changed = true;
    }
    changed
}

fn menus_state() -> PathBuf {
    state_path().with_file_name("bar-menus")
}

/// Au démarrage du démon : propose les nouveaux modules à la barre réglée.
pub fn run(config_path: &Path) {
    let state = state_path();
    let todo = pending(&std::fs::read_to_string(&state).unwrap_or_default());
    let macos = !macos_state().exists();
    let menus = !menus_state().exists();
    if todo.is_empty() && !macos && !menus {
        return;
    }
    let cfg = match crate::config::lua::load_settings_only(config_path) {
        Ok(c) => c,
        // settings.lua illisible : on réessaiera au prochain démarrage.
        Err(_) => return,
    };
    let mut s = Settings::from_config(&cfg);
    if macos {
        if let Some(bar) = s.bar.as_mut()
            && macos_layout(bar)
        {
            if let Err(e) = s.save(config_path) {
                tracing::warn!("{}", crate::tr!("barre : pose macOS : {e}", e = e));
                return;
            }
            tracing::info!("{}", crate::tr!("barre : pose macOS appliquée"));
        }
        let _ = std::fs::create_dir_all(state.parent().unwrap_or(Path::new(".")));
        let _ = std::fs::write(macos_state(), "");
    }
    if menus {
        if let Some(bar) = s.bar.as_mut()
            && menus_module(bar)
        {
            if let Err(e) = s.save(config_path) {
                tracing::warn!("{}", crate::tr!("barre : module des menus : {e}", e = e));
                return;
            }
            tracing::info!("{}", crate::tr!("barre : module des menus ajouté"));
        }
        let _ = std::fs::create_dir_all(state.parent().unwrap_or(Path::new(".")));
        let _ = std::fs::write(menus_state(), "");
    }
    if todo.is_empty() {
        return;
    }
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
        assert!(insert(&mut bar, &["nightlight", "updates"]));
        let right: Vec<&str> = bar.zones[2].iter().map(|e| e.type_id.as_str()).collect();
        // Pose macOS : le bureau précède les notifications.
        let n = right.iter().position(|i| *i == "desktop").unwrap();
        assert_eq!(&right[n - 2..n], &["nightlight", "updates"]);
        // Déjà là : rien ne change.
        assert!(!insert(&mut bar, &["nightlight"]));
        // Ce qui a déjà été proposé ne l'est plus (même retiré depuis).
        assert_eq!(
            pending("brightness\nclavier\nnuit\n"),
            vec!["removable", "capture", "updates", "mymfa", "gpu", "voice"]
        );
        assert!(pending(&OFFERED.join("\n")).is_empty());
    }

    #[test]
    fn ancienne_pose_par_defaut_passe_a_celle_de_macos() {
        let mut bar = BarSettings::default();
        let e = |id: &str| BarEntry::new(id);
        bar.zones = [
            vec![e("workspaces"), e("window")],
            vec![BarEntry {
                type_id: "clock".into(),
                fields: vec!["format = \"%a %d %b  %H:%M\"".into()],
            }],
            [
                "tray",
                "media",
                "network",
                "bluetooth",
                "battery",
                "volume",
                "updates",
                "gpu",
                "notifications",
                "desktop",
                "session",
                "controls",
            ]
            .map(e)
            .to_vec(),
        ];
        assert!(macos_layout(&mut bar));
        let ids = |z: &[BarEntry]| z.iter().map(|e| e.type_id.clone()).collect::<Vec<_>>();
        // Les menus entrent ensuite, par `menus_module`.
        assert!(bar.zones[0].is_empty());
        assert_eq!(ids(&bar.zones[1]), ["window"]);
        assert_eq!(
            ids(&bar.zones[2]),
            [
                "tray",
                "media",
                "updates",
                "network",
                "bluetooth",
                "battery",
                "volume",
                "gpu",
                "desktop",
                "notifications",
                "controls",
                "clock"
            ]
        );
        // L'horloge garde son format ; une seconde passe ne change rien.
        assert_eq!(bar.zones[2].last().unwrap().fields.len(), 1);
        assert!(!macos_layout(&mut bar));
        // Une barre déjà personnalisée n'est pas touchée.
        let mut custom = BarSettings {
            zones: [vec![e("window")], vec![e("clock")], vec![e("volume")]],
            ..Default::default()
        };
        assert!(!macos_layout(&mut custom));
    }

    #[test]
    fn menus_en_module_une_fois() {
        let e = |id: &str| BarEntry::new(id);
        let ids = |z: &[BarEntry]| z.iter().map(|e| e.type_id.clone()).collect::<Vec<_>>();
        // Continuité, fenêtre à gauche et centre vide : menus en tête, la
        // fenêtre au centre.
        let mut bar = BarSettings {
            style: crate::config::BarStyle::Continuite,
            zones: [vec![e("window")], vec![], vec![e("clock")]],
            ..Default::default()
        };
        assert!(menus_module(&mut bar));
        assert_eq!(ids(&bar.zones[0]), ["menus"]);
        assert_eq!(ids(&bar.zones[1]), ["window"]);
        assert!(!menus_module(&mut bar));
        // Centre occupé, autre style : les menus en tête, rien d'autre ne
        // bouge.
        let mut island = BarSettings {
            style: crate::config::BarStyle::Island,
            zones: [vec![e("workspaces"), e("window")], vec![e("clock")], vec![]],
            ..Default::default()
        };
        assert!(menus_module(&mut island));
        assert_eq!(ids(&island.zones[0]), ["menus", "workspaces", "window"]);
        // Déjà placé ailleurs (ou retiré puis remis) : il y reste.
        let mut placed = BarSettings {
            style: crate::config::BarStyle::Island,
            zones: [vec![], vec![], vec![e("menus")]],
            ..Default::default()
        };
        assert!(!menus_module(&mut placed));
    }

    #[test]
    fn sans_session_ni_bureau_en_bout_de_barre() {
        let mut bar = BarSettings::default();
        bar.zones[2] = vec![BarEntry::new("volume")];
        assert!(insert(&mut bar, &["keyboard"]));
        assert_eq!(bar.zones[2].last().unwrap().type_id, "keyboard");
    }
}
