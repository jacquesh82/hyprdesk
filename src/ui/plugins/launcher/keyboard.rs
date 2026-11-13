//! Dock au clavier : il prend le clavier quand on l'ouvre au clavier
//! (`launcher-only`, ou Tab depuis le plein bureau). Flèches, Tab et
//! Maj + Tab passent d'une case à l'autre, Début et Fin vont aux bouts,
//! Entrée ou Espace activent la case, Échap rend le clavier.
use iced::Task;
use iced::keyboard::{Key, key::Named};
use iced::window::Id;
use iced_layershell::reexport::KeyboardInteractivity;

use super::{App, Slot, UiMsg};
use crate::ui::DesktopMode;

/// Déplacement demandé au clavier dans le dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Next,
    Previous,
    First,
    Last,
}

/// Case suivante parmi les boutons (`buttons` : rangs triés des cases
/// activables), en bouclant aux bouts. Fonction pure, testée.
pub(super) fn step(buttons: &[usize], current: Option<usize>, step: Step) -> Option<usize> {
    let (first, last) = (*buttons.first()?, *buttons.last()?);
    let Some(cur) = current else {
        return Some(match step {
            Step::Previous | Step::Last => last,
            Step::Next | Step::First => first,
        });
    };
    Some(match step {
        Step::First => first,
        Step::Last => last,
        Step::Next => buttons.iter().copied().find(|&i| i > cur).unwrap_or(first),
        Step::Previous => buttons
            .iter()
            .rev()
            .copied()
            .find(|&i| i < cur)
            .unwrap_or(last),
    })
}

/// Rangs des cases activables (ni titres de workspace, ni séparateur).
fn buttons(slots: &[Slot]) -> Vec<usize> {
    slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| matches!(s, Slot::Item(_)).then_some(i))
        .collect()
}

impl App {
    fn launcher_surface_of(&self, monitor: &str) -> Option<Id> {
        self.launcher_surfaces
            .iter()
            .find(|(_, s)| s.monitor == monitor)
            .map(|(id, _)| *id)
    }

    /// Montre la case choisie : vague et bulle, comme au survol.
    fn launcher_select(&mut self, monitor: &str, index: usize, slots: &[Slot]) -> Task<UiMsg> {
        self.launcher_runtime.focus = Some((monitor.to_string(), index));
        match slots.get(index) {
            Some(Slot::Item(item)) => self.launcher_hover(Some((
                monitor.to_string(),
                item.key.clone(),
                item.popup.clone(),
                index,
            ))),
            _ => Task::none(),
        }
    }

    /// Le dock de l'écran actif prend le clavier, première case choisie. Sa
    /// surface, si elle n'existe pas encore, naîtra avec le clavier.
    pub(in crate::ui) fn launcher_grab_focused(&mut self) -> Task<UiMsg> {
        match self.monitors.iter().find(|m| m.focused) {
            Some(m) => {
                let name = m.name.clone();
                self.launcher_grab(&name)
            }
            None => Task::none(),
        }
    }

    pub(in crate::ui) fn launcher_grab(&mut self, monitor: &str) -> Task<UiMsg> {
        if !self.cfg.plugin_on("launcher") {
            return Task::none();
        }
        let Some(m) = self.monitors.iter().find(|m| m.name == monitor).cloned() else {
            return Task::none();
        };
        let slots = self.dock_slots(&m);
        let Some(first) = step(&buttons(&slots), None, Step::First) else {
            return Task::none();
        };
        let mut tasks = vec![self.launcher_select(monitor, first, &slots)];
        if let Some(id) = self.launcher_surface_of(monitor) {
            tasks.push(Task::done(UiMsg::KeyboardInteractivityChange {
                id,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
            }));
        }
        Task::batch(tasks)
    }

    /// Le dock rend le clavier (aux fenêtres, ou au bureau dessous).
    pub(in crate::ui) fn launcher_release(&mut self) -> Task<UiMsg> {
        let Some((monitor, _)) = self.launcher_runtime.focus.take() else {
            return Task::none();
        };
        let mut tasks = vec![
            self.launcher_hover(None),
            self.launcher_pointer(&monitor, None),
        ];
        if let Some(id) = self.launcher_surface_of(&monitor) {
            tasks.push(Task::done(UiMsg::KeyboardInteractivityChange {
                id,
                keyboard_interactivity: KeyboardInteractivity::None,
            }));
        }
        Task::batch(tasks)
    }

    /// Case activée (clic ou Entrée) : le clavier revient d'abord à ce que
    /// la case ouvre ou désigne.
    pub(in crate::ui) fn launcher_activate(&mut self, msg: UiMsg) -> Task<UiMsg> {
        let release = self.launcher_release();
        let run = self.update_inner(msg);
        Task::batch([release, run])
    }

    /// Touche reçue par une surface du dock.
    pub(in crate::ui) fn launcher_key(&mut self, id: Id, key: &Key) -> Task<UiMsg> {
        let Some(monitor) = self.launcher_surfaces.get(&id).map(|s| s.monitor.clone()) else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == monitor).cloned() else {
            return Task::none();
        };
        let slots = self.dock_slots(&m);
        let current = self
            .launcher_runtime
            .focus
            .as_ref()
            .filter(|(name, _)| name == &monitor)
            .map(|(_, i)| *i)
            .filter(|i| matches!(slots.get(*i), Some(Slot::Item(_))));
        let wanted = match key {
            Key::Named(Named::Escape) => {
                let release = self.launcher_release();
                // Ouvert seul (launcher-only) : Échap le referme, comme son
                // raccourci. Sur le plein bureau, le clavier revient au bureau.
                return if self.mode() == DesktopMode::LauncherOnly {
                    Task::batch([release, self.hide()])
                } else {
                    release
                };
            }
            Key::Named(Named::Enter | Named::Space) => {
                return match current.and_then(|i| slots.get(i)) {
                    Some(Slot::Item(item)) => self.launcher_activate(item.msg.clone()),
                    _ => self.launcher_select_step(&monitor, &slots, current, Step::First),
                };
            }
            Key::Named(Named::Tab) if self.mods.shift() => Step::Previous,
            Key::Named(Named::Tab | Named::ArrowRight | Named::ArrowDown) => Step::Next,
            Key::Named(Named::ArrowLeft | Named::ArrowUp) => Step::Previous,
            Key::Named(Named::Home) => Step::First,
            Key::Named(Named::End) => Step::Last,
            _ => return Task::none(),
        };
        self.launcher_select_step(&monitor, &slots, current, wanted)
    }

    fn launcher_select_step(
        &mut self,
        monitor: &str,
        slots: &[Slot],
        current: Option<usize>,
        wanted: Step,
    ) -> Task<UiMsg> {
        match step(&buttons(slots), current, wanted) {
            Some(i) => self.launcher_select(monitor, i, slots),
            None => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypr::{Client, Monitor};
    use crate::ui::Msg;

    #[test]
    fn steps_skip_labels_and_wrap_at_both_ends() {
        // Rangs 0 et 3 : titres de workspace ; 5 : séparateur.
        let b = [1, 2, 4, 6, 7];
        assert_eq!(step(&b, None, Step::Next), Some(1));
        assert_eq!(step(&b, None, Step::Previous), Some(7));
        assert_eq!(step(&b, Some(2), Step::Next), Some(4));
        assert_eq!(step(&b, Some(4), Step::Previous), Some(2));
        assert_eq!(step(&b, Some(7), Step::Next), Some(1));
        assert_eq!(step(&b, Some(1), Step::Previous), Some(7));
        assert_eq!(step(&b, Some(4), Step::First), Some(1));
        assert_eq!(step(&b, Some(4), Step::Last), Some(7));
        assert_eq!(step(&[], None, Step::Next), None);
    }

    fn app() -> App {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-launcher-keys/config.lua"),
            true,
        )
        .0;
        app.monitors = vec![Monitor {
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
        std::sync::Arc::make_mut(&mut app.cfg).general.running_apps =
            crate::config::RunningApps::All;
        let a = Id::unique();
        app.surfaces.insert(a, "A".into());
        app.sizes.insert(a, iced::Size::new(1440.0, 900.0));
        for (address, class) in [("0x1", "foot"), ("0x2", "firefox")] {
            app.clients.push(Client {
                address: address.into(),
                class: class.into(),
                title: class.into(),
                workspace: "1".into(),
            });
        }
        app
    }

    fn dock(app: &App) -> Id {
        *app.launcher_surfaces
            .keys()
            .next()
            .expect("surface du dock")
    }

    fn selected(app: &App) -> Option<String> {
        let (monitor, i) = app.launcher_runtime.focus.clone()?;
        let m = app.monitors.iter().find(|m| m.name == monitor)?.clone();
        match app.dock_slots(&m).into_iter().nth(i)? {
            Slot::Item(item) => Some(item.key),
            _ => None,
        }
    }

    #[test]
    fn the_launcher_shortcut_hands_the_keyboard_to_the_dock() {
        let mut app = app();
        let _ = app.update_inner(Msg::Control(crate::control::Command::ToggleLauncher));
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
        let id = dock(&app);
        assert_eq!(selected(&app).as_deref(), Some("dock:0x1"));
        let _ = app.update_inner(Msg::Key(id, Key::Named(Named::ArrowRight)));
        assert_eq!(selected(&app).as_deref(), Some("dock:0x2"));
        let _ = app.update_inner(Msg::Key(id, Key::Named(Named::End)));
        let last = selected(&app).unwrap();
        assert!(last.starts_with("dock-btn:"), "{last}");
        // Au bout, la sélection boucle.
        let _ = app.update_inner(Msg::Key(id, Key::Named(Named::Tab)));
        assert_eq!(selected(&app).as_deref(), Some("dock:0x1"));
        let _ = app.update_inner(Msg::Key(id, Key::Named(Named::ArrowLeft)));
        assert_eq!(selected(&app), Some(last));
        // Entrée active la case et rend le clavier ; le mode ne change pas.
        let _ = app.update_inner(Msg::Key(id, Key::Named(Named::Home)));
        let _ = app.update_inner(Msg::Key(id, Key::Named(Named::Enter)));
        assert!(app.launcher_runtime.focus.is_none());
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
    }

    #[test]
    fn escape_closes_a_dock_opened_alone_and_returns_to_the_full_desktop() {
        let mut app = app();
        let _ = app.update_inner(Msg::Control(crate::control::Command::LauncherOnly));
        let id = dock(&app);
        let _ = app.update_inner(Msg::Key(id, Key::Named(Named::Escape)));
        assert!(app.launcher_runtime.focus.is_none());
        assert_eq!(app.mode(), DesktopMode::NoDesktop);

        // Plein bureau : Tab descend dans le dock, Échap remonte au bureau.
        let _ = app.update_inner(Msg::Control(crate::control::Command::Show));
        assert!(
            app.launcher_runtime.focus.is_none(),
            "le bureau garde le clavier"
        );
        let desk = *app.surfaces.keys().next().unwrap();
        let _ = app.update_inner(Msg::Key(desk, Key::Named(Named::Tab)));
        assert!(selected(&app).is_some());
        let id = dock(&app);
        let _ = app.update_inner(Msg::Key(id, Key::Named(Named::Escape)));
        assert!(app.launcher_runtime.focus.is_none());
        assert_eq!(app.mode(), DesktopMode::FullDesktop);
    }

    /// Le sélecteur de la barre (un clic) ne prend pas le clavier : on doit
    /// pouvoir écrire dans une fenêtre en mode lanceur.
    #[test]
    fn the_bar_selector_leaves_the_keyboard_to_the_windows() {
        let mut app = app();
        let _ = app.update_inner(Msg::On(
            "A".into(),
            Box::new(Msg::Control(crate::control::Command::LauncherOnly)),
        ));
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
        assert!(app.launcher_runtime.focus.is_none());
    }

    /// Dock ouvert au clavier puis plein bureau (ou no-desktop) : le dock
    /// rend le clavier, sinon aucune fenêtre ne le reçoit plus.
    #[test]
    fn leaving_launcher_mode_gives_the_keyboard_back() {
        for (to, mode) in [
            (crate::control::Command::Show, DesktopMode::FullDesktop),
            (crate::control::Command::Hide, DesktopMode::NoDesktop),
        ] {
            let mut app = app();
            let _ = app.update_inner(Msg::Control(crate::control::Command::LauncherOnly));
            assert!(app.launcher_runtime.focus.is_some());
            let _ = app.update_inner(Msg::Control(to));
            assert_eq!(app.mode(), mode);
            assert!(app.launcher_runtime.focus.is_none(), "{mode:?}");
        }
    }

    #[test]
    fn a_click_also_gives_the_keyboard_back() {
        let mut app = app();
        let _ = app.update_inner(Msg::Control(crate::control::Command::LauncherOnly));
        assert!(app.launcher_runtime.focus.is_some());
        let _ = app.update_inner(Msg::LauncherActivate(Box::new(Msg::FocusWindow(
            "0x2".into(),
        ))));
        assert!(app.launcher_runtime.focus.is_none());
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
    }
}
