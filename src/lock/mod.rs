//! `hyprdesk lock` : écran de verrouillage, intégré au style de hyprdesk.
//!
//! Sécurité :
//! - protocole ext-session-lock-v1 (iced_sessionlock) : c'est Hyprland qui
//!   garantit le verrou ; si ce processus s'arrête, la session reste verrouillée ;
//! - processus distinct du démon, sans Lua (état lu dans `state::path()`) ;
//! - mot de passe vérifié par PAM sur un fil à part, effacé après usage ;
//!   pas de presse-papiers (pas de collage) ;
//! - une seule instance (verrou de fichier), rien ne déverrouille de l'extérieur.
//!
//! Anti-marquage : le contenu dérive lentement, et s'estompe après un moment
//! sans saisie (l'extinction de l'écran reste l'affaire de hypridle).

pub mod pam;
pub mod state;

use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crate::i18n::text;
use iced::widget::{Space, button, column, container, image, row, stack, text_input};
use iced::{Alignment, Border, Color, Element, Font, Length, Padding, Subscription, Task};
use iced_sessionlock::to_session_message;

use state::LockState;

/// Délai sans saisie avant d'estomper le contenu.
const IDLE_DIM: Duration = Duration::from_secs(20);

#[to_session_message]
#[derive(Debug, Clone)]
enum Msg {
    Tick,
    Input(String),
    Submit,
    Auth(pam::Outcome),
    Background(Option<image::Handle>),
    /// Scène étalée : la part de chaque écran (nom, image).
    Parts(Vec<(String, Option<image::Handle>)>),
    /// Taille d'une surface (associe la surface à son écran).
    Resized(iced::window::Id, iced::Size),
    Avatar(Option<image::Handle>),
    Media(Option<crate::widgets::MediaInfo>),
    PlayPause,
    Keyboard(Option<(String, bool)>),
    Activity,
}

struct Lock {
    st: LockState,
    font: Font,
    title_font: Font,
    user: String,
    real_name: String,
    password: String,
    checking: bool,
    failures: u32,
    error: Option<String>,
    messages: Vec<String>,
    started: Instant,
    last_input: Instant,
    background: Option<image::Handle>,
    /// Scène étalée : part de chaque écran, par nom.
    parts: std::collections::HashMap<String, image::Handle>,
    /// Surfaces dans leur ordre d'apparition (celui des sorties Wayland, donc
    /// des écrans de Hyprland) ; corrigé par la taille quand elle désigne un
    /// seul écran. Le protocole de verrouillage ne donne pas le nom de la sortie.
    seen: std::cell::RefCell<Vec<iced::window::Id>>,
    by_size: std::collections::HashMap<iced::window::Id, String>,
    avatar: Option<image::Handle>,
    battery: Option<(u8, bool)>,
    media: Option<crate::widgets::MediaInfo>,
    /// Disposition du clavier, majuscules verrouillées.
    keyboard: Option<(String, bool)>,
}

fn rgba(c: [u8; 4], a: f32) -> Color {
    Color::from_rgba8(c[0], c[1], c[2], (c[3] as f32 / 255.0) * a)
}

/// Fond préparé hors du fil de l'interface : réduit, et flouté si demandé.
fn load_background(path: PathBuf, blur: bool) -> Option<image::Handle> {
    let img = ::image::open(&path).ok()?;
    let img = if blur {
        // Petit, puis flou : l'agrandissement à l'écran lisse le reste.
        img.thumbnail(480, 480).blur(6.0)
    } else {
        img.thumbnail(3840, 3840)
    };
    let rgba = img.to_rgba8();
    Some(image::Handle::from_rgba(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}

/// Avatar : AccountsService, sinon `~/.face`.
fn load_avatar(user: &str) -> Option<image::Handle> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    [
        PathBuf::from("/var/lib/AccountsService/icons").join(user),
        home.join(".face"),
        home.join(".face.icon"),
    ]
    .into_iter()
    .find_map(|p| {
        let img = ::image::open(&p).ok()?.thumbnail(192, 192).to_rgba8();
        Some(image::Handle::from_rgba(
            img.width(),
            img.height(),
            img.into_raw(),
        ))
    })
}

/// Nom complet du compte (champ GECOS), sinon l'identifiant.
fn real_name(user: &str) -> String {
    // SAFETY : getpwuid rend une structure statique, lue aussitôt.
    let gecos = unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() || (*pw).pw_gecos.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr((*pw).pw_gecos)
                .to_string_lossy()
                .into_owned()
        }
    };
    gecos
        .split(',')
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(user)
        .to_string()
}

/// Batterie de l'ordinateur : (pourcentage, en charge).
fn battery() -> Option<(u8, bool)> {
    let dir = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for e in dir.flatten() {
        let p = e.path();
        let kind = std::fs::read_to_string(p.join("type")).unwrap_or_default();
        if kind.trim() != "Battery" {
            continue;
        }
        let cap: u8 = std::fs::read_to_string(p.join("capacity"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let status = std::fs::read_to_string(p.join("status")).unwrap_or_default();
        return Some((cap, status.trim() == "Charging"));
    }
    None
}

/// Disposition et majuscules du clavier principal, d'après Hyprland.
fn keyboard() -> Option<(String, bool)> {
    let v = hyprpanel_core::ipc::query_json("devices").ok()?;
    let kb = v["keyboards"]
        .as_array()?
        .iter()
        .find(|k| k["main"].as_bool() == Some(true))?
        .clone();
    Some((
        kb["active_keymap"].as_str().unwrap_or("").to_string(),
        kb["capsLock"].as_bool().unwrap_or(false),
    ))
}

const INPUT_ID: &str = "hyprdesk-lock-password";

/// Langue des dates (comme le bureau : `LC_TIME`, sinon `LANG`).
pub(crate) fn locale() -> chrono::Locale {
    crate::i18n::chrono_locale()
}

impl Lock {
    fn new() -> (Self, Task<Msg>) {
        let st = state::read();
        let font = match &st.font {
            Some(f) => Font::with_name(Box::leak(f.clone().into_boxed_str())),
            None => crate::fonts::DEFAULT,
        };
        let title_font = Font {
            weight: iced::font::Weight::Bold,
            ..font
        };
        let user = pam::current_user();
        let now = Instant::now();
        let lock = Lock {
            real_name: real_name(&user),
            font,
            title_font,
            password: String::new(),
            checking: false,
            failures: 0,
            error: None,
            messages: Vec::new(),
            started: now,
            last_input: now,
            background: None,
            parts: Default::default(),
            seen: Default::default(),
            by_size: Default::default(),
            avatar: None,
            battery: battery(),
            media: None,
            keyboard: None,
            user,
            st,
        };
        // Verrouillé tout de suite sur la couleur de fond ; l'image suit.
        let mut tasks = vec![iced::widget::operation::focus(INPUT_ID)];
        // Scène étalée : la part de chaque écran ; sinon une seule image.
        if !lock.st.outputs.is_empty() {
            let blur = lock.st.blur;
            let parts: Vec<(String, PathBuf)> = lock
                .st
                .outputs
                .iter()
                .filter_map(|o| Some((o.name.clone(), o.image.clone()?)))
                .collect();
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        parts
                            .into_iter()
                            .map(|(n, p)| (n, load_background(p, blur)))
                            .collect::<Vec<_>>()
                    })
                    .await
                    .unwrap_or_default()
                },
                Msg::Parts,
            ));
        }
        if let Some(p) = lock.st.image.clone() {
            let blur = lock.st.blur;
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || load_background(p, blur))
                        .await
                        .ok()
                        .flatten()
                },
                Msg::Background,
            ));
        }
        if lock.st.show_user {
            let u = lock.user.clone();
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || load_avatar(&u))
                        .await
                        .ok()
                        .flatten()
                },
                Msg::Avatar,
            ));
        }
        tasks.push(Task::done(Msg::Tick));
        (lock, Task::batch(tasks))
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Tick => {
                let secs = self.started.elapsed().as_secs();
                let mut t = Vec::new();
                if self.st.show_battery && secs.is_multiple_of(30) {
                    self.battery = battery();
                }
                if secs.is_multiple_of(2) {
                    t.push(Task::perform(
                        async { tokio::task::spawn_blocking(keyboard).await.ok().flatten() },
                        Msg::Keyboard,
                    ));
                    if self.st.show_media {
                        t.push(Task::perform(crate::widgets::media("*"), |v| match v {
                            crate::widgets::Value::Media(m) => Msg::Media(m),
                            _ => Msg::Media(None),
                        }));
                    }
                }
                Task::batch(t)
            }
            Msg::Input(s) => {
                self.last_input = Instant::now();
                if !self.checking {
                    self.password = s;
                    self.error = None;
                }
                Task::none()
            }
            Msg::Activity => {
                self.last_input = Instant::now();
                Task::none()
            }
            Msg::Submit => {
                self.last_input = Instant::now();
                if self.checking || self.password.is_empty() {
                    return Task::none();
                }
                self.checking = true;
                self.messages.clear();
                let mut pw = std::mem::take(&mut self.password);
                let user = self.user.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let r = pam::authenticate(&user, &pw);
                            pam::wipe(&mut pw);
                            r
                        })
                        .await
                        .unwrap_or_else(|_| pam::Outcome {
                            ok: false,
                            error: Some(crate::i18n::tr("vérification interrompue").into()),
                            messages: Vec::new(),
                        })
                    },
                    Msg::Auth,
                )
            }
            Msg::Auth(o) => {
                self.checking = false;
                if o.ok {
                    return Task::done(Msg::UnLock);
                }
                self.failures += 1;
                self.error = o.error;
                self.messages = o.messages;
                iced::widget::operation::focus(INPUT_ID)
            }
            Msg::Background(h) => {
                self.background = h;
                Task::none()
            }
            Msg::Parts(list) => {
                self.parts = list
                    .into_iter()
                    .filter_map(|(n, h)| Some((n, h?)))
                    .collect();
                Task::none()
            }
            Msg::Resized(id, size) => {
                let (w, h) = (size.width.round() as u32, size.height.round() as u32);
                let mut same = self
                    .st
                    .outputs
                    .iter()
                    .filter(|o| o.width == w && o.height == h);
                if let (Some(o), None) = (same.next(), same.next()) {
                    self.by_size.insert(id, o.name.clone());
                }
                Task::none()
            }
            Msg::Avatar(h) => {
                self.avatar = h;
                Task::none()
            }
            Msg::Media(m) => {
                self.media = m;
                Task::none()
            }
            Msg::Keyboard(k) => {
                self.keyboard = k;
                Task::none()
            }
            Msg::PlayPause => {
                self.last_input = Instant::now();
                match &self.media {
                    Some(m) => Task::perform(
                        crate::widgets::media_control(m.bus.clone(), "PlayPause"),
                        |_| Msg::Activity,
                    ),
                    None => Task::none(),
                }
            }
            Msg::UnLock => Task::none(),
        }
    }

    /// Écran d'une surface : d'après sa taille si elle le désigne, sinon
    /// d'après son ordre d'apparition.
    fn output_of(&self, id: iced::window::Id) -> Option<&str> {
        if self.st.outputs.is_empty() {
            return None;
        }
        if let Some(n) = self.by_size.get(&id) {
            return Some(n.as_str());
        }
        let mut seen = self.seen.borrow_mut();
        if !seen.contains(&id) {
            seen.push(id);
        }
        let i = seen.iter().position(|x| *x == id)?;
        self.st.outputs.get(i).map(|o| o.name.as_str())
    }

    fn view(&self, id: iced::window::Id) -> Element<'_, Msg> {
        let st = &self.st;
        let idle = self.last_input.elapsed() >= IDLE_DIM;
        // Anti-marquage : contenu estompé au repos, et lente dérive.
        let a = if idle { 0.45 } else { 1.0 };
        let fg = rgba(st.fg, a);
        let dim = rgba(st.fg, 0.65 * a);
        let accent = rgba(st.accent, a);
        let t = self.started.elapsed().as_secs_f32();
        let tau = std::f32::consts::TAU;
        let (dx, dy) = (
            48.0 * (t / 97.0 * tau).sin(),
            32.0 * (t / 131.0 * tau).sin(),
        );

        let now = chrono::Local::now();
        let mut col = column![
            text(now.format("%H:%M").to_string())
                .font(self.title_font)
                .size(120)
                .color(fg),
            text(now.format_localized("%A %-d %B", locale()).to_string())
                .font(self.font)
                .size(22)
                .color(dim),
            Space::new().height(28),
        ]
        .align_x(Alignment::Center)
        .spacing(4);
        if st.show_user {
            let mut who = column![].align_x(Alignment::Center).spacing(8);
            if let Some(h) = &self.avatar {
                who = who.push(
                    image(h.clone())
                        .width(88)
                        .height(88)
                        .border_radius(44.0)
                        .opacity(a),
                );
            }
            who = who.push(
                text(self.real_name.clone())
                    .font(self.title_font)
                    .size(18)
                    .color(fg),
            );
            col = col.push(who);
        }
        let field_border = if self.error.is_some() {
            Color::from_rgb8(0xff, 0x95, 0x80)
        } else {
            accent
        };
        let bg = rgba(st.background, 0.55);
        let input = text_input(
            if self.checking {
                crate::i18n::tr("Vérification…")
            } else {
                "Mot de passe"
            },
            &self.password,
        )
        .id(INPUT_ID)
        .secure(true)
        .on_input(Msg::Input)
        .on_submit(Msg::Submit)
        .font(self.font)
        .size(18)
        .padding(14)
        .width(340)
        .style(move |_, status| text_input::Style {
            background: bg.into(),
            border: Border::default().rounded(14).width(2).color(
                if matches!(status, text_input::Status::Focused { .. }) {
                    field_border
                } else {
                    Color {
                        a: 0.4,
                        ..field_border
                    }
                },
            ),
            icon: fg,
            placeholder: dim,
            value: fg,
            selection: Color { a: 0.3, ..accent },
        });
        col = col.push(input);
        // Indicateurs : majuscules, disposition, échecs et messages de PAM.
        let mut notes = column![].align_x(Alignment::Center).spacing(4);
        if let Some((layout, caps)) = &self.keyboard {
            let mut l = layout.clone();
            if *caps {
                l = crate::tr!("Majuscules verrouillées · {l}", l = l);
            }
            if !l.is_empty() {
                notes = notes.push(text(l).font(self.font).size(13).color(dim));
            }
        }
        if let Some(e) = &self.error {
            notes = notes.push(
                text(if self.failures > 1 {
                    crate::tr!("{e} ({n} échecs)", e = e, n = self.failures)
                } else {
                    crate::tr!("{e} ({n} échec)", e = e, n = self.failures)
                })
                .font(self.font)
                .size(14)
                .color(Color::from_rgb8(0xff, 0x95, 0x80)),
            );
        }
        for m in &self.messages {
            notes = notes.push(text(m.clone()).font(self.font).size(13).color(fg));
        }
        col = col.push(notes);

        // Bas d'écran : batterie et morceau en cours.
        let mut foot = row![].spacing(24).align_y(Alignment::Center);
        if st.show_battery
            && let Some((p, charging)) = self.battery
        {
            foot = foot.push(
                text(format!("{}{p} %", if charging { "⚡ " } else { "" }))
                    .font(self.font)
                    .size(14)
                    .color(dim),
            );
        }
        if st.show_media
            && let Some(m) = &self.media
        {
            let label = if m.artist.is_empty() {
                m.title.clone()
            } else {
                format!("{} — {}", m.title, m.artist)
            };
            foot = foot.push(
                row![
                    button(text(if m.playing { "⏸" } else { "▶" }).size(16).color(fg))
                        .padding([4, 10])
                        .style(move |_, _| button::Style {
                            background: Some(Color { a: 0.15 * a, ..fg }.into()),
                            border: Border::default().rounded(10),
                            text_color: fg,
                            ..Default::default()
                        })
                        .on_press(Msg::PlayPause),
                    text(label).font(self.font).size(14).color(dim),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }

        let content = container(col).center(Length::Fill).padding(Padding {
            left: (dx.max(0.0)) * 2.0,
            right: (-dx).max(0.0) * 2.0,
            top: (dy.max(0.0)) * 2.0,
            bottom: (-dy).max(0.0) * 2.0,
        });
        let footer = container(foot)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::End)
            .padding(Padding {
                bottom: 36.0 + dy.abs(),
                ..Padding::ZERO
            });

        let base = rgba(st.background, 1.0);
        let mut layers: Vec<Element<'_, Msg>> = vec![
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(base.into()),
                    ..Default::default()
                })
                .into(),
        ];
        // Scène étalée : la part de cet écran ; sinon l'image entière.
        let part = self.output_of(id).and_then(|n| self.parts.get(n));
        if let Some(h) = part.or(self.background.as_ref()) {
            layers.push(
                image(h.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Cover)
                    .into(),
            );
        }
        // Voile : assombrissement réglé, plus fort au repos.
        let veil = (st.dim + if idle { 0.25 } else { 0.0 }).clamp(0.0, 0.92);
        layers.push(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color::from_rgba(0.0, 0.0, 0.0, veil).into()),
                    ..Default::default()
                })
                .into(),
        );
        layers.push(content.into());
        layers.push(footer.into());
        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Msg> {
        Subscription::batch([
            iced::time::every(Duration::from_secs(1)).map(|_| Msg::Tick),
            iced::event::listen_with(|e, _, _| match e {
                iced::Event::Keyboard(_)
                | iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_)) => Some(Msg::Activity),
                _ => None,
            }),
            iced::event::listen_with(|e, _, id| match e {
                iced::Event::Window(iced::window::Event::Resized(size)) => {
                    Some(Msg::Resized(id, size))
                }
                _ => None,
            }),
        ])
    }
}

/// Une seule instance : verrou exclusif sur un fichier de la session.
fn single_instance() -> Option<std::fs::File> {
    let p = state::path().with_file_name("lock.pid");
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&p)
        .ok()?;
    // SAFETY : descripteur valide, gardé ouvert tant que le fichier vit.
    let r = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    (r == 0).then_some(f)
}

/// Point d'entrée de `hyprdesk lock`.
pub fn run() -> ExitCode {
    let Some(_guard) = single_instance() else {
        // Déjà verrouillé : rien à faire (hypridle peut appeler plusieurs fois).
        return ExitCode::SUCCESS;
    };
    // Pas de presse-papiers : rien à coller dans le champ du mot de passe.
    iced_sessionlock::disable_clipboard();
    let mut app = iced_sessionlock::application(Lock::new, Lock::update, Lock::view)
        .subscription(Lock::subscription)
        .default_font(crate::fonts::DEFAULT);
    for face in crate::fonts::bundled() {
        app = app.font(face);
    }
    match app.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", crate::tr!("hyprdesk lock : {e}", e = e));
            ExitCode::FAILURE
        }
    }
}
