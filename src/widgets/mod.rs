//! Widgets, en architecture « plug-n-play » :
//!
//! - les types INTÉGRÉS sont déclarés dans [`BUILTINS`] (un module par source
//!   de données : [`system`], [`media`], [`command`]) ;
//! - les PLUGINS sont des fichiers Lua déposés dans un dossier de widgets
//!   ([`plugins`]) : détectés au chargement de la config, listés dans les
//!   réglages, sans recompiler.
//!
//! Les données sont relevées hors du thread UI, et seulement quand le bureau
//! est visible ; l'affichage vit dans `ui::widgets`.

pub mod command;
pub mod controls;
pub mod launcher;
pub mod media;
pub mod plugins;
pub mod system;

pub use command::run_command;
pub use media::{media, media_control, media_raise};
pub use system::sample_system;

use crate::config::{Anchor, WidgetSource, WidgetType};

#[derive(Debug, Clone, PartialEq)]
pub struct SysLine {
    pub label: String,
    pub value: String,
    /// Remplissage 0..1 pour une jauge, si la mesure s'y prête.
    pub ratio: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaInfo {
    /// Nom de bus (`org.mpris.MediaPlayer2.spotify`), pour les commandes.
    pub bus: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    /// Pochette (`mpris:artUrl`), fichier local seulement (MLG-279).
    pub art: Option<std::path::PathBuf>,
    /// Teinte moyenne de la pochette.
    pub tint: Option<crate::config::Rgba>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    /// Rendu riche d'un widget Lua : icône, texte, info-bulle, état.
    Rich(crate::config::Rendered),
    Sys(Vec<SysLine>),
    Media(Option<MediaInfo>),
    Error(String),
}

/// Type de widget intégré : ses métadonnées et les options qu'il accepte.
#[derive(Debug, Clone, Copy)]
pub struct Builtin {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// Options propres, en plus des options communes (placement, intervalle…).
    pub options: &'static [&'static str],
    /// Intervalle de relevé par défaut, en secondes.
    pub interval: f64,
    pub anchor: Anchor,
    /// Activable d'un clic : aucune option obligatoire.
    pub toggleable: bool,
}

/// Options communes à tous les widgets, intégrés comme plugins.
pub const COMMON_OPTIONS: &[&str] = &[
    "workspace",
    "monitor",
    "screen",
    "if_missing",
    "id",
    "anchor",
    "x",
    "y",
    "size",
    "interval",
    "card",
];

/// Registre des widgets intégrés.
pub const BUILTINS: &[Builtin] = &[
    controls::DESCRIPTOR,
    launcher::DESCRIPTOR,
    Builtin {
        id: "clock",
        name: "Horloge",
        description: "Afficher l'heure ou la date, au format strftime.",
        options: &["format", "family"],
        interval: 1.0,
        anchor: Anchor::TopRight,
        toggleable: true,
    },
    Builtin {
        id: "calendar",
        name: "Calendrier",
        description: "Parcourir le mois en cours, aujourd'hui en évidence.",
        options: &["family"],
        interval: 60.0,
        anchor: Anchor::Right,
        toggleable: true,
    },
    Builtin {
        id: "system",
        name: "Système",
        description: "Surveiller le processeur, la mémoire, les disques, la charge et la durée d'allumage.",
        options: &["show", "family"],
        interval: 2.0,
        anchor: Anchor::BottomRight,
        toggleable: true,
    },
    Builtin {
        id: "media",
        name: "Lecteur média",
        description: "Afficher le titre en cours et commander le lecteur (MPRIS).",
        options: &["player", "family"],
        interval: 1.0,
        anchor: Anchor::BottomLeft,
        toggleable: true,
    },
    Builtin {
        id: "command",
        name: "Commande",
        description: "Afficher la sortie d'une commande shell (option exec, dans hyprdesk.lua).",
        options: &["exec", "timeout"],
        interval: 60.0,
        anchor: Anchor::TopLeft,
        toggleable: false,
    },
    Builtin {
        id: "lua",
        name: "Fonction Lua",
        description: "Afficher un texte rendu par une fonction Lua (option render, dans hyprdesk.lua).",
        options: &["render", "on_click", "on_scroll", "menu", "on_toggle"],
        interval: 5.0,
        anchor: Anchor::Left,
        toggleable: false,
    },
];

pub fn builtin(id: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|b| b.id == id)
}

pub fn builtin_types() -> Vec<WidgetType> {
    BUILTINS
        .iter()
        .map(|b| WidgetType {
            id: b.id.into(),
            name: b.name.into(),
            description: b.description.into(),
            source: WidgetSource::Builtin,
            anchor: b.anchor,
            toggleable: b.toggleable,
            error: None,
            options: Vec::new(),
        })
        .collect()
}
