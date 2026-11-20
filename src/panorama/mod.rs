//! Panorama : widget plein écran tactile des écrans « bandeau » (MLG-164).
//!
//! Un écran très allongé (2560×720, 1920×480, 1280×400…, en paysage comme en
//! portrait) reçoit le rôle « panorama » : hyprdesk y pose une surface à pages
//! qu'on fait glisser au doigt (Média, Système, Horloge…), sans jamais prendre
//! le focus clavier. Aucun modèle d'écran n'est codé en dur : la détection
//! part du rapport d'aspect et des périphériques tactiles (`detect`).
//!
//! Ce module est la logique pure (détection, mise en page, configuration) ;
//! le rendu vit dans `ui::plugins::panorama::surface`, les réglages dans
//! `ui::plugins::panorama`.

pub mod auth;
pub mod autofull;
pub mod capture;
pub mod config;
pub mod design;
pub mod detect;
pub mod icons;
pub mod layout;
pub mod modules;
pub mod radio;
pub mod radio_play;
pub mod tasks;
pub mod telemetry;
pub mod video;

pub use config::Config;

/// Namespace layer-shell de la surface (règles de calque de Hyprland).
pub const NAMESPACE: &str = "hyprdesk-panorama";

/// Widgets connus (pages et zones), dans l'ordre par défaut : (identifiant, libellé).
pub const PAGES: &[(&str, &str)] = &[
    ("media", "Média"),
    ("system", "Système"),
    ("clock", "Horloge"),
    ("video", "Vidéo"),
    ("calendar", "Calendrier"),
    ("shortcuts", "Raccourcis"),
    ("hue", "Lumières"),
    ("notes", "Notes"),
    ("voice", "LYRA vocale"),
];

/// Libellé d'une page (« Page » si elle est inconnue).
pub fn page_label(id: &str) -> &'static str {
    if let Some((m, _)) = design::metric_of(id) {
        return design::metric_label(m);
    }
    if id == "playlist" {
        return "Playlist";
    }
    PAGES
        .iter()
        .find(|(p, _)| *p == id)
        .map(|(_, l)| *l)
        .unwrap_or("Page")
}

/// Rattache le tactile à l'écran Panorama dans `inputs.lua` (à chaud puis
/// écrit) ; `None` le rend à Hyprland (choix automatique). Bloquant.
pub fn map_touch(config_path: &std::path::Path, output: Option<&str>) -> Result<(), String> {
    let mut i = crate::inputs::read();
    let want = output.unwrap_or("").to_string();
    if i.touch_output == want {
        return Ok(());
    }
    let cleared = want.is_empty();
    i.touch_output = want;
    crate::inputs::apply(&i)?;
    // Une sortie retirée n'est plus écrite : la rendre aussi à chaud.
    if cleared {
        let _ = hyprpanel_core::ipc::eval(
            "hl.config({ input = { touchdevice = { output = \"[[Auto]]\" } } })",
        );
    }
    crate::inputs::save(&crate::inputs::inputs_path(config_path), &i).map_err(|e| e.to_string())
}

/// Écran Panorama que le tactile doit suivre : celui du même fabricant que
/// le périphérique tactile, sinon le premier.
pub fn touch_target(
    panoramas: &[String],
    screens: &[detect::Screen],
    touch: &[String],
) -> Option<String> {
    panoramas
        .iter()
        .find(|p| {
            screens
                .iter()
                .find(|s| &s.name == *p)
                .is_some_and(|s| touch.iter().any(|d| detect::same_maker(s, d)))
        })
        .or_else(|| panoramas.first())
        .cloned()
}

/// Faut-il rattacher le tactile à `target` ? Oui s'il n'est rattaché à
/// rien (Hyprland le pose alors sur l'écran qui a le focus) ou à un
/// connecteur absent : un écran peut changer de nom d'un démarrage à l'autre
/// (HDMI-A-3 devenu HDMI-A-1), et Hyprland met alors aussi le toucher sur
/// l'écran qui a le focus. Un écran branché choisi ailleurs reste en place.
pub fn touch_stale(current: &str, target: &str, connected: &[String]) -> bool {
    current != target && (current.is_empty() || !connected.iter().any(|c| c == current))
}

/// Remet le tactile sur l'écran Panorama si son rattachement est périmé
/// (`touch_stale`). Bloquant. Rend vrai s'il a été déplacé.
pub fn keep_touch(
    config_path: &std::path::Path,
    target: &str,
    connected: &[String],
) -> Result<bool, String> {
    let current = crate::inputs::read().touch_output;
    if !touch_stale(&current, target, connected) {
        return Ok(false);
    }
    map_touch(config_path, Some(target)).map(|()| true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_follows_renamed_connector() {
        let on = vec!["DP-1".to_string(), "HDMI-A-1".to_string()];
        // Connecteur d'hier disparu : le tactile part sur le Panorama.
        assert!(touch_stale("HDMI-A-3", "HDMI-A-1", &on));
        // Aucun rattachement : le focus déciderait, on rattache.
        assert!(touch_stale("", "HDMI-A-1", &on));
        // Déjà en place, ou écran branché choisi ailleurs : on n'y touche pas.
        assert!(!touch_stale("HDMI-A-1", "HDMI-A-1", &on));
        assert!(!touch_stale("DP-1", "HDMI-A-1", &on));
    }
}
