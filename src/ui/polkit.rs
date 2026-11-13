//! Fenêtre d'authentification polkit du démon : l'agent (`crate::polkit`)
//! est enregistré par un abonnement ; chaque saisie demandée par polkit
//! ouvre une carte du thème au centre de l'écran actif (message de
//! l'action, compte, mot de passe), le clavier pris pour la saisie.
//!
//! La réponse repart par un canal à usage unique vers l'agent, qui l'écrit à
//! l'outil de polkit puis l'efface ; ici, la saisie est effacée dès l'envoi.

use std::sync::{Arc, Mutex};

use iced::futures::{SinkExt, Stream};
use iced::widget::{button, column, container, row, text, text_input};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};
use tokio::sync::oneshot;

use super::{App, Msg, color};
use crate::polkit::{Prompter, Reply, Request};

pub const NAMESPACE: &str = "hyprdesk-polkit";
const INPUT_ID: &str = "hyprdesk-polkit-password";
const WIDTH: f32 = 460.0;
const HEIGHT: f32 = 268.0;
const SHADOW: f32 = 18.0;

/// Réponse attendue par l'agent (une seule fois).
#[derive(Clone)]
pub struct Responder(Arc<Mutex<Option<oneshot::Sender<Option<String>>>>>);

impl std::fmt::Debug for Responder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Responder")
    }
}

impl Responder {
    /// Sans agent au bout (aperçu) : la réponse se perd.
    pub fn detached() -> Responder {
        Responder(Arc::new(Mutex::new(None)))
    }

    /// Envoie la saisie (`None` : annulé) ; sans effet si déjà répondu.
    pub fn send(&self, v: Option<String>) {
        if let Some(tx) = self.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = tx.send(v);
        }
    }
}

/// Ce que l'agent demande à l'interface.
#[derive(Debug, Clone)]
pub enum Event {
    Prompt {
        req: Request,
        text: String,
        echo: bool,
        reply: Responder,
    },
    Show {
        text: String,
        error: bool,
    },
    Done {
        ok: bool,
    },
    /// L'état de l'agent a changé (`crate::polkit::state()`).
    State,
}

#[derive(Debug, Clone)]
pub enum PMsg {
    Event(Event),
    Input(String),
    Submit,
    Cancel,
}

/// Prompter de l'agent : relaie vers l'interface et attend la réponse.
struct UiPrompter {
    tx: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl Prompter for UiPrompter {
    fn prompt<'a>(&'a self, req: &'a Request, text: &'a str, echo: bool) -> Reply<'a> {
        Box::pin(async move {
            let (tx, rx) = oneshot::channel();
            let ev = Event::Prompt {
                req: req.clone(),
                text: text.to_string(),
                echo,
                reply: Responder(Arc::new(Mutex::new(Some(tx)))),
            };
            self.tx.send(ev).ok()?;
            rx.await.ok().flatten()
        })
    }

    fn show(&self, _req: &Request, text: &str, error: bool) {
        let _ = self.tx.send(Event::Show {
            text: text.to_string(),
            error,
        });
    }

    fn done(&self, _req: &Request, ok: bool) {
        let _ = self.tx.send(Event::Done { ok });
    }
}

/// Abonnement : enregistre l'agent auprès de polkit pour la session et
/// relaie ses demandes. Un autre agent tient la session (polkit-kde…) :
/// « occupé », et un nouvel essai toutes les 5 s, pour prendre la main dès
/// qu'il s'arrête ; une autre erreur : nouvel essai, de plus en plus espacé.
pub fn stream() -> impl Stream<Item = Event> {
    iced::stream::channel(
        8,
        async move |mut out: iced::futures::channel::mpsc::Sender<Event>| {
            use crate::polkit::{AgentState, is_busy_error, retry_delay, set_state};
            let mut attempt = 0u32;
            let mut last: Option<AgentState> = None;
            loop {
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                match crate::polkit::register(Arc::new(UiPrompter { tx })).await {
                    Ok((conn, session)) => {
                        // Écrit seulement après la réponse positive de polkit.
                        tracing::info!(
                            "{}",
                            crate::tr!("agent polkit en place (session {s})", s = session)
                        );
                        set_state(AgentState::Active(session));
                        let _ = out.send(Event::State).await;
                        while let Some(ev) = rx.recv().await {
                            if out.send(ev).await.is_err() {
                                break;
                            }
                        }
                        drop(conn);
                        tracing::error!(
                            "{}",
                            crate::i18n::tr(
                                "agent polkit : relais interrompu, nouvel enregistrement"
                            )
                        );
                        attempt = 0;
                        last = None;
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        let busy = is_busy_error(&msg);
                        let st = if busy {
                            AgentState::Busy(crate::takeover::polkit_holder())
                        } else {
                            AgentState::Error(msg.clone())
                        };
                        // Journalisé à chaque changement d'état, pas à chaque essai.
                        if last.as_ref() != Some(&st) {
                            tracing::warn!(
                                "{}",
                                crate::tr!(
                                    "agent polkit non enregistré : {state} — {e}",
                                    state = st.text(),
                                    e = msg
                                )
                            );
                            set_state(st.clone());
                            let _ = out.send(Event::State).await;
                            last = Some(st);
                        }
                        tokio::time::sleep(retry_delay(attempt, busy)).await;
                        if !busy {
                            attempt = attempt.saturating_add(1);
                        }
                    }
                }
            }
        },
    )
}

/// Carte ouverte.
pub struct Dialog {
    pub win: Id,
    pub req: Request,
    pub text: String,
    pub echo: bool,
    pub input: String,
    /// Dernier message de PAM (texte, erreur).
    pub message: Option<(String, bool)>,
    pub reply: Option<Responder>,
}

impl App {
    pub(super) fn polkit_update(&mut self, m: PMsg) -> Task<Msg> {
        match m {
            // État de l'agent changé : `hyprdesk status` le reflète.
            PMsg::Event(Event::State) => {
                self.publish_status();
                Task::none()
            }
            PMsg::Event(Event::Prompt {
                req,
                text,
                echo,
                reply,
            }) => {
                let focus = iced::widget::operation::focus(INPUT_ID);
                if let Some(d) = self.polkit.as_mut() {
                    // Nouvelle tentative : même carte, message gardé.
                    d.req = req;
                    d.text = text;
                    d.echo = echo;
                    crate::lock::pam::wipe(&mut d.input);
                    d.reply = Some(reply);
                    return focus;
                }
                let win = Id::unique();
                self.polkit = Some(Dialog {
                    win,
                    req,
                    text,
                    echo,
                    input: String::new(),
                    message: None,
                    reply: Some(reply),
                });
                Task::batch([
                    Task::done(Msg::NewLayerShell {
                        settings: NewLayerShellSettings {
                            size: Some((
                                (WIDTH + 2.0 * SHADOW) as u32,
                                (HEIGHT + 2.0 * SHADOW) as u32,
                            )),
                            layer: Layer::Overlay,
                            // Sans ancre : centrée par le compositeur.
                            anchor: LsAnchor::empty(),
                            exclusive_zone: Some(-1),
                            // À la demande (comme la carte myMFA, MLG-106) : un clic
                            // ailleurs reprend le clavier, jamais de session figée.
                            keyboard_interactivity: KeyboardInteractivity::OnDemand,
                            output_option: OutputOption::Active,
                            namespace: Some(NAMESPACE.into()),
                            ..Default::default()
                        },
                        id: win,
                    }),
                    focus,
                ])
            }
            PMsg::Event(Event::Show { text, error }) => {
                if let Some(d) = self.polkit.as_mut() {
                    d.message = Some((text, error));
                }
                Task::none()
            }
            PMsg::Event(Event::Done { ok }) => {
                tracing::info!(
                    "{}",
                    if ok {
                        crate::i18n::tr("polkit : autorisé")
                    } else {
                        crate::i18n::tr("polkit : refusé ou annulé")
                    }
                );
                self.close_polkit()
            }
            PMsg::Input(s) => {
                if let Some(d) = self.polkit.as_mut() {
                    let mut old = std::mem::replace(&mut d.input, s);
                    crate::lock::pam::wipe(&mut old);
                }
                Task::none()
            }
            PMsg::Submit => {
                if let Some(d) = self.polkit.as_mut()
                    && let Some(r) = d.reply.take()
                {
                    d.message = None;
                    r.send(Some(std::mem::take(&mut d.input)));
                }
                Task::none()
            }
            PMsg::Cancel => {
                if let Some(r) = self.polkit.as_mut().and_then(|d| d.reply.take()) {
                    r.send(None);
                }
                self.close_polkit()
            }
        }
    }

    /// Ferme la carte ; une saisie encore attendue est annulée.
    pub(super) fn close_polkit(&mut self) -> Task<Msg> {
        match self.polkit.take() {
            Some(mut d) => {
                if let Some(r) = d.reply.take() {
                    r.send(None);
                }
                crate::lock::pam::wipe(&mut d.input);
                iced::window::close(d.win)
            }
            None => Task::none(),
        }
    }

    pub(super) fn view_polkit(&self) -> Element<'_, Msg> {
        let Some(d) = &self.polkit else {
            return iced::widget::Space::new().into();
        };
        let th = &self.cfg.general.theme;
        // Contrastes WCAG AA garantis (texte 4,5:1, champ et boutons 3:1),
        // sur un fond opaque : la fenêtre ne dépend pas de ce qu'il y a dessous.
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let error = color(r.error, 1.0);
        let edge = color(r.edge, 1.0);
        let focus = color(r.focus, 1.0);
        let btn = color(r.button, 1.0);
        let on_btn = color(r.on_button, 1.0);
        let bg = color(th.bg, 1.0);
        let waiting = d.reply.is_none();
        let wrap = |m: PMsg| Msg::Polkit(m);
        let mut input = text_input(
            if waiting {
                crate::i18n::tr("Vérification…")
            } else {
                d.text.trim_end().trim_end_matches(':').trim_end()
            },
            &d.input,
        )
        .id(INPUT_ID)
        .secure(!d.echo)
        .font(self.font)
        .size(16)
        .padding(12)
        .width(Length::Fill)
        .style(move |_, status| {
            let border = match status {
                text_input::Status::Focused { .. } => focus,
                _ if d.message.as_ref().is_some_and(|(_, e)| *e) => error,
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
        if !waiting {
            input = input
                .on_input(move |s| wrap(PMsg::Input(s)))
                .on_submit(wrap(PMsg::Submit));
        }
        let mut col = column![
            text(crate::i18n::tr("Authentification requise"))
                .font(self.title_font())
                .size(20)
                .color(fg),
            text(d.req.message.clone())
                .font(self.font)
                .size(14)
                .color(fg),
            text(crate::tr!(
                "Compte : {u} · {a}",
                u = d.req.user,
                a = d.req.action
            ))
            .font(self.font)
            .size(11)
            .color(dim),
            input,
        ]
        .spacing(12);
        if let Some((m, is_error)) = &d.message {
            col = col.push(
                text(m.clone())
                    .font(self.font)
                    .size(12)
                    .color(if *is_error { error } else { dim }),
            );
        }
        // Désactivé (vérification en cours) : mêmes couleurs, donc mêmes
        // contrastes ; seul le contour passe à la couleur des champs.
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
            border: Border::default().rounded(10).width(2).color(
                if matches!(st, button::Status::Disabled) {
                    edge
                } else {
                    btn
                },
            ),
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
            text(crate::i18n::tr("Authentifier"))
                .font(self.font)
                .size(14),
        )
        .padding([8, 16])
        .style(ok_style);
        if !waiting {
            ok = ok.on_press(wrap(PMsg::Submit));
        }
        col = col.push(
            row![
                iced::widget::Space::new().width(Length::Fill),
                button(text(crate::i18n::tr("Annuler")).font(self.font).size(14))
                    .padding([8, 16])
                    .style(cancel_style)
                    .on_press(wrap(PMsg::Cancel)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn la_reponse_rejoint_l_agent() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let p = UiPrompter { tx };
        let req = Request {
            action: "a".into(),
            message: "m".into(),
            user: "alice".into(),
            cookie: "c".into(),
            attempt: 1,
        };
        let ask = tokio::spawn(async move { p.prompt(&req, "Mot de passe :", false).await });
        let Some(Event::Prompt { reply, echo, .. }) = rx.recv().await else {
            panic!("pas de demande");
        };
        assert!(!echo);
        reply.send(Some("s3cret".into()));
        // Une seconde réponse est ignorée.
        reply.send(Some("autre".into()));
        assert_eq!(ask.await.unwrap().as_deref(), Some("s3cret"));
    }

    #[tokio::test]
    async fn annuler_rend_none() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let p = UiPrompter { tx };
        let req = Request {
            action: "a".into(),
            message: "m".into(),
            user: "alice".into(),
            cookie: "c".into(),
            attempt: 1,
        };
        let ask = tokio::spawn(async move { p.prompt(&req, "Mot de passe :", false).await });
        let Some(Event::Prompt { reply, .. }) = rx.recv().await else {
            panic!("pas de demande");
        };
        drop(reply);
        assert_eq!(ask.await.unwrap(), None);
    }
}
