//! Moteur de thème de hyprdesk (spécification : `docs/THEME-ENGINE.md`).
//!
//! Un pack de thème est un dossier de données (manifeste `theme.toml` +
//! gabarits, jamais de code) : le moteur découvre les packs, résout la
//! variante et l'héritage, calcule la palette en OKLCH, rend les gabarits et
//! applique le résultat aux cibles de façon transactionnelle.
//!
//! Bibliothèque d'abord : le démon l'appelle en cours de processus (Réglages
//! → Thème), le binaire `hyprdesk-theme-engine` en ligne de commande, et la
//! crate cliente `hyprdesk-theme-client` lit la palette publiée. Le module ne
//! dépend ni de `settings`, ni de `ui` : le binaire dédié ne tire ni Lua, ni
//! iced, ni l'index.
//!
//! Pipeline (§2 de la spécification) : résoudre → générer → committer →
//! recharger → rapporter. Chaque étape est dans un sous-module :
//!
//! - [`color`] : maths couleur (OKLCH, WCAG, formats de sortie) ;
//! - [`manifest`] : lecture et validation du manifeste `theme.toml` ;
//! - [`packs`] : découverte (utilisateur, système, embarqué) et héritage ;
//! - [`resolve`] : variante + héritage + slots dérivés → palette complète ;
//! - [`paths`] : emplacements XDG, paramétrables pour les essais.

pub mod color;
pub mod manifest;
pub mod packs;
pub mod paths;
pub mod resolve;

use thiserror::Error;

/// Erreurs du moteur : chaque échec nomme le pack et la cause, sans jamais
/// inventer de valeur manquante.
#[derive(Debug, Error)]
pub enum Error {
    #[error("manifeste du pack {0} : {1}")]
    Manifest(String, String),
    #[error("pack introuvable : {0}")]
    Missing(String),
    #[error("héritage du pack {0} : cyclique ou plus profond que {1} niveaux")]
    Chain(String, usize),
    #[error("couleur « {0} » invalide : {1}")]
    Color(String, String),
    #[error("slot requis absent : {0}")]
    Slot(String),
    #[error("variante inconnue : {0} (attendu : dark ou light)")]
    Variant(String),
    #[error("lecture ou écriture : {0}")]
    Io(#[from] std::io::Error),
    #[error("manifeste illisible : {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
