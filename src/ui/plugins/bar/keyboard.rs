//! Barre au clavier (HIG ENT-02) : le raccourci `keys.focus_bar` donne le
//! clavier à la barre de l'écran actif. Les modules qu'ouvre un clic gauche
//! portent un anneau de focus (l'enveloppe du centre de contrôle) ; flèches,
//! Tab et Maj + Tab passent de l'un à l'autre, Entrée ou Espace l'ouvrent,
//! Échap rend le clavier aux fenêtres. Un menu ouvert prend le clavier : la
//! barre, qui reste la sienne, le retrouve à sa fermeture.
use iced::Task;
use iced::keyboard::{Key, key::Named};
use iced::window::Id;
use iced_layershell::reexport::KeyboardInteractivity;

use super::App;
use crate::ui::Msg;

/// La barre `id` aux opérations de focus : elles y restent bornées, sans
/// aller aux autres surfaces ni aux barres des autres écrans.
pub(in crate::ui) fn scope(id: Id) -> iced::widget::Id {
    iced::widget::Id::from(format!("hyprdesk-bar-{id}"))
}

/// Le module suivant (précédent) de la barre prend le focus, le seul.
fn step(id: Id, previous: bool) -> Task<Msg> {
    crate::ui::focus::step(scope(id), previous)
}

/// Module focusable de la barre : l'anneau s'efface.
fn unfocus(id: Id) -> Task<Msg> {
    iced::advanced::widget::operate(iced::advanced::widget::operation::scope(
        scope(id),
        iced::advanced::widget::operation::focusable::unfocus(),
    ))
}

impl App {
    /// Le clavier va à la barre de l'écran actif (raccourci `keys.focus_bar`,
    /// HIG ENT-02) : son premier module activable est choisi. Pressé deux
    /// fois, la barre le rend aux fenêtres. Sans barre, rien.
    pub(in crate::ui) fn bar_focus(&mut self) -> Task<Msg> {
        if let Some(id) = self.bar_keyboard {
            return self.bar_release(id);
        }
        let Some(id) = self.focused_bar() else {
            return Task::none();
        };
        self.bar_keyboard = Some(id);
        // Au tour suivant : la barre est redessinée, ses anneaux avec, quand
        // le focus se pose.
        Task::batch([
            // Une seule surface à la fois : le dock rend le clavier.
            self.launcher_release(),
            Task::done(Msg::KeyboardInteractivityChange {
                id,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
            }),
            // Masquée hors survol : elle paraît ; son premier module est
            // choisi alors (`bar_reveal_apply`).
            Task::done(Msg::BarReveal(id, true)),
            Task::done(Msg::BarFocusFirst(id)),
        ])
    }

    /// La barre qui tient le clavier vient de paraître : son premier module
    /// activable est choisi.
    pub(in crate::ui) fn bar_focus_first(&self, id: Id) -> Task<Msg> {
        if self.bar_keyboard == Some(id) {
            step(id, false)
        } else {
            Task::none()
        }
    }

    /// Touche reçue par la barre qui tient le clavier.
    pub(in crate::ui) fn bar_key(&mut self, id: Id, key: &Key) -> Task<Msg> {
        // Barre verticale : haut et bas, au lieu de gauche et droite.
        let (next, previous) = if self
            .bar_surfaces
            .get(&id)
            .is_some_and(|s| s.position.vertical())
        {
            (Named::ArrowDown, Named::ArrowUp)
        } else {
            (Named::ArrowRight, Named::ArrowLeft)
        };
        match key {
            Key::Named(Named::Escape) => self.bar_release(id),
            Key::Named(Named::Tab) => step(id, self.mods.shift()),
            Key::Named(k) if *k == next => step(id, false),
            Key::Named(k) if *k == previous => step(id, true),
            _ => Task::none(),
        }
    }

    /// La barre rend le clavier aux fenêtres ; elle ne le tenait pas : rien.
    pub(in crate::ui) fn bar_release(&mut self, id: Id) -> Task<Msg> {
        if self.bar_keyboard != Some(id) {
            return Task::none();
        }
        self.bar_keyboard = None;
        Task::batch([
            Task::done(Msg::KeyboardInteractivityChange {
                id,
                keyboard_interactivity: KeyboardInteractivity::None,
            }),
            unfocus(id),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::plugins::bar::surface::BarSurface;

    fn app() -> (App, Id) {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-bar-keys/config.lua"),
            true,
        )
        .0;
        app.monitors = vec![crate::hypr::Monitor {
            dpms_off: false,
            name: "A".into(),
            width: 900,
            height: 640,
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            workspace: "1".into(),
            base: String::new(),
            focused: true,
            reserved: (0.0, 44.0, 0.0, 0.0),
        }];
        let bar = Id::unique();
        app.bar_surfaces.insert(
            bar,
            BarSurface {
                monitor: "A".into(),
                position: crate::config::BarPosition::Top,
                height: 44,
                margin: 0,
                ..Default::default()
            },
        );
        (app, bar)
    }

    /// HIG PAT-03 : par-dessus une fenêtre plein écran, la barre se retire
    /// au bord (bande de rappel) ; le plein écran parti, elle revient.
    #[test]
    fn the_bar_stays_reachable_over_a_fullscreen_window() {
        let (mut app, _) = app();
        app.bar_surfaces.clear();
        std::sync::Arc::make_mut(&mut app.cfg).bar = Some(crate::config::BarDef {
            shape: Default::default(),
            monitors: None,
            position: crate::config::BarPosition::Top,
            height: 40,
            margin: 0,
            zones: Default::default(),
            portrait: None,
            compact: crate::config::BarCompact::Auto,
            style: crate::config::BarStyle::Continuite,
            colors: Default::default(),
            convex: Default::default(),
        });
        let _ = app.sync_bars();
        let bar = |app: &App| {
            assert_eq!(app.bar_surfaces.len(), 1);
            *app.bar_surfaces.keys().next().unwrap()
        };
        let before = bar(&app);
        let shape = app.bar_surfaces[&before].shape;
        let autohide = shape.visibility == crate::config::BarVisibility::AutoHide;
        assert!(!app.bar_over_fullscreen.contains(&before));
        assert_eq!(app.bar_hides(before, &shape), autohide, "hors plein écran");
        let window = |fullscreen| crate::hypr::WindowGeo {
            address: "0x1".into(),
            class: "foot".into(),
            title: String::new(),
            workspace: "1".into(),
            floating: false,
            rect: (0, 0, 900, 640),
            fullscreen,
        };
        let clients = app.clients.clone();
        let _ = app.update_inner(Msg::Hypr(crate::hypr::Update::Clients(
            clients.clone(),
            vec![window(true)],
        )));
        let over = bar(&app);
        assert_ne!(over, before, "surface recréée sur le calque Overlay");
        assert!(app.bar_over_fullscreen.contains(&over));
        assert!(app.bar_hides(over, &shape));
        assert!(!app.bar_revealed(over), "réduite d'emblée à sa bande");
        // Même état relu : rien ne bouge.
        let _ = app.update_inner(Msg::Hypr(crate::hypr::Update::Clients(
            clients.clone(),
            vec![window(true)],
        )));
        assert_eq!(bar(&app), over, "pas de recréation sans changement");
        let _ = app.update_inner(Msg::Hypr(crate::hypr::Update::Clients(
            clients,
            vec![window(false)],
        )));
        let back = bar(&app);
        assert!(
            !app.bar_over_fullscreen.contains(&back),
            "revenue à sa place"
        );
        assert_eq!(app.bar_hides(back, &shape), autohide);
        assert!(app.bar_over_fullscreen.is_empty());
    }

    /// HIG ENT-02 : le raccourci de la barre lui donne le clavier sur l'écran
    /// actif ; pressé deux fois, ou par Échap, elle le rend.
    #[test]
    fn the_bar_shortcut_hands_the_keyboard_to_the_bar_and_takes_it_back() {
        let focus = || Msg::Control(crate::control::Command::Focus("bar".into()));
        {
            let (mut app, bar) = app();
            assert!(!app.listens_to_events(), "la barre n'écoute pas le clavier");
            let _ = app.update_inner(focus());
            assert_eq!(app.bar_keyboard, Some(bar));
            assert!(app.listens_to_events());
            // Son menu prend le clavier : la barre le garde, elle le retrouve à
            // la fermeture du menu.
            let _ = app.open_popover(
                bar,
                "volume".into(),
                crate::ui::plugins::bar::popover::Kind::Volume,
            );
            let _ = app.update_inner(Msg::KeyboardLeft(bar));
            assert_eq!(
                app.bar_keyboard,
                Some(bar),
                "menu ouvert : la barre le garde"
            );
            // Pressé deux fois : rendu (le menu reste au clavier, lui).
            let _ = app.update_inner(focus());
            assert_eq!(app.bar_keyboard, None);
        }
        // Sans barre : rien.
        {
            let (mut app, _) = app();
            app.bar_surfaces.clear();
            let _ = app.update_inner(focus());
            assert_eq!(app.bar_keyboard, None);
        }
        // Une fenêtre reprend le clavier : la barre le rend. Puis Échap, dans
        // la barre, le rend aussi.
        {
            let (mut app, bar) = app();
            let _ = app.update_inner(focus());
            let _ = app.update_inner(Msg::KeyboardLeft(bar));
            assert_eq!(app.bar_keyboard, None, "sans menu ouvert : rendu");
            let _ = app.update_inner(focus());
            // Tab et les flèches circulent dans la barre sans rendre le clavier.
            for key in [Named::Tab, Named::ArrowRight, Named::ArrowLeft] {
                let _ = app.update_inner(Msg::Key(bar, Key::Named(key)));
                assert_eq!(app.bar_keyboard, Some(bar), "{key:?}");
            }
            let _ = app.update_inner(Msg::Key(bar, Key::Named(Named::Escape)));
            assert_eq!(app.bar_keyboard, None);
            assert!(!app.listens_to_events(), "rendu : la barre n'écoute plus");
        }
    }
}
