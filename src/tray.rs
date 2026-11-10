//! Zone de notification (StatusNotifierItem) : hyprdesk est un « hôte » qui
//! lit les éléments enregistrés auprès du `org.kde.StatusNotifierWatcher`
//! (fourni par la session, ex. wayle), affiche leurs icônes, les active, et
//! présente leur menu `com.canonical.dbusmenu` dans un menu déroulant natif.

use std::collections::HashMap;

use tokio::sync::OnceCell;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::config::MenuEntry;

static BUS: OnceCell<zbus::Connection> = OnceCell::const_new();

async fn bus() -> zbus::Result<&'static zbus::Connection> {
    BUS.get_or_try_init(zbus::Connection::session).await
}

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";
const MENU: &str = "com.canonical.dbusmenu";

/// Élément de la zone de notification.
#[derive(Debug, Clone, PartialEq)]
pub struct TrayItem {
    /// `bus` + `path` : clé stable de l'élément.
    pub key: String,
    pub bus: String,
    pub path: String,
    pub title: String,
    pub icon_name: Option<String>,
    pub icon_theme_path: Option<String>,
    /// Image fournie par l'application (RGBA), si pas de nom d'icône.
    pub pixmap: Option<(u32, u32, Vec<u8>)>,
    pub menu: Option<String>,
    pub item_is_menu: bool,
    /// `Passive` : l'application demande à être masquée.
    pub passive: bool,
}

/// `":1.42/org/ayatana/NotificationItem/x"` ou `"org.kde.StatusNotifierItem-12-1"`.
pub fn split_service(s: &str) -> (String, String) {
    match s.find('/') {
        Some(i) => (s[..i].to_string(), s[i..].to_string()),
        None => (s.to_string(), "/StatusNotifierItem".to_string()),
    }
}

/// ARGB32 (ordre réseau) → RGBA.
pub fn argb_to_rgba(data: &[u8]) -> Vec<u8> {
    data.chunks_exact(4).flat_map(|p| [p[1], p[2], p[3], p[0]]).collect()
}

fn get<T: TryFrom<OwnedValue>>(props: &HashMap<String, OwnedValue>, k: &str) -> Option<T> {
    props.get(k).and_then(|v| v.try_clone().ok()).and_then(|v| T::try_from(v).ok())
}

async fn registered(conn: &zbus::Connection) -> zbus::Result<Vec<String>> {
    let reply = conn
        .call_method(Some(WATCHER), "/StatusNotifierWatcher", Some("org.freedesktop.DBus.Properties"), "Get", &(WATCHER, "RegisteredStatusNotifierItems"))
        .await?;
    let v: OwnedValue = reply.body().deserialize()?;
    Ok(Vec::<String>::try_from(v).unwrap_or_default())
}

async fn item(conn: &zbus::Connection, service: &str) -> Option<TrayItem> {
    let (bus, path) = split_service(service);
    let reply = conn
        .call_method(Some(bus.as_str()), path.as_str(), Some("org.freedesktop.DBus.Properties"), "GetAll", &(ITEM,))
        .await
        .ok()?;
    let props: HashMap<String, OwnedValue> = reply.body().deserialize().ok()?;
    let pixmap = get::<Vec<(i32, i32, Vec<u8>)>>(&props, "IconPixmap")
        .and_then(|l| l.into_iter().filter(|(w, h, d)| *w > 0 && *h > 0 && d.len() == (*w * *h * 4) as usize).max_by_key(|(w, _, _)| *w))
        .map(|(w, h, d)| (w as u32, h as u32, argb_to_rgba(&d)));
    Some(TrayItem {
        key: format!("{bus}{path}"),
        title: get::<String>(&props, "Title").unwrap_or_default(),
        icon_name: get::<String>(&props, "IconName").filter(|s| !s.is_empty()),
        icon_theme_path: get::<String>(&props, "IconThemePath").filter(|s| !s.is_empty()),
        pixmap,
        menu: get::<OwnedObjectPath>(&props, "Menu").map(|p| p.to_string()).filter(|p| p != "/"),
        item_is_menu: get::<bool>(&props, "ItemIsMenu").unwrap_or(false),
        passive: get::<String>(&props, "Status").is_some_and(|s| s == "Passive"),
        bus,
        path,
    })
}

/// Éléments actuels (vide s'il n'y a pas de watcher dans la session).
pub async fn items() -> Vec<TrayItem> {
    let Ok(conn) = bus().await else { return Vec::new() };
    let Ok(list) = registered(conn).await else { return Vec::new() };
    let mut out = Vec::new();
    for s in list {
        if let Some(i) = item(conn, &s).await
            && !i.passive
        {
            out.push(i);
        }
    }
    out
}

/// Clic gauche (`Activate`) ou du milieu (`SecondaryActivate`).
pub async fn activate(bus_name: String, path: String, secondary: bool, x: i32, y: i32) {
    let Ok(conn) = bus().await else { return };
    let method = if secondary { "SecondaryActivate" } else { "Activate" };
    if let Err(e) = conn.call_method(Some(bus_name.as_str()), path.as_str(), Some(ITEM), method, &(x, y)).await {
        tracing::debug!("zone de notification {method} : {e}");
    }
}

/// Libellé dbusmenu sans les mnémoniques (`_Fichier` → `Fichier`, `__` → `_`).
pub fn clean_label(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                out.push('_');
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

/// Aplatit l'arbre dbusmenu : les sous-menus deviennent des titres de section.
fn flatten(children: Vec<OwnedValue>, depth: u8, out: &mut Vec<MenuEntry>) {
    for c in children {
        let Ok((id, props, sub)) = Layout::try_from(c) else { continue };
        if get::<bool>(&props, "visible") == Some(false) {
            continue;
        }
        if get::<String>(&props, "type").as_deref() == Some("separator") {
            if !matches!(out.last(), None | Some(MenuEntry::Separator)) {
                out.push(MenuEntry::Separator);
            }
            continue;
        }
        let label = clean_label(&get::<String>(&props, "label").unwrap_or_default());
        let submenu = get::<String>(&props, "children-display").as_deref() == Some("submenu") || !sub.is_empty();
        if submenu && depth < 2 {
            if !label.is_empty() {
                out.push(MenuEntry::Header(label));
            }
            flatten(sub, depth + 1, out);
            continue;
        }
        let checked = match get::<String>(&props, "toggle-type").as_deref() {
            Some("checkmark") | Some("radio") => Some(get::<i32>(&props, "toggle-state") == Some(1)),
            _ => None,
        };
        out.push(MenuEntry::Item {
            label,
            detail: None,
            icon: get::<String>(&props, "icon-name").filter(|s| !s.is_empty()),
            checked,
            enabled: get::<bool>(&props, "enabled").unwrap_or(true),
            action: Some(id as i64),
            confirm: None,
        });
    }
    while matches!(out.last(), Some(MenuEntry::Separator)) {
        out.pop();
    }
}

/// Menu d'un élément (dbusmenu).
pub async fn menu(bus_name: &str, menu_path: &str) -> Result<Vec<MenuEntry>, String> {
    let conn = bus().await.map_err(|e| e.to_string())?;
    // Certaines applications ne remplissent leur menu qu'à l'ouverture.
    let _ = conn.call_method(Some(bus_name), menu_path, Some(MENU), "AboutToShow", &(0i32,)).await;
    let reply = conn
        .call_method(Some(bus_name), menu_path, Some(MENU), "GetLayout", &(0i32, -1i32, Vec::<String>::new()))
        .await
        .map_err(|e| e.to_string())?;
    let (_rev, (_id, _props, children)): (u32, Layout) = reply.body().deserialize().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    flatten(children, 0, &mut out);
    Ok(out)
}

/// Clic sur une entrée du menu.
pub async fn menu_event(bus_name: &str, menu_path: &str, id: i32) {
    let Ok(conn) = bus().await else { return };
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as u32).unwrap_or(0);
    if let Err(e) = conn.call_method(Some(bus_name), menu_path, Some(MENU), "Event", &(id, "clicked", Value::from(0i32), ts)).await {
        tracing::warn!("menu de la zone de notification : {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn services_pixels_et_libelles() {
        assert_eq!(split_service(":1.4/org/ayatana/NotificationItem/x"), (":1.4".into(), "/org/ayatana/NotificationItem/x".into()));
        assert_eq!(split_service("org.kde.StatusNotifierItem-1-1"), ("org.kde.StatusNotifierItem-1-1".into(), "/StatusNotifierItem".into()));
        assert_eq!(argb_to_rgba(&[255, 10, 20, 30]), vec![10, 20, 30, 255]);
        assert_eq!(clean_label("_Fichier"), "Fichier");
        assert_eq!(clean_label("a__b"), "a_b");
    }
}
