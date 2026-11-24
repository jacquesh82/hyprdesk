//! Menus de la barre (HIG MENU-01) : des titres en tête de la zone amont,
//! toujours présents et hors catalogue. « hyprdesk » vient toujours le
//! premier : il rassemble les commandes du shell, qui restent par ailleurs
//! là où elles étaient (titre de la fenêtre, carte des espaces, centre de
//! contrôle). « Aide » vient en dernier (HIG MENU-05). La carte est celle de
//! tous les menus de la barre.

use std::path::Path;

use iced::Task;

use crate::config::{IconDef, IconKind, MenuEntry};
use crate::control::Command;
use crate::ui::context;
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
const TRASH: i64 = 9;
// Menu « Aide » (HIG MENU-05), à la suite de celles de « hyprdesk ».
pub(in crate::ui) const HELP_SEARCH: i64 = 10;
const HELP_TOUR: i64 = 11;
const HELP_KEYS: i64 = 12;
const HELP_DOCS: i64 = 13;
// Menus contextuels des modules de barre (HIG ENT-07), à la suite.
const MODULE_MUTE: i64 = 20;
const MODULE_SOUND: i64 = 21;
const MODULE_ENERGY: i64 = 22;
const MODULE_DATETIME: i64 = 23;
const MODULE_READ: i64 = 24;
const MODULE_DND: i64 = 25;

/// Clé du module de volume, du module de luminosité (ancre de leur carte) ;
/// type au catalogue du widget de notifications.
const VOLUME: &str = "volume";
const BRIGHTNESS: &str = "brightness";
const NOTIFICATIONS: &str = "notifications";

/// Dossier des documents du shell, tel que le paquet l'installe.
const GUIDE_DIR: &str = "/usr/share/doc/hyprdesk";

/// Titres des menus, dans l'ordre de la barre : clef d'ancre de la carte,
/// libellé, contenu demandé. « Fenêtre » et « Espaces » ouvrent les menus
/// qui existent déjà ailleurs (titre de la fenêtre, carte des espaces) ;
/// « Aide » ferme la barre (HIG MENU-05).
pub(in crate::ui) fn titles() -> Vec<(&'static str, &'static str, popover::Request)> {
    vec![
        ("menus:hyprdesk", "hyprdesk", popover::Request::Shell),
        ("menus:window", "Fenêtre", popover::Request::Window),
        ("menus:workspaces", "Espaces", popover::Request::Workspaces),
        ("menus:help", "Aide", popover::Request::Help),
    ]
}

/// Entrées du menu « Aide » (HIG MENU-05) : le champ de recherche en tête —
/// la saisie validée ouvre la recherche des Réglages sur ces mots —, puis
/// l'aide du shell. « Documentation » ouvre le guide installé par le
/// paquet ; sans lui, elle reste affichée et inactive : jamais un lien vers
/// un emplacement qui n'existe pas.
pub(in crate::ui) fn help_entries() -> Vec<MenuEntry> {
    let tr = |s: &str| crate::i18n::tr(s).to_string();
    let item = |label: &str, icon: &str, enabled: bool, action: i64| MenuEntry::Item {
        label: tr(label),
        detail: None,
        icon: (!icon.is_empty()).then(|| icon.to_string()),
        checked: None,
        enabled,
        action: Some(action),
        confirm: None,
    };
    vec![
        MenuEntry::Input {
            label: None,
            placeholder: tr("Rechercher dans les réglages"),
            secret: false,
            action: Some(HELP_SEARCH),
        },
        MenuEntry::Separator,
        item("Visite guidée…", "", true, HELP_TOUR),
        item("Raccourcis clavier", "", true, HELP_KEYS),
        item("Documentation", "", guide().is_some(), HELP_DOCS),
        MenuEntry::Separator,
        item("À propos de hyprdesk", "help-about", true, ABOUT),
    ]
}

/// Guide du shell installé, dans la langue de l'interface ; l'autre langue
/// à défaut.
fn guide_in(dir: &Path, lang: crate::i18n::Lang) -> Option<std::path::PathBuf> {
    let names = match lang {
        crate::i18n::Lang::En => ["GUIDE.md", "GUIDE.fr.md"],
        crate::i18n::Lang::Fr => ["GUIDE.fr.md", "GUIDE.md"],
    };
    names.iter().map(|n| dir.join(n)).find(|p| p.is_file())
}

fn guide() -> Option<std::path::PathBuf> {
    guide_in(Path::new(GUIDE_DIR), crate::i18n::current())
}

/// Mots à chercher dans les Réglages pour la saisie `text` du champ d'aide :
/// rien sur une saisie vide.
fn help_search(text: &str) -> Option<Command> {
    let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!words.is_empty()).then(|| Command::Settings(format!("?{words}")))
}

/// Champ validé du menu « Aide » : `action` reçoit le texte.
pub(in crate::ui) fn submit(action: i64, text: &str) -> Task<Msg> {
    match action {
        HELP_SEARCH => help_search(text).map_or_else(Task::none, |c| Task::done(Msg::Control(c))),
        _ => Task::none(),
    }
}

/// Entrées du menu « hyprdesk ». `selected` : l'icône du bureau sélectionnée
/// (clic ou flèches) quand c'est un fichier ou un dossier ; « Mettre à la
/// corbeille » la vise. L'entrée reste affichée, inactive, sans elle : une
/// commande du clic droit a sa place dans la barre (HIG MENU-04).
pub(in crate::ui) fn entries(selected: Option<&IconDef>) -> Vec<MenuEntry> {
    let tr = |s: &str| crate::i18n::tr(s).to_string();
    let trashed = selected.and_then(|d| match &d.kind {
        IconKind::Path(p) => Some(d.label.clone().unwrap_or_else(|| file_name(p))),
        _ => None,
    });
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
        MenuEntry::Item {
            label: tr("Mettre à la corbeille"),
            detail: None,
            icon: Some("user-trash-symbolic".into()),
            checked: None,
            enabled: trashed.is_some(),
            action: Some(TRASH),
            confirm: trashed
                .map(|name| crate::tr!("Mettre à la corbeille « {name} » ?", name = name)),
        },
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
        // Le fichier ou le dossier de l'icône sélectionnée, lu à l'action :
        // celui que la confirmation a nommé.
        TRASH => {
            let Some(IconKind::Path(p)) = app.selected_file().map(|d| &d.kind) else {
                return Task::none();
            };
            let p = p.clone();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || context::trash(&p))
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                },
                |r| match r {
                    Ok(()) => Msg::ConfigChanged,
                    Err(e) => {
                        crate::hypr::notify(
                            0,
                            5000,
                            &crate::tr!("hyprdesk : mettre à la corbeille — {e}", e = e),
                        );
                        Msg::Noop
                    }
                },
            )
        }
        // Menu « Aide » : la visite ouvre les Réglages et s'y lance, les
        // raccourcis en sont une page. Le guide est relu à l'action : sans
        // lui, rien ne se passe.
        HELP_TOUR => Task::done(Msg::Control(Command::Tour)),
        HELP_KEYS => Task::done(Msg::Control(Command::Settings("keys".into()))),
        HELP_DOCS => {
            if let Some(p) = guide() {
                let _ = crate::launch::open_path(&p);
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

/// Entrées du menu contextuel d'un module de barre (HIG ENT-07). `module` :
/// la clé du module. Deux commandes au plus, toutes prises ailleurs dans
/// l'interface — le clic milieu, les Réglages, le centre de contrôle (HIG
/// MENU-04 : jamais un menu contextuel seule voie). Hors des modules qui ont
/// un menu (volume, luminosité, horloge, calendrier, widget de
/// notifications) : rien, la carte n'ouvre pas.
pub(in crate::ui) fn module_entries(app: &App, module: &str) -> Vec<MenuEntry> {
    let tr = |s: &str| crate::i18n::tr(s).to_string();
    let item = |label: &str, icon: &str, action: i64| MenuEntry::Item {
        label: tr(label),
        detail: None,
        icon: (!icon.is_empty()).then(|| icon.to_string()),
        checked: None,
        enabled: true,
        action: Some(action),
        confirm: None,
    };
    let muted = app.volume.is_some_and(|(_, m)| m);
    let widget = app.cfg.widgets.iter().find(|w| w.id == module);
    match module {
        VOLUME => vec![
            item(
                if muted {
                    "Rétablir le son"
                } else {
                    "Couper le son"
                },
                super::surface::volume_icon(app.volume),
                MODULE_MUTE,
            ),
            MenuEntry::Separator,
            item(
                "Réglages du son…",
                "preferences-system-symbolic",
                MODULE_SOUND,
            ),
        ],
        BRIGHTNESS => vec![item(
            "Réglages d'énergie…",
            "preferences-system-symbolic",
            MODULE_ENERGY,
        )],
        // Horloge, calendrier, widget de notifications : des widgets, leur
        // clé est l'id.
        _ => match widget {
            // Sans non-lue, « Tout marquer comme lu » se retire ; la coche
            // marque le Silence en cours.
            Some(w) if w.type_id == NOTIFICATIONS => {
                let mut e = Vec::new();
                if app.toasts.unread() > 0 {
                    e.push(item(
                        "Tout marquer comme lu",
                        "preferences-system-notifications-symbolic",
                        MODULE_READ,
                    ));
                    e.push(MenuEntry::Separator);
                }
                e.push(MenuEntry::Item {
                    label: tr("Ne pas déranger"),
                    detail: None,
                    icon: Some(
                        if app.toasts.dnd {
                            "notifications-disabled-symbolic"
                        } else {
                            "preferences-system-notifications-symbolic"
                        }
                        .into(),
                    ),
                    checked: Some(app.toasts.dnd),
                    enabled: true,
                    action: Some(MODULE_DND),
                    confirm: None,
                });
                e
            }
            Some(w)
                if matches!(
                    w.kind,
                    crate::config::WidgetKind::Clock { .. } | crate::config::WidgetKind::Calendar
                ) =>
            {
                vec![item(
                    "Réglages de date et d'heure…",
                    "preferences-system-symbolic",
                    MODULE_DATETIME,
                )]
            }
            _ => Vec::new(),
        },
    }
}

/// Applique l'entrée `action` du menu contextuel d'un module de barre.
pub(in crate::ui) fn module_act(app: &App, action: i64) -> Task<Msg> {
    let settings = |page: &str| Task::done(Msg::Control(Command::Settings(page.into())));
    match action {
        // Le clic milieu du module.
        MODULE_MUTE => Task::done(Msg::VolumeMute),
        MODULE_SOUND => settings("son"),
        MODULE_ENERGY => settings("energie"),
        MODULE_DATETIME => settings("date-heure"),
        MODULE_READ => Task::done(Msg::Toasts(crate::ui::toasts::TMsg::Read)),
        // La bascule du centre de contrôle : le Silence en cours se coupe ;
        // sans lui, sa fin se choisit dans sa propre carte.
        MODULE_DND => {
            if app.toasts.dnd {
                Task::done(Msg::Control(Command::Notifications("dnd off".into())))
            } else {
                app.popover.as_ref().map_or_else(Task::none, |p| {
                    Task::done(Msg::OpenPopover(
                        p.bar,
                        "silence".into(),
                        popover::Request::Silence,
                    ))
                })
            }
        }
        _ => Task::none(),
    }
}

/// Nom affiché d'un fichier ou d'un dossier de bureau : celui de son icône.
fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
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

    fn libelles(e: &[MenuEntry]) -> Vec<String> {
        e.iter()
            .filter_map(|x| match x {
                MenuEntry::Item { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect()
    }

    fn icone(e: &[MenuEntry], i: usize) -> Option<String> {
        match &e[i] {
            MenuEntry::Item { icon, .. } => icon.clone(),
            _ => None,
        }
    }

    /// Le menu contextuel du volume (HIG ENT-07) suit l'état du son — «
    /// Rétablir le son » coupé, « Couper le son » sinon, l'icône du module —
    /// puis mène aux Réglages du son. Luminosité, horloge et calendrier :
    /// leurs Réglages, une seule entrée. Tout autre module : rien, la carte
    /// n'ouvre pas.
    #[test]
    fn menu_du_module_de_volume_selon_l_etat() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-menus-tests/config.lua"),
            true,
        )
        .0;
        let tr = |s: &str| crate::i18n::tr(s).to_string();
        app.volume = Some((0.8, false));
        let e = module_entries(&app, VOLUME);
        assert_eq!(actions(&e), [MODULE_MUTE, MODULE_SOUND]);
        assert_eq!(libelles(&e), [tr("Couper le son"), tr("Réglages du son…")]);
        assert_eq!(icone(&e, 0).as_deref(), Some("audio-volume-high-symbolic"));
        // Le groupe des Réglages porte des icônes à toutes ses lignes.
        assert!(
            e.iter()
                .all(|x| !matches!(x, MenuEntry::Item { icon: None, .. }))
        );
        app.volume = Some((0.8, true));
        let e = module_entries(&app, VOLUME);
        assert_eq!(libelles(&e)[0], tr("Rétablir le son"));
        assert_eq!(icone(&e, 0).as_deref(), Some("audio-volume-muted-symbolic"));
        // Chaque entrée fait ce que fait la commande dont elle vient.
        assert_eq!(
            module_entries(&app, BRIGHTNESS)
                .iter()
                .filter_map(|x| match x {
                    MenuEntry::Item { label, .. } => Some(label.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [tr("Réglages d'énergie…").as_str()]
        );
        for kind in [
            crate::config::WidgetKind::Calendar,
            crate::config::WidgetKind::Clock {
                format: String::new(),
            },
        ] {
            std::sync::Arc::make_mut(&mut app.cfg)
                .widgets
                .push(crate::config::WidgetDef {
                    origin: Default::default(),
                    sel: Default::default(),
                    id: "horloge".into(),
                    type_id: "clock".into(),
                    kind,
                    anchor: Default::default(),
                    x: 0.0,
                    y: 0.0,
                    size: 13.0,
                    interval: std::time::Duration::from_secs(1),
                    card: false,
                    in_bar: true,
                    screen: Default::default(),
                    if_missing: Default::default(),
                });
            assert_eq!(
                libelles(&module_entries(&app, "horloge")),
                [tr("Réglages de date et d'heure…")]
            );
            std::sync::Arc::make_mut(&mut app.cfg).widgets.clear();
        }
        // Widget de notifications (HIG ENT-07) : sans non-lue, « Tout marquer
        // comme lu » se retire ; « Ne pas déranger » bascule, sa coche marque
        // le Silence en cours.
        std::sync::Arc::make_mut(&mut app.cfg)
            .widgets
            .push(crate::config::WidgetDef {
                origin: Default::default(),
                sel: Default::default(),
                id: "notif".into(),
                type_id: NOTIFICATIONS.into(),
                kind: crate::config::WidgetKind::Lua {
                    func: 0,
                    click: None,
                    scroll: None,
                    menu: None,
                    toggle: None,
                },
                anchor: Default::default(),
                x: 0.0,
                y: 0.0,
                size: 13.0,
                interval: std::time::Duration::from_secs(1),
                card: false,
                in_bar: true,
                screen: Default::default(),
                if_missing: Default::default(),
            });
        assert_eq!(
            libelles(&module_entries(&app, "notif")),
            [tr("Ne pas déranger")],
            "sans non-lue"
        );
        app.toasts.history = vec![crate::notifications::build(
            1,
            "Signal",
            "",
            "s",
            "",
            &[],
            &std::collections::HashMap::new(),
            -1,
        )];
        let e = module_entries(&app, "notif");
        assert_eq!(actions(&e), [MODULE_READ, MODULE_DND]);
        let dermier = |e: &[MenuEntry]| match e.last() {
            Some(MenuEntry::Item { label, checked, .. }) => (label.clone(), *checked),
            other => panic!("pas une entrée : {other:?}"),
        };
        assert_eq!(
            dermier(&e),
            (tr("Ne pas déranger"), Some(false)),
            "en dernier, sans Silence"
        );
        app.toasts.dnd = true;
        let e = module_entries(&app, "notif");
        assert_eq!(dermier(&e).1, Some(true), "la coche marque le Silence");
        app.toasts.history.clear();
        app.toasts.dnd = false;
        assert!(
            module_entries(&app, "tray:1").is_empty(),
            "hors d'un module à menu : rien"
        );
    }

    #[test]
    fn menu_hyprdesk_rassemble_les_commandes_du_shell() {
        let e = entries(None);
        // À propos d'abord, éteindre en dernier, quatre groupes.
        assert_eq!(
            actions(&e),
            [
                ABOUT, SETTINGS, LOCK, TRASH, OVERVIEW, ORGANIZE, SUSPEND, REBOOT, POWER_OFF
            ]
        );
        assert_eq!(
            e.iter()
                .filter(|x| matches!(x, MenuEntry::Separator))
                .count(),
            4
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
            // Sans icône sélectionnée, seule la corbeille est inactive.
            assert_eq!(enabled, action != TRASH);
            // Seules les commandes qui perdent la session demandent confirmation.
            assert_eq!(confirm, matches!(action, REBOOT | POWER_OFF));
            // Un groupe porte des icônes à toutes ses lignes ou à aucune : le
            // groupe de puissance n'en a pas.
            assert_eq!(icon, !matches!(action, SUSPEND | REBOOT | POWER_OFF));
        }
    }

    #[test]
    fn corbeille_vise_l_icone_du_bureau_selectionnee() {
        // Une icône de fichier sélectionnée : la commande est active et sa
        // confirmation nomme ce fichier (HIG MENU-04).
        let file = IconDef {
            origin: Default::default(),
            sel: Default::default(),
            id: "notes".into(),
            label: Some("Notes".into()),
            kind: IconKind::Path("/home/jean/Documents/notes.txt".into()),
            icon: None,
            pos: None,
        };
        let trash = |e: &[MenuEntry]| {
            e.iter().find_map(|x| match x {
                MenuEntry::Item {
                    action: Some(TRASH),
                    enabled,
                    confirm,
                    ..
                } => Some((*enabled, confirm.clone())),
                _ => None,
            })
        };
        assert_eq!(
            trash(&entries(Some(&file))),
            Some((
                true,
                Some(crate::i18n::tr("Mettre à la corbeille « Notes » ?").to_string())
            ))
        );
        // Sans libellé d'icône : le nom du fichier.
        let unnamed = IconDef {
            label: None,
            ..file.clone()
        };
        assert!(matches!(
            trash(&entries(Some(&unnamed))),
            Some((true, Some(q))) if q.ends_with("notes.txt » ?")
        ));
        // Une application ou une commande du bureau ne va pas à la corbeille.
        let app = IconDef {
            kind: IconKind::App("firefox".into()),
            ..file
        };
        assert_eq!(trash(&entries(Some(&app))), Some((false, None)));
        assert_eq!(trash(&entries(None)), Some((false, None)));
    }

    #[test]
    fn titres_dans_l_ordre_de_la_barre() {
        let t = titles();
        assert_eq!(
            t.iter().map(|x| x.1).collect::<Vec<_>>(),
            ["hyprdesk", "Fenêtre", "Espaces", "Aide"]
        );
        // « hyprdesk » d'abord ; puis les menus de la fenêtre et des espaces,
        // tels qu'ils existent déjà ailleurs dans la barre ; « Aide » en
        // dernier (HIG MENU-05).
        assert!(matches!(
            t.iter().map(|x| &x.2).collect::<Vec<_>>().as_slice(),
            [
                popover::Request::Shell,
                popover::Request::Window,
                popover::Request::Workspaces,
                popover::Request::Help
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

    #[test]
    fn menu_aide_commence_par_sa_recherche() {
        let e = help_entries();
        // Le champ en tête : sans libellé, en clair, il porte sa propre action.
        assert!(matches!(
            e.first(),
            Some(MenuEntry::Input {
                label: None,
                placeholder,
                secret: false,
                action: Some(HELP_SEARCH),
            }) if placeholder == crate::i18n::tr("Rechercher dans les réglages")
        ));
        // L'aide du shell, puis « À propos de hyprdesk » seul dans son groupe :
        // deux traits, comme « À propos » du menu « hyprdesk » (même action).
        assert_eq!(actions(&e), [HELP_TOUR, HELP_KEYS, HELP_DOCS, ABOUT]);
        assert_eq!(
            e.iter()
                .filter(|x| matches!(x, MenuEntry::Separator))
                .count(),
            2
        );
        let labels: Vec<_> = e
            .iter()
            .filter_map(|x| match x {
                MenuEntry::Item { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            labels,
            [
                crate::i18n::tr("Visite guidée…"),
                crate::i18n::tr("Raccourcis clavier"),
                crate::i18n::tr("Documentation"),
                crate::i18n::tr("À propos de hyprdesk"),
            ]
        );
        // Les commandes d'aide ne perdent rien : aucune ne demande confirmation.
        assert!(e.iter().all(|x| match x {
            MenuEntry::Item { confirm, .. } => confirm.is_none(),
            _ => true,
        }));
    }

    #[test]
    fn champ_d_aide_ouvre_la_recherche_des_reglages() {
        // Les mots sont réduits à un espace ; rien sur une saisie blanche.
        assert_eq!(
            help_search("  volume   de nuit "),
            Some(Command::Settings("?volume de nuit".into()))
        );
        assert_eq!(help_search(" \t "), None);
    }

    #[test]
    fn guide_installe_dans_la_langue_de_l_interface() {
        use crate::i18n::Lang;
        let dir = std::env::temp_dir().join(format!("hyprdesk-guide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Paquet sans guide : rien à ouvrir.
        assert_eq!(guide_in(&dir, Lang::Fr), None);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("GUIDE.md"), "# hyprdesk").unwrap();
        // Seul le guide anglais est installé : c'est lui, dans les deux langues.
        assert_eq!(guide_in(&dir, Lang::Fr), Some(dir.join("GUIDE.md")));
        assert_eq!(guide_in(&dir, Lang::En), Some(dir.join("GUIDE.md")));
        std::fs::write(dir.join("GUIDE.fr.md"), "# hyprdesk").unwrap();
        // Installés tous les deux : celui de la langue de l'interface.
        assert_eq!(guide_in(&dir, Lang::Fr), Some(dir.join("GUIDE.fr.md")));
        assert_eq!(guide_in(&dir, Lang::En), Some(dir.join("GUIDE.md")));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
