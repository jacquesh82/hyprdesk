//! Panorama : widget plein écran tactile des écrans « bandeau » (MLG-164).
//!
//! Un écran très allongé (2560×720, 1920×480, 1280×400…, en paysage comme en
//! portrait) reçoit le rôle « panorama » : hyprdesk y pose une surface à pages
//! qu'on fait glisser au doigt (Média, Système, Horloge…), sans jamais prendre
//! le focus clavier. Aucun modèle d'écran n'est codé en dur : la détection
//! part du rapport d'aspect et des périphériques tactiles (`detect`).
//!
//! Ce module est la logique pure (détection, mise en page, configuration) ;
//! le rendu vit dans `ui::panorama`, les réglages dans `ui::plugins::panorama`.

pub mod config;
pub mod detect;
pub mod layout;
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
];

/// Libellé d'une page (« Page » si elle est inconnue).
pub fn page_label(id: &str) -> &'static str {
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
