//! Tiroir « Assistant » : la conversation avec LYRA, dessinée par hyprdesk,
//! au même format que le tiroir des Réglages (même couche, en-tête, cartes).
//!
//! Le moteur est un processus enfant qui parle le protocole `serve`
//! (`crate::chat`) : `hyprpanel-chat serve`, ou `$HYPRDESK_CHAT_ENGINE`
//! (développement, ex. `hyprdesk chat-mock`). Il vit tant que le tiroir est
//! ouvert ; le fermer l'arrête.

use iced::futures::{SinkExt, Stream};
use iced::widget::{
    Space, button, column, container, markdown, pick_list, row, scrollable, text, text_input,
};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Padding, Subscription, Task};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::{App, Msg, color};
use crate::chat::{ClientMsg, ServerMsg};

/// Espace de noms de la surface du tiroir de l'assistant.
pub const CHAT_NAMESPACE: &str = "hyprdesk-assistant";

/// Une entrée du fil de la conversation.
pub enum Entry {
    User(String),
    Assistant(markdown::Content),
    Tool {
        name: String,
        summary: String,
        status: String,
        result: Option<String>,
    },
    Confirm {
        id: String,
        command: String,
        reason: String,
        risk: String,
        answer: Option<bool>,
    },
    Choice {
        id: String,
        prompt: String,
        options: Vec<crate::chat::ChoiceOption>,
        answer: Option<String>,
    },
    Error(String),
}

pub struct ChatPanel {
    /// Surface du bureau de l'écran visé (contexte).
    pub win: Id,
    /// Surface propre du tiroir.
    pub surface: Id,
    /// Session : change à chaque ouverture (relance le moteur).
    pub session: u64,
    pub engine: Vec<String>,
    pub entries: Vec<Entry>,
    pub input: String,
    pub busy: bool,
    pub thinking: bool,
    pub progress: Option<(String, Option<f32>)>,
    pub provider: String,
    pub providers: Vec<String>,
    pub sender: Option<tokio::sync::mpsc::UnboundedSender<ClientMsg>>,
    /// Le moteur s'est arrêté (message), ou n'a pas pu démarrer.
    pub ended: Option<String>,
    next_id: u64,
}

impl ChatPanel {
    pub fn new(win: Id, surface: Id, session: u64, engine: Vec<String>) -> Self {
        ChatPanel {
            win,
            surface,
            session,
            engine,
            entries: Vec::new(),
            input: String::new(),
            busy: false,
            thinking: false,
            progress: None,
            provider: String::new(),
            providers: Vec::new(),
            sender: None,
            ended: None,
            next_id: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChatMsg {
    Connected(tokio::sync::mpsc::UnboundedSender<ClientMsg>),
    Engine(ServerMsg),
    Ended(String),
    Input(String),
    Send,
    Cancel,
    Confirm(String, bool),
    Choose(String, String),
    NewConversation,
    Provider(String),
    Link(String),
    Close,
}

/// Flux du moteur : lance le processus, transmet ses lignes, écrit les nôtres.
fn engine_stream(data: &(u64, Vec<String>)) -> impl Stream<Item = ChatMsg> + use<> {
    let argv = data.1.clone();
    iced::stream::channel(
        64,
        async move |mut out: iced::futures::channel::mpsc::Sender<ChatMsg>| {
            let Some((prog, args)) = argv.split_first() else {
                let _ = out
                    .send(ChatMsg::Ended("aucun moteur d'assistant".into()))
                    .await;
                return;
            };
            let child = tokio::process::Command::new(prog)
                .args(args)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn();
            let mut child = match child {
                Ok(c) => c,
                Err(e) => {
                    let _ = out.send(ChatMsg::Ended(format!("{prog} : {e}"))).await;
                    return;
                }
            };
            let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
                let _ = out
                    .send(ChatMsg::Ended("moteur sans entrée/sortie".into()))
                    .await;
                return;
            };
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ClientMsg>();
            let _ = out.send(ChatMsg::Connected(tx)).await;
            let mut lines = BufReader::new(stdout).lines();
            loop {
                tokio::select! {
                    line = lines.next_line() => match line {
                        Ok(Some(l)) if l.trim().is_empty() => {}
                        Ok(Some(l)) => {
                            let _ = out.send(ChatMsg::Engine(crate::chat::parse(&l))).await;
                        }
                        _ => {
                            let _ = out.send(ChatMsg::Ended("l'assistant s'est arrêté".into())).await;
                            break;
                        }
                    },
                    msg = rx.recv() => match msg {
                        Some(m) => {
                            let line = crate::chat::encode(&m) + "\n";
                            if stdin.write_all(line.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                                let _ = out.send(ChatMsg::Ended("l'assistant ne répond plus".into())).await;
                                break;
                            }
                        }
                        None => break,
                    },
                }
            }
            let _ = child.kill().await;
        },
    )
}

/// Libellé d'un fournisseur.
fn provider_label(p: &str) -> String {
    match p {
        "claude_code" => "Claude Code".into(),
        "codex" => "Codex".into(),
        "zcode" => "Zcode".into(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ProviderChoice(String);

impl std::fmt::Display for ProviderChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&provider_label(&self.0))
    }
}

impl App {
    /// Ouvre le tiroir de l'assistant sur l'écran principal (le tiroir des
    /// réglages se ferme : un seul tiroir à la fois). `None` : pas de moteur.
    pub(super) fn open_chat(&mut self, engine: Vec<String>) -> Task<Msg> {
        if self.chat.is_some() {
            return Task::none();
        }
        let close_settings = if self.panel.is_some() {
            self.close_panel()
        } else {
            Task::none()
        };
        let primary = self.primary_monitor();
        let Some((win, monitor)) = self
            .surfaces
            .iter()
            .find(|(_, n)| Some(*n) == primary.as_ref())
            .or_else(|| self.surfaces.iter().next())
            .map(|(id, n)| (*id, n.clone()))
        else {
            return close_settings;
        };
        let surface = Id::unique();
        self.chat_session = self.chat_session.wrapping_add(1);
        self.chat = Some(ChatPanel::new(win, surface, self.chat_session, engine));
        let create = Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((self.panel_width() as u32, 0)),
                layer: Layer::Top,
                anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Right,
                exclusive_zone: Some(0),
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                output_option: OutputOption::OutputName(monitor),
                namespace: Some(CHAT_NAMESPACE.into()),
                ..Default::default()
            },
            id: surface,
        });
        let focus = iced::widget::operation::focus(CHAT_INPUT);
        Task::batch([close_settings, create, self.sync_window_gaps(), focus])
    }

    /// Ferme le tiroir de l'assistant (le moteur s'arrête avec lui).
    pub(super) fn close_chat(&mut self) -> Task<Msg> {
        match self.chat.take() {
            Some(c) => Task::batch([iced::window::close(c.surface), self.sync_window_gaps()]),
            None => Task::none(),
        }
    }

    pub(super) fn chat_subscription(&self) -> Subscription<Msg> {
        match &self.chat {
            // Sans moteur (aperçu) : rien à lancer.
            Some(c) if c.engine.is_empty() => Subscription::none(),
            Some(c) => {
                Subscription::run_with((c.session, c.engine.clone()), engine_stream).map(Msg::Chat)
            }
            None => Subscription::none(),
        }
    }

    fn chat_send(&mut self, m: ClientMsg) {
        if let Some(tx) = self.chat.as_ref().and_then(|c| c.sender.as_ref()) {
            let _ = tx.send(m);
        }
    }

    pub(super) fn chat_update(&mut self, msg: ChatMsg) -> Task<Msg> {
        if let ChatMsg::Close = msg {
            return self.close_chat();
        }
        if let ChatMsg::Link(url) = msg {
            return Task::perform(
                async move {
                    let cmd = format!("xdg-open {}", crate::launch::sh_quote(&url));
                    let _ = tokio::task::spawn_blocking(move || crate::hypr::exec(&cmd)).await;
                },
                |_| Msg::Noop,
            );
        }
        let Some(c) = self.chat.as_mut() else {
            return Task::none();
        };
        match msg {
            ChatMsg::Connected(tx) => {
                c.sender = Some(tx);
                c.ended = None;
                self.chat_send(ClientMsg::Hello {
                    client: "hyprdesk".into(),
                    protocol: crate::chat::PROTOCOL,
                    language: "fr".into(),
                });
            }
            ChatMsg::Ended(why) => {
                c.sender = None;
                c.busy = false;
                c.thinking = false;
                c.ended = Some(why);
            }
            ChatMsg::Input(s) => c.input = s,
            ChatMsg::Send => {
                let text = c.input.trim().to_string();
                if text.is_empty() || c.busy || c.sender.is_none() {
                    return Task::none();
                }
                c.input.clear();
                c.next_id += 1;
                let id = format!("m{}", c.next_id);
                c.entries.push(Entry::User(text.clone()));
                c.busy = true;
                self.chat_send(ClientMsg::Message {
                    id,
                    text,
                    attachments: Vec::new(),
                });
            }
            ChatMsg::Cancel => self.chat_send(ClientMsg::Cancel),
            ChatMsg::Confirm(id, accept) => {
                for e in c.entries.iter_mut() {
                    if let Entry::Confirm { id: i, answer, .. } = e
                        && *i == id
                    {
                        *answer = Some(accept);
                    }
                }
                c.busy = true;
                self.chat_send(ClientMsg::Confirm {
                    request: id,
                    accept,
                });
            }
            ChatMsg::Choose(id, value) => {
                for e in c.entries.iter_mut() {
                    if let Entry::Choice { id: i, answer, .. } = e
                        && *i == id
                    {
                        *answer = Some(value.clone());
                    }
                }
                c.busy = true;
                self.chat_send(ClientMsg::Choice { request: id, value });
            }
            ChatMsg::NewConversation => {
                c.entries.clear();
                c.busy = false;
                c.thinking = false;
                c.progress = None;
                self.chat_send(ClientMsg::NewConversation);
            }
            ChatMsg::Provider(p) => {
                c.provider = p.clone();
                self.chat_send(ClientMsg::Provider { value: p });
            }
            ChatMsg::Engine(m) => self.chat_engine(m),
            ChatMsg::Link(_) | ChatMsg::Close => {}
        }
        Task::none()
    }

    /// Aperçu : une conversation déjà remplie, rejouée par le protocole.
    pub(super) fn chat_demo(&mut self, win: Id) {
        use crate::chat::ChoiceOption;
        self.chat = Some(ChatPanel::new(win, Id::unique(), 0, Vec::new()));
        let msgs = [ServerMsg::Ready {
            protocol: 1,
            provider: "claude_code".into(),
            providers: vec!["zcode".into(), "claude_code".into(), "codex".into()],
            fs: false,
        }];
        for m in msgs {
            self.chat_engine(m);
        }
        if let Some(c) = self.chat.as_mut() {
            c.entries.push(Entry::User(
                "Mes fenêtres sont trop collées, tu peux espacer ?".into(),
            ));
        }
        for m in [
            ServerMsg::Delta {
                text: "Les marges extérieures valent **5 px**. Je propose de passer à `10` \
                       (aperçu à chaud, non enregistré) :"
                    .into(),
            },
            ServerMsg::Tool {
                id: "t1".into(),
                name: "hypr_get_option".into(),
                summary: "Lecture de general:gaps_out".into(),
                status: "done".into(),
                result: Some("5 5 5 5".into()),
            },
            ServerMsg::ConfirmRequest {
                id: "c1".into(),
                command: "hyprctl keyword general:gaps_out 10".into(),
                reason: "Passer les marges extérieures à 10 px.".into(),
                risk: "low".into(),
            },
            ServerMsg::ChoiceRequest {
                id: "q1".into(),
                kind: "color".into(),
                prompt: "Et la couleur d'accent ?".into(),
                options: ["#9580ff", "#ff80bf", "#80ffea"]
                    .iter()
                    .map(|c| ChoiceOption {
                        label: c.to_string(),
                        value: c.to_string(),
                        preview: Some(c.to_string()),
                    })
                    .collect(),
            },
        ] {
            self.chat_engine(m);
        }
        if let Some(c) = self.chat.as_mut() {
            c.input = "Oui, et augmente aussi les marges intérieures".into();
        }
    }

    fn chat_engine(&mut self, m: ServerMsg) {
        let Some(c) = self.chat.as_mut() else { return };
        // Texte de l'assistant : ajouté à sa dernière bulle, ou à une nouvelle.
        let push_text = |c: &mut ChatPanel, t: &str| {
            if let Some(Entry::Assistant(content)) = c.entries.last_mut() {
                content.push_str(t);
            } else {
                let mut content = markdown::Content::new();
                content.push_str(t);
                c.entries.push(Entry::Assistant(content));
            }
        };
        match m {
            ServerMsg::Ready {
                provider,
                providers,
                ..
            } => {
                c.provider = provider;
                c.providers = providers;
            }
            ServerMsg::TurnStart { .. } => {
                c.busy = true;
            }
            ServerMsg::Delta { text } => {
                c.thinking = false;
                push_text(c, &text);
            }
            ServerMsg::Thinking { active } => c.thinking = active,
            ServerMsg::Progress { phase, fraction } => c.progress = Some((phase, fraction)),
            ServerMsg::Tool {
                name,
                summary,
                status,
                result,
                ..
            } => {
                // Un outil qui change d'état remplace sa ligne précédente.
                if let Some(Entry::Tool {
                    name: n,
                    status: s,
                    result: r,
                    ..
                }) = c.entries.last_mut()
                    && *n == name
                    && s == "running"
                {
                    *s = status;
                    *r = result;
                } else {
                    c.entries.push(Entry::Tool {
                        name,
                        summary,
                        status,
                        result,
                    });
                }
            }
            ServerMsg::ConfirmRequest {
                id,
                command,
                reason,
                risk,
            } => {
                c.busy = false;
                c.thinking = false;
                c.entries.push(Entry::Confirm {
                    id,
                    command,
                    reason,
                    risk,
                    answer: None,
                });
            }
            ServerMsg::ChoiceRequest {
                id,
                prompt,
                options,
                ..
            } => {
                c.busy = false;
                c.thinking = false;
                c.entries.push(Entry::Choice {
                    id,
                    prompt,
                    options,
                    answer: None,
                });
            }
            ServerMsg::TurnEnd { .. } => {
                c.busy = false;
                c.thinking = false;
                c.progress = None;
            }
            ServerMsg::Error { message, fatal } => {
                c.entries.push(Entry::Error(message));
                if fatal {
                    c.busy = false;
                }
            }
            ServerMsg::Unknown => {}
        }
    }

    /// Le tiroir de l'assistant, au format du tiroir des réglages.
    pub(super) fn view_chat(&self) -> Element<'_, Msg> {
        let Some(c) = &self.chat else {
            return Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0x95, 0x80);
        let close_icon = super::icons::get("window-close").unwrap();
        let close = iced::widget::tooltip(
            button(
                iced::widget::svg(close_icon)
                    .width(16)
                    .height(16)
                    .style(move |_, _| iced::widget::svg::Style { color: Some(fg) }),
            )
            .padding(9)
            .style(button::text)
            .on_press(Msg::Chat(ChatMsg::Close)),
            text("Fermer · Échap").font(self.font),
            iced::widget::tooltip::Position::Left,
        );
        let providers: Vec<ProviderChoice> =
            c.providers.iter().cloned().map(ProviderChoice).collect();
        let header = row![
            column![
                text("HYPRDESK")
                    .font(self.title_font())
                    .size(10)
                    .color(Color { a: 0.55, ..fg }),
                text("Assistant").font(self.title_font()).size(27).color(fg),
                text(match &c.ended {
                    Some(_) => "LYRA · arrêtée".to_string(),
                    None if c.provider.is_empty() => "LYRA · démarrage…".to_string(),
                    None => format!("LYRA · {}", provider_label(&c.provider)),
                })
                .font(self.font)
                .size(12)
                .color(Color { a: 0.65, ..fg }),
            ]
            .spacing(5)
            .width(Length::Fill),
            pick_list(
                providers,
                (!c.provider.is_empty()).then(|| ProviderChoice(c.provider.clone())),
                move |p| Msg::Chat(ChatMsg::Provider(p.0))
            )
            .font(self.font)
            .text_size(13)
            .padding([5, 10])
            .placeholder("Fournisseur")
            .width(140),
            button(text("Nouvelle").font(self.font).size(12))
                .padding([5, 10])
                .style(button::secondary)
                .on_press(Msg::Chat(ChatMsg::NewConversation)),
            close,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        // Fil de la conversation.
        let md_settings = markdown::Settings::with_text_size(14, &self.theme);
        let bubble = |content, mine| bubble(content, mine, fg, accent);
        let mut thread = column![].spacing(10);
        if c.entries.is_empty() && c.ended.is_none() {
            thread = thread.push(
                text(
                    "Demandez à LYRA d'explorer, dépanner ou personnaliser votre bureau. \
                     Elle connaît hyprdesk (contexte transmis) et demande toujours avant d'agir.",
                )
                .font(self.font)
                .size(13)
                .color(dim),
            );
        }
        for e in &c.entries {
            thread = thread.push(match e {
                Entry::User(t) => bubble(
                    text(t.clone()).font(self.font).size(14).color(fg).into(),
                    true,
                ),
                Entry::Assistant(content) => bubble(
                    markdown::view(content.items(), md_settings)
                        .map(|u| Msg::Chat(ChatMsg::Link(u))),
                    false,
                ),
                Entry::Tool {
                    name,
                    summary,
                    status,
                    result,
                } => {
                    let mark = match status.as_str() {
                        "done" => "✓",
                        "error" => "⚠",
                        _ => "…",
                    };
                    let mut l =
                        format!("{mark} {}", if summary.is_empty() { name } else { summary });
                    if let Some(r) = result.as_deref().filter(|r| !r.is_empty()) {
                        l += &format!(" — {r}");
                    }
                    text(l).font(self.font).size(12).color(dim).into()
                }
                Entry::Confirm {
                    id,
                    command,
                    reason,
                    risk,
                    answer,
                } => {
                    let edge = if risk == "high" { warn } else { accent };
                    let mut card = column![
                        text(if reason.is_empty() {
                            "Confirmer cette action ?".to_string()
                        } else {
                            reason.clone()
                        })
                        .font(self.font)
                        .size(13)
                        .color(fg),
                        container(
                            text(command.clone())
                                .font(iced::Font::MONOSPACE)
                                .size(12)
                                .color(fg)
                        )
                        .padding([6, 10])
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            background: Some(Color { a: 0.08, ..fg }.into()),
                            border: Border::default().rounded(8),
                            ..Default::default()
                        }),
                    ]
                    .spacing(8);
                    card = card.push(match answer {
                        None => Element::from(
                            row![
                                button(text("Accepter").font(self.font).size(13))
                                    .padding([5, 14])
                                    .on_press(Msg::Chat(ChatMsg::Confirm(id.clone(), true))),
                                button(text("Refuser").font(self.font).size(13))
                                    .padding([5, 14])
                                    .style(button::secondary)
                                    .on_press(Msg::Chat(ChatMsg::Confirm(id.clone(), false))),
                            ]
                            .spacing(10),
                        ),
                        Some(true) => text("Accepté")
                            .font(self.font)
                            .size(12)
                            .color(accent)
                            .into(),
                        Some(false) => text("Refusé").font(self.font).size(12).color(dim).into(),
                    });
                    container(card)
                        .padding(12)
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            border: Border::default().rounded(12).width(1).color(edge),
                            ..Default::default()
                        })
                        .into()
                }
                Entry::Choice {
                    id,
                    prompt,
                    options,
                    answer,
                } => {
                    let pills = row(options.iter().map(|o| {
                        let chosen = answer.as_deref() == Some(o.value.as_str());
                        let swatch: Option<Color> = o
                            .preview
                            .as_deref()
                            .and_then(crate::config::Rgba::parse)
                            .map(|c| color(c, 1.0));
                        let mut label = row![].spacing(6).align_y(Alignment::Center);
                        if let Some(sw) = swatch {
                            label = label.push(container(Space::new().width(14).height(14)).style(
                                move |_| container::Style {
                                    background: Some(sw.into()),
                                    border: Border::default().rounded(7),
                                    ..Default::default()
                                },
                            ));
                        }
                        label = label.push(text(o.label.clone()).font(self.font).size(13));
                        button(label)
                            .padding([5, 12])
                            .style(if chosen {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .on_press_maybe(
                                answer.is_none().then(|| {
                                    Msg::Chat(ChatMsg::Choose(id.clone(), o.value.clone()))
                                }),
                            )
                            .into()
                    }))
                    .spacing(8);
                    column![
                        text(prompt.clone()).font(self.font).size(13).color(fg),
                        pills
                    ]
                    .spacing(8)
                    .into()
                }
                Entry::Error(m) => text(m.clone()).font(self.font).size(12).color(warn).into(),
            });
        }
        if c.thinking || (c.busy && !matches!(c.entries.last(), Some(Entry::Assistant(_)))) {
            let label = match &c.progress {
                Some((phase, Some(f))) => format!("{phase} — {:.0} %", f * 100.0),
                Some((phase, None)) => phase.clone(),
                None => "LYRA réfléchit…".into(),
            };
            thread = thread.push(text(label).font(self.font).size(12).color(dim));
        }
        if let Some(why) = &c.ended {
            thread = thread.push(text(why.clone()).font(self.font).size(12).color(warn));
        }
        let body = scrollable(container(thread).padding(Padding {
            right: 14.0,
            ..Padding::ZERO
        }))
        .anchor_bottom()
        .height(Length::Fill);

        // Saisie.
        let can_send = c.sender.is_some() && !c.busy;
        let composer = row![
            text_input("Écrire à LYRA…", &c.input)
                .id(CHAT_INPUT)
                .font(self.font)
                .size(14)
                .padding(10)
                .on_input(|s| Msg::Chat(ChatMsg::Input(s)))
                .on_submit(Msg::Chat(ChatMsg::Send))
                .width(Length::Fill),
            if c.busy {
                button(text("Arrêter").font(self.font).size(13))
                    .padding([8, 14])
                    .style(button::secondary)
                    .on_press(Msg::Chat(ChatMsg::Cancel))
            } else {
                button(text("Envoyer").font(self.font).size(13))
                    .padding([8, 14])
                    .on_press_maybe(can_send.then(|| Msg::Chat(ChatMsg::Send)))
            },
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let card = container(
            column![header, iced::widget::rule::horizontal(1), body, composer].spacing(16),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme| super::design::surface(theme, super::design::Surface::Panel));
        container(iced::widget::mouse_area(card).on_press(Msg::Noop))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12)
            .into()
    }
}

/// Bulle de la conversation : la vôtre à droite (accent), celle de LYRA à gauche.
fn bubble(content: Element<'_, Msg>, mine: bool, fg: Color, accent: Color) -> Element<'_, Msg> {
    let bg = if mine {
        Color { a: 0.22, ..accent }
    } else {
        Color { a: 0.06, ..fg }
    };
    let b = container(content)
        .padding([10, 14])
        .max_width(560)
        .style(move |_| container::Style {
            background: Some(bg.into()),
            border: Border::default().rounded(14),
            ..Default::default()
        });
    container(b)
        .width(Length::Fill)
        .align_x(if mine {
            Alignment::End
        } else {
            Alignment::Start
        })
        .into()
}

/// Identifiant du champ de saisie (focus à l'ouverture).
pub const CHAT_INPUT: &str = "hyprdesk-chat-input";

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_chat() -> App {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        let win = Id::unique();
        app.surfaces.insert(win, "DP-1".into());
        let _ = app.open_chat(vec!["moteur".into()]);
        app
    }

    /// Un tour complet : saisie, texte au fil de l'eau, confirmation, fin ;
    /// rien n'est envoyé pendant un tour ni sans moteur connecté.
    #[test]
    fn un_tour_de_conversation() {
        let mut app = app_with_chat();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = app.chat_update(ChatMsg::Connected(tx));
        assert!(matches!(rx.try_recv(), Ok(ClientMsg::Hello { .. })));
        let _ = app.chat_update(ChatMsg::Input("  règle les gaps ".into()));
        let _ = app.chat_update(ChatMsg::Send);
        assert!(
            matches!(rx.try_recv(), Ok(ClientMsg::Message { text, .. }) if text == "règle les gaps")
        );
        // Pendant le tour : un second envoi est refusé.
        let _ = app.chat_update(ChatMsg::Input("encore".into()));
        let _ = app.chat_update(ChatMsg::Send);
        assert!(rx.try_recv().is_err());
        for t in ["Je ", "propose **10**."] {
            let _ = app.chat_update(ChatMsg::Engine(ServerMsg::Delta { text: t.into() }));
        }
        let _ = app.chat_update(ChatMsg::Engine(ServerMsg::ConfirmRequest {
            id: "c1".into(),
            command: "hyprctl keyword general:gaps_out 10".into(),
            reason: String::new(),
            risk: "low".into(),
        }));
        let c = app.chat.as_ref().unwrap();
        // Une seule bulle pour le texte en flux ; la confirmation rend la main.
        assert_eq!(c.entries.len(), 3);
        assert!(!c.busy);
        let _ = app.chat_update(ChatMsg::Confirm("c1".into(), false));
        assert!(matches!(
            rx.try_recv(),
            Ok(ClientMsg::Confirm { accept: false, .. })
        ));
        let _ = app.chat_update(ChatMsg::Engine(ServerMsg::TurnEnd {
            id: "c1".into(),
            text: String::new(),
        }));
        let c = app.chat.as_ref().unwrap();
        assert!(!c.busy);
        assert!(matches!(
            c.entries[2],
            Entry::Confirm {
                answer: Some(false),
                ..
            }
        ));
        // Moteur arrêté : l'envoi est impossible, la raison est affichée.
        let _ = app.chat_update(ChatMsg::Ended("arrêt".into()));
        let _ = app.chat_update(ChatMsg::Send);
        assert!(app.chat.as_ref().unwrap().sender.is_none());
    }

    /// Un seul tiroir : ouvrir les réglages ferme l'assistant.
    #[test]
    fn un_seul_tiroir() {
        let mut app = app_with_chat();
        assert!(app.chat.is_some());
        let _ = app.open_panel();
        assert!(app.chat.is_none());
        assert!(app.panel.is_some());
    }
}
