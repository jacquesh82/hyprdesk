//! Trousseau myMFA dans le démon (MLG-44, MLG-45) : abonnement au bus de
//! session (état du coffre, demandes de déverrouillage des applications) et
//! carte au centre de l'écran actif, dans le style de la carte polkit.
//!
//! La carte accompagne tout le parcours, sans terminal :
//! - pas de coffre : création (mot de passe choisi et confirmé) ou reprise
//!   de la sauvegarde du web — proposée d'elle-même à l'ouverture de session,
//!   et à la place du déverrouillage quand une application demande un secret ;
//! - coffre présent mais pas encore scellé, clé de session reçue : une saisie
//!   proposée à l'ouverture de session, la dernière ;
//! - ensuite, la connexion ouvre le coffre (pam_mymfa) et la carte ne sert
//!   plus qu'au déverrouillage après un verrouillage manuel.
//!
//! Le mot de passe part au démon mymfa (`Unlock`, `CreateVault`,
//! `InstallVault`) puis est effacé.

use iced::futures::{SinkExt, Stream, StreamExt};
use iced::widget::{button, column, container, row, text, text_input};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};
use crate::mymfa::State;
use std::path::PathBuf;

pub const NAMESPACE: &str = "hyprdesk-mymfa";
const INPUT_ID: &str = "hyprdesk-mymfa-password";
const CONFIRM_ID: &str = "hyprdesk-mymfa-confirm";
const WIDTH: f32 = 480.0;
const SHADOW: f32 = 18.0;
/// Délai, après le démarrage, pendant lequel l'accueil attend la clé de
/// session (remise par pam_mymfa, consommée par le démon en une seconde).
pub const GREET_WINDOW: std::time::Duration = std::time::Duration::from_secs(20);

/// Visage de la carte.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Coffre présent et verrouillé.
    Unlock,
    /// Aucun coffre : création.
    Create,
    /// Aucun coffre : reprise de la sauvegarde du web (`mymfa-vault.json`).
    Install(PathBuf),
    /// Coffre créé ou scellé ; `sealed` : il s'ouvrira seul à la connexion.
    Ready { sealed: bool },
}

impl Kind {
    fn height(&self) -> f32 {
        match self {
            Kind::Unlock => 250.0,
            Kind::Create => 350.0,
            Kind::Install(_) => 255.0,
            Kind::Ready { .. } => 215.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    State(State),
    /// Une application attend le coffre (nom tiré de /proc/PID/comm).
    UnlockRequested(String),
}

#[derive(Debug, Clone)]
pub enum MMsg {
    Event(Event),
    /// Ouvre la carte (barre, lanceur, réglages) ; sans effet si ouvert.
    Ask,
    /// Ouvre la carte d'un ordinateur sans coffre : reprise de cette
    /// sauvegarde, ou (sans) recherche d'une sauvegarde puis création.
    Setup(Option<PathBuf>),
    /// Sauvegarde cherchée : (application qui attend, sauvegarde trouvée).
    Found(String, Option<PathBuf>),
    /// Création ↔ reprise de la sauvegarde.
    Switch(Kind),
    Input(String),
    Confirm(String),
    /// Entrée dans le premier champ de la création : vers la confirmation.
    Next,
    Submit,
    Cancel,
    /// Réponse du démon à Unlock : `Ok(scellé)`, `Err((mauvais mot de passe, message))`.
    Unlocked(Result<bool, (bool, String)>),
    /// Réponse à CreateVault / InstallVault : `Ok(scellé)`.
    Created(Result<bool, String>),
    Lock,
    Refresh,
}

/// Carte ouverte.
pub struct Dialog {
    pub win: Id,
    pub kind: Kind,
    /// Application qui attend (vide : demande de l'utilisateur).
    pub app: String,
    pub input: String,
    /// Confirmation du mot de passe (création).
    pub confirm: String,
    /// Sauvegarde du web trouvée (proposée depuis la création).
    pub backup: Option<PathBuf>,
    /// Ce déverrouillage scellera le coffre (dernière saisie).
    pub sealing: bool,
    pub message: Option<String>,
    pub busy: bool,
}

impl Dialog {
    pub(super) fn wipe(&mut self) {
        crate::lock::pam::wipe(&mut self.input);
        crate::lock::pam::wipe(&mut self.confirm);
        self.input.clear();
        self.confirm.clear();
    }
}

/// Ce qui manque à un nouveau mot de passe, ou `None` s'il convient.
pub fn create_issue(password: &str, confirm: &str) -> Option<String> {
    let n = crate::mymfa::length(password);
    if n < crate::mymfa::MIN_LEN {
        return Some(crate::tr!(
            "Encore {n} caractères.",
            n = crate::mymfa::MIN_LEN - n
        ));
    }
    if confirm != password {
        return Some(crate::i18n::tr("Les deux saisies diffèrent.").to_string());
    }
    None
}

/// Abonnement : état du coffre et demandes de déverrouillage. Sans client
/// installé, un nouveau coup d'œil toutes les 30 s (installé entre-temps).
pub fn stream() -> impl Stream<Item = Event> {
    iced::stream::channel(
        8,
        async move |mut out: iced::futures::channel::mpsc::Sender<Event>| {
            loop {
                let Ok(conn) = zbus::Connection::session().await else {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    continue;
                };
                let st = crate::mymfa::state_of(&conn).await;
                let _ = out.send(Event::State(st.clone())).await;
                if !st.installed {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    continue;
                }
                let Ok(p) = crate::mymfa::MyMfa1Proxy::new(&conn).await else {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    continue;
                };
                let (Ok(mut changed), Ok(mut asked)) = (
                    p.receive_state_changed().await,
                    p.receive_unlock_requested().await,
                ) else {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    continue;
                };
                // Le démon redémarré (propriétaire du nom changé) : état relu.
                let mut owner = p.inner().receive_owner_changed().await.ok();
                // Coffre créé, sceau posé, clé de session reçue : état relu.
                let mut vault = p.receive_vault_present_changed().await;
                let mut sealed = p.receive_sealed_changed().await;
                let mut key = p.receive_session_key_changed().await;
                loop {
                    tokio::select! {
                        Some(_) = vault.next() => {
                            let st = crate::mymfa::state_of(&conn).await;
                            let _ = out.send(Event::State(st)).await;
                        }
                        Some(_) = sealed.next() => {
                            let st = crate::mymfa::state_of(&conn).await;
                            let _ = out.send(Event::State(st)).await;
                        }
                        Some(_) = key.next() => {
                            let st = crate::mymfa::state_of(&conn).await;
                            let _ = out.send(Event::State(st)).await;
                        }
                        s = changed.next() => {
                            if s.is_none() { break; }
                            let st = crate::mymfa::state_of(&conn).await;
                            let _ = out.send(Event::State(st)).await;
                        }
                        a = asked.next() => {
                            let Some(a) = a else { break };
                            let app = a.args().map(|x| x.application.clone()).unwrap_or_default();
                            let _ = out.send(Event::UnlockRequested(app)).await;
                        }
                        o = async {
                            match owner.as_mut() {
                                Some(s) => s.next().await,
                                None => std::future::pending().await,
                            }
                        } => {
                            if o.is_none() { break; }
                            let st = crate::mymfa::state_of(&conn).await;
                            let _ = out.send(Event::State(st)).await;
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        },
    )
}

impl App {
    /// Ouvre la carte sous ce visage (ou change celui de la carte ouverte).
    fn open_mfa_card(&mut self, kind: Kind, app: String, backup: Option<PathBuf>) -> Task<Msg> {
        let focus = iced::widget::operation::focus(INPUT_ID);
        let sealing = kind == Kind::Unlock && self.mfa.session_key && !self.mfa.sealed;
        if let Some(d) = self.mfa_dialog.as_mut() {
            if d.busy {
                return Task::none();
            }
            if d.app.is_empty() {
                d.app = app;
            }
            if backup.is_some() {
                d.backup = backup;
            }
            if d.kind == kind {
                return focus;
            }
            let resize = d.kind.height() != kind.height();
            d.wipe();
            d.message = None;
            d.sealing = sealing;
            d.kind = kind;
            let (win, h) = (d.win, d.kind.height());
            return Task::batch([
                if resize {
                    Task::done(Msg::SizeChange {
                        id: win,
                        size: ((WIDTH + 2.0 * SHADOW) as u32, (h + 2.0 * SHADOW) as u32),
                    })
                } else {
                    Task::none()
                },
                focus,
            ]);
        }
        let win = Id::unique();
        let h = kind.height();
        self.mfa_dialog = Some(Dialog {
            win,
            kind,
            app,
            input: String::new(),
            confirm: String::new(),
            backup,
            sealing,
            message: None,
            busy: false,
        });
        Task::batch([
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some(((WIDTH + 2.0 * SHADOW) as u32, (h + 2.0 * SHADOW) as u32)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::empty(),
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option: OutputOption::Active,
                    namespace: Some(NAMESPACE.into()),
                    ..Default::default()
                },
                id: win,
            }),
            focus,
        ])
    }

    /// Ordinateur sans coffre : la sauvegarde du web est cherchée (dans
    /// Téléchargements), puis la carte s'ouvre sur sa reprise, ou sur la
    /// création s'il n'y en a pas.
    fn open_mfa_setup(&mut self, app: String, backup: Option<PathBuf>) -> Task<Msg> {
        if let Some(b) = backup {
            return self.open_mfa_card(Kind::Install(b.clone()), app, Some(b));
        }
        Task::perform(
            async {
                tokio::task::spawn_blocking(crate::mymfa::find_backup)
                    .await
                    .ok()
                    .flatten()
            },
            move |b| Msg::Mymfa(MMsg::Found(app.clone(), b)),
        )
    }

    /// Accueil de l'ouverture de session : sans coffre, la carte de
    /// création ; coffre pas encore scellé et clé de session reçue, la
    /// dernière saisie. Une fois par session.
    fn mfa_greet(&mut self) -> Task<Msg> {
        let Some(t) = self.mfa_greet else {
            return Task::none();
        };
        let s = &self.mfa;
        if !s.installed {
            if t.elapsed() > GREET_WINDOW {
                self.mfa_greet = None;
            }
            return Task::none();
        }
        if !s.vault {
            self.mfa_greet = None;
            return self.open_mfa_setup(String::new(), None);
        }
        if !s.locked || s.sealed || t.elapsed() > GREET_WINDOW {
            self.mfa_greet = None;
            return Task::none();
        }
        if s.session_key {
            self.mfa_greet = None;
            return self.open_mfa_card(Kind::Unlock, String::new(), None);
        }
        Task::none()
    }

    pub(super) fn mfa_update(&mut self, m: MMsg) -> Task<Msg> {
        match m {
            MMsg::Event(Event::State(s)) => {
                let first = !self.mfa.installed && s.installed;
                crate::mymfa::write_state(&s);
                self.mfa = s;
                if first {
                    // hyprdesk répond aux demandes : invite de repli du démon coupée.
                    std::thread::spawn(crate::mymfa::ensure_dropin);
                }
                // Coffre ouvert ou créé ailleurs (CLI, connexion) : carte inutile.
                let s = &self.mfa;
                let obsolete = self.mfa_dialog.as_ref().is_some_and(|d| {
                    !d.busy
                        && match d.kind {
                            Kind::Unlock => s.vault && !s.locked,
                            Kind::Create | Kind::Install(_) => s.vault,
                            Kind::Ready { .. } => false,
                        }
                });
                if obsolete {
                    return self.close_mfa_dialog();
                }
                self.mfa_greet()
            }
            MMsg::Event(Event::UnlockRequested(app)) => {
                if !self.mfa.vault {
                    return self.open_mfa_setup(app, None);
                }
                if !self.mfa.locked {
                    return Task::none();
                }
                self.open_mfa_card(Kind::Unlock, app, None)
            }
            MMsg::Ask => {
                if !self.mfa.installed {
                    return Task::none();
                }
                if !self.mfa.vault {
                    return self.open_mfa_setup(String::new(), None);
                }
                if !self.mfa.locked {
                    return Task::none();
                }
                self.open_mfa_card(Kind::Unlock, String::new(), None)
            }
            MMsg::Setup(backup) => {
                if !self.mfa.installed || self.mfa.vault {
                    return Task::none();
                }
                self.open_mfa_setup(String::new(), backup)
            }
            MMsg::Found(app, backup) => {
                if self.mfa.vault {
                    return Task::none();
                }
                let kind = match &backup {
                    Some(b) => Kind::Install(b.clone()),
                    None => Kind::Create,
                };
                self.open_mfa_card(kind, app, backup)
            }
            MMsg::Switch(kind) => {
                if self.mfa_dialog.is_none() {
                    return Task::none();
                }
                self.open_mfa_card(kind, String::new(), None)
            }
            MMsg::Input(s) => {
                if let Some(d) = self.mfa_dialog.as_mut() {
                    let mut old = std::mem::replace(&mut d.input, s);
                    crate::lock::pam::wipe(&mut old);
                    d.message = None;
                }
                Task::none()
            }
            MMsg::Confirm(s) => {
                if let Some(d) = self.mfa_dialog.as_mut() {
                    let mut old = std::mem::replace(&mut d.confirm, s);
                    crate::lock::pam::wipe(&mut old);
                    d.message = None;
                }
                Task::none()
            }
            MMsg::Next => iced::widget::operation::focus(CONFIRM_ID),
            MMsg::Submit => {
                let Some(d) = self.mfa_dialog.as_mut() else {
                    return Task::none();
                };
                if d.busy {
                    return Task::none();
                }
                if let Kind::Ready { .. } = d.kind {
                    return self.close_mfa_dialog();
                }
                if d.input.is_empty() {
                    return Task::none();
                }
                if d.kind == Kind::Create
                    && let Some(issue) = create_issue(&d.input, &d.confirm)
                {
                    d.message = Some(issue);
                    return iced::widget::operation::focus(if d.confirm.is_empty() {
                        CONFIRM_ID
                    } else {
                        INPUT_ID
                    });
                }
                d.busy = true;
                d.message = None;
                let pw = std::mem::take(&mut d.input);
                let mut confirm = std::mem::take(&mut d.confirm);
                crate::lock::pam::wipe(&mut confirm);
                match d.kind.clone() {
                    Kind::Unlock => {
                        Task::perform(crate::mymfa::unlock(pw), |r| Msg::Mymfa(MMsg::Unlocked(r)))
                    }
                    Kind::Create => Task::perform(crate::mymfa::create_vault(pw), |r| {
                        Msg::Mymfa(MMsg::Created(r))
                    }),
                    Kind::Install(path) => {
                        Task::perform(crate::mymfa::install_vault(path, pw), |r| {
                            Msg::Mymfa(MMsg::Created(r))
                        })
                    }
                    Kind::Ready { .. } => Task::none(),
                }
            }
            MMsg::Unlocked(Ok(sealed)) => {
                self.mfa.locked = false;
                crate::mymfa::write_state(&self.mfa);
                // Dernière saisie : la carte dit que c'était la dernière.
                if self.mfa_dialog.as_ref().is_some_and(|d| d.sealing) && sealed {
                    return Task::batch([
                        self.show_mfa_ready(true),
                        self.mfa_update(MMsg::Refresh),
                    ]);
                }
                Task::batch([self.close_mfa_dialog(), self.mfa_update(MMsg::Refresh)])
            }
            MMsg::Unlocked(Err((_, msg))) | MMsg::Created(Err(msg)) => {
                if let Some(d) = self.mfa_dialog.as_mut() {
                    d.busy = false;
                    d.message = Some(msg);
                }
                iced::widget::operation::focus(INPUT_ID)
            }
            MMsg::Created(Ok(sealed)) => {
                Task::batch([self.show_mfa_ready(sealed), self.mfa_update(MMsg::Refresh)])
            }
            MMsg::Cancel => self.close_mfa_dialog(),
            MMsg::Lock => Task::perform(crate::mymfa::lock(), |r| {
                if let Err(e) = r {
                    tracing::warn!("{}", crate::tr!("myMFA : {e}", e = e));
                }
                Msg::Mymfa(MMsg::Refresh)
            }),
            MMsg::Refresh => Task::perform(crate::mymfa::state(), |s| {
                Msg::Mymfa(MMsg::Event(Event::State(s)))
            }),
        }
    }

    fn show_mfa_ready(&mut self, sealed: bool) -> Task<Msg> {
        if let Some(d) = self.mfa_dialog.as_mut() {
            d.busy = false;
        }
        self.open_mfa_card(Kind::Ready { sealed }, String::new(), None)
    }

    /// Entrée sur la carte « Coffre prêt » (sans champ pour la recevoir).
    pub(super) fn mfa_ready_open(&self) -> bool {
        self.mfa_dialog
            .as_ref()
            .is_some_and(|d| matches!(d.kind, Kind::Ready { .. }))
    }

    pub(super) fn close_mfa_dialog(&mut self) -> Task<Msg> {
        match self.mfa_dialog.take() {
            Some(mut d) => {
                d.wipe();
                iced::window::close(d.win)
            }
            None => Task::none(),
        }
    }

    pub(super) fn view_mfa_dialog(&self) -> Element<'_, Msg> {
        let Some(d) = &self.mfa_dialog else {
            return iced::widget::Space::new().into();
        };
        use crate::i18n::tr;
        let th = &self.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let error = color(r.error, 1.0);
        let edge = color(r.edge, 1.0);
        let focus = color(r.focus, 1.0);
        let btn = color(r.button, 1.0);
        let on_btn = color(r.on_button, 1.0);
        let bg = color(th.bg, 1.0);
        let wrap = Msg::Mymfa;
        let failed = d.message.is_some();
        let field = |id: &'static str,
                     placeholder: String,
                     value: &str,
                     on_input: fn(String) -> MMsg,
                     on_submit: MMsg| {
            let mut input = text_input(
                if d.busy {
                    tr("Vérification…").to_string()
                } else {
                    placeholder
                }
                .as_str(),
                value,
            )
            .id(id)
            .secure(true)
            .font(self.font)
            .size(16)
            .padding(12)
            .width(Length::Fill)
            .style(move |_, status| {
                let border = match status {
                    text_input::Status::Focused { .. } => focus,
                    _ if failed => error,
                    _ => edge,
                };
                text_input::Style {
                    background: bg.into(),
                    border: Border::default().rounded(10).width(2).color(border),
                    icon: fg,
                    placeholder: dim,
                    value: fg,
                    selection: Color { a: 0.4, ..focus },
                }
            });
            if !d.busy {
                input = input
                    .on_input(move |s| wrap(on_input(s)))
                    .on_submit(wrap(on_submit));
            }
            input
        };
        let title = |t: String| text(t).font(self.title_font()).size(20).color(fg);
        let para = |t: String| text(t).font(self.font).size(14).color(fg);
        let note = |t: String, c: Color| text(t).font(self.font).size(12).color(c);
        // Lien discret : bascule création ↔ reprise.
        let link = |t: String, to: Kind| {
            button(text(t).font(self.font).size(12).color(focus))
                .padding(0)
                .style(|_: &iced::Theme, _| button::Style::default())
                .on_press_maybe((!d.busy).then(|| wrap(MMsg::Switch(to))))
        };
        let backup_name = |p: &PathBuf| {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string())
        };
        let ok_style = move |_: &iced::Theme, st: button::Status| button::Style {
            background: Some(
                match st {
                    button::Status::Hovered | button::Status::Pressed => {
                        color(crate::theme::mix(r.button, r.on_button, 0.12), 1.0)
                    }
                    button::Status::Disabled => Color { a: 0.5, ..btn },
                    _ => btn,
                }
                .into(),
            ),
            text_color: on_btn,
            border: Border::default().rounded(10).width(2).color(btn),
            ..Default::default()
        };
        let cancel_style = move |_: &iced::Theme, st: button::Status| button::Style {
            background: matches!(st, button::Status::Hovered | button::Status::Pressed)
                .then(|| Color { a: 0.12, ..fg }.into()),
            text_color: fg,
            border: Border::default().rounded(10).width(2).color(edge),
            ..Default::default()
        };
        let actions = |cancel: Option<String>, ok: String, enabled: bool| {
            let mut ok = button(text(ok).font(self.font).size(14))
                .padding([8, 16])
                .style(ok_style);
            if enabled && !d.busy {
                ok = ok.on_press(wrap(MMsg::Submit));
            }
            let mut r = row![iced::widget::Space::new().width(Length::Fill)]
                .spacing(10)
                .align_y(Alignment::Center);
            if let Some(c) = cancel {
                r = r.push(
                    button(text(c).font(self.font).size(14))
                        .padding([8, 16])
                        .style(cancel_style)
                        .on_press(wrap(MMsg::Cancel)),
                );
            }
            r.push(ok)
        };
        let waiting = (!d.app.is_empty())
            .then(|| crate::tr!("« {a} » demande un secret du trousseau.", a = d.app));

        let col = match &d.kind {
            Kind::Unlock => {
                let why = waiting.unwrap_or_else(|| {
                    if d.sealing {
                        tr("Une saisie, cette fois seulement : il s'ouvrira ensuite seul à la connexion.")
                            .to_string()
                    } else {
                        tr("Codes 2FA, mots de passe et secrets des applications.").to_string()
                    }
                });
                let mut col = column![
                    title(tr("Déverrouiller le coffre myMFA").to_string()),
                    para(why),
                    field(
                        INPUT_ID,
                        tr("Mot de passe du coffre").to_string(),
                        &d.input,
                        MMsg::Input,
                        MMsg::Submit
                    ),
                ]
                .spacing(12);
                if let Some(m) = &d.message {
                    col = col.push(note(m.clone(), error));
                }
                col.push(actions(
                    Some(tr("Annuler").to_string()),
                    tr("Déverrouiller").to_string(),
                    true,
                ))
            }
            Kind::Create => {
                let why = match waiting {
                    Some(w) => format!(
                        "{w} {}",
                        tr("Le trousseau a d'abord besoin de son coffre.")
                    ),
                    None => tr(
                        "Codes 2FA, mots de passe et secrets des applications, chiffrés sur cet ordinateur. Il s'ouvrira ensuite seul à la connexion.",
                    )
                    .to_string(),
                };
                let issue = create_issue(&d.input, &d.confirm);
                let (hint, hint_color) = match (&d.message, &issue) {
                    (Some(m), _) => (m.clone(), error),
                    (None, Some(i)) if !d.input.is_empty() && !d.confirm.is_empty() => {
                        (i.clone(), error)
                    }
                    (None, Some(i)) if !d.input.is_empty() => (i.clone(), dim),
                    _ => (
                        crate::tr!(
                            "{n} caractères au moins. Retenez-le : rien ne permet de le retrouver.",
                            n = crate::mymfa::MIN_LEN
                        ),
                        dim,
                    ),
                };
                let mut col = column![
                    title(tr("Créer votre coffre myMFA").to_string()),
                    para(why),
                    field(
                        INPUT_ID,
                        tr("Mot de passe du coffre").to_string(),
                        &d.input,
                        MMsg::Input,
                        MMsg::Next
                    ),
                    field(
                        CONFIRM_ID,
                        tr("Confirmation").to_string(),
                        &d.confirm,
                        MMsg::Confirm,
                        MMsg::Submit
                    ),
                    note(hint, hint_color),
                ]
                .spacing(12);
                col = col.push(match &d.backup {
                    Some(b) => Element::from(link(
                        crate::tr!(
                            "Déjà un coffre sur le web ? Reprendre {f}",
                            f = backup_name(b)
                        ),
                        Kind::Install(b.clone()),
                    )),
                    None => note(
                        tr("Déjà un coffre sur le web ? Exportez mymfa-vault.json (Réglages → Sauvegarde) dans Téléchargements, puis rouvrez cette carte.")
                            .to_string(),
                        dim,
                    )
                    .into(),
                });
                col.push(actions(
                    Some(tr("Plus tard").to_string()),
                    tr("Créer le coffre").to_string(),
                    issue.is_none(),
                ))
            }
            Kind::Install(path) => {
                let mut col = column![
                    title(tr("Reprendre votre coffre du web").to_string()),
                    para(crate::tr!(
                        "Sauvegarde trouvée : {f}. Son mot de passe est celui du web.",
                        f = backup_name(path)
                    )),
                    field(
                        INPUT_ID,
                        tr("Mot de passe du coffre").to_string(),
                        &d.input,
                        MMsg::Input,
                        MMsg::Submit
                    ),
                ]
                .spacing(12);
                if let Some(m) = &d.message {
                    col = col.push(note(m.clone(), error));
                }
                col = col.push(link(
                    tr("Créer plutôt un nouveau coffre").to_string(),
                    Kind::Create,
                ));
                col.push(actions(
                    Some(tr("Plus tard").to_string()),
                    tr("Installer").to_string(),
                    !d.input.is_empty(),
                ))
            }
            Kind::Ready { sealed } => column![
                title(tr("Coffre prêt").to_string()),
                para(if *sealed {
                    tr("Il s'ouvrira seul à chaque connexion : votre mot de passe de session suffit.")
                        .to_string()
                } else {
                    tr("Il est ouvert pour cette session. Pour qu'il s'ouvre seul ensuite, connectez-vous par l'écran de connexion de hyprdesk : il vous sera demandé une seule fois.")
                        .to_string()
                }),
                note(
                    tr("Codes 2FA : « :2fa » dans le lanceur · mots de passe : « :mdp ».")
                        .to_string(),
                    dim
                ),
                actions(None, tr("Terminé").to_string(), true),
            ]
            .spacing(12),
        };
        let card = container(col)
            .padding(22)
            .width(WIDTH)
            .height(d.kind.height())
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(super::design::RADIUS)
                    .width(1)
                    .color(edge),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 22.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        container(card).padding(SHADOW).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nouveau_mot_de_passe() {
        assert!(create_issue("court", "court").is_some());
        assert!(create_issue("assez long !", "assez long ?").is_some());
        assert_eq!(create_issue("assez long !", "assez long !"), None);
    }
}
