//! Cadre Convex : surface décorative indépendante, sans zone de saisie.
//! La barre conserve sa réservation ; le centre transparent laisse les fenêtres
//! et le bureau recevoir leurs événements, même dans les angles arrondis.
use super::{App, Msg, plugins::bar::style::convexe};
use crate::config::{BarStyle, ConvexDef};
use iced::{Element, Task, widget::Space, window::Id};
use iced_layershell::{
    actions::ActionCallback,
    reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption},
};

use super::{
    kinetic::{self, Spring, Track},
    plugins::bar::style::convexe::Edge,
};
use iced::widget::{button, column, container, mouse_area, pin, row, scrollable, stack, text};
use iced::{Alignment, Length, Point, Rectangle, Size};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Part {
    Left,
    Right,
    Dock,
}
#[derive(Default)]
pub(super) struct Runtime {
    motion: HashMap<(String, Part), Spring>,
    /// Zones survolées : la partie elle-même, et (dernier champ) la bande de
    /// réveil du dock. Comptées à part : quitter la bande en montant vers
    /// le dock ne doit pas effacer le survol du dock, qui n'émet pas de
    /// nouvelle entrée (le pointeur y était déjà).
    hovered: HashSet<(String, Part, bool)>,
    tokens: HashMap<(String, Part), u64>,
}

impl App {
    pub(super) fn convex_options(&self) -> Option<ConvexDef> {
        self.cfg
            .bar
            .as_ref()
            .filter(|b| b.style == BarStyle::Convexe)
            .map(|b| b.convex)
    }

    pub(super) fn sync_convex_frames(&mut self) -> Task<Msg> {
        let want: Vec<_> = if self.convex_options().is_some() {
            self.bar_surfaces
                .values()
                .map(|s| s.monitor.clone())
                .collect()
        } else {
            Vec::new()
        };
        let mut tasks = Vec::new();
        for (id, monitor) in self.convex_frames.clone() {
            if !want.contains(&monitor) {
                self.convex_frames.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        for monitor in want {
            if self.convex_frames.values().any(|m| m == &monitor) {
                continue;
            }
            let id = Id::unique();
            self.convex_frames.insert(id, monitor.clone());
            tasks.push(Task::done(Msg::NewLayerShell {
                id,
                settings: NewLayerShellSettings {
                    size: Some((0, 0)),
                    layer: Layer::Top,
                    anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(monitor),
                    namespace: Some("hyprdesk-convex-frame".into()),
                    ..Default::default()
                },
            }));
        }
        Task::batch(tasks)
    }

    pub(super) fn convex_auto_hide(&self) -> bool {
        self.convex_options().is_some_and(|o| o.auto_hide)
    }
    fn convex_duration(&self, opening: bool) -> Duration {
        self.bar_duration(
            self.convex_options()
                .map_or(180, |o| o.duration as u64)
                .min(if opening { 220 } else { 140 }),
        )
    }
    pub(super) fn convex_motion(&self, monitor: &str, part: Part) -> Spring {
        self.convex_runtime
            .motion
            .get(&(monitor.into(), part))
            .copied()
            .unwrap_or_else(|| Spring::rest(0.0))
    }
    pub(super) fn convex_progress(&self, monitor: &str, part: Part) -> f32 {
        self.convex_runtime
            .motion
            .get(&(monitor.into(), part))
            .map_or(0.0, |f| {
                if self.cfg.general.transition == crate::config::Transition::None {
                    f.to
                } else {
                    f.value(Instant::now())
                }
            })
    }
    pub(super) fn convex_animating(&self) -> bool {
        self.convex_runtime
            .motion
            .values()
            .any(|f| f.from != f.to || !f.done(Instant::now()))
    }
    pub(super) fn convex_toggle(&mut self, monitor: String, part: Part) -> Task<Msg> {
        if self
            .convex_runtime
            .motion
            .get(&(monitor.clone(), part))
            .is_some_and(|f| f.to == 1.0)
        {
            self.convex_hide(monitor, part, None)
        } else {
            self.convex_hover(monitor, part, true)
        }
    }
    pub(super) fn convex_hover(&mut self, monitor: String, part: Part, inside: bool) -> Task<Msg> {
        self.convex_zone(monitor, part, false, inside)
    }
    /// Bande de réveil du dock (bord de l'écran, dock replié).
    pub(super) fn convex_wake(&mut self, monitor: String, inside: bool) -> Task<Msg> {
        self.convex_zone(monitor, Part::Dock, true, inside)
    }
    fn convex_zone(&mut self, monitor: String, part: Part, wake: bool, inside: bool) -> Task<Msg> {
        let key = (monitor.clone(), part);
        let token = self.convex_runtime.tokens.entry(key.clone()).or_default();
        *token += 1;
        let token = *token;
        let zone = (monitor.clone(), part, wake);
        if inside {
            self.convex_runtime.hovered.insert(zone);
            if self
                .convex_runtime
                .motion
                .get(&key)
                .is_some_and(|f| f.to == 1.0)
            {
                return Task::none();
            }
            let motion = self.convex_motion(&monitor, part).retarget(
                1.0,
                self.convex_duration(true),
                Instant::now(),
            );
            self.convex_runtime.motion.insert(key, motion);
            self.convex_inputs(&monitor)
        } else {
            self.convex_runtime.hovered.remove(&zone);
            Task::perform(
                async move {
                    tokio::time::sleep(Duration::from_millis(180)).await;
                    (monitor, part, token)
                },
                |(m, p, t)| Msg::ConvexHide(m, p, t),
            )
        }
    }
    pub(super) fn convex_hide(
        &mut self,
        monitor: String,
        part: Part,
        token: Option<u64>,
    ) -> Task<Msg> {
        let key = (monitor.clone(), part);
        if token.is_some_and(|t| {
            self.convex_runtime.tokens.get(&key) != Some(&t) || self.convex_hovered(&monitor, part)
        }) {
            return Task::none();
        }
        self.convex_runtime
            .hovered
            .retain(|(m, p, _)| !(m == &monitor && *p == part));
        let motion = self.convex_motion(&monitor, part).retarget(
            0.0,
            self.convex_duration(false),
            Instant::now(),
        );
        self.convex_runtime.motion.insert(key, motion);
        self.convex_inputs(&monitor)
    }
    fn convex_hovered(&self, monitor: &str, part: Part) -> bool {
        self.convex_runtime
            .hovered
            .iter()
            .any(|(m, p, _)| m == monitor && *p == part)
    }
    fn convex_inputs(&self, monitor: &str) -> Task<Msg> {
        Task::batch(
            self.convex_frames
                .iter()
                .filter(|(_, m)| m.as_str() == monitor)
                .map(|(id, _)| self.convex_frame_input(*id))
                .chain(
                    self.launcher_surfaces
                        .iter()
                        .filter(|(_, s)| s.monitor == monitor)
                        .map(|(id, _)| self.convex_dock_input(*id)),
                ),
        )
    }
    pub(super) fn convex_frame_tick(&mut self) -> Task<Msg> {
        if !self.convex_animating() {
            return Task::none();
        }
        let now = Instant::now();
        let mut monitors = Vec::new();
        for ((monitor, _), f) in &mut self.convex_runtime.motion {
            if f.from != f.to && f.done(now) {
                f.snap();
                monitors.push(monitor.clone());
            }
        }
        Task::batch(monitors.iter().map(|m| self.convex_inputs(m)))
    }
    fn side_bounds(size: Size, part: Part) -> Rectangle {
        let width = 380.0_f32.min(size.width * 0.85);
        let height = 540.0_f32.min((size.height - 120.0).max(100.0));
        Rectangle::new(
            Point::new(
                if part == Part::Left {
                    0.0
                } else {
                    size.width - width
                },
                (size.height - height) * 0.5,
            ),
            Size::new(width, height),
        )
    }
    pub(super) fn convex_frame_input(&self, id: Id) -> Task<Msg> {
        let Some(monitor) = self.convex_frames.get(&id) else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| &m.name == monitor) else {
            return Task::none();
        };
        let size = Size::new(m.width as f32, m.height as f32);
        let mut rects = Vec::new();
        if self.convex_options().is_some_and(|o| o.attached_panels) {
            for part in [Part::Left, Part::Right] {
                rects.push(Rectangle::new(
                    Point::new(
                        if part == Part::Left {
                            0.0
                        } else {
                            size.width - 12.0
                        },
                        (size.height - 96.0) * 0.5,
                    ),
                    Size::new(12.0, 96.0),
                ));
                if self.convex_progress(monitor, part) > 0.0
                    || self
                        .convex_runtime
                        .motion
                        .get(&(monitor.clone(), part))
                        .is_some_and(|f| f.to == 1.0)
                {
                    rects.push(Self::side_bounds(size, part));
                }
            }
        }
        Task::done(Msg::SetInputRegion {
            id,
            callback: ActionCallback::new(move |region| {
                for r in &rects {
                    region.add(
                        r.x as i32,
                        r.y as i32,
                        r.width.ceil() as i32,
                        r.height.ceil() as i32,
                    );
                }
            }),
        })
    }
    pub(super) fn convex_dock_input(&self, id: Id) -> Task<Msg> {
        let auto_hide = self.convex_auto_hide();
        let Some(surface) = self.launcher_surfaces.get(&id) else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == surface.monitor) else {
            return Task::none();
        };
        let open = !auto_hide
            || self.convex_progress(&m.name, Part::Dock) > 0.0
            || self
                .convex_runtime
                .motion
                .get(&(m.name.clone(), Part::Dock))
                .is_some_and(|f| f.to == 1.0);
        let w = m.width as i32;
        let h = surface.thickness as i32;
        let width = ((self.dock_slots(m).len() as f32
            * (self.cfg.general.launcher.size as f32 + 16.0)
            + 100.0) as i32)
            .min(w);
        Task::done(Msg::SetInputRegion {
            id,
            callback: ActionCallback::new(move |r| {
                r.add(
                    (w - width) / 2,
                    if open { 0 } else { h - 6 },
                    width,
                    if open { h } else { 6 },
                );
            }),
        })
    }
    pub(super) fn view_convex_frame_on<'a>(&'a self, monitor: &'a str) -> Element<'a, Msg> {
        let mut layers = stack![self.view_convex_frame_skin()];
        let Some(m) = self.monitors.iter().find(|m| m.name == monitor) else {
            return layers.into();
        };
        let size = Size::new(m.width as f32, m.height as f32);
        if self.convex_options().is_some_and(|o| o.attached_panels) {
            for part in [Part::Left, Part::Right] {
                let enter = monitor.to_owned();
                let leave = monitor.to_owned();
                let close = monitor.to_owned();
                let bounds = Self::side_bounds(size, part);
                let progress = self.convex_progress(monitor, part);
                let motion = self.convex_motion(monitor, part);
                if progress > 0.0 || motion.to > 0.0 || !motion.done(Instant::now()) {
                    let pal = self.bar_palette();
                    let fg = super::color(pal.fg, 1.0);
                    let title = if part == Part::Left {
                        crate::i18n::tr("Bureau")
                    } else {
                        ""
                    };
                    let head = row![
                        text(title)
                            .font(self.font)
                            .size(16)
                            .color(fg)
                            .width(Length::Fill),
                        button(text("×").color(fg))
                            .on_press(Msg::ConvexClose(close, part))
                            .style(iced::widget::button::text)
                    ]
                    .align_y(Alignment::Center);
                    let body = if part == Part::Left {
                        self.convex_dashboard(monitor)
                    } else {
                        self.controls_view(1.0, monitor, None)
                    };
                    let content =
                        container(column![head, scrollable(body).height(Length::Fill)].spacing(16))
                            .padding([24, 36])
                            .width(Length::Fill)
                            .height(Length::Fill);
                    let drawer = container(kinetic::reveal(
                        content,
                        Track::Spring(motion),
                        if part == Part::Left {
                            Edge::Left
                        } else {
                            Edge::Right
                        },
                        super::color(pal.bg, pal.opacity),
                        Some(Msg::MotionSettled),
                    ))
                    .width(bounds.width)
                    .height(bounds.height)
                    .clip(true);
                    let area = mouse_area(drawer)
                        .on_enter(Msg::ConvexHover(enter.clone(), part, true))
                        .on_exit(Msg::ConvexHover(leave.clone(), part, false));
                    layers = layers.push(pin(area).x(bounds.x).y(bounds.y));
                } else {
                    let indicator = container(iced::widget::Space::new())
                        .width(12)
                        .height(96)
                        .style(|_| container::Style {
                            background: Some(iced::Color::from_rgb8(68, 68, 78).into()),
                            border: iced::Border::default().rounded(6),
                            ..Default::default()
                        });
                    layers = layers.push(
                        pin(mouse_area(indicator)
                            .on_enter(Msg::ConvexHover(enter, part, true))
                            .on_exit(Msg::ConvexHover(leave, part, false)))
                        .x(if part == Part::Left {
                            0.0
                        } else {
                            size.width - 12.0
                        })
                        .y((size.height - 96.0) * 0.5),
                    );
                }
            }
        }
        layers.into()
    }
    fn convex_dashboard(&self, monitor: &str) -> Element<'_, Msg> {
        let fg = super::color(self.bar_palette().fg, 1.0);
        let mut col = column![
            text(super::format_now("%H:%M"))
                .font(self.font)
                .size(36)
                .color(fg),
            text(super::format_now("%A %-d %B"))
                .font(self.font)
                .color(fg)
        ]
        .spacing(12);
        for w in self.workspaces.iter().filter(|w| w.monitor == monitor) {
            col = col.push(
                button(text(w.name.clone()).font(self.font).color(fg))
                    .width(Length::Fill)
                    .on_press(Msg::BarWorkspace(monitor.into(), w.name.clone()))
                    .style(iced::widget::button::text),
            );
        }
        for value in self.values.values() {
            if let crate::widgets::Value::Sys(lines) = value {
                for line in lines {
                    col = col.push(row![
                        text(line.label.clone())
                            .font(self.font)
                            .color(fg)
                            .width(Length::Fill),
                        text(line.value.clone()).font(self.font).color(fg)
                    ]);
                }
                break;
            }
        }
        col.push(
            button(text(crate::i18n::tr("Applications")).font(self.font))
                .on_press(Msg::Control(crate::control::Command::Apps)),
        )
        .into()
    }

    pub(super) fn view_convex_frame(&self) -> Element<'_, Msg> {
        match self.monitors.first() {
            Some(m) => self.view_convex_frame_on(&m.name),
            None => self.view_convex_frame_skin(),
        }
    }
    fn view_convex_frame_skin(&self) -> Element<'_, Msg> {
        match self.convex_options() {
            Some(options) => convexe::frame(
                &self.bar_palette(),
                options,
                self.cfg.bar.as_ref().map_or(48.0, |b| b.height as f32),
                self.cfg
                    .bar
                    .as_ref()
                    .is_none_or(|b| b.position == crate::config::BarPosition::Top),
            ),
            None => Space::new().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        sync::Arc,
        time::{Duration, Instant},
    };
    fn app() -> App {
        let (mut app, _) =
            App::boot_with(PathBuf::from("/tmp/hyprdesk-convex-tests/config.lua"), true);
        let loaded = crate::config::lua::load_source(
            "hd.bar({style='convex'})",
            "test.lua",
            Path::new("/tmp"),
        )
        .unwrap();
        app.cfg = Arc::new(loaded.config);
        app
    }
    #[test]
    fn frames_follow_only_monitors_with_a_convex_bar_and_leave_with_the_style() {
        let mut app = app();
        let id = Id::unique();
        app.bar_surfaces.insert(
            id,
            crate::ui::plugins::bar::surface::BarSurface {
                monitor: "A".into(),
                position: crate::config::BarPosition::Top,
                height: 48,
                margin: 0,
            },
        );
        let _ = app.sync_convex_frames();
        assert_eq!(app.convex_frames.len(), 1);
        let frame = *app.convex_frames.keys().next().unwrap();
        let _ = app.sync_convex_frames();
        assert!(app.convex_frames.contains_key(&frame));
        app.bar_surfaces.clear();
        let _ = app.sync_convex_frames();
        assert!(app.convex_frames.is_empty());
        app.bar_surfaces.insert(
            id,
            crate::ui::plugins::bar::surface::BarSurface {
                monitor: "A".into(),
                position: crate::config::BarPosition::Top,
                height: 48,
                margin: 0,
            },
        );
        let _ = app.sync_convex_frames();
        assert_eq!(app.convex_frames.len(), 1);
        Arc::make_mut(&mut app.cfg).bar.as_mut().unwrap().style = BarStyle::Continuite;
        let _ = app.sync_convex_frames();
        assert!(app.convex_frames.is_empty());
    }
    #[test]
    fn hover_opens_immediately_and_delayed_hide_cannot_cancel_reentry() {
        let mut app = app();
        assert!(app.convex_auto_hide());
        assert_eq!(app.convex_progress("A", Part::Dock), 0.0);
        for part in [Part::Dock, Part::Left, Part::Right] {
            let _ = app.convex_hover("A".into(), part, true);
            assert_eq!(app.convex_runtime.motion[&("A".into(), part)].to, 1.0);
            let _ = app.convex_hover("A".into(), part, false);
            let old = app.convex_runtime.tokens[&("A".into(), part)];
            let _ = app.convex_hover("A".into(), part, true);
            let _ = app.convex_hide("A".into(), part, Some(old));
            assert_eq!(app.convex_runtime.motion[&("A".into(), part)].to, 1.0);
            let _ = app.convex_hide("A".into(), part, None);
            app.convex_runtime
                .motion
                .get_mut(&("A".into(), part))
                .unwrap()
                .start = Instant::now() - Duration::from_secs(1);
        }
        let _ = app.convex_frame_tick();
        assert!(!app.convex_animating());
        assert_eq!(app.convex_progress("A", Part::Dock), 0.0);
        let pal = app.bar_palette();
        assert_eq!(pal.bg, crate::config::Rgba(12, 12, 16, 255));
        assert_eq!(pal.opacity, 1.0);
    }
    #[test]
    fn leaving_the_wake_strip_upward_keeps_the_dock_open() {
        let mut app = app();
        // Le pointeur entre dans la bande de réveil, déjà sur le dock.
        let _ = app.convex_wake("A".into(), true);
        let _ = app.convex_hover("A".into(), Part::Dock, true);
        // Il monte vers les icônes : seule la bande signale sa sortie.
        let _ = app.convex_wake("A".into(), false);
        let token = app.convex_runtime.tokens[&("A".into(), Part::Dock)];
        let _ = app.convex_hide("A".into(), Part::Dock, Some(token));
        assert_eq!(app.convex_runtime.motion[&("A".into(), Part::Dock)].to, 1.0);
        // Il quitte le dock : le repli différé passe.
        let _ = app.convex_hover("A".into(), Part::Dock, false);
        let token = app.convex_runtime.tokens[&("A".into(), Part::Dock)];
        let _ = app.convex_hide("A".into(), Part::Dock, Some(token));
        assert_eq!(app.convex_runtime.motion[&("A".into(), Part::Dock)].to, 0.0);
    }
    #[test]
    fn local_hover_needs_no_application_frame_subscription() {
        let mut app = app();
        let _ = app.convex_hover("A".into(), Part::Left, true);
        assert!(app.convex_animating());
        assert!(
            !app.animating(),
            "le widget doit demander ses images sans réveiller tous les écrans"
        );
    }

    #[test]
    fn reduced_motion_applies_to_hover_and_independent_sides() {
        let mut app = app();
        Arc::make_mut(&mut app.cfg).general.transition = crate::config::Transition::None;
        let _ = app.convex_hover("A".into(), Part::Left, true);
        let _ = app.convex_hover("A".into(), Part::Right, true);
        assert_eq!(app.convex_progress("A", Part::Left), 1.0);
        let _ = app.convex_hide("A".into(), Part::Left, None);
        assert_eq!(app.convex_progress("A", Part::Left), 0.0);
        assert_eq!(app.convex_progress("A", Part::Right), 1.0);
    }

    #[test]
    fn launcher_reveal_reverses_then_releases_the_surface_after_closing() {
        let mut app = app();
        assert!(!app.launcher_sheet());
        let _ = app.toggle_search();
        let win = app.search.as_ref().unwrap().win;
        let _ = app.close_search();
        assert_eq!(app.search.as_ref().unwrap().visibility.unwrap().to, 0.0);
        let _ = app.toggle_search();
        assert_eq!(app.search.as_ref().unwrap().win, win);
        assert_eq!(app.search.as_ref().unwrap().visibility.unwrap().to, 1.0);
        let _ = app.close_search();
        app.search
            .as_mut()
            .unwrap()
            .visibility
            .as_mut()
            .unwrap()
            .start = Instant::now() - Duration::from_secs(2);
        let _ = app.search_frame();
        assert!(app.search.is_none());
    }
}
