//! Menus de la barre (HIG MENU-01) : des titres en tête de la zone amont,
//! toujours présents et hors catalogue. « hyprdesk » vient toujours le
//! premier : il rassemble les commandes du shell, qui restent par ailleurs
//! là où elles étaient (titre de la fenêtre, carte des espaces, centre de
//! contrôle). La carte est celle de tous les menus de la barre.

use iced::Task;

use crate::config::MenuEntry;
use crate::control::Command;
use crate::ui::plugins::bar::popover;
use crate::ui::{App, Msg};

pub(in crate::ui) const ABOUT: i64 = 1;
const SETTINGS: i64 = 2;
const LOCK: i64 = 3;
const OVERVIEW: i64 = 4;
const ORGANIZE: i64 = 5;
pub(in crate::ui) const SUSPEND: i64 = 6;
pub(in crate::ui) const REBOOT: i64 = 7;
pub(in crate::ui) const POWER_OFF: i64 = 8;

/// Titres des menus, dans l'ordre de la barre : clef d'ancre de la carte,
/// libellé, contenu demandé. « Fenêtre » et « Espaces » ouvrent les menus
/// qui existent déjà ailleurs (titre de la fenêtre, carte des espaces).
pub(in crate::ui) fn titles() -> Vec<(&'static str, &'static str, popover::Request)> {
    vec![
        ("menus:hyprdesk", "hyprdesk", popover::Request::Shell),
        ("menus:window", "Fenêtre", popover::Request::Window),
        ("menus:workspaces", "Espaces", popover::Request::Workspaces),
    ]
}

/// Entrées du menu « hyprdesk ».
pub(in crate::ui) fn entries() -> Vec<MenuEntry> {
    let tr = |s: &str| crate::i18n::tr(s).to_string();
    let item = |label: &str, icon: &str, confirm: Option<&str>, action: i64| MenuEntry::Item {
        label: tr(label),
        detail: None,
        icon: (!icon.is_empty()).then(|| icon.to_string()),
        checked: None,
        enabled: true,
        action: Some(action),
        confirm: confirm.map(tr),
    };
    // Le groupe de puissance ne porte pas d'icône : hyprdesk n'en a pas de
    // sûre pour la mise en veille, et un groupe en porte à toutes ses lignes
    // ou à aucune.
    vec![
        item("À propos de hyprdesk", "help-about", None, ABOUT),
        MenuEntry::Separator,
        item("Réglages…", "preferences-system-symbolic", None, SETTINGS),
        item("Verrouiller", "system-lock-screen", None, LOCK),
        MenuEntry::Separator,
        item("Espaces de travail", "view-grid-symbolic", None, OVERVIEW),
        item(
            "Organiser les fenêtres…",
            "workspace-switcher",
            None,
            ORGANIZE,
        ),
        MenuEntry::Separator,
        item("Mise en veille…", "", None, SUSPEND),
        item(
            "Redémarrer…",
            "",
            Some("Redémarrer l'ordinateur maintenant ?"),
            REBOOT,
        ),
        item(
            "Éteindre…",
            "",
            Some("Éteindre l'ordinateur maintenant ?"),
            POWER_OFF,
        ),
    ]
}

/// Applique l'entrée `action` du menu « hyprdesk ».
pub(in crate::ui) fn act(app: &App, action: i64) -> Task<Msg> {
    match action {
        ABOUT | SETTINGS => Task::done(Msg::Control(Command::Settings(
            if action == ABOUT { "about" } else { "desktop" }.into(),
        ))),
        LOCK => app.run_from_desktop("hyprdesk lock".into(), false),
        OVERVIEW => Task::done(Msg::Control(Command::Overview)),
        ORGANIZE => Task::done(Msg::BarOrganize),
        SUSPEND | REBOOT | POWER_OFF => Task::perform(power(action), |_| Msg::Noop),
        _ => Task::none(),
    }
}

/// Veille, redémarrage, extinction (`systemctl`), comme l'écran de
/// connexion. Un échec va au journal : la session reste utilisable.
pub(in crate::ui) async fn power(action: i64) {
    let verb = match action {
        SUSPEND => "suspend",
        REBOOT => "reboot",
        POWER_OFF => "poweroff",
        _ => return,
    };
    let r = tokio::task::spawn_blocking(move || {
        std::process::Command::new("systemctl")
            .arg(verb)
            .output()
            .map_err(|e| e.to_string())
            .and_then(|o| {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
                }
            })
    })
    .await;
    if let Err(e) = r {
        tracing::warn!(
            "{}",
            crate::tr!("shell : systemctl {verb} : {e}", verb = verb, e = e)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions(e: &[MenuEntry]) -> Vec<i64> {
        e.iter()
            .filter_map(|x| match x {
                MenuEntry::Item { action, .. } => *action,
                _ => None,
            })
            .collect()
    }

    #[test]
    fn menu_hyprdesk_rassemble_les_commandes_du_shell() {
        let e = entries();
        // À propos d'abord, éteindre en dernier, trois groupes.
        assert_eq!(
            actions(&e),
            [
                ABOUT, SETTINGS, LOCK, OVERVIEW, ORGANIZE, SUSPEND, REBOOT, POWER_OFF
            ]
        );
        assert_eq!(
            e.iter()
                .filter(|x| matches!(x, MenuEntry::Separator))
                .count(),
            3
        );
        for (action, confirm, icon, enabled) in e.iter().filter_map(|x| match x {
            MenuEntry::Item {
                action: Some(a),
                confirm,
                icon,
                enabled,
                ..
            } => Some((*a, confirm.is_some(), icon.is_some(), *enabled)),
            _ => None,
        }) {
            assert!(enabled);
            // Seules les commandes qui perdent la session demandent confirmation.
            assert_eq!(confirm, matches!(action, REBOOT | POWER_OFF));
            // Un groupe porte des icônes à toutes ses lignes ou à aucune : le
            // groupe de puissance n'en a pas.
            assert_eq!(icon, !matches!(action, SUSPEND | REBOOT | POWER_OFF));
        }
    }

    #[test]
    fn titres_dans_l_ordre_de_la_barre() {
        let t = titles();
        assert_eq!(
            t.iter().map(|x| x.1).collect::<Vec<_>>(),
            ["hyprdesk", "Fenêtre", "Espaces"]
        );
        // « hyprdesk » d'abord ; puis les menus de la fenêtre et des espaces,
        // tels qu'ils existent déjà ailleurs dans la barre.
        assert!(matches!(
            t.iter().map(|x| &x.2).collect::<Vec<_>>().as_slice(),
            [
                popover::Request::Shell,
                popover::Request::Window,
                popover::Request::Workspaces
            ]
        ));
        // La clef d'ancre ne peut pas rencontrer celle d'un module.
        let keys: Vec<_> = t.iter().map(|x| x.0).collect();
        assert!(keys.iter().all(|k| k.starts_with("menus:")));
        assert_eq!(
            keys.len(),
            keys.iter().collect::<std::collections::HashSet<_>>().len()
        );
    }
}
