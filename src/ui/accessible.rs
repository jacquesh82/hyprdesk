//! Arbre d'accessibilité des surfaces (HIG ACC-01) : calculé depuis l'état
//! de l'`App`, pas depuis l'arbre iced (sans sémantique), et nommé par les
//! libellés déjà affichés. Publié au pont seulement quand un lecteur d'écran
//! le lit (`crate::a11y::wanted`) ou que `hyprdesk a11y dump` le demande.

use crate::a11y::model::{Node, Role, State};
use crate::i18n::tr;
use crate::ui::plugins::bar::popover::PMsg;
use crate::ui::plugins::dock::{PopupContent, Slot};
use crate::ui::{App, Msg};
use iced::Task;

/// Collecteur des nœuds de la page des Réglages (HIG ACC-01), ouvert le
/// temps d'un dessin pour l'arbre (`App::a11y_page_nodes`) : les entrées
/// dans l'ordre du dessin, `bound` le rang après la dernière clôture.
#[derive(Default)]
pub(in crate::ui) struct Collect {
    entries: Vec<Entry>,
    bound: usize,
    next: usize,
}

/// Un nœud déposé : son numéro, ce qui dit s'il est encore dessiné (vivant
/// tant que l'élément de sa primitive l'est), la commande qu'un libellé de
/// `field` désigne.
struct Entry {
    seq: usize,
    node: Node,
    life: std::rc::Weak<()>,
    claim: Option<usize>,
}

/// Jeton d'un nœud déposé ([`App::a11y_note`]) : lié à l'élément de la
/// primitive par [`keep`], il garde le nœud tant que l'élément existe.
pub(in crate::ui) struct Noted {
    seq: usize,
    life: std::rc::Rc<()>,
}

/// Lie le nœud du jeton à l'élément : jeté, l'élément emporte le jeton et
/// le nœud sort de l'arbre. Sans jeton (arbre non lu), l'élément tel quel.
pub(in crate::ui) fn keep<'a>(
    noted: Option<Noted>,
    el: iced::Element<'a, Msg>,
) -> iced::Element<'a, Msg> {
    match noted {
        Some(n) => {
            let life = n.life;
            el.map(move |m| {
                let _ = &life;
                m
            })
        }
        None => el,
    }
}

impl App {
    /// Une `Frame` par barre et par dock, dans l'ordre des écrans ; en
    /// Graphite, le dock logé dans la barre est une `Frame` dans la sienne.
    pub(super) fn a11y_surfaces(&self) -> Vec<Node> {
        let mut out = Vec::new();
        if let Some(def) = self.cfg.bar.as_ref() {
            let mut bars: Vec<(&str, iced::window::Id)> = self
                .bar_surfaces
                .iter()
                .map(|(id, b)| (b.monitor.as_str(), *id))
                .collect();
            bars.sort_unstable_by_key(|&(monitor, _)| monitor);
            for (monitor, bar_id) in bars {
                let frame = format!("bar:{monitor}");
                // Écran vertical : la composition portrait, comme à l'écran.
                let zones = if self.bar_vertical(monitor) {
                    def.portrait.as_ref().unwrap_or(&def.zones)
                } else {
                    &def.zones
                };
                // Graphite : le dock logé dans la barre y est une `Frame`,
                // à la place de son module, comme le dock seul.
                let embedded = self
                    .monitors
                    .iter()
                    .find(|m| m.name == monitor)
                    .filter(|m| self.launcher_in_bar(&m.name));
                // La surface active est la fenêtre de premier niveau : la
                // barre, quand son dock tient le clavier (`Window:Activate`
                // part d'elle, pas de la `Frame` imbriquée).
                let mut bar = Node::new(Role::Frame, tr("Barre des menus")).key(frame);
                if let Some((x, y, w, h)) = self.bar_extents(bar_id) {
                    bar = bar.at(x, y, w, h);
                }
                if embedded.is_some() && self.dock_focus(monitor).is_some() {
                    bar = bar.state(State::Active);
                }
                // Titres des menus du shell, en tête (HIG MENU-01) ;
                // « Fenêtre » inactif sans fenêtre active, comme à l'écran.
                let titles = crate::ui::plugins::bar::menus::titles().into_iter().map(
                    |(key, label, request)| {
                        let n = Node::new(Role::PushButton, tr(label))
                            .key(key)
                            .state(State::Focusable)
                            .state(State::HasPopup);
                        if self.bar_title_live(&request) {
                            n.action()
                        } else {
                            let mut n = n;
                            n.states = n.states.without(State::Enabled).without(State::Sensitive);
                            n
                        }
                    },
                );
                out.push(
                    bar.children(titles)
                        .children(zones.iter().flatten().map(|item| {
                            match embedded.filter(|_| self.bar_item_is_launcher(item)) {
                                Some(m) => self.dock_a11y(m, false),
                                None => {
                                    let b = button(self.bar_item_label(item)).key(
                                        crate::ui::plugins::bar::surface::bar_item_key(item)
                                            .to_string(),
                                    );
                                    // Un module qui ouvre une carte s'active
                                    // comme son clic.
                                    if self.bar_item_press(bar_id, item).is_some() {
                                        b.action()
                                    } else {
                                        b
                                    }
                                }
                            }
                        })),
                );
            }
        }
        let mut docks: Vec<&str> = self
            .launcher_surfaces
            .values()
            .map(|d| d.monitor.as_str())
            .collect();
        docks.sort_unstable();
        for monitor in docks {
            let Some(m) = self.monitors.iter().find(|m| m.name == monitor) else {
                continue;
            };
            out.push(self.dock_a11y(m, true));
        }
        if let Some(l) = self.launcher_a11y() {
            out.push(l);
        }
        if let Some(m) = self.menu_a11y() {
            out.push(m);
        }
        if let Some(w) = self.switcher_a11y() {
            out.push(w);
        }
        if let Some(s) = self.settings_a11y() {
            out.push(s);
        }
        out
    }

    /// Tiroir des Réglages ouvert : son champ de recherche, ses pages (la
    /// navigation, recherche comprise : seules les pages qui ont une
    /// correspondance), le titre de la page et ses onglets ; la page et
    /// l'onglet affichés choisis. Pages et onglets s'activent comme leur clic.
    fn settings_a11y(&self) -> Option<Node> {
        use crate::ui::plugins;
        let p = self.panel.as_ref().filter(|_| self.panel_open())?;
        let nav = self.settings_pages();
        let tab = |role_key: String, label: &str, on: bool| {
            let n = Node::new(Role::PageTab, tr(label))
                .key(role_key)
                .state(State::Selectable)
                .state(State::Focusable)
                .action();
            if on { n.state(State::Selected) } else { n }
        };
        let pages = nav
            .iter()
            .map(|i| tab(format!("page:{}", i.id), i.label, p.tab == i.id));
        let mut frame = Node::new(Role::Frame, tr("Réglages"))
            .key("settings")
            .child(
                Node::new(Role::Entry, tr("Rechercher un réglage…"))
                    .describe(p.search.clone())
                    .key("search")
                    .state(State::Editable)
                    .state(State::Focusable),
            )
            .child(
                Node::new(Role::PageTabList, tr("Pages"))
                    .key("pages")
                    .children(pages),
            );
        if let Some(page) = plugins::BUILTIN.iter().find(|i| i.id == p.tab) {
            frame = frame.child(Node::new(Role::Heading, tr(page.label)).key("title"));
        }
        let on = self.subtab(p.tab);
        let tabs = plugins::visible_tabs(p.tab, self.advanced());
        // Tiroir large : les onglets sont sous l'entrée de la page dans la
        // navigation, absents avec elle (recherche sans correspondance).
        let shown = self.panel_width() < 640.0 || nav.iter().any(|i| i.id == p.tab);
        if !tabs.is_empty() && shown {
            frame = frame.child(
                Node::new(Role::PageTabList, tr("Onglets"))
                    .key("tabs")
                    .children(
                        tabs.into_iter()
                            .map(|(t, l)| tab(format!("tab:{t}"), l, t == on)),
                    ),
            );
        }
        // Le contenu de la page, sous ses onglets, comme à l'écran.
        let content = self.a11y_page_nodes(p);
        if !content.is_empty() {
            frame = frame.child(
                Node::new(Role::Panel, tr("Contenu"))
                    .key("content")
                    .children(content),
            );
        }
        Some(frame)
    }

    /// Le dock de l'écran `m`, seul ou logé dans la barre : ses cases en
    /// `PushButton`, les titres d'espace en `Label` ; au clavier, la surface
    /// active (`top` : surface de premier niveau ; logé dans la barre, c'est
    /// elle) et la case choisie au focus.
    fn dock_a11y(&self, m: &crate::hypr::Monitor, top: bool) -> Node {
        let monitor = m.name.as_str();
        let focus = self.dock_focus(monitor);
        let mut frame = Node::new(Role::Frame, tr("Dock")).key(format!("dock:{monitor}"));
        // Logé dans la barre, il prend la place de la sienne.
        if top
            && let Some(l) = self
                .launcher_surfaces
                .values()
                .find(|l| l.monitor == monitor)
        {
            let t = l.thickness as f32;
            let (mw, mh) = (m.width as f32, m.height as f32);
            let (x, y, w, h) = match l.position {
                crate::config::LauncherPosition::Bottom => (m.x, m.y + mh - t, mw, t),
                crate::config::LauncherPosition::Top => (m.x, m.y, mw, t),
                crate::config::LauncherPosition::Left => (m.x, m.y, t, mh),
                crate::config::LauncherPosition::Right => (m.x + mw - t, m.y, t, mh),
            };
            frame = frame.at(x, y, w, h);
        }
        if top && focus.is_some() {
            frame = frame.state(State::Active);
        }
        frame.children(self.dock_slots(m).into_iter().enumerate().filter_map(
            |(k, slot)| match slot {
                Slot::Item(i) => {
                    let b = button(self.dock_item_name(&i.popup))
                        .key(i.key.clone())
                        .action();
                    Some(if focus == Some(k) {
                        b.state(State::Focused)
                    } else {
                        b
                    })
                }
                Slot::Label(ws) => Some(Node::new(Role::Label, ws.clone()).key(format!("ws:{ws}"))),
                Slot::Separator => None,
            },
        ))
    }

    /// Rangées et commandes de la page affichée des Réglages : la page est
    /// dessinée pour l'arbre, et ses primitives (`section`, `adv`, `field`,
    /// `slider`, `toggle`) y déposent leurs nœuds dans l'ordre du dessin.
    /// Chaque nœud vit avec l'élément de sa primitive : ce qui est construit
    /// puis jeté (l'onglet qu'une page écarte par `match`, une branche non
    /// retenue) n'y entre pas. Calculé quand l'arbre l'est (pont actif,
    /// `hyprdesk a11y dump`), jamais pendant le dessin de l'écran : l'arbre
    /// suit l'état, pas la dernière image. Résultats de la recherche en
    /// liste : pas de page, rien.
    fn a11y_page_nodes(&self, p: &crate::ui::panel::Panel) -> Vec<Node> {
        if p.search_list && !p.search.trim().is_empty() {
            return Vec::new();
        }
        let mut nodes = self.a11y_collected(|| crate::ui::plugins::view(self, p, p.tab));
        // Une commande restée sans nom (hors d'une `field`, sans `named`) ne
        // sert à rien au lecteur d'écran : elle n'est pas montrée.
        nodes.retain(|n| !n.name.is_empty());
        nodes
    }

    /// Les nœuds que le dessin `view` dépose, collecteur ouvert le temps de
    /// ce dessin (voir [`App::a11y_page_nodes`]).
    fn a11y_collected<'a>(&'a self, view: impl FnOnce() -> iced::Element<'a, Msg>) -> Vec<Node> {
        *self.a11y_collect.borrow_mut() = Some(Collect::default());
        let page = view();
        let entries = self
            .a11y_collect
            .borrow_mut()
            .take()
            .map(|c| c.entries)
            .unwrap_or_default();
        // Vivants tant que la page l'est : ce qu'elle a gardé.
        let kept: Vec<Entry> = entries
            .into_iter()
            .filter(|e| e.life.strong_count() > 0)
            .collect();
        drop(page);
        // Le libellé d'une `field` nomme la commande qu'il désigne et
        // s'efface (pas de lecture en double) ; commande jetée, il reste.
        let alive: std::collections::HashSet<usize> = kept.iter().map(|e| e.seq).collect();
        let mut names: std::collections::HashMap<usize, String> = kept
            .iter()
            .filter_map(|e| {
                e.claim
                    .filter(|l| alive.contains(l))
                    .map(|l| (l, e.node.name.clone()))
            })
            .collect();
        kept.into_iter()
            .filter(|e| !e.claim.is_some_and(|l| alive.contains(&l)))
            .map(|e| {
                let mut n = e.node;
                if let Some(name) = names.remove(&e.seq) {
                    n.name = name;
                }
                // Une clé stable par nœud (rôle et nom) : la différence suit
                // le nœud d'un dessin à l'autre, pas son rang.
                if n.key.is_empty() {
                    n.key = format!("{}:{}", n.role.name(), n.name);
                }
                n
            })
            .collect()
    }

    /// Nœud déposé par une primitive de page pendant le dessin pour l'arbre
    /// (sans effet sinon). Le jeton rendu le garde en vie : la primitive le
    /// lie à son élément ([`keep`]).
    pub(in crate::ui) fn a11y_note(&self, node: impl FnOnce() -> Node) -> Option<Noted> {
        let mut g = self.a11y_collect.borrow_mut();
        let c = g.as_mut()?;
        let life = std::rc::Rc::new(());
        let seq = c.next;
        c.next += 1;
        c.entries.push(Entry {
            seq,
            node: node(),
            life: std::rc::Rc::downgrade(&life),
            claim: None,
        });
        Some(Noted { seq, life })
    }

    /// Clôture : une `field` qui suit ne nomme que ce qui est déposé après
    /// (titre de section, triangle d'une section avancée).
    pub(in crate::ui) fn a11y_bound(&self) {
        if let Some(c) = self.a11y_collect.borrow_mut().as_mut() {
            c.bound = c.entries.len();
        }
    }

    /// Le nœud du jeton `at`, s'il est encore au collecteur.
    fn a11y_with(&self, at: Option<&Noted>, f: impl FnOnce(&mut Node)) {
        let mut g = self.a11y_collect.borrow_mut();
        if let (Some(c), Some(at)) = (g.as_mut(), at)
            && let Some(e) = c.entries.iter_mut().rev().find(|e| e.seq == at.seq)
        {
            f(&mut e.node);
        }
    }

    /// Nomme le nœud du jeton `at` (libellé d'un interrupteur).
    pub(in crate::ui) fn a11y_rename(&self, at: Option<&Noted>, name: &str) {
        self.a11y_with(at, |n| n.name = name.to_string());
    }

    /// Le nœud du jeton `at` devient actif (commande avec message) :
    /// focalisable au clavier quand l'accès clavier complet l'y met.
    pub(in crate::ui) fn a11y_enable(&self, at: Option<&Noted>) {
        let keys = self.cfg.general.keyboard_access;
        self.a11y_with(at, |n| {
            n.states = n.states.with(State::Enabled).with(State::Sensitive);
            if keys {
                n.states = n.states.with(State::Focusable);
            }
        });
    }

    /// Fin d'une `field` : son libellé nomme la commande que son contrôle a
    /// déposée (un curseur, un interrupteur). Le contrôle s'évalue avant la
    /// `field` : ses commandes sont celles déposées sans nom depuis la
    /// dernière clôture. Une seule sans nom : le libellé la désigne (il la
    /// nommera si elle est gardée, sinon il reste seul) ; sinon le libellé en
    /// `Label`, avant les commandes du contrôle. Une commande sans libellé se
    /// dessine donc dans une `field`, ou se nomme (`named`). Libellé vide
    /// : celui du triangle d'une section avancée qui l'enveloppe
    /// directement, pour sa seule commande. Le jeton rendu lie le libellé à
    /// l'élément de la `field`.
    pub(in crate::ui) fn a11y_field(&self, label: &str) -> Option<Noted> {
        let mut g = self.a11y_collect.borrow_mut();
        let c = g.as_mut()?;
        let open: Vec<usize> = (c.bound..c.entries.len())
            .filter(|&i| c.entries[i].node.name.is_empty())
            .collect();
        let mut noted = None;
        if label.is_empty() {
            let adv = c
                .bound
                .checked_sub(1)
                .and_then(|i| c.entries.get(i))
                .filter(|e| e.node.key.starts_with("adv:"))
                .map(|e| e.node.name.clone());
            if let (Some(name), [one]) = (adv, open.as_slice()) {
                c.entries[*one].node.name = name;
            }
        } else {
            let life = std::rc::Rc::new(());
            let seq = c.next;
            c.next += 1;
            let entry = Entry {
                seq,
                node: Node::new(Role::Label, tr(label).to_string()),
                life: std::rc::Rc::downgrade(&life),
                claim: match open.as_slice() {
                    [one] => Some(c.entries[*one].seq),
                    _ => None,
                },
            };
            // Avant la première commande déposée depuis la clôture : celles
            // du contrôle, interrupteurs à libellé compris.
            let first = c.bound.min(c.entries.len());
            c.entries.insert(first, entry);
            noted = Some(Noted { seq, life });
        }
        c.bound = c.entries.len();
        noted
    }

    /// Pages de la navigation des Réglages, dans l'ordre de la vue.
    fn settings_pages(&self) -> Vec<&'static crate::ui::plugins::PluginInfo> {
        let query = self
            .panel
            .as_ref()
            .map(|p| p.search.trim().to_string())
            .unwrap_or_default();
        let found = super::settings_search::search(&query, |id| self.setting_visible(id));
        crate::ui::plugins::GROUPS
            .iter()
            .flat_map(|g| self.nav_items(g.ids, &found))
            .collect()
    }

    /// Place de la barre `id` à l'écran : celle que sa forme lui donne
    /// (`bar_roi` : bord, marges, longueur, alignement), sur son écran.
    fn bar_extents(&self, id: iced::window::Id) -> Option<(f32, f32, f32, f32)> {
        let b = self.bar_surfaces.get(&id)?;
        let m = self.monitors.iter().find(|m| m.name == b.monitor)?;
        let r = self.bar_roi(b);
        Some((m.x + r.x, m.y + r.y, r.width, r.height))
    }

    /// Case du dock de l'écran `monitor` choisie au clavier (le rang compte
    /// toutes les cases, comme la sélection).
    fn dock_focus(&self, monitor: &str) -> Option<usize> {
        self.launcher_runtime
            .focus
            .as_ref()
            .filter(|(name, _)| name == monitor)
            .map(|(_, i)| *i)
    }

    /// Lanceur ouvert : surface active, champ de recherche, et en mode
    /// applications la liste des résultats, la sélection au focus.
    fn launcher_a11y(&self) -> Option<Node> {
        let s = self.search.as_ref().filter(|s| s.shown)?;
        // Mode de la carte, comme la vue : son invite nomme le champ.
        let kind = s.view_kind();
        let prompt = match kind {
            3 => tr("Rechercher un fichier ou un dossier…"),
            4 => tr("Rechercher dans les réglages…"),
            _ => tr("Rechercher une application…"),
        };
        let mut items: Vec<Node> = Vec::new();
        if kind == 0 {
            for (k, &i) in s.results.iter().enumerate() {
                let Some(app) = self.apps.get(i) else {
                    continue;
                };
                let item = Node::new(Role::ListItem, app.name.clone())
                    .key(format!("app:{}", app.id))
                    .action()
                    .state(State::Selectable)
                    .state(State::Focusable);
                items.push(if k == s.selected {
                    item.state(State::Selected).state(State::Focused)
                } else {
                    item
                });
            }
        }
        let chosen = items.iter().any(|n| n.states.contains(State::Focused));
        let mut field = Node::new(Role::Entry, prompt)
            .key("query")
            .describe(s.query.clone())
            .state(State::Editable)
            .state(State::Focusable);
        // Sans résultat choisi, le focus reste au champ.
        if !chosen {
            field = field.state(State::Focused);
        }
        let mut frame = Node::new(Role::Frame, tr("Lanceur"))
            .key("launcher")
            .state(State::Active)
            .child(field);
        if kind == 0 {
            frame = frame.child(
                Node::new(Role::List, tr("Résultats"))
                    .key("results")
                    .children(items),
            );
        }
        Some(frame)
    }

    /// Menu ouvert depuis la barre : surface active, ses entrées en
    /// `MenuItem`, sous-menus compris ; l'entrée choisie au clavier, que
    /// l'`App` tient (`Popover::keyed`), au focus.
    pub(in crate::ui) fn menu_a11y(&self) -> Option<Node> {
        let p = self.popover.as_ref().filter(|p| !p.closing())?;
        let crate::ui::plugins::bar::popover::Kind::Menu { entries, .. } = &p.kind else {
            return None;
        };
        // Nom : le titre du menu de la barre, sinon le libellé du module
        // qui l'ouvre, sinon « Menu » ; jamais une clé interne.
        let module = crate::ui::plugins::bar::popover::module_key(&p.key);
        let name = crate::ui::plugins::bar::menus::titles()
            .into_iter()
            .find(|(key, ..)| *key == p.key)
            .map(|(_, title, _)| tr(title).to_string())
            .or_else(|| {
                crate::config::BAR_ITEMS
                    .iter()
                    .find(|(id, _)| *id == module)
                    .map(|(_, l)| tr(l).to_string())
            })
            .unwrap_or_else(|| tr("Menu").to_string());
        let mut menu = Node::new(Role::Menu, name);
        if let Some(m) = self
            .bar_surfaces
            .get(&p.bar)
            .and_then(|b| self.monitors.iter().find(|m| m.name == b.monitor))
            && let Some((x, y, w, h)) = p.screen_rect(m)
        {
            menu = menu.at(x, y, w, h);
        }
        Some(
            menu.key(p.key.clone())
                .state(State::Active)
                .children(menu_items(entries, p.sub, p.keyed, p.keyed_sub, false)),
        )
    }

    /// Commutateur de fenêtres ouvert : surface active, la fenêtre choisie
    /// au focus.
    fn switcher_a11y(&self) -> Option<Node> {
        let sw = self.switcher.as_ref()?;
        // Rien à choisir : le texte affiché à la place de la liste.
        let empty = sw.wins.is_empty().then(|| {
            Node::new(
                Role::Label,
                tr(if sw.loading {
                    "Lecture des fenêtres…"
                } else {
                    "Aucune fenêtre ouverte."
                }),
            )
        });
        Some(
            Node::new(Role::Frame, tr("Fenêtres"))
                .key("switcher")
                .state(State::Active)
                .children(empty)
                .children(sw.wins.iter().enumerate().map(|(k, w)| {
                    let name = if w.title.is_empty() {
                        &w.class
                    } else {
                        &w.title
                    };
                    let item = Node::new(Role::ListItem, name.clone())
                        .key(format!("win:{}", w.address))
                        .action()
                        .state(State::Selectable)
                        .state(State::Focusable);
                    if k == sw.selected {
                        item.state(State::Selected).state(State::Focused)
                    } else {
                        item
                    }
                })),
        )
    }

    /// Nom d'une case du dock : celui de sa bulle ; pour une application
    /// regroupée, le nom de l'application plutôt que sa classe.
    fn dock_item_name(&self, popup: &PopupContent) -> String {
        match popup {
            PopupContent::Label(name) => name.clone(),
            PopupContent::Windows(class) => self
                .apps
                .iter()
                .find(|a| self.app_matches(&a.id, class))
                .map_or_else(|| class.clone(), |a| a.name.clone()),
        }
    }

    /// `DoAction` du lecteur d'écran sur le nœud de clé `path` (chemin des
    /// clés depuis la racine, `/dock:A/dock-pin:firefox`) : le clic que la
    /// surface exécuterait. Un nœud disparu entre-temps : rien.
    pub(super) fn a11y_action(&mut self, path: &str) -> Task<Msg> {
        let Some((head, rest)) = path.trim_start_matches('/').split_once('/') else {
            return Task::none();
        };
        // Menu ouvert depuis la barre : `e<rang>`, puis le rang dans la
        // colonne de son sous-menu.
        if self
            .popover
            .as_ref()
            .is_some_and(|p| p.key == head && !p.closing())
        {
            let rank = |s: &str| s.strip_prefix('e').and_then(|n| n.parse::<usize>().ok());
            return match rest.split_once('/') {
                None => match rank(rest) {
                    Some(i) => self.popover_update(PMsg::Activate(i)),
                    None => Task::none(),
                },
                Some((sub, item)) => match (rank(sub), rank(item)) {
                    // La colonne doit être celle qu'on a lue.
                    (Some(i), Some(j))
                        if self.popover.as_ref().is_some_and(|p| p.sub == Some(i)) =>
                    {
                        self.popover_update(PMsg::SubActivate(j))
                    }
                    _ => Task::none(),
                },
            };
        }
        if let Some(monitor) = head.strip_prefix("dock:") {
            return self.a11y_dock_action(monitor, rest);
        }
        if let Some(monitor) = head.strip_prefix("bar:") {
            // Graphite : le dock logé dans la barre.
            if let Some((dock, item)) = rest.split_once('/')
                && dock.strip_prefix("dock:") == Some(monitor)
            {
                return self.a11y_dock_action(monitor, item);
            }
            let bar = self
                .bar_surfaces
                .iter()
                .find(|(_, s)| s.monitor == monitor)
                .map(|(id, _)| *id);
            let Some(bar) = bar else {
                return Task::none();
            };
            let title = crate::ui::plugins::bar::menus::titles()
                .into_iter()
                .find(|(key, ..)| *key == rest);
            if let Some((key, _, request)) = title {
                return if self.bar_title_live(&request) {
                    Task::done(Msg::OpenPopover(bar, key.into(), request))
                } else {
                    Task::none()
                };
            }
            // Un module de la composition de cet écran : le clic que sa vue
            // lui donne.
            let press = self.cfg.bar.as_ref().and_then(|def| {
                let zones = if self.bar_vertical(monitor) {
                    def.portrait.as_ref().unwrap_or(&def.zones)
                } else {
                    &def.zones
                };
                zones
                    .iter()
                    .flatten()
                    .find(|item| crate::ui::plugins::bar::surface::bar_item_key(item) == rest)
                    .and_then(|item| self.bar_item_press(bar, item))
            });
            return press.map_or_else(Task::none, Task::done);
        }
        match (head, rest) {
            // Réglages : une page de la navigation, un onglet de la page.
            ("settings", item) => {
                use crate::ui::plugins;
                // Seulement ce que l'arbre montre : tiroir ouvert, page de
                // la navigation, onglet affiché.
                let Some(tree) = self.settings_a11y() else {
                    return Task::none();
                };
                let tab_shown = |t: &str| {
                    tree.children
                        .iter()
                        .filter(|n| n.key == "tabs")
                        .flat_map(|n| n.children.iter())
                        .any(|n| n.key == format!("tab:{t}"))
                };
                if let Some(id) = item.strip_prefix("pages/page:") {
                    return match self.settings_pages().into_iter().find(|i| i.id == id) {
                        Some(i) => self.panel_update(super::panel::PanelMsg::Tab(i.id)),
                        None => Task::none(),
                    };
                }
                let Some(t) = item.strip_prefix("tabs/tab:").filter(|t| tab_shown(t)) else {
                    return Task::none();
                };
                let Some(pkg) = self.panel.as_ref().map(|p| p.tab) else {
                    return Task::none();
                };
                match plugins::visible_tabs(pkg, self.advanced())
                    .into_iter()
                    .find(|(id, _)| *id == t)
                {
                    Some((id, _)) => self.panel_update(super::panel::PanelMsg::SubTab(pkg, id)),
                    None => Task::none(),
                }
            }
            ("switcher", win) => {
                let address = win.strip_prefix("win:").unwrap_or_default();
                let Some(sw) = self.switcher.as_mut() else {
                    return Task::none();
                };
                let Some(k) = sw.wins.iter().position(|w| w.address == address) else {
                    return Task::none();
                };
                sw.selected = k;
                self.switcher_commit()
            }
            ("launcher", item) => {
                let id = item.strip_prefix("results/app:").unwrap_or_default();
                // Seulement un résultat encore affiché : un `DoAction` lu
                // avant une nouvelle saisie ne lance pas ce qui n'y est plus.
                let shown = self
                    .search
                    .as_ref()
                    .filter(|s| s.shown && s.view_kind() == 0)
                    .and_then(|s| {
                        s.results
                            .iter()
                            .copied()
                            .find(|&i| self.apps.get(i).is_some_and(|a| a.id == id))
                    });
                match shown {
                    Some(i) => self.search_update(super::search::SMsg::LaunchApp(i)),
                    None => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }

    /// Case `key` du dock de l'écran `monitor` : son clic.
    fn a11y_dock_action(&mut self, monitor: &str, key: &str) -> Task<Msg> {
        let Some(m) = self.monitors.iter().find(|m| m.name == monitor).cloned() else {
            return Task::none();
        };
        let msg = self.dock_slots(&m).into_iter().find_map(|slot| match slot {
            Slot::Item(i) if i.key == key => Some(i.msg),
            _ => None,
        });
        match msg {
            Some(msg) => self.launcher_activate(msg),
            None => Task::none(),
        }
    }

    /// Après chaque mise à jour : l'arbre va au pont s'il est lu.
    pub(super) fn a11y_publish(&self) {
        if crate::a11y::wanted() {
            crate::a11y::publish(self.a11y_surfaces());
        }
    }
}

/// Entrées d'un menu : commandes en `MenuItem`, sous-menus en `Menu`
/// (déplié si `open` est son rang), titres en `Label`, champs en `Entry`.
/// `keyed` : l'entrée choisie au clavier, au focus ; `keyed_sub` : celle de
/// la colonne d'un sous-menu (ligne parente, rang), tant qu'il est déplié.
/// `nested` : la colonne d'un sous-menu, où seules les commandes sans
/// confirmation s'activent.
fn menu_items(
    entries: &[crate::config::MenuEntry],
    open: Option<usize>,
    keyed: Option<usize>,
    keyed_sub: Option<(usize, usize)>,
    nested: bool,
) -> Vec<Node> {
    use crate::config::MenuEntry;
    use crate::ui::plugins::bar::popover::keyable;
    let off = |mut n: Node, enabled: bool| {
        if !enabled {
            n.states = n.states.without(State::Enabled).without(State::Sensitive);
        }
        n
    };
    // Descendu dans une colonne : le focus est à elle, plus à sa ligne.
    let column = keyed_sub.filter(|(parent, _)| open == Some(*parent));
    let focus = |n: Node, i: usize| {
        if column.is_none() && keyed == Some(i) {
            n.state(State::Focused)
        } else {
            n
        }
    };
    entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e {
            MenuEntry::Item {
                label,
                checked,
                enabled,
                ..
            } => {
                // Clé : le rang de l'entrée, que `DoAction` active.
                let mut n = Node::new(Role::MenuItem, label.clone())
                    .key(format!("e{i}"))
                    .state(State::Focusable);
                if keyable(e, nested) {
                    n = n.action();
                }
                if let Some(c) = checked {
                    n = n.state(State::Checkable);
                    if *c {
                        n = n.state(State::Checked);
                    }
                }
                Some(focus(off(n, *enabled), i))
            }
            MenuEntry::Submenu {
                label,
                enabled,
                children,
                ..
            } => {
                let mut n = Node::new(Role::Menu, label.clone())
                    .key(format!("e{i}"))
                    .state(State::Focusable)
                    .state(State::Expandable)
                    .state(State::HasPopup);
                // Activé, il ouvre sa colonne.
                if !nested && keyable(e, false) {
                    n = n.action();
                }
                if open == Some(i) {
                    n = n.state(State::Expanded).children(menu_items(
                        children,
                        None,
                        column.map(|(_, j)| j),
                        None,
                        true,
                    ));
                }
                Some(focus(off(n, *enabled), i))
            }
            MenuEntry::Header(t) => Some(Node::new(Role::Label, t.clone())),
            MenuEntry::Input {
                label, placeholder, ..
            } => {
                let n = Node::new(
                    Role::Entry,
                    label.clone().unwrap_or_else(|| placeholder.clone()),
                )
                .state(State::Editable)
                .state(State::Focusable);
                // Le premier champ tient le clavier tant qu'aucune entrée
                // n'est choisie (il le prend à l'ouverture).
                let first = entries
                    .iter()
                    .position(|e| matches!(e, MenuEntry::Input { .. }));
                Some(if keyed.is_none() && column.is_none() && first == Some(i) {
                    n.state(State::Focused)
                } else {
                    n
                })
            }
            MenuEntry::Separator | MenuEntry::Image(_) => None,
        })
        .collect()
}

fn button(name: impl Into<String>) -> Node {
    Node::new(Role::PushButton, name).state(State::Focusable)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use iced::window::Id;

    use crate::ui::plugins::bar::surface::BarSurface;
    use crate::ui::plugins::dock::LauncherSurface;
    use crate::ui::{App, DesktopMode};

    /// Instantané : une barre et un dock sur l'écran « A », nommés par les
    /// libellés affichés.
    #[test]
    fn arbre_de_la_barre_et_du_dock() {
        let (mut app, _) =
            App::boot_with(PathBuf::from("/tmp/hyprdesk-a11y-tests/config.lua"), true);
        let loaded = crate::config::lua::load_source(
            "hd.bar({left={'workspaces'},right={'volume','tray'},portrait={left={'volume'}}})",
            "test.lua",
            Path::new("/tmp"),
        )
        .unwrap();
        app.cfg = Arc::new(loaded.config);
        {
            let l = &mut Arc::make_mut(&mut app.cfg).general.launcher;
            l.pinned = vec!["firefox".into()];
            l.show_settings = true;
            // LYRA dépend du poste : hors de l'instantané.
            l.show_assistant = false;
        }
        app.test_mode(DesktopMode::FullDesktop);
        app.bar_surfaces.insert(
            Id::unique(),
            BarSurface {
                monitor: "A".into(),
                ..Default::default()
            },
        );
        app.launcher_surfaces.insert(
            Id::unique(),
            LauncherSurface {
                monitor: "A".into(),
                position: Default::default(),
                thickness: 80,
                over_fullscreen: false,
            },
        );
        let tree = crate::a11y::root(app.a11y_surfaces());
        // Actions : les titres de menu et les cases du dock s'activent ;
        // « Fenêtre », sans fenêtre active, est inactif, sans action.
        let bar = &tree.children[0];
        assert!(bar.children[0].action, "hyprdesk");
        assert!(!bar.children[1].action, "Fenêtre");
        assert!(
            !bar.children[1]
                .states
                .contains(crate::a11y::model::State::Enabled)
        );
        assert!(tree.children[1].children.iter().all(|b| b.action));
        // Un module qui ouvre une carte s'active ; les espaces, non (leurs
        // touches sont à part).
        let module = |name: &str| bar.children.iter().find(|n| n.name == name).unwrap();
        assert!(module("Volume").action);
        assert!(!module("Workspaces").action);
        // `DoAction` : le clic du module, celui du titre actif ; rien pour
        // un titre inactif ou une clé inconnue.
        assert_eq!(app.a11y_action("/bar:A/volume").units(), 1);
        assert_eq!(app.a11y_action("/bar:A/menus:hyprdesk").units(), 1);
        assert_eq!(app.a11y_action("/bar:A/menus:window").units(), 0);
        assert_eq!(app.a11y_action("/bar:A/workspaces").units(), 0);
        assert_eq!(app.a11y_action("/bar:B/volume").units(), 0);
        // Comme la vue : la fenêtre sans fenêtre active, la luminosité sans
        // niveau n'ont rien à cliquer.
        use crate::config::BarItem;
        let bar_id = *app.bar_surfaces.keys().next().unwrap();
        assert!(app.bar_item_press(bar_id, &BarItem::Window).is_none());
        app.active_window = Some(("foot".into(), String::new()));
        assert!(app.bar_item_press(bar_id, &BarItem::Window).is_some());
        app.active_window = None;
        if app.light.level_for("A").is_none() {
            assert!(app.bar_item_press(bar_id, &BarItem::Brightness).is_none());
        }
        // Dock au clavier sur Firefox : surface active, bouton au focus.
        app.launcher_runtime.focus = Some(("A".into(), 0));
        let focused = app.a11y_surfaces();
        let dock = &focused[1];
        assert!(dock.states.contains(crate::a11y::model::State::Active));
        assert!(
            dock.children[0]
                .states
                .contains(crate::a11y::model::State::Focused)
        );
        assert!(
            !dock.children[1]
                .states
                .contains(crate::a11y::model::State::Focused)
        );
        app.launcher_runtime.focus = None;

        // Lanceur ouvert sans résultat : surface active, focus au champ ; en
        // mode réglages, le champ prend l'invite de ce mode.
        let _ = app.toggle_search();
        {
            let s = app.search.as_mut().expect("lanceur ouvert");
            s.shown = true;
            s.results.clear();
        }
        let launcher = app.launcher_a11y().expect("lanceur décrit");
        assert!(launcher.states.contains(crate::a11y::model::State::Active));
        let field = &launcher.children[0];
        assert!(field.states.contains(crate::a11y::model::State::Focused));
        assert_eq!(field.name, "Rechercher une application…");
        app.search.as_mut().unwrap().query = ":r gamma".into();
        let launcher = app.launcher_a11y().unwrap();
        assert_eq!(launcher.children[0].name, "Rechercher dans les réglages…");
        assert_eq!(launcher.children.len(), 1, "pas de liste d'applications");
        app.search = None;

        // Commutateur : la fenêtre choisie au focus, nommée par son titre.
        let win = |address: &str, title: &str| crate::switcher::Win {
            address: address.into(),
            class: "firefox".into(),
            title: title.into(),
            workspace: "1".into(),
            focus: 0,
            stashed: false,
        };
        app.switcher = Some(crate::ui::switcher::Switcher {
            win: Id::unique(),
            wins: vec![win("0x1", "Courrier"), win("0x2", "")],
            selected: 1,
            loading: false,
            back: false,
        });
        let sw = app.switcher_a11y().unwrap();
        assert_eq!(
            sw.dump(),
            "frame « Fenêtres »\n  list item « Courrier »\n  list item « firefox »\n"
        );
        assert!(
            sw.children[1]
                .states
                .contains(crate::a11y::model::State::Focused)
        );
        app.switcher.as_mut().unwrap().wins.clear();
        app.switcher.as_mut().unwrap().loading = true;
        assert_eq!(
            app.switcher_a11y().unwrap().dump(),
            "frame « Fenêtres »\n  label « Lecture des fenêtres… »\n"
        );
        app.switcher = None;
        let mut portrait = app.monitors[0].clone();
        portrait.name = "B".into();
        (portrait.width, portrait.height) = (900, 1440);
        app.monitors.push(portrait);
        app.bar_surfaces.insert(
            Id::unique(),
            BarSurface {
                monitor: "B".into(),
                ..Default::default()
            },
        );
        // Écran vertical : la composition portrait de la barre.
        let surfaces = app.a11y_surfaces();
        assert_eq!(
            crate::a11y::model::Node::dump(&surfaces[1]),
            "frame « Barre des menus »\n  push button « hyprdesk »\n  push button « Fenêtre »\n  push button « Espaces »\n  push button « Aide »\n  push button « Volume »\n"
        );
        assert_eq!(
            tree.dump(),
            "application « hyprdesk »
  frame « Barre des menus »
    push button « hyprdesk »
    push button « Fenêtre »
    push button « Espaces »
    push button « Aide »
    push button « Workspaces »
    push button « Volume »
    push button « Zone de notification »
  frame « Dock »
    push button « firefox »
    push button « Toutes les applications »
    push button « Réglages »
"
        );
    }

    /// Graphite : le dock logé dans la barre est une `Frame` dans la sienne, à
    /// la place de son module ; au clavier, la barre est la surface active et
    /// la case choisie au focus.
    #[test]
    fn dock_loge_dans_la_barre() {
        use crate::a11y::model::{Role, State};
        let (mut app, _) =
            App::boot_with(PathBuf::from("/tmp/hyprdesk-a11y-tests/config.lua"), true);
        app.monitors = vec![crate::hypr::Monitor {
            dpms_off: false,
            name: "A".into(),
            width: 1440,
            height: 900,
            workspace: "1".into(),
            base: String::new(),
            focused: true,
            scale: 1.0,
            reserved: (0.0, 0.0, 0.0, 0.0),
            x: 0.0,
            y: 0.0,
        }];
        let loaded = crate::config::lua::load_source(
            "hd.bar({ style = 'graphite' })",
            "test.lua",
            Path::new("/tmp"),
        )
        .unwrap();
        app.cfg = Arc::new(loaded.config);
        {
            let l = &mut Arc::make_mut(&mut app.cfg).general.launcher;
            l.pinned = vec!["firefox".into()];
            l.show_settings = true;
            l.show_assistant = false;
        }
        app.test_mode(DesktopMode::FullDesktop);
        let _ = app.sync_bars();
        let _ = app.sync_launchers();
        assert!(app.launcher_in_bar("A"));
        let surfaces = app.a11y_surfaces();
        assert_eq!(surfaces.len(), 1, "pas de dock à part : {surfaces:?}");
        let dock = |surfaces: &[crate::a11y::model::Node]| {
            surfaces[0]
                .children
                .iter()
                .find(|n| n.role == Role::Frame && n.name == "Dock")
                .cloned()
                .expect("le dock dans la barre")
        };
        let d = dock(&surfaces);
        assert_eq!(d.key, "dock:A");
        // Sa place : celle de la barre, sur l'écran de 1440 px.
        assert!(d.extents.is_none());
        let [x, _, w, h] = surfaces[0].extents.expect("place de la barre");
        assert_eq!((x, w), (0, 1440));
        assert!(h > 0);
        assert!(d.children.iter().any(|n| n.name == "firefox"));
        assert!(!surfaces[0].states.contains(State::Active));
        // Le module du dock y prend la place de son bouton : un nœud par
        // titre de menu du shell et par module, une seule `Frame`.
        let items = app.cfg.bar.as_ref().unwrap().zones.iter().flatten().count();
        let titles = crate::ui::plugins::bar::menus::titles().len();
        assert_eq!(surfaces[0].children.len(), titles + items);
        assert_eq!(
            surfaces[0]
                .children
                .iter()
                .filter(|n| n.role == Role::Frame)
                .count(),
            1
        );
        // Au clavier : la barre, fenêtre de premier niveau, est la surface
        // active ; la case choisie au focus.
        let _ = app.launcher_grab("A", true);
        let surfaces = app.a11y_surfaces();
        assert!(surfaces[0].states.contains(State::Active));
        let d = dock(&surfaces);
        assert!(!d.states.contains(State::Active));
        assert_eq!(
            d.children
                .iter()
                .filter(|n| n.states.contains(State::Focused))
                .count(),
            1
        );
    }

    /// Réglages : champ de recherche, pages (la page affichée choisie),
    /// titre, onglets ; une page s'ouvre par `DoAction`.
    #[test]
    fn reglages_decrits() {
        use crate::a11y::model::{Role, State};
        let (mut app, _) =
            App::boot_with(PathBuf::from("/tmp/hyprdesk-a11y-tests/config.lua"), true);
        app.surfaces.insert(Id::unique(), "A".into());
        assert!(app.settings_a11y().is_none(), "fermés : rien");
        let _ = app.open_panel();
        let s = app.settings_a11y().expect("ouverts");
        assert_eq!(s.children[0].role, Role::Entry);
        let pages = &s.children[1];
        assert_eq!(pages.role, Role::PageTabList);
        let chosen: Vec<&str> = pages
            .children
            .iter()
            .filter(|n| n.states.contains(State::Selected))
            .map(|n| n.key.as_str())
            .collect();
        let tab = app.panel.as_ref().unwrap().tab;
        assert_eq!(chosen, [format!("page:{tab}")]);
        assert!(pages.children.iter().all(|n| n.action));
        // Une autre page, par le lecteur d'écran.
        let other = pages
            .children
            .iter()
            .find(|n| !n.states.contains(State::Selected))
            .expect("une autre page")
            .key
            .clone();
        let _ = app.a11y_action(&format!("/settings/pages/{other}"));
        assert_eq!(
            format!("page:{}", app.panel.as_ref().unwrap().tab),
            other,
            "la page lue est affichée"
        );
        let s = app.settings_a11y().unwrap();
        let page = crate::ui::plugins::BUILTIN
            .iter()
            .find(|i| format!("page:{}", i.id) == other)
            .unwrap();
        assert!(
            s.children
                .iter()
                .any(|n| n.role == Role::Heading && n.name == crate::i18n::tr(page.label))
        );
        assert_eq!(
            s.children[0].name,
            crate::i18n::tr("Rechercher un réglage…")
        );
        // Un onglet inconnu : rien ne change.
        let before = app.subtab(app.panel.as_ref().unwrap().tab);
        let _ = app.a11y_action("/settings/tabs/tab:inexistant");
        assert_eq!(app.subtab(app.panel.as_ref().unwrap().tab), before);
        // Recherche : seules les pages qui ont une correspondance, la requête
        // lue dans le champ.
        app.panel.as_mut().unwrap().search = "fond d'écran".into();
        let s = app.settings_a11y().unwrap();
        assert_eq!(s.children[0].description, "fond d'écran");
        assert!(s.children[1].children.len() < pages.children.len());
        // Tiroir fermé : plus d'arbre, plus d'action.
        let _ = app.close_panel();
        let tab = app.panel.as_ref().map(|p| p.tab);
        let _ = app.a11y_action(&format!("/settings/pages/{}", pages.children[0].key));
        assert_eq!(app.panel.as_ref().map(|p| p.tab), tab);
    }

    /// Primitives de page : un curseur prend le libellé de sa `field` (lu
    /// une fois), le libellé d'une rangée d'interrupteurs les précède, un
    /// élément jeté sort de l'arbre, une section avancée repliée ne montre
    /// que son triangle.
    #[test]
    fn primitives_de_page() {
        use crate::a11y::model::{Role, State};
        use crate::ui::Msg;
        use crate::ui::panel::{SliderLook, SliderSpec};
        let (app, _) = App::boot_with(PathBuf::from("/tmp/hyprdesk-a11y-tests/config.lua"), true);
        let spec = || SliderSpec::new("essai", (0.0, 10.0), 1.0, |_| Msg::Noop, Msg::Noop);
        let look = || SliderLook::fixed(13.0, 200, iced::Alignment::Center);
        let nodes = app.a11y_collected(|| {
            let jete = app.toggle(true).label("Jeté");
            drop(jete);
            iced::widget::column![
                app.section("Titre"),
                app.field(
                    "Vitesse",
                    "",
                    app.slider(&spec(), 3.0, |v| format!("{v}"), look())
                ),
                app.field(
                    "Boutons",
                    "",
                    iced::widget::column![
                        app.toggle(true).label("A").on_toggle(|_| Msg::Noop),
                        app.toggle(false).label("B"),
                    ]
                ),
                app.field("Seul", "", app.toggle(true).on_toggle(|_| Msg::Noop)),
                app.field("Texte", "", iced::widget::text("x")),
            ]
            .into()
        });
        let seen: Vec<(Role, &str)> = nodes.iter().map(|n| (n.role, n.name.as_str())).collect();
        // Le titre de section clôt sans nœud : une page le construit parfois
        // après son contenu.
        assert_eq!(
            seen,
            [
                (Role::Slider, crate::i18n::tr("Vitesse")),
                (Role::Label, crate::i18n::tr("Boutons")),
                (Role::CheckBox, "A"),
                (Role::CheckBox, "B"),
                (Role::CheckBox, crate::i18n::tr("Seul")),
                (Role::Label, crate::i18n::tr("Texte")),
            ]
        );
        assert_eq!(nodes[0].description, "3");
        assert!(
            nodes[2].states.contains(State::Checked) && nodes[2].states.contains(State::Enabled)
        );
        assert!(
            !nodes[3].states.contains(State::Checked) && !nodes[3].states.contains(State::Enabled)
        );
        // Hors d'un dessin pour l'arbre : rien n'est collecté.
        let _ = app.section("Titre");
        assert!(app.a11y_collect.borrow().is_none());
    }

    /// Contenu des pages des Réglages : chaque page et chaque onglet,
    /// configuration avancée coupée puis mise ; curseurs et interrupteurs
    /// nommés, libellé absorbé par sa commande, sections avancées repliées
    /// sans contenu, rien en mode liste de la recherche.
    #[test]
    fn reglages_contenu() {
        use crate::a11y::model::{Node, Role, State};
        use crate::ui::Msg;
        use crate::ui::panel::PanelMsg;
        let (mut app, _) =
            App::boot_with(PathBuf::from("/tmp/hyprdesk-a11y-tests/config.lua"), true);
        app.surfaces.insert(Id::unique(), "A".into());
        let _ = app.open_panel();
        let content = |app: &App| -> Vec<Node> {
            app.settings_a11y()
                .unwrap()
                .children
                .into_iter()
                .find(|n| n.key == "content")
                .map(|n| n.children)
                .unwrap_or_default()
        };
        let mut unnamed = Vec::new();
        let mut leaks = Vec::new();
        let mut sliders = 0;
        let mut checks = 0;
        for advanced in [false, true] {
            std::sync::Arc::make_mut(&mut app.cfg)
                .general
                .advanced_settings = advanced;
            for page in crate::ui::plugins::BUILTIN {
                let _ = app.update(Msg::Panel(PanelMsg::Tab(page.id)));
                let mut tabs: Vec<Option<&'static str>> =
                    crate::ui::plugins::visible_tabs(page.id, advanced)
                        .into_iter()
                        .map(|(t, _)| Some(t))
                        .collect();
                if tabs.is_empty() {
                    tabs.push(None);
                }
                let mut seen: Vec<(Option<&str>, Vec<String>)> = Vec::new();
                for t in tabs {
                    if let Some(t) = t {
                        let _ = app.update(Msg::Panel(PanelMsg::SubTab(page.id, t)));
                    }
                    // Juste après le changement d'onglet : la page affichée.
                    assert_eq!(app.panel.as_ref().unwrap().tab, page.id);
                    let nodes = content(&app);
                    let here = format!("{} / {t:?}", page.id);
                    // Avant le tri des commandes sans nom : toutes nommées
                    // sur les pages de la configuration d'essai.
                    let raw = app.a11y_collected(|| {
                        crate::ui::plugins::view(&app, app.panel.as_ref().unwrap(), page.id)
                    });
                    for n in raw.iter().filter(|n| {
                        matches!(n.role, Role::Slider | Role::CheckBox) && n.name.is_empty()
                    }) {
                        unnamed.push(format!("{here} : {}", n.dump().trim()));
                    }
                    for n in &nodes {
                        assert!(!n.name.is_empty(), "{here} : nœud sans nom montré");
                        assert!(!n.key.is_empty());
                        sliders += usize::from(n.role == Role::Slider);
                        checks += usize::from(n.role == Role::CheckBox);
                        if n.role == Role::ToggleButton && n.key.starts_with("adv:") {
                            assert!(
                                advanced,
                                "{here} : section avancée hors configuration avancée"
                            );
                            assert!(
                                !n.states.contains(State::Expanded),
                                "{here} : repliée par défaut"
                            );
                        }
                    }
                    // Une page qui choisit son onglet par `match` : aucune
                    // commande d'un autre onglet (construite puis jetée)
                    // n'entre dans celui-ci (les titres peuvent se répéter).
                    let keys: Vec<String> = nodes
                        .iter()
                        .filter(|n| matches!(n.role, Role::Slider | Role::CheckBox))
                        .map(|n| n.key.clone())
                        .collect();
                    for (other, ks) in &seen {
                        for k in keys.iter().filter(|k| ks.contains(k)) {
                            leaks.push(format!("{here} et {other:?} : {k}"));
                        }
                    }
                    seen.push((t, keys));
                    // Un libellé absorbé par sa commande n'est pas lu deux fois.
                    for w in nodes.windows(2) {
                        assert!(
                            !(w[0].role == Role::Label && w[0].name == w[1].name),
                            "{here} : {} lu deux fois",
                            w[0].name
                        );
                    }
                }
            }
        }
        assert!(leaks.is_empty(), "onglets mêlés :\n{}", leaks.join("\n"));
        assert!(
            unnamed.is_empty(),
            "commandes sans nom :\n{}",
            unnamed.join("\n")
        );
        assert!(
            sliders > 0 && checks > 0,
            "{sliders} curseurs, {checks} interrupteurs"
        );
        // Saisie : la vitesse du pointeur est un curseur nommé, sa valeur en
        // description, sans libellé à part.
        std::sync::Arc::make_mut(&mut app.cfg)
            .general
            .advanced_settings = false;
        let _ = app.update(Msg::Panel(PanelMsg::Tab("input")));
        for (t, _) in crate::ui::plugins::visible_tabs("input", false) {
            let _ = app.update(Msg::Panel(PanelMsg::SubTab("input", t)));
            for n in content(&app).iter().filter(|n| n.role == Role::Slider) {
                assert!(!n.description.is_empty(), "{}", n.dump());
                assert!(
                    !content(&app)
                        .iter()
                        .any(|l| l.role == Role::Label && l.name == n.name),
                    "{}",
                    n.name
                );
            }
        }
        // Une section avancée dépliée : son contenu suit son triangle.
        std::sync::Arc::make_mut(&mut app.cfg)
            .general
            .advanced_settings = true;
        let mut unfolded = false;
        'pages: for page in crate::ui::plugins::BUILTIN {
            let _ = app.update(Msg::Panel(PanelMsg::Tab(page.id)));
            let nodes = content(&app);
            if let Some(k) = nodes.iter().position(|n| n.key.starts_with("adv:")) {
                let before = nodes.len();
                let _ = app.update(Msg::Panel(PanelMsg::AdvToggle(format!(
                    "{}\u{1}{}\u{1}{}",
                    page.id,
                    app.subtab(page.id),
                    &nodes[k].key["adv:".len()..]
                ))));
                let after = content(&app);
                let head = after.iter().position(|n| n.key == nodes[k].key).unwrap();
                assert!(after[head].states.contains(State::Expanded));
                if after.len() > before {
                    unfolded = true;
                    break 'pages;
                }
            }
        }
        assert!(unfolded, "une section avancée dépliée montre son contenu");
        // Résultats de la recherche en liste : pas de page, pas de contenu.
        app.panel.as_mut().unwrap().search = "fond".into();
        app.panel.as_mut().unwrap().search_list = true;
        assert!(content(&app).is_empty());
        // `hyprdesk a11y dump` : l'arbre des surfaces porte le contenu.
        app.panel.as_mut().unwrap().search.clear();
        app.panel.as_mut().unwrap().search_list = false;
        let dump: String = app.a11y_surfaces().iter().map(Node::dump).collect();
        assert!(
            dump.contains("Contenu") || dump.contains("Content"),
            "{dump}"
        );
    }

    /// Menus : commandes, case cochée, entrée grisée, sous-menu déplié.
    #[test]
    fn entrees_de_menu() {
        use crate::a11y::model::{Node, Role, State};
        use crate::config::MenuEntry;
        let item = |label: &str, enabled: bool, checked: Option<bool>| MenuEntry::Item {
            label: label.into(),
            detail: None,
            icon: None,
            checked,
            enabled,
            action: None,
            confirm: None,
        };
        let entries = [
            MenuEntry::Header("Son".into()),
            item("Muet", true, Some(true)),
            MenuEntry::Separator,
            item("Sortie", false, None),
            MenuEntry::Submenu {
                label: "Périphérique".into(),
                icon: None,
                enabled: true,
                children: vec![item("Casque", true, None)],
            },
        ];
        let menu = Node::new(Role::Menu, "Volume").children(super::menu_items(
            &entries,
            Some(4),
            None,
            None,
            false,
        ));
        assert_eq!(
            menu.dump(),
            "menu « Volume »\n  label « Son »\n  menu item « Muet »\n  menu item « Sortie »\n  menu « Périphérique »\n    menu item « Casque »\n"
        );
        assert!(menu.children[1].states.contains(State::Checked));
        assert!(!menu.children[2].states.contains(State::Enabled), "grisée");
        assert!(menu.children[3].states.contains(State::Expanded));
        assert!(
            !menu.children[1].states.contains(State::Focused),
            "rien de choisi au clavier"
        );
        // Choisie au clavier : au focus.
        let keyed = super::menu_items(&entries, Some(4), Some(1), None, false);
        assert!(keyed[1].states.contains(State::Focused));
        // Descendu dans la colonne : le focus passe de la ligne à l'entrée.
        let column = super::menu_items(&entries, Some(4), Some(4), Some((4, 0)), false);
        assert!(!column[3].states.contains(State::Focused));
        assert!(column[3].children[0].states.contains(State::Focused));
        // Colonne d'un autre sous-menu, refermé : la ligne garde le focus.
        let closed = super::menu_items(&entries, None, Some(4), Some((4, 0)), false);
        assert!(closed[3].states.contains(State::Focused));
        assert!(closed[3].children.is_empty());
    }
}
