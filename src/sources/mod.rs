//! Sources d'état système tenues à jour par D-Bus, lues sans processus par
//! les widgets Lua (MLG-134).
//!
//! Avant, les widgets intégrés de la barre sondaient par sous-processus à
//! chaque intervalle (`nmcli`, `bluetoothctl`, `powerprofilesctl`,
//! `hyprdesk index status`…) : 150 à 200 lancements par minute avec la barre
//! par défaut, sur le fil Lua unique. Ici, un fil `hyprdesk-sources` suit
//! NetworkManager, BlueZ et power-profiles-daemon sur le bus système (signaux
//! `PropertiesChanged`, `InterfacesAdded/Removed`, changements de
//! propriétaire), et garde des instantanés que `hd.network()`,
//! `hd.bluetooth()`, `hd.power_profile()` rendent aussitôt.
//!
//! Chaque source démarre au premier appel de sa fonction Lua (une barre sans
//! widget Bluetooth ne suit pas BlueZ). Service absent ou bus injoignable :
//! la fonction rend `nil`, et le widget reprend son ancien chemin `hd.exec`.
//!
//! S'y ajoutent des lectures directes sans processus : batteries
//! (`/sys/class/power_supply`), état du démon d'index (son socket) et état
//! des notifications quand hyprdesk tient lui-même le serveur.

mod bluetooth;
mod network;
mod power;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Once, RwLock};
use std::time::{Duration, Instant};

use iced::futures::StreamExt;
use iced::futures::future::BoxFuture;
use mlua::{Lua, Table, Value};
use zbus::zvariant::{self, OwnedValue};

/// Attente du premier instantané au démarrage d'une source : le premier
/// rendu du widget profite déjà de D-Bus.
const FIRST_WAIT: Duration = Duration::from_millis(600);

/// Instantané partagé d'une source, écrit par le fil des sources et lu par
/// le fil Lua.
pub(crate) struct Slot<T> {
    value: RwLock<Option<T>>,
    /// Premier état connu (instantané ou absence du service).
    ready: AtomicBool,
    started: Once,
}

impl<T> Slot<T> {
    pub(crate) const fn new() -> Self {
        Slot {
            value: RwLock::new(None),
            ready: AtomicBool::new(false),
            started: Once::new(),
        }
    }

    pub(crate) fn set(&self, v: Option<T>) {
        *self.value.write().unwrap_or_else(|e| e.into_inner()) = v;
        self.ready.store(true, Ordering::Release);
    }

    /// Modifie l'instantané en place, s'il y en a un.
    pub(crate) fn update(&self, f: impl FnOnce(&mut T)) {
        if let Some(v) = self
            .value
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            f(v);
        }
    }

    pub(crate) fn read<R>(&self, f: impl FnOnce(Option<&T>) -> R) -> R {
        f(self
            .value
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref())
    }

    /// Lance la source une seule fois, puis attend brièvement son premier état.
    fn ensure(&'static self, start: impl FnOnce() -> bool) {
        self.started.call_once(|| {
            if !enabled() || !start() {
                self.ready.store(true, Ordering::Release);
                return;
            }
            let deadline = Instant::now() + FIRST_WAIT;
            while !self.ready.load(Ordering::Acquire) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
        });
    }
}

/// Les tests n'ouvrent jamais le bus système : les sources y restent absentes.
fn enabled() -> bool {
    !cfg!(test)
}

/// Fil dédié aux sources (un seul fil de travail : tout y est attente).
fn spawn(fut: impl std::future::Future<Output = ()> + Send + 'static) -> bool {
    static RT: std::sync::LazyLock<Option<tokio::runtime::Runtime>> =
        std::sync::LazyLock::new(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .thread_name("hyprdesk-sources")
                .enable_all()
                .build()
                .map_err(|e| tracing::warn!("sources D-Bus : {e}"))
                .ok()
        });
    match RT.as_ref() {
        Some(rt) => {
            rt.spawn(fut);
            true
        }
        None => false,
    }
}

/// Connexion au bus système, partagée par les sources.
async fn system_bus() -> Option<zbus::Connection> {
    static BUS: tokio::sync::OnceCell<Option<zbus::Connection>> =
        tokio::sync::OnceCell::const_new();
    BUS.get_or_init(|| async {
        zbus::Connection::system()
            .await
            .map_err(|e| tracing::debug!("sources D-Bus : bus système injoignable : {e}"))
            .ok()
    })
    .await
    .clone()
}

/// Suivi d'un service : la session tourne tant que l'un des `names` a un
/// propriétaire ; elle est relancée quand il change (redémarrage du
/// service), et l'instantané vidé pendant son absence.
pub(crate) type Session =
    fn(zbus::Connection, &'static str) -> BoxFuture<'static, zbus::Result<()>>;

async fn follow<T: Send + Sync + 'static>(
    slot: &'static Slot<T>,
    names: &'static [&'static str],
    session: Session,
) {
    let Some(conn) = system_bus().await else {
        slot.set(None);
        return;
    };
    let dbus = match zbus::fdo::DBusProxy::new(&conn).await {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!("sources D-Bus : {e}");
            slot.set(None);
            return;
        }
    };
    let mut streams = Vec::new();
    for name in names {
        match dbus
            .receive_name_owner_changed_with_args(&[(0, *name)])
            .await
        {
            Ok(s) => streams.push(Box::pin(s)),
            Err(e) => {
                tracing::debug!("sources D-Bus : {name} : {e}");
                slot.set(None);
                return;
            }
        }
    }
    let mut owners = iced::futures::stream::select_all(streams);
    loop {
        let mut owner = None;
        for name in names {
            let Ok(bus_name) = zbus::names::BusName::try_from(*name) else {
                continue;
            };
            if dbus.name_has_owner(bus_name).await.unwrap_or(false) {
                owner = Some(*name);
                break;
            }
        }
        let Some(name) = owner else {
            slot.set(None);
            // Service absent : on attend son arrivée (nouvel essai par minute).
            if let Ok(None) = tokio::time::timeout(Duration::from_secs(60), owners.next()).await {
                return;
            }
            continue;
        };
        let by_owner = tokio::select! {
            r = session(conn.clone(), name) => {
                if let Err(e) = r {
                    tracing::debug!("sources D-Bus : {name} : {e}");
                }
                false
            }
            o = owners.next() => {
                if o.is_none() {
                    slot.set(None);
                    return;
                }
                true
            }
        };
        slot.set(None);
        if !by_owner {
            // Session en erreur, service toujours là : nouvel essai sans s'emballer.
            if let Ok(None) = tokio::time::timeout(Duration::from_secs(10), owners.next()).await {
                return;
            }
        }
    }
}

/// Flux des `PropertiesChanged` d'un service (et de ses objets sous `namespace`).
pub(crate) async fn properties_changed(
    conn: &zbus::Connection,
    sender: &'static str,
    namespace: &'static str,
) -> zbus::Result<zbus::MessageStream> {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(sender)?
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path_namespace(namespace)?
        .build();
    zbus::MessageStream::for_match_rule(rule, conn, Some(256)).await
}

/// Corps d'un `PropertiesChanged` : (interface, propriétés changées, invalidées).
type Changed = (String, Props, Vec<String>);

/// Propriétés d'une interface D-Bus (`GetAll`).
pub(crate) type Props = HashMap<String, OwnedValue>;

async fn get_all(
    conn: &zbus::Connection,
    dest: &'static str,
    path: &str,
    iface: &str,
) -> zbus::Result<Props> {
    conn.call_method(
        Some(dest),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "GetAll",
        &(iface,),
    )
    .await?
    .body()
    .deserialize()
}

// Décodage des valeurs D-Bus, en traversant les variants imbriqués
// (`a{sv}` dans un tableau, par exemple).
fn plain<'v, 'a>(v: &'v zvariant::Value<'a>) -> &'v zvariant::Value<'a> {
    match v {
        zvariant::Value::Value(inner) => plain(inner),
        o => o,
    }
}

pub(crate) fn prop_str(p: &Props, k: &str) -> Option<String> {
    match plain(p.get(k)?) {
        zvariant::Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

pub(crate) fn prop_bool(p: &Props, k: &str) -> Option<bool> {
    match plain(p.get(k)?) {
        zvariant::Value::Bool(b) => Some(*b),
        _ => None,
    }
}

/// Entier non signé, quelle que soit sa largeur (octet, u16, u32…).
pub(crate) fn prop_uint(p: &Props, k: &str) -> Option<u64> {
    value_uint(plain(p.get(k)?))
}

fn value_uint(v: &zvariant::Value<'_>) -> Option<u64> {
    match plain(v) {
        zvariant::Value::U8(n) => Some(u64::from(*n)),
        zvariant::Value::U16(n) => Some(u64::from(*n)),
        zvariant::Value::U32(n) => Some(u64::from(*n)),
        zvariant::Value::U64(n) => Some(*n),
        zvariant::Value::I16(n) => u64::try_from(*n).ok(),
        zvariant::Value::I32(n) => u64::try_from(*n).ok(),
        zvariant::Value::I64(n) => u64::try_from(*n).ok(),
        _ => None,
    }
}

pub(crate) fn prop_path(p: &Props, k: &str) -> Option<String> {
    match plain(p.get(k)?) {
        zvariant::Value::ObjectPath(o) => Some(o.to_string()),
        _ => None,
    }
}

pub(crate) fn prop_paths(p: &Props, k: &str) -> Option<Vec<String>> {
    match plain(p.get(k)?) {
        zvariant::Value::Array(a) => Some(
            a.inner()
                .iter()
                .filter_map(|v| match plain(v) {
                    zvariant::Value::ObjectPath(o) => Some(o.to_string()),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    }
}

pub(crate) fn prop_bytes(p: &Props, k: &str) -> Option<Vec<u8>> {
    match plain(p.get(k)?) {
        zvariant::Value::Array(a) => Some(
            a.inner()
                .iter()
                .filter_map(|v| match plain(v) {
                    zvariant::Value::U8(b) => Some(*b),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Tableau de dictionnaires `aa{sv}` : la valeur de `field` dans chacun.
pub(crate) fn prop_dict_field(p: &Props, k: &str, field: &str) -> Option<Vec<String>> {
    let zvariant::Value::Array(a) = plain(p.get(k)?) else {
        return None;
    };
    Some(
        a.inner()
            .iter()
            .filter_map(|d| {
                let zvariant::Value::Dict(d) = plain(d) else {
                    return None;
                };
                d.iter()
                    .find_map(|(key, val)| match (plain(key), plain(val)) {
                        (zvariant::Value::Str(key), zvariant::Value::Str(val))
                            if key.as_str() == field =>
                        {
                            Some(val.to_string())
                        }
                        _ => None,
                    })
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// Lectures sans D-Bus.

/// Batterie lue dans `/sys/class/power_supply`.
#[derive(Debug, Clone, PartialEq)]
pub struct Battery {
    pub name: String,
    pub cap: i64,
    pub status: String,
    /// Périphérique (souris, clavier…) plutôt que batterie de l'ordinateur.
    pub device: bool,
}

/// Batteries sous `root` (`/sys/class/power_supply`), dans l'ordre des noms.
pub fn batteries(root: &std::path::Path) -> Vec<Battery> {
    let read = |p: std::path::PathBuf| {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
    };
    let mut names: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let d = root.join(&name);
            if read(d.join("type")).as_deref() != Some("Battery") {
                return None;
            }
            let cap = read(d.join("capacity"))?.parse().ok()?;
            Some(Battery {
                name: read(d.join("model_name"))
                    .filter(|s| !s.is_empty())
                    .unwrap_or(name),
                cap,
                status: read(d.join("status")).unwrap_or_default(),
                device: read(d.join("scope")).as_deref() == Some("Device"),
            })
        })
        .collect()
}

/// État du démon d'index (JSON de `Request::Status`), sans lancer `hyprdesk` ;
/// `None` si le démon ne répond pas (délai court : le fil Lua n'attend pas).
fn index_status() -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("hyprdesk-index-status".into())
        .spawn(move || {
            let r = crate::fileindex::proto::request(&crate::fileindex::proto::Request::Status);
            let _ = tx.send(r);
        })
        .ok()?;
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(crate::fileindex::proto::Reply::Status(json))) => Some(json),
        _ => None,
    }
}

/// Table Lua de l'état de l'index : `phase`, `fichiers`, `progression`.
fn index_table(lua: &Lua, json: &str) -> mlua::Result<Value> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Ok(Value::Nil);
    };
    let t = lua.create_table()?;
    t.set("phase", v["phase"].as_str().unwrap_or("?"))?;
    t.set("fichiers", v["fichiers"].as_u64().unwrap_or(0))?;
    t.set("progression", v["progression"].as_u64().unwrap_or(0))?;
    Ok(Value::Table(t))
}

/// Notifications vues par la barre quand hyprdesk tient le serveur : l'état
/// publié par `ui::toasts` (`notifications::render_state`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Notifs {
    pub dnd: bool,
    /// Fin programmée du Silence (HIG PAT-04) : échéance en secondes depuis
    /// l'époque ; `None` quand le Silence n'a pas de fin.
    pub dnd_until: Option<i64>,
    pub count: u64,
    /// (application, titre, texte), les plus récentes d'abord.
    pub list: Vec<(String, String, String)>,
}

pub fn parse_notifications(s: &str) -> Notifs {
    let mut n = Notifs::default();
    for line in s.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k {
            "dnd" => n.dnd = v == "1",
            "dnd_until" => n.dnd_until = v.parse().ok(),
            "count" => n.count = v.parse().unwrap_or(0),
            "n" => {
                let mut f = v.splitn(3, '\t');
                let app = f.next().unwrap_or_default().to_string();
                let title = f.next().unwrap_or_default().to_string();
                let body = f.next().unwrap_or_default().to_string();
                n.list.push((app, title, body));
            }
            _ => {}
        }
    }
    n
}

fn notifications_table(lua: &Lua, n: &Notifs) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("source", "hyprdesk")?;
    t.set("dnd", n.dnd)?;
    t.set("dnd_until", n.dnd_until)?;
    t.set("count", n.count)?;
    let list = lua.create_table()?;
    for (i, (app, title, body)) in n.list.iter().enumerate() {
        let e = lua.create_table()?;
        e.set("app", app.as_str())?;
        e.set("title", title.as_str())?;
        e.set("body", body.as_str())?;
        list.set(i + 1, e)?;
    }
    t.set("list", list)?;
    Ok(t)
}

fn batteries_table(lua: &Lua, list: &[Battery]) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for (i, b) in list.iter().enumerate() {
        let e = lua.create_table()?;
        e.set("name", b.name.as_str())?;
        e.set("cap", b.cap)?;
        e.set("status", b.status.as_str())?;
        e.set("device", b.device)?;
        t.set(i + 1, e)?;
    }
    Ok(t)
}

/// Fonctions `hd.*` sans processus (documentées dans `assets/hyprdesk.meta.lua`).
pub fn install(lua: &Lua, hd: &Table) -> mlua::Result<()> {
    hd.set(
        "network",
        lua.create_function(|lua, ()| {
            network::SLOT.ensure(|| spawn(network::run()));
            network::SLOT.read(|s| match s {
                Some(s) => network::to_lua(lua, s).map(Value::Table),
                None => Ok(Value::Nil),
            })
        })?,
    )?;
    hd.set(
        "bluetooth",
        lua.create_function(|lua, ()| {
            bluetooth::SLOT.ensure(|| spawn(bluetooth::run()));
            bluetooth::SLOT.read(|s| match s {
                Some(s) => bluetooth::to_lua(lua, s).map(Value::Table),
                None => Ok(Value::Nil),
            })
        })?,
    )?;
    hd.set(
        "power_profile",
        lua.create_function(|lua, ()| {
            power::SLOT.ensure(|| spawn(power::run()));
            power::SLOT.read(|s| match s {
                Some(s) => power::to_lua(lua, s).map(Value::Table),
                None => Ok(Value::Nil),
            })
        })?,
    )?;
    hd.set(
        "batteries",
        lua.create_function(|lua, ()| {
            batteries_table(
                lua,
                &batteries(std::path::Path::new("/sys/class/power_supply")),
            )
        })?,
    )?;
    hd.set(
        "index_status",
        lua.create_function(|lua, ()| match index_status() {
            Some(json) => index_table(lua, &json),
            None => Ok(Value::Nil),
        })?,
    )?;
    // GPU NVIDIA basculables vfio-pci ⇄ nvidia (MLG-162), lus dans sysfs.
    hd.set(
        "gpus",
        lua.create_function(|lua, ()| {
            let busy = crate::gpu::busy();
            let gpus = crate::gpu::list();
            // Sondé seulement s'il y a un GPU à basculer (puis gardé en cache).
            let ready = !gpus.is_empty() && crate::gpu::switch_bin().is_some();
            let t = lua.create_table()?;
            for (i, g) in gpus.iter().enumerate() {
                let e = lua.create_table()?;
                e.set("address", g.address.as_str())?;
                e.set("name", g.name.as_str())?;
                e.set("driver", g.driver.as_str())?;
                e.set("state", g.state())?;
                e.set("boot", g.boot().as_str())?;
                e.set("boot_default", g.boot_default.as_str())?;
                e.set("boot_saved", g.boot_saved.is_some())?;
                e.set("nvidia_module", g.nvidia_module)?;
                e.set("display", g.display)?;
                e.set("busy", busy)?;
                e.set("ready", ready)?;
                t.set(i + 1, e)?;
            }
            Ok(t)
        })?,
    )?;
    hd.set(
        "notifications",
        lua.create_function(|lua, ()| {
            if !crate::notifications::SERVED.load(Ordering::Relaxed) {
                return Ok(Value::Nil);
            }
            let s = std::fs::read_to_string(crate::notifications::state_path()).unwrap_or_default();
            notifications_table(lua, &parse_notifications(&s)).map(Value::Table)
        })?,
    )?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn props(pairs: Vec<(&str, OwnedValue)>) -> Props {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    pub(crate) fn s(v: &str) -> OwnedValue {
        OwnedValue::from(zvariant::Str::from(v.to_string()))
    }

    pub(crate) fn path(v: &str) -> OwnedValue {
        OwnedValue::from(zvariant::ObjectPath::try_from(v.to_string()).unwrap())
    }

    pub(crate) fn paths(v: &[&str]) -> OwnedValue {
        let list: Vec<zvariant::ObjectPath<'_>> = v
            .iter()
            .map(|p| zvariant::ObjectPath::try_from(*p).unwrap())
            .collect();
        OwnedValue::try_from(zvariant::Value::from(list)).unwrap()
    }

    pub(crate) fn bytes(v: &[u8]) -> OwnedValue {
        OwnedValue::try_from(zvariant::Value::from(v.to_vec())).unwrap()
    }

    #[test]
    fn decodage_des_valeurs() {
        let p = props(vec![
            ("a", s("x")),
            ("b", OwnedValue::from(true)),
            ("c", OwnedValue::from(70u32)),
            ("d", OwnedValue::from(42u8)),
            ("e", path("/o/1")),
            ("f", paths(&["/o/1", "/o/2"])),
            ("g", bytes(b"Maison")),
            // Variant dans un variant : décodé comme la valeur qu'il porte.
            (
                "h",
                OwnedValue::try_from(zvariant::Value::Value(Box::new(zvariant::Value::from("y"))))
                    .unwrap(),
            ),
        ]);
        assert_eq!(prop_str(&p, "a").as_deref(), Some("x"));
        assert_eq!(prop_bool(&p, "b"), Some(true));
        assert_eq!(prop_uint(&p, "c"), Some(70));
        assert_eq!(prop_uint(&p, "d"), Some(42));
        assert_eq!(prop_path(&p, "e").as_deref(), Some("/o/1"));
        assert_eq!(prop_paths(&p, "f").unwrap(), vec!["/o/1", "/o/2"]);
        assert_eq!(prop_bytes(&p, "g").unwrap(), b"Maison");
        assert_eq!(prop_str(&p, "h").as_deref(), Some("y"));
        // Type inattendu ou clé absente : rien, sans panique.
        assert_eq!(prop_bool(&p, "a"), None);
        assert_eq!(prop_str(&p, "zz"), None);
    }

    #[test]
    fn batteries_lues_dans_sysfs() {
        let root =
            std::env::temp_dir().join(format!("hyprdesk-power-supply-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mk = |name: &str, files: &[(&str, &str)]| {
            let d = root.join(name);
            std::fs::create_dir_all(&d).unwrap();
            for (f, v) in files {
                std::fs::write(d.join(f), format!("{v}\n")).unwrap();
            }
        };
        mk("AC", &[("type", "Mains"), ("online", "1")]);
        mk(
            "BAT0",
            &[
                ("type", "Battery"),
                ("capacity", "57"),
                ("status", "Discharging"),
            ],
        );
        mk(
            "hidpp_battery_0",
            &[
                ("type", "Battery"),
                ("capacity", "80"),
                ("status", "Discharging"),
                ("scope", "Device"),
                ("model_name", "MX Master 3"),
            ],
        );
        // Sans capacité lisible : ignorée.
        mk("BAT9", &[("type", "Battery")]);
        let b = batteries(&root);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            b,
            vec![
                Battery {
                    name: "BAT0".into(),
                    cap: 57,
                    status: "Discharging".into(),
                    device: false
                },
                Battery {
                    name: "MX Master 3".into(),
                    cap: 80,
                    status: "Discharging".into(),
                    device: true
                },
            ]
        );
        assert!(batteries(std::path::Path::new("/chemin/absent")).is_empty());
    }

    #[test]
    fn etat_des_notifications_relu() {
        let n = crate::notifications::Notification {
            action_icons: Vec::new(),
            id: 1,
            app: "kitty".into(),
            icon: String::new(),
            summary: "Fini".into(),
            body: "cargo test".into(),
            actions: Vec::new(),
            urgency: crate::notifications::Urgency::Normal,
            timeout_ms: -1,
            resident: false,
            transient: false,
            desktop_entry: None,
            category: None,
            unread: false,
        };
        let st = parse_notifications(&crate::notifications::render_state(
            true,
            Some(1_000_600),
            &[n],
        ));
        assert_eq!(
            st,
            Notifs {
                dnd: true,
                dnd_until: Some(1_000_600),
                count: 1,
                list: vec![("kitty".into(), "Fini".into(), "cargo test".into())],
            }
        );
        assert_eq!(parse_notifications(""), Notifs::default());
    }

    #[test]
    fn etat_de_l_index_en_table() {
        let lua = Lua::new();
        let Value::Table(t) =
            index_table(&lua, r#"{"phase":"scan","fichiers":3,"progression":1200}"#).unwrap()
        else {
            panic!()
        };
        assert_eq!(t.get::<String>("phase").unwrap(), "scan");
        assert_eq!(t.get::<u64>("progression").unwrap(), 1200);
        assert!(matches!(
            index_table(&lua, "pas du json").unwrap(),
            Value::Nil
        ));
    }

    /// Sans service (les tests n'ouvrent pas le bus) : `nil`, et le widget
    /// garde son ancien chemin.
    #[test]
    fn fonctions_lua_sans_service() {
        let lua = Lua::new();
        let hd = lua.create_table().unwrap();
        install(&lua, &hd).unwrap();
        lua.globals().set("hd", hd).unwrap();
        let (net, bt, pp, bat): (Value, Value, Value, Table) = lua
            .load("return hd.network(), hd.bluetooth(), hd.power_profile(), hd.batteries()")
            .eval()
            .unwrap();
        assert!(matches!(net, Value::Nil));
        assert!(matches!(bt, Value::Nil));
        assert!(matches!(pp, Value::Nil));
        // Liste (vide ou non selon la machine), jamais nil.
        let _ = bat.raw_len();
    }
}
