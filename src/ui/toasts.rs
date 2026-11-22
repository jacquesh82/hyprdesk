//! Bulles de notifications : le démon publie le serveur
//! `org.freedesktop.Notifications` (`crate::notifications`) par un
//! abonnement et affiche chaque notification reçue dans une carte du thème,
//! en haut à droite de l'écran actif, sous la barre. Un clic lance l'action
//! par défaut, les boutons les autres actions, ✕ ferme ; la bulle s'efface
//! d'elle-même après son délai (jamais pour une notification critique).
//!
//! Si un autre démon de notifications tient le nom (wayle, mako…), rien
//! n'est affiché ici : c'est lui qui montre les notifications.

use crate::ui::tokens::Shown as _;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use iced::futures::{SinkExt, Stream};
use iced::widget::{button, column, container, mouse_area, row, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};
use crate::notifications::{Event, Notification, Reason, Urgency};

pub const NAMESPACE: &str = "hyprdesk-notifications";
/// Bulles visibles à la fois (les plus récentes).
pub const VISIBLE: usize = 4;
/// Délai quand l'application laisse le choix au serveur.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(6);
const WIDTH: f32 = 380.0;
const CARD: f32 = 104.0;
const GAP: f32 = 8.0;
const SHADOW: f32 = 18.0;
/// Icône d'en-tête des bulles (jeu d'icônes de hyprdesk).
const ICON: &str = "preferences-system-notifications-symbolic";

/// Demande de l'interface au serveur.
#[derive(Debug, Clone)]
pub enum Cmd {
    Invoke(u32, String),
    Dismiss(u32, Reason),
}

static CMDS: OnceLock<tokio::sync::mpsc::UnboundedSender<Cmd>> = OnceLock::new();

fn send(c: Cmd) {
    if let Some(tx) = CMDS.get() {
        let _ = tx.send(c);
    }
}

/// Abonnement : publie le serveur ; relaie ses événements à l'interface et
/// les demandes de l'interface au serveur.
pub fn stream() -> impl Stream<Item = Event> {
    iced::stream::channel(
        16,
        async move |mut out: iced::futures::channel::mpsc::Sender<Event>| {
            let conn = match zbus::Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("{}", crate::tr!("notifications : {e}", e = e));
                    return;
                }
            };
            let mut h = match crate::notifications::start(conn).await {
                Ok(Some(h)) => h,
                Ok(None) => {
                    tracing::info!(
                        "{}",
                        crate::i18n::tr(
                            "notifications : un autre démon est en place, hyprdesk ne les affiche pas"
                        )
                    );
                    return;
                }
                Err(e) => {
                    tracing::warn!("{}", crate::tr!("notifications : {e}", e = e));
                    return;
                }
            };
            tracing::info!(
                "{}",
                crate::i18n::tr("notifications affichées par hyprdesk")
            );
            crate::notifications::SERVED.store(true, std::sync::atomic::Ordering::Relaxed);
            let _ = out.send(Event::Active).await;
            let (tx, mut cmds) = tokio::sync::mpsc::unbounded_channel();
            let _ = CMDS.set(tx);
            loop {
                tokio::select! {
                    ev = h.events.recv() => {
                        let Some(ev) = ev else { break };
                        if out.send(ev).await.is_err() {
                            break;
                        }
                    }
                    Some(c) = cmds.recv() => {
                        let r = match c {
                            Cmd::Invoke(id, key) => h.invoke(id, &key).await,
                            Cmd::Dismiss(id, reason) => h.dismiss(id, reason).await,
                        };
                        if let Err(e) = r {
                            tracing::warn!("{}", crate::tr!("notifications : {e}", e = e));
                        }
                    }
                }
            }
        },
    )
}

/// Bulle affichée.
#[derive(Debug, Clone)]
pub struct Toast {
    pub n: Notification,
    pub since: Instant,
}

impl Toast {
    /// Délai d'affichage ; `None` : reste jusqu'à ce qu'on la ferme.
    pub fn lifetime(&self) -> Option<Duration> {
        if self.n.urgency == Urgency::Critical {
            return None;
        }
        match self.n.timeout_ms {
            0 => None,
            t if t < 0 => Some(DEFAULT_TIMEOUT),
            t => Some(Duration::from_millis(t as u64)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TMsg {
    Event(Event),
    /// Clic sur une action (`default` : la bulle elle-même).
    Invoke(u32, String),
    Close(u32),
    Tick,
    Shown(Id),
}

/// Surface des bulles et ses bulles (les plus récentes en premier).
#[derive(Default)]
pub struct Toasts {
    pub win: Option<Id>,
    pub list: Vec<Toast>,
    /// hyprdesk tient le service (sinon wayle, mako… affichent).
    pub active: bool,
    /// Historique (les plus récentes d'abord), hors notifications `transient`.
    pub history: Vec<Notification>,
    /// Ne pas déranger : pas de bulle, sauf pour une notification critique.
    pub dnd: bool,
    visibility: Option<super::Fade>,
    previous: Vec<Toast>,
    convex: bool,
    shown: bool,
}

impl Toasts {
    /// Écrit l'état lu par le module de barre et `hyprdesk notifications`.
    fn publish(&self) {
        if self.active {
            let _ = std::fs::write(
                crate::notifications::state_path(),
                crate::notifications::render_state(self.dnd, &self.history),
            );
        }
    }

    /// Applique une commande `hyprdesk notifications …` : `dnd [on|off]`, `clear`.
    pub fn command(&mut self, cmd: &str) {
        let mut it = cmd.split_whitespace();
        match (it.next(), it.next()) {
            (Some("dnd"), Some("on")) => self.dnd = true,
            (Some("dnd"), Some("off")) => self.dnd = false,
            (Some("dnd"), _) => self.dnd = !self.dnd,
            (Some("clear"), _) => self.history.clear(),
            _ => return,
        }
        let p = crate::notifications::dnd_path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = if self.dnd {
            std::fs::write(&p, "1\n")
        } else {
            std::fs::remove_file(&p)
        };
        self.publish();
    }
}

fn height(n: usize) -> u32 {
    let n = n.min(VISIBLE) as f32;
    (n * CARD + (n - 1.0).max(0.0) * GAP + 2.0 * SHADOW).ceil() as u32
}

impl App {
    pub(super) fn toasts_update(&mut self, m: TMsg) -> Task<Msg> {
        match m {
            TMsg::Shown(id) => {
                if self.toasts.win == Some(id) && !self.toasts.shown {
                    self.toasts.shown = true;
                    if let Some(f) = self.toasts.visibility.as_mut() {
                        f.start = Instant::now();
                        f.from = 0.0;
                    }
                }
                Task::none()
            }
            TMsg::Event(Event::Active) => {
                self.toasts.active = true;
                self.toasts.dnd = crate::notifications::dnd_path().exists();
                self.toasts.publish();
                Task::none()
            }
            TMsg::Event(Event::Shown(n)) => {
                if !n.transient {
                    self.toasts.history.retain(|h| h.id != n.id);
                    self.toasts.history.insert(0, n.clone());
                    self.toasts.history.truncate(crate::notifications::HISTORY);
                    self.toasts.publish();
                }
                // Ne pas déranger : dans l'historique seulement, sauf critique.
                if self.toasts.dnd && n.urgency != Urgency::Critical {
                    return Task::none();
                }
                let t = Toast {
                    n,
                    since: Instant::now(),
                };
                match self.toasts.list.iter_mut().find(|o| o.n.id == t.n.id) {
                    Some(o) => *o = t,
                    None => self.toasts.list.insert(0, t),
                }
                let icon = self.want_icon(ICON);
                Task::batch([icon, self.sync_toasts()])
            }
            TMsg::Event(Event::Closed(id)) => {
                self.toasts.list.retain(|t| t.n.id != id);
                self.sync_toasts()
            }
            TMsg::Invoke(id, key) => {
                send(Cmd::Invoke(id, key));
                let resident = self
                    .toasts
                    .list
                    .iter()
                    .any(|t| t.n.id == id && t.n.resident);
                if !resident {
                    self.toasts.list.retain(|t| t.n.id != id);
                }
                self.sync_toasts()
            }
            TMsg::Close(id) => {
                send(Cmd::Dismiss(id, Reason::Dismissed));
                self.toasts.list.retain(|t| t.n.id != id);
                self.sync_toasts()
            }
            TMsg::Tick => {
                let expired: Vec<u32> = self
                    .toasts
                    .list
                    .iter()
                    .filter(|t| t.lifetime().is_some_and(|l| t.since.elapsed() >= l))
                    .map(|t| t.n.id)
                    .collect();
                if expired.is_empty() {
                    return Task::none();
                }
                for id in &expired {
                    send(Cmd::Dismiss(*id, Reason::Expired));
                }
                self.toasts.list.retain(|t| !expired.contains(&t.n.id));
                self.sync_toasts()
            }
        }
    }

    /// Ouvre, redimensionne ou ferme la surface selon les bulles.
    pub(super) fn toasts_animating(&self) -> bool {
        self.toasts.shown
            && self
                .toasts
                .visibility
                .is_some_and(|f| f.from != f.to || !f.done(Instant::now()))
    }
    pub(super) fn toasts_frame(&mut self) -> Task<Msg> {
        if !self.toasts.shown {
            return Task::none();
        }
        if let Some(f) = self.toasts.visibility.as_mut()
            && f.done(Instant::now())
        {
            f.from = f.to;
            f.dur = Duration::ZERO;
            if f.to == 0.0 {
                self.toasts.visibility = None;
                self.toasts.previous.clear();
                self.toasts.shown = false;
                return self
                    .toasts
                    .win
                    .take()
                    .map_or_else(Task::none, iced::window::close);
            }
        }
        Task::none()
    }
    fn sync_convex_toasts(&mut self) -> Task<Msg> {
        let n = self.toasts.list.len();
        let duration = self.bar_duration(self.convex_options().map_or(180, |o| o.duration as u64));
        if n == 0 {
            if !self.toasts.shown {
                self.toasts.visibility = None;
                self.toasts.previous.clear();
                return self
                    .toasts
                    .win
                    .take()
                    .map_or_else(Task::none, iced::window::close);
            }
            if let Some(f) = self.toasts.visibility.as_mut()
                && f.to != 0.0
            {
                *f = super::Fade {
                    from: f.value(Instant::now()),
                    to: 0.0,
                    start: Instant::now(),
                    dur: duration.min(Duration::from_millis(140)),
                };
            }
            if duration.is_zero() {
                return self.toasts_frame();
            }
            return Task::none();
        }
        self.toasts.previous = self.toasts.list.clone();
        let bh = self.cfg.bar.as_ref().map_or(48, |b| b.height) as u32;
        let h = height(n) + bh + 12;
        if let Some(win) = self.toasts.win {
            if let Some(f) = self.toasts.visibility.as_mut()
                && f.to == 0.0
            {
                *f = super::Fade {
                    from: f.value(Instant::now()),
                    to: 1.0,
                    start: Instant::now(),
                    dur: duration,
                };
            }
            return Task::done(Msg::SizeChange {
                id: win,
                size: ((WIDTH + 72.0) as u32, h),
            });
        }
        let win = Id::unique();
        self.toasts.win = Some(win);
        self.toasts.shown = false;
        self.toasts.convex = true;
        self.toasts.visibility = Some(super::Fade {
            from: 0.0,
            to: 1.0,
            start: Instant::now(),
            dur: duration,
        });
        Task::done(Msg::NewLayerShell {
            id: win,
            settings: NewLayerShellSettings {
                size: Some(((WIDTH + 72.0) as u32, h)),
                layer: Layer::Overlay,
                anchor: LsAnchor::Top,
                exclusive_zone: Some(-1),
                margin: Some((0, 0, 0, 0)),
                keyboard_interactivity: KeyboardInteractivity::None,
                output_option: OutputOption::Active,
                namespace: Some(NAMESPACE.into()),
                ..Default::default()
            },
        })
    }
    pub(super) fn sync_toasts(&mut self) -> Task<Msg> {
        if self.convex_options().is_some() {
            return self.sync_convex_toasts();
        }
        let n = self.toasts.list.len();
        match (self.toasts.win, n) {
            (Some(win), 0) => {
                self.toasts.win = None;
                iced::window::close(win)
            }
            (None, 0) => Task::none(),
            (Some(win), n) => Task::done(Msg::SizeChange {
                id: win,
                size: ((WIDTH + 2.0 * SHADOW) as u32, height(n)),
            }),
            (None, n) => {
                let win = Id::unique();
                self.toasts.win = Some(win);
                // Sous la barre si elle est en haut.
                let top = match &self.cfg.bar {
                    Some(b) if b.position == crate::config::BarPosition::Top => b.height as i32,
                    _ => 0,
                };
                Task::done(Msg::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some(((WIDTH + 2.0 * SHADOW) as u32, height(n))),
                        layer: Layer::Overlay,
                        anchor: LsAnchor::Top | LsAnchor::Right,
                        exclusive_zone: Some(-1),
                        margin: Some((top, 0, 0, 0)),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        output_option: OutputOption::Active,
                        namespace: Some(NAMESPACE.into()),
                        ..Default::default()
                    },
                    id: win,
                })
            }
        }
    }

    fn view_toast<'a>(&'a self, t: &'a Toast) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let convex = self.convex_options().is_some();
        let fg = color(if convex { self.bar_palette().fg } else { th.fg }, 1.0);
        let dim = Color { a: 0.65, ..fg };
        let accent = color(th.accent, 1.0);
        let bg = color(
            th.bg,
            th.shown(crate::config::Place::Menus).opacity.max(0.97),
        );
        let edge = if t.n.urgency == Urgency::Critical {
            accent
        } else {
            Color { a: 0.12, ..fg }
        };
        let id = t.n.id;
        let icon: Element<'_, Msg> = self
            .icon_named(ICON, 18.0, dim)
            .unwrap_or_else(|| iced::widget::Space::new().width(18).into());
        let head = row![
            icon,
            text(if t.n.app.is_empty() {
                crate::i18n::tr("Notification").to_string()
            } else {
                t.n.app.clone()
            })
            .font(self.font)
            .size(crate::ui::tokens::ts(11.0))
            .color(dim)
            .width(Length::Fill),
            button(
                text("✕")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim)
            )
            .padding([0, 6])
            .style(button::text)
            .on_press(Msg::Toasts(TMsg::Close(id))),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let mut col = column![
            head,
            text(t.n.summary.clone())
                .font(self.title_font())
                .size(crate::ui::tokens::ts(14.0))
                .color(fg)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(4);
        if !t.n.body.is_empty() {
            // Deux lignes au plus (la carte a une hauteur fixe).
            let body: String = t.n.body.lines().take(2).collect::<Vec<_>>().join("\n");
            col = col.push(
                text(body)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .line_height(crate::ui::tokens::line_height())
                    .color(dim),
            );
        }
        let extra: Vec<&(String, String)> =
            t.n.actions
                .iter()
                .filter(|(k, _)| k != "default")
                .take(3)
                .collect();
        if !extra.is_empty() {
            let mut r = row![].spacing(8);
            for (k, label) in extra {
                r = r.push(
                    button(
                        text(label.clone())
                            .font(self.font)
                            .size(crate::ui::tokens::ts(11.0)),
                    )
                    .padding([2, 10])
                    .on_press(Msg::Toasts(TMsg::Invoke(id, k.clone()))),
                );
            }
            col = col.push(r);
        }
        let card = container(col)
            .padding([10, 14])
            .width(WIDTH)
            .height(CARD)
            .clip(true)
            .style(move |_| {
                if convex {
                    container::Style {
                        text_color: Some(fg),
                        ..Default::default()
                    }
                } else {
                    container::Style {
                        background: Some(bg.into()),
                        border: Border::default()
                            .rounded(super::design::SMALL_RADIUS * 1.4)
                            .width(1)
                            .color(edge),
                        shadow: Shadow {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                            offset: Vector::new(0.0, 6.0),
                            blur_radius: 18.0,
                        },
                        text_color: Some(fg),
                        ..Default::default()
                    }
                }
            });
        // Clic sur la carte : action par défaut (si l'application en propose une).
        if t.n.actions.iter().any(|(k, _)| k == "default") {
            mouse_area(card)
                .on_press(Msg::Toasts(TMsg::Invoke(id, "default".into())))
                .into()
        } else {
            card.into()
        }
    }

    pub(super) fn view_notification_history(&self, alpha: f32) -> Element<'_, Msg> {
        let fg = color(self.bar_palette().fg, alpha);
        let mut col = column![
            text(crate::i18n::tr("Notifications"))
                .font(self.font)
                .size(crate::ui::tokens::ts(16.0))
                .color(fg)
        ]
        .spacing(12);
        if self.toasts.history.is_empty() {
            col = col.push(
                text(crate::i18n::tr("Aucune notification"))
                    .font(self.font)
                    .color(fg),
            );
        }
        for n in self.toasts.history.iter().take(30) {
            col = col.push(
                column![
                    text(n.app.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(11.0))
                        .color(fg),
                    text(n.summary.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(14.0))
                        .color(fg),
                    text(n.body.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .line_height(crate::ui::tokens::line_height())
                        .color(fg)
                ]
                .spacing(4),
            );
        }
        col.into()
    }

    pub(super) fn view_toasts(&self) -> Element<'_, Msg> {
        let mut col = column![].spacing(GAP);
        let visible = if self.toasts.list.is_empty() && self.toasts.convex {
            &self.toasts.previous
        } else {
            &self.toasts.list
        };
        for t in visible.iter().take(VISIBLE) {
            col = col.push(self.view_toast(t));
        }
        if self.convex_options().is_some() {
            use super::{
                kinetic::{self, Track},
                plugins::bar::style::convexe::Edge,
            };
            let win = self.toasts.win.unwrap_or_else(Id::unique);
            let track = if !self.toasts.shown {
                super::Fade {
                    from: 0.0,
                    to: 0.0,
                    start: Instant::now(),
                    dur: Duration::ZERO,
                }
            } else {
                self.toasts.visibility.unwrap_or(super::Fade {
                    from: 1.0,
                    to: 1.0,
                    start: Instant::now(),
                    dur: Duration::ZERO,
                })
            };
            let bh = self.cfg.bar.as_ref().map_or(48, |b| b.height) as f32;
            let content = container(col)
                .padding(iced::Padding {
                    top: bh + 12.0,
                    bottom: 24.0,
                    left: 36.0,
                    right: 36.0,
                })
                .height(Length::Fill)
                .width(Length::Fill);
            let content: Element<'_, Msg> = if self.toasts.visibility.is_some_and(|f| f.to == 0.0) {
                crate::ui::plugins::bar::motion::inert(content)
            } else {
                content.into()
            };
            let pal = self.bar_palette();
            return iced::widget::sensor(
                container(kinetic::reveal(
                    content,
                    Track::Ease(track),
                    Edge::Top,
                    color(pal.bg, pal.opacity),
                    self.toasts.shown.then_some(Msg::MotionSettled),
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true),
            )
            .key(win)
            .on_show(move |_| Msg::Toasts(TMsg::Shown(win)))
            .into();
        }
        container(col).padding(SHADOW).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn toast(urgency: u8, timeout: i32) -> Toast {
        let mut h = HashMap::new();
        h.insert(
            "urgency".to_string(),
            zbus::zvariant::OwnedValue::from(urgency),
        );
        Toast {
            n: crate::notifications::build(1, "a", "", "s", "b", &[], &h, timeout),
            since: Instant::now(),
        }
    }

    #[test]
    fn convex_notification_waits_for_presentation_and_retracts_before_destroying() {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-toast-tests/config.lua"),
            true,
        );
        let loaded = crate::config::lua::load_source(
            "hd.bar({style='convex'})",
            "test.lua",
            std::path::Path::new("/tmp"),
        )
        .unwrap();
        app.cfg = std::sync::Arc::new(loaded.config);
        let _ = app.toasts_update(TMsg::Event(Event::Shown(toast(1, -1).n)));
        let win = app.toasts.win.unwrap();
        app.toasts.visibility.as_mut().unwrap().start = Instant::now() - Duration::from_secs(1);
        let _ = app.toasts_frame();
        assert!(!app.toasts_animating());
        let _ = app.toasts_update(TMsg::Shown(win));
        assert!(app.toasts_animating());
        assert!(app.toasts.visibility.unwrap().start.elapsed() < Duration::from_millis(50));
        let _ = app.toasts_update(TMsg::Close(1));
        assert_eq!(app.toasts.win, Some(win));
        assert_eq!(app.toasts.previous.len(), 1);
        app.toasts.visibility.as_mut().unwrap().start = Instant::now() - Duration::from_secs(1);
        let _ = app.toasts_frame();
        assert!(app.toasts.win.is_none());
        assert!(!app.toasts_animating());
    }

    #[test]
    fn duree_d_affichage() {
        assert_eq!(toast(1, -1).lifetime(), Some(DEFAULT_TIMEOUT));
        assert_eq!(toast(1, 2500).lifetime(), Some(Duration::from_millis(2500)));
        assert_eq!(toast(1, 0).lifetime(), None);
        assert_eq!(toast(2, 2500).lifetime(), None, "critique : reste");
    }

    #[test]
    fn hauteur_de_la_surface() {
        assert_eq!(height(1), (CARD + 2.0 * SHADOW).ceil() as u32);
        assert_eq!(height(9), height(VISIBLE));
    }
}
