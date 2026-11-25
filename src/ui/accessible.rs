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
        out
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
