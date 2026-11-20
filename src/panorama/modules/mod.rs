//! Modules du plugin Panorama : ce que la bande du profil Raccourcis (et
//! les compositions) peuvent montrer. Ici la logique, sans interface ; les
//! tuiles sont dans `ui::plugins::panorama::surface::modules`, leurs réglages
//! dans Réglages → Panorama → Modules (`ui::plugins::panorama::modules`).
//!
//! Options des modules : `plugin_options.panorama`, clés `<module>.<option>`
//! (`hue.hidden`, `notes.command`, `notes.monitor`).

pub mod hue;
pub mod notes;

use crate::config::Config;
use crate::tiling::OptValue;

/// Modules proposés dans la bande du profil Raccourcis : (identifiant du
/// widget, libellé, glyphe intégré de `ui::icons`). Vidéo et musique y entrent par leurs widgets
/// (`playlist` : plateformes, `media` : lecture en cours).
pub const CATALOGUE: &[(&str, &str, &str)] = &[
    ("notes", "Notes", "accessories-text-editor"),
    ("hue", "Lumières", "lightbulb"),
    ("voice", "LYRA vocale", "audio-input-microphone"),
    ("media", "Musique", "audio-volume-high"),
    ("playlist", "Plateformes vidéo", "video-display"),
    ("clock", "Horloge", "preferences-system-time"),
    ("calendar", "Calendrier", "x-office-calendar"),
    ("system", "Système", "computer"),
];

/// Plugin qui porte les options des modules.
pub const PLUGIN: &str = "panorama";

/// Option d'un module (`hue.hidden`…) ; à défaut, l'ancienne clé de
/// l'époque où le module était un paquet à part (`plugin_options.hue`).
pub fn option(cfg: &Config, module: &str, key: &str) -> String {
    let o = &cfg.general.plugin_options;
    o.get(PLUGIN)
        .and_then(|m| m.get(&format!("{module}.{key}")))
        .or_else(|| o.get(module).and_then(|m| m.get(key)))
        .map(OptValue::display)
        .unwrap_or_default()
}

/// Options après réglage d'un module : la nouvelle clé posée (ou retirée si
/// vide), l'ancienne retirée.
pub fn with_option(
    mut all: std::collections::BTreeMap<String, std::collections::BTreeMap<String, OptValue>>,
    module: &str,
    key: &str,
    value: &str,
) -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, OptValue>> {
    if let Some(old) = all.get_mut(module) {
        old.remove(key);
    }
    let m = all.entry(PLUGIN.to_string()).or_default();
    let k = format!("{module}.{key}");
    if value.trim().is_empty() {
        m.remove(&k);
    } else {
        m.insert(k, OptValue::Str(value.trim().to_string()));
    }
    all.retain(|_, m| !m.is_empty());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_et_ancienne_cle() {
        let mut cfg = Config::default();
        let o = &mut cfg.general.plugin_options;
        o.entry("hue".into())
            .or_default()
            .insert("hidden".into(), OptValue::Str("3,4".into()));
        assert_eq!(option(&cfg, "hue", "hidden"), "3,4");
        let all = with_option(cfg.general.plugin_options.clone(), "hue", "hidden", "4");
        assert!(!all.contains_key("hue"), "ancienne clé retirée");
        cfg.general.plugin_options = all;
        assert_eq!(option(&cfg, "hue", "hidden"), "4");
        let all = with_option(cfg.general.plugin_options.clone(), "hue", "hidden", "");
        assert!(all.is_empty());
    }
}
