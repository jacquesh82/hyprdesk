//! Arbre d'accessibilité des surfaces (HIG ACC-01) : calculé depuis l'état
//! de l'`App`, pas depuis l'arbre iced (sans sémantique), et nommé par les
//! libellés déjà affichés. Publié au pont seulement quand un lecteur d'écran
//! le lit (`crate::a11y::wanted`) ou que `hyprdesk a11y dump` le demande.

use crate::a11y::model::{Node, Role, State};
use crate::i18n::tr;
use crate::ui::App;
use crate::ui::plugins::dock::{PopupContent, Slot};

impl App {
    /// Une `Frame` par barre et par dock, dans l'ordre des écrans.
    pub(super) fn a11y_surfaces(&self) -> Vec<Node> {
        let mut out = Vec::new();
        if let Some(def) = self.cfg.bar.as_ref() {
            let mut bars: Vec<&str> = self
                .bar_surfaces
                .values()
                .map(|b| b.monitor.as_str())
                .collect();
            bars.sort_unstable();
            for monitor in bars {
                let frame = format!("bar:{monitor}");
                // Écran vertical : la composition portrait, comme à l'écran.
                let zones = if self.bar_vertical(monitor) {
                    def.portrait.as_ref().unwrap_or(&def.zones)
                } else {
                    &def.zones
                };
                out.push(
                    Node::new(Role::Frame, tr("Barre des menus"))
                        .key(frame)
                        .children(zones.iter().flatten().map(|item| {
                            button(self.bar_item_label(item)).key(
                                crate::ui::plugins::bar::surface::bar_item_key(item).to_string(),
                            )
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
            // Dock au clavier : la surface active, la case choisie au focus
            // (le rang compte toutes les cases, comme la sélection).
            let focus = self
                .launcher_runtime
                .focus
                .as_ref()
                .filter(|(name, _)| name == monitor)
                .map(|(_, i)| *i);
            let mut frame = Node::new(Role::Frame, tr("Dock")).key(format!("dock:{monitor}"));
            if focus.is_some() {
                frame = frame.state(State::Active);
            }
            out.push(
                frame.children(self.dock_slots(m).into_iter().enumerate().filter_map(
                    |(k, slot)| match slot {
                        Slot::Item(i) => {
                            let b = button(self.dock_item_name(&i.popup)).key(i.key.clone());
                            Some(if focus == Some(k) {
                                b.state(State::Focused)
                            } else {
                                b
                            })
                        }
                        Slot::Label(ws) => {
                            Some(Node::new(Role::Label, ws.clone()).key(format!("ws:{ws}")))
                        }
                        Slot::Separator => None,
                    },
                )),
            );
        }
        if let Some(l) = self.launcher_a11y() {
            out.push(l);
        }
        out
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

    /// Après chaque mise à jour : l'arbre va au pont s'il est lu.
    pub(super) fn a11y_publish(&self) {
        if crate::a11y::wanted() {
            crate::a11y::publish(self.a11y_surfaces());
        }
    }
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
            "frame « Barre des menus »\n  push button « Volume »\n"
        );
        assert_eq!(
            tree.dump(),
            "application « hyprdesk »
  frame « Barre des menus »
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
}
