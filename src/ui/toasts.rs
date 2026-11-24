//! Bulles de notifications : le démon publie le serveur
//! `org.freedesktop.Notifications` (`crate::notifications`) par un
//! abonnement et affiche chaque notification reçue dans une carte du thème,
//! en haut à droite de l'écran actif, sous la barre. Un clic lance l'action
//! par défaut, les boutons les autres actions, ✕ ferme ; la bulle s'efface
//! d'elle-même après son délai (jamais pour une notification critique).
//! Une notification de basse urgence ne montre pas de bulle : livrée
//! discrètement, elle va à l'historique seul, comme sous Silence.
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
use crate::notifications::{Event, Notification, Reason, SilenceEnd, Urgency};
use crate::ui::plugins::bar::surface::{URGENT_ICON, truncate};

pub const NAMESPACE: &str = "hyprdesk-notifications";
/// Bulles visibles à la fois (les plus récentes).
pub const VISIBLE: usize = 4;
/// Délai quand l'application laisse le choix au serveur.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(6);
const WIDTH: f32 = 380.0;
const CARD: f32 = 104.0;
const GAP: f32 = 8.0;
const SHADOW: f32 = 18.0;
/// Cloche de repli, quand l'application n'a pas donné d'icône (jeu de hyprdesk).
const ICON: &str = "preferences-system-notifications-symbolic";
/// Côté de l'icône de l'application dans une bulle.
const APP_ICON: f32 = 36.0;
/// Côté de l'icône de l'application dans une carte du centre.
pub(super) const CARD_ICON: f32 = 28.0;
/// Titre et corps bornés dans les cartes de notifications, comme au centre
/// Continuité (HIG LAY-04) : le corps entier de trente notifications ne tient
/// pas dans la hauteur fixe de la carte.
pub(super) const TITLE_CHARS: usize = 42;
pub(super) const BODY_CHARS: usize = 74;
/// Texte secondaire (application, corps) : la couleur du texte courant à
/// cette opacité, dans la bulle comme dans la carte du centre.
const DIM: f32 = 0.65;
/// Côté du repère d'une notification critique dans la bulle.
const CRITICAL_MARK: f32 = 10.0;

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
                        let action = matches!(c, Cmd::Invoke(..));
                        let r = match c {
                            Cmd::Invoke(id, key) => h.invoke(id, &key).await,
                            Cmd::Dismiss(id, reason) => h.dismiss(id, reason).await,
                        };
                        if let Err(e) = r {
                            tracing::warn!("{}", crate::tr!("notifications : {e}", e = e));
                            // L'échec est montré, pas seulement journalisé
                            // (HIG PAT-11) ; la bulle de Hyprland est hors de
                            // ce serveur : elle ne se répond pas à elle-même.
                            let msg = if action {
                                crate::tr!("Action de la notification impossible : {e}", e = e)
                            } else {
                                crate::tr!(
                                    "Fermeture d'une notification non signalée à son application : {e}",
                                    e = e
                                )
                            };
                            tokio::task::spawn_blocking(move || crate::hypr::notify(0, 8000, &msg));
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
        self.lifetime_or(DEFAULT_TIMEOUT)
    }

    /// Délai d'affichage, `default` quand l'application n'en demande pas
    /// (réglé : `notifications.timeout`, MLG-279).
    pub fn lifetime_or(&self, default: Duration) -> Option<Duration> {
        if self.n.urgency == Urgency::Critical {
            return None;
        }
        match self.n.timeout_ms {
            0 => None,
            t if t < 0 => Some(default),
            t => Some(Duration::from_millis(t as u64)),
        }
    }
}

/// Ce que fait un clic sur une carte du centre : l'action `default` quand
/// l'application en propose une, sinon l'ouverture de l'application si elle
/// est connue (`desktop-entry`). `None` : la carte ne mène nulle part.
#[derive(Debug, Clone, PartialEq)]
enum CardClick {
    Default,
    Open(String),
}

/// Identifiant d'application d'un hint `desktop-entry` : le suffixe
/// `.desktop` est toléré (les deux formes circulent). `None` si le hint
/// n'en est pas un (vide, ou un chemin : il ne doit pas mener hors des
/// dossiers `applications`).
fn app_id(entry: &str) -> Option<&str> {
    let id = entry.strip_suffix(".desktop").unwrap_or(entry);
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

fn card_click(n: &Notification) -> Option<CardClick> {
    if n.actions.iter().any(|(k, _)| k == "default") {
        return Some(CardClick::Default);
    }
    Some(CardClick::Open(
        app_id(n.desktop_entry.as_deref()?)?.to_string(),
    ))
}

/// Message d'un clic sur une carte du centre ; `None` : rien à ouvrir, la
/// carte n'est pas cliquable.
pub(super) fn card_press(n: &Notification) -> Option<Msg> {
    card_click(n).map(|_| Msg::Toasts(TMsg::Card(n.id)))
}

/// Titre et corps d'une carte de notification, bornés (HIG LAY-04) ; le corps
/// est `None` quand il est vide, un texte générique quand les aperçus sont
/// masqués (HIG SYS-05).
pub(super) fn card_text(n: &Notification, hide: bool) -> (String, Option<String>) {
    let body = if hide {
        crate::i18n::tr("Notification").to_string()
    } else {
        truncate(&n.body.replace('\n', " "), BODY_CHARS)
    };
    (
        truncate(&n.summary, TITLE_CHARS),
        (!n.body.is_empty()).then_some(body),
    )
}

/// Ouvre l'application d'origine par son identifiant `desktop-entry`, et
/// seulement si son entrée est installée. Le hint arrive par le bus : il ne
/// doit jamais servir de commande (le lanceur tente l'identifiant seul quand
/// l'entrée manque). La recherche lit des dossiers : hors du thread de
/// l'interface.
fn open_app(entry: String) -> Task<Msg> {
    Task::perform(
        async move {
            let found = tokio::task::spawn_blocking({
                let e = entry.clone();
                move || crate::launch::find_desktop_file(&e).is_some()
            })
            .await
            .unwrap_or(false);
            found.then_some(entry)
        },
        |entry| entry.map_or(Msg::Noop, Msg::LauncherLaunch),
    )
}

#[derive(Debug, Clone)]
pub enum TMsg {
    Event(Event),
    /// Clic sur une action (`default` : la bulle elle-même).
    Invoke(u32, String),
    /// Clic sur une carte du centre de notifications.
    Card(u32),
    Close(u32),
    Tick,
    Shown(Id),
    /// La fin programmée du Silence est atteinte (HIG PAT-04).
    Expire,
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
    /// Fin du Silence (HIG PAT-04) : échéance à l'horloge murale, en secondes
    /// depuis l'époque ; `None`, jusqu'à désactivation.
    pub dnd_until: Option<i64>,
    visibility: Option<super::Fade>,
    previous: Vec<Toast>,
    convex: bool,
    shown: bool,
}

/// Carte de choix de la fin du Silence (HIG PAT-04). L'action d'une entrée
/// est son rang, qui ne dépend pas des entrées proposées.
pub(in crate::ui) fn silence_menu() -> Vec<crate::config::MenuEntry> {
    use crate::config::MenuEntry;
    let end = |rank: i64, label: &str| MenuEntry::Item {
        label: crate::i18n::tr(label).to_string(),
        detail: None,
        icon: None,
        checked: None,
        enabled: true,
        action: Some(rank),
        confirm: None,
    };
    let mut entries = vec![
        MenuEntry::Header(crate::i18n::tr("Silence").to_string()),
        end(0, "1 heure"),
    ];
    // Le soir, « jusqu'à ce soir » est déjà passé.
    let soir = chrono::Timelike::hour(&chrono::Local::now());
    if soir < crate::notifications::EVENING_HOUR {
        entries.push(end(1, "Jusqu'à ce soir"));
    }
    entries.push(end(2, "Jusqu'à désactivation"));
    entries
}

/// Commande de l'entrée `rank` de `silence_menu`.
pub(in crate::ui) fn silence_command(rank: i64) -> String {
    match rank {
        0 => "dnd on 1h",
        1 => "dnd on soir",
        _ => "dnd on",
    }
    .to_string()
}

/// Abonnement de la fin du Silence (HIG PAT-04) : un message à l'échéance.
/// Le temps monotone s'arrête en veille : le réveil est borné et l'heure
/// relue à l'horloge murale à chaque tour.
pub(super) fn silence_ends(until: i64) -> impl Stream<Item = TMsg> {
    iced::stream::channel(
        1,
        async move |mut out: iced::futures::channel::mpsc::Sender<TMsg>| {
            const BOUND: i64 = 300;
            loop {
                let left = until - crate::notifications::unix_now();
                if left <= 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(left.min(BOUND) as u64)).await;
            }
            let _ = out.send(TMsg::Expire).await;
        },
    )
}

impl Toasts {
    /// Écrit l'état lu par le module de barre et `hyprdesk notifications`.
    fn publish(&self) {
        if self.active {
            let _ = std::fs::write(
                crate::notifications::state_path(),
                crate::notifications::render_state(self.dnd, self.dnd_until, &self.history),
            );
        }
    }

    /// Bascule le Silence et le garde d'une session à l'autre.
    fn set_silence(&mut self, on: bool, until: Option<i64>) {
        self.dnd = on;
        self.dnd_until = on.then_some(until).flatten();
        if on {
            crate::notifications::Silence { until }.save();
        } else {
            crate::notifications::Silence::off();
        }
    }

    /// Éteint un Silence dont l'échéance est passée, à l'horloge murale.
    fn silence_expire(&mut self) -> bool {
        if !self.dnd
            || !self
                .dnd_until
                .is_some_and(|t| t <= crate::notifications::unix_now())
        {
            return false;
        }
        self.set_silence(false, None);
        self.publish();
        true
    }

    /// Applique une commande `hyprdesk notifications …` : `dnd [on [1h|soir]
    /// | off]`, `clear`. Sans mot de fin, le Silence dure jusqu'à
    /// désactivation (HIG PAT-04).
    pub fn command(&mut self, cmd: &str) {
        let mut it = cmd.split_whitespace();
        match (it.next(), it.next(), it.next()) {
            (Some("dnd"), Some("off"), _) => self.set_silence(false, None),
            (Some("dnd"), Some("on"), word) => {
                let until = match word {
                    None => None,
                    Some(w) => {
                        match SilenceEnd::parse(w).and_then(|e| e.deadline(chrono::Local::now())) {
                            Some(t) => Some(t),
                            None => return,
                        }
                    }
                };
                self.set_silence(true, until);
            }
            (Some("dnd"), None, _) => self.set_silence(!self.dnd, None),
            (Some("clear"), None, _) => self.history.clear(),
            _ => return,
        }
        self.publish();
    }
}

fn height(n: usize) -> u32 {
    height_of(n, VISIBLE)
}

fn height_of(n: usize, visible: usize) -> u32 {
    let n = n.min(visible.max(1)) as f32;
    (n * CARD + (n - 1.0).max(0.0) * GAP + 2.0 * SHADOW).ceil() as u32
}

/// Notifications réglées (MLG-279) : largeur, nombre visible, coin,
/// sens d'empilement, durée.
struct Layout {
    width: f32,
    visible: usize,
    corner: String,
    up: bool,
    timeout: Duration,
}

impl Layout {
    fn of(g: &crate::config::General) -> Self {
        use crate::design_options::{choice, num};
        Self {
            width: num(g, "notifications.width") as f32,
            visible: num(g, "notifications.max_visible").max(1.0) as usize,
            corner: choice(g, "notifications.corner"),
            up: choice(g, "notifications.stack") == "up",
            timeout: Duration::from_secs(num(g, "notifications.timeout").max(1.0) as u64),
        }
    }
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
                // Une échéance passée n'est pas relayée (HIG PAT-04).
                let silence = crate::notifications::Silence::load();
                self.toasts.dnd = silence.is_some();
                self.toasts.dnd_until = silence.and_then(|s| s.until);
                self.toasts.publish();
                Task::none()
            }
            TMsg::Expire => {
                self.toasts.silence_expire();
                Task::none()
            }
            TMsg::Event(Event::Shown(n)) => {
                // Le Silence a peut-être expiré pendant une veille.
                self.toasts.silence_expire();
                if !n.transient {
                    self.toasts.history.retain(|h| h.id != n.id);
                    self.toasts.history.insert(0, n.clone());
                    self.toasts.history.truncate(crate::notifications::HISTORY);
                    self.toasts.publish();
                }
                // Icône de l'application émettrice, pour la bulle et le centre :
                // la sienne, sinon celle de son application (HIG ICO-03).
                // Demandée aussi sous Silence : le centre montre la carte.
                let app_icon = self.want_notif_icons(&n);
                // Basse urgence (HIG PAT-06) ou ne pas déranger : dans
                // l'historique seulement, sauf critique.
                if n.urgency == Urgency::Low || (self.toasts.dnd && n.urgency != Urgency::Critical)
                {
                    return app_icon;
                }
                // Urgence critique : son repère doit être prêt dès la première image.
                let alert = if n.urgency == Urgency::Critical {
                    self.want_icon(URGENT_ICON)
                } else {
                    Task::none()
                };
                let t = Toast {
                    n,
                    since: Instant::now(),
                };
                match self.toasts.list.iter_mut().find(|o| o.n.id == t.n.id) {
                    Some(o) => *o = t,
                    None => self.toasts.list.insert(0, t),
                }
                let icon = self.want_icon(ICON);
                Task::batch([icon, app_icon, alert, self.sync_toasts()])
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
            // Carte du centre : ce qu'elle ouvre, puis le centre se referme.
            TMsg::Card(id) => {
                let click = self
                    .toasts
                    .history
                    .iter()
                    .find(|n| n.id == id)
                    .and_then(card_click);
                let close = self.close_popover();
                match click {
                    Some(CardClick::Default) => Task::batch([
                        close,
                        self.toasts_update(TMsg::Invoke(id, "default".into())),
                    ]),
                    Some(CardClick::Open(entry)) => Task::batch([close, open_app(entry)]),
                    None => close,
                }
            }
            TMsg::Close(id) => {
                send(Cmd::Dismiss(id, Reason::Dismissed));
                self.toasts.list.retain(|t| t.n.id != id);
                self.sync_toasts()
            }
            TMsg::Tick => {
                let default = Layout::of(&self.cfg.general).timeout;
                let expired: Vec<u32> = self
                    .toasts
                    .list
                    .iter()
                    .filter(|t| {
                        t.lifetime_or(default)
                            .is_some_and(|l| t.since.elapsed() >= l)
                    })
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
            (Some(win), n) => {
                let l = Layout::of(&self.cfg.general);
                Task::done(Msg::SizeChange {
                    id: win,
                    size: ((l.width + 2.0 * SHADOW) as u32, height_of(n, l.visible)),
                })
            }
            (None, n) => {
                let win = Id::unique();
                self.toasts.win = Some(win);
                let l = Layout::of(&self.cfg.general);
                // Sous la barre si elle est sur le même bord.
                let bar = |p: crate::config::BarPosition| match &self.cfg.bar {
                    Some(b) if b.position == p => b.height as i32,
                    _ => 0,
                };
                let (anchor, margin) = match l.corner.as_str() {
                    "top-left" => (
                        LsAnchor::Top | LsAnchor::Left,
                        (bar(crate::config::BarPosition::Top), 0, 0, 0),
                    ),
                    "bottom-right" => (
                        LsAnchor::Bottom | LsAnchor::Right,
                        (0, 0, bar(crate::config::BarPosition::Bottom), 0),
                    ),
                    "bottom-left" => (
                        LsAnchor::Bottom | LsAnchor::Left,
                        (0, 0, bar(crate::config::BarPosition::Bottom), 0),
                    ),
                    "top-center" => (
                        LsAnchor::Top,
                        (bar(crate::config::BarPosition::Top), 0, 0, 0),
                    ),
                    _ => (
                        LsAnchor::Top | LsAnchor::Right,
                        (bar(crate::config::BarPosition::Top), 0, 0, 0),
                    ),
                };
                Task::done(Msg::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some(((l.width + 2.0 * SHADOW) as u32, height_of(n, l.visible))),
                        layer: Layer::Overlay,
                        anchor,
                        exclusive_zone: Some(-1),
                        margin: Some(margin),
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

    /// Icône de repli d'une notification qui n'en apporte pas : celle de son
    /// application, cherchée parmi les applications installées d'après son
    /// `desktop-entry` (à défaut son nom), comme une classe de fenêtre
    /// (HIG ICO-03).
    fn class_icon_name(&self, n: &Notification) -> Option<String> {
        let class = n
            .desktop_entry
            .as_deref()
            .and_then(app_id)
            .or_else(|| app_id(&n.app))?;
        self.apps
            .iter()
            .find(|a| {
                a.wm_class
                    .as_deref()
                    .is_some_and(|w| w.eq_ignore_ascii_case(class))
                    || a.id.eq_ignore_ascii_case(class)
            })
            .and_then(|a| a.icon.clone())
            .filter(|icon| !icon.is_empty())
    }

    /// Nom de l'icône à montrer pour une notification : la sienne, sinon
    /// celle de son application ; `None` quand aucune.
    fn notif_icon_name(&self, n: &Notification) -> Option<String> {
        (!n.icon.is_empty())
            .then(|| n.icon.clone())
            .or_else(|| self.class_icon_name(n))
    }

    /// Demande l'icône d'une notification à son arrivée : la sienne, ou
    /// celle de son application ; puis celles de ses actions (HIG SYS-18).
    pub(super) fn want_notif_icons(&mut self, n: &Notification) -> Task<Msg> {
        let icon = self
            .notif_icon_name(n)
            .map(|name| self.want_icon(&name))
            .into_iter();
        let actions = n
            .action_icons
            .iter()
            .flatten()
            .map(|name| self.want_icon(name))
            .collect::<Vec<_>>();
        Task::batch(icon.chain(actions))
    }

    /// Icône de l'application émettrice, `size` de côté. À défaut, celle de
    /// son application ; la cloche générique sert de dernier repli et occupe
    /// la même place : le texte ne bouge pas quand l'icône arrive.
    pub(super) fn notif_icon<'a>(
        &'a self,
        n: &Notification,
        size: f32,
        tint: Color,
    ) -> Element<'a, Msg> {
        let name = self.notif_icon_name(n).unwrap_or_default();
        self.icon_named(&name, size, tint).unwrap_or_else(|| {
            container(
                self.icon_named(ICON, 18.0, tint)
                    .unwrap_or_else(|| iced::widget::Space::new().width(18).into()),
            )
            .width(size)
            .align_x(Alignment::Center)
            .into()
        })
    }

    fn view_toast<'a>(&'a self, t: &'a Toast) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let convex = self.convex_options().is_some();
        let fg = color(if convex { self.bar_palette().fg } else { th.fg }, 1.0);
        let dim = Color { a: DIM, ..fg };
        let bg = color(
            th.bg,
            th.shown(crate::config::Place::Menus).opacity.max(0.97),
        );
        // Urgence critique : le ton « attention » porte le filet et le losange
        // de l'urgence. L'accent reste à la sélection et aux actions, et la
        // forme du repère dit l'urgence sans la couleur.
        let critical = t.n.urgency == Urgency::Critical;
        let warning = crate::ui::tone::warning();
        let edge = if critical {
            warning
        } else {
            Color { a: 0.12, ..fg }
        };
        let id = t.n.id;
        // Icône de l'application, en grand au bord gauche de la carte.
        let glyph: Element<'_, Msg> = self.notif_icon(&t.n, APP_ICON, dim);
        let mut head = row![].spacing(8).align_y(Alignment::Center);
        if critical {
            head = head.push(
                self.icon_named(URGENT_ICON, CRITICAL_MARK, warning)
                    .unwrap_or_else(|| {
                        iced::widget::Space::new()
                            .width(CRITICAL_MARK)
                            .height(CRITICAL_MARK)
                            .into()
                    }),
            );
        }
        let head = head
            .push(
                text(if t.n.app.is_empty() {
                    crate::i18n::tr("Notification").to_string()
                } else {
                    t.n.app.clone()
                })
                .font(self.font)
                .size(crate::ui::tokens::ts(11.0))
                .color(dim)
                .width(Length::Fill),
            )
            .push(
                button(
                    // Fermeture : le glyphe du jeu, comme les menus (HIG ICO-07).
                    crate::ui::icons::glyph("window-close", crate::ui::tokens::ts(14.0), dim)
                        .expect("glyphe de fermeture"),
                )
                .padding([0, 6])
                .style(button::text)
                .on_press(Msg::Toasts(TMsg::Close(id))),
            );
        let mut col = column![
            head,
            text(t.n.summary.clone())
                .font(self.title_font())
                .size(crate::ui::tokens::ts(14.0))
                .color(fg)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(4)
        .width(Length::Fill);
        if !t.n.body.is_empty() {
            // Deux lignes au plus (la carte a une hauteur fixe) ; rien du corps
            // quand les aperçus sont masqués (HIG SYS-05).
            let hide =
                crate::design_options::flag(&self.cfg.general, "notifications.hide_previews");
            let body: String = if hide {
                crate::i18n::tr("Notification").to_string()
            } else {
                t.n.body.lines().take(2).collect::<Vec<_>>().join("\n")
            };
            col = col.push(
                text(body)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .line_height(crate::ui::tokens::line_height())
                    .color(dim),
            );
        }
        let extra: Vec<(usize, &(String, String))> =
            t.n.actions
                .iter()
                .enumerate()
                .filter(|(_, (k, _))| k != "default")
                .take(3)
                .collect();
        if !extra.is_empty() {
            // Boutons sans bordure : 12 px d'écart au moins (HIG ACC-10).
            let mut r = row![].spacing(12);
            for (i, (k, label)) in extra {
                let caption = text(label.clone())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(11.0));
                // Icône de l'action (hint `action-icons`) après le libellé
                // (HIG SYS-18).
                let icon =
                    t.n.action_icons
                        .get(i)
                        .and_then(|icon| icon.as_deref())
                        .and_then(|name| self.icon_named(name, crate::ui::tokens::ts(12.0), fg));
                let content: Element<'_, Msg> = match icon {
                    Some(icon) => row![caption, icon]
                        .spacing(6)
                        .align_y(Alignment::Center)
                        .into(),
                    None => caption.into(),
                };
                r = r.push(
                    button(content)
                        .padding([2, 10])
                        .on_press(Msg::Toasts(TMsg::Invoke(id, k.clone()))),
                );
            }
            col = col.push(r);
        }
        let card = container(row![glyph, col].spacing(12).align_y(Alignment::Start))
            .padding([10, 14])
            .width(Layout::of(&self.cfg.general).width)
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

    /// Carte Notifications de la barre : les dernières notifications. Titre
    /// dans la couleur du texte, application et corps en secondaire.
    pub(super) fn view_notification_history(&self, alpha: f32) -> Element<'_, Msg> {
        let fg = color(self.bar_palette().fg, alpha);
        let dim = Color {
            a: alpha * DIM,
            ..fg
        };
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
        // Aperçus masqués (HIG SYS-05) : un texte générique au lieu du corps.
        let hide = crate::design_options::flag(&self.cfg.general, "notifications.hide_previews");
        for n in self.toasts.history.iter().take(30) {
            let (title, body) = card_text(n, hide);
            let card: Element<'_, Msg> = row![
                self.notif_icon(n, CARD_ICON, fg),
                column![
                    text(n.app.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(11.0))
                        .color(dim),
                    text(title)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(14.0))
                        .color(fg),
                    body.map(|body| {
                        text(body)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                            .line_height(crate::ui::tokens::line_height())
                            .color(dim)
                    })
                ]
                .spacing(4)
                .width(Length::Fill),
            ]
            .spacing(12)
            .align_y(Alignment::Start)
            .into();
            // Carte cliquable quand elle mène quelque part : l'action de
            // l'application, ou l'application elle-même.
            col = col.push(match card_press(n) {
                Some(press) => mouse_area(card)
                    .interaction(iced::mouse::Interaction::Pointer)
                    .on_press(press)
                    .into(),
                None => card,
            });
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
        let l = Layout::of(&self.cfg.general);
        let shown: Vec<&Toast> = visible.iter().take(l.visible).collect();
        // Empilement vers le haut : la plus récente en bas.
        if l.up {
            for t in shown.into_iter().rev() {
                col = col.push(self.view_toast(t));
            }
        } else {
            for t in shown {
                col = col.push(self.view_toast(t));
            }
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

    #[test]
    fn l_icone_de_l_application_est_demandee_meme_sous_silence() {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-toast-tests/config.lua"),
            true,
        );
        let demandee = |app: &App, nom: &str| {
            app.named_icons.contains_key(nom) || app.named_pending.contains(nom)
        };
        let n = crate::notifications::build(
            7,
            "Firefox",
            "firefox",
            "s",
            "b",
            &[],
            &HashMap::new(),
            -1,
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(n)));
        assert!(
            demandee(&app, "firefox"),
            "l'icône de l'application doit être demandée"
        );

        // Sous Silence : pas de bulle, mais le centre montre la carte.
        app.toasts.dnd = true;
        let n = crate::notifications::build(8, "a", "calendar", "s", "b", &[], &HashMap::new(), -1);
        let _ = app.toasts_update(TMsg::Event(Event::Shown(n)));
        assert_eq!(app.toasts.list.len(), 1, "Silence : pas de nouvelle bulle");
        assert_eq!(app.toasts.history.len(), 2, "Silence : l'historique garde");
        assert!(
            demandee(&app, "calendar"),
            "l'icône est demandée sous Silence aussi"
        );
    }

    /// Sans icône, une notification prend celle de son application, cherchée
    /// par son `desktop-entry`, sinon par son nom (HIG ICO-03).
    #[test]
    fn une_notification_sans_icone_prend_celle_de_son_application() {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-toast-tests/config.lua"),
            true,
        );
        app.apps = std::sync::Arc::new(vec![crate::launch::AppInfo {
            id: "firefox".into(),
            name: "Firefox".into(),
            icon: Some("firefox".into()),
            wm_class: Some("Navigator".into()),
            mime_types: Vec::new(),
            categories: Vec::new(),
            available: true,
        }]);
        let demandee = |app: &App, nom: &str| {
            app.named_icons.contains_key(nom) || app.named_pending.contains(nom)
        };
        let hints = HashMap::from([(
            "desktop-entry".to_string(),
            zbus::zvariant::OwnedValue::try_from(zbus::zvariant::Value::from("firefox")).unwrap(),
        )]);
        let par_entree =
            crate::notifications::build(9, "Navigateur", "", "s", "b", &[], &hints, -1);
        let _ = app.toasts_update(TMsg::Event(Event::Shown(par_entree.clone())));
        assert!(
            demandee(&app, "firefox"),
            "l'icône de l'application est demandée à la place"
        );
        assert_eq!(app.notif_icon_name(&par_entree).as_deref(), Some("firefox"));

        // Sans `desktop-entry`, le nom de l'application sert de classe.
        let par_nom =
            crate::notifications::build(10, "Navigator", "", "s", "b", &[], &HashMap::new(), -1);
        assert_eq!(app.notif_icon_name(&par_nom).as_deref(), Some("firefox"));

        // L'icône envoyée reste prioritaire ; une application inconnue n'en
        // impose aucune.
        let propre =
            crate::notifications::build(11, "a", "/tmp/a.png", "s", "b", &[], &HashMap::new(), -1);
        assert_eq!(app.notif_icon_name(&propre).as_deref(), Some("/tmp/a.png"));
        let inconnue =
            crate::notifications::build(12, "Inconnue", "", "s", "b", &[], &HashMap::new(), -1);
        assert_eq!(app.notif_icon_name(&inconnue), None);

        // Icônes des actions (HIG SYS-18) : demandées à l'arrivée.
        let mut icons = HashMap::new();
        icons.insert(
            "action-icons".to_string(),
            zbus::zvariant::OwnedValue::try_from(zbus::zvariant::Value::from(vec![
                "-",
                "mail-reply-sender",
            ]))
            .unwrap(),
        );
        let actions = [
            "default".to_string(),
            "Ouvrir".to_string(),
            "reply".to_string(),
            "Répondre".to_string(),
        ];
        let avec = crate::notifications::build(13, "a", "i", "s", "b", &actions, &icons, -1);
        assert_eq!(
            avec.action_icons,
            vec![None, Some("mail-reply-sender".to_string())]
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(avec)));
        assert!(
            demandee(&app, "mail-reply-sender"),
            "l'icône de l'action est demandée"
        );
    }

    /// Titre et corps d'une carte sont bornés, sans saut de ligne (HIG LAY-04).
    #[test]
    fn une_carte_borne_titre_et_corps() {
        let longue = "é".repeat(100);
        let corps = format!("{}\n{longue}", "b".repeat(30));
        let n = crate::notifications::build(1, "a", "i", &longue, &corps, &[], &HashMap::new(), -1);
        let (titre, corps) = card_text(&n, false);
        assert_eq!(titre.chars().count(), TITLE_CHARS + 1);
        assert!(titre.ends_with('…'));
        let corps = corps.expect("un corps");
        assert_eq!(corps.chars().count(), BODY_CHARS + 1);
        assert!(!corps.contains('\n') && corps.ends_with('…'));
        let vide = crate::notifications::build(2, "a", "i", "t", "", &[], &HashMap::new(), -1);
        assert_eq!(card_text(&vide, false), ("t".to_string(), None));

        // Aperçus masqués (HIG SYS-05) : le titre reste, le corps devient
        // générique ; rien à masquer dans un corps vide.
        let (titre, corps) = card_text(&n, true);
        assert_eq!(titre.chars().count(), TITLE_CHARS + 1);
        assert_eq!(corps.as_deref(), Some(crate::i18n::tr("Notification")));
        assert_eq!(card_text(&vide, true), ("t".to_string(), None));
    }

    #[test]
    fn une_notification_basse_va_a_l_historique_sans_bulle() {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-toast-tests/config.lua"),
            true,
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(toast(0, -1).n)));
        assert!(app.toasts.list.is_empty(), "basse urgence : pas de bulle");
        assert_eq!(
            app.toasts.history.len(),
            1,
            "basse urgence : l'historique garde"
        );
        // Hors Silence, la notification normale garde sa bulle.
        let _ = app.toasts_update(TMsg::Event(Event::Shown(toast(1, -1).n)));
        assert_eq!(app.toasts.list.len(), 1);
    }

    /// Une notification « a » avec ou sans action `default`, avec ou sans
    /// application d'origine (`desktop-entry`).
    fn carte(actions: &[&str], entry: Option<&str>) -> Notification {
        let mut h = HashMap::new();
        if let Some(e) = entry {
            h.insert(
                "desktop-entry".to_string(),
                zbus::zvariant::OwnedValue::try_from(zbus::zvariant::Value::from(e)).unwrap(),
            );
        }
        let acts: Vec<String> = actions.iter().map(|s| s.to_string()).collect();
        crate::notifications::build(1, "a", "", "s", "b", &acts, &h, -1)
    }

    #[test]
    fn un_clic_sur_une_carte_du_centre_sait_ou_mener() {
        // L'action `default` de l'application passe avant tout.
        assert_eq!(
            card_click(&carte(&["default", "Ouvrir"], Some("firefox"))),
            Some(CardClick::Default)
        );
        // Sans elle : l'application d'origine, suffixe `.desktop` toléré.
        assert_eq!(
            card_click(&carte(&[], Some("firefox.desktop"))),
            Some(CardClick::Open("firefox".into()))
        );
        assert_eq!(
            card_click(&carte(&["reply", "Répondre"], Some("org.mozilla.firefox"))),
            Some(CardClick::Open("org.mozilla.firefox".into()))
        );
        // Ni action ni application connue : rien à ouvrir, la carte n'est pas
        // cliquable. Un chemin ne désigne jamais une application installée.
        assert_eq!(card_click(&carte(&[], None)), None);
        assert_eq!(card_click(&carte(&[], Some(""))), None);
        assert_eq!(
            card_click(&carte(&[], Some("../../partage/piege.desktop"))),
            None
        );
        assert!(card_press(&carte(&[], None)).is_none());
        assert!(matches!(
            card_press(&carte(&[], Some("firefox"))),
            Some(Msg::Toasts(TMsg::Card(1)))
        ));
    }

    #[test]
    fn la_carte_du_centre_lance_l_action_par_defaut() {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-toast-tests/config.lua"),
            true,
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(carte(
            &["default", "Ouvrir"],
            Some("firefox"),
        ))));
        assert_eq!(app.toasts.list.len(), 1);
        assert_eq!(app.toasts.history.len(), 1);
        // Un clic sur la carte vaut un clic sur la bulle : l'action est
        // demandée, la bulle se ferme, l'historique garde la carte.
        let _ = app.toasts_update(TMsg::Card(1));
        assert!(app.toasts.list.is_empty());
        assert_eq!(app.toasts.history.len(), 1);
    }

    #[test]
    fn seule_une_notification_critique_demande_son_repere() {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-toast-tests/config.lua"),
            true,
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(toast(1, -1).n)));
        assert!(!app.named_icons.contains_key(URGENT_ICON));
        let _ = app.toasts_update(TMsg::Event(Event::Shown(toast(2, -1).n)));
        assert!(
            app.named_icons.contains_key(URGENT_ICON),
            "le repère de l'urgence critique est prêt dès la première image"
        );
    }
}

#[cfg(test)]
mod silence_tests {
    use super::{silence_command, silence_menu};

    /// HIG PAT-04 : la fin du Silence se choisit à l'activation. L'action
    /// d'une entrée est son rang, quand bien même « ce soir » manque.
    #[test]
    fn la_fin_du_silence_se_choisit_a_l_activation() {
        let items: Vec<(String, i64)> = silence_menu()
            .iter()
            .filter_map(|e| match e {
                crate::config::MenuEntry::Item {
                    label,
                    action: Some(a),
                    ..
                } => Some((label.clone(), *a)),
                _ => None,
            })
            .collect();
        let mut attendu = vec![("1 heure", 0)];
        let soir =
            chrono::Timelike::hour(&chrono::Local::now()) < crate::notifications::EVENING_HOUR;
        if soir {
            attendu.push(("Jusqu'à ce soir", 1));
        }
        attendu.push(("Jusqu'à désactivation", 2));
        let attendu: Vec<(String, i64)> = attendu
            .into_iter()
            .map(|(l, a)| (crate::i18n::tr(l).to_string(), a))
            .collect();
        assert_eq!(items, attendu, "soir : {soir}");
        assert_eq!(silence_command(0), "dnd on 1h");
        assert_eq!(silence_command(1), "dnd on soir");
        assert_eq!(silence_command(2), "dnd on");
    }
}
