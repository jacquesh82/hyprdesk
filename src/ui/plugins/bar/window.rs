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
}
