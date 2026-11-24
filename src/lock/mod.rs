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
//! Anti-marquage : le contenu dérive lentement, sauf à « Mouvement réduit » ;
//! après un moment sans saisie, seul le voile du fond se renforce (le texte
//! garde son contraste ; l'extinction de l'écran reste l'affaire de hypridle).

// Authentification : crate `hyprdesk-auth` (crates/), chemins inchangés.
pub use hyprdesk_auth::{enroll, fprint, pam};
mod scope;
pub mod state;

use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crate::config::Rgba;
use crate::i18n::text;
use crate::theme;
use iced::widget::{Space, button, column, container, image, row, stack, text_input};
use iced::{Alignment, Border, Color, Element, Font, Length, Padding, Subscription, Task};
use iced_sessionlock::to_session_message;

use state::LockState;
use zeroize::{Zeroize, Zeroizing};

/// Délai sans saisie avant de renforcer le voile du fond.
const IDLE_DIM: Duration = Duration::from_secs(20);
/// Plus petit contraste du texte posé sur le fond (HIG ACC-02 : 4,5:1
/// jusqu'à 17 px ; l'horloge et la date, plus grandes, en ont plus).
const TEXT_CONTRAST: f64 = 4.5;
/// Opacité du texte secondaire : date, notes, pied d'écran.
const SECONDARY: f32 = 0.65;
/// Message d'erreur : la couleur du thème.
const ERROR: Rgba = Rgba(0xff, 0x95, 0x80, 0xff);

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
    Background(Option<Background>),
    /// Trame suivante du fond vidéo commun (boucle ffmpeg).
    VideoBackground(crate::video::Tiles),
    /// Scène étalée : la part de chaque écran (nom, image).
    Parts(Vec<(String, Option<Background>)>),
    /// Trame suivante du fond vidéo d'un écran (boucle ffmpeg).
    VideoPart(String, crate::video::Tiles),
    /// Taille d'une surface (associe la surface à son écran).
    Resized(iced::window::Id, iced::Size),
    Avatar(Option<image::Handle>),
    Media(Option<crate::widgets::MediaInfo>),
    PlayPause,
    /// Touche multimédia « Lecture et pause » : comme le bouton (HIG ACC-08).
    MediaKey,
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
    background: Option<Background>,
    /// Scène étalée : part de chaque écran, par nom.
    parts: std::collections::HashMap<String, Background>,
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

fn tint(c: Rgba, alpha: f32) -> Color {
    rgba([c.0, c.1, c.2, c.3], alpha)
}

fn opaque(c: [u8; 4]) -> Rgba {
    Rgba(c[0], c[1], c[2], 0xff)
}

/// Corps de texte à l'échelle réglée (`general.text_scale`), comme `tokens::ts`
/// dans le bureau — sans le rôle du thème, que le verrouillage et l'écran de
/// connexion ne chargent pas — et jamais sous le minimum lisible (HIG ACC-06).
pub(crate) fn ts(reference: f32, scale: f32) -> f32 {
    (reference * scale.clamp(0.75, 2.0)).max(crate::ui::tokens::MIN_TEXT)
}

/// Ce que le voile du fond doit rendre lisible, et la couleur de ce voile
/// (HIG ACC-02). Sous un texte clair, le voile est noir : l'obscurité fait le
/// contraste. Sous un texte sombre (thème clair), il prend la couleur du fond
/// du thème : c'est la lumière qui le fait.
#[derive(Debug, Clone, Copy)]
struct Ink {
    veil: Rgba,
    /// Texte secondaire : celui du thème, à [`SECONDARY`]. Le texte principal
    /// en est plus contrasté : ce qui est garanti à l'un l'est à l'autre.
    secondary: (Rgba, f32),
    /// Message d'erreur : la couleur du thème, assez contrastée sur le voile.
    error: Rgba,
}

impl Ink {
    fn of(st: &LockState) -> Self {
        let fg = opaque(st.fg);
        let bg = opaque(st.background);
        let black = Rgba(0, 0, 0, 0xff);
        let veil = if theme::contrast(bg, fg) > theme::contrast(black, fg) {
            bg
        } else {
            black
        };
        Ink {
            veil,
            secondary: (fg, SECONDARY),
            error: theme::ensure_contrast(ERROR, veil, fg, TEXT_CONTRAST),
        }
    }

    /// Le plus petit voile (0..1) sous lequel chaque pixel garde avec chaque
    /// texte un contraste de [`TEXT_CONTRAST`] ; 0 si aucun n'y suffit.
    fn veil_over(&self, pixels: &[Rgba]) -> f32 {
        let (mut lo, mut hi) = (0.0f64, 1.0f64);
        // Déjà lisible, ou voile qui ne peut rien : le plancher réglé seul.
        if !self.readable_over(pixels, hi) || self.readable_over(pixels, lo) {
            return 0.0;
        }
        for _ in 0..12 {
            let mid = (lo + hi) / 2.0;
            if self.readable_over(pixels, mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi as f32
    }

    /// Sous ce voile, chaque pixel laisse chaque texte à son contraste.
    fn readable_over(&self, pixels: &[Rgba], veil: f64) -> bool {
        pixels.iter().all(|p| {
            let bg = theme::mix(*p, self.veil, veil);
            [self.secondary, (self.error, 1.0)].iter().all(|(c, a)| {
                theme::contrast(theme::mix(bg, *c, f64::from(*a)), bg) >= TEXT_CONTRAST
            })
        })
    }
}

/// Fond préparé hors du fil de l'interface : réduit, et flouté si demandé ;
/// avec le voile minimal qui garde le texte lisible dessus (HIG ACC-02).
#[derive(Debug, Clone)]
struct Background {
    handle: image::Handle,
    veil: f32,
}

fn load_background(path: PathBuf, blur: bool, ink: Ink) -> Option<Background> {
    let img = ::image::open(&path).ok()?;
    // Petit, puis flou : l'agrandissement à l'écran lisse le reste.
    let small = img.thumbnail(480, 480);
    let shown = if blur {
        small.blur(6.0)
    } else {
        img.thumbnail(3840, 3840)
    };
    // Mesuré sur l'image floutée même, ou sur une réduction de l'image nette :
    // l'image telle qu'elle se lit à l'écran, par zones de la taille d'une lettre.
    let measured = if blur { &shown } else { &small };
    let veil = ink.veil_over(&pixels(&measured.to_rgba8()));
    let rgba = shown.to_rgba8();
    Some(Background {
        handle: image::Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw()),
        veil,
    })
}

fn pixels(img: &::image::RgbaImage) -> Vec<Rgba> {
    img.pixels().map(|p| opaque(p.0)).collect()
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
        let ink = Ink::of(&lock.st);
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
                            .map(|(n, p)| (n, load_background(p, blur, ink)))
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
                    tokio::task::spawn_blocking(move || load_background(p, blur, ink))
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
            Msg::MediaKey => {
                // Comme le bouton, seulement s'il est montré.
                if self.st.show_media {
                    self.update(Msg::PlayPause)
                } else {
                    Task::none()
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
        // Le texte garde toujours son opacité : au repos, seul le voile du fond
        // se renforce (HIG ACC-03). `ink` dit ce que ce voile rend lisible.
        let ink = Ink::of(st);
        // Corps du texte à l'échelle réglée (HIG ACC-06) ; `k` la porte sur ce
        // qui enveloppe le texte : champ, icônes.
        let px = |v: f32| ts(v, st.text_scale);
        let k = st.text_scale.clamp(0.75, 2.0);
        let fg = rgba(st.fg, 1.0);
        let dim = rgba(st.fg, SECONDARY);
        let accent = rgba(st.accent, 1.0);
        // Écran bandeau (Panorama : 2560×720, 1920×480…) : tout tient en
        // ligne, la hauteur ne suffit pas à la colonne.
        let strip = self
            .sizes
            .get(&id)
            .is_some_and(|s| is_strip(s.width, s.height));
        // Anti-marquage : lente dérive du contenu.
        let (dx, dy) = drift(
            self.started.elapsed().as_secs_f32(),
            strip,
            st.reduced_motion,
        );

        let now = chrono::Local::now();
        let side = if strip {
            Alignment::Start
        } else {
            Alignment::Center
        };
        let clock = column![
            text(now.format("%H:%M").to_string())
                .font(self.title_font)
                .size(px(120.0))
                .color(fg),
            text(now.format_localized("%A %-d %B", locale()).to_string())
                .font(self.font)
                .size(px(22.0))
                .color(dim),
        ]
        .align_x(side)
        .spacing(4);
        let mut col = column![].align_x(side).spacing(4);
        // En bandeau, l'heure va à côté de la colonne (voir `body`).
        let mut clock = Some(clock);
        // Horloge ailleurs qu'au centre (MLG-279) : posée à part, plus bas.
        let apart = !strip && st.clock_position != "center";
        let mut placed_clock = None;
        if apart {
            placed_clock = clock.take();
        } else if !strip && let Some(c) = clock.take() {
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
            });
            let name = text(self.real_name.clone())
                .font(self.title_font)
                .size(px(18.0))
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
            col = col
                .push(who)
                .push(Space::new().height(if strip { 6 } else { 0 }));
        }
        let error = Color::from_rgba8(ink.error.0, ink.error.1, ink.error.2, 1.0);
        let field_border = if self.error.is_some() { error } else { accent };
        // Densité réglée du panneau de saisie (MLG-279).
        let bg = rgba(st.background, st.panel_opacity);
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
        .size(px(18.0))
        .padding(14.0 * k)
        .width(340.0 * k)
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
                notes = notes.push(text(l).font(self.font).size(px(13.0)).color(dim));
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
                .size(px(14.0))
                .color(error),
            );
        }
        if let Some(n) = wait {
            notes = notes.push(
                text(crate::tr!("Nouvel essai possible dans {n} s", n = n))
                    .font(self.font)
                    .size(px(13.0))
                    .color(fg),
            );
        }
        for m in &self.messages {
            notes = notes.push(text(m.clone()).font(self.font).size(px(13.0)).color(fg));
        }
        if self.finger {
            notes = notes.push(
                text(crate::i18n::tr(
                    "Ou posez le doigt sur le lecteur d'empreintes.",
                ))
                .font(self.font)
                .size(px(13.0))
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
                row![
                    // Batterie du jeu, éclair dedans quand elle charge (HIG ICO-07).
                    crate::ui::icons::glyph(
                        &format!(
                            "battery-level-{p}{}",
                            if charging { "-charging" } else { "" }
                        ),
                        16.0 * k,
                        dim,
                    )
                    .expect("glyphe de batterie"),
                    text(format!("{p} %"))
                        .font(self.font)
                        .size(px(14.0))
                        .color(dim),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
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
                    // Lecture et pause : les glyphes du jeu (HIG ICO-07).
                    button(
                        crate::ui::icons::glyph(
                            if m.playing {
                                "media-playback-pause"
                            } else {
                                "media-playback-start"
                            },
                            16.0 * k,
                            fg,
                        )
                        .expect("glyphe de lecture"),
                    )
                    .padding([6, 10])
                    .style(move |_, _| button::Style {
                        background: Some(Color { a: 0.15, ..fg }.into()),
                        border: Border::default().rounded(10),
                        text_color: fg,
                        ..Default::default()
                    })
                    .on_press(Msg::PlayPause),
                    text(label).font(self.font).size(px(14.0)).color(dim),
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
        } else if let Some(b) = part.or(self.background.as_ref()) {
            layers.push(
                image(b.handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Cover)
                    .into(),
            );
        }
        // Voile : le plancher réglé (`lock.dim`), relevé jusqu'au contraste du
        // texte quand le fond l'exige (HIG ACC-02) ; plus fort au repos. Les
        // fonds vidéo changent de trame : le plancher seul.
        let shown = match video {
            Some(_) => None,
            None => Some(part.or(self.background.as_ref())),
        };
        let required = match shown {
            None => 0.0,
            Some(Some(b)) => b.veil,
            Some(None) => ink.veil_over(&[opaque(st.background)]),
        };
        let veil = (st.dim.clamp(0.0, 0.9).max(required) + if idle { 0.25 } else { 0.0 }).min(1.0);
        let tint = tint(ink.veil, veil);
        layers.push(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(tint.into()),
                    ..Default::default()
                })
                .into(),
        );
        layers.push(content.into());
        if let Some(c) = placed_clock {
            let (ax, ay) = if st.clock_position == "top" {
                (Alignment::Center, Alignment::Start)
            } else {
                (Alignment::Start, Alignment::End)
            };
            layers.push(
                container(c)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(ax)
                    .align_y(ay)
                    .padding(64)
                    .into(),
            );
        }
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
            iced::event::listen_with(|e, _, _| match &e {
                // La touche maintenue ne bascule pas sans fin.
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key,
                    repeat: false,
                    ..
                }) if is_play_key(key) => Some(Msg::MediaKey),
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

/// Touche multimédia « Lecture et pause » (HIG ACC-08) : la seule que
/// l'écran interprète, comme le bouton du morceau en cours.
fn is_play_key(key: &iced::keyboard::Key) -> bool {
    matches!(
        key,
        iced::keyboard::Key::Named(iced::keyboard::key::Named::MediaPlayPause)
    )
}

/// Dérive du contenu : ±48 px horizontalement sur 97 s, ±10 px (bandeau) ou
/// ±32 px verticalement sur 131 s ; rien à « Mouvement réduit » (HIG ACC-05).
fn drift(t: f32, strip: bool, reduced_motion: bool) -> (f32, f32) {
    if reduced_motion {
        return (0.0, 0.0);
    }
    let tau = std::f32::consts::TAU;
    (
        48.0 * (t / 97.0 * tau).sin(),
        if strip { 10.0 } else { 32.0 } * (t / 131.0 * tau).sin(),
    )
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
    // Hors du service de hyprdesk avant tout : son redémarrage ne doit plus
    // emporter l'écran de verrouillage (voir `scope`).
    if let Err(e) = scope::detach_from_service() {
        let msg = crate::tr!(
            "hyprdesk lock : verrou laissé dans le service de hyprdesk ({e})",
            e = e
        );
        eprintln!("{msg}");
        pam::journal(&msg);
    }
    // Ni le groupe de processus de hypridle : le démon l'arrête par groupe.
    if let Err(e) = scope::leave_process_group() {
        let msg = crate::tr!(
            "hyprdesk lock : verrou laissé dans le groupe de processus de hypridle ({e})",
            e = e
        );
        eprintln!("{msg}");
        pam::journal(&msg);
    }
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
        // Refusé par Hyprland (session déjà verrouillée par un écran disparu,
        // autre verrou) : sortir libère le verrou d'instance ; attendre le
        // gardait, et plus rien ne pouvait reverrouiller la session.
        Err(iced_sessionlock::Error::WaylandDispatchFailed(
            sessionlockev::SessionLockEventError::LockFinished,
        )) => {
            let msg = crate::tr!("hyprdesk lock : verrouillage refusé ou interrompu par Hyprland");
            eprintln!("{msg}");
            pam::journal(msg);
            ExitCode::FAILURE
        }
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

    fn encre(fg: [u8; 4], background: [u8; 4]) -> Ink {
        Ink::of(&LockState {
            fg,
            background,
            ..Default::default()
        })
    }

    /// Fond clair : le voile noir monte jusqu'à 4,5:1, pour le texte secondaire
    /// comme pour l'erreur, sur chaque pixel ; fond sombre : le plancher
    /// réglé suffit (HIG ACC-02).
    #[test]
    fn voile_releve_au_contraste_du_texte() {
        let ink = encre([0xf2, 0xf4, 0xfc, 0xff], [0x13, 0x17, 0x24, 0xff]);
        assert_eq!(ink.veil, Rgba(0, 0, 0, 0xff), "texte clair : voile noir");
        assert_eq!(
            ink.error, ERROR,
            "l'erreur du thème se lit déjà sur le noir"
        );
        let (black, white) = (Rgba(0, 0, 0, 0xff), Rgba(255, 255, 255, 0xff));
        let v = ink.veil_over(&[white]);
        assert!((0.7..0.9).contains(&v), "fond blanc : voile {v}");
        assert!(ink.readable_over(&[white], f64::from(v)));
        assert!(
            !ink.readable_over(&[white], f64::from(v) - 0.02),
            "le voile trouvé est le plus petit"
        );
        let over = theme::mix(white, ink.veil, f64::from(v));
        let (text, a) = ink.secondary;
        assert!(theme::contrast(theme::mix(over, text, f64::from(a)), over) >= TEXT_CONTRAST);
        assert!(
            theme::contrast(over, text) >= TEXT_CONTRAST,
            "texte principal"
        );
        // Un seul pixel clair impose son voile à tout l'écran.
        assert_eq!(ink.veil_over(&[black, white]), v);
        assert_eq!(ink.veil_over(&[black]), 0.0, "fond noir : rien à ajouter");
        let img = ::image::RgbaImage::from_fn(64, 64, |x, _| ::image::Rgba([(x * 4) as u8; 4]));
        let gradient = pixels(&img);
        let g = ink.veil_over(&gradient);
        assert!(g < v && ink.readable_over(&gradient, f64::from(g)));
    }

    /// Thème clair : le voile prend la couleur du fond du thème, et l'erreur du
    /// thème s'assombrit jusqu'à se lire dessus.
    #[test]
    fn theme_clair_voile_clair() {
        let bg = Rgba(0xf5, 0xf5, 0xf7, 0xff);
        let ink = encre([0x1f, 0x23, 0x30, 0xff], [bg.0, bg.1, bg.2, bg.3]);
        assert_eq!(ink.veil, bg, "texte sombre : voile de la couleur du fond");
        assert_ne!(ink.error, ERROR);
        assert!(theme::contrast(ink.error, bg) >= TEXT_CONTRAST);
        // Fond noir : le voile clair monte ; texte sombre et erreur s'y lisent.
        let black = [Rgba(0, 0, 0, 0xff)];
        let v = ink.veil_over(&black);
        assert!(v > 0.5, "fond noir sous texte sombre : voile {v}");
        assert!(ink.readable_over(&black, f64::from(v)));
        // Fond déjà clair : rien à ajouter.
        assert_eq!(ink.veil_over(&[bg]), 0.0);
        // Thème sombre sur un fond réglé clair : le voile reste noir, c'est le
        // texte clair qu'il sert.
        let sombre = encre([0xf2, 0xf4, 0xfc, 0xff], [bg.0, bg.1, bg.2, bg.3]);
        assert_eq!(sombre.veil, Rgba(0, 0, 0, 0xff));
    }

    /// Corps de texte à l'échelle réglée, jamais sous le minimum lisible
    /// (HIG ACC-06).
    #[test]
    fn corps_a_l_echelle() {
        assert_eq!(ts(13.0, 1.0), 13.0);
        assert_eq!(ts(13.0, 2.0), 26.0);
        assert_eq!(ts(120.0, 0.75), 90.0);
        assert_eq!(ts(6.0, 1.0), crate::ui::tokens::MIN_TEXT);
        // Échelle hors clous (profil ancien, JSON à la main) : bornée.
        assert_eq!(ts(13.0, 0.0), crate::ui::tokens::MIN_TEXT);
        assert_eq!(ts(13.0, 9.0), 26.0);
    }

    /// Seule la touche multimédia « Lecture et pause » agit, comme le bouton
    /// (HIG ACC-08).
    #[test]
    fn touche_multimedia_lecture_et_pause() {
        use iced::keyboard::Key;
        use iced::keyboard::key::Named;
        assert!(is_play_key(&Key::Named(Named::MediaPlayPause)));
        assert!(!is_play_key(&Key::Named(Named::MediaPause)));
        assert!(!is_play_key(&Key::Named(Named::Space)));
        assert!(!is_play_key(&Key::Character(" ".into())));
    }

    /// « Mouvement réduit » fige la dérive du contenu (HIG ACC-05).
    #[test]
    fn derive_figee_a_mouvement_reduit() {
        assert_eq!(drift(24.25, false, true), (0.0, 0.0));
        assert_eq!(drift(61.5, true, true), (0.0, 0.0));
        assert_eq!(drift(0.0, false, false), (0.0, 0.0));
        let (dx, dy) = drift(24.25, false, false);
        assert!(
            (dx - 48.0).abs() < 1e-3,
            "extrémité horizontale à 97 s / 4 : {dx}"
        );
        assert!(dy.abs() > 20.0, "{dy}");
        // En bandeau, la dérive verticale est plus courte.
        assert!((drift(32.75, true, false).1 - 10.0).abs() < 1e-3);
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
