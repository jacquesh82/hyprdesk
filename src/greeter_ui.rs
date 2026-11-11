//! `hyprdesk greeter` : écran de connexion graphique pour greetd (MLG-23),
//! dans le style de l'écran de verrouillage. Une surface layer-shell plein
//! écran (clavier exclusif) sous le compositeur que greetd lance pour le
//! greeter (cage, ou un Hyprland minimal).
//!
//! Heure et date, identifiant, session (wayland-sessions, xsessions), puis
//! les questions de greetd une à une (mot de passe masqué, code, messages).
//! La conversation avance par étapes (`greeter::step`), chaque échange sur
//! un fil à part ; la réponse secrète est effacée après envoi. Session
//! lancée : le greeter quitte et greetd prend le relais.
//!
//! `hyprdesk greeter --demo` : le même écran sans greetd, pour l'essayer
//! dans une session ouverte (aucun mot de passe vérifié, rien n'est lancé ;
//! Échap ou « Quitter la démo » le referme).

use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::greeter::{self, AuthKind, Request, Response, Session, Step};
use crate::i18n::text;
use iced::widget::{Space, button, column, container, pick_list, row, text_input};
use iced::{Alignment, Border, Color, Element, Font, Length, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::to_layer_message;

const USER_ID: &str = "greeter-user";
const ANSWER_ID: &str = "greeter-answer";

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum GMsg {
    Tick,
    User(String),
    Session(SessionChoice),
    /// Identifiant validé : ouvre la conversation.
    Begin,
    Answer(String),
    /// Réponse validée.
    Send,
    Reply(Result<Response, String>),
    /// Annulée côté greetd après un échec : retour à l'identifiant.
    Cancelled(String),
    /// Démo : Échap ou « Quitter la démo ».
    Quit,
}

/// Entrée de la liste des sessions.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionChoice(pub usize, pub String);

impl std::fmt::Display for SessionChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    /// Saisie de l'identifiant.
    User,
    /// Échange en cours avec greetd.
    Waiting,
    /// Question posée.
    Ask(AuthKind, String),
}

pub struct Greeter {
    font: Font,
    title_font: Font,
    theme: crate::config::Theme,
    sessions: Vec<Session>,
    session: usize,
    user: String,
    answer: String,
    phase: Phase,
    started: bool,
    messages: Vec<String>,
    error: Option<String>,
    stream: Arc<Mutex<Option<UnixStream>>>,
    /// Démo sans greetd (`--demo`).
    demo: bool,
}

fn rgba(c: crate::config::Rgba, a: f32) -> Color {
    Color::from_rgba8(c.0, c.1, c.2, (c.3 as f32 / 255.0) * a)
}

/// Échange avec greetd hors du fil de l'interface.
fn send(stream: Arc<Mutex<Option<UnixStream>>>, req: Request) -> Task<GMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let mut guard = stream.lock().unwrap_or_else(|e| e.into_inner());
                let Some(s) = guard.as_mut() else {
                    return Err(crate::i18n::tr("greetd injoignable.").to_string());
                };
                let mut req = req;
                let r = greeter::exchange(s, &req).map_err(|e| e.to_string());
                if let Request::PostAuthMessageResponse { response: Some(a) } = &mut req {
                    crate::lock::pam::wipe(a);
                }
                r
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()))
        },
        GMsg::Reply,
    )
}

impl Greeter {
    pub fn new() -> Self {
        let sessions = greeter::sessions();
        Greeter {
            font: crate::fonts::DEFAULT,
            title_font: Font {
                weight: iced::font::Weight::Bold,
                ..crate::fonts::DEFAULT
            },
            theme: crate::config::Theme::default(),
            sessions,
            session: 0,
            user: String::new(),
            answer: String::new(),
            phase: Phase::User,
            started: false,
            messages: Vec::new(),
            error: None,
            stream: Arc::new(Mutex::new(None)),
            demo: false,
        }
    }

    fn demo() -> Self {
        Greeter {
            demo: true,
            user: std::env::var("USER").unwrap_or_default(),
            ..Greeter::new()
        }
    }

    /// Démo : ce qui se passerait à la validation du mot de passe.
    fn demo_outcome(&self) -> String {
        crate::tr!(
            "Démo : la session « {s} » serait ouverte pour {u}.",
            s = self
                .sessions
                .get(self.session)
                .map(|s| s.name.as_str())
                .unwrap_or("sh"),
            u = self.user.trim()
        )
    }

    /// État d'exemple pour `hyprdesk preview` (mot de passe demandé).
    fn preview() -> Self {
        let mut g = Greeter::new();
        g.sessions = vec![
            Session {
                id: "hyprland".into(),
                name: "Hyprland".into(),
                exec: "Hyprland".into(),
                wayland: true,
            },
            Session {
                id: "gnome".into(),
                name: "GNOME".into(),
                exec: "gnome-session".into(),
                wayland: true,
            },
        ];
        g.user = "jacques".into();
        g.answer = "secret".into();
        g.phase = Phase::Ask(AuthKind::Secret, "Mot de passe :".into());
        g.messages = vec!["Touchez le lecteur d'empreintes ou saisissez le mot de passe.".into()];
        g
    }

    fn fail(&mut self, msg: String) -> Task<GMsg> {
        self.phase = Phase::Waiting;
        self.started = false;
        let stream = self.stream.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let mut guard = stream.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(s) = guard.as_mut() {
                        let _ = greeter::exchange(s, &Request::CancelSession);
                    }
                    *guard = None;
                })
                .await
                .ok();
                msg
            },
            GMsg::Cancelled,
        )
    }

    pub fn update(&mut self, msg: GMsg) -> Task<GMsg> {
        match msg {
            GMsg::Tick => Task::none(),
            GMsg::User(u) => {
                self.user = u;
                self.error = None;
                Task::none()
            }
            GMsg::Session(SessionChoice(i, _)) => {
                self.session = i;
                Task::none()
            }
            GMsg::Begin => {
                if self.user.trim().is_empty() || self.phase != Phase::User {
                    return Task::none();
                }
                if self.demo {
                    self.messages.clear();
                    self.error = None;
                    self.phase =
                        Phase::Ask(AuthKind::Secret, crate::i18n::tr("Mot de passe :").into());
                    return iced::widget::operation::focus(ANSWER_ID);
                }
                let Some(sock) = std::env::var_os("GREETD_SOCK") else {
                    self.error =
                        Some(crate::i18n::tr("GREETD_SOCK absent : lancer depuis greetd.").into());
                    return Task::none();
                };
                match UnixStream::connect(sock) {
                    Ok(s) => {
                        *self.stream.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
                        self.phase = Phase::Waiting;
                        self.messages.clear();
                        self.error = None;
                        send(
                            self.stream.clone(),
                            Request::CreateSession {
                                username: self.user.trim().to_string(),
                            },
                        )
                    }
                    Err(e) => {
                        self.error = Some(e.to_string());
                        Task::none()
                    }
                }
            }
            GMsg::Answer(a) => {
                self.answer = a;
                Task::none()
            }
            GMsg::Send => {
                if !matches!(self.phase, Phase::Ask(..)) {
                    return Task::none();
                }
                if self.demo {
                    let mut a = std::mem::take(&mut self.answer);
                    crate::lock::pam::wipe(&mut a);
                    self.phase = Phase::User;
                    self.messages = vec![self.demo_outcome()];
                    return iced::widget::operation::focus(USER_ID);
                }
                self.phase = Phase::Waiting;
                let a = std::mem::take(&mut self.answer);
                send(
                    self.stream.clone(),
                    Request::PostAuthMessageResponse { response: Some(a) },
                )
            }
            GMsg::Reply(Err(e)) => self.fail(e),
            GMsg::Reply(Ok(r)) => match greeter::step(self.started, r) {
                Step::Ask(k, t) => {
                    self.phase = Phase::Ask(k, t);
                    iced::widget::operation::focus(ANSWER_ID)
                }
                Step::Show(_, t) => {
                    self.messages.push(t);
                    send(
                        self.stream.clone(),
                        Request::PostAuthMessageResponse { response: None },
                    )
                }
                Step::Start => {
                    self.started = true;
                    let s = self.sessions.get(self.session).cloned();
                    let (cmd, env) = match &s {
                        Some(s) => (
                            vec!["sh".into(), "-c".into(), format!("exec {}", s.exec)],
                            greeter::session_env(s),
                        ),
                        None => (vec!["sh".into()], Vec::new()),
                    };
                    send(self.stream.clone(), Request::StartSession { cmd, env })
                }
                Step::Done => iced::exit(),
                Step::Fail(m) => self.fail(m),
            },
            GMsg::Quit if self.demo => iced::exit(),
            GMsg::Cancelled(m) => {
                self.phase = Phase::User;
                self.error = Some(m);
                self.answer.clear();
                iced::widget::operation::focus(USER_ID)
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, GMsg> {
        let th = &self.theme;
        let fg = rgba(th.fg, 1.0);
        let dim = rgba(th.fg, 0.65);
        let accent = rgba(th.accent, 1.0);
        let field_bg = rgba(th.bg, 0.6);
        let now = chrono::Local::now();
        let field = move |_: &iced::Theme, _| text_input::Style {
            background: field_bg.into(),
            border: Border::default().rounded(14).width(1).color(accent),
            icon: fg,
            placeholder: dim,
            value: fg,
            selection: Color { a: 0.35, ..accent },
        };
        let mut col = column![
            text(now.format("%H:%M").to_string())
                .font(self.title_font)
                .size(110)
                .color(fg),
            text(
                now.format_localized("%A %-d %B", crate::lock::locale())
                    .to_string()
            )
            .font(self.font)
            .size(22)
            .color(dim),
            Space::new().height(28),
        ]
        .align_x(Alignment::Center)
        .spacing(6)
        .width(Length::Fixed(380.0));
        let choices: Vec<SessionChoice> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, s)| SessionChoice(i, s.name.clone()))
            .collect();
        let current = choices.get(self.session).cloned();
        let user_step = self.phase == Phase::User;
        let mut user = text_input(crate::i18n::tr("Identifiant"), &self.user)
            .id(USER_ID)
            .font(self.font)
            .size(18)
            .padding(14)
            .style(field);
        if user_step {
            user = user.on_input(GMsg::User).on_submit(GMsg::Begin);
        }
        col = col.push(user);
        col = col.push(self.session_picker(choices, current, accent, fg, dim));
        for m in &self.messages {
            col = col.push(
                text(m.clone())
                    .font(self.font)
                    .size(14)
                    .color(dim)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
            );
        }
        match &self.phase {
            Phase::Ask(kind, q) => {
                col = col.push(text(q.clone()).font(self.font).size(14).color(fg));
                col = col.push(
                    text_input("", &self.answer)
                        .id(ANSWER_ID)
                        .secure(*kind == AuthKind::Secret)
                        .on_input(GMsg::Answer)
                        .on_submit(GMsg::Send)
                        .font(self.font)
                        .size(18)
                        .padding(14)
                        .style(field),
                );
            }
            Phase::Waiting => {
                col = col.push(
                    text(crate::i18n::tr("Vérification…"))
                        .font(self.font)
                        .size(14)
                        .color(dim),
                );
            }
            Phase::User => {
                col = col.push(
                    button(
                        row![
                            text(crate::i18n::tr("Se connecter"))
                                .font(self.title_font)
                                .size(15)
                        ]
                        .align_y(Alignment::Center),
                    )
                    .padding([10, 18])
                    .on_press(GMsg::Begin),
                );
            }
        }
        if self.demo {
            col = col.push(Space::new().height(12));
            col = col.push(
                text(crate::i18n::tr(
                    "Démo : rien n'est vérifié ni lancé. Échap pour quitter.",
                ))
                .font(self.font)
                .size(12)
                .color(dim),
            );
            col = col.push(
                button(
                    text(crate::i18n::tr("Quitter la démo"))
                        .font(self.font)
                        .size(13),
                )
                .padding([6, 14])
                .on_press(GMsg::Quit),
            );
        }
        if let Some(e) = &self.error {
            col = col.push(
                text(e.clone())
                    .font(self.font)
                    .size(14)
                    .color(Color::from_rgb8(0xff, 0x95, 0x80)),
            );
        }
        let bg = rgba(th.bg, 1.0);
        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                text_color: Some(fg),
                ..Default::default()
            })
            .into()
    }

    /// Choix de l'environnement : un bouton par session (jusqu'à cinq),
    /// sinon une liste déroulante.
    fn session_picker(
        &self,
        choices: Vec<SessionChoice>,
        current: Option<SessionChoice>,
        accent: Color,
        fg: Color,
        dim: Color,
    ) -> Element<'_, GMsg> {
        if choices.len() > 5 {
            return pick_list(choices, current, GMsg::Session)
                .font(self.font)
                .width(Length::Fill)
                .padding(10)
                .into();
        }
        let mut chips = row![].spacing(8);
        for c in choices {
            let on = c.0 == self.session;
            let label = text(c.1.clone())
                .font(if on { self.title_font } else { self.font })
                .size(13)
                .color(if on { fg } else { dim });
            chips = chips.push(
                button(label)
                    .padding([6, 12])
                    .style(move |_, status| {
                        let hover = matches!(status, button::Status::Hovered);
                        button::Style {
                            background: Some(
                                Color {
                                    a: if on {
                                        0.35
                                    } else if hover {
                                        0.15
                                    } else {
                                        0.0
                                    },
                                    ..accent
                                }
                                .into(),
                            ),
                            border: Border::default().rounded(12).width(1).color(Color {
                                a: if on { 1.0 } else { 0.35 },
                                ..accent
                            }),
                            text_color: fg,
                            ..Default::default()
                        }
                    })
                    .on_press(GMsg::Session(c)),
            );
        }
        column![
            text(crate::i18n::tr("Environnement"))
                .font(self.font)
                .size(12)
                .color(dim),
            chips.wrap()
        ]
        .spacing(6)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<GMsg> {
        let tick = iced::time::every(Duration::from_secs(1)).map(|_| GMsg::Tick);
        if !self.demo {
            return tick;
        }
        let esc = iced::event::listen_with(|e, _, _| match e {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                ..
            }) => Some(GMsg::Quit),
            _ => None,
        });
        Subscription::batch([tick, esc])
    }
}

/// Vue d'exemple pour `hyprdesk preview` (scène « connexion »).
pub fn preview_view() -> Element<'static, GMsg> {
    static G: std::sync::OnceLock<Greeter> = std::sync::OnceLock::new();
    G.get_or_init(Greeter::preview).view()
}

/// Point d'entrée de `hyprdesk greeter` (`demo` : sans greetd).
pub fn run(demo: bool) -> std::process::ExitCode {
    // Pas de presse-papiers : rien à coller dans le champ du mot de passe.
    iced_layershell::disable_clipboard();
    let mut app = iced_layershell::application(
        move || {
            let g = if demo {
                Greeter::demo()
            } else {
                Greeter::new()
            };
            (g, iced::widget::operation::focus(USER_ID))
        },
        "hyprdesk-greeter",
        Greeter::update,
        Greeter::view,
    )
    .subscription(Greeter::subscription)
    .default_font(crate::fonts::DEFAULT)
    .layer_settings(LayerShellSettings {
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Overlay,
        exclusive_zone: -1,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        start_mode: StartMode::Active,
        ..Default::default()
    });
    for face in crate::fonts::bundled() {
        app = app.font(face);
    }
    match app.run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", crate::tr!("hyprdesk greeter : {e}", e = e));
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Démo : identifiant, mot de passe, puis ce qui serait lancé ; rien
    /// ne part vers greetd et la réponse est effacée.
    #[test]
    fn demo_sans_greetd() {
        let mut g = Greeter::demo();
        g.user = "alice".into();
        g.sessions = vec![Session {
            id: "hyprland".into(),
            name: "Hyprland".into(),
            exec: "Hyprland".into(),
            wayland: true,
        }];
        let _ = g.update(GMsg::Begin);
        assert!(matches!(g.phase, Phase::Ask(AuthKind::Secret, _)));
        let _ = g.update(GMsg::Answer("secret".into()));
        let _ = g.update(GMsg::Send);
        assert_eq!(g.phase, Phase::User);
        assert!(g.answer.is_empty());
        assert!(g.messages[0].contains("Hyprland") && g.messages[0].contains("alice"));
        assert!(g.stream.lock().unwrap().is_none());
        // Hors démo, Quit ne ferme pas l'écran de connexion.
        let mut real = Greeter::new();
        let _ = real.update(GMsg::Quit);
        assert_eq!(real.phase, Phase::User);
    }
}
