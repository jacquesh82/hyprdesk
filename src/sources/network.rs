//! NetworkManager (`org.freedesktop.NetworkManager`, bus système) : état
//! global, Wi-Fi activé, connexions actives (nom, type) et, pour le Wi-Fi,
//! SSID et force du point d'accès actif.
//!
//! Mise à jour par `PropertiesChanged` : un signal de NetworkManager, d'une
//! connexion active ou d'un de leurs appareils relit l'état (quelques appels
//! D-Bus, aucun processus) ; un signal du point d'accès actif met à jour sa
//! force sur place. Les autres points d'accès (balayage) sont ignorés.

use iced::futures::{FutureExt, StreamExt};
use mlua::{Lua, Table};

use super::{Changed, Props, Slot, get_all, prop_bool, prop_bytes, prop_path, prop_paths};
use super::{prop_str, prop_uint};

const NM: &str = "org.freedesktop.NetworkManager";
const ROOT: &str = "/org/freedesktop/NetworkManager";
const IFACE_ACTIVE: &str = "org.freedesktop.NetworkManager.Connection.Active";
const IFACE_WIRELESS: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const IFACE_AP: &str = "org.freedesktop.NetworkManager.AccessPoint";

/// `NM_ACTIVE_CONNECTION_STATE_ACTIVATED`.
const ACTIVATED: u64 = 2;
/// `NM_ACTIVATION_STATE_FLAG_EXTERNAL` : connexion créée hors de
/// NetworkManager (pont docker…), que `nmcli device` ne dit pas « connected ».
const FLAG_EXTERNAL: u64 = 0x80;

pub(super) static SLOT: Slot<Network> = Slot::new();

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Network {
    /// `NMState` (70 : connecté à Internet).
    pub state: u64,
    pub wifi_enabled: bool,
    /// Chemin de la connexion primaire (`/` : aucune).
    pub primary: String,
    pub connections: Vec<Active>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Active {
    pub path: String,
    pub name: String,
    /// `wifi`, `ethernet`, `vpn`, `wireguard`… (type NetworkManager sinon).
    pub kind: String,
    pub state: u64,
    pub external: bool,
    pub devices: Vec<String>,
    /// Point d'accès suivi (Wi-Fi).
    pub ap: Option<String>,
    pub ssid: Option<String>,
    pub strength: Option<u8>,
}

/// Nom court du type de connexion NetworkManager.
pub fn kind(nm_type: &str) -> String {
    match nm_type {
        "802-11-wireless" => "wifi",
        "802-3-ethernet" => "ethernet",
        o => o,
    }
    .to_string()
}

/// `NMState` en mot.
pub fn state_name(state: u64) -> &'static str {
    match state {
        10 => "asleep",
        20 => "disconnected",
        30 => "disconnecting",
        40 => "connecting",
        50 => "connected-local",
        60 => "connected-site",
        70 => "connected",
        _ => "unknown",
    }
}

/// Objet racine : état, Wi-Fi, connexion primaire ; rend aussi les chemins
/// des connexions actives.
pub fn decode_root(p: &Props) -> (Network, Vec<String>) {
    (
        Network {
            state: prop_uint(p, "State").unwrap_or(0),
            wifi_enabled: prop_bool(p, "WirelessEnabled").unwrap_or(false),
            primary: prop_path(p, "PrimaryConnection").unwrap_or_else(|| "/".into()),
            connections: Vec::new(),
        },
        prop_paths(p, "ActiveConnections").unwrap_or_default(),
    )
}

pub fn decode_active(path: &str, p: &Props) -> Active {
    Active {
        path: path.to_string(),
        name: prop_str(p, "Id").unwrap_or_default(),
        kind: kind(&prop_str(p, "Type").unwrap_or_default()),
        state: prop_uint(p, "State").unwrap_or(0),
        external: prop_uint(p, "StateFlags").unwrap_or(0) & FLAG_EXTERNAL != 0,
        devices: prop_paths(p, "Devices").unwrap_or_default(),
        ..Active::default()
    }
}

/// Propriétés (toutes ou changées) du point d'accès actif.
pub fn apply_ap(a: &mut Active, p: &Props) {
    if let Some(s) = prop_bytes(p, "Ssid") {
        a.ssid = Some(String::from_utf8_lossy(&s).into_owned());
    }
    if let Some(s) = prop_uint(p, "Strength") {
        a.strength = Some(s.min(100) as u8);
    }
}

/// Ce que demande un signal venu de `path`.
#[derive(Debug, PartialEq)]
pub enum Route {
    /// Point d'accès actif de la connexion n° : mise à jour sur place.
    Ap(usize),
    /// État à relire.
    Refresh,
    Ignore,
}

pub fn route(n: &Network, path: &str) -> Route {
    if path == ROOT {
        return Route::Refresh;
    }
    if let Some(i) = n
        .connections
        .iter()
        .position(|c| c.ap.as_deref() == Some(path))
    {
        return Route::Ap(i);
    }
    if n.connections
        .iter()
        .any(|c| c.path == path || c.devices.iter().any(|d| d == path))
    {
        return Route::Refresh;
    }
    Route::Ignore
}

/// Relit tout l'état (chemins suivis compris).
async fn fetch(conn: &zbus::Connection) -> zbus::Result<Network> {
    let (mut net, actives) = decode_root(&get_all(conn, NM, ROOT, NM).await?);
    for path in actives {
        // Une connexion peut disparaître entre deux appels : on la saute.
        let Ok(p) = get_all(conn, NM, &path, IFACE_ACTIVE).await else {
            continue;
        };
        let mut a = decode_active(&path, &p);
        if a.kind == "wifi"
            && let Some(dev) = a.devices.first()
            && let Ok(w) = get_all(conn, NM, dev, IFACE_WIRELESS).await
            && let Some(ap) = prop_path(&w, "ActiveAccessPoint").filter(|p| p != "/")
        {
            if let Ok(props) = get_all(conn, NM, &ap, IFACE_AP).await {
                apply_ap(&mut a, &props);
            }
            a.ap = Some(ap);
        }
        net.connections.push(a);
    }
    Ok(net)
}

async fn session(conn: zbus::Connection) -> zbus::Result<()> {
    // Abonnement avant la première lecture : aucun changement perdu entre les deux.
    let mut stream = super::properties_changed(&conn, NM, ROOT).await?;
    let mut net = fetch(&conn).await?;
    SLOT.set(Some(net.clone()));
    while let Some(msg) = stream.next().await {
        let msg = msg?;
        let path = msg
            .header()
            .path()
            .map(|p| p.to_string())
            .unwrap_or_default();
        match route(&net, &path) {
            Route::Ignore => {}
            Route::Ap(i) => {
                let Ok((_, changed, _)) = msg.body().deserialize::<Changed>() else {
                    continue;
                };
                apply_ap(&mut net.connections[i], &changed);
                SLOT.update(|s| {
                    if let Some(c) = s.connections.get_mut(i) {
                        apply_ap(c, &changed);
                    }
                });
            }
            Route::Refresh => {
                // Une activation envoie une rafale de signaux : on la laisse passer.
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                while let Some(Some(_)) = stream.next().now_or_never() {}
                net = fetch(&conn).await?;
                SLOT.set(Some(net.clone()));
            }
        }
    }
    Ok(())
}

pub(super) async fn run() {
    super::follow(&SLOT, &[NM], |conn, _| session(conn).boxed()).await;
}

fn active_table(lua: &Lua, a: &Active, primary: bool) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("name", a.name.as_str())?;
    t.set("type", a.kind.as_str())?;
    t.set("active", a.state == ACTIVATED)?;
    t.set("external", a.external)?;
    t.set("primary", primary)?;
    if let Some(s) = &a.ssid {
        t.set("ssid", s.as_str())?;
    }
    if let Some(s) = a.strength {
        t.set("strength", s)?;
    }
    Ok(t)
}

/// Table de `hd.network()`.
pub fn to_lua(lua: &Lua, n: &Network) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("state", state_name(n.state))?;
    t.set("connected", n.state >= 50)?;
    t.set("wifi_enabled", n.wifi_enabled)?;
    let list = lua.create_table()?;
    for (i, a) in n.connections.iter().enumerate() {
        let primary = a.path == n.primary;
        let e = active_table(lua, a, primary)?;
        if primary {
            t.set("primary", e.clone())?;
        }
        list.set(i + 1, e)?;
    }
    t.set("connections", list)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{bytes, path, paths, props, s};
    use super::*;
    use zbus::zvariant::OwnedValue;

    fn sample() -> Network {
        let (mut n, actives) = decode_root(&props(vec![
            ("State", OwnedValue::from(70u32)),
            ("WirelessEnabled", OwnedValue::from(true)),
            (
                "PrimaryConnection",
                path("/org/freedesktop/NetworkManager/ActiveConnection/2"),
            ),
            (
                "ActiveConnections",
                paths(&[
                    "/org/freedesktop/NetworkManager/ActiveConnection/1",
                    "/org/freedesktop/NetworkManager/ActiveConnection/2",
                ]),
            ),
        ]));
        assert_eq!(actives.len(), 2);
        n.connections.push(decode_active(
            &actives[0],
            &props(vec![
                ("Id", s("docker0")),
                ("Type", s("bridge")),
                ("State", OwnedValue::from(2u32)),
                ("StateFlags", OwnedValue::from(0x80u32 | 0x04)),
                (
                    "Devices",
                    paths(&["/org/freedesktop/NetworkManager/Devices/5"]),
                ),
            ]),
        ));
        let mut wifi = decode_active(
            &actives[1],
            &props(vec![
                ("Id", s("Maison")),
                ("Type", s("802-11-wireless")),
                ("State", OwnedValue::from(2u32)),
                ("StateFlags", OwnedValue::from(0x04u32)),
                (
                    "Devices",
                    paths(&["/org/freedesktop/NetworkManager/Devices/3"]),
                ),
            ]),
        );
        apply_ap(
            &mut wifi,
            &props(vec![
                ("Ssid", bytes(b"Maison 5G")),
                ("Strength", OwnedValue::from(64u8)),
            ]),
        );
        wifi.ap = Some("/org/freedesktop/NetworkManager/AccessPoint/9".into());
        n.connections.push(wifi);
        n
    }

    #[test]
    fn instantane_networkmanager() {
        let n = sample();
        assert_eq!(state_name(n.state), "connected");
        assert!(n.wifi_enabled);
        assert_eq!(n.connections[0].kind, "bridge");
        assert!(n.connections[0].external);
        let w = &n.connections[1];
        assert_eq!((w.name.as_str(), w.kind.as_str()), ("Maison", "wifi"));
        assert!(!w.external);
        assert_eq!(w.ssid.as_deref(), Some("Maison 5G"));
        assert_eq!(w.strength, Some(64));
        assert_eq!(kind("802-3-ethernet"), "ethernet");
        assert_eq!(kind("wireguard"), "wireguard");
        // Racine vide (propriétés manquantes) : valeurs neutres.
        let (vide, actives) = decode_root(&Props::new());
        assert_eq!(
            (vide.state, vide.wifi_enabled, vide.primary.as_str()),
            (0, false, "/")
        );
        assert!(actives.is_empty());
    }

    #[test]
    fn signaux_routes() {
        let mut n = sample();
        assert_eq!(route(&n, ROOT), Route::Refresh);
        assert_eq!(
            route(&n, "/org/freedesktop/NetworkManager/AccessPoint/9"),
            Route::Ap(1)
        );
        // Autre point d'accès (balayage) : ignoré.
        assert_eq!(
            route(&n, "/org/freedesktop/NetworkManager/AccessPoint/10"),
            Route::Ignore
        );
        assert_eq!(
            route(&n, "/org/freedesktop/NetworkManager/Devices/3"),
            Route::Refresh
        );
        assert_eq!(
            route(&n, "/org/freedesktop/NetworkManager/ActiveConnection/1"),
            Route::Refresh
        );
        // Force changée seule : le reste du point d'accès ne bouge pas.
        apply_ap(
            &mut n.connections[1],
            &props(vec![("Strength", OwnedValue::from(91u8))]),
        );
        assert_eq!(n.connections[1].strength, Some(91));
        assert_eq!(n.connections[1].ssid.as_deref(), Some("Maison 5G"));
    }

    #[test]
    fn table_lua() {
        let lua = Lua::new();
        let t = to_lua(&lua, &sample()).unwrap();
        assert_eq!(t.get::<String>("state").unwrap(), "connected");
        assert!(t.get::<bool>("wifi_enabled").unwrap());
        let p: Table = t.get("primary").unwrap();
        assert_eq!(p.get::<String>("type").unwrap(), "wifi");
        assert_eq!(p.get::<u8>("strength").unwrap(), 64);
        let list: Table = t.get("connections").unwrap();
        assert_eq!(list.raw_len(), 2);
        let first: Table = list.get(1).unwrap();
        assert!(first.get::<bool>("external").unwrap());
        assert!(first.get::<bool>("active").unwrap());
    }
}
