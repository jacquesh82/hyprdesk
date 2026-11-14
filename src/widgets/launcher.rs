//! Présentation du plugin Lanceur comme widget de barre ou du bureau.
use super::Builtin;
use crate::config::Anchor;

pub const DESCRIPTOR: Builtin = Builtin {
    id: "launcher",
    name: "Lanceur d’applications",
    description: "Applications épinglées et ouvertes, recherche et vue des tâches.",
    options: &[],
    interval: 60.0,
    anchor: Anchor::Bottom,
    toggleable: true,
};
