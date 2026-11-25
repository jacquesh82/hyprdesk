//! État transitoire du dock : géométrie rendue, transitions et lancements.
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use iced::{Point, Rectangle, Size, Task, Vector};

use super::surface::{PopupContent, thickness};
use super::{App, UiMsg};
use crate::config::{LauncherPosition, Transition};
use crate::hypr::Monitor;
use crate::ui::Fade;

pub(in crate::ui) const OPEN_DELAY: Duration = Duration::from_millis(180);
pub(in crate::ui) const LAUNCH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub(in crate::ui) struct Runtime {
    pub docks: HashMap<String, Fade>,
    pub anchors: HashMap<(String, String), Rectangle>,
    pub pending: Option<Pending>,
    pub launches: HashMap<String, Launch>,
    pub backdrops: HashMap<crate::ui::WallKey, super::contrast::Backdrop>,
    /// Dock qui tient le clavier : écran et rang de la case choisie.
    pub focus: Option<(String, usize)>,
    /// Le dock tient le clavier « à la demande » (fenêtres en place) : il le
    /// rend dès que la souris ou un clic désigne une fenêtre.
    pub on_demand: bool,
}

#[derive(Clone, Debug)]
pub(in crate::ui) struct Pending {
    pub monitor: String,
    pub key: String,
    pub content: PopupContent,
    pub token: u64,
}

#[derive(Clone, Debug)]
pub(in crate::ui) struct Launch {
    pub started: Instant,
    /// Adresses des fenêtres déjà ouvertes au lancement, toutes classes :
    /// toute autre est arrivée depuis.
    pub known: HashSet<String>,
}

/// Fin d'un lancement du dock ou de la grille sans que la fenêtre de
/// l'application arrive (HIG PAT-08, STA-01).
#[derive(Debug, Clone, PartialEq)]
pub(in crate::ui) enum LaunchEnd {
    /// L'entrée ne donne aucune commande : application introuvable.
    Unresolved,
    /// Hyprland a refusé la commande : la cause.
    Refused(String),
    /// Le délai est passé sans qu'aucune fenêtre arrive.
    NoWindow,
}

/// Annonce d'une fin de lancement : icône, durée (ms) et texte. L'échec est
/// un avertissement, le retard un indice : rien n'a échoué.
pub(in crate::ui) fn launch_end_notice_text(name: &str, end: &LaunchEnd) -> (u8, u32, String) {
    match end {
        LaunchEnd::Unresolved => (
            0,
            8000,
            crate::tr!(
                "Lancement de {name} impossible : application introuvable.",
                name = name
            ),
        ),
        LaunchEnd::Refused(e) => (
            0,
            8000,
            crate::tr!("Lancement de {name} impossible : {e}", name = name, e = e),
        ),
        LaunchEnd::NoWindow => (
            2,
            6000,
            crate::tr!(
                "{name} met du temps à s'ouvrir. Si rien ne se lance, cliquez de nouveau sur son icône.",
                name = name
            ),
        ),
    }
}

/// Notification de Hyprland qui porte l'annonce : hors de l'interface, elle
/// n'interrompt rien et ne prend pas le clavier.
fn launch_end_notice(name: &str, end: &LaunchEnd) -> Task<UiMsg> {
    let (icon, ms, msg) = launch_end_notice_text(name, end);
    Task::perform(
        async move {
            let _ = tokio::task::spawn_blocking(move || crate::hypr::notify(icon, ms, &msg)).await;
        },
        |_| UiMsg::Noop,
    )
}

/// Décalage vers le bord, identique à l'entrée et à la sortie.
pub(super) fn edge_offset(position: LauncherPosition, distance: f32) -> Vector {
    match position {
        LauncherPosition::Bottom => Vector::new(0.0, distance),
        LauncherPosition::Top => Vector::new(0.0, -distance),
        LauncherPosition::Left => Vector::new(-distance, 0.0),
        LauncherPosition::Right => Vector::new(distance, 0.0),
    }
}

/// Boîte du bouton, dans les coordonnées logiques de l'écran. Les zones
/// exclusives comprennent le dock lui-même, ainsi que les autres barres.
pub(in crate::ui) fn screen_anchor(
    m: &Monitor,
    position: LauncherPosition,
    size: u16,
    local: Rectangle,
) -> Rectangle {
    let th = thickness(size) as f32;
    let (left, top, right, bottom) = m.reserved;
    let origin = match position {
        LauncherPosition::Bottom => Vector::new(left, m.height as f32 - bottom.max(th)),
        LauncherPosition::Top => Vector::new(left, (top - th).max(0.0)),
        LauncherPosition::Left => Vector::new((left - th).max(0.0), top),
        LauncherPosition::Right => Vector::new(m.width as f32 - right.max(th), top),
    };
    local + origin
}

impl App {
    /// Coin de la surface d'un menu posé contre le bouton `key` du dock
    /// (MLG-307), comme sa bulle : `size` comprend `shadow` px d'ombre autour
    /// de la carte, qui s'arrête à 6 px du bouton.
    pub(in crate::ui) fn dock_menu_origin(
        &self,
        monitor: &str,
        key: &str,
        size: Size,
        shadow: f32,
    ) -> Option<Point> {
        let m = self.monitors.iter().find(|m| m.name == monitor)?;
        let local = self
            .launcher_runtime
            .anchors
            .get(&(monitor.to_string(), key.to_string()))?;
        let position = self.launcher_bar_position(monitor);
        // `place_popup` laisse 8 px entre la surface et le bouton.
        let anchor =
            self.launcher_screen_anchor(m, *local) + edge_offset(position, shadow - 6.0 + 8.0);
        Some(
            place_popup(position, self.launcher_popup_area(m), anchor, size)
                .bounds
                .position(),
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::ui) struct Placement {
    pub bounds: Rectangle,
    /// Origine de l'expansion, dirigée vers le bouton même près d'un bord.
    pub pivot: Point,
}

pub(super) fn workarea(m: &Monitor, position: LauncherPosition, size: u16) -> Rectangle {
    let (mut l, mut t, mut r, mut b) = m.reserved;
    let th = thickness(size) as f32;
    match position {
        LauncherPosition::Bottom => b = (b - th).max(0.0),
        LauncherPosition::Top => t = (t - th).max(0.0),
        LauncherPosition::Left => l = (l - th).max(0.0),
        LauncherPosition::Right => r = (r - th).max(0.0),
    }
    Rectangle::new(
        Point::new(l, t),
        Size::new(
            (m.width as f32 - l - r).max(1.0),
            (m.height as f32 - t - b).max(1.0),
        ),
    )
}

pub(super) fn place_popup(
    position: LauncherPosition,
    screen: Rectangle,
    anchor: Rectangle,
    requested: Size,
) -> Placement {
    let size = Size::new(
        requested.width.min((screen.width - 16.0).max(1.0)),
        requested.height.min((screen.height - 16.0).max(1.0)),
    );
    let center = anchor.center();
    // 10 px de carte autour du bouton, passage de 6 px, ombre de bulle de 8 px.
    let (x, y) = match position {
        LauncherPosition::Bottom => (center.x - size.width / 2.0, anchor.y - 8.0 - size.height),
        LauncherPosition::Top => (center.x - size.width / 2.0, anchor.y + anchor.height + 8.0),
        LauncherPosition::Left => (anchor.x + anchor.width + 8.0, center.y - size.height / 2.0),
        LauncherPosition::Right => (anchor.x - 8.0 - size.width, center.y - size.height / 2.0),
    };
    let x = x
        .clamp(
            screen.x + 8.0,
            (screen.x + screen.width - size.width - 8.0).max(screen.x + 8.0),
        )
        .round();
    let y = y
        .clamp(
            screen.y + 8.0,
            (screen.y + screen.height - size.height - 8.0).max(screen.y + 8.0),
        )
        .round();
    // La transformation porte sur la carte, à l'intérieur des 8 px d'ombre.
    let px = ((center.x - x - 8.0) / (size.width - 16.0).max(1.0)).clamp(0.0, 1.0);
    let py = ((center.y - y - 8.0) / (size.height - 16.0).max(1.0)).clamp(0.0, 1.0);
    let pivot = match position {
        LauncherPosition::Bottom => Point::new(px, 1.0),
        LauncherPosition::Top => Point::new(px, 0.0),
        LauncherPosition::Left => Point::new(0.0, py),
        LauncherPosition::Right => Point::new(1.0, py),
    };
    Placement {
        bounds: Rectangle::new(Point::new(x, y), size),
        pivot,
    }
}

impl App {
    pub(in crate::ui) fn launcher_transition(&self, from: f32, to: f32, ms: u64) -> Fade {
        Fade {
            from,
            to,
            start: Instant::now(),
            dur: self.motion_duration().min(Duration::from_millis(ms)),
        }
    }

    pub(in crate::ui) fn launcher_progress(&self, monitor: &str) -> f32 {
        if self.convex_auto_hide() {
            if self.search.is_some() {
                return 0.0;
            }
            return self.convex_progress(monitor, crate::ui::convex::Part::Dock);
        }
        if self.cfg.general.transition == Transition::None {
            return 1.0;
        }
        self.launcher_runtime
            .docks
            .get(monitor)
            .map_or(1.0, |f| f.value(Instant::now()))
    }

    pub(in crate::ui) fn launcher_animating(&self) -> bool {
        let now = Instant::now();
        self.launcher_runtime.docks.values().any(|f| !f.done(now))
            || self
                .hover_fades
                .iter()
                .any(|(key, f)| key.starts_with("dock-wave:") && !f.done(now))
            || self
                .launcher_popup
                .as_ref()
                .is_some_and(|p| !p.fade.done(now))
            || (self.cfg.general.transition != Transition::None
                && !self.launcher_runtime.launches.is_empty())
    }

    pub(in crate::ui) fn launcher_frame(&mut self) -> Task<UiMsg> {
        let now = Instant::now();
        // `true` : délai passé sans qu'aucune fenêtre arrive, à annoncer.
        let finished: Vec<(String, bool)> =
            self.launcher_runtime
                .launches
                .iter()
                .filter_map(|(id, launch)| {
                    if self.clients.iter().any(|c| {
                        !launch.known.contains(&c.address) && self.app_matches(id, &c.class)
                    }) {
                        return Some((id.clone(), false));
                    }
                    (now.duration_since(launch.started) >= LAUNCH_TIMEOUT)
                        .then(|| (id.clone(), self.launch_stalled(launch)))
                })
                .collect();
        let mut notices = Vec::new();
        for (id, stalled) in finished {
            self.launcher_runtime.launches.remove(&id);
            if stalled {
                let notice = launch_end_notice(&self.launcher_app_name(&id), &LaunchEnd::NoWindow);
                notices.push(notice);
            }
        }
        if self.launcher_popup.as_ref().is_some_and(|p| {
            p.fade.to == 0.0
                && p.fade.done(now)
                && (!self.convex_auto_hide()
                    || self
                        .convex_motion(&p.monitor, crate::ui::convex::Part::Dock)
                        .done(now))
        }) {
            let p = self.launcher_popup.take().unwrap();
            self.launcher_popup_hover = false;
            notices.push(iced::window::close(p.id));
        }
        if notices.is_empty() {
            Task::none()
        } else {
            Task::batch(notices)
        }
    }

    /// Nom affiché de l'application `id` : celui de son entrée, sinon
    /// l'identifiant.
    pub(in crate::ui) fn launcher_app_name(&self, id: &str) -> String {
        self.apps
            .iter()
            .find(|a| a.id == id)
            .map_or_else(|| id.to_string(), |a| a.name.clone())
    }

    /// Aucune fenêtre, quelle que soit sa classe, n'est arrivée depuis le
    /// début du lancement.
    fn launch_stalled(&self, launch: &Launch) -> bool {
        self.clients
            .iter()
            .all(|c| launch.known.contains(&c.address))
    }

    pub(in crate::ui) fn launcher_launch_started(&mut self, id: String) -> Task<UiMsg> {
        let started = Instant::now();
        let known = self.clients.iter().map(|c| c.address.clone()).collect();
        self.launcher_runtime
            .launches
            .insert(id.clone(), Launch { started, known });
        Task::perform(
            async move {
                tokio::time::sleep(LAUNCH_TIMEOUT).await;
                (id, started)
            },
            |(id, token)| UiMsg::LauncherLaunchExpired(id, token),
        )
    }

    /// Termine le lancement `id` s'il est encore suivi sous ce jeton. Sans
    /// fenêtre (`end`), la fin est annoncée, jamais en silence (HIG PAT-08,
    /// STA-01). Au délai passé, une fenêtre arrivée depuis, d'une autre
    /// classe que celle de l'entrée, n'est pas un échec : rien à dire.
    pub(in crate::ui) fn launcher_launch_ended(
        &mut self,
        id: &str,
        token: Instant,
        end: LaunchEnd,
    ) -> Task<UiMsg> {
        let Some(launch) = self
            .launcher_runtime
            .launches
            .get(id)
            .filter(|l| l.started == token)
        else {
            return Task::none();
        };
        let announce = match &end {
            LaunchEnd::NoWindow => self.launch_stalled(launch),
            _ => true,
        };
        self.launcher_runtime.launches.remove(id);
        if announce {
            launch_end_notice(&self.launcher_app_name(id), &end)
        } else {
            Task::none()
        }
    }

    pub(in crate::ui) fn launcher_anchor(
        &mut self,
        monitor: String,
        key: String,
        bounds: Rectangle,
    ) -> Task<UiMsg> {
        self.launcher_runtime
            .anchors
            .insert((monitor.clone(), key.clone()), bounds);
        if self
            .launcher_popup
            .as_ref()
            .is_some_and(|p| p.monitor == monitor && p.key == key)
        {
            return self.launcher_fit_popup(None);
        }
        Task::none()
    }

    pub(in crate::ui) fn launcher_fit_popup(&mut self, content_height: Option<f32>) -> Task<UiMsg> {
        let Some(p) = &self.launcher_popup else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == p.monitor) else {
            return Task::none();
        };
        let Some(local) = self
            .launcher_runtime
            .anchors
            .get(&(p.monitor.clone(), p.key.clone()))
        else {
            return Task::none();
        };
        let anchor = self.launcher_screen_anchor(m, *local);
        let requested = Size::new(
            p.placement.bounds.width,
            content_height
                .filter(|h| h.is_finite() && *h > 0.0)
                .map_or(p.placement.bounds.height, |h| {
                    (h + 32.0).ceil().min(p.max_height)
                }),
        );
        let placement = place_popup(
            self.launcher_bar_position(&m.name),
            self.launcher_popup_area(m),
            anchor,
            requested,
        );
        let p = self.launcher_popup.as_mut().unwrap();
        let old = p.placement.bounds;
        p.placement = placement;
        let new = placement.bounds;
        let mut tasks = Vec::new();
        if old.size() != new.size() {
            tasks.push(Task::done(UiMsg::SizeChange {
                id: p.id,
                size: (new.width as u32, new.height as u32),
            }));
        }
        if old.position() != new.position() {
            tasks.push(Task::done(UiMsg::MarginChange {
                id: p.id,
                margin: (new.y as i32, 0, 0, new.x as i32),
            }));
        }
        Task::batch(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypr::Client;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn app() -> App {
        let mut app = App::boot_with(
            PathBuf::from("/tmp/hyprdesk-launcher-life/config.lua"),
            true,
        )
        .0;
        app.visible = true;
        app.monitors.push(Monitor {
            dpms_off: false,
            name: "test".into(),
            width: 900,
            height: 900,
            workspace: "1".into(),
            base: String::new(),
            focused: true,
            scale: 1.0,
            reserved: (0.0, 44.0, 0.0, 98.0),
            x: -900.0,
            y: 0.0,
        });
        for (key, x) in [("a", 250.0), ("b", 310.0)] {
            app.launcher_runtime.anchors.insert(
                ("test".into(), key.into()),
                Rectangle::new(Point::new(x, 18.0), Size::new(54.0, 62.0)),
            );
        }
        app
    }
    fn hover(app: &mut App, key: &str) -> u64 {
        let _ = app.launcher_hover(Some((
            "test".into(),
            key.into(),
            PopupContent::Label(key.into()),
            0,
        )));
        app.launcher_popup_token
    }

    #[test]
    fn fast_crossings_and_stale_timers_cannot_open_or_close_another_menu() {
        let mut app = app();
        let old = hover(&mut app, "a");
        assert!(app.launcher_popup.is_none());
        let current = hover(&mut app, "b");
        let _ = app.launcher_popup_open(old);
        assert!(app.launcher_popup.is_none());
        let _ = app.launcher_popup_open(current);
        let popup = app.launcher_popup.as_ref().unwrap();
        assert_eq!(popup.key, "b");
        let bounds = popup.placement.bounds;
        assert!((bounds.center_x() - 337.0).abs() < 1.0);
        let _ = app.launcher_popup_timeout(old);
        assert_eq!(app.launcher_popup.as_ref().unwrap().fade.to, 1.0);
        let _ = app.launcher_hover(None);
        let cancel = app.launcher_popup_token;
        let _ = app.launcher_popup_hover(true);
        let _ = app.launcher_popup_timeout(cancel);
        assert_eq!(app.launcher_popup.as_ref().unwrap().fade.to, 1.0);
    }

    #[test]
    fn returning_during_exit_reverses_the_current_value_in_the_same_surface() {
        let mut app = app();
        let token = hover(&mut app, "a");
        let _ = app.launcher_popup_open(token);
        let _ = app.launcher_hover(None);
        let _ = app.launcher_popup_timeout(app.launcher_popup_token);
        let popup = app.launcher_popup.as_mut().unwrap();
        popup.fade.from = 1.0;
        popup.fade.start = Instant::now() - Duration::from_millis(40);
        let id = popup.id;
        let before = popup.fade.value(Instant::now());
        hover(&mut app, "a");
        let popup = app.launcher_popup.as_ref().unwrap();
        assert_eq!(popup.id, id);
        assert_eq!(popup.fade.to, 1.0);
        assert!((popup.fade.from - before).abs() < 0.02);
    }

    #[test]
    fn hidden_dock_cannot_open_a_delayed_popup_and_long_lists_stay_bounded() {
        let mut app = app();
        let token = hover(&mut app, "a");
        app.visible = false;
        let _ = app.launcher_popup_open(token);
        assert!(app.launcher_popup.is_none());
        app.visible = true;
        let token = hover(&mut app, "a");
        let _ = app.launcher_popup_open(token);
        let _ = app.launcher_fit_popup(Some(10_000.0));
        let popup = app.launcher_popup.as_ref().unwrap();
        assert!(popup.placement.bounds.height <= popup.max_height);
        assert!(popup.placement.bounds.y >= 44.0);
        assert!(popup.placement.bounds.y + popup.placement.bounds.height <= 900.0);
    }

    #[test]
    fn launch_feedback_requires_a_new_matching_window_and_old_timeouts_are_ignored() {
        let mut app = app();
        Arc::make_mut(&mut app.cfg)
            .general
            .launcher
            .pinned
            .push("firefox".into());
        app.clients.push(Client {
            address: "old".into(),
            class: "firefox".into(),
            title: String::new(),
            workspace: "1".into(),
            monitor: "A".into(),
        });
        let _ = app.launcher_launch_started("firefox".into());
        let old = app.launcher_runtime.launches["firefox"].started;
        let _ = app.launcher_frame();
        assert!(app.launcher_runtime.launches.contains_key("firefox"));
        let _ = app.launcher_launch_started("firefox".into());
        let _ =
            app.launcher_launch_ended("firefox", old - Duration::from_secs(1), LaunchEnd::NoWindow);
        assert!(app.launcher_runtime.launches.contains_key("firefox"));
        app.clients.push(Client {
            address: "new".into(),
            class: "firefox".into(),
            title: String::new(),
            workspace: "1".into(),
            monitor: "A".into(),
        });
        let _ = app.launcher_frame();
        assert!(app.launcher_runtime.launches.is_empty());
        let _ = app.launcher_launch_started("missing".into());
        app.launcher_runtime
            .launches
            .get_mut("missing")
            .unwrap()
            .started = Instant::now() - LAUNCH_TIMEOUT;
        let _ = app.launcher_frame();
        assert!(app.launcher_runtime.launches.is_empty());
    }

    #[test]
    fn geometry_uses_logical_coordinates_and_clamps_at_every_edge() {
        let screen = Rectangle::new(Point::new(0.0, 44.0), Size::new(640.0, 856.0));
        for pos in [
            LauncherPosition::Bottom,
            LauncherPosition::Top,
            LauncherPosition::Left,
            LauncherPosition::Right,
        ] {
            for anchor in [
                Rectangle::new(Point::new(8.0, 50.0), Size::new(54.0, 62.0)),
                Rectangle::new(Point::new(580.0, 820.0), Size::new(54.0, 62.0)),
            ] {
                let placement = place_popup(pos, screen, anchor, Size::new(416.0, 300.0));
                let b = placement.bounds;
                assert!(b.x >= 8.0 && b.y >= 52.0);
                assert!(b.x + b.width <= 632.0 && b.y + b.height <= 892.0);
                assert!((0.0..=1.0).contains(&placement.pivot.x));
                assert!((0.0..=1.0).contains(&placement.pivot.y));
                if pos.vertical() {
                    let origin = b.y + 8.0 + placement.pivot.y * (b.height - 16.0);
                    let projected = anchor.center_y().clamp(b.y + 8.0, b.y + b.height - 8.0);
                    assert!((origin - projected).abs() < 0.01);
                } else {
                    let origin = b.x + 8.0 + placement.pivot.x * (b.width - 16.0);
                    let projected = anchor.center_x().clamp(b.x + 8.0, b.x + b.width - 8.0);
                    assert!((origin - projected).abs() < 0.01);
                }
            }
        }
        let app = app();
        let m = &app.monitors[0];
        let local = Rectangle::new(Point::new(310.0, 18.0), Size::new(54.0, 62.0));
        let logical = screen_anchor(m, LauncherPosition::Bottom, 42, local);
        assert_eq!(logical.position(), Point::new(310.0, 820.0));
    }

    #[test]
    fn disabled_motion_has_instant_popup_transitions_and_no_launch_frame_loop() {
        let mut app = app();
        Arc::make_mut(&mut app.cfg).general.transition = Transition::None;
        let token = hover(&mut app, "a");
        let _ = app.launcher_popup_open(token);
        assert!(app.launcher_popup.as_ref().unwrap().fade.dur.is_zero());
        let _ = app.launcher_launch_started("pending".into());
        assert!(!app.launcher_animating());
    }

    #[test]
    fn resolution_and_dispatch_failures_clear_only_the_corresponding_launch() {
        let mut app = app();
        let _ = app.launcher_launch_started("test-app".into());
        let token = app.launcher_runtime.launches["test-app"].started;
        let _ = app.update_inner(UiMsg::LauncherResolved(
            "test-app".into(),
            token,
            String::new(),
        ));
        assert!(app.launcher_runtime.launches.is_empty());
        let _ = app.launcher_launch_started("test-app".into());
        let current = app.launcher_runtime.launches["test-app"].started;
        let _ = app.update_inner(UiMsg::LauncherDispatched(
            "test-app".into(),
            token - Duration::from_secs(1),
            Err("refusé".into()),
        ));
        assert!(app.launcher_runtime.launches.contains_key("test-app"));
        // Commande partie : le lancement suit sa fenêtre jusqu'au délai.
        let _ = app.update_inner(UiMsg::LauncherDispatched(
            "test-app".into(),
            current,
            Ok(()),
        ));
        assert!(app.launcher_runtime.launches.contains_key("test-app"));
        let _ = app.update_inner(UiMsg::LauncherDispatched(
            "test-app".into(),
            current,
            Err("refusé".into()),
        ));
        assert!(app.launcher_runtime.launches.is_empty());
    }

    #[test]
    fn une_fin_de_lancement_est_annoncee_une_fois_et_jamais_si_une_fenetre_est_arrivee() {
        let mut app = app();
        Arc::make_mut(&mut app.apps).push(crate::launch::AppInfo {
            id: "test-app".into(),
            name: "Application d'essai".into(),
            icon: None,
            wm_class: None,
            mime_types: Vec::new(),
            categories: Vec::new(),
            available: true,
        });
        assert_eq!(app.launcher_app_name("test-app"), "Application d'essai");
        // Délai passé sans fenêtre : le lancement finit, une seule fois.
        let _ = app.launcher_launch_started("test-app".into());
        let token = app.launcher_runtime.launches["test-app"].started;
        let _ = app.update_inner(UiMsg::LauncherLaunchExpired("test-app".into(), token));
        assert!(app.launcher_runtime.launches.is_empty());
        let _ = app.update_inner(UiMsg::LauncherLaunchExpired("test-app".into(), token));
        assert!(app.launcher_runtime.launches.is_empty());
        // Une fenêtre est arrivée, d'une autre classe que l'entrée : le
        // lancement finit au délai, sans rien annoncer.
        let _ = app.launcher_launch_started("test-app".into());
        app.clients.push(Client {
            address: "new".into(),
            class: "other".into(),
            title: String::new(),
            workspace: "1".into(),
            monitor: "A".into(),
        });
        let launch = app.launcher_runtime.launches["test-app"].clone();
        assert!(!app.launch_stalled(&launch));
        app.launcher_runtime
            .launches
            .get_mut("test-app")
            .unwrap()
            .started = Instant::now() - LAUNCH_TIMEOUT;
        let _ = app.launcher_frame();
        assert!(app.launcher_runtime.launches.is_empty());
    }

    #[test]
    fn une_fin_de_lancement_est_annoncee_par_le_nom_de_l_application_et_sa_cause() {
        // Application introuvable : avertissement.
        let (icon, ms, msg) = launch_end_notice_text("Firefox", &LaunchEnd::Unresolved);
        assert_eq!((icon, ms), (0, 8000));
        assert!(msg.contains("Firefox"), "{msg}");
        // Commande refusée : avertissement, la cause est nommée.
        let (icon, ms, msg) =
            launch_end_notice_text("Firefox", &LaunchEnd::Refused("pas de exec".into()));
        assert_eq!((icon, ms), (0, 8000));
        assert!(
            msg.contains("Firefox") && msg.contains("pas de exec"),
            "{msg}"
        );
        // Sans fenêtre : un indice, bref, qui propose de relancer.
        let (icon, ms, msg) = launch_end_notice_text("Firefox", &LaunchEnd::NoWindow);
        assert_eq!((icon, ms), (2, 6000));
        assert!(msg.contains("Firefox"), "{msg}");
    }
}
