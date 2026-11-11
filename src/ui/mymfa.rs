//! Trousseau myMFA dans le démon (MLG-44) : abonnement au bus de session
//! (état du coffre, demandes de déverrouillage des applications) et carte de
//! déverrouillage au centre de l'écran actif, dans le style de la carte
//! polkit. Le mot de passe part au démon mymfa (`Unlock`) puis est effacé.

use iced::futures::{SinkExt, Stream, StreamExt};
use iced::widget::{button, column, container, row, text, text_input};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};
use crate::mymfa::State;

pub const NAMESPACE: &str = "hyprdesk-mymfa";
const INPUT_ID: &str = "hyprdesk-mymfa-password";
const WIDTH: f32 = 460.0;
const HEIGHT: f32 = 250.0;
const SHADOW: f32 = 18.0;

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
    Input(String),
    Submit,
    Cancel,
    /// Réponse du démon : `Err((mauvais mot de passe, message))`.
    Unlocked(Result<(), (bool, String)>),
    Lock,
    Refresh,
}

/// Carte ouverte.
pub struct Dialog {
    pub win: Id,
    /// Application qui attend (vide : demande de l'utilisateur).
    pub app: String,
    pub input: String,
    pub message: Option<String>,
    pub busy: bool,
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
                loop {
                    tokio::select! {
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
    fn open_mfa_dialog(&mut self, app: String) -> Task<Msg> {
        let focus = iced::widget::operation::focus(INPUT_ID);
        if let Some(d) = self.mfa_dialog.as_mut() {
            if d.app.is_empty() {
                d.app = app;
            }
            return focus;
        }
        let win = Id::unique();
        self.mfa_dialog = Some(Dialog {
            win,
            app,
            input: String::new(),
            message: None,
            busy: false,
        });
        Task::batch([
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((
                        (WIDTH + 2.0 * SHADOW) as u32,
                        (HEIGHT + 2.0 * SHADOW) as u32,
                    )),
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

    pub(super) fn mfa_update(&mut self, m: MMsg) -> Task<Msg> {
        match m {
            MMsg::Event(Event::State(s)) => {
                let first = !self.mfa.installed && s.installed;
                crate::mymfa::write_state(&s);
                let unlocked = !s.locked && s.vault;
                self.mfa = s;
                if first {
                    // hyprdesk répond aux demandes : invite de repli du démon coupée.
                    std::thread::spawn(crate::mymfa::ensure_dropin);
                }
                // Coffre ouvert ailleurs (CLI, connexion) : carte inutile.
                if unlocked && self.mfa_dialog.as_ref().is_some_and(|d| !d.busy) {
                    return self.close_mfa_dialog();
                }
                Task::none()
            }
            MMsg::Event(Event::UnlockRequested(app)) => {
                if self.mfa.vault && !self.mfa.locked {
                    return Task::none();
                }
                self.open_mfa_dialog(app)
            }
            MMsg::Ask => {
                if !self.mfa.installed || !self.mfa.vault || !self.mfa.locked {
                    return Task::none();
                }
                self.open_mfa_dialog(String::new())
            }
            MMsg::Input(s) => {
                if let Some(d) = self.mfa_dialog.as_mut() {
                    let mut old = std::mem::replace(&mut d.input, s);
                    crate::lock::pam::wipe(&mut old);
                }
                Task::none()
            }
            MMsg::Submit => {
                let Some(d) = self.mfa_dialog.as_mut() else {
                    return Task::none();
                };
                if d.busy || d.input.is_empty() {
                    return Task::none();
                }
                d.busy = true;
                d.message = None;
                let pw = std::mem::take(&mut d.input);
                Task::perform(crate::mymfa::unlock(pw), |r| Msg::Mymfa(MMsg::Unlocked(r)))
            }
            MMsg::Unlocked(Ok(())) => {
                self.mfa.locked = false;
                crate::mymfa::write_state(&self.mfa);
                self.close_mfa_dialog()
            }
            MMsg::Unlocked(Err((_, msg))) => {
                if let Some(d) = self.mfa_dialog.as_mut() {
                    d.busy = false;
                    d.message = Some(msg);
                }
                iced::widget::operation::focus(INPUT_ID)
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

    pub(super) fn close_mfa_dialog(&mut self) -> Task<Msg> {
        match self.mfa_dialog.take() {
            Some(mut d) => {
                crate::lock::pam::wipe(&mut d.input);
                iced::window::close(d.win)
            }
            None => Task::none(),
        }
    }

    pub(super) fn view_mfa_dialog(&self) -> Element<'_, Msg> {
        let Some(d) = &self.mfa_dialog else {
            return iced::widget::Space::new().into();
        };
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
        let mut input = text_input(
            if d.busy {
                crate::i18n::tr("Vérification…")
            } else {
                crate::i18n::tr("Mot de passe du coffre")
            },
            &d.input,
        )
        .id(INPUT_ID)
        .secure(true)
        .font(self.font)
        .size(16)
        .padding(12)
        .width(Length::Fill)
        .style(move |_, status| {
            let border = match status {
                text_input::Status::Focused { .. } => focus,
                _ if d.message.is_some() => error,
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
                .on_input(move |s| wrap(MMsg::Input(s)))
                .on_submit(wrap(MMsg::Submit));
        }
        let why = if d.app.is_empty() {
            crate::i18n::tr("Codes 2FA, mots de passe et secrets des applications.").to_string()
        } else {
            crate::tr!("« {a} » demande un secret du trousseau.", a = d.app)
        };
        let mut col = column![
            text(crate::i18n::tr("Déverrouiller le coffre myMFA"))
                .font(self.title_font())
                .size(20)
                .color(fg),
            text(why).font(self.font).size(14).color(fg),
            input,
        ]
        .spacing(12);
        if let Some(m) = &d.message {
            col = col.push(text(m.clone()).font(self.font).size(12).color(error));
        }
        let ok_style = move |_: &iced::Theme, st: button::Status| button::Style {
            background: Some(
                match st {
                    button::Status::Hovered | button::Status::Pressed => {
                        color(crate::theme::mix(r.button, r.on_button, 0.12), 1.0)
                    }
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
        let mut ok = button(
            text(crate::i18n::tr("Déverrouiller"))
                .font(self.font)
                .size(14),
        )
        .padding([8, 16])
        .style(ok_style);
        if !d.busy {
            ok = ok.on_press(wrap(MMsg::Submit));
        }
        col = col.push(
            row![
                iced::widget::Space::new().width(Length::Fill),
                button(text(crate::i18n::tr("Annuler")).font(self.font).size(14))
                    .padding([8, 16])
                    .style(cancel_style)
                    .on_press(wrap(MMsg::Cancel)),
                ok,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
        let card = container(col)
            .padding(22)
            .width(WIDTH)
            .height(HEIGHT)
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
