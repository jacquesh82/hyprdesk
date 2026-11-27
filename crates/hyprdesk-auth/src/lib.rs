//! hyprdesk-auth : authentification de hyprdesk, en crate à part pour que
//! ce code sensible ne dépende que du strict nécessaire (libpam, D-Bus pour
//! fprintd, la langue) : ni iced, ni Lua, ni le reste du bureau.
//!
//! - `pam` : liaison PAM écrite à la main (mot de passe, empreinte, phase
//!   account), journal, effacement des secrets ;
//! - `fprint` : vérification d'empreinte par fprintd ;
//! - `faillock` : état du blocage du compte après des échecs (lecture seule) ;
//! - `enroll` : liste, enregistrement et suppression des empreintes.
//!
//! Utilisé par l'écran de verrouillage (`hyprdesk-lock`), l'écran de
//! connexion (`hyprdesk-greeter`), l'agent polkit (`hyprdesk-polkit`) et les
//! Réglages.

// `crate::tr!` et `crate::i18n` dans les modules, comme dans hyprdesk.
pub(crate) use hyprdesk_i18n as i18n;
pub(crate) use hyprdesk_i18n::tr;

pub mod enroll;
pub mod faillock;
pub mod fprint;
pub mod pam;
