//! `hyprdesk lock` : écran de verrouillage, intégré au style de hyprdesk.
//!
//! Sécurité :
//! - protocole ext-session-lock-v1 (iced_sessionlock) : c'est Hyprland qui
//!   garantit le verrou ; si ce processus s'arrête, la session reste verrouillée ;
//! - processus distinct du démon, sans Lua (état lu dans `state::path()`) ;
//! - mot de passe vérifié par PAM sur un fil à part (authentification puis
//!   phase account), effacé après usage (`zeroize`) ; pas de presse-papiers ;
//! - délai croissant après trois échecs (2 s, 4 s… 30 s), saisie bloquée ;
//! - empreinte désactivée par défaut et soumise à la politique du poste ;
//!   acceptée, elle remplace le mot de passe (voir `pam`) ;
//! - une seule instance (verrou de fichier), rien ne déverrouille de
//!   l'extérieur ; un verrou de fichier impossible à prendre n'empêche pas
//!   de verrouiller.
//!
//! Anti-marquage : le contenu dérive lentement, et s'estompe après un moment
//! sans saisie (l'extinction de l'écran reste l'affaire de hypridle).

pub mod fprint;
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
use zeroize::{Zeroize, Zeroizing};

/// Délai sans saisie avant d'estomper le contenu.
const IDLE_DIM: Duration = Duration::from_secs(20);

/// Attente imposée après `failures` échecs : rien jusqu'à deux, puis 2 s,
/// 4 s, 8 s, 16 s, et 30 s au plus.
fn backoff(failures: u32) -> Option<Duration> {
    (failures >= 3).then(|| Duration::from_secs((1u64 << (failures - 2).min(5)).min(30)))
}

#[to_session_message]
#[derive(Debug, Clone)]
enum Msg {
    Tick,
    Input(String),
    Submit,
    Auth(pam::Outcome),
    Background(Option<image::Handle>),
    /// Trame suivante du fond vidéo commun (boucle ffmpeg).
    VideoBackground(crate::video::Tiles),
    /// Scène étalée : la part de chaque écran (nom, image).
    Parts(Vec<(String, Option<image::Handle>)>),
    /// Trame suivante du fond vidéo d'un écran (boucle ffmpeg).
    VideoPart(String, crate::video::Tiles),
    /// Taille d'une surface (associe la surface à son écran).
    Resized(iced::window::Id, iced::Size),
    Avatar(Option<image::Handle>),
    Media(Option<crate::widgets::MediaInfo>),
    PlayPause,
    Keyboard(Option<(String, bool)>),
    Activity,
    /// Issue de la vérification d'empreinte (fprintd).
    Finger(fprint::Finger),
    /// Issue de PAM pour l'empreinte : service dédié, ou phase account
    /// après une empreinte reconnue par fprintd.
    FingerPam(pam::Outcome),
}

struct Lock {
    st: LockState,
    font: Font,
    title_font: Font,
    user: String,
    real_name: String,
    password: Zeroizing<String>,
    checking: bool,
    failures: u32,
    /// Saisie bloquée jusqu'à cet instant (délai croissant après des échecs).
    retry_at: Option<Instant>,
    error: Option<String>,
    messages: Vec<String>,
    started: Instant,
    last_input: Instant,
    background: Option<image::Handle>,
    /// Scène étalée : part de chaque écran, par nom.
    parts: std::collections::HashMap<String, image::Handle>,
    /// Fond vidéo : trame courante en bandes (voir `video::Tiles` : sans
    /// elles, le fond clignote) — fond commun, et part de chaque écran.
    video_background: Option<crate::video::Tiles>,
    video_parts: std::collections::HashMap<String, crate::video::Tiles>,
    /// Surfaces dans leur ordre d'apparition (celui des sorties Wayland, donc
    /// des écrans de Hyprland) ; corrigé par la taille quand elle désigne un
    /// seul écran. Le protocole de verrouillage ne donne pas le nom de la sortie.
    seen: std::cell::RefCell<Vec<iced::window::Id>>,
    by_size: std::collections::HashMap<iced::window::Id, String>,
    /// Taille de chaque surface : un écran bandeau (Panorama) reçoit une
    /// mise en page en ligne.
    sizes: std::collections::HashMap<iced::window::Id, iced::Size>,
    avatar: Option<image::Handle>,
    battery: Option<(u8, bool)>,
    media: Option<crate::widgets::MediaInfo>,
    /// Disposition du clavier, majuscules verrouillées.
    keyboard: Option<(String, bool)>,
    /// Vérification d'empreinte en cours (indication « posez le doigt »).
    finger: bool,
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
            password: Zeroizing::new(String::new()),
            checking: false,
            failures: 0,
            retry_at: None,
            error: None,
            messages: Vec::new(),
            started: now,
            last_input: now,
            background: None,
            parts: Default::default(),
            video_background: None,
            video_parts: Default::default(),
            seen: Default::default(),
            by_size: Default::default(),
            sizes: Default::default(),
            avatar: None,
            battery: battery(),
            media: None,
            keyboard: None,
            finger: false,
            user,
            st,
        };
        // Verrouillé tout de suite sur la couleur de fond ; l'image suit.
        let mut tasks = vec![iced::widget::operation::focus(INPUT_ID)];
        // Scène étalée : la part de chaque écran ; sinon une seule image.
        // Les vidéos de fond ne passent pas par le décodage d'image : la
        // boucle ffmpeg les apporte (souscription).
        if !lock.st.outputs.is_empty() {
            let blur = lock.st.blur;
            let parts: Vec<(String, PathBuf)> = lock
                .st
                .outputs
                .iter()
                .filter_map(|o| Some((o.name.clone(), o.image.clone()?)))
                .filter(|(_, p)| !crate::video::is_video(p))
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
        if let Some(p) = lock.st.image.clone().filter(|p| !crate::video::is_video(p)) {
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
        // Empreinte en parallèle du mot de passe, si elle est choisie et que
        // la politique du poste l'autorise (sans lecteur : rien ne s'affiche).
        // Service PAM dédié s'il est posé, sinon fprintd puis phase account.
        let mut lock = lock;
        if lock.st.fingerprint && crate::policy::get().fingerprint_allowed() {
            lock.finger = true;
            let u = lock.user.clone();
            if pam::fingerprint_service().is_some() {
                tasks.push(Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || pam::fingerprint(&u))
                            .await
                            .unwrap_or_else(|_| pam::Outcome {
                                ok: false,
                                error: None,
                                messages: Vec::new(),
                            })
                    },
                    Msg::FingerPam,
                ));
            } else {
                tasks.push(Task::perform(
                    async move { fprint::verify(&u).await },
                    Msg::Finger,
                ));
            }
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
            Msg::Input(mut s) => {
                self.last_input = Instant::now();
                if !self.checking && !self.blocked() {
                    // L'ancienne valeur est effacée, pas seulement libérée.
                    self.password.zeroize();
                    *self.password = s;
                    self.error = None;
                } else {
                    s.zeroize();
                }
                Task::none()
            }
            Msg::Activity => {
                self.last_input = Instant::now();
                Task::none()
            }
            Msg::Submit => {
                self.last_input = Instant::now();
                if self.checking || self.password.is_empty() || self.blocked() {
                    return Task::none();
                }
                self.checking = true;
                self.messages.clear();
                // Effacé à la fin de la vérification (Zeroizing).
                let pw = Zeroizing::new(std::mem::take(&mut *self.password));
                let user = self.user.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || pam::authenticate(&user, &pw))
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
            Msg::Finger(f) => match f {
                // Empreinte reconnue : la phase account de PAM décide encore.
                fprint::Finger::Matched => {
                    let u = self.user.clone();
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || pam::account(&u))
                                .await
                                .unwrap_or_else(|_| pam::Outcome {
                                    ok: false,
                                    error: None,
                                    messages: Vec::new(),
                                })
                        },
                        Msg::FingerPam,
                    )
                }
                fprint::Finger::Unavailable => {
                    self.finger = false;
                    Task::none()
                }
                fprint::Finger::Failed(m) => {
                    self.finger = false;
                    self.messages.push(m);
                    Task::none()
                }
            },
            Msg::FingerPam(o) => {
                self.finger = false;
                if o.ok {
                    return Task::done(Msg::UnLock);
                }
                self.messages.extend(o.messages);
                self.messages.extend(o.error);
                Task::none()
            }
            Msg::Auth(o) => {
                self.checking = false;
                if o.ok {
                    return Task::done(Msg::UnLock);
                }
                self.failures += 1;
                self.retry_at = backoff(self.failures).map(|d| Instant::now() + d);
                self.error = o.error;
                self.messages = o.messages;
                iced::widget::operation::focus(INPUT_ID)
            }
            Msg::Background(h) => {
                self.background = h;
                self.video_background = None;
                Task::none()
            }
            Msg::VideoBackground(t) => {
                self.video_background = Some(t);
                Task::none()
            }
            Msg::Parts(list) => {
                self.video_parts.clear();
                self.parts = list
                    .into_iter()
                    .filter_map(|(n, h)| Some((n, h?)))
                    .collect();
                Task::none()
            }
            Msg::VideoPart(n, t) => {
                self.video_parts.insert(n, t);
                Task::none()
            }
            Msg::Resized(id, size) => {
                self.sizes.insert(id, size);
                let (w, h) = (size.width.round() as u32, size.height.round() as u32);
                let mut same = self
                    .st
                    .outputs
                    .iter()
                    .filter(|o| o.width == w && o.height == h);
                if let (Some(o), None) = (same.next(), same.next()) {
                    self.by_size.insert(id, o.name.clone());
                }
                // Surface de verrouillage prête : le champ du mot de passe
                // prend le focus (demandé au démarrage, avant les surfaces,
                // il se perdait : il fallait cliquer avant de taper).
                if self.checking {
                    Task::none()
                } else {
                    iced::widget::operation::focus(INPUT_ID)
                }
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

    /// Saisie bloquée par le délai qui suit des échecs.
    fn blocked(&self) -> bool {
        self.retry_at.is_some_and(|t| Instant::now() < t)
    }

    /// Secondes restantes avant un nouvel essai, pendant le délai.
    fn wait_secs(&self) -> Option<u64> {
        let left = self.retry_at?.checked_duration_since(Instant::now())?;
        Some(left.as_secs() + u64::from(left.subsec_nanos() > 0))
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
        // Écran bandeau (Panorama : 2560×720, 1920×480…) : tout tient en
        // ligne, la hauteur ne suffit pas à la colonne.
        let strip = self.sizes.get(&id).is_some_and(|s| is_strip(s.width, s.height));
        let t = self.started.elapsed().as_secs_f32();
        let tau = std::f32::consts::TAU;
        let (dx, dy) = (
            48.0 * (t / 97.0 * tau).sin(),
            if strip { 10.0 } else { 32.0 } * (t / 131.0 * tau).sin(),
        );

        let now = chrono::Local::now();
        let side = if strip { Alignment::Start } else { Alignment::Center };
        let clock = column![
            text(now.format("%H:%M").to_string())
                .font(self.title_font)
                .size(120)
                .color(fg),
            text(now.format_localized("%A %-d %B", locale()).to_string())
                .font(self.font)
                .size(22)
                .color(dim),
        ]
        .align_x(side)
        .spacing(4);
        let mut col = column![].align_x(side).spacing(4);
        // En bandeau, l'heure va à côté de la colonne (voir `body`).
        let mut clock = Some(clock);
        if !strip && let Some(c) = clock.take() {
            col = col.push(c).push(Space::new().height(28));
        }
        if st.show_user {
            // En bandeau : avatar plus petit, à côté du nom.
            let size = if strip { 48.0 } else { 88.0 };
            let avatar = self.avatar.as_ref().map(|h| {
                image(h.clone())
                    .width(size)
                    .height(size)
                    .border_radius(size / 2.0)
                    .opacity(a)
            });
            let name = text(self.real_name.clone())
                .font(self.title_font)
                .size(18)
                .color(fg);
            let who: Element<'_, Msg> = if strip {
                let mut r = row![].spacing(12).align_y(Alignment::Center);
                if let Some(i) = avatar {
                    r = r.push(i);
                }
                r.push(name).into()
            } else {
                let mut c = column![].align_x(Alignment::Center).spacing(8);
                if let Some(i) = avatar {
                    c = c.push(i);
                }
                c.push(name).into()
            };
            col = col.push(who).push(Space::new().height(if strip { 6 } else { 0 }));
        }
        let field_border = if self.error.is_some() {
            Color::from_rgb8(0xff, 0x95, 0x80)
        } else {
            accent
        };
        let bg = rgba(st.background, 0.55);
        let wait = self.wait_secs();
        let input = text_input(
            if self.checking {
                crate::i18n::tr("Vérification…")
            } else if wait.is_some() {
                crate::i18n::tr("Patientez…")
            } else {
                "Mot de passe"
            },
            self.password.as_str(),
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
        let mut notes = column![].align_x(side).spacing(4);
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
        if let Some(n) = wait {
            notes = notes.push(
                text(crate::tr!("Nouvel essai possible dans {n} s", n = n))
                    .font(self.font)
                    .size(13)
                    .color(fg),
            );
        }
        for m in &self.messages {
            notes = notes.push(text(m.clone()).font(self.font).size(13).color(fg));
        }
        if self.finger {
            notes = notes.push(
                text(crate::i18n::tr(
                    "Ou posez le doigt sur le lecteur d'empreintes.",
                ))
                .font(self.font)
                .size(13)
                .color(dim),
            );
        }
        col = col.push(notes);
        // Bandeau : l'heure à gauche, le compte et la saisie à droite.
        let body: Element<'_, Msg> = if let Some(clock) = clock {
            row![clock, col]
                .spacing(64)
                .align_y(Alignment::Center)
                .into()
        } else {
            col.into()
        };

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

        let content = container(body).center(Length::Fill).padding(Padding {
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
                bottom: if strip { 14.0 } else { 36.0 } + dy.abs(),
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
        let name = self.output_of(id);
        let video = name
            .and_then(|n| self.video_parts.get(n))
            .or(self.video_background.as_ref());
        let part = name.and_then(|n| self.parts.get(n));
        if let Some(t) = video {
            layers.push(t.view(1.0));
        } else if let Some(h) = part.or(self.background.as_ref()) {
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
        let mut subs = vec![
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
        ];
        // Fonds vidéo : une boucle ffmpeg pour le fond commun, une par écran
        // qui a le sien. Sans ffmpeg : la couleur de fond reste.
        if crate::video::has_decoder() {
            let blur = self.st.blur;
            if let Some(p) = self.st.image.as_ref().filter(|p| crate::video::is_video(p)) {
                let (w, h) = if blur {
                    crate::video::target_size(1920, 1080, 480)
                } else {
                    crate::video::target_size(1920, 1080, 1920)
                };
                subs.push(Subscription::run_with(
                    (p.clone(), w, h, blur),
                    |k: &(PathBuf, u32, u32, bool)| video_background(k),
                ));
            }
            for o in &self.st.outputs {
                if let Some(p) = o.image.as_ref().filter(|p| crate::video::is_video(p)) {
                    let (w, h) = crate::video::target_size(
                        o.width.max(1),
                        o.height.max(1),
                        if blur { 480 } else { 1920 },
                    );
                    subs.push(Subscription::run_with(
                        (o.name.clone(), p.clone(), w, h, blur),
                        |k: &(String, PathBuf, u32, u32, bool)| video_part(k),
                    ));
                }
            }
        }
        Subscription::batch(subs)
    }
}

/// Boucle du fond vidéo commun : trames du décodeur partagé (flouté comme
/// l'image du verrouillage l'est).
fn video_background(
    key: &(PathBuf, u32, u32, bool),
) -> impl iced::futures::Stream<Item = Msg> + use<> {
    let (path, w, h, blur) = key.clone();
    let recipe = crate::video::Recipe {
        path,
        w,
        h,
        fps: 30,
        blur: blur.then_some(6.0),
        span: None,
        whole: false,
    };
    use iced::futures::StreamExt;
    crate::video::frames(recipe).map(|f| Msg::VideoBackground(f.tiles))
}

/// Boucle du fond vidéo d'un écran (son propre fond) : trames pour sa
/// surface.
fn video_part(
    key: &(String, PathBuf, u32, u32, bool),
) -> impl iced::futures::Stream<Item = Msg> + use<> {
    let (name, path, w, h, blur) = key.clone();
    let recipe = crate::video::Recipe {
        path,
        w,
        h,
        fps: 30,
        blur: blur.then_some(6.0),
        span: None,
        whole: false,
    };
    use iced::futures::StreamExt;
    crate::video::frames(recipe).map(move |f| Msg::VideoPart(name.clone(), f.tiles))
}

/// Surface d'écran bandeau (Panorama) : trop basse ou trop allongée pour
/// la colonne habituelle (heure, compte, saisie), qui y déborderait.
fn is_strip(w: f32, h: f32) -> bool {
    h > 0.0 && w > h && (h < 560.0 || w / h >= crate::panorama::detect::MIN_RATIO)
}

/// Issue de la prise du verrou d'instance.
#[derive(Debug)]
enum Instance {
    /// Verrou pris : cette instance verrouille la session.
    Held(std::fs::File),
    /// Une autre instance le tient : la session est déjà verrouillée.
    Already,
    /// Verrou impossible à prendre (dossier, fichier) : à ne pas confondre
    /// avec « déjà verrouillé ».
    Failed(String),
}

/// Une seule instance : verrou exclusif sur un fichier de la session.
fn single_instance() -> Instance {
    match state::ensure_dir() {
        Ok(p) => try_instance(&p.with_file_name("lock.pid")),
        Err(e) => Instance::Failed(e),
    }
}

fn try_instance(p: &std::path::Path) -> Instance {
    use std::os::unix::fs::OpenOptionsExt;
    let f = match std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(p)
    {
        Ok(f) => f,
        Err(e) => return Instance::Failed(format!("{} : {e}", p.display())),
    };
    // SAFETY : descripteur valide, gardé ouvert tant que le fichier vit.
    let r = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if r == 0 {
        return Instance::Held(f);
    }
    let e = std::io::Error::last_os_error();
    if e.raw_os_error() == Some(libc::EWOULDBLOCK) {
        Instance::Already
    } else {
        Instance::Failed(format!("flock {} : {e}", p.display()))
    }
}

/// Un `hyprdesk lock` tient son verrou : la session est verrouillée.
///
/// Lecture seule de `/proc/locks` : ne jamais essayer `flock` ici, l'essai
/// ferait échouer un verrouillage qui démarre au même instant (il se
/// croirait déjà lancé et laisserait la session ouverte).
pub fn running() -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(p) = state::path() else {
        return false;
    };
    let Ok(m) = std::fs::metadata(p.with_file_name("lock.pid")) else {
        return false;
    };
    let Ok(locks) = std::fs::read_to_string("/proc/locks") else {
        return false;
    };
    holds_flock(&locks, libc::major(m.dev()), libc::minor(m.dev()), m.ino())
}

/// `/proc/locks` contient un FLOCK actif sur ce fichier (périphérique
/// `majeur:mineur` en hexadécimal, inode en décimal ; les attentes `->` ne
/// comptent pas).
fn holds_flock(locks: &str, major: u32, minor: u32, ino: u64) -> bool {
    let id = format!("{major:02x}:{minor:02x}:{ino}");
    locks.lines().any(|l| {
        let f: Vec<&str> = l.split_whitespace().collect();
        f.get(1) == Some(&"FLOCK") && f.get(5) == Some(&id.as_str())
    })
}

/// Point d'entrée de `hyprdesk lock`.
pub fn run() -> ExitCode {
    let _guard = match single_instance() {
        Instance::Held(f) => Some(f),
        // Déjà verrouillé : rien à faire (hypridle peut appeler plusieurs fois).
        Instance::Already => return ExitCode::SUCCESS,
        // Verrou d'instance impossible : verrouiller quand même (une session
        // laissée ouverte serait pire qu'un second écran de verrouillage).
        Instance::Failed(e) => {
            let msg = crate::tr!(
                "hyprdesk lock : verrou d'instance impossible ({e}), verrouillage quand même",
                e = e
            );
            eprintln!("{msg}");
            pam::journal(&msg);
            None
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verrou_lu_dans_proc_locks() {
        let locks = "1: POSIX  ADVISORY  WRITE 18739 00:20:71843340 0 EOF\n\
                     5: FLOCK  ADVISORY  WRITE 1022 00:20:149598 0 EOF\n\
                     5: -> FLOCK  ADVISORY  WRITE 1023 00:20:777 0 EOF\n";
        assert!(holds_flock(locks, 0, 0x20, 149598));
        assert!(!holds_flock(locks, 0, 0x20, 71843340), "POSIX, pas FLOCK");
        assert!(!holds_flock(locks, 0, 0x20, 777), "attente, pas détention");
        assert!(!holds_flock(locks, 0, 0x21, 149598));
    }

    #[test]
    fn bandeau_en_ligne() {
        // 2560×720 à 1,6 ; 1920×480 ; 1280×400 : en ligne.
        assert!(is_strip(1600.0, 450.0));
        assert!(is_strip(1920.0, 480.0));
        assert!(is_strip(1280.0, 400.0));
        // Écrans ordinaires, et bandeau en portrait : la colonne tient.
        assert!(!is_strip(2560.0, 1440.0));
        assert!(!is_strip(1440.0, 2560.0));
        assert!(!is_strip(450.0, 1600.0));
        assert!(!is_strip(0.0, 0.0));
    }

    #[test]
    fn delai_croissant() {
        assert_eq!(backoff(0), None);
        assert_eq!(backoff(2), None);
        let s = |n| backoff(n).unwrap().as_secs();
        assert_eq!([s(3), s(4), s(5), s(6), s(7), s(20)], [2, 4, 8, 16, 30, 30]);
    }

    /// Verrou déjà tenu (succès, rien à faire) distinct d'un verrou
    /// impossible à prendre (on verrouille quand même).
    #[test]
    fn verrou_tenu_ou_impossible() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-lockpid-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("lock.pid");
        let first = try_instance(&p);
        assert!(matches!(first, Instance::Held(_)), "{first:?}");
        assert!(matches!(try_instance(&p), Instance::Already));
        drop(first);
        assert!(matches!(try_instance(&p), Instance::Held(_)));
        assert!(matches!(
            try_instance(&dir.join("absent").join("lock.pid")),
            Instance::Failed(_)
        ));
        // Un lien posé à la place du fichier n'est pas suivi.
        let link = dir.join("lien.pid");
        std::os::unix::fs::symlink(&p, &link).unwrap();
        assert!(matches!(try_instance(&link), Instance::Failed(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
