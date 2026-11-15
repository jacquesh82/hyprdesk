//! BlueZ (`org.bluez`, bus système) : adaptateur allumé ou non, appareils
//! appairés ou connectés (nom, adresse, batterie `org.bluez.Battery1`).
//!
//! `GetManagedObjects` au départ, puis `InterfacesAdded`/`InterfacesRemoved`
//! et `PropertiesChanged` tiennent le modèle à jour, sans relecture.

use std::collections::{BTreeMap, HashMap};

use iced::futures::{FutureExt, StreamExt};
use mlua::{Lua, Table};
use zbus::zvariant::OwnedObjectPath;

use super::{Changed, Props, Slot, prop_bool, prop_str, prop_uint};

const BLUEZ: &str = "org.bluez";
const IFACE_ADAPTER: &str = "org.bluez.Adapter1";
const IFACE_DEVICE: &str = "org.bluez.Device1";
const IFACE_BATTERY: &str = "org.bluez.Battery1";

pub(super) static SLOT: Slot<Bluetooth> = Slot::new();

/// Instantané rendu par `hd.bluetooth()`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bluetooth {
    /// Au moins un adaptateur.
    pub adapter: bool,
    /// Premier adaptateur (hci0) allumé.
    pub powered: bool,
    /// Appareils appairés ou connectés, par nom.
    pub devices: Vec<Device>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Device {
    pub name: String,
    pub address: String,
    pub paired: bool,
    pub connected: bool,
    pub battery: Option<u8>,
}

/// Un objet BlueZ : ses interfaces utiles, déjà décodées.
#[derive(Debug, Clone, Default, PartialEq)]
struct Object {
    adapter: Option<bool>,
    device: Option<Device>,
    /// Nom de l'appareil (`Name`), quand il n'a pas d'alias.
    name: Option<String>,
    battery: Option<u8>,
}

/// Objets BlueZ par chemin.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model(BTreeMap<String, Object>);

impl Model {
    /// Interface ajoutée ou propriétés changées (fusion).
    pub fn merge(&mut self, path: &str, iface: &str, p: &Props) {
        let o = self.0.entry(path.to_string()).or_default();
        match iface {
            IFACE_ADAPTER => {
                let powered = o.adapter.get_or_insert(false);
                if let Some(v) = prop_bool(p, "Powered") {
                    *powered = v;
                }
            }
            IFACE_DEVICE => {
                let d = o.device.get_or_insert_with(Device::default);
                if let Some(v) = prop_str(p, "Address") {
                    d.address = v;
                }
                if let Some(v) = prop_str(p, "Alias").filter(|v| !v.is_empty()) {
                    d.name = v;
                }
                if let Some(v) = prop_str(p, "Name").filter(|v| !v.is_empty()) {
                    o.name = Some(v);
                }
                if let Some(v) = prop_bool(p, "Paired") {
                    d.paired = v;
                }
                if let Some(v) = prop_bool(p, "Connected") {
                    d.connected = v;
                }
            }
            IFACE_BATTERY => {
                if let Some(v) = prop_uint(p, "Percentage") {
                    o.battery = Some(v.min(100) as u8);
                }
            }
            _ => {}
        }
    }

    pub fn remove(&mut self, path: &str, ifaces: &[String]) {
        let Some(o) = self.0.get_mut(path) else {
            return;
        };
        for i in ifaces {
            match i.as_str() {
                IFACE_ADAPTER => o.adapter = None,
                IFACE_DEVICE => {
                    o.device = None;
                    o.name = None;
                }
                IFACE_BATTERY => o.battery = None,
                _ => {}
            }
        }
        if *o == Object::default() {
            self.0.remove(path);
        }
    }

    pub fn snapshot(&self) -> Bluetooth {
        let mut adapters = self.0.values().filter_map(|o| o.adapter);
        let first = adapters.next();
        let mut devices: Vec<Device> = self
            .0
            .values()
            .filter_map(|o| {
                let mut d = o.device.clone()?;
                if !(d.paired || d.connected) {
                    return None;
                }
                if d.name.is_empty() {
                    d.name = o.name.clone().unwrap_or_else(|| d.address.clone());
                }
                d.battery = o.battery;
                Some(d)
            })
            .collect();
        devices.sort_by_key(|d| d.name.to_lowercase());
        Bluetooth {
            adapter: first.is_some(),
            powered: first.unwrap_or(false),
            devices,
        }
    }
}

type Interfaces = HashMap<String, Props>;

async fn session(conn: zbus::Connection) -> zbus::Result<()> {
    // Abonnements d'abord : aucun changement perdu avant la lecture initiale.
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(BLUEZ)?
        .interface("org.freedesktop.DBus.ObjectManager")?
        .build();
    let objects = zbus::MessageStream::for_match_rule(rule, &conn, Some(256)).await?;
    let props = super::properties_changed(&conn, BLUEZ, "/org/bluez").await?;
    let mut signals = iced::futures::stream::select(objects, props);

    let managed: HashMap<OwnedObjectPath, Interfaces> = conn
        .call_method(
            Some(BLUEZ),
            "/",
            Some("org.freedesktop.DBus.ObjectManager"),
            "GetManagedObjects",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    let mut model = Model::default();
    for (path, ifaces) in &managed {
        for (iface, p) in ifaces {
            model.merge(path.as_str(), iface, p);
        }
    }
    SLOT.set(Some(model.snapshot()));

    while let Some(msg) = signals.next().await {
        let msg = msg?;
        let header = msg.header();
        let member = header.member().map(|m| m.as_str()).unwrap_or_default();
        match member {
            "InterfacesAdded" => {
                if let Ok((path, ifaces)) =
                    msg.body().deserialize::<(OwnedObjectPath, Interfaces)>()
                {
                    for (iface, p) in &ifaces {
                        model.merge(path.as_str(), iface, p);
                    }
                }
            }
            "InterfacesRemoved" => {
                if let Ok((path, ifaces)) =
                    msg.body().deserialize::<(OwnedObjectPath, Vec<String>)>()
                {
                    model.remove(path.as_str(), &ifaces);
                }
            }
            "PropertiesChanged" => {
                let path = header.path().map(|p| p.to_string()).unwrap_or_default();
                if let Ok((iface, changed, _)) = msg.body().deserialize::<Changed>() {
                    model.merge(&path, &iface, &changed);
                }
            }
            _ => continue,
        }
        let snap = model.snapshot();
        // Le RSSI d'un appareil change sans cesse pendant une recherche : on
        // n'écrit que si l'instantané change.
        if SLOT.read(|s| s != Some(&snap)) {
            SLOT.set(Some(snap));
        }
    }
    Ok(())
}

pub(super) async fn run() {
    super::follow(&SLOT, &[BLUEZ], |conn, _| session(conn).boxed()).await;
}

/// Table de `hd.bluetooth()`.
pub fn to_lua(lua: &Lua, b: &Bluetooth) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("adapter", b.adapter)?;
    t.set("powered", b.powered)?;
    let list = lua.create_table()?;
    for (i, d) in b.devices.iter().enumerate() {
        let e = lua.create_table()?;
        e.set("name", d.name.as_str())?;
        e.set("address", d.address.as_str())?;
        e.set("paired", d.paired)?;
        e.set("connected", d.connected)?;
        if let Some(v) = d.battery {
            e.set("battery", v)?;
        }
        list.set(i + 1, e)?;
    }
    t.set("devices", list)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{props, s};
    use super::*;
    use zbus::zvariant::OwnedValue;

    const CASQUE: &str = "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF";

    fn model() -> Model {
        let mut m = Model::default();
        m.merge(
            "/org/bluez/hci0",
            IFACE_ADAPTER,
            &props(vec![("Powered", OwnedValue::from(true))]),
        );
        m.merge(
            CASQUE,
            IFACE_DEVICE,
            &props(vec![
                ("Address", s("AA:BB:CC:DD:EE:FF")),
                ("Alias", s("Casque")),
                ("Paired", OwnedValue::from(true)),
                ("Connected", OwnedValue::from(false)),
            ]),
        );
        // Vu par une recherche, jamais appairé : absent de l'instantané.
        m.merge(
            "/org/bluez/hci0/dev_11_22_33_44_55_66",
            IFACE_DEVICE,
            &props(vec![
                ("Address", s("11:22:33:44:55:66")),
                ("Alias", s("Télé du voisin")),
                ("Paired", OwnedValue::from(false)),
            ]),
        );
        m
    }

    #[test]
    fn instantane_bluez() {
        let mut m = model();
        let b = m.snapshot();
        assert!(b.adapter && b.powered);
        assert_eq!(b.devices.len(), 1);
        assert_eq!(b.devices[0].name, "Casque");
        assert!(!b.devices[0].connected);

        // Connexion, puis batterie annoncée par l'appareil.
        m.merge(
            CASQUE,
            IFACE_DEVICE,
            &props(vec![("Connected", OwnedValue::from(true))]),
        );
        m.merge(
            CASQUE,
            IFACE_BATTERY,
            &props(vec![("Percentage", OwnedValue::from(85u8))]),
        );
        let d = &m.snapshot().devices[0];
        assert!(d.connected && d.paired);
        assert_eq!(d.battery, Some(85));
        assert_eq!(d.address, "AA:BB:CC:DD:EE:FF");

        // Batterie retirée, puis appareil oublié.
        m.remove(CASQUE, &[IFACE_BATTERY.to_string()]);
        assert_eq!(m.snapshot().devices[0].battery, None);
        m.remove(CASQUE, &[IFACE_DEVICE.to_string()]);
        assert!(m.snapshot().devices.is_empty());

        // Adaptateur éteint, puis retiré (clé USB débranchée).
        m.merge(
            "/org/bluez/hci0",
            IFACE_ADAPTER,
            &props(vec![("Powered", OwnedValue::from(false))]),
        );
        assert!(m.snapshot().adapter && !m.snapshot().powered);
        m.remove("/org/bluez/hci0", &[IFACE_ADAPTER.to_string()]);
        assert!(!m.snapshot().adapter);
    }

    #[test]
    fn nom_de_repli() {
        let mut m = Model::default();
        m.merge(
            "/org/bluez/hci0/dev_X",
            IFACE_DEVICE,
            &props(vec![
                ("Address", s("00:11:22:33:44:55")),
                ("Name", s("Souris")),
                ("Connected", OwnedValue::from(true)),
            ]),
        );
        assert_eq!(m.snapshot().devices[0].name, "Souris");
        assert!(!m.snapshot().adapter);
    }

    #[test]
    fn table_lua() {
        let lua = Lua::new();
        let t = to_lua(&lua, &model().snapshot()).unwrap();
        assert!(t.get::<bool>("powered").unwrap());
        let list: Table = t.get("devices").unwrap();
        let d: Table = list.get(1).unwrap();
        assert_eq!(d.get::<String>("name").unwrap(), "Casque");
        assert!(d.get::<Option<u8>>("battery").unwrap().is_none());
    }
}
