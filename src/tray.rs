//! Zone de notification (StatusNotifierItem) : hyprdesk est un « hôte » qui
//! lit les éléments enregistrés auprès du `org.kde.StatusNotifierWatcher`
//! (fourni par la session, ex. wayle), affiche leurs icônes, les active, et
//! présente leur menu `com.canonical.dbusmenu` dans un menu déroulant natif.

use std::collections::HashMap;

use tokio::sync::{Mutex, OnceCell};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::config::MenuEntry;

static BUS: OnceCell<zbus::Connection> = OnceCell::const_new();

async fn bus() -> zbus::Result<&'static zbus::Connection> {
    BUS.get_or_try_init(|| async {
        let conn = zbus::Connection::session().await?;
        conn.object_server()
            .at("/StatusNotifierWatcher", Watcher::default())
            .await?;
        Ok::<_, zbus::Error>(conn)
    })
    .await
}

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";
const MENU: &str = "com.canonical.dbusmenu";

#[derive(Default)]
struct Watcher {
    items: Mutex<Vec<String>>,
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_item(
        &self,
        service: String,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let key = if service.starts_with('/') {
            let sender = header.sender().ok_or_else(|| {
                zbus::fdo::Error::InvalidArgs(crate::i18n::tr("expéditeur absent").into())
            })?;
            format!("{sender}{service}")
        } else {
            service
        };
        let (bus, path) = split_service(&key);
        zbus::names::BusName::try_from(bus.as_str())
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        zbus::zvariant::ObjectPath::try_from(path.as_str())
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        let mut items = self.items.lock().await;
        if !items.contains(&key) {
            items.push(key.clone());
            drop(items);
            Self::status_notifier_item_registered(&emitter, &key).await?;
            self.registered_status_notifier_items_changed(&emitter)
                .await?;
        }
        Ok(())
    }

    async fn register_status_notifier_host(
        &self,
        _service: String,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        Self::status_notifier_host_registered(&emitter)
            .await
            .map_err(zbus::fdo::Error::from)
    }

    #[zbus(signal)]
    async fn status_notifier_host_registered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::Result<()>;

    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.lock().await.clone()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;
}

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
    data.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[1], p[2], p[3], p[0]])
        .collect()
}

/// Taille à laquelle une image d'application est gardée (affichée bien plus petite).
const PIXMAP_KEEP: u32 = 64;

/// Image à retenir parmi celles proposées : la plus petite assez grande, sinon
/// la plus grande. Certaines applications n'envoient qu'une image 1024×1024.
pub fn pick_pixmap(list: Vec<(i32, i32, Vec<u8>)>) -> Option<(u32, u32, Vec<u8>)> {
    let ok: Vec<(u32, u32, Vec<u8>)> = list
        .into_iter()
        .filter(|(w, h, d)| {
            (1..=4096).contains(w)
                && (1..=4096).contains(h)
                && d.len() == (*w as usize * *h as usize * 4)
        })
        .map(|(w, h, d)| (w as u32, h as u32, d))
        .collect();
    let enough = |p: &&(u32, u32, Vec<u8>)| p.0 >= PIXMAP_KEEP;
    let idx = ok
        .iter()
        .enumerate()
        .filter(|(_, p)| enough(p))
        .min_by_key(|(_, p)| p.0)
        .or_else(|| ok.iter().enumerate().max_by_key(|(_, p)| p.0))
        .map(|(i, _)| i)?;
    ok.into_iter().nth(idx)
}

/// Réduit une image RGBA trop grande : inutile de garder 4 Mo pour 20 px.
pub fn shrink(w: u32, h: u32, rgba: Vec<u8>) -> (u32, u32, Vec<u8>) {
    if w <= PIXMAP_KEEP * 2 && h <= PIXMAP_KEEP * 2 {
        return (w, h, rgba);
    }
    match image::RgbaImage::from_raw(w, h, rgba) {
        Some(img) => {
            let small = image::imageops::thumbnail(&img, PIXMAP_KEEP, PIXMAP_KEEP * h / w.max(1));
            (small.width(), small.height(), small.into_raw())
        }
        None => (0, 0, Vec::new()),
    }
}

fn get<T: TryFrom<OwnedValue>>(props: &HashMap<String, OwnedValue>, k: &str) -> Option<T> {
    props
        .get(k)
        .and_then(|v| v.try_clone().ok())
        .and_then(|v| T::try_from(v).ok())
}

async fn registered(conn: &zbus::Connection) -> zbus::Result<Vec<String>> {
    let reply = conn
        .call_method(
            Some(WATCHER),
            "/StatusNotifierWatcher",
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(WATCHER, "RegisteredStatusNotifierItems"),
        )
        .await?;
    let v: OwnedValue = reply.body().deserialize()?;
    Ok(Vec::<String>::try_from(v).unwrap_or_default())
}

async fn item(conn: &zbus::Connection, service: &str) -> Option<TrayItem> {
    let (bus, path) = split_service(service);
    let reply = conn
        .call_method(
            Some(bus.as_str()),
            path.as_str(),
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(ITEM,),
        )
        .await
        .ok()?;
    let props: HashMap<String, OwnedValue> = reply.body().deserialize().ok()?;
    let pixmap = get::<Vec<(i32, i32, Vec<u8>)>>(&props, "IconPixmap")
        .and_then(pick_pixmap)
        .map(|(w, h, d)| shrink(w, h, argb_to_rgba(&d)));
    let title = get::<String>(&props, "Title")
        .filter(|s| !s.trim().is_empty())
        .or_else(|| get::<String>(&props, "Id"))
        .unwrap_or_default();
    Some(TrayItem {
        key: format!("{bus}{path}"),
        title,
        icon_name: get::<String>(&props, "IconName").filter(|s| !s.is_empty()),
        icon_theme_path: get::<String>(&props, "IconThemePath").filter(|s| !s.is_empty()),
        pixmap,
        menu: get::<OwnedObjectPath>(&props, "Menu")
            .map(|p| p.to_string())
            .filter(|p| p != "/"),
        item_is_menu: get::<bool>(&props, "ItemIsMenu").unwrap_or(false),
        passive: get::<String>(&props, "Status").is_some_and(|s| s == "Passive"),
        bus,
        path,
    })
}

/// Éléments actuels (vide s'il n'y a pas de watcher dans la session).
pub async fn items() -> Vec<TrayItem> {
    let Ok(conn) = bus().await else {
        return Vec::new();
    };
    // Coexiste avec un watcher externe ; prend le relais seulement s'il est absent.
    if registered(conn).await.is_err() {
        let _ = conn
            .request_name_with_flags(WATCHER, zbus::fdo::RequestNameFlags::DoNotQueue.into())
            .await;
    }
    let Ok(list) = registered(conn).await else {
        return Vec::new();
    };
    let mut out = Vec::new();
    // Une application peut s'inscrire deux fois (nom connu et nom unique) :
    // un seul élément par processus et par chemin.
    let mut seen = std::collections::HashSet::new();
    for s in list {
        let (bus, path) = split_service(&s);
        let owner = if bus.starts_with(':') {
            bus.clone()
        } else {
            match conn
                .call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "GetNameOwner",
                    &(bus.as_str(),),
                )
                .await
            {
                Ok(r) => r.body().deserialize::<String>().unwrap_or(bus.clone()),
                Err(_) => bus.clone(),
            }
        };
        if !seen.insert((owner, path)) {
            continue;
        }
        if let Ok(Some(i)) =
            tokio::time::timeout(std::time::Duration::from_millis(800), item(conn, &s)).await
            && !i.passive
        {
            out.push(i);
        }
    }
    // Retire les inscriptions dont le propriétaire a quitté le bus.
    if let Ok(interface) = conn
        .object_server()
        .interface::<_, Watcher>("/StatusNotifierWatcher")
        .await
    {
        let watcher = interface.get().await;
        let registered = watcher.items.lock().await.clone();
        for service in registered {
            let (name, _) = split_service(&service);
            if let Ok(reply) = conn
                .call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "NameHasOwner",
                    &(name,),
                )
                .await
                && reply.body().deserialize::<bool>().ok() == Some(false)
            {
                watcher.items.lock().await.retain(|s| s != &service);
                let _ = Watcher::status_notifier_item_unregistered(
                    interface.signal_emitter(),
                    &service,
                )
                .await;
                let _ = watcher
                    .registered_status_notifier_items_changed(interface.signal_emitter())
                    .await;
            }
        }
    }
    out
}

/// Clic gauche (`Activate`) ou du milieu (`SecondaryActivate`).
pub async fn activate(bus_name: String, path: String, secondary: bool, x: i32, y: i32) {
    let Ok(conn) = bus().await else { return };
    let method = if secondary {
        "SecondaryActivate"
    } else {
        "Activate"
    };
    if let Err(e) = conn
        .call_method(
            Some(bus_name.as_str()),
            path.as_str(),
            Some(ITEM),
            method,
            &(x, y),
        )
        .await
    {
        tracing::debug!(
            "{}",
            crate::tr!(
                "zone de notification {method} : {e}",
                method = method,
                e = e
            )
        );
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

/// Entrée de l'arbre dbusmenu, lue une fois (puis aplatie, testable).
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: i32,
    pub label: String,
    pub visible: bool,
    pub separator: bool,
    /// Sous-menu (même vide : à remplir par AboutToShow).
    pub submenu: bool,
    pub toggle: Option<bool>,
    pub enabled: bool,
    pub icon: Option<String>,
    pub children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            id: 0,
            label: String::new(),
            visible: true,
            separator: false,
            submenu: false,
            toggle: None,
            enabled: true,
            icon: None,
            children: Vec::new(),
        }
    }
}

/// Arbre dbusmenu (profondeur bornée).
fn nodes(children: Vec<OwnedValue>, depth: u8) -> Vec<Node> {
    if depth > 8 {
        return Vec::new();
    }
    children
        .into_iter()
        .take(256)
        .filter_map(|c| {
            let (id, props, sub) = Layout::try_from(c).ok()?;
            let children = nodes(sub, depth + 1);
            Some(Node {
                id,
                label: clean_label(&get::<String>(&props, "label").unwrap_or_default()),
                visible: get::<bool>(&props, "visible") != Some(false),
                separator: get::<String>(&props, "type").as_deref() == Some("separator"),
                submenu: get::<String>(&props, "children-display").as_deref() == Some("submenu")
                    || !children.is_empty(),
                toggle: match get::<String>(&props, "toggle-type").as_deref() {
                    Some("checkmark") | Some("radio") => {
                        Some(get::<i32>(&props, "toggle-state") == Some(1))
                    }
                    _ => None,
                },
                enabled: get::<bool>(&props, "enabled").unwrap_or(true),
                icon: get::<String>(&props, "icon-name").filter(|s| !s.is_empty()),
                children,
            })
        })
        .collect()
}

/// Sous-menus encore vides (l'application les remplit à l'ouverture).
pub fn empty_submenus(nodes: &[Node], out: &mut Vec<i32>) {
    for n in nodes {
        if n.submenu && n.children.is_empty() && n.visible {
            out.push(n.id);
        }
        empty_submenus(&n.children, out);
    }
}

/// Aplatit l'arbre : chaque sous-menu devient un titre de section portant
/// son chemin (« Fichier › Récents »), à toute profondeur.
pub fn flatten(nodes: &[Node], path: &str, out: &mut Vec<MenuEntry>) {
    for n in nodes {
        if out.len() >= 256 {
            break;
        }
        if !n.visible {
            continue;
        }
        if n.separator {
            if !matches!(out.last(), None | Some(MenuEntry::Separator)) {
                out.push(MenuEntry::Separator);
            }
            continue;
        }
        if n.submenu {
            let here = match (path.is_empty(), n.label.is_empty()) {
                (true, _) => n.label.clone(),
                (false, true) => path.to_string(),
                (false, false) => format!("{path} › {}", n.label),
            };
            if !here.is_empty() && !n.children.is_empty() {
                out.push(MenuEntry::Header(here.clone()));
            }
            flatten(&n.children, &here, out);
            continue;
        }
        out.push(MenuEntry::Item {
            label: n.label.clone(),
            detail: None,
            icon: n.icon.clone(),
            checked: n.toggle,
            enabled: n.enabled,
            action: Some(n.id as i64),
            confirm: None,
        });
    }
    while matches!(out.last(), Some(MenuEntry::Separator)) {
        out.pop();
    }
}

async fn layout(
    conn: &zbus::Connection,
    bus_name: &str,
    menu_path: &str,
) -> Result<Vec<Node>, String> {
    let reply = conn
        .call_method(
            Some(bus_name),
            menu_path,
            Some(MENU),
            "GetLayout",
            // Profondeur -1 : tout l'arbre.
            &(0i32, -1i32, Vec::<String>::new()),
        )
        .await
        .map_err(|e| e.to_string())?;
    let (_rev, (_id, _props, children)): (u32, Layout) =
        reply.body().deserialize().map_err(|e| e.to_string())?;
    Ok(nodes(children, 0))
}

/// Menu d'un élément (dbusmenu).
pub async fn menu(bus_name: &str, menu_path: &str) -> Result<Vec<MenuEntry>, String> {
    let conn = bus().await.map_err(|e| e.to_string())?;
    let about = |id: i32| {
        let conn = conn.clone();
        async move {
            let _ = conn
                .call_method(Some(bus_name), menu_path, Some(MENU), "AboutToShow", &(id,))
                .await;
        }
    };
    // Certaines applications ne remplissent leur menu qu'à l'ouverture.
    about(0).await;
    let mut tree = layout(conn, bus_name, menu_path).await?;
    // Sous-menus vides : les ouvrir, puis relire l'arbre (une fois).
    let mut empty = Vec::new();
    empty_submenus(&tree, &mut empty);
    if !empty.is_empty() {
        for id in empty.into_iter().take(32) {
            about(id).await;
        }
        tree = layout(conn, bus_name, menu_path).await?;
    }
    let mut out = Vec::new();
    flatten(&tree, "", &mut out);
    Ok(out)
}

/// Clic sur une entrée du menu.
pub async fn menu_event(bus_name: &str, menu_path: &str, id: i32) {
    let Ok(conn) = bus().await else { return };
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);
    if let Err(e) = conn
        .call_method(
            Some(bus_name),
            menu_path,
            Some(MENU),
            "Event",
            &(id, "clicked", Value::from(0i32), ts),
        )
        .await
    {
        tracing::warn!(
            "{}",
            crate::tr!("menu de la zone de notification : {e}", e = e)
        );
    }
}

#[cfg(test)]
mod tests {

    fn item(id: i32, label: &str) -> Node {
        Node {
            id,
            label: label.into(),
            ..Default::default()
        }
    }

    fn sub(id: i32, label: &str, children: Vec<Node>) -> Node {
        Node {
            id,
            label: label.into(),
            submenu: true,
            children,
            ..Default::default()
        }
    }

    #[test]
    fn sous_menus_imbriques() {
        let tree = vec![
            item(1, "Ouvrir"),
            Node {
                separator: true,
                ..Default::default()
            },
            sub(
                2,
                "Fichier",
                vec![
                    item(3, "Nouveau"),
                    sub(4, "Récents", vec![item(5, "a.txt")]),
                ],
            ),
            Node {
                visible: false,
                ..item(6, "Caché")
            },
            Node {
                separator: true,
                ..Default::default()
            },
        ];
        let mut out = Vec::new();
        flatten(&tree, "", &mut out);
        let labels: Vec<String> = out
            .iter()
            .map(|e| match e {
                MenuEntry::Item { label, .. } => label.clone(),
                MenuEntry::Header(h) => format!("# {h}"),
                MenuEntry::Separator => "-".into(),
                MenuEntry::Input { .. } => "input".into(),
            })
            .collect();
        assert_eq!(
            labels,
            [
                "Ouvrir",
                "-",
                "# Fichier",
                "Nouveau",
                "# Fichier › Récents",
                "a.txt"
            ]
        );
    }

    #[test]
    fn sous_menus_vides_reperes() {
        let tree = vec![
            sub(2, "Vide", vec![]),
            sub(3, "Plein", vec![item(4, "x"), sub(5, "Vide aussi", vec![])]),
        ];
        let mut ids = Vec::new();
        empty_submenus(&tree, &mut ids);
        assert_eq!(ids, [2, 5]);
    }

    use super::*;

    #[tokio::test]
    #[ignore = "exécuter sous dbus-run-session, jamais sur le bus de la session utilisateur"]
    async fn native_watcher_lifecycle() {
        assert!(items().await.is_empty());
        let watcher = bus().await.unwrap();
        let app = zbus::Connection::session().await.unwrap();
        let name = app.unique_name().unwrap().to_string();
        app.call_method(
            Some(WATCHER),
            "/StatusNotifierWatcher",
            Some(WATCHER),
            "RegisterStatusNotifierItem",
            &("/Demo",),
        )
        .await
        .unwrap();
        assert_eq!(
            registered(watcher).await.unwrap(),
            vec![format!("{name}/Demo")]
        );
        app.close().await.unwrap();
        assert!(items().await.is_empty());
        assert!(registered(watcher).await.unwrap().is_empty());
        // Même un watcher externe remplaçable et momentanément indisponible reste propriétaire.
        watcher.release_name(WATCHER).await.unwrap();
        let external = zbus::Connection::session().await.unwrap();
        external
            .object_server()
            .at("/OtherObject", Watcher::default())
            .await
            .unwrap();
        external
            .request_name_with_flags(
                WATCHER,
                zbus::fdo::RequestNameFlags::AllowReplacement
                    | zbus::fdo::RequestNameFlags::DoNotQueue,
            )
            .await
            .unwrap();
        let expected = external.unique_name().unwrap().to_string();
        assert!(items().await.is_empty());
        let reply = watcher
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "GetNameOwner",
                &(WATCHER,),
            )
            .await
            .unwrap();
        assert_eq!(reply.body().deserialize::<String>().unwrap(), expected);
        external.close().await.unwrap();
        assert!(items().await.is_empty());
    }

    #[test]
    fn grandes_images_reduites() {
        let px = |n: i32| (n, n, vec![255u8; (n * n * 4) as usize]);
        // Seule une 1024×1024 : retenue puis réduite.
        let (w, h, d) = pick_pixmap(vec![px(1024)]).unwrap();
        assert_eq!((w, h), (1024, 1024));
        let (w, h, d) = shrink(w, h, argb_to_rgba(&d));
        assert_eq!((w, h, d.len()), (64, 64, 64 * 64 * 4));
        // La plus petite assez grande, sinon la plus grande.
        assert_eq!(pick_pixmap(vec![px(16), px(256), px(64)]).unwrap().0, 64);
        assert_eq!(pick_pixmap(vec![px(16), px(32)]).unwrap().0, 32);
        // Taille incohérente : ignorée.
        assert!(pick_pixmap(vec![(8, 8, vec![0; 10])]).is_none());
    }

    #[test]
    fn services_pixels_et_libelles() {
        assert_eq!(
            split_service(":1.4/org/ayatana/NotificationItem/x"),
            (":1.4".into(), "/org/ayatana/NotificationItem/x".into())
        );
        assert_eq!(
            split_service("org.kde.StatusNotifierItem-1-1"),
            (
                "org.kde.StatusNotifierItem-1-1".into(),
                "/StatusNotifierItem".into()
            )
        );
        assert_eq!(argb_to_rgba(&[255, 10, 20, 30]), vec![10, 20, 30, 255]);
        assert_eq!(clean_label("_Fichier"), "Fichier");
        assert_eq!(clean_label("a__b"), "a_b");
    }
}
