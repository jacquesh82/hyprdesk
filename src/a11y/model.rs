//! Arbre d'accessibilité pur : rôles et états AT-SPI, nœuds, arbre aplati
//! et chemins des objets. Aucune dépendance à D-Bus : testable seul.

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
    pub role: Role,
    pub name: String,
    pub description: String,
    pub states: StateSet,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(role: Role, name: impl Into<String>) -> Node {
        Node {
            role,
            name: name.into(),
            description: String::new(),
            states: StateSet::shown(),
            children: Vec::new(),
        }
    }

    pub fn describe(mut self, d: impl Into<String>) -> Node {
        self.description = d.into();
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

/// Un nœud de l'arbre aplati : liens par rang dans [`Tree::nodes`].
#[derive(Clone, Debug, PartialEq)]
pub struct Flat {
    pub role: Role,
    pub name: String,
    pub description: String,
    pub states: StateSet,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// Rang dans le parent (`GetIndexInParent`), -1 pour la racine.
    pub index_in_parent: i32,
}

/// L'arbre aplati, en ordre préfixe : la racine est au rang 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub nodes: Vec<Flat>,
}

impl Tree {
    /// Aplatit `root`, qui devient le nœud 0 (l'application).
    pub fn flatten(root: &Node) -> Tree {
        let mut nodes = Vec::new();
        push(&mut nodes, root, None, -1);
        Tree { nodes }
    }

    /// Chemin D-Bus du nœud `i`.
    pub fn path(i: usize) -> String {
        if i == 0 {
            ROOT_PATH.to_string()
        } else {
            format!("{NODE_PREFIX}{i}")
        }
    }

    pub fn get(&self, i: usize) -> Option<&Flat> {
        self.nodes.get(i)
    }
}

fn push(nodes: &mut Vec<Flat>, n: &Node, parent: Option<usize>, index: i32) -> usize {
    let me = nodes.len();
    nodes.push(Flat {
        role: n.role,
        name: n.name.clone(),
        description: n.description.clone(),
        states: n.states,
        parent,
        children: Vec::new(),
        index_in_parent: index,
    });
    for (k, c) in n.children.iter().enumerate() {
        let id = push(nodes, c, Some(me), k as i32);
        nodes[me].children.push(id);
    }
    me
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
