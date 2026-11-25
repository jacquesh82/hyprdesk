//! Objets AT-SPI servis sur une connexion donnée : la racine
//! (`org.a11y.atspi.Application` et `org.a11y.atspi.Accessible`, au chemin
//! fixe du protocole) et un objet `Accessible` par nœud de l'arbre. Une fois
//! la racine servie, [`start`] l'inscrit auprès du registre
//! (`org.a11y.atspi.Socket.Embed`) ; la référence rendue devient le parent de
//! la racine.
//!
//! La connexion est celle du bus d'accessibilité ; son choix (et le suivi de
//! `org.a11y.Status`) appartient à l'appelant.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use zbus::zvariant::{OwnedObjectPath, Type};

use super::model::{Flat, Ids, NULL_PATH, Node, ROOT_PATH, State, StateSet, Tree, diff, locale};

/// Nom du registre AT-SPI sur le bus d'accessibilité.
pub const REGISTRY: &str = "org.a11y.atspi.Registry";
/// Interface d'inscription du registre.
pub const SOCKET: &str = "org.a11y.atspi.Socket";
/// Interfaces servies par la racine, puis par les autres nœuds.
const ROOT_INTERFACES: [&str; 2] = ["org.a11y.atspi.Accessible", "org.a11y.atspi.Application"];
const NODE_INTERFACES: [&str; 1] = ["org.a11y.atspi.Accessible"];
/// Événements d'objet (structure, propriétés, états).
const EVENT_OBJECT: &str = "org.a11y.atspi.Event.Object";
/// Événements de fenêtre : sous Wayland, Orca n'a que `Activate` pour savoir
/// quelle surface tient le clavier.
const EVENT_WINDOW: &str = "org.a11y.atspi.Event.Window";

/// Référence à un objet accessible : nom sur le bus et chemin (`(so)`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Ref(pub String, pub OwnedObjectPath);

impl Ref {
    /// `path` vient de [`Tree::path`] ou des constantes du protocole : il est
    /// toujours valide ; sinon, la référence nulle.
    pub fn new(name: &str, path: &str) -> Ref {
        let p = OwnedObjectPath::try_from(path.to_string())
            .unwrap_or_else(|_| OwnedObjectPath::try_from(NULL_PATH).expect("chemin nul valide"));
        Ref(name.to_string(), p)
    }

    pub fn null() -> Ref {
        Ref::new("", NULL_PATH)
    }
}

impl From<Ref> for zbus::zvariant::Value<'static> {
    fn from(r: Ref) -> Self {
        zbus::zvariant::Value::from((r.0, r.1))
    }
}

/// État commun à tous les objets servis.
struct Shared {
    tree: Mutex<Tree>,
    /// Référence rendue par le registre : parent de la racine.
    parent: Mutex<Ref>,
    /// Nom unique de la connexion qui sert les objets.
    name: String,
}

impl Shared {
    fn tree(&self) -> MutexGuard<'_, Tree> {
        self.tree.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn at(&self, i: usize) -> Ref {
        Ref::new(&self.name, &Tree::path(i))
    }
}

/// Un nœud servi : `org.a11y.atspi.Accessible`.
struct Accessible {
    idx: usize,
    shared: Arc<Shared>,
}

impl Accessible {
    fn node<R: Default>(&self, f: impl FnOnce(&Flat) -> R) -> R {
        self.shared.tree().get(self.idx).map(f).unwrap_or_default()
    }
}

#[zbus::interface(name = "org.a11y.atspi.Accessible")]
impl Accessible {
    #[zbus(property)]
    fn name(&self) -> String {
        self.node(|n| n.name.clone())
    }

    #[zbus(property)]
    fn description(&self) -> String {
        self.node(|n| n.description.clone())
    }

    #[zbus(property)]
    fn parent(&self) -> Ref {
        match self.node(|n| n.parent) {
            Some(p) => self.shared.at(p),
            None if self.idx == 0 => self
                .shared
                .parent
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            None => Ref::null(),
        }
    }

    #[zbus(property)]
    fn child_count(&self) -> i32 {
        self.node(|n| n.children.len() as i32)
    }

    #[zbus(property)]
    fn locale(&self) -> String {
        locale().to_string()
    }

    #[zbus(property)]
    fn accessible_id(&self) -> String {
        Tree::path(self.idx)
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string()
    }

    fn get_child_at_index(&self, index: i32) -> Ref {
        let child = self.node(|n| {
            usize::try_from(index)
                .ok()
                .and_then(|k| n.children.get(k).copied())
        });
        child.map(|c| self.shared.at(c)).unwrap_or_else(Ref::null)
    }

    fn get_children(&self) -> Vec<Ref> {
        let ids = self.node(|n| n.children.clone());
        ids.into_iter().map(|c| self.shared.at(c)).collect()
    }

    fn get_index_in_parent(&self) -> i32 {
        self.shared
            .tree()
            .get(self.idx)
            .map_or(-1, |n| n.index_in_parent)
    }

    fn get_relation_set(&self) -> Vec<(u32, Vec<Ref>)> {
        Vec::new()
    }

    fn get_role(&self) -> u32 {
        self.node(|n| n.role as u32)
    }

    fn get_role_name(&self) -> String {
        self.node(|n| n.role.name().to_string())
    }

    /// Orca traduit lui-même les rôles qu'il annonce ; le nom non traduit
    /// suffit ici.
    fn get_localized_role_name(&self) -> String {
        self.get_role_name()
    }

    fn get_state(&self) -> Vec<u32> {
        self.node(|n| n.states.words().to_vec())
    }

    fn get_attributes(&self) -> HashMap<String, String> {
        HashMap::from([("toolkit".to_string(), "hyprdesk".to_string())])
    }

    fn get_application(&self) -> Ref {
        self.shared.at(0)
    }

    /// Liste exacte : libatspi s'y fie pour savoir quoi interroger.
    fn get_interfaces(&self) -> Vec<String> {
        let list: &[&str] = if self.idx == 0 {
            &ROOT_INTERFACES
        } else {
            &NODE_INTERFACES
        };
        list.iter().map(|s| s.to_string()).collect()
    }
}

/// La racine : `org.a11y.atspi.Application`.
struct Application {
    /// Posé par le registre (`Properties.Set`).
    id: i32,
}

#[zbus::interface(name = "org.a11y.atspi.Application")]
impl Application {
    #[zbus(property)]
    fn toolkit_name(&self) -> String {
        "hyprdesk".to_string()
    }

    #[zbus(property)]
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[zbus(property)]
    fn atspi_version(&self) -> String {
        "2.1".to_string()
    }

    #[zbus(property)]
    fn id(&self) -> i32 {
        self.id
    }

    #[zbus(property)]
    fn set_id(&mut self, id: i32) {
        self.id = id;
    }

    fn get_locale(&self, _lctype: u32) -> String {
        locale().to_string()
    }
}

/// Pont en place : la connexion qui sert les objets et le parent rendu par
/// le registre.
pub struct Handle {
    pub conn: zbus::Connection,
    /// Référence rendue par le registre ; nulle tant que le pont n'est pas
    /// inscrit.
    pub parent: Ref,
    shared: Arc<Shared>,
    /// Numéros des objets, stables par clé.
    ids: Ids,
    /// Objets `Accessible` servis.
    served: std::collections::BTreeSet<usize>,
}

impl Handle {
    /// La racine, telle que le registre l'inscrit.
    fn plug(&self) -> Ref {
        Ref::new(&self.shared.name, ROOT_PATH)
    }

    fn set_parent(&mut self, parent: Ref) {
        *self.shared.parent.lock().unwrap_or_else(|e| e.into_inner()) = parent.clone();
        self.parent = parent;
    }

    /// Inscrit la racine auprès du registre (`Socket.Embed`) ; la référence
    /// rendue devient son parent. À refaire si le registre renaît.
    pub async fn embed(&mut self) -> zbus::Result<()> {
        let reply = self
            .conn
            .call_method(
                Some(REGISTRY),
                ROOT_PATH,
                Some(SOCKET),
                "Embed",
                &(self.plug(),),
            )
            .await?;
        let parent: Ref = reply.body().deserialize()?;
        self.set_parent(parent);
        Ok(())
    }

    /// Le registre est parti : l'inscription est perdue avec lui.
    pub fn lost(&mut self) {
        self.set_parent(Ref::null());
    }

    /// Désinscrit la racine (`Socket.Unembed`).
    pub async fn unembed(&mut self) -> zbus::Result<()> {
        self.conn
            .call_method(
                Some(REGISTRY),
                ROOT_PATH,
                Some(SOCKET),
                "Unembed",
                &(self.plug(),),
            )
            .await?;
        self.lost();
        Ok(())
    }

    /// Remplace l'arbre servi par différence : les nœuds gardés (même clé)
    /// gardent leur objet ; les nouveaux sont servis, puis annoncés
    /// (`ChildrenChanged:add`) ; les disparus annoncés (`:remove`), puis
    /// retirés ; un nom changé émet `PropertyChange:accessible-name`.
    pub async fn replace(&mut self, root: &Node) -> zbus::Result<()> {
        let new = Tree::flatten_with(root, &mut self.ids);
        // Objets à servir ou à retirer d'après ce qui est servi, pas d'après
        // l'arbre précédent : un échec passé se rattrape au remplacement
        // suivant.
        let added: Vec<usize> = new.ids().filter(|id| !self.served.contains(id)).collect();
        let removed: Vec<usize> = self
            .served
            .iter()
            .copied()
            .filter(|&id| new.get(id).is_none())
            .collect();
        let (d, old_names) = {
            let mut cur = self.shared.tree();
            let d = diff(&cur, &new);
            // Noms d'avant : une surface qui se ferme est annoncée sous le sien.
            let names: HashMap<usize, String> = d
                .states
                .iter()
                .filter_map(|&(id, ..)| cur.get(id).map(|n| (id, n.name.clone())))
                .collect();
            *cur = new;
            (d, names)
        };
        let mut first: zbus::Result<()> = Ok(());
        let mut keep = |r: zbus::Result<()>| {
            if let Err(e) = r
                && first.is_ok()
            {
                first = Err(e);
            }
        };
        let server = self.conn.object_server();
        for id in added {
            let node = Accessible {
                idx: id,
                shared: self.shared.clone(),
            };
            match server.at(Tree::path(id), node).await {
                Ok(_) => {
                    self.served.insert(id);
                }
                Err(e) => keep(Err(e)),
            }
        }
        // Focus et surface active. Pertes d'abord, avant même les retraits
        // annoncés (l'objet est encore là) : le focus, puis la fenêtre ;
        // prises après les ajouts : la fenêtre, puis le focus.
        let mut flips: Vec<(usize, State, bool)> = Vec::new();
        let born = d.added.iter().filter_map(|&id| {
            self.shared
                .tree()
                .get(id)
                .map(|n| (id, StateSet::default(), n.states))
        });
        let changes: Vec<_> = d.states.iter().copied().chain(born).collect();
        for (id, before, after) in changes {
            for st in [State::Active, State::Focused] {
                if before.contains(st) != after.contains(st) {
                    flips.push((id, st, after.contains(st)));
                }
            }
        }
        flips.sort_by_key(|&(_, st, on)| (on, (st == State::Focused) == on));
        let (losses, gains): (Vec<_>, Vec<_>) = flips.into_iter().partition(|f| !f.2);
        for f in losses {
            keep(self.flip(f, &old_names).await);
        }
        for &(parent, add, index, child) in &d.children {
            let detail = if add { "add" } else { "remove" };
            let child = self.shared.at(child);
            keep(
                self.emit(parent, "ChildrenChanged", detail, index, child.into())
                    .await,
            );
        }
        for &id in &d.renamed {
            let name = self
                .shared
                .tree()
                .get(id)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            keep(
                self.emit(id, "PropertyChange", "accessible-name", 0, name.into())
                    .await,
            );
        }
        for f in gains {
            keep(self.flip(f, &old_names).await);
        }
        for id in removed {
            // Absent du serveur ou retiré : plus servi dans les deux cas.
            let _ = server.remove::<Accessible, _>(Tree::path(id)).await;
            self.served.remove(&id);
        }
        first
    }

    /// Un état qui bascule : `StateChanged`, et pour la surface active
    /// `Window:Activate` ou `Deactivate`, sous son nom (l'ancien si elle part).
    async fn flip(
        &self,
        (id, st, on): (usize, State, bool),
        old_names: &HashMap<usize, String>,
    ) -> zbus::Result<()> {
        if st == State::Active {
            let name = self
                .shared
                .tree()
                .get(id)
                .map(|n| n.name.clone())
                .or_else(|| old_names.get(&id).cloned())
                .unwrap_or_default();
            let member = if on { "Activate" } else { "Deactivate" };
            self.emit_on(EVENT_WINDOW, id, member, "", 0, name.into())
                .await?;
        }
        let detail = if st == State::Active {
            "active"
        } else {
            "focused"
        };
        self.emit(id, "StateChanged", detail, i32::from(on), 0i32.into())
            .await
    }

    /// Événement `org.a11y.atspi.Event.Object`, corps `(siiva{sv})`, émis
    /// depuis le nœud `id`.
    async fn emit(
        &self,
        id: usize,
        member: &str,
        detail: &str,
        detail1: i32,
        value: zbus::zvariant::Value<'_>,
    ) -> zbus::Result<()> {
        self.emit_on(EVENT_OBJECT, id, member, detail, detail1, value)
            .await
    }

    async fn emit_on(
        &self,
        interface: &str,
        id: usize,
        member: &str,
        detail: &str,
        detail1: i32,
        value: zbus::zvariant::Value<'_>,
    ) -> zbus::Result<()> {
        let props: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        self.conn
            .emit_signal(
                None::<zbus::names::BusName<'_>>,
                Tree::path(id).as_str(),
                interface,
                member,
                &(detail, detail1, 0i32, value, props),
            )
            .await
    }

    /// Retire les objets servis ; la connexion se ferme avec le pont.
    pub async fn stop(self) {
        let server = self.conn.object_server();
        let _ = server.remove::<Application, _>(ROOT_PATH).await;
        for &id in &self.served {
            let _ = server.remove::<Accessible, _>(Tree::path(id)).await;
        }
    }
}

/// Sert l'arbre `root` (dont la racine devient l'application) sur `conn`,
/// sans l'inscrire.
pub async fn serve(conn: zbus::Connection, root: &Node) -> zbus::Result<Handle> {
    let name = conn
        .unique_name()
        .map(|n| n.to_string())
        .unwrap_or_default();
    let mut ids = Ids::default();
    let tree = Tree::flatten_with(root, &mut ids);
    let all: Vec<usize> = tree.ids().collect();
    let shared = Arc::new(Shared {
        tree: Mutex::new(tree),
        parent: Mutex::new(Ref::null()),
        name,
    });
    let server = conn.object_server();
    server.at(ROOT_PATH, Application { id: 0 }).await?;
    let mut served = std::collections::BTreeSet::new();
    for id in all {
        let node = Accessible {
            idx: id,
            shared: shared.clone(),
        };
        server.at(Tree::path(id), node).await?;
        served.insert(id);
    }
    Ok(Handle {
        conn,
        parent: Ref::null(),
        shared,
        ids,
        served,
    })
}

/// Sert l'arbre, puis l'inscrit : la racine est servie avant l'inscription,
/// le registre peut l'interroger aussitôt.
pub async fn start(conn: zbus::Connection, root: &Node) -> zbus::Result<Handle> {
    let mut h = serve(conn, root).await?;
    h.embed().await?;
    Ok(h)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::a11y::model::{Role, State};
    use iced::futures::StreamExt;
    use zbus::zvariant::{OwnedValue, Value};

    /// Faux registre : retient la référence inscrite et rend la sienne.
    pub(crate) struct FakeRegistry {
        pub(crate) seen: Arc<Mutex<Option<Ref>>>,
        pub(crate) gone: Arc<Mutex<Option<Ref>>>,
        pub(crate) name: String,
    }

    #[zbus::interface(name = "org.a11y.atspi.Socket")]
    impl FakeRegistry {
        fn embed(&self, plug: Ref) -> Ref {
            *self.seen.lock().unwrap() = Some(plug);
            Ref::new(&self.name, ROOT_PATH)
        }

        fn unembed(&self, plug: Ref) {
            *self.gone.lock().unwrap() = Some(plug);
        }
    }

    const ACC: &str = "org.a11y.atspi.Accessible";
    const APP: &str = "org.a11y.atspi.Application";
    const PROPS: &str = "org.freedesktop.DBus.Properties";

    async fn get(c: &zbus::Connection, to: &str, path: &str, iface: &str, p: &str) -> OwnedValue {
        c.call_method(Some(to), path, Some(PROPS), "Get", &(iface, p))
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap()
    }

    async fn call<R>(c: &zbus::Connection, to: &str, path: &str, m: &str) -> R
    where
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        c.call_method(Some(to), path, Some(ACC), m, &())
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap()
    }

    #[tokio::test]
    #[ignore = "exécuter sous dbus-run-session, jamais sur le bus de la session utilisateur"]
    async fn pont_sur_le_bus() {
        // Le registre, sur sa propre connexion.
        let reg = zbus::Connection::session().await.unwrap();
        let reg_name = reg.unique_name().unwrap().to_string();
        let seen: Arc<Mutex<Option<Ref>>> = Default::default();
        let gone: Arc<Mutex<Option<Ref>>> = Default::default();
        reg.object_server()
            .at(
                ROOT_PATH,
                FakeRegistry {
                    seen: seen.clone(),
                    gone: gone.clone(),
                    name: reg_name.clone(),
                },
            )
            .await
            .unwrap();
        reg.request_name(REGISTRY).await.unwrap();

        // Le pont sert l'arbre et s'inscrit.
        let app = zbus::Connection::session().await.unwrap();
        let me = app.unique_name().unwrap().to_string();
        let root = Node::new(Role::Application, "hyprdesk")
            .child(
                Node::new(Role::Frame, "Dock")
                    .child(Node::new(Role::PushButton, "Firefox").state(State::Focusable)),
            )
            .child(Node::new(Role::Frame, "Barre des menus"));
        let mut h = start(app, &root).await.unwrap();
        assert_eq!(seen.lock().unwrap().clone(), Some(Ref::new(&me, ROOT_PATH)));
        assert_eq!(h.parent, Ref::new(&reg_name, ROOT_PATH));

        // Un lecteur d'écran l'interroge.
        let at = zbus::Connection::session().await.unwrap();
        at.call_method(
            Some(me.as_str()),
            ROOT_PATH,
            Some(PROPS),
            "Set",
            &(APP, "Id", Value::from(7i32)),
        )
        .await
        .unwrap();
        let id: i32 = get(&at, &me, ROOT_PATH, APP, "Id")
            .await
            .try_into()
            .unwrap();
        assert_eq!(id, 7, "Id posé par le registre");
        let tk: String = get(&at, &me, ROOT_PATH, APP, "ToolkitName")
            .await
            .try_into()
            .unwrap();
        assert_eq!(tk, "hyprdesk");
        let name: String = get(&at, &me, ROOT_PATH, ACC, "Name")
            .await
            .try_into()
            .unwrap();
        assert_eq!(name, "hyprdesk");
        let loc: String = get(&at, &me, ROOT_PATH, ACC, "Locale")
            .await
            .try_into()
            .unwrap();
        assert_eq!(loc, locale(), "langue de l'interface, pas celle de LANG");
        let parent: (String, OwnedObjectPath) = get(&at, &me, ROOT_PATH, ACC, "Parent")
            .await
            .try_into()
            .unwrap();
        assert_eq!(Ref(parent.0, parent.1), Ref::new(&reg_name, ROOT_PATH));
        let role: u32 = call(&at, &me, ROOT_PATH, "GetRole").await;
        assert_eq!(role, Role::Application as u32);
        let ifaces: Vec<String> = call(&at, &me, ROOT_PATH, "GetInterfaces").await;
        assert_eq!(ifaces, ROOT_INTERFACES);
        let children: Vec<Ref> = call(&at, &me, ROOT_PATH, "GetChildren").await;
        assert_eq!(
            children,
            [Ref::new(&me, &Tree::path(1)), Ref::new(&me, &Tree::path(3))]
        );

        // Un nœud : nom, rôle, parent, rang, états.
        let dock = Tree::path(1);
        let firefox = Tree::path(2);
        let n: String = get(&at, &me, &firefox, ACC, "Name")
            .await
            .try_into()
            .unwrap();
        assert_eq!(n, "Firefox");
        let role: u32 = call(&at, &me, &firefox, "GetRole").await;
        assert_eq!(role, Role::PushButton as u32);
        let p: (String, OwnedObjectPath) = get(&at, &me, &firefox, ACC, "Parent")
            .await
            .try_into()
            .unwrap();
        assert_eq!(Ref(p.0, p.1), Ref::new(&me, &dock));
        let idx: i32 = call(&at, &me, &Tree::path(3), "GetIndexInParent").await;
        assert_eq!(idx, 1);
        let state: Vec<u32> = call(&at, &me, &firefox, "GetState").await;
        assert_eq!(state.len(), 2);
        assert_ne!(state[0] & 1 << State::Focusable as u32, 0);
        let app_ref: Ref = call(&at, &me, &firefox, "GetApplication").await;
        assert_eq!(app_ref, Ref::new(&me, ROOT_PATH));

        // L'arbre change : un nœud de moins, l'objet de trop est retiré et
        // annoncé ; le nom changé aussi.
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(EVENT_OBJECT)
            .unwrap()
            .build();
        let mut events = zbus::MessageStream::for_match_rule(rule, &at, Some(16))
            .await
            .unwrap();
        let smaller = Node::new(Role::Application, "hyprdesk")
            .child(Node::new(Role::Frame, "Barre des menus"));
        h.replace(&smaller).await.unwrap();
        let n: String = get(&at, &me, &dock, ACC, "Name").await.try_into().unwrap();
        assert_eq!(n, "Barre des menus", "même rang, même objet");
        let r = at
            .call_method(
                Some(me.as_str()),
                firefox.as_str(),
                Some(ACC),
                "GetRole",
                &(),
            )
            .await;
        assert!(r.is_err(), "nœud retiré");
        let mut seen = Vec::new();
        while seen.len() < 3 {
            let m = tokio::time::timeout(std::time::Duration::from_secs(5), events.next())
                .await
                .expect("événement")
                .unwrap()
                .unwrap();
            let (detail, ..): (String, i32, i32, OwnedValue, HashMap<String, OwnedValue>) =
                m.body().deserialize().unwrap();
            let member = m.header().member().unwrap().to_string();
            seen.push(format!("{member}:{detail}"));
        }
        seen.sort();
        assert_eq!(
            seen,
            [
                "ChildrenChanged:remove",
                "ChildrenChanged:remove",
                "PropertyChange:accessible-name"
            ]
        );
        h.replace(&root).await.unwrap();
        let kids: Vec<Ref> = call(&at, &me, &dock, "GetChildren").await;
        let n: String = get(&at, &me, kids[0].1.as_str(), ACC, "Name")
            .await
            .try_into()
            .unwrap();
        assert_eq!(n, "Firefox", "nœud rajouté");
        let firefox = kids[0].1.to_string();

        // Le focus va à Firefox : `StateChanged:focused`, depuis son objet.
        drop(events);
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(EVENT_OBJECT)
            .unwrap()
            .member("StateChanged")
            .unwrap()
            .build();
        let mut events = zbus::MessageStream::for_match_rule(rule, &at, Some(16))
            .await
            .unwrap();
        let focused = Node::new(Role::Application, "hyprdesk")
            .child(
                Node::new(Role::Frame, "Dock").child(
                    Node::new(Role::PushButton, "Firefox")
                        .state(State::Focusable)
                        .state(State::Focused),
                ),
            )
            .child(Node::new(Role::Frame, "Barre des menus"));
        h.replace(&focused).await.unwrap();
        let m = tokio::time::timeout(std::time::Duration::from_secs(5), events.next())
            .await
            .expect("StateChanged")
            .unwrap()
            .unwrap();
        let (detail, on, ..): (String, i32, i32, OwnedValue, HashMap<String, OwnedValue>) =
            m.body().deserialize().unwrap();
        assert_eq!((detail.as_str(), on), ("focused", 1));
        assert_eq!(m.header().path().unwrap().as_str(), firefox);

        // Le dock prend le clavier, puis le rend : `Window:Activate`, puis
        // `Deactivate`, depuis sa Frame.
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(EVENT_WINDOW)
            .unwrap()
            .build();
        let mut windows = zbus::MessageStream::for_match_rule(rule, &at, Some(16))
            .await
            .unwrap();
        let active = |on: bool| {
            let frame = Node::new(Role::Frame, "Dock")
                .child(Node::new(Role::PushButton, "Firefox").state(State::Focusable));
            Node::new(Role::Application, "hyprdesk")
                .child(if on {
                    frame.state(State::Active)
                } else {
                    frame
                })
                .child(Node::new(Role::Frame, "Barre des menus"))
        };
        for (on, member) in [(true, "Activate"), (false, "Deactivate")] {
            h.replace(&active(on)).await.unwrap();
            let m = tokio::time::timeout(std::time::Duration::from_secs(5), windows.next())
                .await
                .expect("Window")
                .unwrap()
                .unwrap();
            assert_eq!(m.header().member().unwrap().as_str(), member);
            assert_eq!(m.header().path().unwrap().as_str(), dock);
        }

        // Lecteur coupé : désinscription, puis plus aucun objet.
        h.unembed().await.unwrap();
        assert_eq!(gone.lock().unwrap().clone(), Some(Ref::new(&me, ROOT_PATH)));
        assert_eq!(h.parent, Ref::null());
        h.stop().await;
        let r = at
            .call_method(
                Some(me.as_str()),
                firefox.as_str(),
                Some(ACC),
                "GetRole",
                &(),
            )
            .await;
        assert!(r.is_err(), "objet retiré");
    }
}
