//! Tiroir « Assistant » : la conversation avec LYRA, dessinée par hyprdesk,
//! au même format que le tiroir des Réglages (même couche, en-tête, cartes).
//!
//! Le moteur est un processus enfant qui parle le protocole `serve`
//! (`crate::chat`) : `hyprpanel-chat serve`, ou `$HYPRDESK_CHAT_ENGINE`
//! (développement, ex. `hyprdesk chat-mock`). Il vit tant que le tiroir est
//! ouvert ; le fermer l'arrête.

mod appearance;

use crate::i18n::text;
use appearance::Presence;
use iced::futures::{SinkExt, Stream};
use iced::widget::{
    Space, button, column, container, markdown, pick_list, row, scrollable, text_input,
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
    /// Message de l'utilisateur, avec le nombre d'images jointes.
    User {
        text: String,
        images: usize,
    },
    /// Réponse de LYRA : texte brut (copie, sélection) et rendu Markdown.
    Assistant {
        raw: String,
        md: markdown::Content,
    },
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

/// Une conversation : son moteur (un processus `serve` à elle) et son fil.
pub struct Conversation {
    /// Identifiant du moteur (relancé si elle repart de zéro).
    pub session: u64,
    pub engine: Vec<String>,
    /// Titre au sélecteur : « Conversation n », ou le module qui l'a ouverte.
    pub title: String,
    pub entries: Vec<Entry>,
    pub input: String,
    pub busy: bool,
    pub thinking: bool,
    pub progress: Option<(String, Option<f32>)>,
    pub provider: String,
    pub providers: Vec<String>,
    pub sender: Option<tokio::sync::mpsc::UnboundedSender<ClientMsg>>,
    /// Message à envoyer dès que le moteur est prêt et libre (module).
    pub pending: Option<String>,
    /// Le moteur a dit « prêt » (après Hello).
    pub ready: bool,
    /// Le moteur s'est arrêté (message), ou n'a pas pu démarrer.
    pub ended: Option<String>,
    /// Images collées, envoyées avec le prochain message.
    pub attachments: Vec<std::path::PathBuf>,
    /// Réponse affichée en texte sélectionnable (n° d'entrée, contenu).
    pub selecting: Option<(usize, iced::widget::text_editor::Content)>,
    /// Groupes d'étapes techniques dépliés (n° de leur première entrée).
    pub expanded: std::collections::HashSet<usize>,
    /// Du nouveau pendant qu'on regardait une autre conversation.
    pub unread: bool,
    next_id: u64,
}

impl Conversation {
    pub fn new(session: u64, engine: Vec<String>, title: String) -> Self {
        Conversation {
            session,
            engine,
            title,
            entries: Vec::new(),
            input: String::new(),
            busy: false,
            thinking: false,
            progress: None,
            provider: String::new(),
            providers: Vec::new(),
            sender: None,
            pending: None,
            ready: false,
            ended: None,
            attachments: Vec::new(),
            selecting: None,
            expanded: Default::default(),
            unread: false,
            next_id: 0,
        }
    }
}

/// Conversations ouvertes en même temps, au plus.
pub const MAX_CONVERSATIONS: usize = 8;

/// Le tiroir et ses conversations.
pub struct ChatPanel {
    /// Surface du bureau de l'écran visé (contexte).
    pub win: Id,
    /// Surface propre du tiroir ; `None` : caché (les conversations
    /// continuent, leurs moteurs restent lancés).
    pub surface: Option<Id>,
    pub convs: Vec<Conversation>,
    /// Conversation affichée (indice dans `convs`).
    pub active: usize,
}

impl ChatPanel {
    pub fn new(win: Id, surface: Option<Id>, session: u64, engine: Vec<String>) -> Self {
        ChatPanel {
            win,
            surface,
            convs: vec![Conversation::new(
                session,
                engine,
                crate::tr!("Conversation {n}", n = 1),
            )],
            active: 0,
        }
    }

    pub fn cur(&self) -> &Conversation {
        &self.convs[self.active.min(self.convs.len() - 1)]
    }

    pub fn cur_mut(&mut self) -> &mut Conversation {
        let i = self.active.min(self.convs.len() - 1);
        &mut self.convs[i]
    }

    pub fn by_session(&mut self, session: u64) -> Option<&mut Conversation> {
        self.convs.iter_mut().find(|c| c.session == session)
    }

    /// Ajoute une conversation (la plus ancienne inactive part au-delà du
    /// maximum) ; rend son indice.
    pub fn push(&mut self, conv: Conversation) -> usize {
        if self.convs.len() >= MAX_CONVERSATIONS
            && let Some(i) = self
                .convs
                .iter()
                .enumerate()
                .filter(|(i, c)| *i != self.active && !c.busy)
                .map(|(i, _)| i)
                .next()
        {
            self.convs.remove(i);
            if self.active > i {
                self.active -= 1;
            }
        }
        self.convs.push(conv);
        self.convs.len() - 1
    }

    /// Prochain numéro libre pour « Conversation n ».
    fn next_title(&self) -> String {
        let n = (1..)
            .find(|n| {
                let t = crate::tr!("Conversation {n}", n = n);
                !self.convs.iter().any(|c| c.title == t)
            })
            .unwrap_or(1);
        crate::tr!("Conversation {n}", n = n)
    }
}

#[derive(Debug, Clone)]
pub enum ChatMsg {
    /// Événements d'un moteur (par session).
    Connected(u64, tokio::sync::mpsc::UnboundedSender<ClientMsg>),
    Engine(u64, ServerMsg),
    Ended(u64, String),
    Input(String),
    /// Préremplit la saisie depuis l'accueil et lui rend le focus.
    Suggest(String),
    Send,
    Cancel,
    Confirm(String, bool),
    Choose(String, String),
    /// Nouvelle conversation (son propre moteur), affichée.
    NewConversation,
    /// Affiche une conversation (sélecteur).
    Switch(u64),
    /// Ferme une conversation (son moteur s'arrête).
    CloseConversation(u64),
    /// Ouvre le tiroir sur une conversation (notification « Ouvrir »).
    Show(u64),
    /// Déplie ou replie un groupe d'étapes techniques.
    ToggleSteps(usize),
    Provider(String),
    Link(String),
    Close,
    /// Copie un texte (réponse, bloc de code, message).
    Copy(String),
    /// Bascule une réponse en texte sélectionnable (ou la rend).
    Select(usize),
    /// Action dans le texte sélectionnable (sélection, pas d'édition).
    EditorAction(iced::widget::text_editor::Action),
    /// Colle l'image du presse-papiers (bouton).
    PasteImage,
    /// Ctrl+V dans le tiroir : image jointe, ou texte ajouté à la saisie.
    KeyPaste(Id),
    Pasted(Result<std::path::PathBuf, String>),
    PastedText(String),
    RemoveAttachment(usize),
}

/// Flux d'un moteur : lance le processus, transmet ses lignes, écrit les nôtres.
fn engine_stream(data: &(u64, Vec<String>)) -> impl Stream<Item = ChatMsg> + use<> {
    let (session, argv) = (data.0, data.1.clone());
    iced::stream::channel(
        64,
        async move |mut out: iced::futures::channel::mpsc::Sender<ChatMsg>| {
            let Some((prog, args)) = argv.split_first() else {
                let _ = out
                    .send(ChatMsg::Ended(
                        session,
                        crate::i18n::tr("aucun moteur d'assistant").into(),
                    ))
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
                    let _ = out
                        .send(ChatMsg::Ended(session, format!("{prog} : {e}")))
                        .await;
                    return;
                }
            };
            let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
                let _ = out
                    .send(ChatMsg::Ended(
                        session,
                        crate::i18n::tr("moteur sans entrée/sortie").into(),
                    ))
                    .await;
                return;
            };
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ClientMsg>();
            let _ = out.send(ChatMsg::Connected(session, tx)).await;
            let mut lines = BufReader::new(stdout).lines();
            loop {
                tokio::select! {
                    line = lines.next_line() => match line {
                        Ok(Some(l)) if l.trim().is_empty() => {}
                        Ok(Some(l)) => {
                            let _ = out.send(ChatMsg::Engine(session, crate::chat::parse(&l))).await;
                        }
                        _ => {
                            let _ = out.send(ChatMsg::Ended(session, crate::i18n::tr("l'assistant s'est arrêté").into())).await;
                            break;
                        }
                    },
                    msg = rx.recv() => match msg {
                        Some(m) => {
                            let line = crate::chat::encode(&m) + "\n";
                            if stdin.write_all(line.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                                let _ = out.send(ChatMsg::Ended(session, crate::i18n::tr("l'assistant ne répond plus").into())).await;
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

/// Entrée du sélecteur de conversations.
#[derive(Debug, Clone, PartialEq)]
struct ConvChoice(u64, String);

impl std::fmt::Display for ConvChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

/// Groupes d'étapes techniques consécutives : (première, après la dernière).
pub fn step_groups(entries: &[Entry]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < entries.len() {
        if matches!(entries[i], Entry::Tool { .. }) {
            let start = i;
            while i < entries.len() && matches!(entries[i], Entry::Tool { .. }) {
                i += 1;
            }
            out.push((start, i));
        } else {
            i += 1;
        }
    }
    out
}

impl App {
    /// Un module soumet un texte à LYRA : une conversation à lui s'ouvre
    /// (titre `origin`), le texte part dès que le moteur est prêt, et une
    /// notification le signale (« Ouvrir » affiche la conversation). `show` :
    /// demande explicite, le tiroir s'ouvre aussitôt dessus. `None` : LYRA ne
    /// sait pas se faire dessiner ici.
    pub(super) fn ask_assistant(
        &mut self,
        text: String,
        origin: &str,
        show: bool,
    ) -> Option<Task<Msg>> {
        let engine = crate::assistant::serve_command(&self.cfg)?;
        if !self.cfg.plugin_on("assistant") {
            return None;
        }
        self.chat_session = self.chat_session.wrapping_add(1);
        let session = self.chat_session;
        let mut conv = Conversation::new(session, engine, origin.to_string());
        conv.pending = Some(text);
        conv.unread = !show;
        let win = self.chat_target()?.0;
        let chat = self.chat.get_or_insert_with(|| ChatPanel {
            win,
            surface: None,
            convs: Vec::new(),
            active: 0,
        });
        let i = chat.push(conv);
        if show || chat.convs.len() == 1 {
            chat.active = i;
        }
        let open = if show && !self.chat_shown() {
            self.show_chat()
        } else {
            Task::none()
        };
        let notify = if show {
            Task::none()
        } else {
            notify_conversation(session, origin.to_string())
        };
        Some(Task::batch([open, notify]))
    }

    pub(super) fn chat_shown(&self) -> bool {
        self.chat.as_ref().is_some_and(|c| c.surface.is_some())
    }

    /// Surface de bureau et écran du tiroir (celui du clic, sinon l'écran actif).
    fn chat_target(&self) -> Option<(Id, String)> {
        // L'écran du clic, sinon l'écran actif.
        self.target_surface()
    }

    /// Ouvre le tiroir (une première conversation s'il n'y en a pas).
    pub(super) fn open_chat(&mut self, engine: Vec<String>) -> Task<Msg> {
        if self.chat_shown() {
            return Task::none();
        }
        if self.chat.as_ref().is_none_or(|c| c.convs.is_empty()) {
            let Some((win, _)) = self.chat_target() else {
                return Task::none();
            };
            self.chat_session = self.chat_session.wrapping_add(1);
            let fresh = ChatPanel::new(win, None, self.chat_session, engine);
            match self.chat.as_mut() {
                Some(c) => {
                    c.convs = fresh.convs;
                    c.active = 0;
                }
                None => self.chat = Some(fresh),
            }
        }
        self.show_chat()
    }

    /// Affiche la surface du tiroir sur les conversations existantes.
    fn show_chat(&mut self) -> Task<Msg> {
        let close_settings = if self.panel.is_some() {
            self.close_panel()
        } else {
            Task::none()
        };
        let Some((win, monitor)) = self.chat_target() else {
            return close_settings;
        };
        let surface = Id::unique();
        let Some(c) = self.chat.as_mut() else {
            return close_settings;
        };
        c.win = win;
        c.surface = Some(surface);
        c.cur_mut().unread = false;
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

    /// Cache le tiroir de l'assistant : les conversations continuent
    /// (moteurs lancés) et reviennent à la prochaine ouverture.
    pub(super) fn close_chat(&mut self) -> Task<Msg> {
        match self.chat.as_mut().and_then(|c| c.surface.take()) {
            Some(id) => Task::batch([iced::window::close(id), self.sync_window_gaps()]),
            None => Task::none(),
        }
    }

    pub(super) fn chat_subscription(&self) -> Subscription<Msg> {
        let Some(chat) = &self.chat else {
            return Subscription::none();
        };
        // Un moteur par conversation (sans moteur : aperçu, rien à lancer).
        let mut subs: Vec<Subscription<Msg>> = chat
            .convs
            .iter()
            .filter(|c| !c.engine.is_empty())
            .map(|c| {
                Subscription::run_with((c.session, c.engine.clone()), engine_stream).map(Msg::Chat)
            })
            .collect();
        if chat.surface.is_some() {
            // Ctrl+V dans le tiroir : image jointe, ou texte ajouté à la saisie.
            subs.push(iced::event::listen_with(|e, _, id| match e {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Character(c),
                    modifiers,
                    ..
                }) if modifiers.control() && c.as_str().eq_ignore_ascii_case("v") => {
                    Some(Msg::Chat(ChatMsg::KeyPaste(id)))
                }
                _ => None,
            }));
        }
        Subscription::batch(subs)
    }

    fn chat_send(&mut self, session: u64, m: ClientMsg) {
        if let Some(tx) = self
            .chat
            .as_mut()
            .and_then(|c| c.by_session(session))
            .and_then(|c| c.sender.as_ref())
        {
            let _ = tx.send(m);
        }
    }

    /// Envoie un message dans une conversation (refusé pendant un tour, ou
    /// sans moteur connecté).
    fn send_in(&mut self, session: u64, text: String) {
        let Some(c) = self.chat.as_mut().and_then(|c| c.by_session(session)) else {
            return;
        };
        let text = text.trim().to_string();
        if text.is_empty() || c.busy || c.sender.is_none() {
            return;
        }
        c.next_id += 1;
        let id = format!("m{}", c.next_id);
        let files = std::mem::take(&mut c.attachments);
        c.entries.push(Entry::User {
            text: text.clone(),
            images: files.len(),
        });
        c.busy = true;
        self.chat_send(
            session,
            ClientMsg::Message {
                id,
                text,
                attachments: files
                    .into_iter()
                    .map(|p| crate::chat::Attachment {
                        path: p.display().to_string(),
                    })
                    .collect(),
            },
        );
    }

    /// Message d'un module en attente : envoyé dès que le moteur est prêt et libre.
    fn flush_pending(&mut self, session: u64) {
        let Some(c) = self.chat.as_mut().and_then(|c| c.by_session(session)) else {
            return;
        };
        if !c.ready || c.busy || c.sender.is_none() {
            return;
        }
        if let Some(p) = c.pending.take() {
            self.send_in(session, p);
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
        match msg {
            ChatMsg::Suggest(prompt) => {
                if let Some(chat) = self.chat.as_mut() {
                    chat.cur_mut().input = prompt;
                }
                return iced::widget::operation::focus(CHAT_INPUT);
            }
            ChatMsg::Copy(t) => return copy_text(t),
            ChatMsg::PasteImage => return paste_clipboard(false),
            ChatMsg::KeyPaste(id) => {
                return if self.chat.as_ref().is_some_and(|c| c.surface == Some(id)) {
                    paste_clipboard(true)
                } else {
                    Task::none()
                };
            }
            ChatMsg::Show(session) => {
                if let Some(chat) = self.chat.as_mut()
                    && let Some(i) = chat.convs.iter().position(|c| c.session == session)
                {
                    chat.active = i;
                    chat.convs[i].unread = false;
                    if !self.chat_shown() {
                        return self.show_chat();
                    }
                }
                return Task::none();
            }
            ChatMsg::NewConversation => {
                let Some(engine) = crate::assistant::serve_command(&self.cfg)
                    .or_else(|| self.chat.as_ref().map(|c| c.cur().engine.clone()))
                else {
                    return Task::none();
                };
                self.chat_session = self.chat_session.wrapping_add(1);
                let session = self.chat_session;
                if let Some(chat) = self.chat.as_mut() {
                    let title = chat.next_title();
                    let i = chat.push(Conversation::new(session, engine, title));
                    chat.active = i;
                }
                return iced::widget::operation::focus(CHAT_INPUT);
            }
            ChatMsg::Connected(session, tx) => {
                if let Some(c) = self.chat.as_mut().and_then(|c| c.by_session(session)) {
                    c.sender = Some(tx);
                    c.ended = None;
                }
                self.chat_send(
                    session,
                    ClientMsg::Hello {
                        client: "hyprdesk".into(),
                        protocol: crate::chat::PROTOCOL,
                        language: crate::i18n::code().into(),
                    },
                );
                return Task::none();
            }
            ChatMsg::Engine(session, m) => {
                self.chat_engine(session, m);
                self.flush_pending(session);
                return Task::none();
            }
            ChatMsg::Ended(session, why) => {
                if let Some(c) = self.chat.as_mut().and_then(|c| c.by_session(session)) {
                    c.sender = None;
                    c.ready = false;
                    c.busy = false;
                    c.thinking = false;
                    c.ended = Some(why);
                }
                return Task::none();
            }
            ChatMsg::CloseConversation(session) => {
                if let Some(chat) = self.chat.as_mut()
                    && let Some(i) = chat.convs.iter().position(|c| c.session == session)
                {
                    chat.convs.remove(i);
                    if chat.active > i || chat.active >= chat.convs.len() {
                        chat.active = chat.active.saturating_sub(1);
                    }
                    if chat.convs.is_empty() {
                        // Plus aucune : une conversation neuve pour le tiroir.
                        return Task::done(Msg::Chat(ChatMsg::NewConversation));
                    }
                }
                return Task::none();
            }
            ChatMsg::Switch(session) => {
                if let Some(chat) = self.chat.as_mut()
                    && let Some(i) = chat.convs.iter().position(|c| c.session == session)
                {
                    chat.active = i;
                    chat.convs[i].unread = false;
                }
                return iced::widget::operation::focus(CHAT_INPUT);
            }
            ChatMsg::Send => {
                let Some(chat) = self.chat.as_mut() else {
                    return Task::none();
                };
                let c = chat.cur_mut();
                if c.busy || c.sender.is_none() || c.input.trim().is_empty() {
                    return Task::none();
                }
                let (session, text) = (c.session, std::mem::take(&mut c.input));
                self.send_in(session, text);
                return Task::none();
            }
            _ => {}
        }
        let Some(chat) = self.chat.as_mut() else {
            return Task::none();
        };
        let c = chat.cur_mut();
        let session = c.session;
        match msg {
            ChatMsg::Pasted(Ok(p)) => c.attachments.push(p),
            ChatMsg::Pasted(Err(e)) => c.entries.push(Entry::Error(e)),
            ChatMsg::PastedText(t) => {
                // La saisie a pu le recevoir elle-même (presse-papiers d'iced) : pas de doublon.
                if !c.input.ends_with(&t) {
                    c.input.push_str(&t);
                }
            }
            ChatMsg::RemoveAttachment(i) => {
                if i < c.attachments.len() {
                    c.attachments.remove(i);
                }
            }
            ChatMsg::ToggleSteps(i) => {
                if !c.expanded.remove(&i) {
                    c.expanded.insert(i);
                }
            }
            ChatMsg::Select(i) => {
                c.selecting = match (&c.selecting, c.entries.get(i)) {
                    (Some((j, _)), _) if *j == i => None,
                    (_, Some(Entry::Assistant { raw, .. })) => {
                        Some((i, iced::widget::text_editor::Content::with_text(raw)))
                    }
                    _ => None,
                };
            }
            ChatMsg::EditorAction(a) => {
                // Lecture seule : sélectionner, se déplacer, copier ; jamais modifier.
                if !a.is_edit()
                    && let Some((_, content)) = c.selecting.as_mut()
                {
                    content.perform(a);
                }
            }
            ChatMsg::Input(s) => c.input = s,
            ChatMsg::Cancel => self.chat_send(session, ClientMsg::Cancel),
            ChatMsg::Confirm(id, accept) => {
                for e in c.entries.iter_mut() {
                    if let Entry::Confirm { id: i, answer, .. } = e
                        && *i == id
                    {
                        *answer = Some(accept);
                    }
                }
                c.busy = true;
                self.chat_send(
                    session,
                    ClientMsg::Confirm {
                        request: id,
                        accept,
                    },
                );
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
                self.chat_send(session, ClientMsg::Choice { request: id, value });
            }
            ChatMsg::Provider(p) => {
                c.provider = p.clone();
                self.chat_send(session, ClientMsg::Provider { value: p });
            }
            _ => {}
        }
        Task::none()
    }

    /// Aperçu : une conversation déjà remplie, rejouée par le protocole.
    pub(super) fn chat_demo(&mut self, win: Id) {
        use crate::chat::ChoiceOption;
        let mut chat = ChatPanel::new(win, Some(Id::unique()), 0, Vec::new());
        let mut other = Conversation::new(1, Vec::new(), crate::i18n::tr("Mises à jour").into());
        other.unread = true;
        chat.convs.push(other);
        self.chat = Some(chat);
        let msgs = [ServerMsg::Ready {
            protocol: 1,
            provider: "claude_code".into(),
            providers: vec!["zcode".into(), "claude_code".into(), "codex".into()],
            fs: false,
        }];
        for m in msgs {
            self.chat_engine(0, m);
        }
        if let Some(c) = self.chat.as_mut() {
            c.cur_mut().entries.push(Entry::User {
                text: crate::i18n::tr("Mes fenêtres sont trop collées, tu peux espacer ?").into(),
                images: 0,
            });
        }
        for m in [
            ServerMsg::Tool {
                id: "t0".into(),
                name: "hypr_status".into(),
                summary: "État de Hyprland".into(),
                status: "done".into(),
                result: None,
            },
            ServerMsg::Tool {
                id: "t1".into(),
                name: "hypr_get_option".into(),
                summary: "Lecture de general:gaps_out".into(),
                status: "done".into(),
                result: Some("5 5 5 5".into()),
            },
            ServerMsg::Delta {
                text: crate::i18n::tr(
                    "Les marges extérieures valent **5 px**. Je propose de passer à `10` \
                       (aperçu à chaud, non enregistré) :",
                )
                .into(),
            },
            ServerMsg::ConfirmRequest {
                id: "c1".into(),
                command: "hyprctl keyword general:gaps_out 10".into(),
                reason: crate::i18n::tr("Passer les marges extérieures à 10 px.").into(),
                risk: "low".into(),
            },
            ServerMsg::ChoiceRequest {
                id: "q1".into(),
                kind: "color".into(),
                prompt: crate::i18n::tr("Et la couleur d'accent ?").into(),
                options: ["#9580ff", "#ff80bf", "#80ffea"]
                    .iter()
                    .map(|c| ChoiceOption {
                        label: c.to_string(),
                        value: c.to_string(),
                        preview: Some(c.to_string()),
                    })
                    .collect(),
            },
            ServerMsg::ChoiceRequest {
                id: "q2".into(),
                kind: "choice".into(),
                prompt: crate::i18n::tr("Comment veux-tu procéder ?").into(),
                options: [
                    "A — Retirer wayle, hyprdesk gère tout",
                    "A + recréer le bouton GPU dans hyprdesk",
                    "B — Garder wayle",
                    "C — Chercher plus loin dans la config de wayle",
                ]
                .iter()
                .map(|l| ChoiceOption {
                    label: l.to_string(),
                    value: l.to_string(),
                    preview: None,
                })
                .collect(),
            },
            ServerMsg::TurnStart { id: "m2".into() },
            ServerMsg::Thinking { active: true },
        ] {
            self.chat_engine(0, m);
        }
        if let Some(c) = self.chat.as_mut() {
            c.cur_mut().input =
                crate::i18n::tr("Oui, et augmente aussi les marges intérieures").into();
        }
    }

    fn chat_engine(&mut self, session: u64, m: ServerMsg) {
        let Some(chat) = self.chat.as_mut() else {
            return;
        };
        let shown = chat.surface.is_some();
        let active = chat.cur().session;
        let Some(c) = chat.by_session(session) else {
            return;
        };
        // Réponse dans une conversation qu'on ne regarde pas : marquée.
        if matches!(m, ServerMsg::Delta { .. } | ServerMsg::TurnEnd { .. })
            && (!shown || session != active)
        {
            c.unread = true;
        }
        // Texte de l'assistant : ajouté à sa dernière bulle, ou à une nouvelle.
        let push_text = |c: &mut Conversation, t: &str| {
            if let Some(Entry::Assistant { raw, md }) = c.entries.last_mut() {
                raw.push_str(t);
                md.push_str(t);
            } else {
                let mut md = markdown::Content::new();
                md.push_str(t);
                c.entries.push(Entry::Assistant {
                    raw: t.to_string(),
                    md,
                });
            }
        };
        match m {
            ServerMsg::Ready {
                provider,
                providers,
                ..
            } => {
                c.ready = true;
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
        let Some(chat) = &self.chat else {
            return Space::new().into();
        };
        if chat.convs.is_empty() {
            return Space::new().into();
        }
        let c = chat.cur();
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.72, ..fg };
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
            .style(|t, s| appearance::pill(t, s, false))
            .on_press(Msg::Chat(ChatMsg::Close)),
            text("Fermer · Échap").font(self.font),
            iced::widget::tooltip::Position::Left,
        );
        let providers: Vec<ProviderChoice> =
            c.providers.iter().cloned().map(ProviderChoice).collect();
        let presence = Presence::of(c);
        let animate = self.cfg.general.transition != crate::config::Transition::None;
        let identity = row![
            appearance::avatar(82.0, presence, animate, accent),
            column![
                text("Votre assistante du bureau")
                    .font(self.font)
                    .size(12)
                    .color(dim),
                text("LYRA").font(self.title_font()).size(32).color(fg),
                text(presence.label()).font(self.font).size(13).color(
                    if presence == Presence::Offline {
                        dim
                    } else {
                        accent
                    }
                ),
            ]
            .spacing(3)
            .width(Length::Fill),
        ]
        .spacing(14)
        .align_y(Alignment::Center)
        .width(Length::Fill);
        let provider = pick_list(
            providers,
            (!c.provider.is_empty()).then(|| ProviderChoice(c.provider.clone())),
            move |p| Msg::Chat(ChatMsg::Provider(p.0)),
        )
        .font(self.font)
        .text_size(12)
        .padding([8, 14])
        .style(appearance::selector)
        .placeholder(crate::i18n::tr("Fournisseur"))
        .width(145);
        let new_conversation = button(text("+ Nouvelle").font(self.font).size(12))
            .padding([8, 14])
            .style(|t, s| appearance::pill(t, s, false))
            .on_press(Msg::Chat(ChatMsg::NewConversation));
        let header: Element<'_, Msg> = if self.panel_width() < 760.0 {
            column![
                row![identity, close].spacing(8).align_y(Alignment::Center),
                row![provider, Space::new().width(Length::Fill), new_conversation]
                    .spacing(8)
                    .align_y(Alignment::Center),
            ]
            .spacing(14)
            .into()
        } else {
            row![identity, provider, new_conversation, close]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
        };

        // Sélecteur des conversations : ● en cours de réponse, • du nouveau.
        let choices: Vec<ConvChoice> = chat
            .convs
            .iter()
            .map(|v| {
                let mark = if v.busy {
                    "● "
                } else if v.unread {
                    "• "
                } else {
                    ""
                };
                ConvChoice(v.session, format!("{mark}{}", v.title))
            })
            .collect();
        let current = choices.iter().find(|x| x.0 == c.session).cloned();
        let others_unread = chat
            .convs
            .iter()
            .filter(|v| v.session != c.session && v.unread)
            .count();
        let mut conv_row = row![
            pick_list(choices, current, |x: ConvChoice| Msg::Chat(
                ChatMsg::Switch(x.0)
            ))
            .font(self.font)
            .text_size(13)
            .padding([8, 14])
            .style(appearance::selector)
            .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if others_unread > 0 {
            conv_row = conv_row.push(
                text(crate::tr!("{n} à lire", n = others_unread))
                    .font(self.font)
                    .size(12)
                    .color(accent),
            );
        }
        conv_row = conv_row.push(iced::widget::tooltip(
            button(text("×").font(self.font).size(15))
                .padding([2, 8])
                .style(button::text)
                .on_press(Msg::Chat(ChatMsg::CloseConversation(c.session))),
            text(crate::i18n::tr("Fermer cette conversation")).font(self.font),
            iced::widget::tooltip::Position::Left,
        ));

        // Fil de la conversation.
        let md_settings = markdown::Settings::with_text_size(14, &self.theme);
        let bubble = |content, mine| bubble(content, mine, fg, accent);
        let mut thread = column![].spacing(14);
        if c.entries.is_empty() && c.ended.is_none() {
            let mut welcome = column![
                text("On fait quoi, ensemble ?")
                    .font(self.title_font())
                    .size(25)
                    .color(fg),
                text("Un bureau à votre image, un souci à résoudre ou une idée à explorer. Je suis là.")
                    .font(self.font)
                    .size(14)
                    .color(dim),
            ].spacing(12);
            for (label, prompt) in [
                (
                    "Personnaliser mon bureau",
                    "Aide-moi à personnaliser mon bureau.",
                ),
                (
                    "Résoudre un problème",
                    "Aide-moi à diagnostiquer un problème sur mon bureau.",
                ),
                ("Explorer hyprdesk", "Que puis-je faire avec hyprdesk ?"),
            ] {
                welcome = welcome.push(
                    button(text(crate::i18n::tr(label)).font(self.font).size(13))
                        .padding([10, 16])
                        .width(Length::Fill)
                        .style(|t, s| appearance::pill(t, s, false))
                        .on_press(Msg::Chat(ChatMsg::Suggest(crate::i18n::tr(prompt).into()))),
                );
            }
            thread = thread.push(
                container(welcome)
                    .padding([28, 20])
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(Color { a: 0.045, ..accent }.into()),
                        border: Border::default().rounded(24),
                        ..Default::default()
                    }),
            );
        }
        // Petites actions sous une bulle (copier, sélectionner).
        let action = |label: &'static str, msg: ChatMsg| {
            button(
                text(crate::i18n::tr(label))
                    .font(self.font)
                    .size(11)
                    .color(dim),
            )
            .padding([1, 6])
            .style(button::text)
            .on_press(Msg::Chat(msg))
        };
        // Étapes techniques (outils) : repliées par défaut, une ligne par groupe.
        let groups = step_groups(&c.entries);
        for (i, e) in c.entries.iter().enumerate() {
            if let Some(&(start, end)) = groups.iter().find(|(a, b)| i >= *a && i < *b) {
                let open = c.expanded.contains(&start);
                if i == start {
                    let n = end - start;
                    let running = c.entries[start..end]
                        .iter()
                        .any(|e| matches!(e, Entry::Tool { status, .. } if status == "running"));
                    let label = if open {
                        crate::tr!("▾ Masquer les détails ({n} étape(s))", n = n)
                    } else if running {
                        crate::tr!("▸ {n} étape(s) technique(s) — en cours…", n = n)
                    } else {
                        crate::tr!("▸ {n} étape(s) technique(s)", n = n)
                    };
                    thread = thread.push(
                        button(text(label).font(self.font).size(12).color(dim))
                            .padding([1, 4])
                            .style(button::text)
                            .on_press(Msg::Chat(ChatMsg::ToggleSteps(start))),
                    );
                }
                if !open {
                    continue;
                }
            }
            let el: Element<'_, Msg> = match e {
                Entry::User { text: t, images } => {
                    let mut body =
                        column![text(t.clone()).font(self.font).size(14).color(fg)].spacing(4);
                    if *images > 0 {
                        body = body.push(
                            text(crate::tr!("{n} image(s) jointe(s)", n = images))
                                .font(self.font)
                                .size(11)
                                .color(dim),
                        );
                    }
                    column![
                        bubble(body.into(), true),
                        container(action("Copier", ChatMsg::Copy(t.clone())))
                            .width(Length::Fill)
                            .align_x(Alignment::End),
                    ]
                    .spacing(2)
                    .into()
                }
                Entry::Assistant { raw, md } => {
                    let selecting = c.selecting.as_ref().filter(|(j, _)| *j == i);
                    let content: Element<'_, Msg> = match selecting {
                        // Texte sélectionnable : lecture seule, Ctrl+C copie la sélection.
                        Some((_, editor)) => iced::widget::text_editor(editor)
                            .font(self.font)
                            .size(14)
                            .on_action(|a| Msg::Chat(ChatMsg::EditorAction(a)))
                            .into(),
                        None => {
                            markdown::view_with(md.items(), md_settings, &CopyCode).map(Msg::Chat)
                        }
                    };
                    column![
                        row![
                            iced::widget::image(appearance::portrait())
                                .width(24)
                                .height(24)
                                .content_fit(iced::ContentFit::Cover)
                                .border_radius(12),
                            text("LYRA").font(self.title_font()).size(11).color(accent),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        bubble(content, false),
                        row![
                            action("Copier", ChatMsg::Copy(raw.clone())),
                            action(
                                if selecting.is_some() {
                                    "Terminer la sélection"
                                } else {
                                    "Sélectionner"
                                },
                                ChatMsg::Select(i)
                            ),
                        ]
                        .spacing(4),
                    ]
                    .spacing(2)
                    .into()
                }
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
                                    .padding([8, 18])
                                    .style(|t, s| appearance::pill(t, s, true))
                                    .on_press(Msg::Chat(ChatMsg::Confirm(id.clone(), true))),
                                button(text("Refuser").font(self.font).size(13))
                                    .padding([8, 18])
                                    .style(|t, s| appearance::pill(t, s, false))
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
                        .padding(16)
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            background: Some(Color { a: 0.045, ..edge }.into()),
                            border: Border::default()
                                .rounded(22)
                                .width(1)
                                .color(Color { a: 0.55, ..edge }),
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
                    // Propositions lisibles : courtes (couleurs…), des pastilles qui
                    // passent à la ligne ; sinon une par ligne, sur toute la largeur.
                    // Texte aux couleurs du thème, contraste garanti ; une fois
                    // répondu, le choix reste en évidence, les autres s'estompent.
                    let compact = options.iter().all(|o| o.label.chars().count() <= 14);
                    let option = |o: &crate::chat::ChoiceOption| -> Element<'_, Msg> {
                        let chosen = answer.as_deref() == Some(o.value.as_str());
                        let open = answer.is_none();
                        let swatch: Option<Color> = o
                            .preview
                            .as_deref()
                            .and_then(crate::config::Rgba::parse)
                            .map(|c| color(c, 1.0));
                        let mut label = row![].spacing(8).align_y(Alignment::Center);
                        if let Some(sw) = swatch {
                            label = label.push(container(Space::new().width(14).height(14)).style(
                                move |_| container::Style {
                                    background: Some(sw.into()),
                                    border: Border::default().rounded(7),
                                    ..Default::default()
                                },
                            ));
                        }
                        let tc = if open || chosen { fg } else { dim };
                        label = label.push(
                            text(o.label.clone())
                                .font(self.font)
                                .size(13.5)
                                .color(tc)
                                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                        );
                        let mut b =
                            button(label)
                                .padding([7, 12])
                                .style(move |_, st| {
                                    let hover = matches!(st, button::Status::Hovered);
                                    let a = if chosen {
                                        0.4
                                    } else if open && hover {
                                        0.22
                                    } else if open {
                                        0.1
                                    } else {
                                        0.04
                                    };
                                    button::Style {
                                        background: Some(
                                            if chosen || (open && hover) {
                                                Color { a, ..accent }
                                            } else {
                                                Color { a, ..fg }
                                            }
                                            .into(),
                                        ),
                                        border: Border::default()
                                            .rounded(if compact { 20 } else { 16 })
                                            .width(1)
                                            .color(if chosen {
                                                accent
                                            } else {
                                                Color {
                                                    a: if open { 0.22 } else { 0.08 },
                                                    ..fg
                                                }
                                            }),
                                        text_color: tc,
                                        ..Default::default()
                                    }
                                })
                                .on_press_maybe(open.then(|| {
                                    Msg::Chat(ChatMsg::Choose(id.clone(), o.value.clone()))
                                }));
                        if !compact {
                            b = b.width(Length::Fill);
                        }
                        b.into()
                    };
                    let pills: Element<'_, Msg> = if compact {
                        row(options.iter().map(option)).spacing(8).wrap().into()
                    } else {
                        column(options.iter().map(option)).spacing(6).into()
                    };
                    column![
                        text(prompt.clone()).font(self.font).size(13).color(fg),
                        pills
                    ]
                    .spacing(8)
                    .into()
                }
                Entry::Error(m) => text(m.clone()).font(self.font).size(12).color(warn).into(),
            };
            thread = thread.push(el);
        }
        // LYRA travaille : un repère bien visible tant que le tour dure ; la
        // phase technique (préparation du contexte) seulement en détail.
        if c.busy || c.thinking {
            let writing = matches!(c.entries.last(), Some(Entry::Assistant { .. })) && !c.thinking;
            let label = if writing {
                crate::i18n::tr("LYRA écrit…").to_string()
            } else {
                crate::i18n::tr("LYRA réfléchit…").to_string()
            };
            let mut pill = row![
                appearance::activity(animate, accent),
                text(label).font(self.font).size(13).color(fg),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            if let Some((phase, f)) = &c.progress {
                let detail = match f {
                    Some(f) => format!("{phase} — {:.0} %", f * 100.0),
                    None => phase.clone(),
                };
                pill = pill.push(text(detail).font(self.font).size(11).color(dim));
            }
            thread =
                thread.push(
                    container(pill)
                        .padding([8, 14])
                        .style(move |_| container::Style {
                            background: Some(Color { a: 0.10, ..accent }.into()),
                            border: Border::default().rounded(22),
                            ..Default::default()
                        }),
                );
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
        let can_send = c.sender.is_some() && !c.busy && !c.input.trim().is_empty();
        let composer = row![
            text_input(crate::i18n::tr("Écrire à LYRA…"), &c.input)
                .id(CHAT_INPUT)
                .font(self.font)
                .size(14)
                .padding([12, 14])
                .style(appearance::input)
                .on_input(|s| Msg::Chat(ChatMsg::Input(s)))
                .on_submit(Msg::Chat(ChatMsg::Send))
                .width(Length::Fill),
            if c.busy {
                button(text("Arrêter").font(self.font).size(13))
                    .padding([10, 18])
                    .style(|t, s| appearance::pill(t, s, false))
                    .on_press(Msg::Chat(ChatMsg::Cancel))
            } else {
                button(text("Envoyer").font(self.font).size(13))
                    .padding([10, 18])
                    .style(|t, s| appearance::pill(t, s, true))
                    .on_press_maybe(can_send.then(|| Msg::Chat(ChatMsg::Send)))
            },
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let mut attached = row![].spacing(8).align_y(Alignment::Center);
        for (i, p) in c.attachments.iter().enumerate() {
            attached = attached.push(
                container(
                    row![
                        iced::widget::image(iced::widget::image::Handle::from_path(p))
                            .width(40)
                            .height(40)
                            .content_fit(iced::ContentFit::Cover),
                        button(text("×").font(self.font).size(14))
                            .padding([0, 6])
                            .style(button::text)
                            .on_press(Msg::Chat(ChatMsg::RemoveAttachment(i))),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center),
                )
                .padding(4)
                .style(move |_| container::Style {
                    border: Border::default()
                        .rounded(8)
                        .width(1)
                        .color(Color { a: 0.2, ..fg }),
                    ..Default::default()
                }),
            );
        }
        attached = attached.push(
            button(
                text(crate::i18n::tr("Coller une image"))
                    .font(self.font)
                    .size(12),
            )
            .padding([4, 10])
            .style(button::text)
            .on_press(Msg::Chat(ChatMsg::PasteImage)),
        );
        let composer = container(column![attached.wrap(), composer].spacing(6))
            .padding(8)
            .width(Length::Fill)
            .style(appearance::composer);
        let card = container(column![header, conv_row, body, composer].spacing(16))
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme| {
                let mut style = super::design::surface(theme, super::design::Surface::Panel);
                let p = theme.palette();
                let tint = |amount| Color {
                    r: p.background.r + (p.primary.r - p.background.r) * amount,
                    g: p.background.g + (p.primary.g - p.background.g) * amount,
                    b: p.background.b + (p.primary.b - p.background.b) * amount,
                    a: 0.99,
                };
                style.background = Some(
                    iced::gradient::Linear::new(iced::Radians(1.1))
                        .add_stop(0.0, tint(0.10))
                        .add_stop(0.4, tint(0.025))
                        .add_stop(1.0, tint(0.0))
                        .into(),
                );
                style.border = style.border.rounded(30);
                // tiny-skia 0.14 dessine les ombres hors de la zone à
                // rafraîchir : une animation locale assombrirait tout le
                // tiroir à chaque image. Le contour porte ici la profondeur.
                style.shadow = iced::Shadow::default();
                style
            });
        container(iced::widget::mouse_area(card).on_press(Msg::Noop))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12)
            .into()
    }
}

/// Rendu Markdown des réponses : chaque bloc de code a son bouton « Copier ».
struct CopyCode;

impl<'a> markdown::Viewer<'a, ChatMsg> for CopyCode {
    fn on_link_click(url: markdown::Uri) -> ChatMsg {
        ChatMsg::Link(url)
    }

    fn code_block(
        &self,
        settings: markdown::Settings,
        _language: Option<&'a str>,
        code: &'a str,
        lines: &'a [markdown::Text],
    ) -> Element<'a, ChatMsg> {
        column![
            markdown::code_block(settings, lines, ChatMsg::Link),
            container(
                button(text(crate::i18n::tr("Copier le code")).size(11))
                    .padding([1, 6])
                    .style(button::text)
                    .on_press(ChatMsg::Copy(code.to_string())),
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
        ]
        .spacing(2)
        .into()
    }
}

/// Copie un texte dans le presse-papiers (wl-copy), hors du fil de l'interface.
fn copy_text(t: String) -> Task<Msg> {
    Task::perform(
        async move {
            let _ = tokio::task::spawn_blocking(move || {
                use std::io::Write;
                if let Ok(mut c) = std::process::Command::new("wl-copy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                {
                    if let Some(i) = c.stdin.as_mut() {
                        let _ = i.write_all(t.as_bytes());
                    }
                    drop(c.stdin.take());
                    let _ = c.wait();
                }
            })
            .await;
        },
        |_| Msg::Noop,
    )
}

/// Image du presse-papiers (wl-paste) enregistrée dans le dossier de la
/// session, pour être jointe au prochain message. Rien s'il n'y en a pas.
/// Ce que contient le presse-papiers.
enum Clip {
    Image(Result<std::path::PathBuf, String>),
    Text(String),
}

/// Colle le presse-papiers : une image se joint au message ; du texte
/// (`text`, Ctrl+V) s'ajoute à la saisie.
fn paste_clipboard(text: bool) -> Task<Msg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || -> Option<Clip> {
                let types = std::process::Command::new("wl-paste")
                    .arg("--list-types")
                    .output()
                    .ok()?;
                let types = String::from_utf8_lossy(&types.stdout).to_string();
                let image = types
                    .lines()
                    .find(|t| t.starts_with("image/png"))
                    .or_else(|| types.lines().find(|t| t.starts_with("image/")))
                    .map(|t| t.trim().to_string());
                let Some(mime) = image else {
                    if !text {
                        return None;
                    }
                    let plain = types.lines().any(|t| {
                        t.starts_with("text/plain") || t == "UTF8_STRING" || t == "STRING"
                    });
                    if !plain {
                        return None;
                    }
                    let out = std::process::Command::new("wl-paste")
                        .args(["--no-newline", "--type", "text/plain;charset=utf-8"])
                        .output()
                        .ok()
                        .filter(|o| o.status.success())
                        .or_else(|| {
                            std::process::Command::new("wl-paste")
                                .arg("--no-newline")
                                .output()
                                .ok()
                        })?;
                    let t = String::from_utf8_lossy(&out.stdout).into_owned();
                    return (!t.is_empty()).then_some(Clip::Text(t));
                };
                let ext = mime
                    .trim_start_matches("image/")
                    .split(';')
                    .next()
                    .unwrap_or("png")
                    .to_string();
                let out = std::process::Command::new("wl-paste")
                    .args(["--no-newline", "--type", &mime])
                    .output()
                    .ok()?;
                if out.stdout.is_empty() {
                    return None;
                }
                let dir = crate::lock::state::path().with_file_name("chat");
                Some(Clip::Image((|| {
                    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                    let p = dir.join(format!("image-{}.{ext}", crate::storage::revision()));
                    std::fs::write(&p, &out.stdout).map_err(|e| e.to_string())?;
                    Ok(p)
                })()))
            })
            .await
            .ok()
            .flatten()
        },
        |r| match r {
            Some(Clip::Image(r)) => Msg::Chat(ChatMsg::Pasted(r)),
            Some(Clip::Text(t)) => Msg::Chat(ChatMsg::PastedText(t)),
            None => Msg::Noop,
        },
    )
}

/// Notification « LYRA : nouvelle discussion » ; « Ouvrir » affiche la conversation.
fn notify_conversation(session: u64, origin: String) -> Task<Msg> {
    Task::perform(
        async move {
            let title = crate::i18n::tr("LYRA : nouvelle discussion").to_string();
            let body = crate::tr!("{origin} souhaite en parler avec LYRA.", origin = origin);
            let open = format!("open={}", crate::i18n::tr("Ouvrir"));
            crate::processes::output(
                tokio::process::Command::new("notify-send").args([
                    "-a",
                    "LYRA",
                    "-i",
                    "dialog-information",
                    "-A",
                    &open,
                    "--wait",
                    &title,
                    &body,
                ]),
                std::time::Duration::from_secs(3600),
            )
            .await
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "open")
            .unwrap_or(false)
        },
        move |open| {
            if open {
                Msg::Chat(ChatMsg::Show(session))
            } else {
                Msg::Noop
            }
        },
    )
}

/// Bulle de la conversation : la vôtre à droite (accent), celle de LYRA à gauche.
fn bubble(content: Element<'_, Msg>, mine: bool, fg: Color, accent: Color) -> Element<'_, Msg> {
    let bg = if mine {
        Color { a: 0.22, ..accent }
    } else {
        Color { a: 0.06, ..fg }
    };
    let b = container(content)
        .padding([12, 16])
        .max_width(560)
        .style(move |_| container::Style {
            background: Some(bg.into()),
            border: Border::default().rounded(if mine {
                iced::border::Radius {
                    top_left: 22.0,
                    top_right: 22.0,
                    bottom_left: 22.0,
                    bottom_right: 6.0,
                }
            } else {
                iced::border::Radius {
                    top_left: 6.0,
                    top_right: 22.0,
                    bottom_left: 22.0,
                    bottom_right: 22.0,
                }
            }),
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

    fn cur(app: &App) -> &Conversation {
        app.chat.as_ref().unwrap().cur()
    }

    fn ready(app: &mut App, session: u64) {
        let _ = app.chat_update(ChatMsg::Engine(
            session,
            ServerMsg::Ready {
                protocol: 1,
                provider: "claude_code".into(),
                providers: vec![],
                fs: false,
            },
        ));
    }

    /// Un tour complet : saisie, texte au fil de l'eau, confirmation, fin ;
    /// rien n'est envoyé pendant un tour ni sans moteur connecté.
    #[test]
    fn un_tour_de_conversation() {
        let mut app = app_with_chat();
        let s = cur(&app).session;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = app.chat_update(ChatMsg::Connected(s, tx));
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
            let _ = app.chat_update(ChatMsg::Engine(s, ServerMsg::Delta { text: t.into() }));
        }
        let _ = app.chat_update(ChatMsg::Engine(
            s,
            ServerMsg::ConfirmRequest {
                id: "c1".into(),
                command: "hyprctl keyword general:gaps_out 10".into(),
                reason: String::new(),
                risk: "low".into(),
            },
        ));
        // Une seule bulle pour le texte en flux ; la confirmation rend la main.
        assert_eq!(cur(&app).entries.len(), 3);
        assert!(!cur(&app).busy);
        let _ = app.chat_update(ChatMsg::Confirm("c1".into(), false));
        assert!(matches!(
            rx.try_recv(),
            Ok(ClientMsg::Confirm { accept: false, .. })
        ));
        let _ = app.chat_update(ChatMsg::Engine(
            s,
            ServerMsg::TurnEnd {
                id: "c1".into(),
                text: String::new(),
            },
        ));
        assert!(!cur(&app).busy);
        assert!(matches!(
            cur(&app).entries[2],
            Entry::Confirm {
                answer: Some(false),
                ..
            }
        ));
        // Moteur arrêté : l'envoi est impossible, la raison est affichée.
        let _ = app.chat_update(ChatMsg::Ended(s, "arrêt".into()));
        let _ = app.chat_update(ChatMsg::Send);
        assert!(cur(&app).sender.is_none());
    }

    /// Cacher le tiroir garde les conversations ; « Nouvelle » en ouvre une
    /// autre (son propre moteur) sans perdre la première.
    #[test]
    fn conversations_gardees() {
        let mut app = app_with_chat();
        let s = cur(&app).session;
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = app.chat_update(ChatMsg::Connected(s, tx));
        let _ = app.chat_update(ChatMsg::Input("bonjour".into()));
        let _ = app.chat_update(ChatMsg::Send);
        let _ = app.chat_update(ChatMsg::Engine(
            s,
            ServerMsg::Delta {
                text: "Salut **toi**".into(),
            },
        ));
        let _ = app.open_panel();
        assert!(!app.chat_shown());
        assert!(cur(&app).sender.is_some(), "moteur gardé");
        let _ = app.open_chat(vec!["moteur".into()]);
        assert!(app.chat_shown() && cur(&app).session == s && cur(&app).entries.len() == 2);
        // Sélection : la réponse passe en texte sélectionnable, puis revient.
        let _ = app.chat_update(ChatMsg::Select(1));
        assert!(cur(&app).selecting.is_some());
        let _ = app.chat_update(ChatMsg::Select(1));
        assert!(cur(&app).selecting.is_none());
        let _ = app.chat_update(ChatMsg::NewConversation);
        let chat = app.chat.as_ref().unwrap();
        assert_eq!(chat.convs.len(), 2);
        assert!(cur(&app).entries.is_empty() && cur(&app).session != s);
        // Retour sur la première, intacte ; puis on la ferme.
        let _ = app.chat_update(ChatMsg::Switch(s));
        assert_eq!(cur(&app).entries.len(), 2);
        let _ = app.chat_update(ChatMsg::CloseConversation(s));
        assert_eq!(app.chat.as_ref().unwrap().convs.len(), 1);
        assert_ne!(cur(&app).session, s);
    }

    /// Un module parle à LYRA : une conversation à lui, envoyée dès que son
    /// moteur est prêt — même si une autre conversation est occupée.
    #[test]
    fn un_module_ouvre_sa_conversation() {
        let mut app = app_with_chat();
        let first = cur(&app).session;
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = app.chat_update(ChatMsg::Connected(first, tx));
        let _ = app.chat_update(ChatMsg::Input("une longue question".into()));
        let _ = app.chat_update(ChatMsg::Send);
        assert!(cur(&app).busy);
        // Sans moteur LYRA installé, ask_assistant ne fait rien : on simule.
        app.chat_session += 1;
        let s = app.chat_session;
        let mut conv = Conversation::new(s, vec!["moteur".into()], "Mises à jour".into());
        conv.pending = Some("La mise à jour a échoué".into());
        conv.unread = true;
        app.chat.as_mut().unwrap().push(conv);
        assert_eq!(
            cur(&app).session,
            first,
            "la conversation en cours reste affichée"
        );
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = app.chat_update(ChatMsg::Connected(s, tx));
        assert!(matches!(rx.try_recv(), Ok(ClientMsg::Hello { .. })));
        assert!(rx.try_recv().is_err(), "rien avant « prêt »");
        ready(&mut app, s);
        assert!(
            matches!(rx.try_recv(), Ok(ClientMsg::Message { text, .. }) if text == "La mise à jour a échoué")
        );
        let _ = app.chat_update(ChatMsg::Show(s));
        assert_eq!(cur(&app).session, s);
        assert!(!cur(&app).unread);
    }

    /// Étapes techniques consécutives : un groupe, replié par défaut.
    #[test]
    fn etapes_techniques_groupees() {
        let tool = || Entry::Tool {
            name: "t".into(),
            summary: String::new(),
            status: "done".into(),
            result: None,
        };
        let entries = vec![
            Entry::User {
                text: "x".into(),
                images: 0,
            },
            tool(),
            tool(),
            Entry::Error("e".into()),
            tool(),
        ];
        assert_eq!(step_groups(&entries), vec![(1, 3), (4, 5)]);
    }

    /// Un seul tiroir : ouvrir les réglages ferme l'assistant.
    #[test]
    fn un_seul_tiroir() {
        let mut app = app_with_chat();
        assert!(app.chat.is_some());
        let _ = app.open_panel();
        assert!(!app.chat_shown(), "caché, pas perdu");
        assert!(app.panel.is_some());
    }

    #[test]
    fn tiroir_assistant_borne_par_son_ecran() {
        let mut app = app_with_chat();
        let win = app.chat.as_ref().unwrap().win;
        for (screen, expected) in [(640.0, 640.0), (900.0, 900.0), (1440.0, 780.0)] {
            app.sizes.insert(win, iced::Size::new(screen, 900.0));
            assert_eq!(app.panel_width(), expected);
        }
    }
}
