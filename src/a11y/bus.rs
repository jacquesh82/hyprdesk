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
/// Tout nœud hors racine sert `Action`, sans action `NActions` à 0 : un nœud
/// gardé dont l'action vient ou part (« Fenêtre », une entrée grisée) ne
/// change pas d'interfaces, que libatspi garde en cache.
/// `Component` aussi, partout : la place d'un nœud est celle de sa surface.
const NODE_INTERFACES: [&str; 3] = [
    "org.a11y.atspi.Accessible",
    "org.a11y.atspi.Action",
    "org.a11y.atspi.Component",
];
/// Nom de l'action par défaut, celui que GTK et Qt donnent au clic.
const CLICK: &str = "click";
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

    /// Place du nœud `i` : la sienne, sinon celle de son plus proche ancêtre
    /// qui en a une ; inconnue : zéro.
    fn extents(&self, mut i: usize) -> [i32; 4] {
        let tree = self.tree();
        while let Some(n) = tree.get(i) {
            if let Some(e) = n.extents {
                return e;
            }
            match n.parent {
                Some(p) => i = p,
                None => break,
            }
        }
        [0; 4]
    }

    /// La surface du nœud `i` : son ancêtre directement sous l'application.
    fn window(&self, mut i: usize) -> usize {
        let tree = self.tree();
        while let Some(p) = tree.get(i).and_then(|n| n.parent) {
            if p == 0 {
                break;
            }
            i = p;
        }
        i
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

/// L'action d'un nœud : `org.a11y.atspi.Action`. Servie et annoncée sur
/// chaque nœud hors racine ; sans action, `NActions` vaut 0.
struct Actions {
    idx: usize,
    shared: Arc<Shared>,
}

impl Actions {
    /// L'action numéro `index` existe : la seule, le clic.
    fn has(&self, index: i32) -> bool {
        index == 0 && self.shared.tree().get(self.idx).is_some_and(|n| n.action)
    }

    fn text(&self, index: i32, f: impl FnOnce() -> String) -> String {
        if self.has(index) { f() } else { String::new() }
    }
}

#[zbus::interface(name = "org.a11y.atspi.Action")]
impl Actions {
    #[zbus(property, name = "NActions")]
    fn n_actions(&self) -> i32 {
        i32::from(self.has(0))
    }

    fn get_name(&self, index: i32) -> String {
        self.text(index, || CLICK.to_string())
    }

    fn get_localized_name(&self, index: i32) -> String {
        self.text(index, || crate::i18n::tr("Cliquer").to_string())
    }

    fn get_description(&self, _index: i32) -> String {
        String::new()
    }

    fn get_key_binding(&self, _index: i32) -> String {
        String::new()
    }

    /// `a(sss)` : nom localisé, description, raccourci.
    fn get_actions(&self) -> Vec<(String, String, String)> {
        if self.has(0) {
            vec![(
                crate::i18n::tr("Cliquer").to_string(),
                String::new(),
                String::new(),
            )]
        } else {
            Vec::new()
        }
    }

    /// Renvoie la clé du nœud à l'interface, qui l'exécute comme un clic ;
    /// répond sans attendre. Faux sans action ou sans interface qui écoute.
    fn do_action(&self, index: i32) -> bool {
        if !self.has(index) {
            return false;
        }
        let key = self.shared.tree().get(self.idx).map(|n| n.key.clone());
        key.is_some_and(super::request)
    }
}

/// La place d'un nœud : `org.a11y.atspi.Component`. Coordonnées de l'écran
/// (0), de la surface (1) ou du parent (2), en pixels logiques.
struct Component {
    idx: usize,
    shared: Arc<Shared>,
}

/// Couches AT-SPI (`AtspiComponentLayer`) : une fenêtre, un contrôle.
const LAYER_WIDGET: u32 = 3;
const LAYER_WINDOW: u32 = 7;

impl Component {
    fn extents_in(&self, coord_type: u32) -> [i32; 4] {
        let [x, y, w, h] = self.shared.extents(self.idx);
        // Le verrou de l'arbre est rendu avant d'être repris par `extents`.
        let parent = self.shared.tree().get(self.idx).and_then(|n| n.parent);
        let origin = match coord_type {
            1 => Some(self.shared.window(self.idx)),
            2 => parent,
            _ => None,
        }
        .filter(|&o| o != 0)
        .map_or([0; 4], |o| self.shared.extents(o));
        [x - origin[0], y - origin[1], w, h]
    }
}

#[zbus::interface(name = "org.a11y.atspi.Component")]
impl Component {
    /// Un seul argument `(iiii)`, comme libatspi le lit : un tuple nu serait
    /// rendu en quatre arguments.
    fn get_extents(&self, coord_type: u32) -> ((i32, i32, i32, i32),) {
        let [x, y, w, h] = self.extents_in(coord_type);
        ((x, y, w, h),)
    }

    fn get_position(&self, coord_type: u32) -> (i32, i32) {
        let [x, y, ..] = self.extents_in(coord_type);
        (x, y)
    }

    fn get_size(&self) -> (i32, i32) {
        let [.., w, h] = self.shared.extents(self.idx);
        (w, h)
    }

    fn contains(&self, x: i32, y: i32, coord_type: u32) -> bool {
        let [ex, ey, w, h] = self.extents_in(coord_type);
        x >= ex && y >= ey && x < ex + w && y < ey + h
    }

    /// L'enfant au point : les enfants n'ont que la place de leur surface,
    /// aucun ne se distingue ; nul, pour qu'un client qui descend s'arrête.
    fn get_accessible_at_point(&self, _x: i32, _y: i32, _coord_type: u32) -> Ref {
        Ref::null()
    }

    fn get_layer(&self) -> u32 {
        if self.shared.tree().get(self.idx).and_then(|n| n.parent) == Some(0) {
            LAYER_WINDOW
        } else {
            LAYER_WIDGET
        }
    }

    #[zbus(name = "GetMDIZOrder")]
    fn get_mdi_z_order(&self) -> i16 {
        0
    }

    /// Le focus suit le clavier de hyprdesk, pas le lecteur d'écran.
    fn grab_focus(&self) -> bool {
        false
    }

    fn get_alpha(&self) -> f64 {
        1.0
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
            match serve_node(server, id, &self.shared).await {
                Ok(()) => {
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
            let _ = server.remove::<Component, _>(Tree::path(id)).await;
            let _ = server.remove::<Actions, _>(Tree::path(id)).await;
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
            let _ = server.remove::<Component, _>(Tree::path(id)).await;
            let _ = server.remove::<Actions, _>(Tree::path(id)).await;
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
        serve_node(server, id, &shared).await?;
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

/// Sert le nœud `id` : `Accessible`, et hors de la racine `Action` et `Component`.
async fn serve_node(
    server: &zbus::ObjectServer,
    id: usize,
    shared: &Arc<Shared>,
) -> zbus::Result<()> {
    let path = Tree::path(id);
    server
        .at(
            path.as_str(),
            Accessible {
                idx: id,
                shared: shared.clone(),
            },
        )
        .await?;
    if id == 0 {
        return Ok(());
    }
    let actions = Actions {
        idx: id,
        shared: shared.clone(),
    };
    let component = Component {
        idx: id,
        shared: shared.clone(),
    };
    let r = match server.at(path.as_str(), actions).await {
        Ok(_) => server.at(path.as_str(), component).await.map(|_| ()),
        Err(e) => Err(e),
    };
    if let Err(e) = r {
        // Pas de nœud à moitié servi : il ne serait jamais retiré.
        let _ = server.remove::<Actions, _>(path.as_str()).await;
        let _ = server.remove::<Accessible, _>(path.as_str()).await;
        return Err(e);
    }
    Ok(())
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
                    .at(10.0, 800.0, 1420.0, 80.0)
                    .child(
                        Node::new(Role::PushButton, "Firefox")
                            .state(State::Focusable)
                            .action(),
                    ),
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

        // Action : annoncée seulement là où il y en a une ; `DoAction` renvoie
        // la clé du nœud à l'interface.
        const ACTION: &str = "org.a11y.atspi.Action";
        let ifaces: Vec<String> = call(&at, &me, &firefox, "GetInterfaces").await;
        assert_eq!(ifaces, NODE_INTERFACES);
        let n: i32 = get(&at, &me, &firefox, ACTION, "NActions")
            .await
            .try_into()
            .unwrap();
        assert_eq!(n, 1);
        let n: i32 = get(&at, &me, &dock, ACTION, "NActions")
            .await
            .try_into()
            .unwrap();
        assert_eq!(n, 0, "sans action");
        let act = |path: String, m: &'static str| {
            let at = at.clone();
            let me = me.clone();
            async move {
                at.call_method(Some(me.as_str()), path.as_str(), Some(ACTION), m, &(0i32,))
                    .await
                    .unwrap()
            }
        };
        let name: String = act(firefox.clone(), "GetName")
            .await
            .body()
            .deserialize()
            .unwrap();
        assert_eq!(name, CLICK);
        let loc: String = act(firefox.clone(), "GetLocalizedName")
            .await
            .body()
            .deserialize()
            .unwrap();
        assert!(!loc.is_empty());
        let all: Vec<(String, String, String)> = at
            .call_method(
                Some(me.as_str()),
                firefox.as_str(),
                Some(ACTION),
                "GetActions",
                &(),
            )
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!(all.len(), 1);
        let none: Vec<(String, String, String)> = at
            .call_method(
                Some(me.as_str()),
                dock.as_str(),
                Some(ACTION),
                "GetActions",
                &(),
            )
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert!(none.is_empty());
        let second: bool = at
            .call_method(
                Some(me.as_str()),
                firefox.as_str(),
                Some(ACTION),
                "DoAction",
                &(1i32,),
            )
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert!(!second, "une seule action");

        // Component : la place de la surface, l'enfant prend celle du dock.
        const COMP: &str = "org.a11y.atspi.Component";
        let comp = |path: String, m: &'static str, body: (u32,)| {
            let at = at.clone();
            let me = me.clone();
            async move {
                at.call_method(Some(me.as_str()), path.as_str(), Some(COMP), m, &body)
                    .await
                    .unwrap()
            }
        };
        let ext = |path: String, coord: u32| {
            let r = comp(path, "GetExtents", (coord,));
            async move {
                let m = r.await;
                assert_eq!(m.body().signature().to_string(), "(iiii)");
                let (r,): ((i32, i32, i32, i32),) = m.body().deserialize().unwrap();
                r
            }
        };
        assert_eq!(ext(dock.clone(), 0).await, (10, 800, 1420, 80));
        assert_eq!(ext(firefox.clone(), 0).await, (10, 800, 1420, 80));
        assert_eq!(
            ext(firefox.clone(), 1).await,
            (0, 0, 1420, 80),
            "dans sa surface"
        );
        let layer = |path: String| {
            let at = at.clone();
            let me = me.clone();
            async move {
                let r: u32 = at
                    .call_method(
                        Some(me.as_str()),
                        path.as_str(),
                        Some(COMP),
                        "GetLayer",
                        &(),
                    )
                    .await
                    .unwrap()
                    .body()
                    .deserialize()
                    .unwrap();
                r
            }
        };
        assert_eq!(layer(dock.clone()).await, LAYER_WINDOW);
        assert_eq!(layer(firefox.clone()).await, LAYER_WIDGET);
        let inside: bool = at
            .call_method(
                Some(me.as_str()),
                dock.as_str(),
                Some(COMP),
                "Contains",
                &(20i32, 810i32, 0u32),
            )
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert!(inside);
        let outside: bool = at
            .call_method(
                Some(me.as_str()),
                dock.as_str(),
                Some(COMP),
                "Contains",
                &(5i32, 810i32, 0u32),
            )
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert!(!outside);
        let (x, y): (i32, i32) = comp(firefox.clone(), "GetPosition", (0,))
            .await
            .body()
            .deserialize()
            .unwrap();
        assert_eq!((x, y), (10, 800));
        let mut requests = Box::pin(crate::a11y::requests());
        let done: bool = act(dock.clone(), "DoAction")
            .await
            .body()
            .deserialize()
            .unwrap();
        assert!(!done, "sans action, rien");
        let done: bool = act(firefox.clone(), "DoAction")
            .await
            .body()
            .deserialize()
            .unwrap();
        assert!(done);
        let key = tokio::time::timeout(std::time::Duration::from_secs(5), requests.next())
            .await
            .expect("demande d'action")
            .unwrap();
        assert_eq!(key, "/#0/#0", "la clé du nœud, pour l'interface");

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
