//! Profil d'énergie (power-profiles-daemon, bus système) : profil actif et
//! profils proposés, suivis par `PropertiesChanged`. Nom récent
//! `org.freedesktop.UPower.PowerProfiles`, repli `net.hadess.PowerProfiles`.

use iced::futures::{FutureExt, StreamExt};
use mlua::{Lua, Table};

use super::{Props, Slot, get_all, prop_dict_field, prop_str};

const NAMES: &[&str] = &[
    "org.freedesktop.UPower.PowerProfiles",
    "net.hadess.PowerProfiles",
];

pub(super) static SLOT: Slot<Profiles> = Slot::new();

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Profiles {
    pub active: String,
    /// Profils proposés par la machine (`performance` n'existe pas partout).
    pub available: Vec<String>,
}

/// Propriétés (toutes ou changées) du service.
pub fn apply(p: &mut Profiles, props: &Props) {
    if let Some(a) = prop_str(props, "ActiveProfile") {
        p.active = a;
    }
    if let Some(list) = prop_dict_field(props, "Profiles", "Profile") {
        p.available = list;
    }
}

fn object_path(name: &str) -> &'static str {
    if name == NAMES[0] {
        "/org/freedesktop/UPower/PowerProfiles"
    } else {
        "/net/hadess/PowerProfiles"
    }
}

async fn session(conn: zbus::Connection, name: &'static str) -> zbus::Result<()> {
    let path = object_path(name);
    let mut stream = super::properties_changed(&conn, name, path).await?;
    let mut p = Profiles::default();
    apply(&mut p, &get_all(&conn, name, path, name).await?);
    SLOT.set(Some(p.clone()));
    while let Some(msg) = stream.next().await {
        let msg = msg?;
        if let Ok((iface, changed, _)) = msg.body().deserialize::<super::Changed>()
            && iface == name
        {
            apply(&mut p, &changed);
            SLOT.set(Some(p.clone()));
        }
    }
    Ok(())
}

pub(super) async fn run() {
    super::follow(&SLOT, NAMES, |conn, name| session(conn, name).boxed()).await;
}

/// Table de `hd.power_profile()`.
pub fn to_lua(lua: &Lua, p: &Profiles) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("active", p.active.as_str())?;
    let list = lua.create_table()?;
    for (i, name) in p.available.iter().enumerate() {
        list.set(i + 1, name.as_str())?;
    }
    t.set("profiles", list)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{props, s};
    use super::*;
    use std::collections::HashMap;
    use zbus::zvariant::{OwnedValue, Value};

    fn profile(name: &str) -> HashMap<String, Value<'static>> {
        HashMap::from([
            ("Profile".to_string(), Value::from(name.to_string())),
            (
                "Driver".to_string(),
                Value::from("platform_profile".to_string()),
            ),
        ])
    }

    #[test]
    fn profils_decodes() {
        let list = Value::from(vec![profile("power-saver"), profile("balanced")]);
        let mut p = Profiles::default();
        apply(
            &mut p,
            &props(vec![
                ("ActiveProfile", s("balanced")),
                ("Profiles", OwnedValue::try_from(list).unwrap()),
            ]),
        );
        assert_eq!(p.active, "balanced");
        assert_eq!(p.available, vec!["power-saver", "balanced"]);
        // Changement du seul profil actif : la liste reste.
        apply(&mut p, &props(vec![("ActiveProfile", s("power-saver"))]));
        assert_eq!(p.active, "power-saver");
        assert_eq!(p.available.len(), 2);
        assert_eq!(object_path(NAMES[1]), "/net/hadess/PowerProfiles");
    }

    #[test]
    fn table_lua() {
        let lua = Lua::new();
        let t = to_lua(
            &lua,
            &Profiles {
                active: "balanced".into(),
                available: vec!["balanced".into()],
            },
        )
        .unwrap();
        assert_eq!(t.get::<String>("active").unwrap(), "balanced");
        let l: Table = t.get("profiles").unwrap();
        assert_eq!(l.get::<String>(1).unwrap(), "balanced");
    }
}
