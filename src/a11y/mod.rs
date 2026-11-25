//! Pont d'accessibilité : les surfaces du démon se décrivent à un lecteur
//! d'écran (Orca) par le protocole AT-SPI 2, servi au-dessus de `zbus`.
//!
//! - [`model`] : l'arbre pur (rôles, états, noms), sans D-Bus ;
//! - [`bus`] : les objets AT-SPI servis sur une connexion donnée, et
//!   l'inscription auprès du registre (`Socket.Embed`).

pub mod bus;
pub mod model;
