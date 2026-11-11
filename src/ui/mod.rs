//! Interface : un démon iced_layershell, UNE surface par écran.
//!
//! Au repos, chaque surface vit sur le calque `background` et dessine le fond
//! d'écran du workspace affiché (elle remplace hyprpaper). Appelée, elle reste
//! sous les fenêtres, met de côté celles du workspace, prend le clavier et
//! affiche icônes et widgets en fondu. Masquée, elle restitue les fenêtres.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Local;
use iced::futures::{SinkExt, Stream};
pub(crate) mod bar;
mod chat;
mod design;
mod icons;
mod osd;
mod panel;
pub(crate) mod plugins;
mod polkit;
mod popover;
pub mod preview;
mod toasts;
mod tour;
mod widget_view;

use crate::i18n::text;
use iced::widget::{Space, Stack, column, container, image, mouse_area, pin, svg};
use iced::window::Id;
use iced::{
    Alignment, Background, Border, Color, Element, Event, Font, Length, Padding, Point, Size,
    Subscription, Task, keyboard, mouse,
};
use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::config::lua::LuaHandle;
use crate::config::resolve::{Desktop, resolve};
use crate::config::{
    self, Config, Fit, HookEvent, IconDef, Mode, Rgba, RunningApps, ShowOn, Transition, WallSource,
    WidgetKind,
};
use crate::hypr::{self, Client, Monitor};
use crate::launch::AppInfo;
use crate::layout::{self, Cell, Layout};
use crate::settings::Settings as DeskSettings;
use crate::{control, launch, widgets};
use panel::{Panel, PanelMsg};

const NAMESPACE: &str = "hyprdesk";
const DRAG_THRESHOLD: f32 = 6.0;
/// Icônes de l'interface, dans l'ordre de `App::ui_imgs` : commandes média, puis réglages.
const UI_ICONS: [&str; 10] = [
    "media-skip-backward",
    "media-playback-pause",
    "media-playback-start",
    "media-skip-forward",
    "preferences-system",
    "user-desktop",
    "audio-volume-muted",
    "audio-volume-low",
    "audio-volume-medium",
    "audio-volume-high",
];
const ICON_SETTINGS: usize = 4;
/// Décalage vertical du contenu au début d'une transition « glissement ».
const SLIDE_PX: f32 = 48.0;

/// Clé du cache d'images décodées : fichier, taille cible, ajustement.
/// Clé du cache d'images décodées : fichier, taille cible, ajustement, flou (mode verre).
/// Image, taille de l'écran, ajustement, flou (mode verre), et place de
/// l'écran dans la disposition si la scène est étalée sur tous les écrans.
type WallKey = (PathBuf, u32, u32, Fit, bool, Option<crate::span::Placement>);

/// Mode verre : l'image est décodée à 1/BLUR_DOWNSCALE de sa taille puis
/// légèrement floutée ; l'agrandissement par le GPU achève le flou, pour un
/// coût négligeable.
const BLUR_DOWNSCALE: u32 = 8;

/// Glisser-déposer de fichiers sur une surface.
#[derive(Debug, Clone)]
pub enum DropEvent {
    Hover,
    Left,
    File(PathBuf),
}

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Msg {
    Control(control::Command),
    Hypr(hypr::Update),
    Apps(Vec<AppInfo>),
    ClassIcons(u64, Vec<(String, Option<PathBuf>)>),
    FocusWindow(String),
    OpenPanel,
    Panel(PanelMsg),
    /// Tiroir de l'assistant (LYRA, protocole serve).
    Chat(chat::ChatMsg),
    SettingsLoaded(u64, DeskSettings),
    /// Barre : aller au workspace (écran, nom).
    BarWorkspace(String, String),
    /// Barre : molette sur les workspaces (écran, ±1).
    BarScroll(String, i32),
    VolumeMute,
    VolumeStep(i32),
    VolumeSet(f32),
    VolumeValue(Option<(f32, bool)>),
    /// Micro coupé ou rétabli.
    MicMute,
    MicSet(f32),
    /// Choisit la sortie ou l'entrée par défaut (id PipeWire).
    AudioDefault(u32),
    AudioValue(bar::Audio),
    /// Ouvre l'assistant IA (LYRA), sans changer le mode du bureau.
    OpenAssistant,
    /// Lanceur : survol d'une entrée (écran, clé, contenu de la bulle), ou sortie.
    LauncherHover(Option<(String, String, plugins::launcher::PopupContent, usize)>),
    /// Position relative dans le dock (écran, rang continu), ou sortie du dock.
    LauncherPointer(String, Option<f32>),
    LauncherAnchor(String, String, iced::Rectangle),
    LauncherShown(String),
    LauncherPopupOpen(u64),
    LauncherPopupMeasured(Id, f32),
    LauncherLaunchExpired(String, Instant),
    LauncherResolved(String, Instant, String),
    LauncherDispatched(String, Instant, bool),
    /// Fin du délai de fermeture de la bulle (jeton du survol).
    LauncherPopupClose(u64),
    LauncherPopupHover(bool),
    /// Fenêtre choisie dans la bulle d'une application regroupée.
    LauncherPick(String),
    /// Lanceur : lance une application épinglée (id `.desktop`).
    LauncherLaunch(String),
    /// Lanceur : application regroupée (écran, classe), fenêtre suivante.
    LauncherApp(String, String),
    /// Écrans détectés (désactivés compris).
    Screens(Vec<crate::screens::Screen>),
    /// Seconde écoulée pendant qu'un changement d'écrans attend confirmation.
    ScreensTick,
    /// Ouvre (ou referme) un menu sous un module de la barre : (barre, clé du module, contenu).
    OpenPopover(Id, String, popover::Request),
    PopoverReady(u64, Id, String, popover::Kind),
    PopoverPositioned(u64, Id, String, popover::Kind, Option<(f32, f32)>),
    ClosePopover,
    PopoverMeasured(Id, f32),
    Popover(popover::PMsg),
    /// Appelle une fonction Lua d'un widget (on_click, on_scroll) puis le relève.
    LuaCall(String, usize, Option<i64>),
    /// Relève un widget tout de suite.
    Repoll(String),
    IconResolved(u64, String, Option<PathBuf>),
    TrayItems(Vec<crate::tray::TrayItem>),
    /// (barre, bus, chemin, secondaire)
    TrayActivate(Id, String, String, bool),
    /// Raccourcis installés : conflits (raccourci, action déjà liée) et raccourcis universels.
    KeysInstalled(Result<hypr::KeyReport, String>),
    ConfigChanged,
    ConfigLoaded(Result<Arc<Config>, String>),
    /// Génération, icônes du bureau, puis icônes des commandes média.
    IconsResolved(u64, Vec<launch::Resolved>, Vec<Option<PathBuf>>),
    WallLoaded(WallKey, Result<DecodedWall, String>),
    Rotate,
    /// L'affichage à l'écran (OSD) s'efface-t-il ?
    OsdTick,
    /// Agent polkit : demande de mot de passe, saisie, réponse.
    Polkit(polkit::PMsg),
    /// Bulles de notifications.
    Toasts(toasts::TMsg),
    Frame,
    Poll,
    WidgetValue(u64, String, widgets::Value),
    Resized(Id, Size),
    Closed(Id),
    Cursor(Id, Point),
    /// Glisser-déposer de fichiers sur une surface (survol, sortie, dépôt).
    Drop(Id, DropEvent),
    Released,
    Key(Id, keyboard::Key),
    IconPress(Id, usize),
    /// Clic sur un widget, réglages ouverts : sélection et début de glissé.
    WidgetPress(Id, String),
    IconHover(Option<(Id, usize)>),
    Hover(String, bool),
    Launch(usize),
    Media(String, &'static str),
    Noop,
}

#[derive(Debug, Clone, Copy)]
struct Fade {
    from: f32,
    to: f32,
    start: Instant,
    dur: Duration,
}

impl Fade {
    fn value(&self, now: Instant) -> f32 {
        if self.dur.is_zero() {
            return self.to;
        }
        let p = (now.saturating_duration_since(self.start).as_secs_f32() / self.dur.as_secs_f32())
            .clamp(0.0, 1.0);
        // Ease-out cubique.
        let e = 1.0 - (1.0 - p).powi(3);
        self.from + (self.to - self.from) * e
    }
    fn done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.start) >= self.dur
    }
}

/// Ce qu'un écran affiche comme fond : une image décodée ou un aplat.
#[derive(Debug, Clone, PartialEq)]
enum Wall {
    Image(WallKey),
    Color(Rgba),
}

#[derive(Debug)]
struct Shown {
    current: Wall,
    prev: Option<Wall>,
    since: Instant,
}

#[derive(Debug)]
struct Rot {
    files: Vec<PathBuf>,
    idx: usize,
    next: Instant,
}

#[derive(Debug, Clone, Copy)]
enum IconImg {
    None,
    Raster(usize),
    Vector(usize),
}

#[derive(Debug)]
struct Drag {
    win: Id,
    icon: usize,
    start: Point,
    active: bool,
}

/// Widget glissé à la souris, réglages ouverts.
#[derive(Debug)]
struct WidgetDrag {
    win: Id,
    id: String,
    start: Point,
    from: config::Placement,
    moved: bool,
}

pub struct App {
    lua: LuaHandle,
    config_path: PathBuf,
    cfg: Arc<Config>,
    config_error: Option<String>,
    font: Font,
    monitors: Vec<Monitor>,
    /// Surface → nom d'écran.
    surfaces: HashMap<Id, String>,
    sizes: HashMap<Id, Size>,
    visible: bool,
    /// Surfaces dont le contenu de bureau est actif.
    raised: HashSet<Id>,
    fade: Option<Fade>,
    panel_fade: Option<Fade>,
    hover_fades: HashMap<String, Fade>,
    walls: HashMap<WallKey, image::Handle>,
    loading: HashSet<WallKey>,
    shown: HashMap<String, Shown>,
    rot: HashMap<usize, Rot>,
    /// Icônes résolues, alignées sur `cfg.icons`.
    icons: Vec<launch::Resolved>,
    icon_imgs: Vec<IconImg>,
    /// Voir `UI_ICONS`.
    ui_imgs: Vec<IconImg>,
    rasters: Vec<image::Handle>,
    vectors: Vec<svg::Handle>,
    icons_gen: u64,
    values: HashMap<String, widgets::Value>,
    last_poll: HashMap<String, Instant>,
    pending_widgets: HashSet<String>,
    widget_generation: u64,
    layout: Layout,
    selected: Option<(Id, usize)>,
    hovered: Option<(Id, usize)>,
    cursor: HashMap<Id, Point>,
    drag: Option<Drag>,
    wdrag: Option<WidgetDrag>,
    /// Réglages de l'écran ⚙ (relus depuis settings.lua seul).
    settings: DeskSettings,
    settings_revision: Arc<std::sync::atomic::AtomicU64>,
    panel: Option<Panel>,
    /// Tiroir de l'assistant ; `chat_session` relance le moteur à chaque ouverture.
    chat: Option<chat::ChatPanel>,
    chat_session: u64,
    /// Onglet choisi dans chaque paquet de réglages (retenu d'une ouverture à l'autre).
    subtabs: HashMap<&'static str, &'static str>,
    /// Surface survolée par un glisser-déposer de fichiers, et fichiers déjà
    /// posés pendant ce dépôt (pour les ranger côte à côte).
    drop_hover: Option<Id>,
    drop_count: u16,
    /// Applications installées (liste des raccourcis, icônes des fenêtres).
    apps: Arc<Vec<AppInfo>>,
    clients: Vec<Client>,
    /// Icône par classe de fenêtre ; invalidées avec `icons_gen`.
    class_imgs: HashMap<String, IconImg>,
    class_pending: HashSet<String>,
    theme: iced::Theme,
    key_conflicts: Vec<(String, String)>,
    universal_binds: Vec<String>,
    /// La submap « hyprdesk » est active (raccourcis de Hyprland suspendus).
    in_submap: bool,
    /// Surfaces passées au-dessus des fenêtres (mode verre seulement).
    /// Fenêtres mises de côté pendant que le bureau est affiché.
    windows: crate::windows::WindowManager,
    /// Fenêtres ouvertes pendant la session bureau : le bureau les contient.
    session_windows: HashSet<String>,
    /// Parmi elles, celles OUVERTES sur le bureau (pas celles qu'on y a
    /// ramenées) : flottantes, elles suivent le bureau quand il est masqué
    /// (`hypr::DESK`) et reviennent avec lui.
    desk_windows: HashSet<String>,
    /// Affichage à l'écran des touches de volume et de luminosité.
    osd: Option<osd::Osd>,
    /// Carte d'authentification polkit ouverte.
    polkit: Option<polkit::Dialog>,
    /// Bulles de notifications (si hyprdesk tient le nom du démon).
    toasts: toasts::Toasts,
    /// Surfaces de la barre native.
    bar_surfaces: HashMap<Id, bar::BarSurface>,
    workspaces: Vec<hypr::Workspace>,
    active_window: Option<(String, String)>,
    volume: Option<(f32, bool)>,
    /// Micro et périphériques audio (menu Son).
    audio: bar::Audio,
    /// Bureau affiché SOUS les fenêtres, laissées en place : icônes, widgets
    /// et dock visibles entre elles (sans effet quand le bureau est masqué).
    with_windows: bool,
    /// Marges posées dans Hyprland pour dégager les contrôles du bureau, par
    /// écran (haut, droite, bas, gauche) : vide hors du bureau avec les fenêtres.
    window_gaps: Vec<(String, [i32; 4])>,
    /// Règle Hyprland posée pour LYRA (rectangle du tiroir), si elle l'est.
    lyra_rule: Option<(i32, i32, i32, i32)>,
    /// Cartes d'identification des écrans (surface, « numéro\nnom\nmodèle »).
    identify: Vec<(Id, String)>,
    identify_gen: u64,
    /// Surfaces du lanceur, une par écran où le bureau est affiché.
    launcher_surfaces: HashMap<Id, plugins::launcher::LauncherSurface>,
    launcher_runtime: plugins::launcher::Runtime,
    /// Bulle du lanceur (nom d'une entrée, ou fenêtres d'une application).
    launcher_popup: Option<plugins::launcher::Popup>,
    launcher_popup_token: u64,
    launcher_popup_hover: bool,
    /// Lanceur regroupé par application : rang de la prochaine fenêtre, par classe.
    launcher_cycle: HashMap<String, usize>,
    /// Écrans tels que Hyprland les voit, pour l'onglet Écrans.
    screens: Vec<crate::screens::Screen>,
    /// Changement d'écrans appliqué mais pas confirmé : disposition d'avant
    /// et échéance du retour automatique.
    screens_pending: Option<(Vec<crate::screens::Screen>, Instant)>,
    popover: Option<popover::Popover>,
    menu_request: u64,
    /// Icônes du thème par nom (symboliques teintées), résolues à la demande.
    named_icons: HashMap<String, IconImg>,
    named_pending: HashSet<String>,
    tray: Vec<crate::tray::TrayItem>,
    /// Images fournies par les applications de la zone de notification : clé → (empreinte, image).
    tray_imgs: HashMap<String, (u64, image::Handle)>,
    /// Police de l'interface (barre, menus) : celle de la config, sinon Mindlog.
    ui_family: &'static str,
}

pub fn run(config_path: PathBuf) -> Result<(), iced_layershell::Error> {
    daemon(
        move || App::boot(config_path.clone()),
        || NAMESPACE.to_string(),
        App::update,
        App::view,
    )
    .subscription(App::subscription)
    .style(|_, _| iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: Color::WHITE,
    })
    .theme(|app: &App, _| Some(app.theme.clone()))
    .settings(Settings {
        fonts: crate::fonts::bundled(),
        default_font: crate::fonts::DEFAULT,
        layer_settings: LayerShellSettings {
            start_mode: StartMode::Background,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

// ---------------------------------------------------------------------------
// Petits utilitaires

fn color(c: Rgba, alpha: f32) -> Color {
    Color::from_rgba8(c.0, c.1, c.2, (c.3 as f32 / 255.0) * alpha.clamp(0.0, 1.0))
}

fn index_of<T>(slice: &[T], item: &T) -> usize {
    (item as *const T as usize - slice.as_ptr() as usize) / std::mem::size_of::<T>().max(1)
}

fn intern_font(name: &str) -> &'static str {
    static FONTS: std::sync::LazyLock<std::sync::Mutex<HashMap<String, &'static str>>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));
    let mut fonts = FONTS.lock().unwrap_or_else(|e| e.into_inner());
    fonts
        .entry(name.to_string())
        .or_insert_with(|| Box::leak(name.to_string().into_boxed_str()))
}

fn chrono_locale() -> chrono::Locale {
    crate::i18n::chrono_locale()
}

/// Date formatée sans paniquer sur un format invalide.
fn format_now(fmt: &str) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    if write!(s, "{}", Local::now().format_localized(fmt, chrono_locale())).is_err() {
        return format!("format invalide : {fmt}");
    }
    s
}

fn list_images(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                        matches!(e.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp")
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Décode et met à l'échelle une image de fond (hors thread UI).
#[derive(Debug, Clone)]
pub struct DecodedWall {
    handle: image::Handle,
    backdrop: plugins::launcher::Backdrop,
}

fn decode_wall(key: &WallKey) -> Result<DecodedWall, String> {
    let (path, w, h, fit, blur, span) = key;
    let img = ::image::open(path).map_err(|e| format!("{} : {e}", path.display()))?;
    // Scène étalée : la part de cet écran, à sa taille (l'ajustement ne joue plus).
    let (img, fit) = match span {
        Some(p) => (crate::span::cut(&img, *w, *h, *p), &Fit::Fill),
        None => (img, fit),
    };
    let img = if *blur {
        img.thumbnail(
            (img.width() / BLUR_DOWNSCALE).max(1),
            (img.height() / BLUR_DOWNSCALE).max(1),
        )
    } else {
        img
    };
    let (w, h) = if *blur {
        (
            &((*w / BLUR_DOWNSCALE).max(1)),
            &((*h / BLUR_DOWNSCALE).max(1)),
        )
    } else {
        (w, h)
    };
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let (tw, th) = (*w as f32, *h as f32);
    use ::image::imageops::FilterType;
    let img = match fit {
        Fit::Fill => img.resize_exact(*w, *h, FilterType::Triangle),
        Fit::Center => img,
        Fit::Cover | Fit::Contain => {
            let s = if *fit == Fit::Cover {
                (tw / iw).max(th / ih)
            } else {
                (tw / iw).min(th / ih)
            };
            let (nw, nh) = (
                (iw * s).round().max(1.0) as u32,
                (ih * s).round().max(1.0) as u32,
            );
            img.resize_exact(nw, nh, FilterType::Triangle)
        }
    };
    let img = if *blur { img.fast_blur(2.5) } else { img };
    let rgba = img.to_rgba8();
    let backdrop = plugins::launcher::Backdrop::from_rgba(&rgba);
    Ok(DecodedWall {
        handle: image::Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw()),
        backdrop,
    })
}

fn watch_stream(path: &std::path::Path) -> impl Stream<Item = ()> + use<> {
    let path = path.to_path_buf();
    iced::stream::channel(
        4,
        async move |mut out: iced::futures::channel::mpsc::Sender<()>| {
            use notify::{RecursiveMode, Watcher};
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
            let dir = path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let modules = dir.join("hyprdesk");
            let (target, mods) = (path.clone(), modules.clone());
            let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(ev) = res
                    && !matches!(ev.kind, notify::EventKind::Access(_))
                    && ev
                        .paths
                        .iter()
                        .any(|p| p == &target || p.starts_with(&mods))
                {
                    let _ = tx.send(());
                }
            });
            let mut watcher = match watcher {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("{}", crate::tr!("surveillance de la config : {e}", e = e));
                    return;
                }
            };
            if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
                tracing::error!(
                    "{}",
                    crate::tr!("surveillance de {p0} : {e}", p0 = dir.display(), e = e)
                );
            }
            if modules.is_dir() {
                let _ = watcher.watch(&modules, RecursiveMode::Recursive);
            }
            while rx.recv().await.is_some() {
                // Les éditeurs écrivent en plusieurs fois : on regroupe.
                tokio::time::sleep(Duration::from_millis(200)).await;
                while rx.try_recv().is_ok() {}
                let _ = out.send(()).await;
            }
            drop(watcher);
        },
    )
}

fn listen(event: Event, _status: iced::event::Status, id: Id) -> Option<Msg> {
    match event {
        Event::Mouse(mouse::Event::CursorMoved { position }) => Some(Msg::Cursor(id, position)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => Some(Msg::Released),
        Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => Some(Msg::Key(id, key)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------

impl App {
    fn boot(config_path: PathBuf) -> (Self, Task<Msg>) {
        Self::boot_with(config_path, false)
    }

    fn boot_with(config_path: PathBuf, preview: bool) -> (Self, Task<Msg>) {
        let app = App {
            lua: LuaHandle::spawn(),
            config_path,
            cfg: Arc::new(Config::default()),
            config_error: None,
            font: crate::fonts::DEFAULT,
            monitors: Vec::new(),
            surfaces: HashMap::new(),
            sizes: HashMap::new(),
            visible: false,
            raised: HashSet::new(),
            fade: None,
            panel_fade: None,
            hover_fades: HashMap::new(),
            walls: HashMap::new(),
            loading: HashSet::new(),
            shown: HashMap::new(),
            rot: HashMap::new(),
            icons: Vec::new(),
            icon_imgs: Vec::new(),
            ui_imgs: Vec::new(),
            rasters: Vec::new(),
            vectors: Vec::new(),
            icons_gen: 0,
            values: HashMap::new(),
            last_poll: HashMap::new(),
            pending_widgets: HashSet::new(),
            widget_generation: 0,
            layout: Layout::load(),
            selected: None,
            hovered: None,
            cursor: HashMap::new(),
            drag: None,
            wdrag: None,
            settings: DeskSettings::default(),
            settings_revision: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            panel: None,
            chat: None,
            chat_session: 0,
            subtabs: HashMap::new(),
            drop_hover: None,
            drop_count: 0,
            apps: Arc::new(Vec::new()),
            clients: Vec::new(),
            class_imgs: HashMap::new(),
            class_pending: HashSet::new(),
            theme: make_theme(&Config::default()),
            key_conflicts: Vec::new(),
            universal_binds: Vec::new(),
            in_submap: false,
            windows: if preview {
                crate::windows::WindowManager::disabled()
            } else {
                crate::windows::WindowManager::spawn()
            },
            session_windows: HashSet::new(),
            desk_windows: HashSet::new(),
            osd: None,
            polkit: None,
            toasts: Default::default(),
            bar_surfaces: HashMap::new(),
            workspaces: Vec::new(),
            active_window: None,
            volume: None,
            audio: bar::Audio::default(),
            with_windows: false,
            window_gaps: Vec::new(),
            launcher_cycle: HashMap::new(),
            lyra_rule: None,
            identify: Vec::new(),
            identify_gen: 0,
            launcher_surfaces: HashMap::new(),
            launcher_runtime: plugins::launcher::Runtime::default(),
            launcher_popup: None,
            launcher_popup_token: 0,
            launcher_popup_hover: false,
            screens: Vec::new(),
            screens_pending: None,
            popover: None,
            menu_request: 0,
            named_icons: HashMap::new(),
            named_pending: HashSet::new(),
            tray: Vec::new(),
            tray_imgs: HashMap::new(),
            ui_family: crate::fonts::FAMILY,
        };
        let apps = Task::perform(
            async {
                tokio::task::spawn_blocking(launch::installed_apps)
                    .await
                    .unwrap_or_default()
            },
            Msg::Apps,
        );
        let t = Task::batch([app.load_config(), apps]);
        (app, t)
    }

    fn load_config(&self) -> Task<Msg> {
        let (lua, path) = (self.lua.clone(), self.config_path.clone());
        let p2 = path.clone();
        let settings_revision = self
            .settings_revision
            .load(std::sync::atomic::Ordering::Acquire);
        let settings = Task::perform(
            async move {
                tokio::task::spawn_blocking(move || config::lua::load_settings_only(&p2)).await
            },
            move |r| match r {
                Ok(Ok(c)) => Msg::SettingsLoaded(settings_revision, DeskSettings::from_config(&c)),
                Ok(Err(e)) => {
                    tracing::warn!("{}", crate::tr!("settings.lua : {e}", e = e));
                    Msg::Noop
                }
                Err(_) => Msg::Noop,
            },
        );
        Task::batch([
            Task::perform(
                async move { lua.load(path).await.map(Arc::new) },
                Msg::ConfigLoaded,
            ),
            settings,
        ])
    }

    /// Remplace la config effective en ne refaisant que ce qui a changé : un
    /// réglage modifié en direct, ou un rechargement identique, ne fait ni
    /// clignoter les icônes ni repartir le diaporama.
    fn install_config(&mut self, cfg: Arc<Config>) -> Task<Msg> {
        let old = std::mem::replace(&mut self.cfg, cfg);
        crate::i18n::apply(&self.cfg.general.language);
        let n_owned = self.cfg.general.clone();
        let (o, n) = (&old.general, &n_owned);
        if o.font != n.font || self.icons_gen == 0 {
            self.font = match &n.font {
                // Les noms de police doivent vivre aussi longtemps que le programme.
                Some(f) => Font::with_name(intern_font(f)),
                None => crate::fonts::DEFAULT,
            };
            self.ui_family = match &n.font {
                Some(f) => intern_font(f),
                None => crate::fonts::FAMILY,
            };
        }
        if o.theme != n.theme {
            self.theme = make_theme(&self.cfg);
        }
        if old.wallpapers != self.cfg.wallpapers {
            self.rot.clear();
        }
        let mut tasks = Vec::new();
        let widgets_changed = old.widgets.len() != self.cfg.widgets.len()
            || old
                .widgets
                .iter()
                .zip(&self.cfg.widgets)
                .any(|(a, b)| !a.same_content(b));
        if widgets_changed {
            self.values.clear();
            self.last_poll.clear();
            if self.visible {
                tasks.push(Task::done(Msg::Poll));
            }
        }
        // La taille d'icône ne demande pas de nouvelle recherche : l'image est mise à l'échelle.
        let icons_changed = old.icons != self.cfg.icons
            || o.icon_theme != n.icon_theme
            || o.terminal != n.terminal
            || self.icons_gen == 0;
        if icons_changed {
            self.icons.clear();
            self.icon_imgs.clear();
            self.selected = None;
            self.icons_gen += 1;
            let (gen_, cfg) = (self.icons_gen, self.cfg.clone());
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        let g = &cfg.general;
                        let icons = cfg
                            .icons
                            .iter()
                            .map(|d| {
                                launch::resolve(
                                    d,
                                    g.icon_theme.as_deref(),
                                    g.grid.icon_size,
                                    &g.terminal,
                                )
                            })
                            .collect::<Vec<_>>();
                        let media = UI_ICONS
                            .iter()
                            .map(|n| launch::lookup_icon(n, g.icon_theme.as_deref(), 24))
                            .collect::<Vec<_>>();
                        (icons, media)
                    })
                    .await
                    .unwrap_or_default()
                },
                move |(icons, media)| Msg::IconsResolved(gen_, icons, media),
            ));
        } else if o.running_apps != n.running_apps || o.launcher.pinned != n.launcher.pinned {
            tasks.push(self.resolve_class_icons());
        }
        if o.show_on != n.show_on || o.primary != n.primary {
            tasks.push(self.sync_raised());
        }
        // Premier chargement (catalogue vide avant), raccourcis ou flou modifiés.
        if o.keys != n.keys
            || o.theme.blurred_namespaces() != n.theme.blurred_namespaces()
            || old.catalog.is_empty()
        {
            tasks.push(self.install_keys());
        }
        if o.workspaces != n.workspaces
            || o.workspace_monitors != n.workspace_monitors
            || o.primary != n.primary
            || old.assigns != self.cfg.assigns
            || o.disabled_plugins != n.disabled_plugins
            || old.catalog.is_empty()
        {
            tasks.push(self.install_rules());
        }
        if old.bar != self.cfg.bar
            || o.primary != n.primary
            || o.disabled_plugins != n.disabled_plugins
        {
            tasks.push(self.sync_bars());
            self.last_poll.retain(|k, _| !k.starts_with("bar:"));
            tasks.push(Task::done(Msg::Poll));
        }
        let idle_changed = o.idle != n.idle;
        if old.components != self.cfg.components || idle_changed || old.catalog.is_empty() {
            // Composants de session : lancés, arrêtés ou relancés selon la config ;
            // hypridle s'y ajoute quand hyprdesk gère la veille (`crate::idle`).
            let mut defs = self.cfg.components.clone();
            if let Err(e) = crate::idle::write(&n.idle) {
                tracing::warn!("{}", crate::tr!("configuration de hypridle : {e}", e = e));
            }
            if let Some(d) = crate::idle::component(&n.idle)
                && !defs.iter().any(|c| c.name == d.name)
            {
                defs.push(d);
            }
            // Délais changés, composant identique : relire le fichier.
            let reload_idle = idle_changed && o.idle.manage && n.idle.manage;
            tasks.push(Task::perform(
                async move {
                    crate::session::sync(&defs).await;
                    if reload_idle {
                        crate::session::restart(crate::idle::COMPONENT).await;
                    }
                },
                |_| Msg::Noop,
            ));
        }
        if o.suspend_binds != n.suspend_binds {
            tasks.push(self.sync_submap());
        }
        // LYRA en tiroir : règle Hyprland sur sa fenêtre.
        tasks.push(self.sync_lyra_drawer());
        // Thème appliqué aux autres applications (kitty…), à chaque changement.
        if plugins::apptheme::effective(&old) != plugins::apptheme::effective(&self.cfg)
            || (old.general.theme != self.cfg.general.theme
                && !plugins::apptheme::effective(&self.cfg).is_empty())
        {
            tasks.push(self.sync_app_themes(false));
        }
        // Layouts de Hyprland réglés dans hyprdesk (paquet « Layouts »).
        if plugins::layouts::effective(&old) != plugins::layouts::effective(&self.cfg) {
            tasks.push(self.sync_tiling());
        }
        // Animations et effets de Hyprland réglés dans hyprdesk (paquet « Animations »).
        if plugins::animations::effective(&old) != plugins::animations::effective(&self.cfg) {
            tasks.push(self.sync_fx());
        }
        // État de l'écran de verrouillage (valeurs résolues : le verrou n'exécute pas de Lua).
        tasks.push(self.sync_lock_state());
        // Contexte de l'assistant IA (LYRA), tenu à jour hors du fil de l'interface.
        let (cfg, path) = (self.cfg.clone(), self.config_path.clone());
        tasks.push(Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || {
                    crate::assistant::write_context(&cfg, &path)
                })
                .await;
            },
            |_| Msg::Noop,
        ));
        // Icônes, widgets ou grille changés : la place des contrôles aussi.
        tasks.push(self.sync_window_gaps());
        tasks.push(self.refresh_walls());
        Task::batch(tasks)
    }

    /// (Ré)installe les raccourcis de hyprdesk dans Hyprland.
    fn install_keys(&self) -> Task<Msg> {
        let keys = self.cfg.general.keys.clone();
        let blurred = self.cfg.general.theme.blurred_namespaces();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    hypr::install_keys(&keys, &blurred).map_err(|e| e.to_string())
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            Msg::KeysInstalled,
        )
    }

    /// (Ré)installe les workspaces permanents et l'affectation des applications.
    fn install_rules(&self) -> Task<Msg> {
        // Paquet « Workspaces » coupé : aucune règle (les anciennes sont retirées).
        let (n, rules) = if self.cfg.plugin_on("workspaces") {
            (self.cfg.general.workspaces, self.cfg.assigns.clone())
        } else {
            (None, Vec::new())
        };
        let placement = if self.cfg.plugin_on("workspaces") {
            self.workspace_placement()
        } else {
            Vec::new()
        };
        Task::perform(
            async move {
                match tokio::task::spawn_blocking(move || {
                    hypr::install_rules(n, &placement, &rules)
                })
                .await
                {
                    Ok(Err(e)) => {
                        tracing::warn!("{}", crate::tr!("règles de workspace : {e}", e = e))
                    }
                    Err(e) => tracing::warn!("{}", crate::tr!("règles de workspace : {e}", e = e)),
                    Ok(Ok(())) => {}
                }
            },
            |_| Msg::Noop,
        )
    }

    /// Écran de chaque workspace, `primary` résolu ; le premier de chaque
    /// écran est son workspace par défaut.
    pub(super) fn workspace_placement(&self) -> Vec<hypr::Placement> {
        let primary = self.primary_monitor();
        let mut out = Vec::new();
        for (mon, list) in &self.cfg.general.workspace_monitors {
            let mon = if mon == "primary" {
                match &primary {
                    Some(p) => p.clone(),
                    None => continue,
                }
            } else {
                mon.clone()
            };
            for (i, ws) in list.iter().enumerate() {
                out.push((ws.clone(), mon.clone(), i == 0));
            }
        }
        out
    }

    /// Submap « hyprdesk » active si et seulement si le bureau est affiché
    /// et que la suspension des raccourcis est demandée.
    fn sync_submap(&mut self) -> Task<Msg> {
        // Dès qu'une fenêtre est ouverte depuis le bureau, ses raccourcis
        // habituels doivent pouvoir agir sur elle.
        let want = self.visible
            && !self.with_windows
            && self.cfg.general.suspend_binds
            && self.session_windows.is_empty();
        if want == self.in_submap {
            return Task::none();
        }
        self.in_submap = want;
        Self::window_result(self.windows.submap(want))
    }

    /// Écran principal effectif : celui réglé s'il est branché, sinon celui
    /// placé en 0×0, sinon le premier.
    pub(super) fn primary_monitor(&self) -> Option<String> {
        let p = self.cfg.general.primary.as_deref();
        self.monitors
            .iter()
            .find(|m| Some(m.name.as_str()) == p)
            .or_else(|| self.monitors.iter().find(|m| m.x == 0.0 && m.y == 0.0))
            .or_else(|| self.monitors.first())
            .map(|m| m.name.clone())
    }

    /// Écrit l'état de l'écran de verrouillage (valeurs résolues, part de
    /// chaque écran pour une scène étalée). À chaque config et changement d'écrans.
    fn sync_lock_state(&self) -> Task<Msg> {
        let primary = self.primary_monitor();
        let ws = self
            .monitors
            .iter()
            .find(|m| Some(&m.name) == primary.as_ref())
            .map(|m| m.workspace.clone())
            .unwrap_or_else(|| "1".into());
        let mut lock =
            crate::lock::state::from_config(&self.cfg, primary.as_deref().unwrap_or(""), &ws);
        // Fond propre de chaque écran (son workspace affiché), pour le verrouillage.
        let own: Vec<Option<PathBuf>> = self
            .monitors
            .iter()
            .map(|m| {
                let d = config::resolve::resolve(&self.cfg, &m.name, &m.workspace);
                match d.wallpaper.map(|w| &w.source) {
                    Some(WallSource::Path(p)) => Some(p.clone()),
                    Some(WallSource::Rotate { dir, .. }) => crate::lock::state::first_image(dir),
                    _ => None,
                }
            })
            .collect();
        // Scène étalée au verrouillage : la part de chaque écran, dans l'ordre de Hyprland.
        let screens: Vec<crate::span::Screen> = self
            .monitors
            .iter()
            .map(|m| crate::span::Screen {
                name: m.name.clone(),
                rect: crate::span::Rect {
                    x: m.x,
                    y: m.y,
                    w: m.width as f32,
                    h: m.height as f32,
                },
                scale: m.scale,
            })
            .collect();
        Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || {
                    crate::lock::state::prepare_span(&mut lock, &screens, &own);
                    crate::lock::state::write(&lock)
                })
                .await;
            },
            |_| Msg::Noop,
        )
    }

    /// Les fenêtres en mosaïque laissent la place du lanceur (launcher-only)
    /// et du tiroir des réglages ; sinon, les marges de Hyprland reviennent.
    fn sync_window_gaps(&mut self) -> Task<Msg> {
        // launcher-only : le dock en bas ; réglages ouverts, quel que soit le
        // mode : le tiroir à droite.
        let drawer_on = self.panel.as_ref().map(|p| p.win).or_else(|| {
            self.chat
                .as_ref()
                .filter(|c| c.surface.is_some())
                .map(|c| c.win)
        });
        let mut want: Vec<(String, [i32; 4])> = self
            .surfaces
            .iter()
            .filter_map(|(id, mon)| {
                // Le lanceur réserve lui-même sa place (zone exclusive de sa surface).
                let mut m = [0; 4];
                if drawer_on == Some(*id) {
                    m[1] = m[1].max(self.panel_width().round() as i32);
                }
                (m != [0; 4]).then(|| (mon.clone(), m))
            })
            .collect();
        want.sort();
        let launchers = self.sync_launchers();
        if want == self.window_gaps {
            return launchers;
        }
        self.window_gaps = want.clone();
        Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || hypr::set_desktop_gaps(&want)).await;
            },
            |_| Msg::Noop,
        )
    }

    /// Mode du bureau, tel que l'utilisateur le nomme.
    pub(super) fn mode(&self) -> DesktopMode {
        match (self.visible, self.with_windows) {
            (false, _) => DesktopMode::NoDesktop,
            (true, true) => DesktopMode::LauncherOnly,
            (true, false) => DesktopMode::FullDesktop,
        }
    }

    /// Le bureau affiché apparaît-il sur cet écran (`show_on`) ?
    fn shows_on(&self, monitor: &str) -> bool {
        match self.cfg.general.show_on {
            ShowOn::All => true,
            ShowOn::Focused => self.monitors.iter().any(|m| m.focused && m.name == monitor),
            ShowOn::Primary => self.primary_monitor().as_deref() == Some(monitor),
        }
    }

    /// Bureau affiché : aligne les surfaces actives sur `show_on` (tous, actif, principal).
    fn sync_raised(&mut self) -> Task<Msg> {
        if !self.visible {
            return Task::none();
        }
        let focused = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .map(|m| m.name.clone());
        let mut tasks = Vec::new();
        for (id, mon) in self.surfaces.clone() {
            let want = self.shows_on(&mon);
            let has = self.raised.contains(&id);
            if want && !has {
                tasks.extend(self.activate_surface(id, Some(&mon) == focused.as_ref()));
            } else if want && has {
                let kb = if Some(&mon) != focused.as_ref() {
                    KeyboardInteractivity::None
                } else if self.session_windows.is_empty() && !self.with_windows {
                    KeyboardInteractivity::Exclusive
                } else {
                    KeyboardInteractivity::OnDemand
                };
                tasks.push(Task::done(Msg::KeyboardInteractivityChange {
                    id,
                    keyboard_interactivity: kb,
                }));
            } else if !want && has {
                self.raised.remove(&id);
                tasks.push(Task::done(Msg::KeyboardInteractivityChange {
                    id,
                    keyboard_interactivity: KeyboardInteractivity::None,
                }));
                tasks.extend(self.lower_surface(id));
                tasks.push(self.restore_stash(Some(&mon)));
            }
        }
        tasks.push(self.sync_window_gaps());
        Task::batch(tasks)
    }

    /// Active le bureau sur une surface. Le bureau est la RACINE des fenêtres :
    /// il reste toujours dessous. Les fenêtres du workspace sont mises de côté
    /// et celles qu'on ouvre depuis le bureau s'ouvrent dessus. Le mode verre
    /// n'est qu'un effet sur le fond (fond d'écran flouté et teinté).
    fn activate_surface(&mut self, id: Id, has_focus: bool) -> Vec<Task<Msg>> {
        let Some(m) = self.monitor(id).cloned() else {
            return Vec::new();
        };
        self.raised.insert(id);
        let mut tasks = vec![self.stash_workspace(&m)];
        // Avec les fenêtres, le bureau ne prend le clavier que si on clique dessus.
        let kb = if has_focus && self.with_windows {
            KeyboardInteractivity::OnDemand
        } else if has_focus {
            KeyboardInteractivity::Exclusive
        } else {
            KeyboardInteractivity::None
        };
        tasks.push(Task::done(Msg::KeyboardInteractivityChange {
            id,
            keyboard_interactivity: kb,
        }));
        self.fire(HookEvent::Show, "show", &m);
        tasks
    }

    /// Redescend une surface au repos (fond d'écran seul, plein écran).
    /// La surface du bureau reste plein écran : seul son contenu suit la zone
    /// réservée par les barres (pas de redimensionnement, pas de saut du fond).
    fn lower_surface(&mut self, _id: Id) -> Vec<Task<Msg>> {
        Vec::new()
    }

    /// Zone réservée par les barres sur l'écran d'une surface (gauche, haut, droite, bas).
    fn reserved(&self, id: Id) -> (f32, f32, f32, f32) {
        self.monitor(id).map(|m| m.reserved).unwrap_or_default()
    }

    /// Met de côté les fenêtres du workspace affiché sur cet écran (sauf
    /// celles ouvertes depuis le bureau). L'acteur traite les demandes dans l'ordre,
    /// sans bloquer l'interface et sans intercaler une restitution au milieu des déplacements.
    fn stash_workspace(&mut self, m: &Monitor) -> Task<Msg> {
        // Bureau sous les fenêtres, workspace inconnu ou déjà celui de mise de
        // côté : rien à mettre de côté.
        if self.with_windows || m.workspace.is_empty() || m.workspace == hypr::STASH {
            return Task::none();
        }
        let picked = self
            .clients
            .iter()
            .filter(|c| c.workspace == m.workspace && !self.session_windows.contains(&c.address))
            .map(|c| hypr::Stashed {
                address: c.address.clone(),
                workspace: c.workspace.clone(),
                monitor: m.name.clone(),
            })
            .collect();
        Self::window_result(self.windows.stash(picked))
    }

    fn restore_stash(&mut self, monitor: Option<&str>) -> Task<Msg> {
        Self::window_result(self.windows.restore(monitor))
    }

    fn window_result(reply: tokio::sync::oneshot::Receiver<Result<(), String>>) -> Task<Msg> {
        Task::perform(
            async move {
                reply.await.unwrap_or_else(|_| {
                    Err(crate::i18n::tr("gestionnaire de fenêtres arrêté").into())
                })
            },
            |r| {
                if let Err(e) = r {
                    tracing::error!("{}", crate::tr!("fenêtres : {e}", e = e));
                }
                Msg::Noop
            },
        )
    }

    /// Rejoue l'apparition du bureau (aperçu d'une transition).
    fn replay_fade(&mut self) {
        if self.visible {
            self.fade = Some(Fade {
                from: 0.0,
                to: 1.0,
                start: Instant::now(),
                dur: self.fade_duration(),
            });
        }
    }

    /// Rejoue un fondu enchaîné sur un écran, depuis l'aplat de fond (aperçu).
    fn replay_crossfade(&mut self, monitor: &str) {
        let bgc = self.cfg.general.background;
        if let Some(s) = self.shown.get_mut(monitor) {
            s.prev = Some(Wall::Color(bgc));
            s.since = Instant::now();
        }
    }

    /// `hyprdesk menu <type>` : ouvre le menu du premier module de ce type sur la
    /// barre de l'écran actif (accès clavier, comme la barre de menus macOS).
    fn open_menu_by_type(&mut self, kind: &str) -> Task<Msg> {
        let focused = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .map(|m| m.name.clone());
        let Some(bar_id) = self
            .bar_surfaces
            .iter()
            .find(|(_, s)| Some(&s.monitor) == focused.as_ref())
            .or_else(|| self.bar_surfaces.iter().next())
            .map(|(id, _)| *id)
        else {
            return Task::none();
        };
        let items: Vec<config::BarItem> = self
            .cfg
            .bar
            .iter()
            .flat_map(|b| b.zones.iter().flatten().cloned())
            .collect();
        for it in items {
            let req = match &it {
                config::BarItem::Volume if kind == "volume" => {
                    Some(("volume".to_string(), popover::Request::Volume))
                }
                config::BarItem::Tray if kind == "tray" => self.tray.iter().find_map(|t| {
                    t.menu.clone().map(|m| {
                        (
                            format!("tray:{}", t.key),
                            popover::Request::Tray {
                                bus: t.bus.clone(),
                                menu_path: m,
                            },
                        )
                    })
                }),
                config::BarItem::Widget(id) => self
                    .cfg
                    .widgets
                    .iter()
                    .find(|w| &w.id == id && w.type_id == kind)
                    .and_then(|w| match &w.kind {
                        config::WidgetKind::Clock { .. } | config::WidgetKind::Calendar => {
                            Some((id.clone(), popover::Request::Calendar))
                        }
                        config::WidgetKind::Lua { menu: Some(f), .. } => Some((
                            id.clone(),
                            popover::Request::LuaMenu {
                                widget: id.clone(),
                                func: *f,
                            },
                        )),
                        _ => None,
                    }),
                _ => None,
            };
            if let Some((key, req)) = req {
                return Task::done(Msg::OpenPopover(bar_id, key, req));
            }
        }
        Task::none()
    }

    /// Résout les icônes des classes de fenêtres pas encore connues.
    fn resolve_class_icons(&mut self) -> Task<Msg> {
        let off = self.cfg.general.running_apps == RunningApps::Off;
        let pinned = &self.cfg.general.launcher.pinned;
        if off && pinned.is_empty() {
            return Task::none();
        }
        // Fenêtres ouvertes, et applications épinglées au lanceur (même fermées).
        let missing: Vec<String> = self
            .clients
            .iter()
            .filter(|_| !off)
            .map(|c| c.class.clone())
            .chain(pinned.iter().cloned())
            .filter(|c| !self.class_imgs.contains_key(c) && !self.class_pending.contains(c))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if missing.is_empty() {
            return Task::none();
        }
        self.class_pending.extend(missing.iter().cloned());
        let (apps, theme, gen_) = (
            self.apps.clone(),
            self.cfg.general.icon_theme.clone(),
            self.icons_gen,
        );
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    missing
                        .into_iter()
                        .map(|c| {
                            let p = launch::icon_for_class(&c, &apps, theme.as_deref(), 48);
                            (c, p)
                        })
                        .collect::<Vec<_>>()
                })
                .await
                .unwrap_or_default()
            },
            move |l| Msg::ClassIcons(gen_, l),
        )
    }

    fn monitor(&self, id: Id) -> Option<&Monitor> {
        let name = self.surfaces.get(&id)?;
        self.monitors.iter().find(|m| &m.name == name)
    }

    fn desktop_of(&self, id: Id) -> Option<(&Monitor, Desktop<'_>)> {
        let m = self.monitor(id)?;
        Some((m, resolve(&self.cfg, &m.name, &m.workspace)))
    }

    /// Taille de la zone de contenu du bureau : la surface moins la zone réservée.
    fn size_of(&self, id: Id) -> Size {
        let full = self
            .sizes
            .get(&id)
            .copied()
            .or_else(|| {
                self.monitor(id)
                    .map(|m| Size::new(m.width as f32, m.height as f32))
            })
            .unwrap_or(Size::new(1920.0, 1080.0));
        let (l, t, r, b) = self.reserved(id);
        Size::new(
            (full.width - l - r).max(1.0),
            (full.height - t - b).max(1.0),
        )
    }

    fn fade_value(&self) -> f32 {
        match self.fade {
            Some(f) => f.value(Instant::now()),
            None if self.visible => 1.0,
            None => 0.0,
        }
    }

    fn animating(&self) -> bool {
        self.launcher_animating() || self.fade.is_some()
            || self.shown.values().any(|s| s.prev.is_some())
            // Jusqu'à ce que Frame solde le fondu (fermeture comprise).
            || self.panel_fade.is_some()
            || self.hover_fades.values().any(|f| !f.done(Instant::now()))
            || self
                .popover
                .as_ref()
                .is_some_and(|p| p.started.elapsed() < self.motion_duration())
    }

    fn motion_duration(&self) -> Duration {
        if self.cfg.general.transition == Transition::None {
            Duration::ZERO
        } else {
            Duration::from_millis(self.cfg.general.fade_ms.min(220))
        }
    }

    fn hover_value(&self, key: &str) -> f32 {
        self.hover_fades
            .get(key)
            .map(|f| f.value(Instant::now()))
            .unwrap_or(0.0)
    }

    fn close_panel(&mut self) -> Task<Msg> {
        let from = self
            .panel_fade
            .map(|f| f.value(Instant::now()))
            .unwrap_or(1.0);
        self.panel_fade = Some(Fade {
            from,
            to: 0.0,
            start: Instant::now(),
            dur: self.motion_duration(),
        });
        Task::done(Msg::Frame)
    }

    fn hooks(&self, ev: HookEvent) -> Vec<usize> {
        self.cfg
            .hooks
            .iter()
            .filter(|(e, _)| *e == ev)
            .map(|(_, f)| *f)
            .collect()
    }

    fn fire(&self, ev: HookEvent, name: &str, monitor: &Monitor) {
        self.lua
            .hook(self.hooks(ev), name, &monitor.name, &monitor.workspace);
    }

    fn publish_status(&self) {
        let screens: Vec<String> = self
            .monitors
            .iter()
            .map(|m| format!("{}:{}", m.name, m.workspace))
            .collect();
        control::set_status(crate::tr!(
            "{mode} (pid {pid}) — écrans {screens} — config {state} ({path})",
            mode = self.mode().as_str(),
            pid = std::process::id(),
            screens = screens.join(" "),
            state = match &self.config_error {
                None => "ok".to_string(),
                Some(e) => crate::tr!("en erreur : {e}", e = e),
            },
            path = self.config_path.display()
        ));
    }

    // -- Affichage / masquage ------------------------------------------------

    fn show(&mut self) -> Task<Msg> {
        if self.visible && self.with_windows {
            // Bureau déjà affiché avec les fenêtres : on les met de côté.
            self.with_windows = false;
            let mut tasks: Vec<Task<Msg>> = self
                .surfaces
                .iter()
                .filter(|(id, _)| self.raised.contains(id))
                .filter_map(|(id, _)| self.monitor(*id).cloned())
                .collect::<Vec<_>>()
                .iter()
                .map(|m| self.stash_workspace(m))
                .collect();
            tasks.push(self.sync_raised());
            tasks.push(self.sync_submap());
            self.publish_status();
            return Task::batch(tasks);
        }
        if self.visible && self.fade.is_none_or(|f| f.to >= 1.0) {
            return Task::none();
        }
        self.visible = true;
        let focused = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .map(|m| m.name.clone());
        let targets: Vec<(Id, bool)> = self
            .surfaces
            .iter()
            .filter(|(_, mon)| self.shows_on(mon))
            .map(|(id, mon)| (*id, Some(mon) == focused.as_ref()))
            .collect();
        let from = self.fade_value();
        self.fade = Some(Fade {
            from,
            to: 1.0,
            start: Instant::now(),
            dur: self.fade_duration(),
        });
        self.selected = None;
        let mut tasks = Vec::new();
        for (id, has_focus) in targets {
            if !self.raised.contains(&id) {
                tasks.extend(self.activate_surface(id, has_focus));
            }
        }
        // Les fenêtres du bureau, parties avec lui, reviennent (après la mise
        // de côté des autres : l'acteur traite les demandes dans l'ordre).
        tasks.extend(self.bring_back_desk_windows());
        self.last_poll.clear();
        tasks.push(Task::done(Msg::Poll));
        tasks.push(self.refresh_walls());
        // Les raccourcis de Hyprland ne doivent pas agir sur les fenêtres cachées.
        tasks.push(self.sync_submap());
        // Bureau avec les fenêtres : elles laissent la place des contrôles.
        tasks.push(self.sync_window_gaps());
        Task::batch(tasks)
    }

    fn fade_duration(&self) -> Duration {
        match self.cfg.general.transition {
            Transition::None => Duration::ZERO,
            _ => Duration::from_millis(self.cfg.general.fade_ms),
        }
    }

    fn crossfade(&self) -> Duration {
        Duration::from_millis(self.cfg.general.crossfade_ms)
    }

    fn set_mode(&mut self, mode: DesktopMode) -> Task<Msg> {
        match mode {
            DesktopMode::NoDesktop => self.hide(),
            DesktopMode::LauncherOnly => self.show_with_windows(),
            DesktopMode::FullDesktop => self.show(),
        }
    }

    /// Bureau affiché sous les fenêtres, qui restent (ou reviennent) en place.
    fn show_with_windows(&mut self) -> Task<Msg> {
        if self.visible && self.with_windows {
            return Task::none();
        }
        self.with_windows = true;
        if !self.visible {
            return self.show();
        }
        // Depuis le bureau vide : les fenêtres reviennent, le bureau reste dessous.
        let tasks = vec![
            self.restore_stash(None),
            self.sync_raised(),
            self.sync_submap(),
        ];
        self.publish_status();
        Task::batch(tasks)
    }

    fn hide(&mut self) -> Task<Msg> {
        if !self.visible {
            return Task::none();
        }
        self.visible = false;
        self.with_windows = false;
        self.drag = None;
        self.hovered = None;
        let from = self.fade_value();
        self.fade = Some(Fade {
            from,
            to: 0.0,
            start: Instant::now(),
            dur: self.fade_duration(),
        });
        let mut tasks: Vec<Task<Msg>> = Vec::new();
        for id in self.raised.iter().copied().collect::<Vec<_>>() {
            tasks.push(Task::done(Msg::KeyboardInteractivityChange {
                id,
                keyboard_interactivity: KeyboardInteractivity::None,
            }));
            if let Some(m) = self.monitor(id) {
                self.fire(HookEvent::Hide, "hide", m);
            }
        }
        // Les fenêtres flottantes ouvertes sur le bureau partent avec lui
        // (avant le retour des fenêtres mises de côté, qui reprennent le focus).
        let desk: Vec<String> = self.desk_windows.drain().collect();
        if !desk.is_empty() {
            tasks.push(Self::window_result(self.windows.park(desk)));
        }
        // Les fenêtres mises de côté reviennent.
        tasks.push(self.restore_stash(None));
        tasks.push(self.sync_window_gaps());
        tasks.push(self.refresh_walls());
        self.session_windows.clear();
        tasks.push(self.sync_submap());
        tasks.push(self.finish_fade());
        Task::batch(tasks)
    }

    /// Fenêtres en attente dans `hypr::DESK` : elles reviennent sur le bureau
    /// affiché et en refont partie (elles repartiront s'il est masqué).
    fn bring_back_desk_windows(&mut self) -> Vec<Task<Msg>> {
        let parked: Vec<String> = self
            .clients
            .iter()
            .filter(|c| c.workspace == hypr::DESK)
            .map(|c| c.address.clone())
            .collect();
        if parked.is_empty() {
            return Vec::new();
        }
        for a in &parked {
            self.session_windows.insert(a.clone());
            self.desk_windows.insert(a.clone());
        }
        let mut tasks = vec![Self::window_result(self.windows.unpark())];
        for id in self.raised.iter().copied().collect::<Vec<_>>() {
            tasks.push(Task::done(Msg::KeyboardInteractivityChange {
                id,
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
            }));
        }
        tasks.push(self.sync_submap());
        tasks
    }

    /// En fin de fondu sortant, les surfaces redescendent sur `background`.
    fn finish_fade(&mut self) -> Task<Msg> {
        let now = Instant::now();
        let Some(f) = self.fade else {
            return Task::none();
        };
        if !f.done(now) {
            return Task::none();
        }
        self.fade = None;
        if self.visible {
            return Task::none();
        }
        let ids: Vec<Id> = self.raised.drain().collect();
        let tasks: Vec<Task<Msg>> = ids
            .into_iter()
            .flat_map(|id| self.lower_surface(id))
            .collect();
        Task::batch(tasks)
    }

    // -- Écrans ----------------------------------------------------------------

    fn set_monitors(&mut self, list: Vec<Monitor>) -> Task<Msg> {
        if list.is_empty() {
            return Task::none();
        }
        let old = std::mem::replace(&mut self.monitors, list);
        let mut tasks = Vec::new();
        // Disposition ou workspace affiché changés (ou premiers écrans connus) :
        // l'état du verrouillage suit (fond de chaque écran).
        // Le workspace affiché compte aussi : son fond est celui du verrouillage.
        let geometry = |l: &[Monitor]| -> Vec<(String, i64, i64, u32, u32, i64, String)> {
            l.iter()
                .map(|m| {
                    (
                        m.name.clone(),
                        m.x as i64,
                        m.y as i64,
                        m.width,
                        m.height,
                        (m.scale * 100.0) as i64,
                        m.workspace.clone(),
                    )
                })
                .collect()
        };
        if geometry(&old) != geometry(&self.monitors) {
            tasks.push(self.sync_lock_state());
        }
        // Écran branché pendant la session : le signaler, il se règle dans Réglages → Écrans.
        if !old.is_empty() && self.cfg.plugin_on("screens") {
            for m in self
                .monitors
                .iter()
                .filter(|m| !old.iter().any(|o| o.name == m.name))
            {
                tracing::info!("{}", crate::tr!("nouvel écran : {p0}", p0 = m.name));
                let name = m.name.clone();
                tasks.push(Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            hypr::notify(
                                1,
                                8000,
                                &crate::tr!("hyprdesk : nouvel écran {name} — Réglages → Écrans pour le placer", name = name),
                            )
                        })
                        .await;
                    },
                    |_| Msg::Noop,
                ));
            }
        }
        if self.panel.is_some() {
            tasks.push(Self::detect_screens());
        }
        for m in &self.monitors {
            if !self.surfaces.values().any(|n| n == &m.name) {
                let id = Id::unique();
                self.surfaces.insert(id, m.name.clone());
                tracing::info!(
                    "{}",
                    crate::tr!(
                        "surface pour {p0} ({p1}x{p2})",
                        p0 = m.name,
                        p1 = m.width,
                        p2 = m.height
                    )
                );
                tasks.push(Task::done(Msg::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some((0, 0)),
                        layer: Layer::Background,
                        anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                        exclusive_zone: Some(-1),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        output_option: OutputOption::OutputName(m.name.clone()),
                        namespace: Some(NAMESPACE.into()),
                        ..Default::default()
                    },
                    id,
                }));
            }
        }
        let gone: Vec<Id> = self
            .surfaces
            .iter()
            .filter(|(_, n)| !self.monitors.iter().any(|m| &m.name == *n))
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            self.surfaces.remove(&id);
            self.raised.remove(&id);
            tasks.push(iced::window::close(id));
        }
        // Changement de workspace pendant que le bureau est affiché.
        for m in &self.monitors {
            let changed = old
                .iter()
                .find(|o| o.name == m.name)
                .is_some_and(|o| o.workspace != m.workspace);
            let raised = self
                .surfaces
                .iter()
                .any(|(id, n)| n == &m.name && self.raised.contains(id));
            if changed && raised {
                self.fire(HookEvent::Workspace, "workspace", m);
                self.selected = None;
            }
        }
        // Bureau affiché, on change de workspace : ses fenêtres sont mises de côté à leur tour.
        let to_stash: Vec<Monitor> = self
            .monitors
            .iter()
            .filter(|m| {
                old.iter()
                    .find(|o| o.name == m.name)
                    .is_some_and(|o| o.workspace != m.workspace)
                    && self
                        .surfaces
                        .iter()
                        .any(|(id, n)| n == &m.name && self.raised.contains(id))
            })
            .cloned()
            .collect();
        for m in to_stash {
            tasks.push(self.stash_workspace(&m));
        }
        tasks.push(self.refresh_walls());
        tasks.push(self.sync_bars());
        tasks.push(self.sync_lyra_drawer());
        let names = |l: &[Monitor]| l.iter().map(|m| m.name.clone()).collect::<Vec<_>>();
        if names(&old) != names(&self.monitors) && !self.cfg.general.workspace_monitors.is_empty() {
            tasks.push(self.install_rules());
        }
        if self.visible {
            tasks.push(self.sync_raised());
            tasks.push(Task::done(Msg::Poll));
        }
        Task::batch(tasks)
    }

    // -- Fonds d'écran ---------------------------------------------------------

    fn rot_file(
        &mut self,
        idx: usize,
        dir: &std::path::Path,
        interval: Duration,
        shuffle: bool,
    ) -> Option<PathBuf> {
        let r = self.rot.entry(idx).or_insert_with(|| {
            let mut files = list_images(dir);
            if shuffle {
                use rand::seq::SliceRandom;
                files.shuffle(&mut rand::rng());
            }
            if files.is_empty() {
                tracing::warn!(
                    "{}",
                    crate::tr!("rotation : aucune image dans {p0}", p0 = dir.display())
                );
            }
            Rot {
                files,
                idx: 0,
                next: Instant::now() + interval,
            }
        });
        r.files.get(r.idx % r.files.len().max(1)).cloned()
    }

    fn refresh_walls(&mut self) -> Task<Msg> {
        let mut tasks = Vec::new();
        let cfg = self.cfg.clone();
        let monitors = self.monitors.clone();
        // Canevas des scènes étalées : tous les écrans, positions de Hyprland.
        let rect = |m: &Monitor| crate::span::Rect {
            x: m.x,
            y: m.y,
            w: m.width as f32,
            h: m.height as f32,
        };
        let canvas = crate::span::canvas(&monitors.iter().map(rect).collect::<Vec<_>>());
        for m in &monitors {
            let d = resolve(&cfg, &m.name, &m.workspace);
            let place = |rule: &config::WallpaperRule| {
                canvas
                    .filter(|_| rule.span)
                    .map(|c| crate::span::placement(c, rect(m)))
            };
            // Mode verre, bureau affiché : le même fond, flouté.
            let blur = self.visible
                && d.mode == Mode::Glass
                && self
                    .surfaces
                    .iter()
                    .any(|(id, n)| n == &m.name && self.raised.contains(id));
            let want = match (d.wallpaper, d.wallpaper_idx) {
                (None, _) | (_, None) => Wall::Color(cfg.general.background),
                (Some(rule), Some(idx)) => match &rule.source {
                    WallSource::Color(c) => Wall::Color(*c),
                    WallSource::Path(p) => {
                        Wall::Image((p.clone(), m.width, m.height, rule.fit, blur, place(rule)))
                    }
                    WallSource::Rotate {
                        dir,
                        interval,
                        shuffle,
                    } => match self.rot_file(idx, dir, *interval, *shuffle) {
                        Some(p) => Wall::Image((p, m.width, m.height, rule.fit, blur, place(rule))),
                        None => Wall::Color(cfg.general.background),
                    },
                },
            };
            if self.shown.get(&m.name).is_some_and(|s| s.current == want) {
                continue;
            }
            if let Wall::Image(key) = &want
                && !self.walls.contains_key(key)
            {
                // On garde l'ancien fond jusqu'à la fin du décodage.
                if self.loading.insert(key.clone()) {
                    let k = key.clone();
                    tasks.push(Task::perform(
                        async move {
                            let k2 = k.clone();
                            let r = tokio::task::spawn_blocking(move || decode_wall(&k2))
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()));
                            (k, r)
                        },
                        |(k, r)| Msg::WallLoaded(k, r),
                    ));
                }
                continue;
            }
            let prev = self.shown.remove(&m.name).map(|s| s.current);
            self.shown.insert(
                m.name.clone(),
                Shown {
                    current: want,
                    prev,
                    since: Instant::now(),
                },
            );
        }
        // Le cache ne garde que les images affichées (courantes ou en fondu).
        let used: HashSet<&WallKey> = self
            .shown
            .values()
            .flat_map(|s| [Some(&s.current), s.prev.as_ref()])
            .flatten()
            .filter_map(|w| {
                if let Wall::Image(k) = w {
                    Some(k)
                } else {
                    None
                }
            })
            .collect();
        let keep: HashSet<WallKey> = used.into_iter().cloned().collect();
        self.walls
            .retain(|k, _| keep.contains(k) || self.loading.contains(k));
        self.launcher_runtime
            .backdrops
            .retain(|k, _| self.walls.contains_key(k));
        Task::batch(tasks)
    }

    // -- Widgets ---------------------------------------------------------------

    fn poll(&mut self) -> Task<Msg> {
        let now = Instant::now();
        let mut seen = HashSet::new();
        let mut tasks = Vec::new();
        let raised: Vec<Id> = self.raised.iter().copied().collect();
        let cfg = self.cfg.clone();
        // Widgets à relever : ceux des bureaux affichés, et ceux de la barre (toujours visible).
        let mut todo: Vec<&config::WidgetDef> = Vec::new();
        for id in raised {
            let Some(m) = self.monitor(id) else { continue };
            todo.extend(resolve(&cfg, &m.name, &m.workspace).widgets);
        }
        let bar_ids = self.bar_widget_ids();
        todo.extend(cfg.widgets.iter().filter(|w| bar_ids.contains(&w.id)));
        if self.bar_has(&config::BarItem::Volume)
            && !self.pending_widgets.contains("bar:volume")
            && self
                .last_poll
                .get("bar:volume")
                .is_none_or(|t| now.duration_since(*t) >= Duration::from_secs(2))
        {
            self.pending_widgets.insert("bar:volume".into());
            self.last_poll.insert("bar:volume".into(), now);
            tasks.push(Task::perform(bar::read_volume(), Msg::VolumeValue));
        }
        if self.bar_has(&config::BarItem::Tray)
            && !self.pending_widgets.contains("bar:tray")
            && self
                .last_poll
                .get("bar:tray")
                .is_none_or(|t| now.duration_since(*t) >= Duration::from_secs(2))
        {
            self.pending_widgets.insert("bar:tray".into());
            self.last_poll.insert("bar:tray".into(), now);
            tasks.push(Task::perform(
                async {
                    tokio::time::timeout(Duration::from_secs(4), crate::tray::items())
                        .await
                        .unwrap_or_default()
                },
                Msg::TrayItems,
            ));
        }
        {
            for w in todo {
                if self.pending_widgets.contains(&w.id) || !seen.insert(w.id.clone()) {
                    continue;
                }
                if self
                    .last_poll
                    .get(&w.id)
                    .is_some_and(|t| now.duration_since(*t) < w.interval)
                {
                    continue;
                }
                let wid = w.id.clone();
                let generation = self.widget_generation;
                let task: Option<Task<Msg>> = match &w.kind {
                    WidgetKind::Clock { .. } | WidgetKind::Calendar => None,
                    WidgetKind::System { show } => {
                        let show = show.clone();
                        Some(Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || widgets::sample_system(&show))
                                    .await
                                    .map(widgets::Value::Sys)
                                    .unwrap_or_else(|e| widgets::Value::Error(e.to_string()))
                            },
                            move |v| Msg::WidgetValue(generation, wid.clone(), v),
                        ))
                    }
                    WidgetKind::Media { player } => {
                        let p = player.clone();
                        Some(Task::perform(
                            async move {
                                tokio::time::timeout(Duration::from_secs(3), widgets::media(&p))
                                    .await
                                    .unwrap_or_else(|_| {
                                        widgets::Value::Error(
                                            crate::i18n::tr("MPRIS : délai dépassé").into(),
                                        )
                                    })
                            },
                            move |v| Msg::WidgetValue(generation, wid.clone(), v),
                        ))
                    }
                    WidgetKind::Command { exec, timeout } => {
                        let (c, t) = (exec.clone(), *timeout);
                        Some(Task::perform(
                            async move { widgets::run_command(&c, t).await },
                            move |v| Msg::WidgetValue(generation, wid.clone(), v),
                        ))
                    }
                    WidgetKind::Lua { func, .. } => {
                        let (lua, f) = (self.lua.clone(), *func);
                        Some(Task::perform(
                            async move {
                                match lua.render(f).await {
                                    Ok(r) => widgets::Value::Rich(r),
                                    Err(e) => widgets::Value::Error(e),
                                }
                            },
                            move |v| Msg::WidgetValue(generation, wid.clone(), v),
                        ))
                    }
                };
                if let Some(t) = task {
                    self.last_poll.insert(w.id.clone(), now);
                    self.pending_widgets.insert(w.id.clone());
                    tasks.push(t);
                }
            }
        }
        Task::batch(tasks)
    }

    // -- Icônes ----------------------------------------------------------------

    /// Icônes d'une surface avec leur cellule : (index dans cfg.icons, cellule).
    fn placed(&self, id: Id) -> Vec<(usize, Cell)> {
        let Some((m, d)) = self.desktop_of(id) else {
            return Vec::new();
        };
        let g = &self.cfg.general.grid;
        let size = self.size_of(id);
        let (cols, rows) = layout::grid_dims(size.width, size.height, g.cell, g.gap, g.margin);
        let wanted: Vec<Option<Cell>> = d
            .icons
            .iter()
            .map(|i| {
                self.layout
                    .get(&layout::key(&m.name, &m.workspace, &i.id))
                    .or(i.pos)
            })
            .collect();
        let cells = layout::place(&wanted, cols, rows);
        d.icons
            .iter()
            .zip(cells)
            .filter_map(|(i, c)| Some((index_of(&self.cfg.icons, *i), c?)))
            .collect()
    }

    fn builtin_icon(&mut self, name: &str) -> IconImg {
        match icons::get(name) {
            Some(handle) => {
                self.vectors.push(handle);
                IconImg::Vector(self.vectors.len() - 1)
            }
            None => IconImg::None,
        }
    }

    /// Handle iced pour une image d'icône (SVG ou matricielle), créé une fois.
    fn handle_for(&mut self, p: Option<&std::path::Path>) -> IconImg {
        match p {
            Some(p) if p.extension().is_some_and(|e| e == "svg" || e == "svgz") => {
                self.vectors.push(svg::Handle::from_path(p));
                IconImg::Vector(self.vectors.len() - 1)
            }
            Some(p) => {
                self.rasters.push(image::Handle::from_path(p));
                IconImg::Raster(self.rasters.len() - 1)
            }
            None => IconImg::None,
        }
    }

    /// Police de l'interface, à la graisse voulue.
    /// Graisse de la barre et des titres : la version grasse de la police.
    pub(super) fn title_font(&self) -> Font {
        self.ui_font(iced::font::Weight::Bold)
    }

    pub(super) fn ui_font(&self, weight: iced::font::Weight) -> Font {
        Font {
            family: iced::font::Family::Name(self.ui_family),
            weight,
            ..Font::DEFAULT
        }
    }

    /// Icône du thème par nom ; les icônes symboliques sont teintées (`tint`).
    pub(super) fn icon_named<'a>(
        &self,
        name: &str,
        size: f32,
        tint: Color,
    ) -> Option<Element<'a, Msg>> {
        match self.named_icons.get(name).copied()? {
            IconImg::Vector(i) => {
                let symbolic = name.ends_with("-symbolic") || icons::get(name).is_some();
                Some(
                    svg(self.vectors[i].clone())
                        .width(size)
                        .height(size)
                        .style(move |_, _| svg::Style {
                            color: symbolic.then_some(tint),
                        })
                        .into(),
                )
            }
            IconImg::Raster(i) => Some(
                image(self.rasters[i].clone())
                    .width(size)
                    .height(size)
                    .into(),
            ),
            IconImg::None => None,
        }
    }

    /// Résout une icône par nom (une fois), hors du thread UI.
    pub(super) fn want_icon(&mut self, name: &str) -> Task<Msg> {
        if self.named_icons.contains_key(name) || self.named_pending.contains(name) {
            return Task::none();
        }
        // Icône d'interface : le jeu de hyprdesk d'abord (cohérence graphique),
        // sans passer par le thème d'icônes.
        let symbolic = name.ends_with("-symbolic") || name.starts_with("hyprdesk-");
        if symbolic && icons::get(name).is_some() {
            let img = self.builtin_icon(name);
            self.named_icons.insert(name.to_string(), img);
            return Task::none();
        }
        self.named_pending.insert(name.to_string());
        let (n, theme, gen_) = (
            name.to_string(),
            self.cfg.general.icon_theme.clone(),
            self.icons_gen,
        );
        Task::perform(
            async move {
                let n2 = n.clone();
                let p = tokio::task::spawn_blocking(move || {
                    // Repli : le thème par défaut. Jamais la version en couleur
                    // d'une icône symbolique : elle détonnerait dans la barre.
                    launch::lookup_icon(&n2, theme.as_deref(), 32)
                        .or_else(|| launch::lookup_icon(&n2, Some("Adwaita"), 32))
                })
                .await
                .ok()
                .flatten();
                (n, p)
            },
            move |(n, p)| Msg::IconResolved(gen_, n, p),
        )
    }

    fn img_element(&self, img: IconImg, size: f32, alpha: f32) -> Option<Element<'_, Msg>> {
        match img {
            IconImg::Raster(i) => Some(
                image(self.rasters[i].clone())
                    .width(size)
                    .height(size)
                    .opacity(alpha)
                    .into(),
            ),
            IconImg::Vector(i) => Some(
                svg(self.vectors[i].clone())
                    .width(size)
                    .height(size)
                    .opacity(alpha)
                    .into(),
            ),
            IconImg::None => None,
        }
    }

    fn launch(&mut self, idx: usize) -> Task<Msg> {
        let Some(r) = self.icons.get(idx) else {
            return Task::none();
        };
        let cmd = r.command.clone();
        let stays = !self.cfg.general.close_on_launch;
        let t = self.run_from_desktop(cmd, stays);
        if stays {
            t
        } else {
            Task::batch([t, self.hide()])
        }
    }

    /// Lance une commande depuis le bureau. S'il reste affiché, la fenêtre
    /// s'y ouvre flottante et centrée (option `float_launched`) : le bureau
    /// la contient.
    pub(super) fn run_from_desktop(&self, cmd: String, stays: bool) -> Task<Msg> {
        self.run_from_desktop_result(cmd, stays, |_| Msg::Noop)
    }

    pub(super) fn run_from_desktop_result(
        &self,
        cmd: String,
        stays: bool,
        then: impl FnOnce(bool) -> Msg + Send + 'static,
    ) -> Task<Msg> {
        let size = (stays && self.cfg.general.float_launched)
            .then(|| {
                self.monitors.iter().find(|m| m.focused).map(|m| {
                    (
                        (m.width as f32 * 0.6) as u32,
                        (m.height as f32 * 0.7) as u32,
                    )
                })
            })
            .flatten();
        tracing::info!("{}", crate::tr!("lancement : {cmd}", cmd = cmd));
        Task::perform(
            async move {
                let r = tokio::task::spawn_blocking(move || match size {
                    Some((w, h)) => hypr::exec_floating(&cmd, w, h),
                    None => hypr::exec(&cmd),
                })
                .await;
                match r {
                    Ok(Err(e)) => {
                        tracing::warn!("{}", crate::tr!("lancement : {e}", e = e));
                        false
                    }
                    Err(e) => {
                        tracing::warn!("{}", crate::tr!("lancement : {e}", e = e));
                        false
                    }
                    Ok(Ok(())) => true,
                }
            },
            then,
        )
    }

    fn navigate(&mut self, id: Id, key: &keyboard::Key) -> Task<Msg> {
        use keyboard::key::Named;
        let placed = self.placed(id);
        if placed.is_empty() {
            return Task::none();
        }
        let cur = self
            .selected
            .filter(|(w, _)| *w == id)
            .and_then(|(_, i)| placed.iter().find(|(x, _)| *x == i).copied());
        let pick = |pred: &dyn Fn(Cell) -> Option<u32>| {
            placed
                .iter()
                .filter_map(|(i, c)| pred(*c).map(|k| (k, *i)))
                .min()
                .map(|(_, i)| i)
        };
        let next = match (key, cur) {
            (keyboard::Key::Named(Named::Enter), Some((i, _))) => return self.launch(i),
            (keyboard::Key::Named(Named::Enter), None) => None,
            (_, None) => placed
                .iter()
                .min_by_key(|(_, c)| (c.0, c.1))
                .map(|(i, _)| *i),
            (keyboard::Key::Named(n), Some((_, (c, r)))) => {
                let dist = |a: u16, b: u16| (a as i32 - b as i32).unsigned_abs();
                match n {
                    Named::ArrowDown => pick(&|(cc, rr)| (cc == c && rr > r).then(|| dist(rr, r))),
                    Named::ArrowUp => pick(&|(cc, rr)| (cc == c && rr < r).then(|| dist(rr, r))),
                    Named::ArrowRight => {
                        pick(&|(cc, rr)| (cc > c).then(|| dist(cc, c) * 1000 + dist(rr, r)))
                    }
                    Named::ArrowLeft => {
                        pick(&|(cc, rr)| (cc < c).then(|| dist(cc, c) * 1000 + dist(rr, r)))
                    }
                    _ => return Task::none(),
                }
            }
            _ => return Task::none(),
        };
        if let Some(i) = next {
            self.selected = Some((id, i));
        }
        Task::none()
    }

    /// Glisser-déposer de fichiers vers le bureau : chaque fichier ou dossier
    /// devient une icône, posée dans la case sous le pointeur (puis les
    /// suivantes pour un dépôt de plusieurs fichiers).
    fn on_drop(&mut self, win: Id, ev: DropEvent) -> Task<Msg> {
        match ev {
            DropEvent::Hover => {
                self.drop_hover = Some(win);
                self.drop_count = 0;
                Task::none()
            }
            DropEvent::Left => {
                self.drop_hover = None;
                Task::none()
            }
            DropEvent::File(path) => {
                // Seulement sur le bureau (pas la barre, le lanceur, les tiroirs).
                if !self.surfaces.contains_key(&win) || !self.cfg.plugin_on("icons") {
                    return Task::none();
                }
                let Some(id) = self.settings.add_dropped(&path) else {
                    return Task::none();
                };
                tracing::info!(
                    "{}",
                    crate::tr!("icône déposée : {path}", path = path.display())
                );
                if let Some(cell) = self.drop_cell(win, self.drop_count)
                    && let Some(m) = self.monitor(win).cloned()
                {
                    self.layout
                        .set(layout::key(&m.name, &m.workspace, &id), cell);
                    self.layout.save();
                }
                self.drop_count += 1;
                self.drop_hover = None;
                self.commit()
            }
        }
    }

    /// Case libre sous le pointeur (ou la `n`-ième suivante sur la ligne).
    fn drop_cell(&self, win: Id, n: u16) -> Option<(u16, u16)> {
        let p = self.cursor.get(&win).copied()?;
        let g = self.cfg.general.grid.clone();
        let size = self.size_of(win);
        let (cols, rows) = layout::grid_dims(size.width, size.height, g.cell, g.gap, g.margin);
        let (rl, rt, _, _) = self.reserved(win);
        let (c, r) = layout::cell_at(p.x - rl, p.y - rt, g.cell, g.gap, g.margin, cols, rows)?;
        let taken: Vec<(u16, u16)> = self.placed(win).into_iter().map(|(_, c)| c).collect();
        // Case visée, puis les suivantes (ligne par ligne) jusqu'à une libre.
        (0..cols as u32 * rows as u32)
            .map(|k| {
                let i = (r as u32 * cols as u32 + c as u32 + k) % (cols as u32 * rows as u32);
                ((i % cols as u32) as u16, (i / cols as u32) as u16)
            })
            .filter(|cell| !taken.contains(cell))
            .nth(n as usize)
    }

    fn drop_icon(&mut self, win: Id, icon: usize) {
        let Some(p) = self.cursor.get(&win).copied() else {
            return;
        };
        let g = self.cfg.general.grid.clone();
        let size = self.size_of(win);
        let (cols, rows) = layout::grid_dims(size.width, size.height, g.cell, g.gap, g.margin);
        // Le curseur est en coordonnées de surface ; la grille commence après la zone réservée.
        let (rl, rt, _, _) = self.reserved(win);
        let Some(target) = layout::cell_at(p.x - rl, p.y - rt, g.cell, g.gap, g.margin, cols, rows)
        else {
            return;
        };
        let placed = self.placed(win);
        let Some(&(_, from)) = placed.iter().find(|(i, _)| *i == icon) else {
            return;
        };
        let Some(m) = self.monitor(win).cloned() else {
            return;
        };
        let key_of = |i: usize| layout::key(&m.name, &m.workspace, &self.cfg.icons[i].id);
        // Cellule occupée : les deux icônes échangent leur place.
        if let Some(&(other, _)) = placed.iter().find(|(_, c)| *c == target) {
            let k = key_of(other);
            self.layout.set(k, from);
        }
        let k = key_of(icon);
        self.layout.set(k, target);
        self.layout.save();
    }

    // -- Mise à jour -------------------------------------------------------------

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        let started = Instant::now();
        let name = msg_name(&msg);
        self.count_msg(&msg);
        let t = self.update_inner(msg);
        self.publish_status();
        tracing::trace!(
            message = name,
            update_us = started.elapsed().as_micros() as u64,
            "UI update"
        );
        t
    }

    fn update_inner(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Control(c) => match c {
                // Le raccourci d'un mode y mène, ou rend no-desktop s'il y est déjà.
                control::Command::Toggle => {
                    if self.mode() == DesktopMode::FullDesktop {
                        self.hide()
                    } else {
                        self.show()
                    }
                }
                control::Command::ToggleLauncher => {
                    if self.mode() == DesktopMode::LauncherOnly {
                        self.hide()
                    } else {
                        self.show_with_windows()
                    }
                }
                control::Command::Show => self.show(),
                control::Command::Hide => self.hide(),
                control::Command::LauncherOnly => self.show_with_windows(),
                control::Command::Cycle => self.set_mode(self.mode().next()),
                control::Command::Reload => self.load_config(),
                control::Command::Quit => {
                    if self.in_submap {
                        drop(self.windows.submap(false));
                    }
                    // Rien ne doit rester de côté après l'arrêt, ni caché avec le bureau.
                    Self::window_result(self.windows.unpark())
                        .chain(self.restore_stash(None))
                        .chain(Task::perform(
                            async {
                                crate::session::stop_all().await;
                                let _ = tokio::task::spawn_blocking(hypr::remove_keys).await;
                                let _ = tokio::task::spawn_blocking(hypr::remove_rules).await;
                                let _ = tokio::task::spawn_blocking(|| hypr::set_desktop_gaps(&[]))
                                    .await;
                            },
                            |_| Msg::Noop,
                        ))
                        .chain(iced::exit())
                }
                control::Command::Status => Task::none(),
                control::Command::Osd(line) => match osd::parse(&line) {
                    Some((kind, value, muted)) => self.show_osd(kind, value, muted),
                    None => Task::none(),
                },
                control::Command::Menu(kind) => self.open_menu_by_type(&kind),
                control::Command::Assistant => self.open_assistant(),
                // Visite guidée : les réglages s'ouvrent, puis la première étape.
                control::Command::Tour => {
                    let o = self.open_panel();
                    Task::batch([
                        o,
                        Task::done(Msg::Panel(PanelMsg::Tour(tour::TourMsg::Start))),
                    ])
                }
                // Les réglages s'ouvrent sans changer le mode du bureau.
                control::Command::Settings(tab) => {
                    let s = Task::none();
                    let o = self.open_panel();
                    // « paquet/onglet » ; paquet inconnu : l'onglet Bureau.
                    let (pkg, sub) = tab.split_once('/').unwrap_or((tab.as_str(), ""));
                    let tab = plugins::find(pkg).map_or("desktop", |p| p.id);
                    if let Some(sub) = plugins::find_tab(tab, sub) {
                        self.subtabs.insert(tab, sub);
                    }
                    Task::batch([s, o, Task::done(Msg::Panel(PanelMsg::Tab(tab)))])
                }
            },
            Msg::Hypr(hypr::Update::Monitors(list)) => self.set_monitors(list),
            Msg::Hypr(hypr::Update::Workspaces(list)) => {
                self.workspaces = list;
                Task::none()
            }
            Msg::Hypr(hypr::Update::ActiveWindow(w)) => {
                self.active_window = w;
                Task::none()
            }
            Msg::BarWorkspace(mon, ws) => Task::perform(
                async move {
                    let _ = tokio::task::spawn_blocking(move || {
                        hypr::focus_monitor(&mon);
                        hypr::focus_workspace(&ws)
                    })
                    .await;
                },
                |_| Msg::Noop,
            ),
            Msg::BarScroll(mon, dir) => Task::perform(
                async move {
                    let _ = tokio::task::spawn_blocking(move || {
                        hypr::focus_monitor(&mon);
                        hypr::focus_workspace(if dir > 0 { "+1" } else { "-1" })
                    })
                    .await;
                },
                |_| Msg::Noop,
            ),
            Msg::VolumeMute => self.volume_action(vec![
                "set-mute".into(),
                "@DEFAULT_AUDIO_SINK@".into(),
                "toggle".into(),
            ]),
            Msg::VolumeStep(d) => self.volume_action(vec![
                "set-volume".into(),
                "-l".into(),
                "1.5".into(),
                "@DEFAULT_AUDIO_SINK@".into(),
                format!("{}%{}", d.abs(), if d > 0 { "+" } else { "-" }),
            ]),
            Msg::VolumeValue(v) => {
                self.pending_widgets.remove("bar:volume");
                self.volume = v;
                Task::none()
            }
            Msg::MicMute => self.mic_action(vec![
                "set-mute".into(),
                "@DEFAULT_AUDIO_SOURCE@".into(),
                "toggle".into(),
            ]),
            Msg::MicSet(v) => {
                if let Some(cur) = self.audio.mic.as_mut() {
                    cur.0 = v;
                }
                self.mic_action(vec![
                    "set-volume".into(),
                    "@DEFAULT_AUDIO_SOURCE@".into(),
                    format!("{v:.2}"),
                ])
            }
            Msg::AudioDefault(id) => {
                let is_output = self.audio.outputs.iter().any(|d| d.id == id);
                let list = if is_output {
                    &mut self.audio.outputs
                } else {
                    &mut self.audio.inputs
                };
                for d in list.iter_mut() {
                    d.default = d.id == id;
                }
                // Le volume affiché suit le nouveau périphérique.
                let after = self.volume_action(vec!["set-default".into(), id.to_string()]);
                after.chain(Task::perform(bar::read_audio(), Msg::AudioValue))
            }
            Msg::LauncherApp(monitor, class) => self.launcher_focus_app(&monitor, &class),
            Msg::OpenAssistant => self.open_assistant(),
            Msg::LauncherHover(spec) => self.launcher_hover(spec),
            Msg::LauncherPointer(monitor, position) => self.launcher_pointer(&monitor, position),
            Msg::LauncherAnchor(monitor, key, bounds) => self.launcher_anchor(monitor, key, bounds),
            Msg::LauncherShown(monitor) => {
                if self
                    .launcher_runtime
                    .docks
                    .get(&monitor)
                    .is_some_and(|f| f.to == 0.0)
                {
                    let fade = self.launcher_transition(0.0, 1.0, 180);
                    self.launcher_runtime.docks.insert(monitor, fade);
                }
                Task::none()
            }
            Msg::LauncherPopupOpen(token) => self.launcher_popup_open(token),
            Msg::LauncherPopupMeasured(id, height) => {
                if self.launcher_popup.as_ref().is_some_and(|p| p.id == id) {
                    self.launcher_fit_popup(Some(height))
                } else {
                    Task::none()
                }
            }
            Msg::LauncherLaunchExpired(id, token) => {
                self.launcher_launch_expired(&id, token);
                Task::none()
            }
            Msg::LauncherResolved(id, token, command) => {
                if !self
                    .launcher_runtime
                    .launches
                    .get(&id)
                    .is_some_and(|l| l.started == token)
                {
                    return Task::none();
                }
                if command.is_empty() {
                    self.launcher_launch_expired(&id, token);
                    return Task::none();
                }
                self.run_from_desktop_result(command, true, move |ok| {
                    Msg::LauncherDispatched(id, token, ok)
                })
            }
            Msg::LauncherDispatched(id, token, ok) => {
                if !ok {
                    self.launcher_launch_expired(&id, token);
                }
                Task::none()
            }
            Msg::LauncherPopupClose(token) => self.launcher_popup_timeout(token),
            Msg::LauncherPopupHover(on) => self.launcher_popup_hover(on),
            Msg::LauncherPick(address) => {
                let close = self.close_launcher_popup();
                let focus = self.update_inner(Msg::FocusWindow(address));
                Task::batch([close, focus])
            }
            Msg::LauncherLaunch(id) => self.launcher_launch(&id),
            Msg::AudioValue(a) => {
                self.audio = a;
                Task::none()
            }
            Msg::Screens(list) => {
                if !list.is_empty() {
                    self.screens = list;
                }
                Task::none()
            }
            Msg::ScreensTick => self.screens_tick(),
            Msg::Hypr(hypr::Update::Reloaded) => {
                // Une relecture de la config Hyprland efface nos raccourcis, nos règles et notre submap.
                // Les règles de marge ont disparu avec la relecture : les reposer.
                self.window_gaps.clear();
                self.lyra_rule = None;
                let mut t = vec![
                    self.install_keys(),
                    self.install_rules(),
                    self.sync_window_gaps(),
                    self.sync_lyra_drawer(),
                    self.sync_tiling(),
                    self.sync_fx(),
                ];
                if self.in_submap {
                    self.in_submap = false;
                    t.push(self.sync_submap());
                }
                Task::batch(t)
            }
            Msg::KeysInstalled(r) => {
                match r {
                    Ok((conflicts, universal)) => {
                        if !conflicts.is_empty() && conflicts != self.key_conflicts {
                            let list: Vec<String> = conflicts
                                .iter()
                                .map(|(k, w)| crate::tr!("{k} (déjà lié à {w})", k = k, w = w))
                                .collect();
                            tracing::warn!(
                                "{}",
                                crate::tr!("raccourcis non installés : {p0}", p0 = list.join(", "))
                            );
                            hypr::notify(
                                0,
                                8000,
                                &crate::tr!(
                                    "hyprdesk : raccourci laissé à hyprland.lua (hyprdesk ne le pose que bureau affiché) — {p0}",
                                    p0 = list.join(", ")
                                ),
                            );
                        }
                        self.key_conflicts = conflicts;
                        self.universal_binds = universal;
                    }
                    Err(e) => tracing::warn!("{}", crate::tr!("raccourcis : {e}", e = e)),
                }
                Task::none()
            }
            Msg::Hypr(hypr::Update::Clients(list)) => {
                let mut tasks = Vec::new();
                // Bureau vide seulement : avec les fenêtres, une nouvelle fenêtre
                // est une fenêtre ordinaire (mise de côté si on revient au bureau vide).
                if self.visible && !self.with_windows {
                    // Fenêtre ouverte sur un bureau affiché : le bureau la contient ;
                    // elle reçoit le clavier et ses raccourcis habituels.
                    let fresh: Vec<Client> = list
                        .iter()
                        .filter(|c| !self.clients.iter().any(|o| o.address == c.address))
                        .filter(|c| c.workspace != hypr::STASH && c.workspace != hypr::DESK)
                        .cloned()
                        .collect();
                    for c in fresh {
                        let surfaces: Vec<Id> = self
                            .surfaces
                            .iter()
                            .filter(|(id, n)| {
                                self.raised.contains(id)
                                    && self
                                        .monitors
                                        .iter()
                                        .any(|m| &m.name == *n && m.workspace == c.workspace)
                            })
                            .map(|(id, _)| *id)
                            .collect();
                        if surfaces.is_empty() {
                            continue;
                        }
                        self.session_windows.insert(c.address.clone());
                        self.desk_windows.insert(c.address.clone());
                        for id in surfaces {
                            tasks.push(Task::done(Msg::KeyboardInteractivityChange {
                                id,
                                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                            }));
                        }
                    }
                    tasks.push(self.sync_submap());
                }
                self.session_windows
                    .retain(|address| list.iter().any(|c| &c.address == address));
                self.desk_windows
                    .retain(|address| list.iter().any(|c| &c.address == address));
                tasks.push(self.sync_submap());
                self.clients = list;
                tasks.push(self.launcher_frame());
                tasks.push(self.sync_raised());
                tasks.push(self.resolve_class_icons());
                Task::batch(tasks)
            }
            Msg::Apps(list) => {
                self.apps = Arc::new(list);
                // Meilleures correspondances classe → .desktop : on recommence.
                self.class_imgs.clear();
                self.class_pending.clear();
                self.resolve_class_icons()
            }
            Msg::ClassIcons(gen_, list) => {
                if gen_ == self.icons_gen {
                    for (class, p) in list {
                        self.class_pending.remove(&class);
                        let img = self.handle_for(p.as_deref());
                        self.class_imgs.insert(class, img);
                    }
                }
                Task::none()
            }
            // Le lanceur ne change jamais de mode. En full-desktop, la fenêtre
            // choisie revient seule de côté, par-dessus le bureau, comme une
            // fenêtre ouverte depuis lui ; les autres restent de côté.
            Msg::FocusWindow(addr) => {
                let back = if self.mode() == DesktopMode::FullDesktop {
                    self.session_windows.insert(addr.clone());
                    let mut t = vec![Self::window_result(self.windows.restore_window(&addr))];
                    for id in self.raised.iter().copied().collect::<Vec<_>>() {
                        t.push(Task::done(Msg::KeyboardInteractivityChange {
                            id,
                            keyboard_interactivity: KeyboardInteractivity::OnDemand,
                        }));
                    }
                    t.push(self.sync_submap());
                    Task::batch(t)
                } else {
                    Task::none()
                };
                let focus = Task::perform(
                    async move {
                        if let Ok(Err(e)) =
                            tokio::task::spawn_blocking(move || hypr::focus_window(&addr)).await
                        {
                            tracing::warn!("{}", crate::tr!("focus : {e}", e = e));
                        }
                    },
                    |_| Msg::Noop,
                );
                back.chain(focus)
            }
            // Les réglages s'ouvrent sans changer le mode du bureau.
            Msg::OpenPanel => self.open_panel(),
            Msg::Panel(m) => self.panel_update(m),
            Msg::Chat(m) => self.chat_update(m),
            Msg::SettingsLoaded(revision, s) => {
                if revision
                    == self
                        .settings_revision
                        .load(std::sync::atomic::Ordering::Acquire)
                {
                    self.settings = s;
                }
                Task::none()
            }
            Msg::ConfigChanged => self.load_config(),
            Msg::ConfigLoaded(Ok(cfg)) => {
                self.widget_generation += 1;
                self.values.clear();
                self.last_poll.clear();
                self.menu_request += 1;
                let close = self.close_popover();
                tracing::info!(
                    "{}",
                    crate::tr!(
                        "config chargée : {p0} fonds, {p1} icônes, {p2} widgets",
                        p0 = cfg.wallpapers.len(),
                        p1 = cfg.icons.len(),
                        p2 = cfg.widgets.len()
                    )
                );
                if self.config_error.take().is_some() {
                    hypr::notify(
                        3,
                        2500,
                        crate::i18n::tr("hyprdesk : configuration rechargée"),
                    );
                }
                Task::batch([close, self.install_config(cfg)])
            }
            Msg::ConfigLoaded(Err(e)) => {
                tracing::error!("{}", crate::tr!("config : {e}", e = e));
                hypr::notify(4, 10000, &format!("hyprdesk : {e}"));
                self.config_error = Some(e);
                Task::none()
            }
            Msg::IconsResolved(gen_, list, media) => {
                if gen_ != self.icons_gen {
                    return Task::none();
                }
                self.rasters.clear();
                self.vectors.clear();
                self.icon_imgs = list
                    .iter()
                    .map(|r| self.handle_for(r.image.as_deref()))
                    .collect();
                self.ui_imgs = media
                    .iter()
                    .zip(UI_ICONS)
                    .map(|(p, name)| match p {
                        Some(p) => self.handle_for(Some(p)),
                        None => self.builtin_icon(name),
                    })
                    .collect();
                self.icons = list;
                // Les poignées des fenêtres et des icônes nommées vivaient dans les vecteurs vidés plus haut.
                self.class_imgs.clear();
                self.class_pending.clear();
                self.named_icons.clear();
                self.named_pending.clear();
                let mut again: Vec<Task<Msg>> = bar::STATIC_ICONS
                    .iter()
                    .map(|n| self.want_icon(n))
                    .collect();
                let names: Vec<String> = self
                    .values
                    .values()
                    .filter_map(|v| {
                        if let widgets::Value::Rich(r) = v {
                            r.icon.clone()
                        } else {
                            None
                        }
                    })
                    .chain(self.tray.iter().filter_map(|t| t.icon_name.clone()))
                    .collect();
                for n in names {
                    again.push(self.want_icon(&n));
                }
                again.push(self.resolve_class_icons());
                Task::batch(again)
            }
            Msg::WallLoaded(key, r) => {
                self.loading.remove(&key);
                match r {
                    Ok(decoded) => {
                        self.launcher_runtime
                            .backdrops
                            .insert(key.clone(), decoded.backdrop);
                        self.walls.insert(key, decoded.handle);
                    }
                    Err(e) => {
                        tracing::warn!("{}", crate::tr!("fond d'écran : {e}", e = e));
                        hypr::notify(
                            0,
                            5000,
                            &crate::tr!("hyprdesk : fond d'écran illisible — {e}", e = e),
                        );
                        return Task::none();
                    }
                }
                self.refresh_walls()
            }
            Msg::OsdTick => self.osd_tick(),
            Msg::Polkit(m) => self.polkit_update(m),
            Msg::Toasts(m) => self.toasts_update(m),
            Msg::Rotate => {
                let now = Instant::now();
                let cfg = self.cfg.clone();
                let mut changed = false;
                for (idx, r) in self.rot.iter_mut() {
                    if now >= r.next
                        && let Some(WallSource::Rotate { interval, .. }) =
                            cfg.wallpapers.get(*idx).map(|w| &w.source)
                    {
                        r.idx = (r.idx + 1) % r.files.len().max(1);
                        r.next = now + *interval;
                        changed = true;
                    }
                }
                if changed {
                    self.refresh_walls()
                } else {
                    Task::none()
                }
            }
            Msg::Frame => {
                let now = Instant::now();
                let mut closed = Task::none();
                if let Some(f) = self.panel_fade
                    && f.done(now)
                {
                    self.panel_fade = None;
                    if f.to == 0.0
                        && let Some(p) = self.panel.take()
                    {
                        // Le tiroir fermé : sa surface disparaît, sa place revient aux fenêtres.
                        closed = Task::batch([
                            iced::window::close(p.surface),
                            self.sync_raised(),
                            self.sync_window_gaps(),
                        ]);
                    }
                }
                self.hover_fades.retain(|_, f| f.to > 0.0 || !f.done(now));
                let cf = self.crossfade();
                for s in self.shown.values_mut() {
                    if s.prev.is_some() && now.duration_since(s.since) >= cf {
                        s.prev = None;
                    }
                }
                let launcher = self.launcher_frame();
                Task::batch([closed, launcher, self.finish_fade()])
            }
            Msg::Poll => self.poll(),
            Msg::WidgetValue(generation, id, v) => {
                self.pending_widgets.remove(&id);
                if generation != self.widget_generation
                    || !self.cfg.widgets.iter().any(|w| w.id == id)
                {
                    return Task::none();
                }
                let t = match &v {
                    widgets::Value::Rich(r) => r
                        .icon
                        .clone()
                        .map(|i| self.want_icon(&i))
                        .unwrap_or_else(Task::none),
                    _ => Task::none(),
                };
                self.values.insert(id, v);
                t
            }
            Msg::IconResolved(gen_, name, path) => {
                if gen_ == self.icons_gen {
                    self.named_pending.remove(&name);
                    let img = match path {
                        Some(p) => self.handle_for(Some(&p)),
                        None => self.builtin_icon(&name),
                    };
                    self.named_icons.insert(name, img);
                }
                Task::none()
            }
            Msg::OpenPopover(bar_id, key, req) => {
                let toggling = self.popover.as_ref().is_some_and(|p| p.key == key);
                let close = self.close_popover();
                self.menu_request += 1;
                let request = self.menu_request;
                if toggling {
                    return close;
                }
                let task = match req {
                    // Les périphériques d'abord : ils fixent la hauteur du menu.
                    popover::Request::Volume => {
                        Task::perform(bar::read_audio(), |a| a).then(move |a| {
                            Task::batch([
                                Task::done(Msg::AudioValue(a)),
                                Task::done(Msg::PopoverReady(
                                    request,
                                    bar_id,
                                    key.clone(),
                                    popover::Kind::Volume,
                                )),
                            ])
                        })
                    }
                    popover::Request::Calendar => Task::done(Msg::PopoverReady(
                        request,
                        bar_id,
                        key,
                        popover::Kind::Calendar,
                    )),
                    popover::Request::LuaMenu { widget, func } => {
                        let lua = self.lua.clone();
                        Task::perform(async move { lua.menu(func).await }, move |r| match r {
                            Ok(entries) => Msg::PopoverReady(
                                request,
                                bar_id,
                                key.clone(),
                                popover::Kind::Menu {
                                    entries,
                                    source: popover::MenuSource::Lua {
                                        widget: widget.clone(),
                                    },
                                },
                            ),
                            Err(e) => {
                                tracing::warn!(
                                    "{}",
                                    crate::tr!("menu de {widget} : {e}", widget = widget, e = e)
                                );
                                hypr::notify(
                                    0,
                                    5000,
                                    &crate::tr!(
                                        "hyprdesk : menu de « {widget} » — {e}",
                                        widget = widget,
                                        e = e
                                    ),
                                );
                                Msg::Noop
                            }
                        })
                    }
                    popover::Request::Tray { bus, menu_path } => Task::perform(
                        {
                            let (b, m) = (bus.clone(), menu_path.clone());
                            async move { crate::tray::menu(&b, &m).await }
                        },
                        move |r| match r {
                            Ok(entries) => Msg::PopoverReady(
                                request,
                                bar_id,
                                key.clone(),
                                popover::Kind::Menu {
                                    entries,
                                    source: popover::MenuSource::Tray {
                                        bus: bus.clone(),
                                        menu_path: menu_path.clone(),
                                    },
                                },
                            ),
                            Err(e) => {
                                tracing::warn!(
                                    "{}",
                                    crate::tr!("menu de la zone de notification : {e}", e = e)
                                );
                                Msg::Noop
                            }
                        },
                    ),
                };
                Task::batch([close, task])
            }
            Msg::PopoverReady(request, bar_id, key, kind) => {
                if request != self.menu_request {
                    return Task::none();
                }
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(hypr::cursor_pos)
                            .await
                            .ok()
                            .flatten()
                    },
                    move |point| {
                        Msg::PopoverPositioned(request, bar_id, key.clone(), kind.clone(), point)
                    },
                )
            }
            Msg::PopoverPositioned(request, bar_id, key, kind, point) => {
                if request != self.menu_request {
                    return Task::none();
                }
                if let Some((x, y)) = point
                    && let Some(bar) = self.bar_surfaces.get(&bar_id)
                    && let Some(m) = self.monitors.iter().find(|m| m.name == bar.monitor)
                {
                    self.cursor.insert(bar_id, Point::new(x - m.x, y - m.y));
                }
                self.open_popover(bar_id, key, kind)
            }
            Msg::ClosePopover => {
                self.menu_request += 1;
                self.close_popover()
            }
            Msg::PopoverMeasured(win, height) => self.fit_popover(win, height),
            Msg::Popover(m) => self.popover_update(m),
            Msg::LuaCall(widget, func, arg) => {
                let lua = self.lua.clone();
                Task::perform(
                    async move {
                        if let Err(e) = lua.call(func, arg).await {
                            tracing::warn!("{widget} : {e}");
                        }
                        widget
                    },
                    Msg::Repoll,
                )
            }
            Msg::Repoll(widget) => {
                self.last_poll.remove(&widget);
                self.poll()
            }
            Msg::VolumeSet(v) => {
                if let Some(cur) = self.volume.as_mut() {
                    cur.0 = v;
                }
                self.volume_action(vec![
                    "set-volume".into(),
                    "@DEFAULT_AUDIO_SINK@".into(),
                    format!("{v:.2}"),
                ])
            }
            Msg::TrayItems(items) => {
                self.pending_widgets.remove("bar:tray");
                self.tray_imgs
                    .retain(|key, _| items.iter().any(|item| &item.key == key));
                let mut tasks = Vec::new();
                for t in &items {
                    if let Some((w, h, data)) = &t.pixmap {
                        use std::hash::{Hash, Hasher};
                        let mut hs = std::collections::hash_map::DefaultHasher::new();
                        data.hash(&mut hs);
                        let fp = hs.finish();
                        if self.tray_imgs.get(&t.key).is_none_or(|(f, _)| *f != fp) {
                            self.tray_imgs.insert(
                                t.key.clone(),
                                (fp, image::Handle::from_rgba(*w, *h, data.clone())),
                            );
                        }
                    } else if let Some(n) = &t.icon_name {
                        self.tray_imgs.remove(&t.key);
                        tasks.push(self.want_icon(n));
                    }
                }
                self.tray = items;
                Task::batch(tasks)
            }
            Msg::TrayActivate(_bar, bus, path, secondary) => Task::perform(
                async move {
                    let (x, y) = tokio::task::spawn_blocking(hypr::cursor_pos)
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                    crate::tray::activate(bus, path, secondary, x as i32, y as i32).await;
                },
                |_| Msg::Noop,
            ),
            Msg::Resized(id, size) => {
                self.sizes.insert(id, size);
                Task::none()
            }
            Msg::Closed(id) => {
                if self.osd.as_ref().is_some_and(|o| o.win == id) {
                    self.osd = None;
                    return Task::none();
                }
                if self.toasts.win == Some(id) {
                    self.toasts.win = None;
                    return Task::none();
                }
                // Carte polkit fermée par le compositeur : la saisie est annulée.
                if self.polkit.as_ref().is_some_and(|d| d.win == id) {
                    if let Some(mut d) = self.polkit.take() {
                        if let Some(r) = d.reply.take() {
                            r.send(None);
                        }
                        crate::lock::pam::wipe(&mut d.input);
                    }
                    return Task::none();
                }
                if self
                    .popover
                    .as_ref()
                    .is_some_and(|p| p.win == id || p.catcher == id)
                {
                    // Fermé par le compositeur : on referme l'autre surface aussi.
                    return self.close_popover();
                }
                if self.panel.as_ref().is_some_and(|p| p.surface == id) {
                    self.panel = None;
                    self.panel_fade = None;
                    return self.sync_window_gaps();
                }
                // Tiroir de l'assistant fermé : caché seulement, la session continue.
                if let Some(c) = self.chat.as_mut().filter(|c| c.surface == Some(id)) {
                    c.surface = None;
                    return self.sync_window_gaps();
                }
                if self.launcher_popup.as_ref().is_some_and(|p| p.id == id) {
                    self.launcher_popup = None;
                    return Task::none();
                }
                self.launcher_surfaces.remove(&id);
                self.bar_surfaces.remove(&id);
                self.surfaces.remove(&id);
                self.raised.remove(&id);
                self.sizes.remove(&id);
                Task::none()
            }
            Msg::Drop(id, ev) => self.on_drop(id, ev),
            Msg::Cursor(id, p) => {
                self.cursor.insert(id, p);
                if let Some(d) = self.wdrag.as_mut()
                    && d.win == id
                {
                    let (dx, dy) = (p.x - d.start.x, p.y - d.start.y);
                    if d.moved || dx.abs() + dy.abs() > DRAG_THRESHOLD {
                        d.moved = true;
                        let mut np = d.from;
                        np.x = (d.from.x + dx).round();
                        np.y = (d.from.y + dy).round();
                        let wid = d.id.clone();
                        self.settings.place_widget(&wid, np);
                        let next = self.settings.apply_to(&self.cfg);
                        return self.install_config(Arc::new(next));
                    }
                    return Task::none();
                }
                if let Some(d) = self.drag.as_mut()
                    && d.win == id
                    && !d.active
                    && p.distance(d.start) > DRAG_THRESHOLD
                {
                    d.active = true;
                }
                Task::none()
            }
            Msg::Released if self.wdrag.is_some() => {
                let moved = self.wdrag.take().is_some_and(|d| d.moved);
                if moved {
                    self.panel_update(PanelMsg::Commit)
                } else {
                    Task::none()
                }
            }
            Msg::Released => match self.drag.take() {
                Some(d) if d.active => {
                    self.drop_icon(d.win, d.icon);
                    Task::none()
                }
                Some(d) => self.launch(d.icon),
                None => Task::none(),
            },
            Msg::Key(id, key) => {
                if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                    if self.popover.is_some() {
                        self.menu_request += 1;
                        return self.close_popover();
                    }
                    if self.panel.is_some() {
                        return self.close_panel();
                    }
                    if self.chat_shown() {
                        return self.close_chat();
                    }
                    return self.hide();
                }
                // Le panneau a ses propres champs de saisie : pas de navigation dessous.
                if self.panel.is_some() || self.chat_shown() {
                    return Task::none();
                }
                self.navigate(id, &key)
            }
            Msg::IconPress(win, icon) => {
                let start = self.cursor.get(&win).copied().unwrap_or(Point::ORIGIN);
                self.selected = Some((win, icon));
                self.drag = Some(Drag {
                    win,
                    icon,
                    start,
                    active: false,
                });
                Task::none()
            }
            Msg::WidgetPress(win, wid) => {
                let Some(from) = self
                    .cfg
                    .widgets
                    .iter()
                    .find(|w| w.id == wid)
                    .map(|w| w.placement())
                else {
                    return Task::none();
                };
                let start = self.cursor.get(&win).copied().unwrap_or(Point::ORIGIN);
                if let Some(p) = self.panel.as_mut() {
                    p.select_widget(&wid);
                    // Son éditeur est dans l'onglet « Sur ce bureau ».
                    self.subtabs.insert(plugins::widgets::INFO.id, "bureau");
                }
                self.wdrag = Some(WidgetDrag {
                    win,
                    id: wid,
                    start,
                    from,
                    moved: false,
                });
                Task::none()
            }
            Msg::IconHover(h) => {
                if let Some((id, index)) = self.hovered {
                    let _ = self.update_inner(Msg::Hover(format!("desktop:{id:?}:{index}"), false));
                }
                self.hovered = h;
                if let Some((id, index)) = h {
                    return self.update_inner(Msg::Hover(format!("desktop:{id:?}:{index}"), true));
                }
                Task::none()
            }
            Msg::Hover(key, on) => {
                let from = self.hover_value(&key);
                self.hover_fades.insert(
                    key,
                    Fade {
                        from,
                        to: if on { 1.0 } else { 0.0 },
                        start: Instant::now(),
                        dur: self.motion_duration(),
                    },
                );
                Task::none()
            }
            Msg::Launch(i) => self.launch(i),
            Msg::Media(bus, method) => {
                for w in &self.cfg.widgets {
                    if matches!(w.kind, WidgetKind::Media { .. }) {
                        self.last_poll.remove(&w.id);
                    }
                }
                Task::perform(widgets::media_control(bus, method), |_| Msg::Noop).chain(
                    Task::perform(tokio::time::sleep(Duration::from_millis(150)), |_| {
                        Msg::Poll
                    }),
                )
            }
            Msg::Noop => Task::none(),
            // Messages layer-shell : traités par iced_layershell.
            _ => Task::none(),
        }
    }

    fn subscription(&self) -> Subscription<Msg> {
        let mut subs = vec![
            Subscription::run(control::server_stream).map(Msg::Control),
            // Agent polkit (fenêtre de saisie) ; inactif si un autre agent est en place.
            Subscription::run(polkit::stream).map(|e| Msg::Polkit(polkit::PMsg::Event(e))),
            // Démon de notifications (bulles) ; inactif si un autre tient le nom.
            Subscription::run(toasts::stream).map(|e| Msg::Toasts(toasts::TMsg::Event(e))),
            Subscription::run(control::signal_stream).map(Msg::Control),
            Subscription::run(hypr::stream).map(Msg::Hypr),
            Subscription::run_with(self.config_path.clone(), |path: &PathBuf| {
                watch_stream(path.as_path())
            })
            .map(|_| Msg::ConfigChanged),
            iced::window::resize_events().map(|(id, s)| Msg::Resized(id, s)),
            iced::window::close_events().map(Msg::Closed),
        ];
        if !self.toasts.list.is_empty() {
            subs.push(
                iced::time::every(Duration::from_millis(500))
                    .map(|_| Msg::Toasts(toasts::TMsg::Tick)),
            );
        }
        if self.osd.is_some() {
            subs.push(iced::time::every(Duration::from_millis(250)).map(|_| Msg::OsdTick));
        }
        if !self.raised.is_empty()
            || self.popover.is_some()
            || self.panel.is_some()
            || self.chat_shown()
        {
            // Bureau affiché (glisser-déposer, clavier), menu ou réglages ouverts (Échap).
            // Jamais pour la seule barre : chaque mouvement de souris coûterait
            // un rendu de toutes les surfaces.
            subs.push(iced::event::listen_with(listen));
        }
        if !self.raised.is_empty() {
            subs.push(iced::time::every(Duration::from_millis(500)).map(|_| Msg::Poll));
        } else if self.cfg.bar.is_some() {
            // La barre reste visible : horloge et relevés continuent, à 1 Hz.
            subs.push(iced::time::every(Duration::from_secs(1)).map(|_| Msg::Poll));
        }
        if self.animating() {
            if self.launcher_animating() {
                let focused = self
                    .monitors
                    .iter()
                    .find(|m| m.focused)
                    .map(|m| m.name.as_str());
                let driver = self
                    .launcher_surfaces
                    .iter()
                    .find(|(_, s)| Some(s.monitor.as_str()) == focused)
                    .or_else(|| self.launcher_surfaces.iter().next())
                    .map(|(id, _)| *id)
                    .or_else(|| self.launcher_popup.as_ref().map(|p| p.id));
                subs.push(
                    iced::event::listen_raw(|event, _, id| {
                        matches!(
                            event,
                            Event::Window(iced::window::Event::RedrawRequested(_))
                        )
                        .then_some(id)
                    })
                    .with(driver)
                    .filter_map(|(driver, id)| {
                        driver
                            .is_none_or(|driver| driver == id)
                            .then_some(Msg::Frame)
                    }),
                );
            } else {
                subs.push(iced::time::every(Duration::from_millis(16)).map(|_| Msg::Frame));
            }
        }
        if !self.rot.is_empty() {
            subs.push(iced::time::every(Duration::from_secs(1)).map(|_| Msg::Rotate));
        }
        if self.screens_pending.is_some() {
            subs.push(iced::time::every(Duration::from_secs(1)).map(|_| Msg::ScreensTick));
        }
        subs.push(self.chat_subscription());
        // Glisser-déposer de fichiers : toujours écouté ; pendant un survol, la
        // position du pointeur est suivie pour savoir où déposer.
        subs.push(iced::event::listen_with(|e, _, id| match e {
            Event::Window(iced::window::Event::FileHovered(_)) => {
                Some(Msg::Drop(id, DropEvent::Hover))
            }
            Event::Window(iced::window::Event::FilesHoveredLeft) => {
                Some(Msg::Drop(id, DropEvent::Left))
            }
            Event::Window(iced::window::Event::FileDropped(p)) => {
                Some(Msg::Drop(id, DropEvent::File(p)))
            }
            _ => None,
        }));
        if self.drop_hover.is_some() {
            subs.push(iced::event::listen_with(|e, _, id| match e {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    Some(Msg::Cursor(id, position))
                }
                _ => None,
            }));
        }
        Subscription::batch(subs)
    }

    // -- Vue ---------------------------------------------------------------------

    fn view(&self, id: Id) -> Element<'_, Msg> {
        if self.bar_surfaces.contains_key(&id) {
            return self.view_bar(id);
        }
        if self.panel.as_ref().is_some_and(|p| p.surface == id) {
            return self.view_panel();
        }
        if self.chat.as_ref().is_some_and(|c| c.surface == Some(id)) {
            return self.view_chat();
        }
        if self.launcher_popup.as_ref().is_some_and(|p| p.id == id) {
            return self.view_launcher_popup();
        }
        if let Some((_, label)) = self.identify.iter().find(|(i, _)| *i == id) {
            return self.view_identify(label);
        }
        if let Some(s) = self.launcher_surfaces.get(&id) {
            let Some(m) = self.monitors.iter().find(|m| m.name == s.monitor) else {
                return Space::new().into();
            };
            return self.view_launcher(m, self.fade_value());
        }
        if self.osd.as_ref().is_some_and(|o| o.win == id) {
            return self.view_osd();
        }
        if self.polkit.as_ref().is_some_and(|d| d.win == id) {
            return self.view_polkit();
        }
        if self.toasts.win == Some(id) {
            return self.view_toasts();
        }
        if let Some(p) = &self.popover {
            if p.win == id {
                return self.view_popover();
            }
            if p.catcher == id {
                return self.view_catcher();
            }
        }
        let Some((m, d)) = self.desktop_of(id) else {
            return Space::new().into();
        };
        let raised = self.raised.contains(&id);
        let t = if raised { self.fade_value() } else { 1.0 };
        let mut layers: Vec<Element<'_, Msg>> = Vec::new();
        // Le bureau est sous les fenêtres : le fond reste plein, seul le contenu apparaît en fondu.
        // Paquet « Fond d'écran » coupé : la couleur de fond seule.
        layers.push(if self.cfg.plugin_on("wallpaper") {
            self.view_wall(&m.name, 1.0)
        } else {
            let c = color(self.cfg.general.background, 1.0);
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| bg(c))
                .into()
        });
        if raised && d.mode == Mode::Glass {
            // Mode verre : un voile teinté sur le fond flouté.
            let veil = color(
                self.cfg.general.theme.bg,
                self.cfg.general.glass_opacity * t,
            );
            layers.push(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| bg(veil))
                    .into(),
            );
        }
        let panel_here = self.panel.as_ref().is_some_and(|p| p.win == id);
        if raised && (!panel_here || self.size_of(id).width - self.panel_width() >= 400.0) {
            // Avec les fenêtres, seul le lanceur (dock) reste : ni icônes ni widgets.
            let mut content: Vec<Element<'_, Msg>> = Vec::new();
            if !self.with_windows {
                if self.cfg.plugin_on("icons") {
                    content.push(self.view_icons(id, t));
                }
                for w in d.widgets.iter().filter(|_| self.cfg.plugin_on("widgets")) {
                    content.push(self.view_widget(id, w, t));
                }
            }
            // Le lanceur a sa propre surface, au-dessus des fenêtres.
            let shift = if self.cfg.general.transition == Transition::Slide {
                (1.0 - t) * SLIDE_PX
            } else {
                0.0
            };
            // Le tiroir des réglages occupe la droite : le bureau reste visible
            // à côté et montre chaque réglage en direct.
            let (rl, rt, rr, rb) = self.reserved(id);
            let right = rr + if panel_here { self.panel_width() } else { 0.0 };
            layers.push(
                container(
                    Stack::with_children(content)
                        .width(Length::Fill)
                        .height(Length::Fill),
                )
                .padding(Padding {
                    top: rt + shift,
                    right,
                    bottom: rb,
                    left: rl,
                })
                .into(),
            );
        }
        Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_wall(&self, monitor: &str, t: f32) -> Element<'_, Msg> {
        if self.cfg.wallpapers.is_empty()
            && self.cfg.general.background == config::General::default().background
        {
            return container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| design::wallpaper())
                .into();
        }
        let Some(s) = self.shown.get(monitor) else {
            let c = color(self.cfg.general.background, t);
            return container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| bg(c))
                .into();
        };
        let base = color(self.cfg.general.background, t);
        let cf = self.crossfade().as_secs_f32();
        let p = if cf <= 0.0 {
            1.0
        } else {
            (Instant::now().duration_since(s.since).as_secs_f32() / cf).clamp(0.0, 1.0)
        };
        let mut layers = vec![
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| bg(base))
                .into(),
        ];
        if let Some(prev) = &s.prev {
            layers.push(self.wall_element(prev, t));
            layers.push(self.wall_element(&s.current, t * p));
        } else {
            layers.push(self.wall_element(&s.current, t));
        }
        Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn wall_element(&self, w: &Wall, alpha: f32) -> Element<'_, Msg> {
        match w {
            Wall::Color(c) => {
                let c = color(*c, alpha);
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| bg(c))
                    .into()
            }
            Wall::Image(key) => match self.walls.get(key) {
                Some(h) => {
                    let fit = match key.3 {
                        Fit::Cover => iced::ContentFit::Cover,
                        Fit::Contain => iced::ContentFit::Contain,
                        Fit::Fill => iced::ContentFit::Fill,
                        Fit::Center => iced::ContentFit::None,
                    };
                    image(h.clone())
                        .content_fit(if key.4 { iced::ContentFit::Cover } else { fit })
                        .filter_method(image::FilterMethod::Linear)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .opacity(alpha)
                        .into()
                }
                None => Space::new().into(),
            },
        }
    }

    fn icon_cell(
        &self,
        win: Id,
        idx: usize,
        def: &IconDef,
        t: f32,
        ghost: bool,
    ) -> Element<'_, Msg> {
        let g = &self.cfg.general;
        let th = &g.theme;
        let size = g.grid.icon_size as f32;
        let alpha = if ghost { t * 0.5 } else { t };
        let img: Element<'_, Msg> = match self.img_element(
            self.icon_imgs.get(idx).copied().unwrap_or(IconImg::None),
            size,
            alpha,
        ) {
            Some(e) => e,
            None => {
                let c = color(th.accent, alpha);
                container(
                    text(
                        def.label
                            .clone()
                            .unwrap_or_default()
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_string(),
                    )
                    .font(self.font)
                    .size(size * 0.5),
                )
                .center(size)
                .style(move |_| container::Style {
                    background: Some(c.into()),
                    border: Border::default().rounded(size / 4.0),
                    ..Default::default()
                })
                .into()
            }
        };
        let label = self
            .icons
            .get(idx)
            .map(|r| r.label.clone())
            .unwrap_or_else(|| def.label.clone().unwrap_or_default());
        let fg = color(th.fg, alpha);
        let selected = self.selected == Some((win, idx));
        let hover = self.hover_value(&format!("desktop:{win:?}:{idx}"));
        let hl = if selected {
            Some(color(th.accent, 0.45 * t))
        } else if hover > 0.0 {
            Some(color(th.fg, 0.09 * t * hover))
        } else {
            None
        };
        let cell = g.grid.cell as f32;
        let body = container(
            column![
                iced::widget::float(img).scale(1.0 + hover * 0.07),
                container(
                    text(label)
                        .font(self.font)
                        .size(g.font_size)
                        .color(fg)
                        .font(self.font)
                        .align_x(Alignment::Center)
                        .width(Length::Fill)
                )
                .padding([3, 7])
                .style(move |_| container::Style {
                    background: Some(Color::from_rgba(0.02, 0.03, 0.06, 0.40 * alpha).into()),
                    border: Border::default().rounded(7),
                    ..Default::default()
                })
            ]
            .spacing(9)
            .align_x(Alignment::Center),
        )
        .width(cell)
        .height(cell)
        .padding(6)
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: hl.map(Background::Color),
            border: Border::default()
                .rounded(16)
                .width(if selected { 1.0 } else { 0.0 })
                .color(color(th.accent, 0.6 * t)),
            // Ombre de texte simulée par un fond léger quand rien n'est survolé.
            ..Default::default()
        });
        mouse_area(body)
            .on_press(Msg::IconPress(win, idx))
            .on_enter(Msg::IconHover(Some((win, idx))))
            .on_exit(Msg::IconHover(None))
            .interaction(mouse::Interaction::Pointer)
            .into()
    }

    fn view_icons(&self, win: Id, t: f32) -> Element<'_, Msg> {
        let g = &self.cfg.general.grid;
        let step = (g.cell + g.gap) as f32;
        let dragging = self
            .drag
            .as_ref()
            .filter(|d| d.active && d.win == win)
            .map(|d| d.icon);
        let mut layers: Vec<Element<'_, Msg>> = Vec::new();
        for (idx, (c, r)) in self.placed(win) {
            let def = &self.cfg.icons[idx];
            let cell = self.icon_cell(win, idx, def, t, dragging == Some(idx));
            layers.push(
                pin(cell)
                    .x(g.margin as f32 + c as f32 * step)
                    .y(g.margin as f32 + r as f32 * step)
                    .into(),
            );
        }
        if let (Some(idx), Some(p)) = (dragging, self.cursor.get(&win)) {
            let half = g.cell as f32 / 2.0;
            let def = &self.cfg.icons[idx];
            let (rl, rt, _, _) = self.reserved(win);
            layers.push(
                pin(self.icon_cell(win, idx, def, t, false))
                    .x(p.x - rl - half)
                    .y(p.y - rt - half)
                    .into(),
            );
        }
        Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl App {}

/// Les trois modes du bureau, dans l'ordre du bouton Bureau de la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DesktopMode {
    /// Fond d'écran seul, fenêtres comme d'habitude.
    NoDesktop,
    /// Fenêtres en place, le lanceur (dock) accessible en bas.
    LauncherOnly,
    /// Bureau complet : icônes, widgets, dock ; fenêtres mises de côté.
    FullDesktop,
}

impl DesktopMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            DesktopMode::NoDesktop => "no-desktop",
            DesktopMode::LauncherOnly => "launcher-only",
            DesktopMode::FullDesktop => "full-desktop",
        }
    }

    /// Mode suivant du bouton : no-desktop → launcher-only → full-desktop → no-desktop.
    pub(super) fn next(self) -> Self {
        match self {
            DesktopMode::NoDesktop => DesktopMode::LauncherOnly,
            DesktopMode::LauncherOnly => DesktopMode::FullDesktop,
            DesktopMode::FullDesktop => DesktopMode::NoDesktop,
        }
    }
}

/// Diagnostic : messages traités par type, relevé toutes les 5 s ; un trafic
/// anormal (> 1500 messages, soit 300/s) est signalé dans le journal.
static MSG_STATS: std::sync::Mutex<Option<(Instant, HashMap<&'static str, u32>)>> =
    std::sync::Mutex::new(None);

fn msg_name(m: &Msg) -> &'static str {
    match m {
        Msg::Control(_) => "Control",
        Msg::Hypr(hypr::Update::Monitors(_)) => "Hypr::Monitors",
        Msg::Hypr(hypr::Update::Clients(_)) => "Hypr::Clients",
        Msg::Hypr(hypr::Update::Workspaces(_)) => "Hypr::Workspaces",
        Msg::Hypr(hypr::Update::ActiveWindow(_)) => "Hypr::ActiveWindow",
        Msg::Hypr(hypr::Update::Reloaded) => "Hypr::Reloaded",
        Msg::Poll => "Poll",
        Msg::Frame => "Frame",
        Msg::Rotate => "Rotate",
        Msg::OsdTick => "OsdTick",
        Msg::Polkit(..) => "Polkit",
        Msg::Toasts(..) => "Toasts",
        Msg::WidgetValue(..) => "WidgetValue",
        Msg::Cursor(..) => "Cursor",
        Msg::Resized(..) => "Resized",
        Msg::IconResolved(..) => "IconResolved",
        Msg::TrayItems(_) => "TrayItems",
        Msg::VolumeValue(_) => "VolumeValue",
        Msg::AudioValue(_) => "AudioValue",
        Msg::WallLoaded(..) => "WallLoaded",
        Msg::Noop => "Noop",
        Msg::ConfigChanged | Msg::ConfigLoaded(_) => "Config",
        _ => "autre",
    }
}

impl App {
    fn count_msg(&self, m: &Msg) {
        let Ok(mut g) = MSG_STATS.lock() else { return };
        let (start, map) = g.get_or_insert_with(|| (Instant::now(), HashMap::new()));
        *map.entry(msg_name(m)).or_default() += 1;
        if start.elapsed() >= Duration::from_secs(5) {
            let total: u32 = map.values().sum();
            if total > 1500 {
                let mut v: Vec<_> = map.iter().map(|(k, n)| (*n, *k)).collect();
                v.sort_by(|a, b| b.cmp(a));
                tracing::warn!(
                    "{}",
                    crate::tr!(
                        "trafic élevé : {total} messages en 5 s — {top}",
                        total = total,
                        top = format!("{:?}", &v[..v.len().min(6)])
                    )
                );
            }
            *g = Some((Instant::now(), HashMap::new()));
        }
    }
}

/// Thème iced des contrôles (panneau Réglages) tiré des couleurs de la config.
fn make_theme(cfg: &Config) -> iced::Theme {
    let th = &cfg.general.theme;
    iced::Theme::custom(
        "hyprdesk",
        iced::theme::Palette {
            background: color(th.bg, 1.0),
            text: color(th.fg, 1.0),
            primary: color(th.accent, 1.0),
            success: Color::from_rgb8(0x8a, 0xff, 0x80),
            warning: Color::from_rgb8(0xff, 0xff, 0x80),
            danger: Color::from_rgb8(0xff, 0x95, 0x80),
        },
    )
}

fn bg(c: Color) -> container::Style {
    container::Style {
        background: Some(c.into()),
        ..Default::default()
    }
}

/// Vérifie la config sans lancer le démon (`hyprdesk check`).
pub fn check(path: &std::path::Path) -> Result<Config, String> {
    config::lua::load_file(path).map(|l| l.config)
}

#[cfg(test)]
mod regression_tests {
    use super::*;

    fn app() -> App {
        App::boot_with(PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"), true).0
    }
    fn monitor(name: &str, focused: bool) -> Monitor {
        Monitor {
            name: name.into(),
            width: 1440,
            height: 900,
            workspace: "1".into(),
            focused,
            reserved: (0.0, 0.0, 0.0, 0.0),
            x: 0.0,
            y: 0.0,
            scale: 1.0,
        }
    }

    #[test]
    fn an_old_result_cannot_repopulate_the_new_configuration() {
        let mut app = app();
        app.widget_generation = 2;
        app.pending_widgets.insert("removed-widget".into());
        let _ = app.update_inner(Msg::WidgetValue(
            1,
            "removed-widget".into(),
            widgets::Value::Text("stale".into()),
        ));
        assert!(app.values.is_empty());
        assert!(app.pending_widgets.is_empty());
    }

    #[test]
    fn a_settings_reload_cannot_overwrite_a_newer_edit() {
        let mut app = app();
        app.settings.font = Some("Adwaita Sans".into());
        app.settings_revision
            .store(2, std::sync::atomic::Ordering::Release);
        let _ = app.update_inner(Msg::SettingsLoaded(1, DeskSettings::default()));
        assert_eq!(app.settings.font.as_deref(), Some("Adwaita Sans"));
    }

    #[test]
    fn desktop_cycles_through_three_modes() {
        let mut app = app();
        let a = Id::unique();
        app.monitors = vec![monitor("A", true)];
        app.surfaces.insert(a, "A".into());
        let cycle = |app: &mut App| {
            let _ = app.update_inner(Msg::Control(control::Command::Cycle));
            app.mode()
        };
        // no-desktop → launcher-only → full-desktop → no-desktop.
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        assert_eq!(cycle(&mut app), DesktopMode::LauncherOnly);
        assert!(app.raised.contains(&a), "le lanceur est affiché");
        assert!(!app.in_submap, "les raccourcis de Hyprland restent actifs");
        assert_eq!(cycle(&mut app), DesktopMode::FullDesktop);
        assert!(app.in_submap);
        assert_eq!(cycle(&mut app), DesktopMode::NoDesktop);
        assert!(!app.in_submap);
    }

    #[test]
    fn each_mode_key_goes_there_or_back_to_no_desktop() {
        let mut app = app();
        let a = Id::unique();
        app.monitors = vec![monitor("A", true)];
        app.surfaces.insert(a, "A".into());
        let key = |app: &mut App, c: control::Command| {
            let _ = app.update_inner(Msg::Control(c));
            app.mode()
        };
        use control::Command::{Hide, Toggle, ToggleLauncher};
        assert_eq!(key(&mut app, ToggleLauncher), DesktopMode::LauncherOnly);
        assert_eq!(key(&mut app, Toggle), DesktopMode::FullDesktop);
        assert_eq!(key(&mut app, ToggleLauncher), DesktopMode::LauncherOnly);
        assert_eq!(key(&mut app, ToggleLauncher), DesktopMode::NoDesktop);
        assert_eq!(key(&mut app, Toggle), DesktopMode::FullDesktop);
        assert_eq!(key(&mut app, Toggle), DesktopMode::NoDesktop);
        assert_eq!(key(&mut app, Hide), DesktopMode::NoDesktop);
    }

    #[test]
    fn settings_never_change_the_mode_and_always_close() {
        let mut app = app();
        let a = Id::unique();
        app.monitors = vec![monitor("A", true)];
        app.surfaces.insert(a, "A".into());
        app.sizes.insert(a, Size::new(1440.0, 900.0));
        Arc::make_mut(&mut app.cfg).general.transition = Transition::None;
        // no-desktop : les réglages s'ouvrent dans leur surface, sans bureau.
        let _ = app.update_inner(Msg::Control(control::Command::Settings("desktop".into())));
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        assert!(app.raised.is_empty(), "le bureau n'est pas affiché");
        let surface = app.panel.as_ref().map(|p| p.surface).unwrap();
        assert_ne!(surface, a, "le tiroir a sa propre surface");
        assert!(
            app.window_gaps[0].1[1] > 0,
            "les fenêtres laissent la place du tiroir"
        );
        // Changer de mode ne ferme pas les réglages.
        let _ = app.update_inner(Msg::Control(control::Command::Show));
        let _ = app.update_inner(Msg::Control(control::Command::Hide));
        assert!(app.panel.is_some());
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        // Fermeture : soldée au tick suivant, même si le fondu est déjà fini.
        let _ = app.update_inner(Msg::Panel(panel::PanelMsg::Close));
        let _ = app.update_inner(Msg::Frame);
        assert!(app.panel.is_none() && app.panel_fade.is_none());
        assert!(app.window_gaps.is_empty());
        assert!(!app.animating(), "plus aucune animation en attente");
    }

    /// `hyprdesk settings bureau/raccourcis` : le paquet et son onglet ;
    /// l'onglet choisi est retenu, un onglet inconnu laisse le premier.
    #[test]
    fn settings_ouvre_un_onglet() {
        let mut app = app();
        let win = Id::unique();
        app.surfaces.insert(win, "DP-1".into());
        let _ = app.update_inner(Msg::Control(control::Command::Settings(
            "bureau/raccourcis".into(),
        )));
        assert_eq!(app.subtab("desktop"), "raccourcis");
        assert_eq!(app.subtab("bar"), "general");
        let _ = app.update_inner(Msg::Panel(PanelMsg::SubTab("bar", "modules")));
        assert_eq!(app.subtab("bar"), "modules");
        let _ = app.update_inner(Msg::Control(control::Command::Settings(
            "barre/nimporte".into(),
        )));
        assert_eq!(app.subtab("bar"), "modules");
        assert_eq!(app.subtab("apps"), "");
    }

    #[test]
    fn the_launcher_never_changes_the_mode() {
        let mut app = app();
        let a = Id::unique();
        app.monitors = vec![monitor("A", true)];
        app.surfaces.insert(a, "A".into());
        app.sizes.insert(a, Size::new(1440.0, 900.0));
        // launcher-only : fenêtre choisie dans le dock, puis réglages.
        let _ = app.update_inner(Msg::Control(control::Command::LauncherOnly));
        let _ = app.update_inner(Msg::FocusWindow("0x1".into()));
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
        let _ = app.update_inner(Msg::OpenPanel);
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
        assert!(app.panel.is_some());
        let [_, right, _, _] = app.window_gaps[0].1;
        assert!(right > 0, "tiroir dégagé : {right}");
        assert_eq!(app.launcher_surfaces.len(), 1, "lanceur dans sa surface");
        // full-desktop : la fenêtre choisie revient seule, le mode reste.
        let _ = app.update_inner(Msg::Control(control::Command::Hide));
        let _ = app.update_inner(Msg::Control(control::Command::Show));
        let _ = app.update_inner(Msg::FocusWindow("0x2".into()));
        assert_eq!(app.mode(), DesktopMode::FullDesktop);
        assert!(app.session_windows.contains("0x2"));
        assert!(!app.in_submap, "la fenêtre rendue garde ses raccourcis");
    }

    #[test]
    fn windows_keep_clear_of_the_controls() {
        let mut app = app();
        let a = Id::unique();
        app.monitors = vec![monitor("A", true)];
        app.surfaces.insert(a, "A".into());
        app.sizes.insert(a, Size::new(1440.0, 900.0));
        {
            let cfg = Arc::make_mut(&mut app.cfg);
            cfg.widgets.push(config::WidgetDef {
                origin: config::Origin::User,
                sel: config::Selector::default(),
                id: "calendar".into(),
                type_id: "calendar".into(),
                kind: config::WidgetKind::Calendar,
                anchor: config::Anchor::Right,
                x: 0.0,
                y: 0.0,
                size: 0.0,
                interval: Duration::from_secs(60),
                card: true,
                in_bar: false,
            });
        }
        let _ = app.update_inner(Msg::Control(control::Command::LauncherOnly));
        // Le lanceur a sa surface (zone réservée) : aucune règle de marge.
        assert!(app.window_gaps.is_empty());
        assert_eq!(app.launcher_surfaces.len(), 1);
        assert_eq!(app.launcher_surfaces.values().next().unwrap().monitor, "A");
        // Bureau vide, puis masqué : plus aucune marge.
        let _ = app.update_inner(Msg::Control(control::Command::Show));
        assert_eq!(
            app.launcher_surfaces.len(),
            1,
            "full-desktop : lanceur aussi"
        );
        let _ = app.update_inner(Msg::Control(control::Command::Hide));
        assert!(
            app.launcher_surfaces.is_empty(),
            "no-desktop : pas de lanceur"
        );
    }

    #[test]
    fn the_bar_adapts_to_narrow_and_vertical_screens() {
        let mut app = app();
        let tall = Monitor {
            width: 1440,
            height: 2560,
            ..monitor("V", false)
        };
        let wide = Monitor {
            width: 2560,
            height: 1440,
            ..monitor("H", true)
        };
        app.monitors = vec![tall, wide];
        Arc::make_mut(&mut app.cfg).bar = Some(config::BarDef {
            monitors: None,
            position: config::BarPosition::Top,
            height: 40,
            zones: Default::default(),
            portrait: None,
            compact: config::BarCompact::Auto,
        });
        assert!(app.bar_vertical("V") && !app.bar_vertical("H"));
        assert!(app.bar_compact("V") && !app.bar_compact("H"));
        Arc::make_mut(&mut app.cfg).bar.as_mut().unwrap().compact = config::BarCompact::Never;
        assert!(!app.bar_compact("V"));
        Arc::make_mut(&mut app.cfg).bar.as_mut().unwrap().compact = config::BarCompact::Always;
        assert!(app.bar_compact("H"));
    }

    #[test]
    fn primary_desktop_ignores_focus() {
        let mut app = app();
        let (a, b) = (Id::unique(), Id::unique());
        let at = |name: &str, focused: bool, x: f32| Monitor {
            x,
            ..monitor(name, focused)
        };
        app.surfaces.insert(a, "A".into());
        app.surfaces.insert(b, "B".into());
        app.visible = true;
        {
            let g = &mut Arc::make_mut(&mut app.cfg).general;
            g.show_on = ShowOn::Primary;
            g.primary = Some("B".into());
        }
        let _ = app.set_monitors(vec![at("A", true, 0.0), at("B", false, 1440.0)]);
        assert_eq!(app.raised, HashSet::from([b]));
        // Le focus change d'écran : le bureau reste sur le principal.
        let _ = app.set_monitors(vec![at("A", false, 0.0), at("B", true, 1440.0)]);
        let _ = app.set_monitors(vec![at("A", true, 0.0), at("B", false, 1440.0)]);
        assert_eq!(app.raised, HashSet::from([b]));
        // Principal débranché : repli sur l'écran en 0×0.
        let _ = app.set_monitors(vec![at("A", true, 0.0)]);
        assert_eq!(app.primary_monitor().as_deref(), Some("A"));
        assert!(app.raised.contains(&a));
    }

    #[test]
    fn focused_desktop_follows_monitor_focus() {
        let mut app = app();
        let (a, b) = (Id::unique(), Id::unique());
        app.monitors = vec![monitor("A", true), monitor("B", false)];
        app.surfaces.insert(a, "A".into());
        app.surfaces.insert(b, "B".into());
        app.raised.insert(a);
        app.visible = true;
        Arc::make_mut(&mut app.cfg).general.show_on = ShowOn::Focused;
        let _ = app.set_monitors(vec![monitor("A", false), monitor("B", true)]);
        assert_eq!(app.raised, HashSet::from([b]));
    }

    #[test]
    fn desk_windows_follow_the_desktop() {
        let mut app = app();
        app.visible = true;
        app.session_windows.insert("0x1".into());
        app.desk_windows.insert("0x1".into());
        let _ = app.hide();
        assert!(app.desk_windows.is_empty() && app.session_windows.is_empty());
        // Parties dans DESK : elles reviennent au prochain affichage, du bureau à nouveau.
        app.clients = vec![hypr::Client {
            address: "0x1".into(),
            class: "kitty".into(),
            title: String::new(),
            workspace: hypr::DESK.into(),
        }];
        app.fade = None;
        let _ = app.show();
        assert!(app.desk_windows.contains("0x1") && app.session_windows.contains("0x1"));
    }

    #[test]
    fn closing_the_last_launched_window_restores_shortcut_suspension() {
        let mut app = app();
        app.visible = true;
        app.session_windows.insert("0x1".into());
        let _ = app.update_inner(Msg::Hypr(hypr::Update::Clients(Vec::new())));
        assert!(app.session_windows.is_empty());
        assert!(app.in_submap);
    }
}
