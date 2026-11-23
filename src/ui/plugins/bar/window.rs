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
/// Déplacer vers l'espace `n` : `MOVE + n`.
const MOVE: i64 = 1000;

/// Fenêtre active : son adresse et son menu ; rien sans fenêtre active.
pub(in crate::ui) fn read(workspaces: &[Workspace]) -> Option<(String, Vec<MenuEntry>)> {
    let v = ipc::query_json("activewindow").ok()?;
    entries(&v, workspaces)
}

/// Menu d'après l'état de la fenêtre (`hyprctl activewindow -j`).
pub(in crate::ui) fn entries(
    v: &Value,
    workspaces: &[Workspace],
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
        out.push(MenuEntry::Separator);
        out.push(MenuEntry::Header(tr("Déplacer vers l'espace")));
        out.extend(others.iter().take(10).map(|w| {
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
        }));
    }
    out.push(MenuEntry::Separator);
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
        let (address, e) = entries(&v, &[ws(1), ws(2), ws(3), ws(-98)]).unwrap();
        assert_eq!(address, "0xabc");
        let find = |a: i64| {
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
        // Plein écran coché ; épingler impossible sans flottante.
        assert_eq!(find(FULLSCREEN), Some((Some(true), true)));
        assert_eq!(find(PIN), Some((None, false)));
        // Les autres espaces normaux seulement (ni l'actuel, ni un spécial).
        assert!(find(MOVE + 1).is_some() && find(MOVE + 3).is_some());
        assert!(find(MOVE + 2).is_none() && find(MOVE - 98).is_none());
        assert!(matches!(
            e.last(),
            Some(MenuEntry::Item {
                action: Some(CLOSE),
                ..
            })
        ));
        assert!(entries(&serde_json::json!({}), &[]).is_none());
    }
}
