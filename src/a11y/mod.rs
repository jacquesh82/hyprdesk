//! Pont d'accessibilité : les surfaces du démon se décrivent à un lecteur
//! d'écran (Orca) par le protocole AT-SPI 2, servi au-dessus de `zbus`.
//!
//! - [`model`] : l'arbre pur (rôles, états, noms), sans D-Bus ;
//! - [`bus`] : les objets AT-SPI servis sur une connexion donnée, et
//!   l'inscription auprès du registre (`Socket.Embed`) ;
//! - [`daemon`] : le démarrage dans le démon, seulement si un lecteur d'écran
//!   le demande (`org.a11y.Status`).

pub mod bus;
pub mod daemon;
pub mod model;

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use model::{Node, Role};

/// Le pont est servi : l'interface doit lui publier son arbre.
static SERVED: AtomicBool = AtomicBool::new(false);

/// L'interface calcule son arbre seulement quand quelqu'un le lit : aucun
/// coût sans lecteur d'écran.
pub fn wanted() -> bool {
    SERVED.load(Ordering::Relaxed)
}

pub(crate) fn set_served(on: bool) {
    SERVED.store(on, Ordering::Relaxed);
}

/// L'application : racine de l'arbre, au-dessus des surfaces.
pub fn root(surfaces: Vec<Node>) -> Node {
    Node::new(Role::Application, "hyprdesk").children(surfaces)
}

/// Dernier arbre publié par l'interface.
fn latest() -> &'static tokio::sync::watch::Sender<Node> {
    static TX: OnceLock<tokio::sync::watch::Sender<Node>> = OnceLock::new();
    TX.get_or_init(|| tokio::sync::watch::channel(root(Vec::new())).0)
}

/// Publie les surfaces de l'interface ; le pont n'est réveillé que si
/// l'arbre a changé.
pub fn publish(surfaces: Vec<Node>) {
    let tree = root(surfaces);
    latest().send_if_modified(|cur| {
        if *cur == tree {
            false
        } else {
            *cur = tree;
            true
        }
    });
}

pub(crate) fn subscribe() -> tokio::sync::watch::Receiver<Node> {
    latest().subscribe()
}

/// L'arbre publié, en texte (`hyprdesk a11y dump`).
pub fn dump() -> String {
    latest().borrow().dump()
}

/// Réveille la commande `a11y dump` quand l'interface a publié l'arbre.
pub fn dumped() -> &'static tokio::sync::Notify {
    static N: OnceLock<tokio::sync::Notify> = OnceLock::new();
    N.get_or_init(tokio::sync::Notify::new)
}
