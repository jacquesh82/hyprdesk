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
                // Écran vertical : la composition portrait, comme à l'écran.
                let zones = if self.bar_vertical(monitor) {
                    def.portrait.as_ref().unwrap_or(&def.zones)
                } else {
                    &def.zones
                };
                out.push(
                    Node::new(Role::Frame, tr("Barre des menus")).children(
                        zones
                            .iter()
                            .flatten()
                            .map(|item| button(self.bar_item_label(item))),
                    ),
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
            out.push(
                Node::new(Role::Frame, tr("Dock")).children(
                    self.dock_slots(m)
                        .into_iter()
                        .filter_map(|slot| match slot {
                            Slot::Item(i) => Some(button(self.dock_item_name(&i.popup))),
                            Slot::Label(ws) => Some(Node::new(Role::Label, ws)),
                            Slot::Separator => None,
                        }),
                ),
            );
        }
        out
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
