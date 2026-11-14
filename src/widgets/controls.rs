//! Centre de contrôle : widget intégré composite, sur le bureau ou dans la barre.
use super::Builtin;
use crate::config::Anchor;

pub const DESCRIPTOR: Builtin = Builtin {
    id: "controls",
    name: "Centre de contrôle",
    description: "Wi-Fi, Bluetooth, son, luminosité et affichage du bureau.",
    options: &[],
    interval: 2.0,
    anchor: Anchor::TopRight,
    toggleable: true,
};

/// Sources partagées : une seule instance et un seul relevé par source.
pub const SOURCES: &[&str] = &["reseau", "bluetooth", "batterie"];
