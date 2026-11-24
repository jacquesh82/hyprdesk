//! Bulles de notifications : le démon publie le serveur
//! `org.freedesktop.Notifications` (`crate::notifications`) par un
//! abonnement et affiche chaque notification reçue dans une carte du thème,
//! en haut à droite de l'écran actif, sous la barre. Un clic lance l'action
//! par défaut, les boutons les autres actions, ✕ ferme ; la bulle s'efface
//! d'elle-même après son délai (jamais pour une notification critique).
//! Une notification de basse urgence ne montre pas de bulle : livrée
//! discrètement, elle va à l'historique seul, comme sous Silence.
//! La première notification critique d'une application explique, dans sa
//! bulle, qu'elle traverse le Silence, et permet de le refuser pour cette
//! application (`crate::notifications::Passage`).
//! La première notification d'une application demande, dans sa bulle,
//! l'autorisation de lui en envoyer ; refusée, l'application n'est plus remise
//! (`crate::notifications::Authorizations`). Accordée, son style de remise
//! s'applique à ses notifications : bannières, alertes (la bulle reste) ou
//! aucune (l'historique seul).
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
use crate::notifications::{
    Authorization, Authorizations, Delivery, Event, Notification, Passage, Reason, SilenceEnd,
    Urgency,
};
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
pub(super) const ICON: &str = "preferences-system-notifications-symbolic";
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
/// Cible tactile des boutons d'une bulle (HIG ACC-04) : 28 px — de côté pour
/// la fermeture, de haut au moins pour les actions.
const BUTTON_TARGET: f32 = 28.0;
/// Lignes du texte qui explique le passage du Silence (HIG PAT-05) : deux, le
/// bouton de refus doit rester lisible dans la carte de hauteur fixe.
const EXPLAIN_LINES: f32 = 2.0;

/// Hauteur de ces lignes, à la taille du corps d'une bulle.
fn explain_text_height() -> f32 {
    EXPLAIN_LINES
        * crate::ui::tokens::line_height()
            .to_absolute(iced::Pixels(crate::ui::tokens::ts(12.0)))
            .0
}

/// Surcroît de la carte d'une bulle qui explique le passage du Silence (HIG
/// PAT-05) : ses deux lignes, un espacement de colonne, le bouton de refus
/// (28 px, HIG ACC-04), un autre espacement.
fn explain_height() -> f32 {
    8.0 + explain_text_height() + crate::ui::tokens::ts(BUTTON_TARGET)
}
/// Notifications de l'historique montrées dans la carte de la barre et le
/// volet de Panorama (le centre Continuité en montre 12).
pub(super) const HISTORY_SHOWN: usize = 30;
/// Fond d'une boîte d'application de l'historique (HIG LAY-02), dans la carte
/// de la barre : la couleur du texte courant à cette opacité.
const GROUP_BOX: f32 = 0.06;

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
    /// Demande d'autorisation (HIG PAT-02) que porte la bulle : la clé de
    /// l'application qui n'a encore aucune décision. Jusqu'à la réponse ou à
    /// la fermeture, la bulle reste et sa notification n'est pas remise.
    pub request: Option<String>,
    /// Style de remise « Alertes » de l'application (HIG SYS-03) : la bulle
    /// reste jusqu'à sa fermeture, comme une critique.
    pub alert: bool,
}

/// Ce que devient, pour l'interface, une notification reçue d'une application
/// (HIG PAT-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reception {
    /// Remise, accordée avec le style de remise de l'application — ou sans le
    /// service (aperçu, autre démon), où rien ne se décide.
    Delivered(Delivery),
    /// Refusée : fermée pour l'application, ni bulle ni historique.
    Refused,
    /// Sans décision : sa bulle porte la demande d'autorisation.
    Asked,
}

impl Toast {
    /// Délai d'affichage ; `None` : reste jusqu'à ce qu'on la ferme.
    pub fn lifetime(&self) -> Option<Duration> {
        self.lifetime_or(DEFAULT_TIMEOUT)
    }

    /// Délai d'affichage, `default` quand l'application n'en demande pas
    /// (réglé : `notifications.timeout`, MLG-279). `None` pour une critique et
    /// pour le style « Alertes » (HIG SYS-03).
    pub fn lifetime_or(&self, default: Duration) -> Option<Duration> {
        if self.n.urgency == Urgency::Critical || self.alert {
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

/// Clé d'application du passage du Silence (HIG PAT-05) : l'identifiant
/// `desktop-entry` quand il en est un, sinon le nom de l'émetteur. Ceux qui ne
/// donnent ni l'un ni l'autre n'en forment qu'un, comme dans l'historique.
pub(super) fn passage_key(n: &Notification) -> String {
    app_id(n.desktop_entry.as_deref().unwrap_or_default())
        .map_or_else(|| n.app.clone(), str::to_string)
        .replace(['\n', '\r'], " ")
}

/// Explication du passage du Silence (HIG PAT-05), portée par la bulle `id` de
/// la première notification critique de l'application `key`. Elle ne se lit que
/// sur cette bulle : partie, elle ne s'applique plus à rien.
#[derive(Debug, Clone, PartialEq)]
pub struct Explain {
    pub key: String,
    pub id: u32,
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

/// La carte du centre mène quelque part : son menu contextuel (HIG ENT-07)
/// porte « Ouvrir » ; sans quoi l'entrée se retire.
pub(super) fn card_openable(n: &Notification) -> bool {
    card_click(n).is_some()
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

/// En-tête du groupe d'une application dans l'historique (HIG LAY-02) : son
/// nom, « Notification » quand elle n'en donne pas, comme la tête d'une bulle.
pub(super) fn card_app(n: &Notification) -> String {
    if n.app.is_empty() {
        crate::i18n::tr("Notification").to_string()
    } else {
        n.app.clone()
    }
}

/// Groupes d'application d'un historique (les plus récentes d'abord, HIG
/// LAY-02) : l'en-tête du groupe et, dans l'ordre de l'historique, le rang de
/// chacune de ses notifications. Les groupes suivent l'ordre de leur
/// notification la plus récente. Une application se reconnaît à son en-tête
/// (`card_app`) : celles qui ne donnent pas de nom n'en forment qu'un.
pub(super) fn app_groups(history: &[Notification]) -> Vec<(String, Vec<usize>)> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, n) in history.iter().enumerate() {
        let name = card_app(n);
        match groups.iter_mut().find(|(group, _)| *group == name) {
            Some((_, ranks)) => ranks.push(i),
            None => groups.push((name, vec![i])),
        }
    }
    groups
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
    /// Clic droit sur une carte du centre de notifications : son menu court
    /// (HIG ENT-07).
    Menu(u32),
    Close(u32),
    /// Refus du passage du Silence pour l'application de la bulle (HIG PAT-05).
    Refuse(u32),
    /// Réponse de la bulle à la demande d'autorisation de son application
    /// (HIG PAT-02) : « Autoriser », « Refuser ».
    Allow(u32),
    Deny(u32),
    Tick,
    Shown(Id),
    /// La fin programmée du Silence est atteinte (HIG PAT-04).
    Expire,
    /// Une liste de l'historique s'ouvre : tout est lu (HIG SYS-02).
    Read,
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
    /// Explications du passage du Silence (HIG PAT-05) portées par les bulles
    /// ouvertes : chacune, celle de la première notification critique de son
    /// application.
    pub explain: Vec<Explain>,
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

/// Teinte du libellé et de l'icône d'un bouton d'action : le ton « danger »
/// pour une action destructrice (HIG SYS-17), celui des autres actions sinon.
fn action_tone(destructive: bool, fg: Color) -> Color {
    if destructive {
        crate::ui::tone::danger()
    } else {
        fg
    }
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

    /// Notifications non lues de l'historique (HIG SYS-02).
    pub fn unread(&self) -> usize {
        self.history.iter().filter(|n| n.unread).count()
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

    /// Refus du passage du Silence (HIG PAT-05) : l'explication quitte la
    /// bulle `id` ; `serving`, hyprdesk tient le service de notifications et
    /// garde la décision (l'aperçu, un autre démon : rien ne se garde). Vrai
    /// quand la bulle portait une explication.
    fn refuse(&mut self, id: u32, serving: bool) -> bool {
        let Some(e) = self.explain.extract_if(.., |e| e.id == id).next() else {
            return false;
        };
        if serving {
            Passage::load().set(&e.key, false).save();
        }
        true
    }

    /// Décision qui s'applique à une notification de l'application `key` (HIG
    /// PAT-02), avec son style de remise quand elle est accordée (HIG
    /// SYS-03). Décide celui qui tient le service de notifications : sans lui
    /// (aperçu, autre démon), rien ne se décide.
    fn reception(&self, key: &str) -> Reception {
        if !self.active {
            return Reception::Delivered(Delivery::default());
        }
        match Authorizations::load().get(key) {
            Some(Authorization {
                granted: true,
                delivery,
            }) => Reception::Delivered(delivery),
            Some(_) => Reception::Refused,
            None => Reception::Asked,
        }
    }

    /// Une notification remise entre dans l'historique, non lue ; un
    /// remplacement (même id) change son contenu : non lue à nouveau (HIG
    /// SYS-02). Une liste de la barre ouverte la montre : lue. Rien d'une
    /// notification `transient`.
    fn deliver(&mut self, n: &Notification, list_open: bool) {
        if n.transient {
            return;
        }
        self.history.retain(|h| h.id != n.id);
        let mut h = n.clone();
        h.unread = !list_open;
        self.history.insert(0, h);
        self.history.truncate(crate::notifications::HISTORY);
        self.publish();
    }

    /// Réponse à la demande d'autorisation que porte la bulle `id` (HIG
    /// PAT-02). `allow` : la demande quitte la bulle, qui devient une bulle
    /// ordinaire — son délai repart, sa notification entre dans l'historique.
    /// `serving`, hyprdesk tient le service et garde la décision (l'aperçu,
    /// un autre démon : rien ne se garde). Vrai quand la bulle portait une
    /// demande.
    fn answer(&mut self, id: u32, allow: bool, serving: bool, list_open: bool) -> bool {
        let Some(t) = self.list.iter_mut().find(|t| t.n.id == id) else {
            return false;
        };
        let Some(key) = t.request.take() else {
            return false;
        };
        let n = t.n.clone();
        if allow {
            t.since = Instant::now();
        }
        if serving {
            Authorizations::load()
                .set(
                    &key,
                    Authorization {
                        granted: allow,
                        delivery: Delivery::default(),
                    },
                )
                .save();
        }
        if allow {
            self.deliver(&n, list_open);
        }
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

/// Surcroît de la carte d'une bulle qui montre un corps et des actions : les
/// boutons ont 28 px (HIG ACC-04) et la carte de `CARD` ne suffit plus.
const CARD_BODY_ACTIONS: f32 = 20.0;

/// Hauteur de la carte d'une bulle : `CARD`, plus 20 px quand elle montre à la
/// fois un corps et des boutons d'action (HIG ACC-04), plus l'explication du
/// passage du Silence quand la bulle la porte (HIG PAT-05), plus la demande
/// d'autorisation quand elle la porte (HIG PAT-02 : ses deux lignes et ses
/// deux boutons, la même mesure que l'explication).
fn card_height(t: &Toast, explained: bool) -> f32 {
    let n = &t.n;
    let actions = n.actions.iter().any(|(k, _)| k != "default");
    let mut h = CARD;
    if actions && !n.body.is_empty() {
        h += CARD_BODY_ACTIONS;
    }
    if explained {
        h += explain_height();
    }
    if t.request.is_some() {
        h += explain_height();
    }
    h
}

fn height(list: &[Toast], explain: &[Explain]) -> u32 {
    height_of(list, VISIBLE, explain)
}

/// Hauteur de la surface des bulles, chacune à la hauteur de sa carte.
fn height_of(list: &[Toast], visible: usize, explain: &[Explain]) -> u32 {
    let n = list.len().min(visible.max(1));
    let explained = |t: &Toast| explain.iter().any(|e| e.id == t.n.id);
    let cards: f32 = list
        .iter()
        .take(n)
        .map(|t| card_height(t, explained(t)))
        .sum();
    (cards + n.saturating_sub(1) as f32 * GAP + 2.0 * SHADOW).ceil() as u32
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

/// Bornes de la vue étendue (HIG SYS-13) : le titre tient sur deux lignes, le
/// corps sur huit ; au-delà, le détail reste dans l'application.
const ACTIVITY_TITLE_CHARS: usize = 120;
const ACTIVITY_BODY_CHARS: usize = 480;
const ACTIVITY_BODY_LINES: usize = 8;

/// Titre et corps de la vue étendue (HIG SYS-13). Aperçus masqués (HIG
/// SYS-05) : un texte générique au lieu du corps.
fn activity_text(n: &Notification, hide: bool) -> (String, Option<String>) {
    let body = if hide {
        crate::i18n::tr("Notification").to_string()
    } else {
        truncate(
            &n.body
                .lines()
                .take(ACTIVITY_BODY_LINES)
                .collect::<Vec<_>>()
                .join("\n"),
            ACTIVITY_BODY_CHARS,
        )
    };
    (
        truncate(&n.summary, ACTIVITY_TITLE_CHARS),
        (!n.body.is_empty()).then_some(body),
    )
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
    /// Une carte de la barre liste l'historique en ce moment — la carte
    /// Notifications, le centre Continuité —, ouverte et non en train de se
    /// refermer (HIG SYS-02) : ce qui y arrive est montré, donc lu.
    pub(super) fn history_list_open(&self) -> bool {
        self.popover.as_ref().is_some_and(|p| {
            !p.closing()
                && (matches!(
                    p.kind,
                    crate::ui::plugins::bar::popover::Kind::Notifications
                ) || p.dashboard.is_some())
        })
    }

    /// La notification de la vue étendue `id` (HIG SYS-13) : sa bulle vivante,
    /// sinon son entrée d'historique — la bulle s'est effacée depuis
    /// l'ouverture de la vue, qui continue de la montrer.
    fn activity_notification(&self, id: u32) -> Option<&Notification> {
        self.toasts
            .list
            .iter()
            .map(|t| &t.n)
            .chain(self.toasts.history.iter())
            .find(|n| n.id == id)
    }

    /// La vue étendue de la bulle `id` est ouverte, et non en train de se
    /// refermer (HIG SYS-13).
    fn activity_open(&self, id: u32) -> bool {
        self.popover.as_ref().is_some_and(|p| {
            !p.closing()
                && matches!(
                    p.kind,
                    crate::ui::plugins::bar::popover::Kind::Activity(i) if i == id
                )
        })
    }

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
            // Une liste de l'historique s'ouvre : tout est lu (HIG SYS-02).
            TMsg::Read => {
                if self.toasts.unread() > 0 {
                    for n in &mut self.toasts.history {
                        n.unread = false;
                    }
                    self.toasts.publish();
                }
                Task::none()
            }
            TMsg::Event(Event::Shown(n)) => {
                // Le Silence a peut-être expiré pendant une veille.
                self.toasts.silence_expire();
                // Une explication sans sa bulle (fermée) ne s'applique plus à rien.
                self.toasts
                    .explain
                    .retain(|e| self.toasts.list.iter().any(|t| t.n.id == e.id));
                // Décision de l'application (HIG PAT-02) : sans décision, sa
                // bulle porte la demande d'autorisation et sa notification
                // n'entre pas dans l'historique ; refusée, elle n'est pas
                // remise — fermée pour l'application aussitôt.
                let key = passage_key(&n);
                let mut request = None;
                // Style de remise de l'application accordée (HIG SYS-03).
                let mut stays = false;
                let mut silent = false;
                match self.toasts.reception(&key) {
                    Reception::Refused => {
                        send(Cmd::Dismiss(n.id, Reason::Expired));
                        return Task::none();
                    }
                    Reception::Asked => request = Some(key.clone()),
                    Reception::Delivered(delivery) => {
                        stays = delivery == Delivery::Alert;
                        silent = delivery == Delivery::Silent;
                        self.toasts.deliver(&n, self.history_list_open());
                    }
                }
                // Une notification critique garde ses règles : elle reste et
                // se montre, quel que soit le style de son application.
                let silent = silent && n.urgency != Urgency::Critical;
                // Icône de l'application émettrice, pour la bulle et le centre :
                // la sienne, sinon celle de son application (HIG ICO-03).
                // Demandée aussi sous Silence : le centre montre la carte.
                let app_icon = self.want_notif_icons(&n);
                // Passage du Silence (HIG PAT-05), application par
                // application. Première notification critique : accordé, et la
                // bulle l'explique, avec moyen de le refuser. La demande prime
                // (HIG PAT-02) : rien ne s'inscrit ni ne s'explique — ce sera
                // à la première critique qui suit l'autorisation.
                let passes = if n.urgency == Urgency::Critical && request.is_none() {
                    let mut passage = Passage::load();
                    let first = passage.granted(&key).is_none();
                    if first {
                        passage = passage.set(&key, true);
                        passage.save();
                    }
                    let passes = passage.granted(&key) != Some(false);
                    if first {
                        self.toasts.explain.push(Explain { key, id: n.id });
                    }
                    passes
                } else {
                    // Une demande critique passe le Silence comme une première
                    // critique.
                    n.urgency == Urgency::Critical
                };
                // Basse urgence (HIG PAT-06), style « Aucune » de l'application
                // (HIG SYS-03) ou ne pas déranger : dans l'historique
                // seulement, sauf critique dont l'application passe le
                // Silence.
                if n.urgency == Urgency::Low || silent || (self.toasts.dnd && !passes) {
                    // Sans bulle, une demande ne se pose pas : la notification,
                    // non remise, se ferme pour l'application — la suivante
                    // redemandera.
                    if request.is_some() {
                        send(Cmd::Dismiss(n.id, Reason::Expired));
                    }
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
                    request,
                    alert: stays,
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
                // La vue étendue de cette bulle (HIG SYS-13) se referme sur
                // son action.
                let close = if self.activity_open(id) {
                    self.close_popover()
                } else {
                    Task::none()
                };
                Task::batch([close, self.sync_toasts()])
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
            // Clic droit sur une carte du centre (HIG ENT-07) : son menu, au
            // curseur, dans la surface qui la montre — la carte Notifications
            // de la barre, le centre Continuité. Plus montrée, ou fermée
            // depuis : rien.
            TMsg::Menu(id) => {
                let shown = self
                    .popover
                    .as_ref()
                    .filter(|_| self.history_list_open())
                    .map(|p| (p.win, p.bar));
                let Some((win, bar)) = shown else {
                    return Task::none();
                };
                let monitor = self
                    .bar_surfaces
                    .get(&bar)
                    .map(|b| b.monitor.clone())
                    .unwrap_or_default();
                self.open_notification_menu(win, &monitor, id)
            }
            TMsg::Close(id) => {
                send(Cmd::Dismiss(id, Reason::Dismissed));
                self.toasts.list.retain(|t| t.n.id != id);
                // La vue étendue de cette bulle (HIG SYS-13) se referme avec
                // elle.
                let close = if self.activity_open(id) {
                    self.close_popover()
                } else {
                    Task::none()
                };
                Task::batch([close, self.sync_toasts()])
            }
            // La bulle refuse le passage du Silence à son application (HIG
            // PAT-05) : décision gardée, explication retirée.
            TMsg::Refuse(id) => {
                if self.toasts.refuse(id, self.toasts.active) {
                    self.sync_toasts()
                } else {
                    Task::none()
                }
            }
            // La bulle autorise son application (HIG PAT-02) : décision gardée,
            // la demande quitte la bulle, qui devient une bulle ordinaire — son
            // délai repart, sa notification entre dans l'historique.
            TMsg::Allow(id) => {
                if self
                    .toasts
                    .answer(id, true, self.toasts.active, self.history_list_open())
                {
                    self.sync_toasts()
                } else {
                    Task::none()
                }
            }
            // La bulle refuse son application (HIG PAT-02) : décision gardée,
            // et la bulle se ferme sans historique, comme « Fermer ».
            TMsg::Deny(id) => {
                if self.toasts.answer(id, false, self.toasts.active, false) {
                    return self.toasts_update(TMsg::Close(id));
                }
                Task::none()
            }
            TMsg::Tick => {
                let default = Layout::of(&self.cfg.general).timeout;
                let expired: Vec<u32> = self
                    .toasts
                    .list
                    .iter()
                    .filter(|t| {
                        // Une demande reste jusqu'à sa réponse ou à sa fermeture.
                        t.request.is_none()
                            && t.lifetime_or(default)
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
        let h = height(&self.toasts.list, &self.toasts.explain) + bh + 12;
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
            (Some(win), _) => {
                let l = Layout::of(&self.cfg.general);
                Task::done(Msg::SizeChange {
                    id: win,
                    size: (
                        (l.width + 2.0 * SHADOW) as u32,
                        height_of(&self.toasts.list, l.visible, &self.toasts.explain),
                    ),
                })
            }
            (None, _) => {
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
                        size: Some((
                            (l.width + 2.0 * SHADOW) as u32,
                            height_of(&self.toasts.list, l.visible, &self.toasts.explain),
                        )),
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
    pub(super) fn class_icon_name(&self, n: &Notification) -> Option<String> {
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
        self.icon_or_bell(&name, size, tint)
    }

    /// Icône nommée, `size` de côté ; la cloche générique sert de dernier
    /// repli et occupe la même place : le texte ne bouge pas quand l'icône
    /// arrive. Sert de même à l'icône d'une application dans les Réglages
    /// (HIG PAT-07).
    pub(super) fn icon_or_bell<'a>(
        &'a self,
        name: &str,
        size: f32,
        tint: Color,
    ) -> Element<'a, Msg> {
        self.icon_named(name, size, tint).unwrap_or_else(|| {
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
        let ink = if convex { self.bar_palette().fg } else { th.fg };
        let fg = color(ink, 1.0);
        // Texte secondaire : celle du thème, garantie à 4,5:1 sur le fond de
        // la bulle (HIG LAY-08).
        let dim = color(crate::theme::readable(ink, th.bg, th.accent).dim, 1.0);
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
            // Bouton à glyphe seul : son infobulle nomme l'action (HIG LAY-12).
            .push(iced::widget::tooltip(
                button(
                    // Fermeture : le glyphe du jeu, comme les menus (HIG ICO-07).
                    crate::ui::icons::glyph("window-close", crate::ui::tokens::ts(14.0), dim)
                        .expect("glyphe de fermeture"),
                )
                // Cible de 28 px de côté (HIG ACC-04) : 14 px de glyphe et
                // 7 px de marge de chaque côté.
                .padding(crate::ui::tokens::ts(7.0))
                .width(crate::ui::tokens::ts(BUTTON_TARGET))
                .height(crate::ui::tokens::ts(BUTTON_TARGET))
                .style(button::text)
                .on_press(Msg::Toasts(TMsg::Close(id))),
                text(crate::i18n::tr("Fermer")).font(self.font),
                iced::widget::tooltip::Position::Left,
            ));
        let mut col = column![head].spacing(4).width(Length::Fill);
        // Demande d'autorisation de l'application (HIG PAT-02) : au-dessus du
        // contenu de la notification, pour qu'on sache ce qu'on accorde.
        if t.request.is_some() {
            col = col.push(self.request_block(&t.n, fg));
        }
        col = col.push(
            text(t.n.summary.clone())
                .font(self.title_font())
                .size(crate::ui::tokens::ts(14.0))
                .color(fg)
                .wrapping(text::Wrapping::None),
        );
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
        if let Some(actions) = self.action_buttons(&t.n, fg) {
            col = col.push(actions);
        }
        // Première notification critique de l'application (HIG PAT-05) : la
        // bulle explique son passage du Silence et permet de le refuser. Deux
        // lignes au plus, comme le corps : le bouton reste dans la carte.
        let explained = self.toasts.explain.iter().any(|e| e.id == id);
        if explained {
            col = col
                .push(
                    text(crate::i18n::tr(
                        "Une notification critique traverse le Silence et reste jusqu'à sa fermeture.",
                    ))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .line_height(crate::ui::tokens::line_height())
                    .color(dim)
                    .height(explain_text_height()),
                )
                .push(
                    button(
                        text(crate::i18n::tr("Bloquer pendant le Silence"))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(11.0)),
                    )
                    // 28 px de haut (HIG ACC-04) : la carte calcule sa hauteur.
                    .padding([crate::ui::tokens::ts(7.0), 10.0])
                    .height(crate::ui::tokens::ts(BUTTON_TARGET))
                    .on_press(Msg::Toasts(TMsg::Refuse(id))),
                );
        }
        let card = container(row![glyph, col].spacing(12).align_y(Alignment::Start))
            .padding([10, 14])
            .width(Layout::of(&self.cfg.general).width)
            .height(card_height(t, explained))
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

    /// Carte Notifications de la barre : les dernières notifications, une
    /// boîte par application avec son nom en en-tête (HIG LAY-02). Titre
    /// dans la couleur du texte, en-tête et corps en secondaire.
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
        // Les plus récentes en haut, dans chaque boîte comme entre elles. La
        // boîte porte le seul cadre : les cartes n'en ont pas.
        let history = &self.toasts.history;
        for (app, ranks) in app_groups(&history[..history.len().min(HISTORY_SHOWN)]) {
            let mut cards = column![].spacing(8).width(Length::Fill);
            for i in ranks {
                let n = &history[i];
                let (title, body) = card_text(n, hide);
                let card: Element<'_, Msg> = row![
                    self.notif_icon(n, CARD_ICON, fg),
                    column![
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
                // l'application, ou l'application elle-même. Un clic droit
                // partout : son menu court (HIG ENT-07).
                let menu = Msg::Toasts(TMsg::Menu(n.id));
                let card: Element<'_, Msg> = match card_press(n) {
                    Some(press) => mouse_area(card)
                        .interaction(iced::mouse::Interaction::Pointer)
                        .on_press(press)
                        .on_right_press(menu)
                        .into(),
                    None => mouse_area(card).on_right_press(menu).into(),
                };
                cards = cards.push(card);
            }
            col = col.push(
                container(
                    column![
                        text(app)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(11.0))
                            .color(dim),
                        cards,
                    ]
                    .spacing(8)
                    .width(Length::Fill),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(
                        Color {
                            a: alpha * GROUP_BOX,
                            ..fg
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(super::design::SMALL_RADIUS),
                    ..Default::default()
                }),
            );
        }
        col.into()
    }

    /// Actions d'une notification, hors `default` (la carte elle-même) :
    /// boutons sans bordure, 12 px d'écart au moins (HIG ACC-10) — la bulle
    /// et la vue étendue les partagent. `None` sans action.
    fn action_buttons<'a>(&'a self, n: &'a Notification, fg: Color) -> Option<Element<'a, Msg>> {
        let extra: Vec<(usize, &(String, String))> = n
            .actions
            .iter()
            .enumerate()
            .filter(|(_, (k, _))| k != "default")
            .take(3)
            .collect();
        if extra.is_empty() {
            return None;
        }
        let id = n.id;
        // Boutons sans bordure : 12 px d'écart au moins (HIG ACC-10).
        let mut r = row![].spacing(12);
        for (i, (k, label)) in extra {
            // Action destructrice (HIG SYS-17) : libellé et icône en rouge,
            // comme une entrée de menu destructive.
            let destructive = n.is_destructive(i);
            let mut caption = text(label.clone())
                .font(self.font)
                .size(crate::ui::tokens::ts(11.0));
            if destructive {
                caption = caption.color(action_tone(true, fg));
            }
            // Icône de l'action (hint `action-icons`) après le libellé
            // (HIG SYS-18).
            let icon = n
                .action_icons
                .get(i)
                .and_then(|icon| icon.as_deref())
                .and_then(|name| {
                    self.icon_named(
                        name,
                        crate::ui::tokens::ts(12.0),
                        action_tone(destructive, fg),
                    )
                });
            let content: Element<'_, Msg> = match icon {
                Some(icon) => row![caption, icon]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .into(),
                None => caption.into(),
            };
            r = r.push(
                button(content)
                    // 28 px de haut au moins (HIG ACC-04) : 14 px de ligne et
                    // 7 px de marge en haut comme en bas.
                    .padding([crate::ui::tokens::ts(7.0), 10.0])
                    .on_press(Msg::Toasts(TMsg::Invoke(id, k.clone()))),
            );
        }
        Some(r.into())
    }

    /// Demande d'autorisation d'une application (HIG PAT-02) : la phrase qui
    /// la nomme, puis « Refuser » et « Autoriser » — 28 px de haut (HIG
    /// ACC-04). Deux lignes au plus, comme le corps : la carte calcule sa
    /// hauteur. La bulle et la vue étendue la partagent.
    fn request_block<'a>(&'a self, n: &'a Notification, ink: Color) -> Element<'a, Msg> {
        let id = n.id;
        let ask = if n.app.is_empty() {
            crate::i18n::tr("Une application veut vous envoyer des notifications.").to_string()
        } else {
            crate::tr!("{app} veut vous envoyer des notifications.", app = n.app)
        };
        let choice = |label: &'static str, press: Msg| {
            button(
                text(crate::i18n::tr(label))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(11.0)),
            )
            // 28 px de haut (HIG ACC-04) : la carte calcule sa hauteur.
            .padding([crate::ui::tokens::ts(7.0), 10.0])
            .height(crate::ui::tokens::ts(BUTTON_TARGET))
            .on_press(press)
        };
        column![
            text(ask)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .line_height(crate::ui::tokens::line_height())
                .color(ink)
                .height(explain_text_height()),
            row![
                choice("Refuser", Msg::Toasts(TMsg::Deny(id))),
                choice("Autoriser", Msg::Toasts(TMsg::Allow(id))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    /// Vue étendue d'une activité de la pilule Island (HIG SYS-13) :
    /// l'application, le titre, le corps et les actions de la notification
    /// `id` — ceux de la bulle, le corps moins coupé —, puis le passage à la
    /// carte des notifications et au calendrier que l'horloge remplace.
    pub(super) fn view_activity(&self, alpha: f32, id: u32) -> Element<'_, Msg> {
        let fg = color(self.bar_palette().fg, alpha);
        let dim = Color {
            a: alpha * DIM,
            ..fg
        };
        let Some(n) = self.activity_notification(id) else {
            return text(crate::i18n::tr("Aucune notification"))
                .font(self.font)
                .color(fg)
                .into();
        };
        // Aperçus masqués (HIG SYS-05) : un texte générique au lieu du corps.
        let (title, body) = activity_text(
            n,
            crate::design_options::flag(&self.cfg.general, "notifications.hide_previews"),
        );
        // « Fermer » : la bulle si elle vit encore, la vue se referme avec
        // elle ; passée à l'historique, la vue seule.
        let close = if self.toasts.list.iter().any(|t| t.n.id == id) {
            Msg::Toasts(TMsg::Close(id))
        } else {
            Msg::ClosePopover
        };
        let mut head = row![].spacing(8).align_y(Alignment::Center);
        if n.urgency == Urgency::Critical {
            // Le repère de l'urgence, comme dans la bulle.
            head = head.push(
                self.icon_named(URGENT_ICON, CRITICAL_MARK, crate::ui::tone::warning())
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
                text(card_app(n))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(11.0))
                    .color(dim)
                    .width(Length::Fill),
            )
            // Bouton à glyphe seul : son infobulle nomme l'action (HIG LAY-12).
            .push(iced::widget::tooltip(
                button(
                    crate::ui::icons::glyph("window-close", crate::ui::tokens::ts(14.0), dim)
                        .expect("glyphe de fermeture"),
                )
                .padding(crate::ui::tokens::ts(7.0))
                .width(crate::ui::tokens::ts(BUTTON_TARGET))
                .height(crate::ui::tokens::ts(BUTTON_TARGET))
                .style(button::text)
                .on_press(close),
                text(crate::i18n::tr("Fermer")).font(self.font),
                iced::widget::tooltip::Position::Left,
            ));
        let mut block = column![head].spacing(4).width(Length::Fill);
        // La bulle porte une demande d'autorisation (HIG PAT-02) : la vue
        // étendue la montre comme elle, et l'on y répond.
        if let Some(t) = self
            .toasts
            .list
            .iter()
            .find(|t| t.n.id == id && t.request.is_some())
        {
            block = block.push(self.request_block(&t.n, fg));
        }
        block = block.push(
            text(title)
                .font(self.title_font())
                .size(crate::ui::tokens::ts(14.0))
                .color(fg),
        );
        if let Some(body) = body {
            block = block.push(
                text(body)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .line_height(crate::ui::tokens::line_height())
                    .color(dim),
            );
        }
        let block: Element<'_, Msg> = row![self.notif_icon(n, CARD_ICON, dim), block]
            .spacing(12)
            .align_y(Alignment::Start)
            .into();
        // Clic sur la notification : action par défaut (si l'application en
        // propose une), comme la bulle.
        let block = if n.actions.iter().any(|(k, _)| k == "default") {
            mouse_area(block)
                .interaction(iced::mouse::Interaction::Pointer)
                .on_press(Msg::Toasts(TMsg::Invoke(id, "default".into())))
                .into()
        } else {
            block
        };
        let mut col = column![block].spacing(12).width(Length::Fill);
        if let Some(actions) = self.action_buttons(n, fg) {
            col = col.push(actions);
        }
        // Le pied mène à la carte des notifications et au calendrier de
        // l'horloge : la bulle critique occupe la pilule jusqu'à sa fermeture.
        use crate::ui::plugins::bar::popover::PMsg;
        let foot = row![
            self.activity_link(
                crate::i18n::tr("Toutes les notifications").to_string(),
                fg,
                Msg::Popover(PMsg::ActivityHistory),
            ),
            self.activity_link(
                crate::i18n::tr("Calendrier").to_string(),
                fg,
                Msg::Popover(PMsg::ActivityCalendar),
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        column![col, self.pop_separator(alpha), foot]
            .spacing(12)
            .width(Length::Fill)
            .into()
    }

    /// Bouton de texte du pied de la vue étendue : sans bordure, 28 px de
    /// haut au moins (HIG ACC-04).
    fn activity_link<'a>(&'a self, label: String, ink: Color, press: Msg) -> Element<'a, Msg> {
        button(
            text(label)
                .font(self.font)
                .size(crate::ui::tokens::ts(11.0))
                .color(ink),
        )
        .padding([crate::ui::tokens::ts(7.0), 10.0])
        .style(button::text)
        .on_press(press)
        .into()
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
            request: None,
            alert: false,
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
        let une = [toast(1, -1)];
        assert_eq!(height(&une, &[]), (CARD + 2.0 * SHADOW).ceil() as u32);
        let neufs: Vec<Toast> = (1..=9).map(|i| toast(i, -1)).collect();
        assert_eq!(height(&neufs, &[]), height_of(&neufs, VISIBLE, &[]));
    }

    /// HIG ACC-04 : les boutons d'une bulle ont 28 px. La carte qui montre un
    /// corps et des actions gagne 20 px, les autres gardent `CARD`, et la
    /// surface suit.
    #[test]
    fn carte_plus_haute_pour_un_corps_et_des_actions() {
        let corps = |actions: &[String]| Toast {
            n: crate::notifications::build(1, "a", "", "s", "b", actions, &HashMap::new(), -1),
            since: Instant::now(),
            request: None,
            alert: false,
        };
        let sans = corps(&["default".into(), "Ouvrir".into()]);
        let avec = corps(&[
            "default".into(),
            "Ouvrir".into(),
            "later".into(),
            "Plus tard".into(),
        ]);
        let deux = [sans, avec];
        assert_eq!(
            card_height(&deux[0], false),
            CARD,
            "un corps sans action tient dans CARD"
        );
        assert_eq!(card_height(&deux[1], false), CARD + CARD_BODY_ACTIONS);
        assert_eq!(
            height(&deux[1..], &[]),
            (CARD + CARD_BODY_ACTIONS + 2.0 * SHADOW).ceil() as u32
        );
        assert_eq!(
            height(&deux, &[]),
            (2.0 * CARD + CARD_BODY_ACTIONS + GAP + 2.0 * SHADOW).ceil() as u32
        );
    }

    /// HIG PAT-05 : la carte qui explique le passage du Silence gagne ses deux
    /// lignes et son bouton, et la surface suit ; les autres bulles ne
    /// changent pas.
    #[test]
    fn carte_plus_haute_quand_elle_explique_le_passage_du_silence() {
        let bulle = |id: u32| Toast {
            n: crate::notifications::build(id, "a", "", "s", "b", &[], &HashMap::new(), -1),
            since: Instant::now(),
            request: None,
            alert: false,
        };
        let deux = [bulle(1), bulle(2)];
        assert_eq!(
            card_height(&deux[1], true) - card_height(&deux[1], false),
            explain_height()
        );
        assert_eq!(
            explain_height(),
            8.0 + explain_text_height() + BUTTON_TARGET
        );
        let explain = Explain {
            key: "a".into(),
            id: 2,
        };
        assert_eq!(
            height_of(&deux, VISIBLE, std::slice::from_ref(&explain)),
            (2.0 * CARD + explain_height() + GAP + 2.0 * SHADOW).ceil() as u32
        );
        // La bulle expliquée hors du nombre visible : la surface ne change pas.
        assert_eq!(
            height_of(&deux, 1, std::slice::from_ref(&explain)),
            height_of(&deux, 1, &[])
        );
        assert_eq!(
            height_of(&deux, 1, std::slice::from_ref(&explain)),
            height(&deux[1..], &[])
        );
    }

    /// HIG PAT-02 : la carte qui porte une demande d'autorisation gagne ses
    /// deux lignes et ses deux boutons, la surface suit ; les autres bulles ne
    /// changent pas.
    #[test]
    fn carte_plus_haute_quand_elle_porte_une_demande() {
        let bulle = |id: u32| Toast {
            n: crate::notifications::build(id, "a", "", "s", "b", &[], &HashMap::new(), -1),
            since: Instant::now(),
            request: None,
            alert: false,
        };
        let deux = [bulle(1), bulle(2)];
        assert_eq!(
            card_height(&deux[0], false),
            CARD,
            "sans demande, la carte ne change pas"
        );
        let avec = [
            deux[0].clone(),
            Toast {
                request: Some("a".into()),
                alert: false,
                ..bulle(2)
            },
        ];
        assert_eq!(
            card_height(&avec[1], false) - card_height(&deux[1], false),
            explain_height()
        );
        assert_eq!(
            height(&avec, &[]),
            (2.0 * CARD + explain_height() + GAP + 2.0 * SHADOW).ceil() as u32
        );
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

    /// Clé d'application propre à un test : le fichier du passage du Silence
    /// est commun aux tests d'un même processus.
    fn cle() -> String {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        format!(
            "pat05-{}",
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )
    }

    /// Le fichier du passage du Silence est commun aux tests d'un processus et
    /// sa réécriture part d'une lecture : deux tests qui y écriraient à la fois
    /// perdraient l'inscription de l'un d'eux — de même du fichier des
    /// autorisations (HIG PAT-02), même verrou. Tout test qui fait passer une
    /// notification par `TMsg::Event(Event::Shown)` en tenant le service prend
    /// ce verrou, un seul à la fois.
    static VERROU_PASSAGE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Prend ce verrou pour tout le test.
    fn verrou_passage() -> std::sync::MutexGuard<'static, ()> {
        VERROU_PASSAGE.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn critique(id: u32, app: &str) -> Notification {
        let mut h = HashMap::new();
        h.insert("urgency".to_string(), zbus::zvariant::OwnedValue::from(2u8));
        crate::notifications::build(id, app, "", "s", "", &[], &h, -1)
    }

    fn application() -> App {
        App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-toast-tests/config.lua"),
            true,
        )
        .0
    }

    /// HIG PAT-05 : la première notification critique d'une application
    /// explique le passage du Silence dans sa bulle ; la seconde, non. La clé
    /// est l'identifiant `desktop-entry` quand il en est un, sinon l'émetteur.
    #[test]
    fn la_premiere_critique_d_une_application_explique_le_passage_du_silence() {
        let _verrou = verrou_passage();
        let (mut app, autre) = (application(), cle());
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(1, &autre))));
        let expliquee = Explain {
            key: autre.clone(),
            id: 1,
        };
        assert_eq!(app.toasts.explain, vec![expliquee.clone()]);
        assert_eq!(
            Passage::load().granted(&autre),
            Some(true),
            "accordé tant que la bulle n'est pas refusée"
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(2, &autre))));
        assert_eq!(
            app.toasts.explain,
            vec![expliquee.clone()],
            "la seconde critique de l'application n'est pas réexpliquée"
        );
        assert_eq!(app.toasts.list.len(), 2);
        // Deux applications expliquées à la fois : chacune garde sa bulle.
        let tierce = cle();
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(4, &tierce))));
        let ids = |app: &App| app.toasts.explain.iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&app), vec![1, 4]);
        // Bulle fermée : le passage reste accordé, l'explication s'oublie.
        let _ = app.toasts_update(TMsg::Event(Event::Closed(4)));
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(5, &tierce))));
        assert_eq!(ids(&app), vec![1]);
        assert_eq!(Passage::load().granted(&tierce), Some(true));
        // Une application redémarrée (même émetteur, même identifiant) est
        // reconnue ; un identifiant qui n'en est pas ne compte pas.
        let mut h = HashMap::new();
        h.insert("urgency".to_string(), zbus::zvariant::OwnedValue::from(2u8));
        h.insert(
            "desktop-entry".to_string(),
            zbus::zvariant::OwnedValue::try_from(zbus::zvariant::Value::from(format!(
                "{autre}.desktop"
            )))
            .unwrap(),
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(crate::notifications::build(
            3,
            "quiconque",
            "",
            "s",
            "",
            &[],
            &h,
            -1,
        ))));
        assert_eq!(app.toasts.explain, vec![expliquee]);
        assert_eq!(passage_key(&recue(1, "un\nnom")), "un nom");
    }

    /// HIG PAT-05 : refusé depuis la bulle, le passage du Silence ne s'accorde
    /// plus à l'application — sous Silence, ses critiques vont à l'historique
    /// seules ; hors Silence, elles gardent leur bulle. Une autre application
    /// passe, et est expliquée. Sans le service (aperçu, autre démon), le
    /// refus retire l'explication sans rien garder.
    #[test]
    fn une_application_refusee_ne_passe_plus_le_silence() {
        let _verrou = verrou_passage();
        let (mut app, refusee, autre) = (application(), cle(), cle());
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(1, &refusee))));
        assert!(app.toasts.refuse(1, true), "la bulle portait l'explication");
        assert!(
            app.toasts.explain.is_empty(),
            "l'explication se retire avec le refus"
        );
        assert_eq!(Passage::load().granted(&refusee), Some(false));
        assert!(
            !app.toasts.refuse(1, true),
            "plus rien à refuser sur cette bulle"
        );
        assert_eq!(
            app.toasts.list.len(),
            1,
            "la notification déjà livrée reste"
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(2, &refusee))));
        assert!(
            app.toasts.explain.is_empty(),
            "une application refusée n'est plus réexpliquée"
        );
        app.toasts.dnd = true;
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(3, &refusee))));
        assert_eq!(
            app.toasts.list.len(),
            2,
            "sous Silence : plus de bulle pour elle"
        );
        assert_eq!(app.toasts.history.len(), 3, "l'historique garde");
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(4, &autre))));
        assert_eq!(
            app.toasts.list.len(),
            3,
            "une autre application passe le Silence"
        );
        assert_eq!(
            app.toasts
                .explain
                .iter()
                .map(|e| (e.key.as_str(), e.id))
                .collect::<Vec<_>>(),
            vec![(autre.as_str(), 4)],
            "et c'est sa première fois : sa bulle explique"
        );
        app.toasts.dnd = false;
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(5, &refusee))));
        assert_eq!(
            app.toasts.list.len(),
            4,
            "hors Silence, la critique refusée garde sa bulle"
        );
        // Sans le service : l'explication se retire, la décision reste.
        let _ = app.toasts.refuse(4, false);
        assert!(app.toasts.explain.is_empty());
        assert_eq!(Passage::load().granted(&autre), Some(true));
    }

    /// HIG PAT-02 : la première notification d'une application porte, dans sa
    /// bulle, la demande d'autorisation. La bulle reste jusqu'à la réponse et
    /// sa notification n'entre pas dans l'historique ; « Autoriser » garde la
    /// décision et remet la bulle comme les autres, son délai reparti ; la
    /// notification suivante arrive sans demande. Sans le service (aperçu,
    /// autre démon), rien ne se demande.
    #[test]
    fn la_premiere_notification_d_une_application_demande_l_autorisation() {
        let _verrou = verrou_passage();
        let (mut app, autre) = (application(), cle());
        app.toasts.active = true;
        let bulles = |app: &App| {
            app.toasts
                .list
                .iter()
                .map(|t| (t.n.id, t.request.clone()))
                .collect::<Vec<_>>()
        };
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(1, &autre))));
        assert_eq!(
            bulles(&app),
            vec![(1, Some(autre.clone()))],
            "la bulle de demande se montre, et nomme son application"
        );
        assert!(
            app.toasts.history.is_empty(),
            "non accordée : rien n'entre dans l'historique"
        );
        assert!(
            Authorizations::load().get(&autre).is_none(),
            "tant qu'il n'y a pas de réponse, rien n'est inscrit"
        );
        app.toasts.list[0].since = Instant::now() - Duration::from_secs(60);
        let _ = app.toasts_update(TMsg::Tick);
        assert_eq!(
            bulles(&app).len(),
            1,
            "une demande ne s'efface pas d'elle-même"
        );
        let _ = app.toasts_update(TMsg::Allow(1));
        assert_eq!(
            Authorizations::load().get(&autre),
            Some(Authorization {
                granted: true,
                delivery: Delivery::Banner,
            }),
            "« Autoriser » garde l'autorisation"
        );
        assert_eq!(bulles(&app), vec![(1, None)], "la bulle devient ordinaire");
        assert_eq!(
            app.toasts.history.len(),
            1,
            "accordée : la notification est remise"
        );
        let _ = app.toasts_update(TMsg::Tick);
        assert_eq!(bulles(&app).len(), 1, "son délai repart de la réponse");
        app.toasts.list[0].since = Instant::now() - Duration::from_secs(60);
        let _ = app.toasts_update(TMsg::Tick);
        assert!(bulles(&app).is_empty(), "puis elle s'efface à son échéance");
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(2, &autre))));
        assert_eq!(
            bulles(&app),
            vec![(2, None)],
            "la suivante arrive sans demande"
        );
        assert_eq!(app.toasts.history.len(), 2);
        // Sans le service : rien ne se demande.
        let (mut app, sans) = (application(), cle());
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(9, &sans))));
        assert_eq!(bulles(&app), vec![(9, None)]);
    }

    /// HIG PAT-02 : « Refuser » ferme la bulle sans historique et garde le
    /// refus ; l'application n'est plus remise — ni bulle, ni historique — et
    /// ne redemande pas. Fermée sans réponse, une demande n'inscrit rien et la
    /// notification suivante redemande ; sous Silence, une demande non
    /// critique ne se pose pas.
    #[test]
    fn une_application_refusee_n_est_plus_remise_et_une_demande_fermee_redemande() {
        let _verrou = verrou_passage();
        let (mut app, refusee, autre) = (application(), cle(), cle());
        app.toasts.active = true;
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(1, &refusee))));
        let _ = app.toasts_update(TMsg::Deny(1));
        assert!(app.toasts.list.is_empty(), "« Refuser » ferme la bulle");
        assert!(app.toasts.history.is_empty(), "sans historique");
        assert_eq!(
            Authorizations::load().get(&refusee),
            Some(Authorization {
                granted: false,
                delivery: Delivery::Banner,
            }),
            "le refus est gardé"
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(2, &refusee))));
        assert!(app.toasts.list.is_empty(), "refusée : plus de bulle");
        assert!(app.toasts.history.is_empty(), "refusée : plus d'historique");
        // Fermée sans réponse : rien n'est inscrit, la suivante redemande.
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(3, &autre))));
        let _ = app.toasts_update(TMsg::Close(3));
        assert!(
            Authorizations::load().get(&autre).is_none(),
            "fermée sans réponse : rien n'est inscrit"
        );
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(4, &autre))));
        assert_eq!(
            app.toasts.list[0].request.as_deref(),
            Some(autre.as_str()),
            "la notification suivante redemande"
        );
        // Sous Silence : pas de bulle, donc pas de demande, et rien dans
        // l'historique.
        let _ = app.toasts_update(TMsg::Close(4));
        app.toasts.dnd = true;
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(5, &autre))));
        assert!(
            app.toasts.list.is_empty(),
            "Silence : pas de bulle de demande"
        );
        assert!(
            app.toasts.history.is_empty(),
            "rien n'est remis sans accord"
        );
    }

    /// HIG PAT-02 : la demande prime sur l'explication du passage du Silence
    /// (PAT-05) — une demande critique passe le Silence, mais rien ne s'inscrit
    /// ni ne s'explique ; l'autorisation donnée, la première critique qui suit
    /// l'explique.
    #[test]
    fn une_demande_critique_passe_le_silence_sans_expliquer_le_passage() {
        let _verrou = verrou_passage();
        let (mut app, autre) = (application(), cle());
        app.toasts.active = true;
        app.toasts.dnd = true;
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(1, &autre))));
        assert_eq!(
            app.toasts.list[0].request.as_deref(),
            Some(autre.as_str()),
            "la demande critique se montre sous Silence"
        );
        assert!(
            app.toasts.explain.is_empty(),
            "la demande prime : rien n'est expliqué"
        );
        assert_eq!(
            Passage::load().granted(&autre),
            None,
            "et le passage n'est pas inscrit"
        );
        let _ = app.toasts_update(TMsg::Allow(1));
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(2, &autre))));
        assert_eq!(
            app.toasts.explain,
            vec![Explain {
                key: autre.clone(),
                id: 2,
            }],
            "la première critique qui suit l'autorisation explique le passage"
        );
    }

    /// HIG SYS-03 : le style de remise de l'application accordée s'applique à
    /// ses notifications ordinaires — « Alertes » garde la bulle jusqu'à sa
    /// fermeture, « Aucune » passe par l'historique seul, « Bannières » garde
    /// le délai ordinaire ; une critique garde ses règles, quel que soit le
    /// style.
    #[test]
    fn le_style_de_remise_de_l_application_s_applique_a_ses_notifications() {
        let _verrou = verrou_passage();
        let accordee = |delivery| {
            let key = cle();
            Authorizations::load()
                .set(
                    &key,
                    Authorization {
                        granted: true,
                        delivery,
                    },
                )
                .save();
            key
        };
        let (mut app, alertes, aucune, bannieres) = (
            application(),
            accordee(Delivery::Alert),
            accordee(Delivery::Silent),
            accordee(Delivery::Banner),
        );
        app.toasts.active = true;
        let bulles = |app: &App| app.toasts.list.iter().map(|t| t.n.id).collect::<Vec<_>>();
        // « Alertes » : la bulle reste.
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(1, &alertes))));
        assert_eq!(bulles(&app), vec![1]);
        assert_eq!(app.toasts.history.len(), 1);
        app.toasts.list[0].since = Instant::now() - Duration::from_secs(600);
        let _ = app.toasts_update(TMsg::Tick);
        assert_eq!(
            bulles(&app),
            vec![1],
            "« Alertes » : la bulle ne s'efface pas"
        );
        // « Aucune » : l'historique seul, sans bulle.
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(2, &aucune))));
        assert_eq!(bulles(&app), vec![1], "« Aucune » : pas de bulle");
        assert_eq!(
            app.toasts.history.len(),
            2,
            "« Aucune » : l'historique seul"
        );
        // Une critique garde ses règles, quel que soit le style.
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(3, &aucune))));
        assert_eq!(bulles(&app), vec![3, 1], "une critique garde sa bulle");
        // « Bannières » : le délai ordinaire.
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(4, &bannieres))));
        assert_eq!(bulles(&app), vec![4, 3, 1]);
        app.toasts.list[0].since = Instant::now() - Duration::from_secs(600);
        let _ = app.toasts_update(TMsg::Tick);
        assert_eq!(
            bulles(&app),
            vec![3, 1],
            "« Bannières » : s'efface à son échéance"
        );
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

    /// Une notification reçue d'une application, sans corps ni icône.
    fn recue(id: u32, app: &str) -> Notification {
        crate::notifications::build(id, app, "", "s", "", &[], &HashMap::new(), -1)
    }

    /// HIG SYS-13 : la vue étendue montre la bulle cliquée, puis son entrée
    /// d'historique quand la bulle s'est effacée ; une action invoquée et
    /// « Fermer » la referment, fermer une autre bulle la laisse.
    #[test]
    fn la_vue_etendue_suit_sa_bulle_puis_son_historique_et_se_referme_sur_son_action() {
        use crate::ui::plugins::bar::popover::Kind;
        use crate::ui::plugins::bar::surface::BarSurface;
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-toast-tests/config.lua"),
            true,
        );
        let bar = Id::unique();
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
        app.bar_surfaces.insert(
            bar,
            BarSurface {
                monitor: "A".into(),
                ..Default::default()
            },
        );
        app.toasts.list = vec![Toast {
            n: carte(&["default", "open"], None),
            since: Instant::now(),
            request: None,
            alert: false,
        }];
        let _ = app.open_popover(bar, "clock".into(), Kind::Activity(1));
        assert!(app.activity_open(1), "la vue de la bulle 1 est ouverte");
        assert!(!app.activity_open(2), "pas celle d'une autre bulle");
        assert_eq!(app.activity_notification(1).map(|n| n.id), Some(1));
        // La bulle s'efface : la vue la relit dans l'historique.
        app.toasts.list.clear();
        assert!(
            app.activity_notification(1).is_none(),
            "sans historique, la notification a disparu"
        );
        app.toasts.history = vec![recue(1, "a")];
        assert_eq!(
            app.activity_notification(1).map(|n| n.id),
            Some(1),
            "l'entrée d'historique"
        );
        // Une action de la vue la referme.
        let _ = app.toasts_update(TMsg::Invoke(1, "open".into()));
        assert!(!app.activity_open(1), "l'action referme la vue");
        // Fermer une autre bulle laisse la vue ; « Fermer » referme la vue
        // de sa bulle.
        let _ = app.open_popover(bar, "clock".into(), Kind::Activity(1));
        app.toasts.list = vec![Toast {
            n: recue(2, "b"),
            since: Instant::now(),
            request: None,
            alert: false,
        }];
        let _ = app.toasts_update(TMsg::Close(2));
        assert!(app.activity_open(1), "fermer une autre bulle laisse la vue");
        let _ = app.toasts_update(TMsg::Close(1));
        assert!(
            !app.activity_open(1),
            "« Fermer » referme la vue avec sa bulle"
        );
        // La vue ne lit rien (HIG SYS-02) : seules les listes le font.
        app.toasts.history = vec![recue(3, "c")];
        let _ = app.open_popover(bar, "clock".into(), Kind::Activity(3));
        assert_eq!(app.toasts.unread(), 1, "la vue étendue n'est pas une liste");
        // Les bornes de la vue : huit lignes de corps, le titre moins coupé
        // que dans une carte de l'historique.
        let long = crate::notifications::build(
            4,
            "a",
            "",
            &"t".repeat(200),
            &(0..12)
                .map(|i| format!("ligne {i}"))
                .collect::<Vec<_>>()
                .join("\n"),
            &[],
            &HashMap::new(),
            -1,
        );
        let (titre, corps) = activity_text(&long, false);
        assert_eq!(
            titre.chars().count(),
            ACTIVITY_TITLE_CHARS + 1,
            "coupé, suivi de « … »"
        );
        let corps = corps.expect("un corps");
        assert_eq!(corps.lines().count(), ACTIVITY_BODY_LINES);
        assert!(corps.starts_with("ligne 0\n") && corps.contains("ligne 7"));
        assert!(!corps.contains("ligne 8"));
        let (_, masque) = activity_text(&long, true);
        assert_eq!(masque, Some(crate::i18n::tr("Notification").to_string()));
    }

    #[test]
    fn l_historique_est_groupe_par_application() {
        // Les plus récentes d'abord : deux applications alternent, deux
        // notifications viennent d'applications sans nom.
        let history = [
            recue(1, "a"),
            recue(2, "b"),
            recue(3, "a"),
            recue(4, ""),
            recue(5, "b"),
            recue(6, ""),
        ];
        let groups = app_groups(&history);
        let names: Vec<&str> = groups.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            ["a", "b", "Notification"],
            "l'ordre de la plus récente de chaque application ; sans nom, une seule"
        );
        assert_eq!(
            groups[0].1,
            [0, 2],
            "dans une boîte, l'ordre de l'historique"
        );
        assert_eq!(groups[1].1, [1, 4]);
        assert_eq!(groups[2].1, [3, 5]);
        // Chaque notification est montrée une fois, dans sa boîte.
        let mut shown: Vec<usize> = groups
            .iter()
            .flat_map(|(_, ranks)| ranks.iter().copied())
            .collect();
        shown.sort_unstable();
        assert_eq!(shown, (0..history.len()).collect::<Vec<_>>());
        assert!(app_groups(&[]).is_empty(), "historique vide : aucune boîte");
    }

    #[test]
    fn une_seule_application_forme_une_boite() {
        let history = [recue(1, "Firefox"), recue(2, "Firefox")];
        assert_eq!(
            app_groups(&history),
            vec![("Firefox".to_string(), vec![0, 1])]
        );
    }

    #[test]
    fn l_en_tete_d_un_groupe_est_le_nom_de_l_application() {
        assert_eq!(card_app(&recue(1, "Signal")), "Signal");
        assert_eq!(
            card_app(&recue(2, "")),
            crate::i18n::tr("Notification"),
            "sans nom : le titre générique, pas une boîte sans en-tête"
        );
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
        let _verrou = verrou_passage();
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

    /// HIG SYS-02 : une notification entre dans l'historique non lue, une
    /// transitoire ne compte pas ; l'ouverture d'une liste lit tout ; un
    /// remplacement (même id) est non lu à nouveau.
    #[test]
    fn les_non_lues_se_comptent_jusqu_a_l_ouverture_d_une_liste() {
        let mut app = application();
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(1, "a"))));
        let mut transitoire = recue(2, "a");
        transitoire.transient = true;
        let _ = app.toasts_update(TMsg::Event(Event::Shown(transitoire)));
        let _ = app.toasts_update(TMsg::Event(Event::Shown(recue(3, "b"))));
        assert_eq!(app.toasts.history.len(), 2, "une transitoire n'entre pas");
        assert_eq!(app.toasts.unread(), 2);

        let _ = app.toasts_update(TMsg::Read);
        assert_eq!(app.toasts.unread(), 0, "une liste ouverte lit tout");
        assert_eq!(app.toasts.history.len(), 2, "l'historique reste");

        // Remplacement : même id, contenu changé.
        let mut remplacee = recue(1, "a");
        remplacee.summary = "changé".into();
        let _ = app.toasts_update(TMsg::Event(Event::Shown(remplacee)));
        assert_eq!(app.toasts.history.len(), 2);
        assert_eq!(app.toasts.unread(), 1, "un remplacement est non lu");
    }

    /// HIG SYS-02 : sous Silence, une critique dont l'application a perdu le
    /// passage va à l'historique sans bulle — elle compte quand même.
    #[test]
    fn une_notification_sans_bulle_compte_comme_non_lue() {
        let _verrou = verrou_passage();
        let (mut app, refusee) = (application(), cle());
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(1, &refusee))));
        assert!(app.toasts.refuse(1, true), "la bulle portait l'explication");
        let _ = app.toasts_update(TMsg::Read);
        assert_eq!(app.toasts.unread(), 0);
        app.toasts.dnd = true;
        let _ = app.toasts_update(TMsg::Event(Event::Shown(critique(2, &refusee))));
        assert_eq!(app.toasts.list.len(), 1, "sous Silence : pas de bulle");
        assert_eq!(app.toasts.unread(), 1, "sans bulle, elle est non lue");
    }

    /// HIG ENT-07 : un clic droit sur une carte du centre ouvre son menu sur
    /// la surface de la carte qui la montre ; sans carte ouverte, ou sur une
    /// notification fermée depuis, rien ne s'ouvre.
    #[test]
    fn clic_droit_d_une_carte_sans_cible_n_ouvre_rien() {
        use crate::ui::plugins::bar::popover::Kind;
        use crate::ui::plugins::bar::surface::BarSurface;
        let mut app = application();
        // Aucune carte du centre ouverte : rien.
        let _ = app.toasts_update(TMsg::Menu(7));
        assert!(app.icon_menu.is_none(), "sans carte ouverte");
        let bar = Id::unique();
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
        app.bar_surfaces.insert(
            bar,
            BarSurface {
                monitor: "A".into(),
                ..Default::default()
            },
        );
        let _ = app.open_popover(bar, "notifications".into(), Kind::Notifications);
        assert!(
            app.history_list_open(),
            "la carte Notifications est ouverte"
        );
        // Carte ouverte, notification inconnue (fermée depuis) : rien.
        let _ = app.toasts_update(TMsg::Menu(9));
        assert!(app.icon_menu.is_none(), "notification fermée depuis");
        // La carte montre la notification : son menu, sur la surface de la
        // carte. Celle-ci ne mène nulle part : pas d'entrée « Ouvrir ».
        app.toasts.history.push(recue(7, "Signal"));
        let _ = app.toasts_update(TMsg::Menu(7));
        let m = app.icon_menu.as_ref().expect("le menu de la carte");
        assert!(matches!(
            &m.target,
            crate::ui::context::MenuTarget::Notification(n) if n.id == 7 && !n.openable
        ));
        assert_eq!(Some(m.desk), app.popover.as_ref().map(|p| p.win));
        // Le menu part avec sa carte.
        let _ = app.destroy_popover();
        assert!(app.icon_menu.is_none(), "carte fermée : plus de menu");
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

#[cfg(test)]
mod action_tests {
    use super::{Color, action_tone};

    /// HIG SYS-17 : une action destructrice prend le ton « danger », celui
    /// des entrées de menu destructrices ; les autres gardent celui du texte.
    #[test]
    fn une_action_destructive_prend_le_ton_danger() {
        let fg = Color::from_rgba8(10, 20, 30, 1.0);
        assert_eq!(action_tone(true, fg), crate::ui::tone::danger());
        assert_eq!(action_tone(false, fg), fg);
    }
}
