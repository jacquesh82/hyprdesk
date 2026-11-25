//! Menu de la fenêtre active (MLG-311) : un clic sur son titre dans la
//! barre. Sous Linux, une barre de menus globale à la macOS n'a pas de sens
//! (GTK4 et libadwaita ne publient plus leurs menus) ; la barre propose à la
//! place ce que le compositeur sait faire de la fenêtre : la réduire,
//! l'agrandir, la mettre en plein écran, flottante, épinglée sur tous les
//! espaces, l'envoyer sur un autre espace, la fermer. Coches : son état.

use serde_json::Value;

use crate::config::MenuEntry;
use crate::hypr::Workspace;
use hyprpanel_core::ipc;

const MINIMIZE: i64 = 1;
const MAXIMIZE: i64 = 2;
const FULLSCREEN: i64 = 3;
const FLOAT: i64 = 4;
const PIN: i64 = 5;
const CLOSE: i64 = 6;
/// Fermer toutes les fenêtres de l'application de la fenêtre active (HIG
/// MENU-04 : la commande du clic droit du dock, dans la barre aussi).
const CLOSE_ALL: i64 = 7;
/// Déplacer vers l'espace `n` : `MOVE + n`.
const MOVE: i64 = 1000;
/// Menu « Fenêtres » : aller à la fenêtre de rang `i` de la liste, `LIST + i`
/// (au-delà de toute commande, `MOVE` compris).
pub(in crate::ui) const LIST: i64 = 1_000_000;

/// Longueur d'un titre de fenêtre dans le menu « Fenêtres ».
const TITLE_MAX: usize = 60;

/// Menu « Fenêtres » de la barre (HIG MENU-01), lu sur le compositeur,
/// comme le menu Fenêtre de macOS : les commandes de la fenêtre active en
/// tête (`entries`), puis toutes les fenêtres ouvertes (`list`, actions
/// `LIST + rang`). Rend l'adresse de la fenêtre active, celles de la liste
/// et les entrées ; `apps` donne l'icône de chaque application.
pub(in crate::ui) fn read_all(
    workspaces: &[Workspace],
    apps: &[crate::launch::AppInfo],
) -> (Option<String>, Vec<String>, Vec<MenuEntry>) {
    let raw = ipc::query_json("clients").ok();
    let clients = raw
        .as_ref()
        .map(crate::hypr::parse_clients)
        .unwrap_or_default();
    let active = ipc::query_json("activewindow").ok();
    let commands = active.as_ref().and_then(|v| {
        let same_app = raw.as_ref().map_or(1, |c| same_application(v, c));
        let (address, mut out) = entries(v, workspaces, same_app)?;
        // La fenêtre visée par ces commandes, nommée en tête.
        let title = v["title"].as_str().map(str::trim).unwrap_or("");
        let name = if title.is_empty() {
            v["class"].as_str().unwrap_or("").to_string()
        } else {
            truncate(title, TITLE_MAX)
        };
        if !name.is_empty() {
            out.insert(0, MenuEntry::Header(name));
        }
        Some((address, out))
    });
    let current = commands.as_ref().map(|(a, _)| a.as_str());
    let mut icons = std::collections::HashMap::new();
    let (addresses, windows) = list(&clients, workspaces, current, LIST, &mut |class| {
        icons
            .entry(class.to_string())
            .or_insert_with(|| {
                crate::launch::icon_name_for_class(class, apps)
                    .map(|n| crate::ui::App::app_icon_key(&n))
            })
            .clone()
    });
    match commands {
        Some((address, mut out)) => {
            out.push(MenuEntry::Separator);
            out.extend(windows);
            (Some(address), addresses, out)
        }
        None => (None, addresses, windows),
    }
}

/// Toutes les fenêtres ouvertes, groupées par espace dans l'ordre des
/// espaces, les espaces spéciaux ensuite, les réduites en dernier ; la
/// fenêtre active cochée. Les fenêtres que hyprdesk a mises de côté
/// (`STASH`) n'y sont pas : le mode du bureau qui les a rangées les rendra.
/// `icon` : nom d'icône de l'application d'une classe ; l'action d'une
/// fenêtre est `base` plus son rang dans les adresses rendues.
fn list(
    clients: &[crate::hypr::Client],
    workspaces: &[Workspace],
    active: Option<&str>,
    base: i64,
    icon: &mut dyn FnMut(&str) -> Option<String>,
) -> (Vec<String>, Vec<MenuEntry>) {
    let tr = |s: &str| crate::i18n::tr(s).to_string();
    // Rang d'un groupe : espaces ordinaires par numéro, puis spéciaux, puis
    // réduites ; et son en-tête.
    let group = |name: &str| -> ((u8, i64, String), String) {
        if name == crate::hypr::MINIMIZED {
            return ((2, 0, String::new()), tr("Réduites"));
        }
        if let Some(special) = name.strip_prefix("special:") {
            return ((1, 0, special.to_string()), special.to_string());
        }
        let id = workspaces
            .iter()
            .find(|w| w.name == name)
            .map(|w| w.id)
            .or_else(|| name.parse().ok())
            .unwrap_or(i64::MAX);
        let header = if name == id.to_string() {
            crate::tr!("Espace {n}", n = id)
        } else {
            name.to_string()
        };
        ((0, id, name.to_string()), header)
    };
    let mut shown: Vec<_> = clients
        .iter()
        .filter(|c| c.workspace != crate::hypr::STASH && !c.workspace.is_empty())
        .map(|c| (group(&c.workspace), c))
        .collect();
    // Tri stable : dans un groupe, l'ordre du compositeur.
    shown.sort_by(|a, b| a.0.0.cmp(&b.0.0));
    if shown.is_empty() {
        return (
            Vec::new(),
            vec![MenuEntry::Item {
                label: tr("Aucune fenêtre ouverte"),
                detail: None,
                icon: None,
                checked: None,
                enabled: false,
                action: None,
                confirm: None,
            }],
        );
    }
    let mut addresses = Vec::with_capacity(shown.len());
    let mut out = Vec::new();
    let mut current = None;
    for ((rank, header), c) in shown {
        if current.as_ref() != Some(&rank) {
            if current.is_some() {
                out.push(MenuEntry::Separator);
            }
            out.push(MenuEntry::Header(header));
            current = Some(rank);
        }
        let title = if c.title.trim().is_empty() {
            c.class.clone()
        } else {
            truncate(c.title.trim(), TITLE_MAX)
        };
        out.push(MenuEntry::Item {
            label: title,
            detail: None,
            icon: icon(&c.class),
            checked: (active == Some(c.address.as_str())).then_some(true),
            enabled: true,
            action: Some(base + addresses.len() as i64),
            confirm: None,
        });
        addresses.push(c.address.clone());
    }
    (addresses, out)
}

/// Titre tenu à `max` caractères, points de suspension compris.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{cut}…")
}

/// Fenêtre active : son adresse et son menu ; rien sans fenêtre active.
pub(in crate::ui) fn read(workspaces: &[Workspace]) -> Option<(String, Vec<MenuEntry>)> {
    let v = ipc::query_json("activewindow").ok()?;
    let same_app = ipc::query_json("clients")
        .map(|c| same_application(&v, &c))
        .unwrap_or(1);
    entries(&v, workspaces, same_app)
}

/// Fenêtre d'adresse `address` et nombre de fenêtres de son application,
/// lues dans `clients` : le menu d'une fenêtre qui n'est pas forcément active
/// (clic droit dans l'aperçu des espaces, HIG MENU-06). Rien si elle a fermé.
pub(in crate::ui) fn named(address: &str) -> Option<(Value, usize)> {
    let clients = ipc::query_json("clients").ok()?;
    let v = clients
        .as_array()?
        .iter()
        .find(|c| c["address"].as_str() == Some(address))?
        .clone();
    let same_app = same_application(&v, &clients);
    Some((v, same_app))
}

/// Fenêtres ouvertes de l'application de la fenêtre active, la sienne
/// comprise ; une au moins.
fn same_application(active: &Value, clients: &Value) -> usize {
    let class = active["class"].as_str().unwrap_or("");
    crate::hypr::parse_clients(clients)
        .iter()
        .filter(|c| c.class == class)
        .count()
        .max(1)
}

/// Menu d'après l'état de la fenêtre (`hyprctl activewindow -j`) et du nombre
/// `same_app` de fenêtres de son application.
pub(in crate::ui) fn entries(
    v: &Value,
    workspaces: &[Workspace],
    same_app: usize,
) -> Option<(String, Vec<MenuEntry>)> {
    let tr = |s: &str| crate::i18n::tr(s).to_string();
    let address = v["address"].as_str().filter(|a| !a.is_empty())?.to_string();
    let floating = v["floating"].as_bool().unwrap_or(false);
    let pinned = v["pinned"].as_bool().unwrap_or(false);
    let fullscreen = v["fullscreen"].as_i64().unwrap_or(0);
    let here = v["workspace"]["id"].as_i64().unwrap_or(0);
    let minimized = v["workspace"]["name"].as_str() == Some(crate::hypr::MINIMIZED);
    let item = |label: String, icon: &str, checked: Option<bool>, enabled: bool, action: i64| {
        MenuEntry::Item {
            label,
            detail: None,
            icon: (!icon.is_empty()).then(|| icon.to_string()),
            checked,
            enabled,
            action: Some(action),
            confirm: None,
        }
    };
    let mut out = vec![
        item(
            tr("Réduire"),
            "window-minimize-symbolic",
            None,
            !minimized,
            MINIMIZE,
        ),
        item(
            tr("Agrandir"),
            "window-maximize-symbolic",
            (fullscreen == 1).then_some(true),
            true,
            MAXIMIZE,
        ),
        item(
            tr("Plein écran"),
            "view-fullscreen-symbolic",
            (fullscreen == 2).then_some(true),
            true,
            FULLSCREEN,
        ),
        item(
            tr("Flottante"),
            "focus-windows-symbolic",
            floating.then_some(true),
            true,
            FLOAT,
        ),
        // Hyprland n'épingle qu'une fenêtre flottante.
        item(
            tr("Sur tous les espaces"),
            "view-pin-symbolic",
            pinned.then_some(true),
            floating,
            PIN,
        ),
    ];
    let mut others: Vec<&Workspace> = workspaces
        .iter()
        .filter(|w| w.id > 0 && w.id != here)
        .collect();
    others.sort_by_key(|w| w.id);
    others.dedup_by_key(|w| w.id);
    if !others.is_empty() {
        // Un sous-menu (HIG MENU-03) : sa colonne liste tous les autres
        // espaces, sans plafond.
        out.push(MenuEntry::Separator);
        out.push(MenuEntry::Submenu {
            label: tr("Déplacer vers l'espace"),
            icon: None,
            enabled: true,
            children: others
                .iter()
                .map(|w| {
                    item(
                        if w.name == w.id.to_string() {
                            crate::tr!("Espace {n}", n = w.id)
                        } else {
                            w.name.clone()
                        },
                        "",
                        None,
                        true,
                        MOVE + w.id,
                    )
                })
                .collect(),
        });
    }
    out.push(MenuEntry::Separator);
    // Toujours à sa place, inactive quand l'application n'a qu'une fenêtre :
    // « Fermer » la ferme déjà. Le nombre de fenêtres visées suit le libellé.
    let several = same_app > 1;
    out.push(MenuEntry::Item {
        label: tr("Fermer toutes les fenêtres"),
        detail: several.then(|| same_app.to_string()),
        icon: Some("window-close-symbolic".into()),
        checked: None,
        enabled: several,
        action: Some(CLOSE_ALL),
        confirm: None,
    });
    out.push(item(
        tr("Fermer"),
        "window-close-symbolic",
        None,
        true,
        CLOSE,
    ));
    Some((address, out))
}

/// Applique l'entrée `action` du menu à la fenêtre `address`.
pub(in crate::ui) fn act(address: &str, action: i64) -> Result<(), String> {
    let w = ipc::lua_escape(address);
    let lua = match action {
        MINIMIZE => {
            return crate::hypr::move_window(address, crate::hypr::MINIMIZED)
                .map_err(|e| e.to_string());
        }
        CLOSE => return crate::hypr::close_window(address).map_err(|e| e.to_string()),
        // Toutes les fenêtres de la même classe que la fenêtre active, comme
        // le menu du dock les range.
        CLOSE_ALL => {
            if let Ok(v) = ipc::query_json("clients") {
                let all = crate::hypr::parse_clients(&v);
                let class = all
                    .iter()
                    .find(|c| c.address == address)
                    .map(|c| c.class.clone());
                if let Some(class) = class {
                    for c in all.iter().filter(|c| c.class == class) {
                        let _ = crate::hypr::close_window(&c.address);
                    }
                }
            }
            return Ok(());
        }
        n if n > MOVE => {
            return crate::hypr::move_window(address, &(n - MOVE).to_string())
                .map_err(|e| e.to_string());
        }
        MAXIMIZE => format!(
            "hl.dsp.window.fullscreen({{ mode = \"maximized\", action = \"toggle\", window = \"address:{w}\" }})"
        ),
        FULLSCREEN => format!(
            "hl.dsp.window.fullscreen({{ mode = \"fullscreen\", action = \"toggle\", window = \"address:{w}\" }})"
        ),
        FLOAT => {
            format!("hl.dsp.window.float({{ action = \"toggle\", window = \"address:{w}\" }})")
        }
        PIN => format!("hl.dsp.window.pin({{ action = \"toggle\", window = \"address:{w}\" }})"),
        _ => return Ok(()),
    };
    let r = ipc::dispatch(&lua).map_err(|e| e.to_string())?;
    if r.trim() == "ok" {
        Ok(())
    } else {
        Err(r.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(id: i64) -> Workspace {
        Workspace {
            id,
            name: id.to_string(),
            monitor: "A".into(),
            windows: 1,
            fullscreen: false,
        }
    }

    #[test]
    fn menu_selon_l_etat_de_la_fenetre() {
        let v: Value = serde_json::from_str(
            r#"{"address":"0xabc","floating":false,"pinned":false,"fullscreen":2,
                "workspace":{"id":2,"name":"2"}}"#,
        )
        .unwrap();
        let (address, e) = entries(&v, &[ws(1), ws(2), ws(3), ws(-98)], 1).unwrap();
        assert_eq!(address, "0xabc");
        let find_in = |e: &[MenuEntry], a: i64| {
            e.iter().find_map(|x| match x {
                MenuEntry::Item {
                    action: Some(n),
                    checked,
                    enabled,
                    ..
                } if *n == a => Some((*checked, *enabled)),
                _ => None,
            })
        };
        let find = |a: i64| find_in(&e, a);
        // Plein écran coché ; épingler impossible sans flottante.
        assert_eq!(find(FULLSCREEN), Some((Some(true), true)));
        assert_eq!(find(PIN), Some((None, false)));
        // « Déplacer vers l'espace » est un sous-menu (HIG MENU-03) : les
        // autres espaces normaux seulement (ni l'actuel, ni un spécial), dans
        // sa colonne, et non plus dans la liste.
        let Some(MenuEntry::Submenu {
            label,
            enabled,
            children,
            ..
        }) = e.iter().find(|x| matches!(x, MenuEntry::Submenu { .. }))
        else {
            panic!("sous-menu attendu");
        };
        assert_eq!(label, "Déplacer vers l'espace");
        assert!(enabled);
        assert!(find(MOVE + 1).is_none());
        assert!(find_in(children, MOVE + 1).is_some() && find_in(children, MOVE + 3).is_some());
        assert!(find_in(children, MOVE + 2).is_none() && find_in(children, MOVE - 98).is_none());
        assert!(matches!(
            e.last(),
            Some(MenuEntry::Item {
                action: Some(CLOSE),
                ..
            })
        ));
        assert!(entries(&serde_json::json!({}), &[], 1).is_none());
    }

    #[test]
    fn deplacer_vers_l_espace_liste_tous_les_espaces() {
        let v: Value =
            serde_json::from_str(r#"{"address":"0xabc","workspace":{"id":1,"name":"1"}}"#).unwrap();
        // Quinze espaces nommés et deux spéciaux : la colonne en liste quatorze.
        let mut spaces: Vec<Workspace> = (1..=15)
            .map(|i| Workspace {
                name: format!("Projet {i}"),
                ..ws(i)
            })
            .collect();
        spaces.push(ws(-98));
        spaces.push(ws(0));
        let (_, e) = entries(&v, &spaces, 1).unwrap();
        let Some(MenuEntry::Submenu { children, .. }) =
            e.iter().find(|x| matches!(x, MenuEntry::Submenu { .. }))
        else {
            panic!("sous-menu attendu");
        };
        let names: Vec<&str> = children
            .iter()
            .map(|c| match c {
                MenuEntry::Item { label, .. } => label.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(names.len(), 14);
        assert_eq!(names[0], "Projet 2");
        assert_eq!(names[13], "Projet 15");
        // Aucun espace autre : pas de sous-menu du tout.
        let (_, only) = entries(&v, &[ws(1)], 1).unwrap();
        assert!(only.iter().all(|x| !matches!(x, MenuEntry::Submenu { .. })));
    }

    #[test]
    fn fermer_toutes_les_fenetres_de_l_application() {
        let v: Value = serde_json::from_str(
            r#"{"address":"0xabc","class":"firefox","workspace":{"id":1,"name":"1"}}"#,
        )
        .unwrap();
        let close_all = |same_app: usize| {
            entries(&v, &[], same_app)
                .unwrap()
                .1
                .into_iter()
                .find_map(|x| match x {
                    MenuEntry::Item {
                        action: Some(CLOSE_ALL),
                        enabled,
                        detail,
                        ..
                    } => Some((enabled, detail)),
                    _ => None,
                })
        };
        // Toujours dans le menu ; inactive tant que l'application n'a qu'une
        // fenêtre, que « Fermer » ferme déjà. « Fermer » reste la dernière.
        assert_eq!(close_all(1), Some((false, None)));
        assert_eq!(close_all(3), Some((true, Some("3".into()))));
        assert!(matches!(
            entries(&v, &[], 3).unwrap().1.last(),
            Some(MenuEntry::Item {
                action: Some(CLOSE),
                ..
            })
        ));
        // Les fenêtres de la classe de la fenêtre active, pas les autres.
        let clients: Value = serde_json::from_str(
            r#"[{"address":"0xabc","class":"firefox"},{"address":"0xdef","class":"firefox"},
                {"address":"0xfff","class":"thunar"}]"#,
        )
        .unwrap();
        assert_eq!(same_application(&v, &clients), 2);
        assert_eq!(same_application(&serde_json::json!({}), &clients), 1);
    }

    fn client(address: &str, class: &str, title: &str, workspace: &str) -> crate::hypr::Client {
        crate::hypr::Client {
            address: address.into(),
            class: class.into(),
            title: title.into(),
            workspace: workspace.into(),
            monitor: "A".into(),
        }
    }

    #[test]
    fn fenetres_par_espace_les_reduites_en_dernier() {
        let clients = [
            client("0x3", "foot", "terminal", crate::hypr::MINIMIZED),
            client("0x1", "brave", "Recherche", "2"),
            client("0x2", "thunar", "", "1"),
            client("0x4", "kitty", "rangée", crate::hypr::STASH),
            client("0x5", "foot", "second", "2"),
        ];
        let mut icon = |class: &str| Some(format!("app:{class}"));
        let (addresses, entries) = list(&clients, &[ws(1), ws(2)], Some("0x5"), 0, &mut icon);
        // Espace 1, espace 2 (ordre du compositeur gardé), réduites ; la
        // fenêtre mise de côté par hyprdesk n'y est pas.
        assert_eq!(addresses, ["0x2", "0x1", "0x5", "0x3"]);
        let rows: Vec<String> = entries
            .iter()
            .map(|e| match e {
                MenuEntry::Header(h) => format!("# {h}"),
                MenuEntry::Separator => "-".into(),
                MenuEntry::Item {
                    label,
                    checked,
                    action,
                    icon,
                    ..
                } => format!(
                    "{label} {action:?} {} {}",
                    checked.is_some(),
                    icon.as_deref().unwrap_or("")
                ),
                _ => "?".into(),
            })
            .collect();
        assert_eq!(
            rows,
            [
                "# Espace 1",
                // Sans titre : le nom de l'application.
                "thunar Some(0) false app:thunar",
                "-",
                "# Espace 2",
                "Recherche Some(1) false app:brave",
                "second Some(2) true app:foot",
                "-",
                "# Réduites",
                "terminal Some(3) false app:foot",
            ]
        );
    }

    #[test]
    fn aucune_fenetre_une_ligne_inactive() {
        let (addresses, entries) = list(&[], &[], None, LIST, &mut |_| None);
        assert!(addresses.is_empty());
        assert!(matches!(
            entries.as_slice(),
            [MenuEntry::Item {
                enabled: false,
                action: None,
                ..
            }]
        ));
    }
}
