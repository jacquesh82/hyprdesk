//! Arbre d'accessibilité pur : rôles et états AT-SPI, nœuds, arbre aplati
//! et chemins des objets. Aucune dépendance à D-Bus : testable seul.

use std::collections::HashMap;

/// Chemin de la racine (l'objet `Application`), fixé par le protocole.
pub const ROOT_PATH: &str = "/org/a11y/atspi/accessible/root";
/// Préfixe des chemins des autres nœuds.
pub const NODE_PREFIX: &str = "/org/a11y/atspi/accessible/";
/// Chemin de la référence nulle (aucun objet).
pub const NULL_PATH: &str = "/org/a11y/atspi/null";

/// Rôles AT-SPI (`AtspiRole`) employés par hyprdesk ; la valeur est celle
/// du protocole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Role {
    Invalid = 0,
    CheckBox = 7,
    Dialog = 16,
    Filler = 20,
    Frame = 23,
    Icon = 26,
    Image = 27,
    Label = 29,
    List = 31,
    ListItem = 32,
    Menu = 33,
    MenuBar = 34,
    MenuItem = 35,
    PageTab = 37,
    PageTabList = 38,
    Panel = 39,
    PasswordText = 40,
    PopupMenu = 41,
    PushButton = 43,
    Slider = 51,
    ToggleButton = 62,
    ToolBar = 63,
    Window = 69,
    Application = 75,
    Entry = 79,
    Heading = 83,
}

impl Role {
    /// Nom non traduit du rôle, tel que libatspi le donne (`GetRoleName`).
    pub fn name(self) -> &'static str {
        match self {
            Role::Invalid => "invalid",
            Role::CheckBox => "check box",
            Role::Dialog => "dialog",
            Role::Filler => "filler",
            Role::Frame => "frame",
            Role::Icon => "icon",
            Role::Image => "image",
            Role::Label => "label",
            Role::List => "list",
            Role::ListItem => "list item",
            Role::Menu => "menu",
            Role::MenuBar => "menu bar",
            Role::MenuItem => "menu item",
            Role::PageTab => "page tab",
            Role::PageTabList => "page tab list",
            Role::Panel => "panel",
            Role::PasswordText => "password text",
            Role::PopupMenu => "popup menu",
            Role::PushButton => "push button",
            Role::Slider => "slider",
            Role::ToggleButton => "toggle button",
            Role::ToolBar => "tool bar",
            Role::Window => "window",
            Role::Application => "application",
            Role::Entry => "entry",
            Role::Heading => "heading",
        }
    }
}

/// États AT-SPI (`AtspiStateType`) ; la valeur est le rang du bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum State {
    Active = 1,
    Checked = 4,
    Editable = 7,
    Enabled = 8,
    Expandable = 9,
    Expanded = 10,
    Focusable = 11,
    Focused = 12,
    Horizontal = 14,
    Modal = 16,
    Pressed = 20,
    Selectable = 22,
    Selected = 23,
    Sensitive = 24,
    Showing = 25,
    SingleLine = 26,
    Vertical = 29,
    Visible = 30,
    Checkable = 41,
    HasPopup = 42,
}

/// Ensemble d'états, sérialisé en deux mots de 32 bits (`GetState -> au`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StateSet(u64);

impl StateSet {
    /// États d'un élément affiché et utilisable.
    pub fn shown() -> StateSet {
        StateSet::default()
            .with(State::Enabled)
            .with(State::Sensitive)
            .with(State::Showing)
            .with(State::Visible)
    }

    pub fn with(self, s: State) -> StateSet {
        StateSet(self.0 | 1 << s as u32)
    }

    pub fn without(self, s: State) -> StateSet {
        StateSet(self.0 & !(1 << s as u32))
    }

    pub fn contains(self, s: State) -> bool {
        self.0 & 1 << s as u32 != 0
    }

    /// Les deux mots du protocole : bits 0 à 31, puis 32 à 63.
    pub fn words(self) -> [u32; 2] {
        [self.0 as u32, (self.0 >> 32) as u32]
    }
}

/// Un nœud de l'arbre, tel que les surfaces le décrivent.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Identité stable parmi ses frères (`dock-pin:firefox`, `volume`) : un
    /// nœud qui garde sa clé garde son objet D-Bus d'un arbre à l'autre. Vide :
    /// son rang.
    pub key: String,
    pub role: Role,
    pub name: String,
    pub description: String,
    pub states: StateSet,
    pub children: Vec<Node>,
    /// Le nœud a une action par défaut (« click », `org.a11y.atspi.Action`) :
    /// `DoAction` renvoie sa clé à l'interface, qui l'exécute comme un clic.
    pub action: bool,
    /// Place à l'écran, en pixels logiques (x, y, largeur, hauteur) : celle
    /// des surfaces ; un nœud sans place prend celle de son plus proche
    /// ancêtre qui en a une (`Component.GetExtents`).
    pub extents: Option<[i32; 4]>,
}

impl Node {
    pub fn new(role: Role, name: impl Into<String>) -> Node {
        Node {
            key: String::new(),
            role,
            name: name.into(),
            description: String::new(),
            states: StateSet::shown(),
            children: Vec::new(),
            action: false,
            extents: None,
        }
    }

    /// Place de la surface à l'écran (`Node::extents`).
    pub fn at(mut self, x: f32, y: f32, w: f32, h: f32) -> Node {
        self.extents = Some([x, y, w, h].map(|v| v.round() as i32));
        self
    }

    /// Le nœud s'active comme un clic (`DoAction`).
    pub fn action(mut self) -> Node {
        self.action = true;
        self
    }

    pub fn describe(mut self, d: impl Into<String>) -> Node {
        self.description = d.into();
        self
    }

    pub fn key(mut self, k: impl Into<String>) -> Node {
        self.key = k.into();
        self
    }

    pub fn state(mut self, s: State) -> Node {
        self.states = self.states.with(s);
        self
    }

    pub fn child(mut self, c: Node) -> Node {
        self.children.push(c);
        self
    }

    pub fn children(mut self, c: impl IntoIterator<Item = Node>) -> Node {
        self.children.extend(c);
        self
    }

    /// L'arbre en texte, un nœud par ligne, indenté de deux espaces par
    /// niveau : `rôle « nom »` (`hyprdesk a11y dump`).
    pub fn dump(&self) -> String {
        let mut out = String::new();
        dump_into(&mut out, self, 0);
        out
    }
}

fn dump_into(out: &mut String, n: &Node, depth: usize) {
    out.push_str(&"  ".repeat(depth));
    out.push_str(n.role.name());
    if !n.name.is_empty() {
        out.push_str(" « ");
        out.push_str(&n.name);
        out.push_str(" »");
    }
    out.push('\n');
    for c in &n.children {
        dump_into(out, c, depth + 1);
    }
}

/// Un nœud de l'arbre aplati. `id` est le numéro de son objet D-Bus ;
/// parent et enfants sont donnés par leurs `id`.
#[derive(Clone, Debug, PartialEq)]
pub struct Flat {
    pub id: usize,
    /// Chemin des clés depuis la racine : l'identité du nœud.
    pub key: String,
    pub role: Role,
    pub name: String,
    pub description: String,
    pub states: StateSet,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// Rang dans le parent (`GetIndexInParent`), -1 pour la racine.
    pub index_in_parent: i32,
    /// Une action par défaut (`Node::action`).
    pub action: bool,
    /// Place à l'écran (`Node::extents`).
    pub extents: Option<[i32; 4]>,
}

/// Numéros des objets, stables d'un arbre à l'autre pour une même clé ; la
/// racine est toujours 0.
#[derive(Clone, Debug)]
pub struct Ids {
    map: HashMap<String, usize>,
    next: usize,
}

impl Default for Ids {
    fn default() -> Self {
        Ids {
            map: HashMap::new(),
            next: 1,
        }
    }
}

impl Ids {
    fn of(&mut self, key: &str) -> usize {
        if key.is_empty() {
            return 0;
        }
        if let Some(id) = self.map.get(key) {
            return *id;
        }
        let id = self.next;
        self.next += 1;
        self.map.insert(key.to_string(), id);
        id
    }
}

/// L'arbre aplati, en ordre préfixe : la racine est au rang 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub nodes: Vec<Flat>,
    pos: HashMap<usize, usize>,
}

impl Tree {
    /// Aplatit `root`, qui devient le nœud 0 (l'application), avec des
    /// numéros neufs.
    pub fn flatten(root: &Node) -> Tree {
        Tree::flatten_with(root, &mut Ids::default())
    }

    /// Aplatit `root` en reprenant les numéros de `ids` ; ceux des clés
    /// disparues sont oubliés.
    pub fn flatten_with(root: &Node, ids: &mut Ids) -> Tree {
        let mut nodes = Vec::new();
        push(&mut nodes, ids, root, String::new(), None, -1);
        let pos: HashMap<usize, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        ids.map.retain(|_, id| pos.contains_key(id));
        Tree { nodes, pos }
    }

    /// Chemin D-Bus du nœud numéro `id`.
    pub fn path(id: usize) -> String {
        if id == 0 {
            ROOT_PATH.to_string()
        } else {
            format!("{NODE_PREFIX}{id}")
        }
    }

    /// Le nœud numéro `id`.
    pub fn get(&self, id: usize) -> Option<&Flat> {
        self.pos.get(&id).and_then(|&i| self.nodes.get(i))
    }

    pub fn ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.nodes.iter().map(|n| n.id)
    }
}

fn push(
    nodes: &mut Vec<Flat>,
    ids: &mut Ids,
    n: &Node,
    key: String,
    parent: Option<usize>,
    index: i32,
) -> usize {
    let id = ids.of(&key);
    let me = nodes.len();
    nodes.push(Flat {
        id,
        key: key.clone(),
        role: n.role,
        name: n.name.clone(),
        description: n.description.clone(),
        states: n.states,
        parent,
        children: Vec::new(),
        index_in_parent: index,
        action: n.action,
        extents: n.extents,
    });
    let mut seen = std::collections::HashSet::new();
    for (k, c) in n.children.iter().enumerate() {
        let mut own = if c.key.is_empty() {
            format!("#{k}")
        } else {
            c.key.clone()
        };
        // Deux frères de même clé : le second prend aussi son rang.
        if !seen.insert(own.clone()) {
            own = format!("{own}#{k}");
        }
        let child = push(nodes, ids, c, format!("{key}/{own}"), Some(id), k as i32);
        nodes[me].children.push(child);
    }
    id
}

/// Ce qui change d'un arbre à l'autre, pour le pont.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diff {
    /// Nœuds nouveaux (objets à servir), en ordre préfixe.
    pub added: Vec<usize>,
    /// Nœuds disparus (objets à retirer).
    pub removed: Vec<usize>,
    /// Nœuds gardés dont le nom change (`PropertyChange:accessible-name`).
    pub renamed: Vec<usize>,
    /// `ChildrenChanged` : (parent, ajout ?, rang, enfant), pour le plus haut
    /// nœud ajouté ou retiré de chaque branche.
    pub children: Vec<(usize, bool, i32, usize)>,
    /// Nœuds gardés dont les états changent, et surfaces actives retirées :
    /// (nœud, avant, après).
    pub states: Vec<(usize, StateSet, StateSet)>,
}

/// Différence de `old` à `new` (numéros tirés des mêmes [`Ids`]). Un
/// enfant gardé qui change de rang est annoncé retiré puis ajouté ; les
/// retraits d'un même parent partent du rang le plus haut, pour que chaque
/// rang annoncé reste exact.
pub fn diff(old: &Tree, new: &Tree) -> Diff {
    let mut d = Diff::default();
    let mut removes = Vec::new();
    let mut adds = Vec::new();
    for n in &old.nodes {
        match new.get(n.id) {
            None => {
                d.removed.push(n.id);
                if let Some(p) = n.parent.filter(|p| new.get(*p).is_some()) {
                    removes.push((p, false, n.index_in_parent, n.id));
                }
                // Une surface active qui se ferme rend le clavier : annoncé
                // avant que son objet ne disparaisse.
                if n.states.contains(State::Active) {
                    d.states.push((n.id, n.states, StateSet::default()));
                }
            }
            Some(m) if m.index_in_parent != n.index_in_parent && m.parent == n.parent => {
                if let Some(p) = n.parent {
                    removes.push((p, false, n.index_in_parent, n.id));
                    adds.push((p, true, m.index_in_parent, n.id));
                }
            }
            Some(_) => {}
        }
    }
    for n in &new.nodes {
        match old.get(n.id) {
            None => {
                d.added.push(n.id);
                if let Some(p) = n.parent.filter(|p| old.get(*p).is_some()) {
                    adds.push((p, true, n.index_in_parent, n.id));
                }
            }
            Some(o) => {
                if o.name != n.name {
                    d.renamed.push(n.id);
                }
                if o.states != n.states {
                    d.states.push((n.id, o.states, n.states));
                }
            }
        }
    }
    removes.sort_by_key(|&(p, _, i, _)| (p, std::cmp::Reverse(i)));
    adds.sort_by_key(|&(p, _, i, _)| (p, i));
    d.children = removes.into_iter().chain(adds).collect();
    d
}

/// Langue annoncée au lecteur d'écran (`Locale`, `GetLocale`), au format
/// POSIX : celle de l'interface, réglage `language` compris (comme
/// [`crate::i18n::chrono_locale`]), pour que la voix suive les noms.
pub fn locale_of(lang: crate::i18n::Lang) -> &'static str {
    match lang {
        crate::i18n::Lang::Fr => "fr_FR",
        crate::i18n::Lang::En => "en_US",
    }
}

/// [`locale_of`] la langue en cours.
pub fn locale() -> &'static str {
    locale_of(crate::i18n::current())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etats_en_deux_mots() {
        let s = StateSet::shown()
            .with(State::Focusable)
            .with(State::HasPopup);
        let [lo, hi] = s.words();
        assert_eq!(
            lo,
            1 << 8 | 1 << 11 | 1 << 24 | 1 << 25 | 1 << 30,
            "Enabled, Focusable, Sensitive, Showing, Visible"
        );
        assert_eq!(hi, 1 << (42 - 32), "HasPopup dans le second mot");
        assert!(s.contains(State::Focusable));
        assert!(!s.without(State::Focusable).contains(State::Focusable));
    }

    #[test]
    fn arbre_aplati_en_ordre_prefixe() {
        let root = Node::new(Role::Application, "hyprdesk")
            .child(
                Node::new(Role::Frame, "Dock")
                    .child(Node::new(Role::PushButton, "Firefox"))
                    .child(Node::new(Role::PushButton, "Fichiers")),
            )
            .child(Node::new(Role::Frame, "Barre des menus"));
        let t = Tree::flatten(&root);
        let names: Vec<&str> = t.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(
            names,
            ["hyprdesk", "Dock", "Firefox", "Fichiers", "Barre des menus"]
        );
        assert_eq!(t.nodes[0].children, [1, 4]);
        assert_eq!(t.nodes[1].children, [2, 3]);
        assert_eq!(t.nodes[3].parent, Some(1));
        assert_eq!(t.nodes[3].index_in_parent, 1);
        assert_eq!(t.nodes[0].index_in_parent, -1);
        assert_eq!(Tree::path(0), ROOT_PATH);
        assert_eq!(Tree::path(3), "/org/a11y/atspi/accessible/3");
    }

    #[test]
    fn difference_par_cle() {
        let mut ids = Ids::default();
        let dock = |apps: &[&str]| {
            Node::new(Role::Application, "hyprdesk").child(
                Node::new(Role::Frame, "Dock").key("dock:A").children(
                    apps.iter()
                        .map(|a| Node::new(Role::PushButton, *a).key(format!("pin:{a}"))),
                ),
            )
        };
        let a = Tree::flatten_with(&dock(&["Firefox", "Fichiers"]), &mut ids);
        let b = Tree::flatten_with(&dock(&["Terminal", "Fichiers"]), &mut ids);
        let id = |t: &Tree, name: &str| t.nodes.iter().find(|n| n.name == name).unwrap().id;
        // Fichiers garde son objet malgré l'insertion devant lui.
        assert_eq!(id(&a, "Fichiers"), id(&b, "Fichiers"));
        let d = diff(&a, &b);
        let frame = id(&a, "Dock");
        assert_eq!(d.removed, [id(&a, "Firefox")]);
        assert_eq!(d.added, [id(&b, "Terminal")]);
        assert!(d.renamed.is_empty());
        assert_eq!(
            d.children,
            [
                (frame, false, 0, id(&a, "Firefox")),
                (frame, true, 0, id(&b, "Terminal"))
            ]
        );
        // Un nom qui change garde l'objet.
        let c = Tree::flatten_with(
            &Node::new(Role::Application, "hyprdesk")
                .child(Node::new(Role::Frame, "Dock (2)").key("dock:A")),
            &mut ids,
        );
        let d = diff(&b, &c);
        assert_eq!(d.renamed, [frame]);
        assert_eq!(d.removed.len(), 2, "les deux boutons");
        assert_eq!(d.children.len(), 2, "un événement par bouton retiré");
    }

    #[test]
    fn permutation_et_cles_en_double() {
        let mut ids = Ids::default();
        let row = |keys: &[&str]| {
            Node::new(Role::Application, "hyprdesk").children(
                keys.iter()
                    .map(|k| Node::new(Role::PushButton, *k).key(k.to_string())),
            )
        };
        let a = Tree::flatten_with(&row(&["a", "b"]), &mut ids);
        let b = Tree::flatten_with(&row(&["b", "a"]), &mut ids);
        let (ia, ib) = (a.nodes[1].id, a.nodes[2].id);
        assert_eq!((b.nodes[1].id, b.nodes[2].id), (ib, ia), "objets gardés");
        let d = diff(&a, &b);
        assert!(d.added.is_empty() && d.removed.is_empty());
        assert_eq!(
            d.children,
            [
                (0, false, 1, ib),
                (0, false, 0, ia),
                (0, true, 0, ib),
                (0, true, 1, ia)
            ],
            "rang changé : retrait (du plus haut) puis ajout"
        );
        // Deux frères de même clé : deux objets distincts.
        let c = Tree::flatten_with(&row(&["x", "x"]), &mut ids);
        assert_ne!(c.nodes[1].id, c.nodes[2].id);
    }

    #[test]
    fn etats_changes() {
        let mut ids = Ids::default();
        let dock = |focus: bool| {
            let b = Node::new(Role::PushButton, "Firefox").key("pin:firefox");
            Node::new(Role::Application, "hyprdesk").child(if focus {
                b.state(State::Focused)
            } else {
                b
            })
        };
        let a = Tree::flatten_with(&dock(false), &mut ids);
        let b = Tree::flatten_with(&dock(true), &mut ids);
        let d = diff(&a, &b);
        assert_eq!(d.states.len(), 1);
        let (id, before, after) = d.states[0];
        assert_eq!(id, a.nodes[1].id);
        assert!(!before.contains(State::Focused) && after.contains(State::Focused));
        // La surface active se ferme : sa perte est annoncée.
        let active = Tree::flatten_with(
            &Node::new(Role::Application, "hyprdesk").child(
                Node::new(Role::Frame, "Lanceur")
                    .key("launcher")
                    .state(State::Active),
            ),
            &mut ids,
        );
        let gone = Tree::flatten_with(&Node::new(Role::Application, "hyprdesk"), &mut ids);
        let d = diff(&active, &gone);
        assert_eq!(d.states.len(), 1);
        assert!(d.states[0].1.contains(State::Active) && !d.states[0].2.contains(State::Active));
    }

    #[test]
    fn arbre_en_texte() {
        let root = Node::new(Role::Application, "hyprdesk").child(
            Node::new(Role::Frame, "Dock").children([
                Node::new(Role::PushButton, "Firefox"),
                Node::new(Role::Label, ""),
            ]),
        );
        assert_eq!(
            root.dump(),
            "application « hyprdesk »\n  frame « Dock »\n    push button « Firefox »\n    label\n"
        );
    }

    #[test]
    fn roles_du_protocole() {
        assert_eq!(Role::PushButton as u32, 43);
        assert_eq!(Role::Application as u32, 75);
        assert_eq!(Role::Frame.name(), "frame");
        assert_eq!(Role::PushButton.name(), "push button");
    }

    #[test]
    fn langue_de_l_interface() {
        assert_eq!(locale_of(crate::i18n::Lang::Fr), "fr_FR");
        assert_eq!(locale_of(crate::i18n::Lang::En), "en_US");
    }
}
