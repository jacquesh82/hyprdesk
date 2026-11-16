//! Paquet intégré « Historique » : la configuration de ~/.config/hypr des
//! 30 derniers jours, instantané par instantané, avec retour arrière
//! (`crate::history`, MLG-138). Sorti de « Configuration de Hyprland » pour
//! être trouvé là où on le cherche. La page et ses messages restent ceux de
//! `hyprconfig` (`hyprconfig_history`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use super::PluginInfo;

pub const INFO: PluginInfo = PluginInfo {
    id: "history",
    label: "Historique",
    icon: "document-open-recent",
    summary: "Configuration des 30 derniers jours : ce qui a changé, et retour à une version précédente.",
    builtin: true,
};
