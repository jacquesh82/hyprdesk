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
use iced::widget::{Space, button, column, container, image, pick_list, row, stack, text_input};
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
    /// Tab (Maj + Tab : `true`) : champ suivant ou précédent.
    FocusNext(bool),
    /// Veille, redémarrage, extinction (`systemctl suspend|reboot|poweroff`).
    Power(&'static str),
    PowerFailed(String),
    /// Trame suivante du fond vidéo (boucle ffmpeg).
    Frame(crate::video::Tiles),
}

/// Dernière connexion réussie (dossier d'état du greeter).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Last {
    user: String,
    session: String,
}

fn last_path() -> std::path::PathBuf {
    std::path::Path::new(crate::greeter_setup::STATE_DIR).join("dernier.json")
}

fn read_last() -> Last {
    std::fs::read_to_string(last_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
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
    /// Démo filmée (`HYPRDESK_GREETER_FILM`, pour les captures) : sans la
    /// mention « Démo », et la validation ferme l'écran comme une ouverture
    /// de session.
    film: bool,
    /// Fond : la copie posée par Réglages → Écran de connexion, sinon celui
    /// de hyprdesk (horizontal ou vertical selon l'écran). Vidéo déléguée à
    /// mpv : aucun fond dessiné (l'écran est transparent sur mpv). Vidéo
    /// décodée à la maison : la trame courante de la boucle ffmpeg.
    wallpaper: Option<image::Handle>,
    /// Fond vidéo sans mpv : trame courante en bandes (voir `video::Tiles`).
    video_frame: Option<crate::video::Tiles>,
    /// Fond vidéo en cours : porté par mpv (son process, l'écran est
    /// transparent au-dessus), ou décodé ici (fichier, trame, fps — un
    /// diviseur entier du rafraîchissement de l'écran, pour ne pas battre).
    video: Option<Video>,
    /// Emplacement du formulaire : 0 (haut-gauche) … 8 (bas-droite).
    anchor: u8,
    /// Logo de hyprdesk, en haut à gauche.
    logo: image::Handle,
}

/// Comment le fond vidéo est porté : par mpv en GPU (le rendu vsyncé de
/// mpv, l'écran du greeter étant transparent au-dessus), ou décodé ici par
/// ffmpeg trame par trame (cage, ou sans mpv).
enum Video {
    Mpv(#[allow(dead_code)] Player),
    Frames {
        path: std::path::PathBuf,
        w: u32,
        h: u32,
        fps: u32,
    },
}

impl Video {
    /// Porté par mpv : l'écran du greeter est transparent sur lui.
    fn playing(&self) -> bool {
        matches!(self, Video::Mpv(_))
    }
}

/// Le mpv lancé : quitté avec le greeter (session ouverte, démo fermée),
/// pour ne jamais laisser un fond jouer derrière la session. Le process est
/// gardé vivant par le champ (un Child abandonné rend la main au système).
struct Player(#[allow(dead_code)] std::process::Child);

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Fonds et logo livrés avec hyprdesk (dans le binaire : le greeter n'a
/// accès à aucun fichier de l'utilisateur).
pub const DEFAULT_WALLPAPER: &[u8] = include_bytes!("../assets/greeter/fond.jpg");
pub const DEFAULT_WALLPAPER_VERTICAL: &[u8] = include_bytes!("../assets/greeter/fond-vertical.jpg");
pub const LOGO: &[u8] = include_bytes!("../assets/greeter/logo.png");
/// Vidéo du fond livré (H.264 1280×720, boucle) : posée une fois dans un
/// dossier inscriptible, les lecteurs y lisent leurs trames. Le paquet
/// livre aussi le fichier (PACKAGED_VIDEO), lisible sans pose.
pub const DEFAULT_VIDEO: &[u8] = include_bytes!("../assets/greeter/fond.mp4");
/// Copie de la vidéo livrée par le paquet.
const PACKAGED_VIDEO: &str = "/usr/share/hyprdesk/greeter/fond.mp4";

/// Alignements d'un ancrage (0 = haut-gauche … 8 = bas-droite) : ceux du
/// formulaire, et la colonne des repères (logo, alimentation), à l'opposé
/// (le centre passe à gauche, pour ne jamais recouvrir le formulaire).
fn anchor_aligns(anchor: u8) -> (Alignment, Alignment, Alignment) {
    let h = |i: u8| match i % 3 {
        0 => Alignment::Start,
        1 => Alignment::Center,
        _ => Alignment::End,
    };
    let v = |i: u8| match i / 3 {
        0 => Alignment::Start,
        1 => Alignment::Center,
        _ => Alignment::End,
    };
    let aux = if anchor.is_multiple_of(3) {
        Alignment::End
    } else {
        Alignment::Start
    };
    (h(anchor), v(anchor), aux)
}

/// La vidéo de fond livrée : la copie posée dans un dossier inscriptible
/// (état du greeter, session), ou celle du paquet en lecture directe.
fn default_video_path() -> Option<std::path::PathBuf> {
    static POSED: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    POSED
        .get_or_init(|| {
            let dirs = [
                std::env::var_os("XDG_RUNTIME_DIR")
                    .map(|d| std::path::PathBuf::from(d).join("hyprdesk")),
                Some(std::path::PathBuf::from(crate::greeter_setup::STATE_DIR)),
                Some(std::env::temp_dir().join("hyprdesk")),
            ];
            for dir in dirs.into_iter().flatten() {
                let p = dir.join("fond.mp4");
                let same = std::fs::metadata(&p)
                    .map(|m| m.len() == DEFAULT_VIDEO.len() as u64)
                    .unwrap_or(false);
                if same {
                    return Some(p);
                }
                let tmp = dir.join("fond.nouveau");
                if std::fs::create_dir_all(&dir).is_err() {
                    continue;
                }
                if std::fs::write(&tmp, DEFAULT_VIDEO).is_ok() && std::fs::rename(&tmp, &p).is_ok()
                {
                    return Some(p);
                }
            }
            None
        })
        .clone()
        // Le paquet livre aussi la vidéo, lisible sans pose (système de
        // fichiers en lecture seule, utilisateur greeter sans HOME).
        .or_else(|| {
            let p = std::path::PathBuf::from(PACKAGED_VIDEO);
            p.is_file().then_some(p)
        })
}

fn rgba(c: crate::config::Rgba, a: f32) -> Color {
    Color::from_rgba8(c.0, c.1, c.2, (c.3 as f32 / 255.0) * a)
}

/// Grand côté des trames du fond vidéo : l'écran les agrandit, et un fond
/// animé ne souffre pas d'être décodé au-delà — chaque trame coûte un
/// repaint complet de la surface (le plein débit clignote).
const VIDEO_FRAME: u32 = 1280;

/// Rythme du fond vidéo décodé ici : le plus grand nombre de trames par
/// seconde qui **divise le rafraîchissement de l'écran** (une trame pendant
/// un nombre entier de rafraîchissements) et reste sous 12 — chaque trame
/// coûte un repaint complet de la surface, et un rythme qui ne tombe pas
/// pile (15 sur 144 Hz : 9,6) fait battre la vidéo. 12 par défaut, 10 sur
/// 90 Hz, 11 sur 165 Hz…
fn video_fps(refresh: f32) -> u32 {
    let refresh = refresh.round() as u32;
    if refresh <= 12 && refresh > 0 {
        return refresh;
    }
    // Un rythme trop lent (75 Hz ne donne que 5) saccade plus qu'il ne bat :
    // on garde 12 et on accepte le battement.
    (1..=12u32)
        .rev()
        .find(|f| refresh.is_multiple_of(*f))
        .filter(|f| *f >= 8)
        .unwrap_or(12)
}

/// L'écran est branché (Hyprland le connaît) ; sans réponse (pas d'IPC), faux :
/// le greeter retombe sur l'écran actif du compositeur.
fn screen_exists(name: &str) -> bool {
    hyprpanel_core::ipc::query_json("monitors")
        .ok()
        .and_then(|v| {
            v.as_array()
                .map(|a| a.iter().any(|m| m["name"].as_str() == Some(name)))
        })
        .unwrap_or(false)
}

/// Rectangle d'un écran dans la disposition : coin global, puis taille
/// logique (pixels divisés par l'échelle, côtés échangés si pivoté) — les
/// unités des dispatchs de fenêtre de Hyprland.
fn logical_rect(
    x: i64,
    y: i64,
    w: f64,
    h: f64,
    scale: f64,
    transform: i64,
) -> (i64, i64, i64, i64) {
    let s = if scale > 0.0 { scale } else { 1.0 };
    let (w, h) = ((w / s).round() as i64, (h / s).round() as i64);
    if transform % 2 == 1 {
        (x, y, h, w)
    } else {
        (x, y, w, h)
    }
}

/// L'écran `name` tel que Hyprland le dispose en ce moment.
fn live_rect(name: &str) -> Option<(i64, i64, i64, i64)> {
    let v = hyprpanel_core::ipc::query_json("monitors").ok()?;
    let m = v
        .as_array()?
        .iter()
        .find(|m| m["name"].as_str() == Some(name))?;
    Some(logical_rect(
        m["x"].as_i64()?,
        m["y"].as_i64()?,
        m["width"].as_f64()?,
        m["height"].as_f64()?,
        m["scale"].as_f64().unwrap_or(1.0),
        m["transform"].as_i64().unwrap_or(0),
    ))
}

/// Lancement de mpv derrière le greeter (l'écran du greeter devient
/// transparent sur lui). Un mpv d'un fond précédent qui traînerait s'en va.
fn spawn_player(path: &std::path::Path) -> Option<Video> {
    crate::video::stop_players();
    let a = crate::video::mpv_args(path);
    match std::process::Command::new(&a[0]).args(&a[1..]).spawn() {
        Ok(c) => Some(Video::Mpv(Player(c))),
        Err(e) => {
            tracing::warn!(
                "{}",
                crate::tr!("fond vidéo : mpv ne démarre pas — {e}", e = e)
            );
            None
        }
    }
}

/// mpv sur l'écran de la connexion (celui du greeter), à la taille de cet
/// écran : il s'ouvre où le curseur traîne (flotté dans un tuile occupé en
/// démo), ce n'est pas forcément là. Ciblé par son `address` — le focus est
/// retenu par la surface du greeter. Sa fenêtre se fait attendre : essais
/// jusqu'à deux secondes. Hors Hyprland (cage), rien à faire : mpv seul,
/// plein écran de toute façon.
fn position_player(monitor: Option<String>, rect: Option<(i64, i64, i64, i64)>) -> Task<GMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                // `dispatch` enveloppe lui-même la commande : la passer déjà
                // enveloppée en fait un `hl.dsp.global` vide, répondu « ok »
                // sans effet.
                let d = |t: &str| {
                    let _ = hyprpanel_core::ipc::dispatch(t);
                };
                // La fenêtre de mpv, retrouvée tant qu'elle n'est pas mappée —
                // et bien celle de hyprdesk (titre), pas un mpv traînant.
                let mut addr = None;
                for _ in 0..7 {
                    std::thread::sleep(Duration::from_millis(300));
                    addr = hyprpanel_core::ipc::query_json("clients")
                        .ok()
                        .and_then(|v| {
                            crate::hypr::parse_clients(&v)
                                .into_iter()
                                .find(|c| c.class == "mpv" && c.title == "hyprdesk-greeter-video")
                                .map(|c| c.address)
                        });
                    if addr.is_some() {
                        break;
                    }
                }
                let Some(addr) = addr else {
                    return;
                };
                // Laisser mpv finir son mapping initial : les dispatchs partis
                // trop tôt sont avalés sans effet.
                std::thread::sleep(Duration::from_millis(600));
                if let Some(m) = monitor.as_deref() {
                    d(&format!("movetomonitorsilent {m}"));
                    std::thread::sleep(Duration::from_millis(150));
                    // Sur le workspace affiché de l'écran (mpv s'ouvre sur le
                    // workspace du curseur : invisible s'il n'est pas montré).
                    if let Some(ws) = hyprpanel_core::ipc::query_json("monitors")
                        .ok()
                        .and_then(|v| {
                            v.as_array()
                                .and_then(|a| a.iter().find(|w| w["name"].as_str() == Some(m)))
                                .and_then(|w| w["activeWorkspace"]["name"].as_str())
                                .map(str::to_string)
                        })
                    {
                        let _ = crate::hypr::move_window(&addr, &ws);
                    }
                }
                let w = format!("address:{addr}");
                // L'écran tel que Hyprland le voit en ce moment (coin global,
                // taille logique) ; à défaut, celui du profil.
                let rect = monitor
                    .as_deref()
                    .and_then(live_rect)
                    .or(rect);
                // Placement en deux passes : la première floate la fenêtre,
                // la deuxième ne refait que resize et move — re-floater une
                // fenêtre déjà flottée la re-tuile (toggle côté compositeur).
                // Coordonnées globales et taille logique (échelle appliquée) :
                // le coin de l'écran dans la disposition, pas (0, 0).
                let mut floated = false;
                for _ in 0..2 {
                    if !floated {
                        d(&format!(
                            "hl.dsp.window.float({{ action = \"enable\", window = \"{w}\" }})"
                        ));
                        std::thread::sleep(Duration::from_millis(120));
                        floated = true;
                    }
                    let Some((x, y, width, height)) = rect else {
                        break;
                    };
                    d(&format!(
                        "hl.dsp.window.resize({{ x = {width}, y = {height}, relative = false, window = \"{w}\" }})"
                    ));
                    std::thread::sleep(Duration::from_millis(120));
                    d(&format!(
                        "hl.dsp.window.move({{ x = {x}, y = {y}, relative = false, window = \"{w}\" }})"
                    ));
                    std::thread::sleep(Duration::from_millis(400));
                    let ok = hyprpanel_core::ipc::query_json("clients")
                        .ok()
                        .and_then(|v| {
                            v.as_array().and_then(|a| {
                                a.iter()
                                    .find(|c| c["address"].as_str() == Some(addr.as_str()))
                                    .and_then(|c| {
                                        let size = c["size"].as_array()?;
                                        let dw = size.first()?.as_i64()? - width;
                                        let dh = size.get(1)?.as_i64()? - height;
                                        Some(dw.abs() <= 2 && dh.abs() <= 2)
                                    })
                            })
                        })
                        .unwrap_or(false);
                    if ok {
                        break;
                    }
                }
                // Plein écran : au-dessus des surfaces « top » (barre,
                // Réglages ouverts en démo), qu'une fenêtre flottante laisse
                // passer entre elle et le greeter.
                d(&format!(
                    "hl.dsp.window.fullscreen({{ mode = \"fullscreen\", action = \"set\", window = \"{w}\" }})"
                ));
            })
            .await
            .ok();
        },
        |_| GMsg::Tick,
    )
}

/// Boucle du fond vidéo : les trames du décodeur partagé, en messages.
fn greeter_video(
    key: &(std::path::PathBuf, u32, u32, u32),
) -> impl iced::futures::Stream<Item = GMsg> + use<> {
    let (path, w, h, fps) = key.clone();
    let recipe = crate::video::Recipe {
        path,
        w,
        h,
        fps: fps.max(1),
        blur: None,
        span: None,
        whole: false,
    };
    use iced::futures::StreamExt;
    crate::video::frames(recipe).map(|f| GMsg::Frame(f.tiles))
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
    pub fn new(window: bool) -> Self {
        let sessions = greeter::sessions();
        // Profil posé par Réglages → Écran de connexion (thème de
        // l'utilisateur), et dernière connexion : identifiant et session.
        let profile = crate::greeter_setup::Profile::installed().unwrap_or_default();
        let theme = profile.theme();
        // Fond : le vertical si l'écran de la connexion est pivoté (et qu'il
        // y en a un), sinon le fond ordinaire. Une vidéo joue en boucle, par
        // mpv en GPU derrière le greeter transparent quand une fenêtre
        // ordinaire peut s'ouvrir (pas sous cage), sinon par le décodeur
        // trame par trame de hyprdesk.
        let vertical = crate::greeter_setup::login_monitor(&profile).is_some_and(|m| m.vertical());
        let usable = |w: &String| !w.is_empty() && std::path::Path::new(w).is_file();
        let chosen = if vertical && usable(&profile.wallpaper_vertical) {
            Some(profile.wallpaper_vertical.clone())
        } else {
            Some(profile.wallpaper.clone()).filter(usable)
        };
        let monitor = crate::greeter_setup::login_monitor(&profile);
        let fps = video_fps(monitor.map(|m| m.refresh).unwrap_or(0.0));
        let target = match monitor {
            Some(m) if m.width > 0 && m.height > 0 => {
                crate::video::target_size(m.width, m.height, VIDEO_FRAME)
            }
            _ if vertical => (720, 1280),
            _ => (1280, 720),
        };
        let play = |p: std::path::PathBuf| -> Option<Video> {
            if !window && crate::video::has_player() {
                return spawn_player(&p);
            }
            crate::video::has_decoder().then_some(Video::Frames {
                path: p,
                w: target.0,
                h: target.1,
                fps,
            })
        };
        let video = chosen
            .as_deref()
            .filter(|p| crate::video::is_video(std::path::Path::new(p)))
            .and_then(|p| play(std::path::PathBuf::from(p)))
            // Fond livré : la vidéo (écran horizontal), sinon les images
            // livrées.
            .or_else(|| {
                (chosen.is_none() && !vertical)
                    .then(|| default_video_path().and_then(play))
                    .flatten()
            });
        let wallpaper = match &video {
            Some(_) => None, // mpv dessine, ou la première trame suivra.
            None => Some(match chosen {
                Some(path) => image::Handle::from_path(path),
                None if vertical => image::Handle::from_bytes(DEFAULT_WALLPAPER_VERTICAL),
                None => image::Handle::from_bytes(DEFAULT_WALLPAPER),
            }),
        };
        let last = read_last();
        let session = sessions
            .iter()
            .position(|s| s.id == last.session)
            .unwrap_or(0);
        Greeter {
            font: crate::fonts::DEFAULT,
            title_font: Font {
                weight: iced::font::Weight::Bold,
                ..crate::fonts::DEFAULT
            },
            theme,
            sessions,
            session,
            user: last.user,
            answer: String::new(),
            phase: Phase::User,
            started: false,
            messages: Vec::new(),
            error: None,
            stream: Arc::new(Mutex::new(None)),
            demo: false,
            film: false,
            wallpaper,
            video_frame: None,
            anchor: profile.anchor.unwrap_or(8),
            video,
            logo: image::Handle::from_bytes(LOGO),
        }
    }

    fn demo(window: bool) -> Self {
        let mut g = Greeter {
            demo: true,
            film: std::env::var_os("HYPRDESK_GREETER_FILM").is_some(),
            user: std::env::var("USER").unwrap_or_default(),
            ..Greeter::new(window)
        };
        // La démo montre la vidéo : celle imposée par HYPRDESK_GREETER_VIDEO,
        // celle du profil si elle en a une, sinon la vidéo livrée (au lieu de
        // l'image du profil).
        if std::env::var_os("HYPRDESK_GREETER_VIDEO").is_none()
            && g.video.is_none()
            && let Some(p) = default_video_path()
        {
            if !window && crate::video::has_player() {
                g.video = spawn_player(&p);
            } else if crate::video::has_decoder() {
                g.video = Some(Video::Frames {
                    path: p,
                    w: 1280,
                    h: 720,
                    fps: 12,
                });
            }
            g.wallpaper = None;
        }
        // Essais sans rien enregistrer : HYPRDESK_GREETER_VIDEO impose un
        // fond vidéo, HYPRDESK_GREETER_ANCHOR (0..8) la place du formulaire.
        if let Some(p) = std::env::var_os("HYPRDESK_GREETER_VIDEO") {
            let p = std::path::PathBuf::from(p);
            if p.is_file() && crate::video::is_video(&p) {
                // La vidéo d'essai remplace celle du profil (et le mpv éventuel
                // du profil s'en va avec).
                if let Some(Video::Mpv(player)) = g.video.take() {
                    drop(player);
                }
                if !window && crate::video::has_player() {
                    g.video = spawn_player(&p);
                } else if crate::video::has_decoder() {
                    g.video = Some(Video::Frames {
                        path: p,
                        w: 1280,
                        h: 720,
                        fps: 12,
                    });
                }
                g.wallpaper = None;
            }
        }
        if let Ok(a) = std::env::var("HYPRDESK_GREETER_ANCHOR")
            && let Ok(v) = a.trim().parse::<u8>()
            && v <= 8
        {
            g.anchor = v;
        }
        g
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
        let mut g = Greeter::new(true);
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
        // Aperçu de la documentation : le fond livré, pas celui du poste.
        g.video = None;
        g.wallpaper = Some(image::Handle::from_bytes(DEFAULT_WALLPAPER));
        g.anchor = 8;
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
                    if self.film {
                        // Comme une session qui s'ouvre : l'attente, puis l'écran part.
                        self.phase = Phase::Waiting;
                        return Task::perform(
                            async {
                                tokio::time::sleep(std::time::Duration::from_millis(900)).await
                            },
                            |_| GMsg::Quit,
                        );
                    }
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
                    // Retenus pour la prochaine fois (sans effet si le dossier
                    // d'état n'existe pas).
                    let last = Last {
                        user: self.user.trim().to_string(),
                        session: s.as_ref().map(|s| s.id.clone()).unwrap_or_default(),
                    };
                    if let Ok(body) = serde_json::to_string(&last) {
                        let _ = std::fs::write(last_path(), body);
                    }
                    let (cmd, env) = match &s {
                        Some(s) => (
                            greeter::session_cmd(&greeter::login_shell(self.user.trim()), &s.exec),
                            greeter::session_env(s),
                        ),
                        None => (
                            vec![greeter::login_shell(self.user.trim()), "-l".into()],
                            Vec::new(),
                        ),
                    };
                    send(self.stream.clone(), Request::StartSession { cmd, env })
                }
                Step::Done => iced::exit(),
                Step::Fail(m) => self.fail(m),
            },
            GMsg::Quit if self.demo => iced::exit(),
            GMsg::Power(verb) => {
                if self.demo {
                    self.messages = vec![
                        match verb {
                            "suspend" => {
                                crate::i18n::tr("Démo : l'ordinateur se mettrait en veille.")
                            }
                            "reboot" => crate::i18n::tr("Démo : l'ordinateur redémarrerait."),
                            _ => crate::i18n::tr("Démo : l'ordinateur s'éteindrait."),
                        }
                        .to_string(),
                    ];
                    return Task::none();
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            std::process::Command::new("systemctl")
                                .arg(verb)
                                .output()
                                .map_err(|e| e.to_string())
                                .and_then(|o| {
                                    if o.status.success() {
                                        Ok(())
                                    } else {
                                        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
                                    }
                                })
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| match r {
                        Ok(()) => GMsg::Tick,
                        Err(e) => GMsg::PowerFailed(e),
                    },
                )
            }
            GMsg::PowerFailed(e) => {
                self.error = Some(e);
                Task::none()
            }
            GMsg::Frame(h) => {
                // Le lecteur rythme déjà les trames (un diviseur entier du
                // rafraîchissement de l'écran) : chaque trame reçue s'applique.
                if self.video.is_some() {
                    self.video_frame = Some(h);
                }
                Task::none()
            }
            GMsg::FocusNext(back) => {
                if back {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                }
            }
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
        if self.demo && !self.film {
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
        let power = |label: &'static str, verb: &'static str| {
            button(
                text(crate::i18n::tr(label))
                    .font(self.font)
                    .size(13)
                    .color(dim),
            )
            .padding([6, 14])
            .style(move |_, status| button::Style {
                background: Some(
                    Color {
                        a: if matches!(status, button::Status::Hovered) {
                            0.18
                        } else {
                            0.0
                        },
                        ..accent
                    }
                    .into(),
                ),
                border: Border::default().rounded(12),
                text_color: fg,
                ..Default::default()
            })
            .on_press(GMsg::Power(verb))
        };
        // Zone de connexion en bas à droite, sur un voile du thème (lisible
        // sur n'importe quelle image ou vidéo) ; alimentation en bas à gauche.
        let transparent = self.video.as_ref().is_some_and(|v| v.playing());
        let veil = rgba(
            th.bg,
            if self.wallpaper.is_some() || self.video.is_some() {
                0.72
            } else {
                0.0
            },
        );
        let card = container(col).padding(28).style(move |_| container::Style {
            background: Some(veil.into()),
            border: Border::default().rounded(28),
            ..Default::default()
        });
        let power_row = container(
            row![
                power("Veille", "suspend"),
                power("Redémarrer", "reboot"),
                power("Éteindre", "poweroff"),
            ]
            .spacing(8),
        )
        .padding(6)
        .style(move |_| container::Style {
            background: Some(veil.into()),
            border: Border::default().rounded(16),
            ..Default::default()
        });
        // Formulaire à son ancrage ; logo et alimentation en face (colonne
        // opposée, à l'autre bout de l'écran).
        let (h_card, v_card, h_aux) = anchor_aligns(self.anchor);
        let overlay = container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(48)
            .align_x(h_card)
            .align_y(v_card);
        let power_overlay = container(power_row)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(48)
            .align_x(h_aux)
            .align_y(Alignment::End);
        let back: Element<'_, GMsg> = match (&self.video_frame, &self.wallpaper) {
            (Some(t), _) if self.video.is_some() => t.view(1.0),
            (_, Some(h)) => image(h.clone())
                .content_fit(iced::ContentFit::Cover)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        // Logo en haut, à la largeur de la zone de connexion.
        let logo = container(image(self.logo.clone()).width(300))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(48)
            .align_x(h_aux)
            .align_y(Alignment::Start);
        container(stack![back, logo, power_overlay, overlay])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                // mpv porte la vidéo : l'écran du greeter est transparent
                // sur lui (sinon : l'aplat du thème, ou la trame).
                background: Some(if transparent { Color::TRANSPARENT } else { bg }.into()),
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
        let tab = iced::event::listen_with(|e, _, _| match e {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
                modifiers,
                ..
            }) => Some(GMsg::FocusNext(modifiers.shift())),
            _ => None,
        });
        let mut subs = vec![tick, tab];
        // Fond vidéo décodé ici : la boucle ffmpeg alimente l'écran trame
        // par trame (mpv, lui, est un processus à part).
        if let Some(Video::Frames { path, w, h, fps }) = &self.video {
            subs.push(Subscription::run_with(
                (path.clone(), *w, *h, *fps),
                |k: &(std::path::PathBuf, u32, u32, u32)| greeter_video(k),
            ));
        }
        if self.demo {
            subs.push(iced::event::listen_with(|e, _, _| match e {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                }) => Some(GMsg::Quit),
                _ => None,
            }));
        }
        Subscription::batch(subs)
    }
}

/// Vue d'exemple pour `hyprdesk preview` (scène « connexion »).
pub fn preview_view() -> Element<'static, GMsg> {
    static G: std::sync::OnceLock<Greeter> = std::sync::OnceLock::new();
    G.get_or_init(Greeter::preview).view()
}

/// Point d'entrée de `hyprdesk greeter` (`demo` : sans greetd ; `window` :
/// fenêtre ordinaire plein écran au lieu d'une surface layer-shell).
///
/// Le greeter tourne sous le compositeur que greetd lance. cage (kiosque)
/// n'offre pas layer-shell : il y faut une fenêtre ordinaire (xdg-shell),
/// que cage met en plein écran. Sous un compositeur à layer-shell (Hyprland,
/// démo dans la session), la surface plein écran au-dessus de tout. Si
/// layer-shell manque malgré tout, repli sur la fenêtre.
pub fn run(demo: bool, window: bool) -> std::process::ExitCode {
    let r = if window {
        run_window(demo)
    } else {
        run_layer(demo).or_else(|e| {
            eprintln!(
                "{}",
                crate::tr!(
                    "hyprdesk greeter : layer-shell indisponible ({e}), fenêtre ordinaire",
                    e = e
                )
            );
            run_window(demo)
        })
    };
    match r {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", crate::tr!("hyprdesk greeter : {e}", e = e));
            std::process::ExitCode::FAILURE
        }
    }
}

fn boot(demo: bool, window: bool) -> (Greeter, Task<GMsg>) {
    let g = if demo {
        Greeter::demo(window)
    } else {
        Greeter::new(window)
    };
    // mpv lancé : à sa place, sur l'écran de la connexion.
    let task = if matches!(g.video, Some(Video::Mpv(_))) {
        let profile = crate::greeter_setup::Profile::installed();
        let mon = profile
            .as_ref()
            .and_then(crate::greeter_setup::login_monitor);
        position_player(
            mon.map(|m| m.name.clone())
                .or_else(crate::greeter_setup::login_screen),
            mon.map(|m| {
                logical_rect(
                    m.x as i64,
                    m.y as i64,
                    m.width as f64,
                    m.height as f64,
                    m.scale as f64,
                    m.transform as i64,
                )
            }),
        )
    } else {
        Task::none()
    };
    (
        g,
        Task::batch([iced::widget::operation::focus(USER_ID), task]),
    )
}

/// Surface layer-shell plein écran, clavier exclusif.
fn run_layer(demo: bool) -> Result<(), String> {
    // Pas de presse-papiers : rien à coller dans le champ du mot de passe.
    iced_layershell::disable_clipboard();
    // Sur l'écran configuré de la connexion s'il est branché (mpv y est
    // envoyé aussi) ; sinon l'écran actif du compositeur.
    let start_mode = match crate::greeter_setup::login_screen() {
        Some(m) if screen_exists(&m) => StartMode::TargetScreen(m),
        _ => StartMode::Active,
    };
    let mut app = iced_layershell::application(
        move || boot(demo, false),
        "hyprdesk-greeter",
        Greeter::update,
        Greeter::view,
    )
    // Le clear de la surface suit l'état : transparent quand mpv porte la
    // vidéo (l'écran du greeter se pose sur lui), l'aplat du thème sinon.
    .style(|g: &Greeter, _| iced::theme::Style {
        background_color: if g.video.as_ref().is_some_and(|v| v.playing()) {
            Color::TRANSPARENT
        } else {
            rgba(g.theme.bg, 1.0)
        },
        text_color: rgba(g.theme.fg, 1.0),
    })
    .subscription(Greeter::subscription)
    .default_font(crate::fonts::DEFAULT)
    .layer_settings(LayerShellSettings {
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Overlay,
        exclusive_zone: -1,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        start_mode,
        ..Default::default()
    });
    for face in crate::fonts::bundled() {
        app = app.font(face);
    }
    app.run().map_err(|e| e.to_string())
}

/// Fenêtre ordinaire plein écran, sans décorations (cage, ou tout
/// compositeur sans layer-shell).
fn run_window(demo: bool) -> Result<(), String> {
    iced::application(move || boot(demo, true), Greeter::update, Greeter::view)
        .subscription(Greeter::subscription)
        .settings(iced::Settings {
            fonts: crate::fonts::bundled(),
            default_font: crate::fonts::DEFAULT,
            ..Default::default()
        })
        .title(crate::i18n::tr("Connexion"))
        .window(iced::window::Settings {
            fullscreen: true,
            decorations: false,
            // Fermer la fenêtre ne doit pas laisser greetd sans écran : en
            // vraie connexion, seul l'ouverture de session la referme.
            exit_on_close_request: demo,
            ..Default::default()
        })
        .run()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La vidéo du fond livré, posée une fois dans l'état du greeter.
    #[test]
    fn video_livree_posee() {
        let Some(p) = default_video_path() else {
            panic!("aucun dossier inscriptible pour la vidéo livrée");
        };
        assert!(p.is_file(), "la vidéo livrée est posée : {}", p.display());
        let m = std::fs::metadata(&p).unwrap();
        assert_eq!(m.len(), DEFAULT_VIDEO.len() as u64, "copie complète");
        assert!(crate::video::is_video(&p));
    }

    /// Ancrages : le formulaire à sa place, les repères en face.
    #[test]
    fn ancrages() {
        use Alignment::{Center, End, Start};
        // Par défaut : bas-droite, repères à gauche (comme toujours).
        assert_eq!(anchor_aligns(8), (End, End, Start));
        // Haut-gauche : repères à droite ; centre : repères à gauche.
        assert_eq!(anchor_aligns(0), (Start, Start, End));
        assert_eq!(anchor_aligns(4), (Center, Center, Start));
        assert_eq!(anchor_aligns(6), (Start, End, End));
        // Hors bornes (impossible au profil, écarté à la lecture) : sans effet.
        assert_eq!(anchor_aligns(3), (Start, Center, End));
    }

    /// Placement de mpv : coin global de l'écran, taille logique.
    #[test]
    fn rectangle_logique() {
        assert_eq!(
            logical_rect(2560, 0, 1920.0, 1080.0, 1.2, 0),
            (2560, 0, 1600, 900),
            "échelle 1,2 : 1600 × 900, au coin de l'écran (pas 0, 0)"
        );
        assert_eq!(
            logical_rect(0, 0, 1920.0, 1080.0, 1.0, 1),
            (0, 0, 1080, 1920),
            "pivoté"
        );
        assert_eq!(
            logical_rect(0, 0, 1920.0, 1080.0, 0.0, 0),
            (0, 0, 1920, 1080),
            "échelle inconnue"
        );
    }

    /// Rythme du fond vidéo : un diviseur entier du rafraîchissement de
    /// l'écran (sinon la vidéo bat), 12 au plus.
    #[test]
    fn rythme_video() {
        assert_eq!(video_fps(144.0), 12, "144 / 12 = 12 rafraîchissements");
        assert_eq!(video_fps(144.04), 12, "arrondi");
        assert_eq!(video_fps(60.0), 12, "60 / 12 = 5");
        assert_eq!(video_fps(165.0), 11, "165 / 11 = 15");
        assert_eq!(video_fps(120.0), 12, "120 / 12 = 10");
        assert_eq!(video_fps(90.0), 10, "90 / 10 = 9");
        assert_eq!(video_fps(0.0), 12, "inconnu : 12");
        assert_eq!(video_fps(24.0), 12, "écran lentement rythmé : sa cadence");
        assert_eq!(video_fps(75.0), 12, "5 fps saccade : 12, battement accepté");
    }

    /// Démo : identifiant, mot de passe, puis ce qui serait lancé ; rien
    /// ne part vers greetd et la réponse est effacée.
    #[test]
    fn demo_sans_greetd() {
        let mut g = Greeter::demo(true);
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
        let mut real = Greeter::new(true);
        let _ = real.update(GMsg::Quit);
        assert_eq!(real.phase, Phase::User);
    }
}
