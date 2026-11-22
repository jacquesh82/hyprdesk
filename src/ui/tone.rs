//! Couleurs d'état de l'interface (MLG-234) : ok, attention, danger.
//!
//! Elles viennent de la palette résolue du moteur de thème (slots
//! `success`, `warning`, `danger`), lisibles sur le fond du thème en place,
//! clair comme sombre — plus de couleurs fixes pensées pour un fond sombre.
//! La palette est posée à chaque config installée (calcul immédiat depuis
//! le thème), puis remplacée par celle du pack actif dès que le moteur
//! répond.

use std::collections::BTreeMap;
use std::sync::RwLock;

use iced::Color;

use crate::config::Rgba;

static PALETTE: RwLock<Option<BTreeMap<String, Rgba>>> = RwLock::new(None);

/// Pose la palette des surfaces.
pub fn set(palette: BTreeMap<String, Rgba>) {
    if let Ok(mut p) = PALETTE.write() {
        *p = Some(palette);
    }
}

/// Slot de la palette en place ; à défaut, celui de la palette dérivée du
/// thème par défaut (avant le premier chargement de la config).
pub fn rgba(slot: &str) -> Rgba {
    if let Ok(p) = PALETTE.read()
        && let Some(c) = p.as_ref().and_then(|p| p.get(slot))
    {
        return *c;
    }
    crate::theme_engine::resolve::from_theme(&crate::config::Theme::default())
        .palette
        .get(slot)
        .copied()
        .unwrap_or(Rgba(0xff, 0xff, 0xff, 0xff))
}

fn color(c: Rgba) -> Color {
    Color::from_rgba8(c.0, c.1, c.2, f32::from(c.3) / 255.0)
}

/// Attention (réglage à vérifier, action à confirmer).
pub fn warning() -> Color {
    color(rgba("warning"))
}

/// Réussite, état sain.
pub fn success() -> Color {
    color(rgba("success"))
}

/// Erreur, action destructrice.
pub fn danger() -> Color {
    color(rgba("danger"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn états_de_la_palette_posée() {
        let mut p = BTreeMap::new();
        p.insert("warning".to_string(), Rgba(1, 2, 3, 255));
        set(p);
        assert_eq!(rgba("warning"), Rgba(1, 2, 3, 255));
        // Slot absent : repli sur la palette dérivée du thème par défaut.
        assert_ne!(rgba("danger"), Rgba(0xff, 0xff, 0xff, 0xff));
    }
}
