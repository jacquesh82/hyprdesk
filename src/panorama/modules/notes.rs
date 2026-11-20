//! Module Notes : prise de note rapide. La commande réglée (l'éditeur de
//! son choix) est lancée sur le workspace actif de l'écran réglé.

use crate::config::Config;

/// (commande, écran) réglés ; vides s'ils ne le sont pas.
pub fn options(cfg: &Config) -> (String, String) {
    (
        super::option(cfg, "notes", "command"),
        super::option(cfg, "notes", "monitor"),
    )
}

/// Lance la commande sur le workspace actif de l'écran (celui qui a le
/// focus si l'écran est absent ou non réglé).
pub fn open(command: &str, monitor: &str) -> Result<(), String> {
    if command.trim().is_empty() {
        return Err(crate::i18n::tr(
            "Aucune commande : réglez-la dans Réglages → Plugins → Modules.",
        )
        .to_string());
    }
    let ws = crate::hypr::monitors()
        .ok()
        .and_then(|list| {
            list.into_iter().find(|m| m.name == monitor).map(|m| {
                if m.base.is_empty() {
                    m.workspace
                } else {
                    m.base
                }
            })
        })
        .filter(|w| !w.is_empty() && !w.starts_with("special:"));
    match ws {
        Some(ws) => crate::hypr::exec_on(command, &ws),
        None => crate::hypr::exec(command),
    }
    .map_err(|e| e.to_string())
}
