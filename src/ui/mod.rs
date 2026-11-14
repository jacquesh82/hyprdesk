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
mod apps_grid;
pub(crate) mod bar;
mod bar_layout;
mod bar_motion;
mod bar_style;
mod chat;
pub(crate) mod color_picker;
mod context;
mod design;
mod desk_layout;
mod icons;
mod mymfa;
mod mymfa_fenetre;
pub use mymfa_fenetre::run as run_mymfa_fenetre;
mod controls;
mod osd;
mod overview;
mod panel;
pub(crate) mod plugins;
mod polkit;
mod popover;
pub mod preview;
mod problem;
mod search;
mod settings_search;
mod switcher;
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
/// Deux clics sur la même icône dans ce délai : double clic.
const DOUBLE_CLICK: Duration = Duration::from_millis(450);
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
    /// État périodique du démon d'index (loupe du dock, page Réglages).
    IndexStatus(Option<crate::ui::plugins::index::StatusView>),
    Apps(Vec<AppInfo>),
    ClassIcons(u64, Vec<(String, Option<PathBuf>)>),
    FocusWindow(String),
    OpenPanel,
    /// Message venu d'un clic sur un écran (dock, barre) : ce qu'il ouvre
    /// s'ouvre sur cet écran.
    On(String, Box<Msg>),
    Panel(PanelMsg),
    /// Tiroir de l'assistant (LYRA, protocole serve).
    Chat(chat::ChatMsg),
    SettingsLoaded(u64, DeskSettings),
    /// Barre : aller au workspace (écran, nom).
    BarWorkspace(String, String),
    /// Barre : molette sur les workspaces (écran, ±1).
    BarScroll(String, i32),
    BarAnchor(Id, String, iced::Rectangle),
    BarLayout(Id, Vec<usize>, u32),
    ControlToggle(String),
    ControlToggled(u64, String, Result<(), String>),
    VolumeMute,
    VolumeStep(i32),
    VolumeSet(f32),
    VolumeValue(Option<(f32, bool)>),
    /// Réglages ouverts : glisser un module de la barre (zone, rang).
    BarDragStart(usize, usize),
    /// Survol pendant le glisser : place d'insertion (zone, rang).
    BarDragOver(usize, usize),
    /// Sortie d'une zone de dépôt (barre ou liste des réglages).
    BarDragLeave(usize),
    BarDragCancel,
    /// Relâché : le module va à la place survolée.
    BarDragEnd,
    /// Luminosité relue (rétroéclairage, écrans DDC/CI).
    LightValue(bar::Light),
    /// Molette sur le module : pas en pour cent.
    LightStep(bar::LightTarget, i32),
    /// Curseur déplacé (affichage seulement).
    LightSet(bar::LightTarget, f32),
    /// Curseur relâché : la valeur s'applique.
    LightApply(bar::LightTarget),
    /// Micro coupé ou rétabli.
    MicMute,
    MicSet(f32),
    /// Choisit la sortie ou l'entrée par défaut (id PipeWire).
    AudioDefault(u32),
    AudioValue(bar::Audio),
    /// Ouvre l'assistant IA (LYRA), sans changer le mode du bureau.
    OpenAssistant,
    /// Demande à LYRA (titre de la conversation, texte) : régler un conflit.
    AskLyra(String, String),
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
    /// Case du dock activée (clic ou Entrée) : le dock rend le clavier,
    /// puis l'action de la case.
    LauncherActivate(Box<Msg>),
    /// Écrans détectés (désactivés compris).
    Screens(Vec<crate::screens::Screen>),
    /// Seconde écoulée pendant qu'un changement d'écrans attend confirmation.
    ScreensTick,
    /// Ouvre (ou referme) un menu sous un module de la barre : (barre, clé du module, contenu).
    OpenPopover(Id, String, popover::Request),
    PopoverReady(u64, Id, String, popover::Kind),
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
    /// Trousseau myMFA : état, demandes de déverrouillage, carte.
    Mymfa(mymfa::MMsg),
    MymfaF(mymfa_fenetre::FMsg),
    /// Bulles de notifications.
    Toasts(toasts::TMsg),
    /// Fenêtre de détail d'un problème.
    Problem(problem::PrMsg),
    /// Lanceur avec recherche.
    Search(search::SMsg),
    /// Clic droit sur une icône du bureau (surface, icône).
    IconContext(Id, usize),
    /// Clic droit sur un widget (Réglages ouverts) ; sur le fond du bureau.
    WidgetContext(Id, String),
    DeskContext(Id),
    IconMenu(context::CMsg),
    Switcher(switcher::SwMsg),
    /// Écran « Espaces de travail » (organisation des fenêtres).
    Overview(overview::OMsg),
    /// Ouvre l'écran « Espaces de travail » depuis un menu de la barre.
    BarOrganize,
    /// Calendrier : mois suivant (+1), précédent (-1), en cours (0).
    CalendarMonth(i32),
    /// Menu du tray relu pendant qu'il est ouvert (clé du popover, entrées).
    MenuRefreshed(String, Vec<config::MenuEntry>),
    /// Touches de modification (relâcher Alt valide le sélecteur de fenêtres).
    Modifiers(keyboard::Modifiers),
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

/// Glisser d'un module de barre : (zone, rang) d'origine, place survolée.
type BarDrag = ((usize, usize), Option<(usize, usize)>);

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
    /// Dernier clic sur une icône (surface, icône, instant) : double clic.
    last_click: Option<(Id, usize, Instant)>,
    /// Hyprbars chargé par hyprdesk (plein bureau, option `hyprbars`).
    hyprbars_on: bool,
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
    /// Raccourcis : conflits, pris à hyprland.lua, laissés (`crate::keymap`).
    key_report: crate::keymap::Report,
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
    /// Trousseau myMFA : état connu, carte de déverrouillage ouverte.
    pub(crate) mfa: crate::mymfa::State,
    mfa_dialog: Option<mymfa::Dialog>,
    mfa_fenetre: Option<mymfa_fenetre::Fenetre>,
    /// Accueil myMFA de l'ouverture de session (création du coffre, ou
    /// dernière saisie avant le sceau) encore à faire ; démarrage du démon.
    mfa_greet: Option<std::time::Instant>,
    /// Bulles de notifications (si hyprdesk tient le nom du démon).
    toasts: toasts::Toasts,
    /// Problème affiché (sans assistant pour le prendre).
    problem: Option<problem::Problem>,
    /// Lanceur avec recherche ouvert.
    search: Option<search::Search>,
    /// État du démon d'index (loupe du dock : pastille et bulle).
    index_status: Option<crate::ui::plugins::index::StatusView>,
    /// Calendrier du popover : mois affiché, relatif au mois en cours.
    cal_months: i32,
    /// Sélecteur de fenêtres (Alt+Tab) ouvert.
    switcher: Option<switcher::Switcher>,
    /// Écran « Espaces de travail » ouvert (organisation des fenêtres).
    overview: Option<overview::Overview>,
    /// Menu contextuel d'une icône du bureau ouvert.
    icon_menu: Option<context::IconMenu>,
    /// Dossier Bureau suivi (réglage desktop_files) : empreinte, dernier relevé.
    desk_stamp: Option<(crate::desktop_files::Stamp, Instant)>,
    /// Surfaces de la barre native.
    bar_surfaces: HashMap<Id, bar::BarSurface>,
    bar_anchors: HashMap<(Id, String), iced::Rectangle>,
    control_actions: HashMap<String, Option<String>>,
    bar_hidden: HashMap<Id, Vec<usize>>,
    bar_full_width: HashMap<Id, u32>,
    /// Poses de démonstration, jamais activées par le démon de session.
    bar_preview: Option<(usize, bool)>,
    workspaces: Vec<hypr::Workspace>,
    /// Fenêtres avec géométrie : minimaps de l'écran d'organisation et des
    /// miniatures du sélecteur de la barre.
    ws_geos: Vec<hypr::WindowGeo>,
    /// Dernière relecture de ces géométries (rappel périodique).
    ws_geos_at: Option<Instant>,
    /// Fenêtres marquées urgentes par Hyprland (adresses) ; une fenêtre
    /// redevient visible ou ferme, elle sort de là.
    urgent: HashSet<String>,
    active_window: Option<(String, String)>,
    volume: Option<(f32, bool)>,
    /// Touches de modification tenues (Maj + Tab : champ précédent).
    mods: keyboard::Modifiers,
    /// Module de barre en cours de déplacement : (zone, rang) d'origine, et
    /// place d'insertion survolée.
    bar_drag: Option<BarDrag>,

    /// Module Luminosité ; dernière relecture des écrans DDC/CI (lente).
    light: bar::Light,
    light_ddc_at: Option<Instant>,
    /// Micro et périphériques audio (menu Son).
    audio: bar::Audio,
    /// Mode du bureau de chaque workspace (absent : no-desktop), mémorisé
    /// d'une session à l'autre (`layout.toml`, hors aperçu).
    ws_modes: HashMap<String, DesktopMode>,
    persist_modes: bool,
    /// Mode appliqué à chaque écran, et workspace qu'il affichait alors.
    applied: HashMap<String, (DesktopMode, String)>,
    /// Fenêtres étiquetées sans barre Hyprbars (écrans hors plein bureau).
    nobar: HashSet<String>,
    /// Marges posées dans Hyprland pour dégager les contrôles du bureau, par
    /// écran (haut, droite, bas, gauche) : vide hors du bureau avec les fenêtres.
    window_gaps: Vec<(String, [i32; 4])>,
    /// Écran du clic en cours de traitement (`Msg::On`).
    click_monitor: Option<String>,
    /// Règle Hyprland posée pour LYRA (rectangle du tiroir), si elle l'est.
    lyra_rule: Option<(i32, i32, i32, i32)>,
    /// Cartes d'identification des écrans (surface, « numéro\nnom\nmodèle »).
    identify: Vec<(Id, String)>,
    /// Tables gamma des écrans (Réglages → Couleurs des écrans).
    gamma: crate::colors::Gamma,
    /// Réglages de couleur envoyés en dernier (None : rien encore).
    colors_applied: Option<crate::colors::Wanted>,
    /// Mire plein écran sur tous les écrans.
    color_test: plugins::colors::ColorTest,
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

/// Premier démarrage (posé par main) : les Réglages s'ouvrent une fois l'interface prête.
pub static OPEN_SETTINGS_AT_START: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

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
        Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => Some(Msg::Modifiers(m)),
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
            last_click: None,
            // État réel : une instance précédente a pu le laisser chargé.
            hyprbars_on: !preview && crate::hyprbars::loaded(),
            hovered: None,
            cursor: HashMap::new(),
            drag: None,
            wdrag: None,
            settings: DeskSettings::default(),
            settings_revision: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            index_status: None,
            panel: None,
            chat: None,
            chat_session: 0,
            subtabs: HashMap::new(),
            drop_hover: None,
            drop_count: 0,
            apps: Arc::new(Vec::new()),
            clients: Vec::new(),
            urgent: HashSet::new(),
            class_imgs: HashMap::new(),
            class_pending: HashSet::new(),
            theme: make_theme(&Config::default()),
            key_report: Default::default(),
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
            mfa: Default::default(),
            mfa_dialog: None,
            mfa_fenetre: None,
            mfa_greet: Some(std::time::Instant::now()),
            toasts: Default::default(),
            problem: None,
            search: None,
            icon_menu: None,
            switcher: None,
            overview: None,
            ws_geos: Vec::new(),
            ws_geos_at: None,
            cal_months: 0,
            desk_stamp: None,
            bar_surfaces: HashMap::new(),
            bar_anchors: HashMap::new(),
            control_actions: HashMap::new(),
            bar_hidden: HashMap::new(),
            bar_full_width: HashMap::new(),
            bar_preview: None,
            workspaces: Vec::new(),
            active_window: None,
            volume: None,
            light: bar::Light::default(),

            bar_drag: None,
            mods: keyboard::Modifiers::default(),
            light_ddc_at: None,
            audio: bar::Audio::default(),
            ws_modes: if preview {
                HashMap::new()
            } else {
                Layout::load()
                    .modes()
                    .iter()
                    .filter_map(|(w, m)| Some((w.clone(), DesktopMode::parse(m)?)))
                    .collect()
            },
            persist_modes: !preview,
            applied: HashMap::new(),
            nobar: HashSet::new(),
            window_gaps: Vec::new(),
            click_monitor: None,
            launcher_cycle: HashMap::new(),
            lyra_rule: None,
            identify: Vec::new(),
            gamma: if preview {
                crate::colors::Gamma::disabled()
            } else {
                crate::colors::Gamma::spawn()
            },
            colors_applied: None,
            color_test: Default::default(),
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
        let mut tasks = vec![app.load_config(), apps];
        if !preview && OPEN_SETTINGS_AT_START.swap(false, std::sync::atomic::Ordering::Relaxed) {
            // Écrans et configuration connus d'abord.
            tasks.push(Task::perform(
                tokio::time::sleep(Duration::from_secs(2)),
                |_| Msg::Control(control::Command::Settings("desktop".into())),
            ));
        }
        let t = Task::batch(tasks);
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
                async move {
                    let mut cfg = lua.load(path).await?;
                    // Dossier Bureau en icônes (réglage desktop_files), hors du fil de l'interface.
                    let cfg = tokio::task::spawn_blocking(move || {
                        crate::desktop_files::augment(&mut cfg);
                        cfg
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(Arc::new(cfg))
                },
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
        // Thème SmartUI : avant toute recherche d'icône.
        crate::smartui::configure(
            self.cfg.general.launcher.smartui,
            self.cfg.general.icon_theme.as_deref(),
        );
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
        tasks.push(self.apply_screen_colors());
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
        // SmartUI design activé ou coupé : toutes les icônes sont relues.
        let icons_changed = old.icons != self.cfg.icons
            || o.icon_theme != n.icon_theme
            || o.terminal != n.terminal
            || o.launcher.smartui != n.launcher.smartui
            || self.icons_gen == 0;
        if icons_changed {
            tasks.push(self.reload_icons());
        } else if o.running_apps != n.running_apps || o.launcher.pinned != n.launcher.pinned {
            tasks.push(self.resolve_class_icons());
        }
        if o.show_on != n.show_on || o.primary != n.primary {
            tasks.push(self.sync_raised());
        }
        // Premier chargement (catalogue vide avant), raccourcis ou flou modifiés.
        if o.keys != n.keys
            || o.apps_launcher.unified != n.apps_launcher.unified
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
            tasks.push(self.sync_launchers());
            tasks.push(self.close_launcher_popup());
            self.last_poll.retain(|k, _| !k.starts_with("bar:"));
            tasks.push(Task::done(Msg::Poll));
        }
        let idle_changed = o.idle != n.idle;
        if old.components != self.cfg.components
            || idle_changed
            || o.clipboard_history != n.clipboard_history
            || old.catalog.is_empty()
        {
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
            // Capot tenu par hyprdesk : inhibiteur logind (`crate::lid`).
            if let Some(d) = crate::lid::component(&n.idle)
                && !defs.iter().any(|c| c.name == d.name)
            {
                defs.push(d);
            }
            let idle = n.idle.clone();
            tasks.push(Task::perform(
                async move {
                    let _ =
                        tokio::task::spawn_blocking(move || crate::lid::install_binds(&idle)).await;
                },
                |_| Msg::Noop,
            ));
            // Historique du presse-papiers : cliphist, sauf veilleur déjà en route.
            if let Some(d) = crate::clipboard::component(n.clipboard_history)
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
        // Démarrage automatique XDG : une fois par session Hyprland (marqueur),
        // au premier chargement ou quand le réglage vient d'être activé.
        if n.autostart && (!o.autostart || old.catalog.is_empty()) {
            let terminal = n.terminal.clone();
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || crate::autostart::launch_all(&terminal))
                        .await
                        .unwrap_or_default()
                },
                |names| {
                    if !names.is_empty() {
                        tracing::info!(
                            "{}",
                            crate::tr!("démarrage automatique : {n}", n = names.join(", "))
                        );
                    }
                    Msg::Noop
                },
            ));
        }
        if o.suspend_binds != n.suspend_binds {
            tasks.push(self.sync_submap());
        }
        // LYRA en tiroir : règle Hyprland sur sa fenêtre.
        tasks.push(self.sync_lyra_drawer());
        // Thème appliqué aux autres applications (kitty…), à chaque changement.
        if plugins::apptheme::effective(&old) != plugins::apptheme::effective(&self.cfg)
            || (crate::apptheme::options(&old.general)
                != crate::apptheme::options(&self.cfg.general)
                && !plugins::apptheme::effective(&self.cfg).is_empty())
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
        let unified = self.cfg.general.apps_launcher.unified;
        let blurred = self.cfg.general.theme.blurred_namespaces();
        let config_path = self.config_path.clone();
        let layouts = crate::tiling::in_use(&plugins::layouts::effective(&self.cfg));
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let source = crate::keymap::load_source(&config_path);
                    hypr::install_keys(&keys, unified, &blurred, &source, &layouts)
                        .map_err(|e| e.to_string())
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
        // Principal : cible des workspaces dont la règle vise un écran absent.
        let (placement, primary) = if self.cfg.plugin_on("workspaces") {
            let primary = self
                .primary_monitor()
                .or_else(|| self.cfg.general.primary.clone())
                .unwrap_or_default();
            (self.workspace_placement(), Some(primary))
        } else {
            (Vec::new(), None)
        };
        Task::perform(
            async move {
                match tokio::task::spawn_blocking(move || {
                    hypr::install_rules(n, &placement, &rules, primary.as_deref())
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
            // Écran absent (configuration d'un autre poste, écran débranché) :
            // ses workspaces restent libres plutôt qu'invisibles. Les règles
            // sont reposées quand les écrans changent.
            if !self.monitors.is_empty() && !self.monitors.iter().any(|m| m.name == mon) {
                continue;
            }
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
            && self.mode() == DesktopMode::FullDesktop
            && self.cfg.general.suspend_binds
            && self.session_windows.is_empty()
            && !self
                .target_monitor()
                .is_some_and(|m| self.windows_shown_on(&m));
        if want == self.in_submap {
            return Task::none();
        }
        self.in_submap = want;
        Self::window_result(self.windows.submap(want))
    }

    /// Hyprbars : chargé en plein bureau si l'option est active, déchargé
    /// sinon (hors du fil de l'interface).
    fn sync_hyprbars(&mut self) -> Task<Msg> {
        // Plugin global : chargé dès qu'un écran est en plein bureau.
        let want = self.cfg.general.hyprbars
            && self
                .applied
                .values()
                .any(|(m, _)| *m == DesktopMode::FullDesktop);
        if want == self.hyprbars_on {
            return self.sync_bar_tags();
        }
        self.hyprbars_on = want;
        let custom = self.cfg.general.hyprbars_path.clone();
        let (fg, bg) = (self.cfg.general.theme.fg, self.cfg.general.theme.bg);
        // Étiquettes posées après le chargement (la règle doit exister) ;
        // toutes retirées au déchargement.
        let (tag, untag): (Vec<String>, Vec<String>) = if want {
            let w = self.nobar_wanted();
            self.nobar = w.clone();
            (w.into_iter().collect(), Vec::new())
        } else {
            (
                Vec::new(),
                std::mem::take(&mut self.nobar).into_iter().collect(),
            )
        };
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let Some(path) = crate::hyprbars::find(&custom) else {
                        return Err(crate::i18n::tr("Hyprbars introuvable (hyprpm add https://github.com/hyprwm/hyprland-plugins, puis hyprpm enable hyprbars)").to_string());
                    };
                    if want {
                        crate::hyprbars::load(&path)?;
                        let r = crate::hyprbars::add_buttons(fg, bg);
                        crate::hyprbars::set_tags(&tag, &[], true);
                        r
                    } else {
                        crate::hyprbars::set_tags(&[], &untag, false);
                        crate::hyprbars::unload(&path)
                    }
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| {
                if let Err(e) = r {
                    tracing::warn!("{}", e);
                }
                Msg::Noop
            },
        )
    }

    /// Fenêtres qui ne doivent pas avoir de barre Hyprbars : celles dont le
    /// workspace n'est affiché par aucun écran en plein bureau.
    fn nobar_wanted(&self) -> HashSet<String> {
        let full: Vec<&String> = self
            .applied
            .values()
            .filter(|(m, _)| *m == DesktopMode::FullDesktop)
            .map(|(_, ws)| ws)
            .collect();
        self.clients
            .iter()
            .filter(|c| !full.contains(&&c.workspace))
            .map(|c| c.address.clone())
            .collect()
    }

    /// Hyprbars chargé : étiquettes mises à jour (seulement les différences).
    fn sync_bar_tags(&mut self) -> Task<Msg> {
        if !self.hyprbars_on {
            return Task::none();
        }
        let want = self.nobar_wanted();
        let add: Vec<String> = want.difference(&self.nobar).cloned().collect();
        let remove: Vec<String> = self
            .nobar
            .difference(&want)
            .filter(|a| self.clients.iter().any(|c| &c.address == *a))
            .cloned()
            .collect();
        self.nobar = want;
        if add.is_empty() && remove.is_empty() {
            return Task::none();
        }
        Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || {
                    crate::hyprbars::set_tags(&add, &remove, false)
                })
                .await;
            },
            |_| Msg::Noop,
        )
    }

    /// Écran où ouvrir les Réglages, LYRA ou le lanceur : celui du clic,
    /// sinon l'écran actif, sinon le principal.
    pub(super) fn target_monitor(&self) -> Option<String> {
        self.click_monitor
            .clone()
            .filter(|n| self.monitors.iter().any(|m| &m.name == n))
            .or_else(|| {
                self.monitors
                    .iter()
                    .find(|m| m.focused)
                    .map(|m| m.name.clone())
            })
            .or_else(|| self.primary_monitor())
    }

    /// Surface du bureau sur l'écran cible (la première à défaut).
    pub(super) fn target_surface(&self) -> Option<(Id, String)> {
        let target = self.target_monitor();
        self.surfaces
            .iter()
            .find(|(_, n)| Some(*n) == target.as_ref())
            .or_else(|| self.surfaces.iter().next())
            .map(|(id, n)| (*id, n.clone()))
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
        let panel_on = self.panel.as_ref().map(|p| p.win);
        let chat_on = self
            .chat
            .as_ref()
            .filter(|c| c.surface.is_some())
            .map(|c| c.win);
        let mut want: Vec<(String, [i32; 4])> = self
            .surfaces
            .iter()
            .filter_map(|(id, mon)| {
                // Le lanceur réserve lui-même sa place (zone exclusive de sa surface).
                let mut m = [0; 4];
                // Réglages ou assistant : chacun à sa largeur de tiroir.
                if panel_on == Some(*id) {
                    m[1] = m[1].max(self.panel_width().round() as i32);
                } else if chat_on == Some(*id) {
                    m[1] = m[1].max(self.chat_width().round() as i32);
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

    /// Écoute des événements bruts (souris, clavier) : bureau affiché
    /// (glisser-déposer, clavier), menu ou réglages ouverts (Échap), et toute
    /// surface qui prend le clavier seule — recherche, sélecteur de fenêtres,
    /// carte myMFA, dock — même bureau masqué. Jamais pour la seule barre :
    /// chaque mouvement de souris coûterait un rendu de toutes les surfaces.
    fn listens_to_events(&self) -> bool {
        !self.raised.is_empty()
            || self.popover.is_some()
            || self.panel.is_some()
            || self.chat_shown()
            || self.search.is_some()
            || self.switcher.is_some()
            || self.overview.is_some()
            || self.mfa_dialog.is_some()
            || self.mfa_fenetre.is_some()
            || !self.launcher_surfaces.is_empty()
            || self.launcher_runtime.focus.is_some()
    }

    /// Mode du bureau, tel que l'utilisateur le nomme.
    /// Mode de l'écran visé (celui du clic, sinon l'écran actif) : celui du
    /// workspace qu'il affiche.
    pub(super) fn mode(&self) -> DesktopMode {
        self.target_monitor()
            .map(|m| self.monitor_mode(&m))
            .unwrap_or(DesktopMode::NoDesktop)
    }

    /// Essais : un écran « A » (workspace 1) dans ce mode, sans transition.
    #[cfg(test)]
    pub(super) fn test_mode(&mut self, mode: DesktopMode) {
        if self.monitors.is_empty() {
            self.monitors.push(Monitor {
                name: "A".into(),
                width: 1440,
                height: 900,
                workspace: "1".into(),
                base: String::new(),
                focused: true,
                reserved: (0.0, 0.0, 0.0, 0.0),
                x: 0.0,
                y: 0.0,
                scale: 1.0,
            });
        }
        let (name, ws) = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .map(|m| (m.name.clone(), m.workspace.clone()))
            .unwrap_or_default();
        self.ws_modes.insert(ws.clone(), mode);
        let id = match self.surfaces.iter().find(|(_, n)| **n == name) {
            Some((id, _)) => *id,
            None => {
                let id = Id::unique();
                self.surfaces.insert(id, name.clone());
                id
            }
        };
        if mode == DesktopMode::NoDesktop {
            self.raised.remove(&id);
        } else {
            self.raised.insert(id);
        }
        self.applied.insert(name, (mode, ws));
        self.visible = mode != DesktopMode::NoDesktop;
    }

    /// Une fenêtre s'affiche-t-elle sur cet écran, hors du bureau ? Un
    /// scratchpad ouvert par-dessus, par exemple : le bureau ne doit alors
    /// pas prendre le clavier pour lui seul, ni suspendre les raccourcis.
    fn windows_shown_on(&self, monitor: &str) -> bool {
        let Some(m) = self.monitors.iter().find(|m| m.name == monitor) else {
            return false;
        };
        m.workspace.starts_with("special:")
            && self.clients.iter().any(|c| c.workspace == m.workspace)
    }

    /// Mode mémorisé d'un workspace (no-desktop par défaut).
    fn ws_mode(&self, ws: &str) -> DesktopMode {
        self.ws_modes
            .get(ws)
            .copied()
            .unwrap_or(DesktopMode::NoDesktop)
    }

    /// Mode voulu d'un écran : celui du workspace affiché ; écran imposé en
    /// no-desktop : jamais de bureau ; workspace spécial ouvert par-dessus :
    /// le mode en place ne change pas.
    pub(super) fn monitor_mode(&self, name: &str) -> DesktopMode {
        let Some(m) = self.monitors.iter().find(|m| m.name == name) else {
            return DesktopMode::NoDesktop;
        };
        let ws = m.mode_workspace();
        if ws.is_empty() {
            return self.applied_mode(name);
        }
        if self.cfg.general.forced_mode(name, ws) == Some("no-desktop") {
            return DesktopMode::NoDesktop;
        }
        self.ws_mode(ws)
    }

    /// Mode appliqué à un écran (surfaces, fenêtres) à la dernière synchronisation.
    pub(super) fn applied_mode(&self, name: &str) -> DesktopMode {
        self.applied
            .get(name)
            .map(|(m, _)| *m)
            .unwrap_or(DesktopMode::NoDesktop)
    }

    /// Retient le mode d'un workspace (et l'écrit, hors aperçu).
    fn store_mode(&mut self, ws: &str, mode: DesktopMode) {
        let changed = if mode == DesktopMode::NoDesktop {
            self.ws_modes.remove(ws).is_some()
        } else {
            self.ws_modes.insert(ws.to_string(), mode) != Some(mode)
        };
        if changed && self.persist_modes {
            let mut l = Layout::load();
            l.set_modes(
                self.ws_modes
                    .iter()
                    .map(|(w, m)| (w.clone(), m.as_str().to_string()))
                    .collect(),
            );
            l.save();
        }
    }

    /// Le bureau affiché apparaît-il sur cet écran (`show_on`) ?
    fn shows_on(&self, monitor: &str) -> bool {
        // Écran (ou workspace affiché) imposé en no-desktop : jamais de bureau.
        let ws = self
            .monitors
            .iter()
            .find(|m| m.name == monitor)
            .map(|m| m.mode_workspace())
            .unwrap_or("");
        if self.cfg.general.forced_mode(monitor, ws) == Some("no-desktop") {
            return false;
        }
        // Chaque écran suit le mode du workspace qu'il affiche.
        if self.monitor_mode(monitor) == DesktopMode::NoDesktop {
            return false;
        }
        match self.cfg.general.show_on {
            ShowOn::All => true,
            ShowOn::Focused => self.monitors.iter().any(|m| m.focused && m.name == monitor),
            ShowOn::Primary => self.primary_monitor().as_deref() == Some(monitor),
        }
    }

    /// Aligne les écrans sur leur mode (`sync_modes`).
    fn sync_raised(&mut self) -> Task<Msg> {
        self.sync_modes()
    }

    /// Chaque écran prend le mode du workspace qu'il affiche (MLG-98) :
    /// surface levée ou rendue au fond, fenêtres du workspace mises de côté
    /// (plein bureau) ou rendues, clavier ; le fondu ne joue que quand le
    /// bureau apparaît ou disparaît partout à la fois.
    fn sync_modes(&mut self) -> Task<Msg> {
        use DesktopMode::{FullDesktop as Full, LauncherOnly as Launcher, NoDesktop as No};
        let mut tasks: Vec<Task<Msg>> = Vec::new();
        // Arrivée sur un workspace (ou un écran) à mode imposé : la règle
        // reprend la main ; un changement à la main reste tant qu'on y est.
        for m in self.monitors.clone() {
            if m.workspace.is_empty() || m.workspace.starts_with("special:") {
                continue;
            }
            let arrived = self
                .applied
                .get(&m.name)
                .is_none_or(|(_, ws)| ws != &m.workspace);
            if arrived
                && let Some(w) = self
                    .cfg
                    .general
                    .forced_mode(&m.name, &m.workspace)
                    .and_then(DesktopMode::parse)
                    .filter(|w| *w != No)
            {
                self.store_mode(&m.workspace, w);
            }
        }
        let focused = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .map(|m| m.name.clone());
        let was_visible = self.visible;
        let wanted: Vec<(Id, String, DesktopMode)> = self
            .surfaces
            .iter()
            .map(|(id, mon)| {
                let w = if self.shows_on(mon) {
                    self.monitor_mode(mon)
                } else {
                    No
                };
                (*id, mon.clone(), w)
            })
            .collect();
        let now_visible = wanted.iter().any(|(_, _, w)| *w != No);
        let mut pull_desk = false;
        for (id, mon, want) in &wanted {
            let Some(m) = self.monitors.iter().find(|m| &m.name == mon).cloned() else {
                continue;
            };
            let (had, had_ws) = self
                .applied
                .get(mon)
                .cloned()
                .unwrap_or((No, m.workspace.clone()));
            let ws_changed = had_ws != m.workspace;
            // Workspaces dont les fenêtres reprennent leur configuration.
            let scope: Vec<String> = [had_ws.clone(), m.workspace.clone(), m.base.clone()]
                .into_iter()
                .filter(|w| !w.is_empty())
                .collect();
            match (had, *want) {
                (a, b) if a == b => {
                    if b == Full && ws_changed {
                        // Bureau affiché, on change de workspace : ses fenêtres
                        // flottent à leur tour.
                        tasks.push(self.stash_workspace(&m));
                    }
                    if b != No && ws_changed {
                        self.fire(HookEvent::Workspace, "workspace", &m);
                        self.selected = None;
                    }
                }
                (No, b) => {
                    self.raised.insert(*id);
                    self.fire(HookEvent::Show, "show", &m);
                    if b == Full {
                        tasks.push(self.stash_workspace(&m));
                        pull_desk = true;
                    }
                }
                (a, No) => {
                    let _ = a;
                    self.fire(HookEvent::Hide, "hide", &m);
                    tasks.extend(self.release_desk_windows_in(scope));
                    tasks.push(self.restore_stash(Some(mon)));
                    // Ailleurs le bureau reste : cet écran redescend tout de
                    // suite ; sinon, à la fin du fondu.
                    if now_visible {
                        self.raised.remove(id);
                        tasks.extend(self.lower_surface(*id));
                    }
                    tasks.push(Task::done(Msg::KeyboardInteractivityChange {
                        id: *id,
                        keyboard_interactivity: KeyboardInteractivity::None,
                    }));
                }
                (Launcher, Full) => {
                    tasks.push(self.stash_workspace(&m));
                    pull_desk = true;
                }
                (Full, _) => {
                    // Vers le mode lanceur : les fenêtres du bureau reprennent
                    // leur configuration, les autres reviennent.
                    tasks.extend(self.release_desk_windows_in(scope));
                    tasks.push(self.restore_stash(Some(mon)));
                }
                _ => {}
            }
            // L'écran quitte le mode lanceur : le dock qui tenait le clavier
            // (ouvert au clavier) le rend, sinon aucune fenêtre ne le reçoit.
            if had == Launcher
                && *want != Launcher
                && self
                    .launcher_runtime
                    .focus
                    .as_ref()
                    .is_some_and(|(f, _)| f == mon)
            {
                tasks.push(self.launcher_release());
            }
            self.applied
                .insert(mon.clone(), (*want, m.workspace.clone()));
        }
        // Fondu : seulement quand le bureau apparaît ou disparaît partout.
        if now_visible != was_visible {
            self.visible = now_visible;
            let from = self.fade_value();
            self.fade = Some(Fade {
                from,
                to: if now_visible { 1.0 } else { 0.0 },
                start: Instant::now(),
                dur: self.fade_duration(),
            });
            if now_visible {
                self.selected = None;
                self.last_poll.clear();
                tasks.push(Task::done(Msg::Poll));
            } else {
                self.drag = None;
                self.hovered = None;
                self.session_windows.clear();
                tasks.push(self.finish_fade());
            }
            tasks.push(self.refresh_walls());
        } else if wanted
            .iter()
            .any(|(id, _, w)| *w != No && !self.raised.contains(id))
        {
            tasks.push(self.refresh_walls());
        }
        // Fenêtres en attente du bureau : elles reviennent en plein bureau.
        if pull_desk {
            tasks.extend(self.bring_back_desk_windows());
        }
        // Clavier : l'écran actif seulement ; exclusif sur un bureau vide.
        for (id, mon, want) in &wanted {
            if *want == No || !self.raised.contains(id) {
                continue;
            }
            let kb = if Some(mon) != focused.as_ref() {
                KeyboardInteractivity::None
            } else if *want == Launcher
                || !self.session_windows.is_empty()
                || self.windows_shown_on(mon)
            {
                KeyboardInteractivity::OnDemand
            } else {
                KeyboardInteractivity::Exclusive
            };
            tasks.push(Task::done(Msg::KeyboardInteractivityChange {
                id: *id,
                keyboard_interactivity: kb,
            }));
        }
        tasks.push(self.sync_submap());
        tasks.push(self.sync_hyprbars());
        tasks.push(self.sync_window_gaps());
        self.publish_status();
        Task::batch(tasks)
    }

    // Le bureau est la RACINE des fenêtres : il reste toujours dessous. En
    // plein bureau, les fenêtres du workspace flottent par-dessus, organisées,
    // et celles qu'on ouvre depuis le bureau s'ouvrent dessus. Le mode verre
    // n'est qu'un effet sur le fond (fond d'écran flouté et teinté).

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

    /// Plein bureau sur cet écran (MLG-55) : les fenêtres du workspace
    /// affiché passent en flottant par-dessus le bureau, à la place qu'elles
    /// avaient la dernière fois sur ce workspace, sinon organisées dans la
    /// zone utile ; elles font partie du bureau (raccourcis actifs) jusqu'à
    /// la sortie du mode, où elles reprennent leur configuration d'origine.
    fn stash_workspace(&mut self, m: &Monitor) -> Task<Msg> {
        if m.workspace.is_empty() || m.workspace.starts_with("special:") {
            return Task::none();
        }
        for c in self.clients.iter().filter(|c| c.workspace == m.workspace) {
            self.session_windows.insert(c.address.clone());
            self.desk_windows.insert(c.address.clone());
        }
        const MARGIN: f32 = 24.0;
        let (l, t, r, b) = m.reserved;
        let area = (
            (m.x + l + MARGIN) as i32,
            (m.y + t + MARGIN) as i32,
            (m.width as f32 - l - r - 2.0 * MARGIN) as i32,
            (m.height as f32 - t - b - 2.0 * MARGIN) as i32,
        );
        Self::window_result(self.windows.arrange(m.workspace.clone(), area))
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
            .flat_map(|b| {
                let zones = if self.bar_vertical(&self.bar_surfaces[&bar_id].monitor) {
                    b.portrait.as_ref().unwrap_or(&b.zones)
                } else {
                    &b.zones
                };
                zones.iter().flatten().cloned()
            })
            .collect();
        for it in items {
            let req = match &it {
                config::BarItem::Workspaces if kind == "workspaces" => {
                    Some(("workspaces".into(), popover::Request::Workspaces))
                }
                config::BarItem::Volume if kind == "volume" => {
                    Some(("volume".to_string(), popover::Request::Volume))
                }
                config::BarItem::Brightness if matches!(kind, "brightness" | "luminosite") => {
                    Some(("brightness".to_string(), popover::Request::Brightness))
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
                        config::WidgetKind::Controls => {
                            Some((id.clone(), popover::Request::Widget(id.clone())))
                        }
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
        // SmartUI design : les nouvelles applications du lanceur reçoivent leur icône.
        let auto = self.smartui_auto(missing.clone());
        let resolve = Task::perform(
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
        );
        Task::batch([resolve, auto])
    }

    /// Relit toutes les icônes (bureau, barre, lanceur, notifications) :
    /// thème changé, SmartUI activé, ou une icône SmartUI ajoutée.
    /// Icônes du bureau, pour SmartUI : identifiant `.desktop` d'une
    /// application, nom d'icône d'une commande, d'un fichier ou d'un dossier
    /// (dossier personnel, dossier, type de fichier) : tout le bureau au même style.
    pub(super) fn smartui_desktop_names(&self) -> Vec<String> {
        self.cfg
            .icons
            .iter()
            .filter_map(|d| match &d.kind {
                crate::config::IconKind::App(id) => Some(id.clone()),
                crate::config::IconKind::Exec { .. } => d
                    .icon
                    .clone()
                    .filter(|n| !n.contains('/') && !crate::smartui::is_symbolic(n)),
                crate::config::IconKind::Path(p) => Some(
                    d.icon
                        .clone()
                        .unwrap_or_else(|| launch::path_icon_name(p).to_string()),
                )
                .filter(|n| !n.contains('/') && !crate::smartui::is_symbolic(n)),
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub(super) fn reload_icons(&mut self) -> Task<Msg> {
        self.icons.clear();
        self.icon_imgs.clear();
        self.selected = None;
        self.icons_gen += 1;
        let (gen_, cfg) = (self.icons_gen, self.cfg.clone());
        Task::perform(
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
        )
    }

    fn monitor(&self, id: Id) -> Option<&Monitor> {
        let name = self.surfaces.get(&id)?;
        self.monitors.iter().find(|m| &m.name == name)
    }

    fn desktop_of(&self, id: Id) -> Option<(&Monitor, Desktop<'_>)> {
        let m = self.monitor(id)?;
        let mut d = resolve(&self.cfg, &m.name, &m.workspace);
        self.keep_widgets_of(&m.name, &mut d);
        Some((m, d))
    }

    /// Plusieurs écrans : chaque widget sur son écran (principal par défaut,
    /// ou l'écran choisi, et l'écran principal ou rien s'il est débranché).
    fn keep_widgets_of(&self, monitor: &str, d: &mut Desktop<'_>) {
        let primary = self.primary_monitor();
        let connected: Vec<&str> = self.monitors.iter().map(|m| m.name.as_str()).collect();
        d.widgets
            .retain(|w| w.on_screen(monitor, primary.as_deref(), &connected));
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
                .is_some_and(|p| p.animating(Instant::now()))
            || self
                .overview
                .as_ref()
                .is_some_and(|o| o.animating(Instant::now()))
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
        self.bar_drag = None;
        // Capture en cours : Hyprland retrouve ses raccourcis.
        let end = if self
            .panel
            .as_ref()
            .is_some_and(|p| p.keys.capture.is_some())
        {
            self.keys_capture_end()
        } else {
            Task::none()
        };
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
        Task::batch([end, Task::done(Msg::Frame)])
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

    pub(super) fn publish_status(&self) {
        let screens: Vec<String> = self
            .monitors
            .iter()
            .map(|m| format!("{}:{}", m.name, m.workspace))
            .collect();
        control::set_status(crate::tr!(
            "{mode} (pid {pid}) — écrans {screens} — config {state} ({path})\nagent polkit : {polkit}",
            mode = self.mode().as_str(),
            pid = std::process::id(),
            screens = screens.join(" "),
            state = match &self.config_error {
                None => "ok".to_string(),
                Some(e) => crate::tr!("en erreur : {e}", e = e),
            },
            path = self.config_path.display(),
            polkit = crate::polkit::state().text()
        ));
    }

    // -- Affichage / masquage ------------------------------------------------

    /// Plein bureau sur le workspace de l'écran visé.
    fn show(&mut self) -> Task<Msg> {
        self.set_mode(DesktopMode::FullDesktop)
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

    /// Mode du workspace de l'écran visé (celui du clic, sinon l'actif) ;
    /// les autres écrans gardent le leur.
    fn set_mode(&mut self, mode: DesktopMode) -> Task<Msg> {
        let Some(ws) = self
            .target_monitor()
            .and_then(|n| self.monitors.iter().find(|m| m.name == n))
            .map(|m| m.mode_workspace().to_string())
            .filter(|w| !w.is_empty())
        else {
            return Task::none();
        };
        self.store_mode(&ws, mode);
        self.sync_modes()
    }

    /// Bureau affiché sous les fenêtres, qui restent (ou reviennent) en place.
    fn show_with_windows(&mut self) -> Task<Msg> {
        self.set_mode(DesktopMode::LauncherOnly)
    }

    fn hide(&mut self) -> Task<Msg> {
        self.set_mode(DesktopMode::NoDesktop)
    }

    /// Sortie du plein bureau (bureau masqué ou mode lanceur) : les fenêtres
    /// du bureau reprennent leur configuration d'origine — en mosaïque sur
    /// leur workspace si le plein bureau les avait fait flotter — et restent
    /// visibles, comme des fenêtres ordinaires (MLG-55).
    fn release_desk_windows_in(&mut self, workspaces: Vec<String>) -> Vec<Task<Msg>> {
        let here: Vec<String> = self
            .desk_windows
            .iter()
            .filter(|a| {
                self.clients
                    .iter()
                    .find(|c| &c.address == *a)
                    .is_none_or(|c| workspaces.contains(&c.workspace))
            })
            .cloned()
            .collect();
        for a in here {
            self.desk_windows.remove(&a);
            self.session_windows.remove(&a);
        }
        vec![Self::window_result(
            self.windows.release_workspaces(workspaces),
        )]
    }

    /// Taille d'une fenêtre flottante sur le bureau : 60 % × 70 % de l'écran actif.
    fn desk_window_size(&self) -> (u32, u32) {
        self.monitors
            .iter()
            .find(|m| m.focused)
            .or_else(|| self.monitors.first())
            .map(|m| {
                (
                    (m.width as f32 * 0.6) as u32,
                    (m.height as f32 * 0.7) as u32,
                )
            })
            .unwrap_or((1200, 800))
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
        // Un espace redevenu actif sur son écran montre ses fenêtres urgentes :
        // leur losange quitte la barre.
        if !self.urgent.is_empty() {
            self.urgent.retain(|a| {
                !self.clients.iter().any(|c| {
                    &c.address == a
                        && self
                            .monitors
                            .iter()
                            .any(|m| m.name == c.monitor && m.workspace == c.workspace)
                })
            });
        }
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
        tasks.push(self.refresh_walls());
        tasks.push(self.sync_bars());
        tasks.push(self.sync_lyra_drawer());
        let names = |l: &[Monitor]| l.iter().map(|m| m.name.clone()).collect::<Vec<_>>();
        // Écrans changés : placement à refaire, et workspaces d'un écran parti
        // (règles d'autres sources comprises) à reloger.
        if names(&old) != names(&self.monitors) && self.cfg.plugin_on("workspaces") {
            tasks.push(self.install_rules());
        }
        // Chaque écran prend le mode du workspace qu'il affiche.
        tasks.push(self.sync_modes());
        if self.visible {
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

    /// Menu du tray ouvert : relu toutes les 2 s (LayoutUpdated, cases cochées…).
    fn refresh_tray_menu(&mut self) -> Task<Msg> {
        let now = Instant::now();
        if let Some(p) = self.popover.as_mut()
            && let popover::Kind::Menu {
                source: popover::MenuSource::Tray { bus, menu_path },
                ..
            } = &p.kind
            && now.duration_since(p.refreshed) >= Duration::from_secs(2)
        {
            p.refreshed = now;
            let (bus, path, key) = (bus.clone(), menu_path.clone(), p.key.clone());
            return Task::perform(
                async move { crate::tray::menu(&bus, &path).await },
                move |r| match r {
                    Ok(e) => Msg::MenuRefreshed(key.clone(), e),
                    Err(_) => Msg::Noop,
                },
            );
        }
        Task::none()
    }

    fn poll(&mut self) -> Task<Msg> {
        let refresh = self.refresh_tray_menu();
        // Minimaps vivantes (écran d'organisation, miniatures du sélecteur) :
        // les événements fenêtres rafraîchissent, un rappel périodique les
        // garde à jour même si un événement a été manqué.
        let fresh = if self.overview.is_some()
            || self.cfg.general.workspace_selector == crate::config::WsSelector::Miniatures
        {
            let now = Instant::now();
            let stale = self
                .ws_geos_at
                .is_none_or(|at| now.duration_since(at) >= Duration::from_secs(5));
            if stale {
                self.ws_geos_at = Some(now);
                Task::perform(crate::hypr::window_geometries_async(), |l| {
                    Msg::Overview(crate::ui::overview::OMsg::Geos(l))
                })
            } else {
                Task::none()
            }
        } else {
            Task::none()
        };
        Task::batch([refresh, fresh, self.poll_widgets()])
    }

    fn poll_widgets(&mut self) -> Task<Msg> {
        let now = Instant::now();
        // Dossier Bureau en icônes : relu quand il change (toutes les 2 s au plus).
        if self.cfg.general.desktop_files
            && self
                .desk_stamp
                .is_none_or(|(_, at)| now.duration_since(at) >= Duration::from_secs(2))
        {
            let s = crate::desktop_files::stamp();
            let changed = self.desk_stamp.is_some_and(|(old, _)| old != s);
            self.desk_stamp = Some((s, now));
            if changed {
                return Task::done(Msg::ConfigChanged);
            }
        }
        let mut seen = HashSet::new();
        let mut tasks = Vec::new();
        let raised: Vec<Id> = self.raised.iter().copied().collect();
        let cfg = self.cfg.clone();
        // Widgets à relever : ceux des bureaux affichés, et ceux de la barre (toujours visible).
        let mut todo: Vec<&config::WidgetDef> = Vec::new();
        for id in raised.into_iter().filter(|_| cfg.plugin_on("widgets")) {
            let Some(m) = self.monitor(id) else { continue };
            let mut d = resolve(&cfg, &m.name, &m.workspace);
            self.keep_widgets_of(&m.name, &mut d);
            todo.extend(d.widgets);
        }
        let bar_ids = self.bar_widget_ids();
        todo.extend(cfg.widgets.iter().filter(|w| bar_ids.contains(&w.id)));
        let controls_visible = todo.iter().any(|w| matches!(w.kind, WidgetKind::Controls));
        if controls_visible {
            todo.extend(
                cfg.widgets
                    .iter()
                    .filter(|w| crate::widgets::controls::SOURCES.contains(&w.type_id.as_str())),
            );
        }
        if (self.bar_has(&config::BarItem::Volume) || controls_visible)
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
        if (self.bar_has(&config::BarItem::Brightness) || controls_visible)
            && !self.pending_widgets.contains("bar:light")
            && self
                .last_poll
                .get("bar:light")
                .is_none_or(|t| now.duration_since(*t) >= Duration::from_secs(2))
        {
            self.pending_widgets.insert("bar:light".into());
            self.last_poll.insert("bar:light".into(), now);
            // Écrans DDC/CI : relus toutes les 30 s (ddcutil est lent).
            let known = self
                .light_ddc_at
                .is_some_and(|t| now.duration_since(t) < Duration::from_secs(30))
                .then(|| self.light.displays.clone());
            if known.is_none() {
                self.light_ddc_at = Some(now);
            }
            tasks.push(Task::perform(bar::read_light(known), Msg::LightValue));
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
                    WidgetKind::Clock { .. }
                    | WidgetKind::Calendar
                    | WidgetKind::Controls
                    | WidgetKind::Launcher => None,
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
            // Icône SmartUI : relue en mémoire, une icône revalidée au même
            // chemin ne doit pas reprendre l'ancienne image en cache.
            Some(p) if crate::smartui::owns(p) => match std::fs::read(p) {
                Ok(bytes) => {
                    self.vectors.push(svg::Handle::from_memory(bytes));
                    IconImg::Vector(self.vectors.len() - 1)
                }
                Err(_) => IconImg::None,
            },
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
        self.launch_from(None, idx)
    }

    /// Lance l'icône `idx` du bureau `desk` : la fenêtre s'ouvre sur le
    /// workspace de cet écran (pas sur celui qui a le focus, souvent
    /// l'autre écran), donc dans son dock. Le bureau garde son mode.
    pub(super) fn launch_from(&mut self, desk: Option<Id>, idx: usize) -> Task<Msg> {
        let Some(r) = self.icons.get(idx) else {
            return Task::none();
        };
        let cmd = r.command.clone();
        let target = desk.and_then(|d| self.monitor(d)).cloned();
        self.run_from_desktop_on(cmd, true, target.as_ref(), |_| Msg::Noop)
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
        self.run_from_desktop_on(cmd, stays, None, then)
    }

    /// `target` : écran du bureau d'où l'on lance (son workspace reçoit la
    /// fenêtre, sa taille la dimensionne) ; sinon l'écran qui a le focus.
    pub(super) fn run_from_desktop_on(
        &self,
        cmd: String,
        stays: bool,
        target: Option<&Monitor>,
        then: impl FnOnce(bool) -> Msg + Send + 'static,
    ) -> Task<Msg> {
        let screen = target.or_else(|| self.monitors.iter().find(|m| m.focused));
        let size = (stays && self.cfg.general.float_launched)
            .then(|| {
                screen.map(|m| {
                    (
                        (m.width as f32 * 0.6) as u32,
                        (m.height as f32 * 0.7) as u32,
                    )
                })
            })
            .flatten();
        let ws = target.map(|m| m.workspace.clone());
        tracing::info!("{}", crate::tr!("lancement : {cmd}", cmd = cmd));
        Task::perform(
            async move {
                let r = tokio::task::spawn_blocking(move || match (size, ws) {
                    (Some((w, h)), ws) => hypr::exec_floating(&cmd, w, h, ws.as_deref()),
                    (None, Some(ws)) => hypr::exec_on(&cmd, &ws),
                    (None, None) => hypr::exec(&cmd),
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
            (keyboard::Key::Named(Named::Enter), Some((i, _))) => {
                return self.launch_from(Some(id), i);
            }
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
                let sel = self
                    .monitor(win)
                    .map(|m| self.cfg.general.icon_scope.selector(&m.name, &m.workspace))
                    .unwrap_or_default();
                let Some(id) = self.settings.add_dropped(&path, sel) else {
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
        let t = Task::batch([t, self.sync_hyprbars()]);
        tracing::trace!(
            message = name,
            update_us = started.elapsed().as_micros() as u64,
            "UI update"
        );
        t
    }

    fn update_inner(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::IndexStatus(st) => {
                self.index_status = st.clone();
                if let Some(p) = self.panel.as_mut() {
                    p.index.status = st;
                }
                Task::none()
            }
            Msg::Control(c) => match c {
                // Le raccourci d'un mode y mène, ou rend no-desktop s'il y est déjà.
                control::Command::Toggle => {
                    if self.mode() == DesktopMode::FullDesktop {
                        self.hide()
                    } else {
                        self.show()
                    }
                }
                // Ouvert au clavier, le dock prend le clavier (première case
                // choisie) : aucune souris nécessaire pour s'en servir.
                control::Command::ToggleLauncher => {
                    if self.mode() == DesktopMode::LauncherOnly {
                        self.hide()
                    } else {
                        let grab = self.launcher_grab_focused();
                        Task::batch([self.show_with_windows(), grab])
                    }
                }
                control::Command::Show => self.show(),
                control::Command::Hide => self.hide(),
                // Au clavier (raccourci, hyprdesk launcher-only), le dock prend
                // le clavier ; d'un clic (barre), il le laisse aux fenêtres.
                control::Command::LauncherOnly => {
                    if self.mode() == DesktopMode::LauncherOnly || self.click_monitor.is_some() {
                        return self.show_with_windows();
                    }
                    let grab = self.launcher_grab_focused();
                    Task::batch([self.show_with_windows(), grab])
                }
                // Mode suivant (SUPER + D, MLG-108) : au clavier, le dock
                // prend le clavier en launcher-only, comme SUPER + ALT + D.
                control::Command::Cycle => match self.mode().next() {
                    DesktopMode::LauncherOnly if self.click_monitor.is_none() => {
                        let grab = self.launcher_grab_focused();
                        Task::batch([self.show_with_windows(), grab])
                    }
                    m => self.set_mode(m),
                },
                control::Command::Reload => self.load_config(),
                control::Command::Quit => {
                    if self.in_submap {
                        drop(self.windows.submap(false));
                    }
                    // Rien ne doit rester de côté après l'arrêt, ni caché avec le
                    // bureau, ni flottant à cause de lui.
                    Self::window_result(self.windows.release())
                        .chain(Self::window_result(self.windows.unpark()))
                        .chain(self.restore_stash(None))
                        .chain(Task::perform(
                            async {
                                crate::session::stop_all().await;
                                let _ = tokio::task::spawn_blocking(hypr::remove_keys).await;
                                let _ = tokio::task::spawn_blocking(hypr::remove_rules).await;
                                let _ = tokio::task::spawn_blocking(|| {
                                    crate::lid::install_binds(&Default::default())
                                })
                                .await;
                                let _ = tokio::task::spawn_blocking(|| hypr::set_desktop_gaps(&[]))
                                    .await;
                            },
                            |_| Msg::Noop,
                        ))
                        .chain(iced::exit())
                }
                control::Command::Status => Task::none(),
                control::Command::ColorTest => {
                    self.colors_update(if self.color_test.wins.is_empty() {
                        plugins::colors::Msg::TestOpen
                    } else {
                        plugins::colors::Msg::TestClose
                    })
                }
                control::Command::ForgetWindowPlaces => Self::window_result(self.windows.forget()),
                control::Command::Problem(t) => self.show_problem(t, false),
                control::Command::ProblemShow(t) => self.show_problem(t, true),
                control::Command::Osd(line) => match osd::parse(&line) {
                    Some((kind, value, muted)) => self.show_osd(kind, value, muted),
                    None => Task::none(),
                },
                control::Command::Menu(kind) => self.open_menu_by_type(&kind),
                control::Command::Assistant => self.open_assistant(),
                // Visite guidée : les réglages s'ouvrent, puis la première étape.
                control::Command::Search => self.toggle_search(),
                control::Command::Files => self.open_search_with(":f "),
                control::Command::Mymfa(a) => match a.as_str() {
                    "lock" => self.mfa_update(mymfa::MMsg::Lock),
                    "setup" => self.mfa_update(mymfa::MMsg::Setup(None)),
                    "open" => self.mfa_fenetre_open(),
                    "codes" => self.open_search_with(":2fa "),
                    "logins" => self.open_search_with(":mdp "),
                    _ => self.mfa_update(mymfa::MMsg::Ask),
                },
                control::Command::Apps => self.toggle_apps(),
                control::Command::Ask(origin, text) => self.show_problem_from(&origin, text, true),
                control::Command::Notifications(a) => {
                    self.toasts.command(&a);
                    Task::none()
                }
                control::Command::Overview => {
                    let cmd = self.cfg.general.overview_command.trim().to_string();
                    if cmd.is_empty() {
                        // SUPER+Tab : l'écran « Espaces de travail » (Alt+Tab
                        // garde le sélecteur de fenêtres).
                        if self.overview.is_some() {
                            return self.close_overview();
                        }
                        self.open_overview()
                    } else {
                        Task::perform(
                            async move {
                                let _ = tokio::task::spawn_blocking(move || hypr::exec(&cmd)).await;
                            },
                            |_| Msg::Noop,
                        )
                    }
                }
                control::Command::Switch(back) => self.switch_windows(back),
                control::Command::Tour => {
                    let o = self.open_panel();
                    Task::batch([
                        o,
                        Task::done(Msg::Panel(PanelMsg::Tour(tour::TourMsg::Start))),
                    ])
                }
                // Les réglages s'ouvrent sans changer le mode du bureau.
                control::Command::Settings(tab) if tab.starts_with('?') => {
                    // « ?mots » : Réglages ouverts sur la recherche.
                    let o = self.open_panel();
                    let q = tab[1..].trim().to_string();
                    Task::batch([o, Task::done(Msg::Panel(PanelMsg::Search(q)))])
                }
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
            Msg::Hypr(hypr::Update::Urgent(addr)) => {
                // La relecture des fenêtres suit dans le flux : la barre saura
                // où placer son losange.
                self.urgent.insert(addr);
                Task::none()
            }
            Msg::BarWorkspace(mon, ws) => {
                let focus = Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            hypr::focus_monitor(&mon);
                            hypr::focus_workspace(&ws)
                        })
                        .await;
                    },
                    |_| Msg::Noop,
                );
                Task::batch([focus, self.close_popover()])
            }
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
            Msg::LauncherActivate(msg) => self.launcher_activate(*msg),
            Msg::OpenAssistant => self.open_assistant(),
            Msg::AskLyra(origin, text) => {
                let close = if self.panel.is_some() {
                    self.close_panel()
                } else {
                    Task::none()
                };
                Task::batch([close, self.show_problem_from(&origin, text, true)])
            }
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
                let idle = self.cfg.general.idle.clone();
                let bars = self.hyprbars_on;
                let (fg, bg) = (self.cfg.general.theme.fg, self.cfg.general.theme.bg);
                let tagged: Vec<String> = self.nobar.iter().cloned().collect();
                let mut t = vec![
                    // Hyprbars oublie ses boutons à chaque relecture.
                    Task::perform(
                        async move {
                            // La règle « sans barre » revient avec les boutons :
                            // étiquettes reposées pour la réévaluation.
                            if bars
                                && let Ok(Err(e)) = tokio::task::spawn_blocking(move || {
                                    let r = crate::hyprbars::add_buttons(fg, bg);
                                    crate::hyprbars::set_tags(&tagged, &[], true);
                                    r
                                })
                                .await
                            {
                                tracing::warn!("{}", e);
                            }
                        },
                        |_| Msg::Noop,
                    ),
                    self.install_keys(),
                    self.install_rules(),
                    Task::perform(
                        async move {
                            let _ = tokio::task::spawn_blocking(move || {
                                crate::lid::install_binds(&idle)
                            })
                            .await;
                        },
                        |_| Msg::Noop,
                    ),
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
                    Ok(report) => {
                        // Conflits nouveaux : dits une fois, à l'installation.
                        let fresh: Vec<String> = report
                            .issues
                            .iter()
                            .filter(|i| !self.key_report.issues.contains(i))
                            .map(|i| i.message())
                            .collect();
                        if !fresh.is_empty() {
                            tracing::warn!(
                                "{}",
                                crate::tr!("raccourcis en conflit : {p0}", p0 = fresh.join(" ; "))
                            );
                            hypr::notify(
                                0,
                                10000,
                                &crate::tr!(
                                    "hyprdesk : raccourcis en conflit (Réglages → Raccourcis, ou hyprdesk keys) — {p0}",
                                    p0 = fresh.join(" ; ")
                                ),
                            );
                        }
                        self.key_report = report;
                    }
                    Err(e) => tracing::warn!("{}", crate::tr!("raccourcis : {e}", e = e)),
                }
                Task::none()
            }
            Msg::Hypr(hypr::Update::Clients(list)) => {
                let mut tasks = Vec::new();
                // Bureau vide seulement : avec les fenêtres, une nouvelle fenêtre
                // est une fenêtre ordinaire (mise de côté si on revient au bureau vide).
                if self.visible {
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
                                    && self.applied_mode(n) == DesktopMode::FullDesktop
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
                        // Ouverte sur le bureau : flottante et centrée (déjà le cas
                        // si hyprdesk l'a lancée) ; en mosaïque à la sortie.
                        tasks.push(Self::window_result(self.windows.float(
                            vec![c.address.clone()],
                            self.desk_window_size(),
                            true,
                        )));
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
                // Une fenêtre fermée n'est plus urgente.
                self.urgent.retain(|a| list.iter().any(|c| &c.address == a));
                tasks.push(self.sync_submap());
                self.clients = list;
                // Minimap vivante : fenêtres ouvertes, fermées, déplacées.
                if self.overview.is_some()
                    || self.cfg.general.workspace_selector == crate::config::WsSelector::Miniatures
                {
                    tasks.push(Task::perform(crate::hypr::window_geometries_async(), |l| {
                        Msg::Overview(crate::ui::overview::OMsg::Geos(l))
                    }));
                }
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
                    // Ramenée sur le bureau : flottante par-dessus lui, puis
                    // rendue à sa configuration d'origine à la sortie.
                    self.desk_windows.insert(addr.clone());
                    let mut t = vec![
                        Self::window_result(self.windows.restore_window(&addr)),
                        Self::window_result(self.windows.float(
                            vec![addr.clone()],
                            self.desk_window_size(),
                            false,
                        )),
                    ];
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
            Msg::OpenPanel => self.toggle_panel(),
            Msg::On(monitor, inner) => {
                self.click_monitor = Some(monitor);
                let t = self.update_inner(*inner);
                self.click_monitor = None;
                t
            }
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
                self.bar_hidden.clear();
                self.bar_full_width.clear();
                self.bar_anchors.clear();
                self.control_actions.clear();
                self.values.clear();
                self.last_poll.clear();
                self.menu_request += 1;
                let close = self.destroy_popover();
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
                // SmartUI design : les applications du bureau reçoivent aussi leur
                // icône (chacune essayée une fois par session).
                let desk_apps = self.smartui_desktop_names();
                let smart = self.smartui_auto(desk_apps);
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
                again.push(smart);
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
            Msg::Mymfa(m) => self.mfa_update(m),
            Msg::MymfaF(m) => self.mfa_fenetre_update(m),
            Msg::Toasts(m) => self.toasts_update(m),
            Msg::Problem(m) => self.problem_update(m),
            Msg::Search(m) => self.search_update(m),
            Msg::IconContext(win, idx) => {
                self.drag = None;
                self.open_icon_menu(win, idx)
            }
            Msg::WidgetContext(win, id) => {
                self.wdrag = None;
                self.open_widget_menu(win, id)
            }
            Msg::DeskContext(win) => {
                self.drag = None;
                self.open_desk_menu(win)
            }
            Msg::IconMenu(m) => self.icon_menu_update(m),
            Msg::Switcher(m) => self.switcher_update(m),
            Msg::Overview(m) => self.overview_update(m),
            Msg::BarOrganize => Task::batch([self.close_popover(), self.open_overview()]),
            Msg::MenuRefreshed(key, new) => {
                if let Some(p) = self.popover.as_mut()
                    && p.key == key
                    && let popover::Kind::Menu {
                        entries,
                        source: popover::MenuSource::Tray { .. },
                    } = &mut p.kind
                    && *entries != new
                {
                    *entries = new;
                }
                Task::none()
            }
            Msg::CalendarMonth(d) => {
                self.cal_months = if d == 0 {
                    0
                } else {
                    (self.cal_months + d).clamp(-1200, 1200)
                };
                Task::none()
            }
            Msg::Modifiers(m) => {
                self.mods = m;
                if self.switcher.is_some() && !m.alt() && !m.logo() && !m.control() {
                    return self.switcher_commit();
                }
                Task::none()
            }
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
                Task::batch([
                    closed,
                    launcher,
                    self.popover_frame(),
                    self.overview_frame(),
                    self.finish_fade(),
                ])
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
                let toggling = self
                    .popover
                    .as_ref()
                    .is_some_and(|p| p.bar == bar_id && p.key == key && !p.closing());
                self.menu_request += 1;
                let request = self.menu_request;
                if toggling {
                    return self.close_popover();
                }
                match req {
                    // Ouverture immédiate ; le relevé des périphériques affine
                    // ensuite la carte via la mesure de son contenu.
                    popover::Request::Widget(widget) => {
                        if !self
                            .cfg
                            .widgets
                            .iter()
                            .any(|w| w.id == widget && matches!(w.kind, WidgetKind::Controls))
                        {
                            return Task::none();
                        }
                        Task::batch([
                            Task::done(Msg::PopoverReady(
                                request,
                                bar_id,
                                key,
                                popover::Kind::Widget(widget),
                            )),
                            Task::perform(bar::read_audio(), Msg::AudioValue),
                        ])
                    }
                    popover::Request::Volume => Task::batch([
                        Task::done(Msg::PopoverReady(
                            request,
                            bar_id,
                            key,
                            popover::Kind::Volume,
                        )),
                        Task::perform(bar::read_audio(), Msg::AudioValue),
                    ]),
                    popover::Request::Overflow => Task::done(Msg::PopoverReady(
                        request,
                        bar_id,
                        key,
                        popover::Kind::Overflow(self.bar_overflow_items(bar_id)),
                    )),
                    popover::Request::Workspaces => Task::done(Msg::PopoverReady(
                        request,
                        bar_id,
                        key,
                        popover::Kind::Workspaces,
                    )),
                    popover::Request::Brightness => Task::done(Msg::PopoverReady(
                        request,
                        bar_id,
                        key,
                        popover::Kind::Brightness,
                    )),
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
                }
            }
            Msg::PopoverReady(request, bar_id, key, kind) => {
                if request != self.menu_request {
                    return Task::none();
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
            Msg::BarDragStart(z, i) => {
                let b = self
                    .settings
                    .bar
                    .get_or_insert_with(|| crate::settings::BarSettings::from_config(&self.cfg));
                self.bar_drag = b.can_drag_entry((z, i)).then_some(((z, i), None));
                Task::none()
            }
            Msg::BarAnchor(id, key, bounds) => {
                self.bar_anchors.insert((id, key), bounds);
                self.refresh_popover_anchor()
            }
            Msg::BarLayout(id, hidden, full) => {
                self.bar_hidden.insert(id, hidden);
                if full > 0 {
                    self.bar_full_width.insert(id, full);
                }
                self.refresh_popover_anchor()
            }
            Msg::ControlToggle(id) => {
                if self.control_actions.get(&id).is_some_and(Option::is_none) {
                    return Task::none();
                }
                let Some(config::WidgetKind::Lua {
                    toggle: Some(func), ..
                }) = self
                    .cfg
                    .widgets
                    .iter()
                    .find(|w| w.id == id)
                    .map(|w| &w.kind)
                else {
                    return Task::none();
                };
                let (lua, func, generation) = (self.lua.clone(), *func, self.widget_generation);
                self.control_actions.insert(id.clone(), None);
                Task::perform(async move { lua.call(func, None).await }, move |r| {
                    Msg::ControlToggled(generation, id.clone(), r)
                })
            }
            Msg::ControlToggled(generation, id, result) => {
                if generation != self.widget_generation {
                    return Task::none();
                }
                match result {
                    Ok(()) => {
                        self.control_actions.remove(&id);
                    }
                    Err(error) => {
                        self.control_actions.insert(id.clone(), Some(error));
                    }
                }
                self.last_poll.remove(&id);
                self.poll()
            }
            Msg::BarDragOver(z, i) => {
                if let Some((from, over)) = self.bar_drag.as_mut() {
                    *over = self
                        .settings
                        .bar
                        .as_ref()
                        .filter(|b| b.can_drop_entry(*from, (z, i)))
                        .map(|_| (z, i));
                }
                Task::none()
            }
            Msg::BarDragLeave(z) => {
                if let Some((_, over)) = self.bar_drag.as_mut()
                    && over.is_some_and(|(zone, _)| z == usize::MAX || zone == z)
                {
                    *over = None;
                }
                Task::none()
            }
            Msg::BarDragCancel => {
                self.bar_drag = None;
                Task::none()
            }
            Msg::BarDragEnd => {
                if self.finish_bar_drag() {
                    self.commit_reload()
                } else {
                    Task::none()
                }
            }
            Msg::LightValue(l) => {
                self.pending_widgets.remove("bar:light");
                self.light = l;
                Task::none()
            }
            Msg::LightSet(t, v) => {
                self.light.set_level(t, v);
                Task::none()
            }
            Msg::LightApply(t) => {
                let v = self.light.level(t).unwrap_or(0.0);
                match self.light.set_level(t, v) {
                    Some(raw) => Task::perform(
                        bar::apply_light(self.light.clone(), t, raw),
                        Msg::LightValue,
                    ),
                    None => Task::none(),
                }
            }
            Msg::LightStep(t, d) => {
                let v = self.light.level(t).unwrap_or(0.0) + d as f32 / 100.0;
                // L'écran ne descend jamais au noir complet par la molette.
                let floor = if t == bar::LightTarget::Kbd {
                    0.0
                } else {
                    0.01
                };
                match self.light.set_level(t, v.max(floor)) {
                    Some(raw) => Task::perform(
                        bar::apply_light(self.light.clone(), t, raw),
                        Msg::LightValue,
                    ),
                    None => Task::none(),
                }
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
                if self.problem.as_ref().is_some_and(|p| p.win == id) {
                    self.problem = None;
                    return Task::none();
                }
                if self.search.as_ref().is_some_and(|s| s.win == id) {
                    self.search = None;
                    return Task::none();
                }
                if self.switcher.as_ref().is_some_and(|s| s.win == id) {
                    self.switcher = None;
                    return Task::none();
                }
                if self
                    .icon_menu
                    .as_ref()
                    .is_some_and(|m| m.win == id || m.catcher == id)
                {
                    return self.close_icon_menu();
                }
                if self.mfa_dialog.as_ref().is_some_and(|d| d.win == id) {
                    if let Some(mut d) = self.mfa_dialog.take() {
                        d.wipe();
                    }
                    return Task::none();
                }
                if self.mfa_fenetre.as_ref().is_some_and(|f| f.win == id) {
                    self.mfa_fenetre_oublier();
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
                    return self.destroy_popover();
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
                        let mut np = d.from.clone();
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
            // Écran « Espaces de travail » : clic ou dépôt d'une fenêtre glissée.
            Msg::Released if self.overview.as_ref().is_some_and(|o| o.press.is_some()) => {
                self.overview_release()
            }
            Msg::Released if self.wdrag.is_some() => {
                let moved = self.wdrag.take().is_some_and(|d| d.moved);
                if moved {
                    self.panel_update(PanelMsg::Commit)
                } else {
                    Task::none()
                }
            }
            Msg::Released if self.bar_drag.is_some() => self.update_inner(Msg::BarDragEnd),
            Msg::Released => match self.drag.take() {
                Some(d) if d.active => {
                    self.drop_icon(d.win, d.icon);
                    Task::none()
                }
                // Comme sur un bureau classique : un clic sélectionne, un
                // double clic ouvre (ou un clic, selon `icon_double_click`).
                Some(d) => {
                    let now = Instant::now();
                    let double = self.last_click.is_some_and(|(w, i, at)| {
                        w == d.win && i == d.icon && now.duration_since(at) < DOUBLE_CLICK
                    });
                    if !self.cfg.general.icon_double_click || double {
                        self.last_click = None;
                        self.launch_from(Some(d.win), d.icon)
                    } else {
                        self.last_click = Some((d.win, d.icon, now));
                        Task::none()
                    }
                }
                // Clic ailleurs que sur une icône : plus de sélection.
                None => {
                    self.selected = None;
                    Task::none()
                }
            },
            Msg::Key(id, key) => {
                if self.bar_drag.is_some()
                    && key == keyboard::Key::Named(keyboard::key::Named::Escape)
                {
                    self.bar_drag = None;
                    return Task::none();
                }
                // Capture d'un raccourci (Réglages → Raccourcis) : les touches
                // vont à la capture, Échap compris.
                if self
                    .panel
                    .as_ref()
                    .is_some_and(|p| p.keys.capture.is_some())
                {
                    return Task::none();
                }
                // Carte myMFA : Échap l'annule ; Entrée referme « Coffre prêt » ; Tab
                // passe du mot de passe à sa confirmation et retour.
                if self.mfa_dialog.is_some()
                    && key == keyboard::Key::Named(keyboard::key::Named::Escape)
                {
                    return self.close_mfa_dialog();
                }
                if self.mfa_fenetre.is_some()
                    && key == keyboard::Key::Named(keyboard::key::Named::Escape)
                {
                    return self.mfa_fenetre_close();
                }
                if self.mfa_ready_open() && key == keyboard::Key::Named(keyboard::key::Named::Enter)
                {
                    return self.mfa_update(mymfa::MMsg::Submit);
                }
                if self.mfa_dialog.is_some()
                    && key == keyboard::Key::Named(keyboard::key::Named::Tab)
                {
                    return self.mfa_update(mymfa::MMsg::Tab);
                }
                // Écran « Espaces de travail » : Échap referme (ou annule le
                // renommage), Entrée ouvre l'espace choisi, les flèches choisissent.
                if self.overview.is_some() {
                    return self.overview_key(&key);
                }
                // Sélecteur de fenêtres : Échap annule, Entrée valide, flèches et Tab.
                if self.switcher.is_some() {
                    use keyboard::key::Named;
                    return match key {
                        keyboard::Key::Named(Named::Escape) => self.close_switcher(),
                        keyboard::Key::Named(Named::Enter) => self.switcher_commit(),
                        keyboard::Key::Named(Named::ArrowDown | Named::Tab) => {
                            self.switch_windows(false)
                        }
                        keyboard::Key::Named(Named::ArrowUp) => self.switch_windows(true),
                        _ => Task::none(),
                    };
                }
                // Recherche ouverte : Échap la ferme, les flèches choisissent.
                if self.search.is_some() {
                    use keyboard::key::Named;
                    return match key {
                        keyboard::Key::Named(Named::Escape) => self.close_search(),
                        keyboard::Key::Named(Named::ArrowDown) => self.search_move(true),
                        keyboard::Key::Named(Named::ArrowUp) => self.search_move(false),
                        // Grille : gauche et droite d'une tuile (le champ garde
                        // ces touches quand on y tape).
                        keyboard::Key::Named(Named::ArrowRight)
                            if self.search.as_ref().is_some_and(|s| s.grid) =>
                        {
                            self.search_step(1)
                        }
                        keyboard::Key::Named(Named::ArrowLeft)
                            if self.search.as_ref().is_some_and(|s| s.grid) =>
                        {
                            self.search_step(-1)
                        }
                        _ => Task::none(),
                    };
                }
                // Le dock tient le clavier : il garde ses touches.
                if self.launcher_surfaces.contains_key(&id)
                    || (self.bar_surfaces.contains_key(&id)
                        && self.launcher_runtime.focus.is_some())
                {
                    return self.launcher_key(id, &key);
                }
                if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                    if self.icon_menu.is_some() {
                        return self.close_icon_menu();
                    }
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
                    // Échap ne change pas de mode : il retire la sélection.
                    self.selected = None;
                    return Task::none();
                }
                // Le panneau a ses propres champs de saisie : pas de navigation
                // dessous ; Tab et Maj + Tab passent d'un champ à l'autre.
                if self.panel.is_some() || self.chat_shown() {
                    if key == keyboard::Key::Named(keyboard::key::Named::Tab) {
                        return if self.mods.shift() {
                            iced::widget::operation::focus_previous()
                        } else {
                            iced::widget::operation::focus_next()
                        };
                    }
                    return Task::none();
                }
                // Bureau : Tab descend dans le dock (Échap y remonte).
                if key == keyboard::Key::Named(keyboard::key::Named::Tab)
                    && let Some(monitor) = self.surfaces.get(&id).cloned()
                {
                    return self.launcher_grab(&monitor);
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
            // État du démon d'index : la loupe du dock affiche la pastille.
            Subscription::run(crate::ui::plugins::index::status_stream),
            // Agent polkit (fenêtre de saisie) ; inactif si un autre agent est en place.
            Subscription::run(polkit::stream).map(|e| Msg::Polkit(polkit::PMsg::Event(e))),
            // Trousseau myMFA : état et demandes de déverrouillage (sans client : rien).
            Subscription::run(mymfa::stream).map(|e| Msg::Mymfa(mymfa::MMsg::Event(e))),
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
        if self.listens_to_events() {
            subs.push(iced::event::listen_with(listen));
        }
        // Capture d'un raccourci : chaque touche, sans modificateurs appliqués.
        if self
            .panel
            .as_ref()
            .is_some_and(|p| p.keys.capture.is_some())
        {
            subs.push(iced::event::listen_with(|e, _, _| match e {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    physical_key,
                    modifiers,
                    ..
                }) => Some(plugins::keys::wrap(plugins::keys::Msg::CaptureKey(
                    key,
                    physical_key,
                    modifiers,
                ))),
                _ => None,
            }));
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
        // Couleurs des écrans ouvertes : problèmes des tables et programmes
        // gênants revus (un wlsunset lancé après coup apparaît, killswitch).
        if self.panel.as_ref().is_some_and(|p| p.tab == "colors") {
            subs.push(
                iced::time::every(Duration::from_secs(3))
                    .map(|_| plugins::colors::wrap(plugins::colors::Msg::Status)),
            );
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
        if let Some((mon, img)) = self.color_test_of(id) {
            return self.view_color_test(mon, img);
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
        if self.mfa_dialog.as_ref().is_some_and(|d| d.win == id) {
            return self.view_mfa_dialog();
        }
        if self.mfa_fenetre.as_ref().is_some_and(|f| f.win == id) {
            return self.view_mfa_fenetre();
        }
        if self.toasts.win == Some(id) {
            return self.view_toasts();
        }
        if self.problem.as_ref().is_some_and(|p| p.win == id) {
            return self.view_problem();
        }
        if self.search.as_ref().is_some_and(|s| s.win == id) {
            return self.view_search();
        }
        if self.switcher.as_ref().is_some_and(|s| s.win == id) {
            return self.view_switcher();
        }
        if self.overview.as_ref().is_some_and(|o| o.win == id) {
            return self.view_overview();
        }
        if let Some(m) = &self.icon_menu {
            if m.win == id {
                return self.view_icon_menu();
            }
            if m.catcher == id {
                return self.view_icon_catcher();
            }
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
            let with_windows = self
                .surfaces
                .get(&id)
                .is_some_and(|n| self.applied_mode(n) == DesktopMode::LauncherOnly);
            let mut content: Vec<Element<'_, Msg>> = Vec::new();
            if !with_windows {
                if self.cfg.plugin_on("icons") {
                    content.push(self.view_icons(id, t));
                }
                if self.cfg.plugin_on("widgets") && !d.widgets.is_empty() {
                    // Sans superposition : chaque widget écarté de ceux déjà posés.
                    content.push(desk_layout::desk(
                        d.widgets
                            .iter()
                            .map(|w| self.view_widget(id, w, t))
                            .collect(),
                    ));
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
            // Clic droit sur le fond (pas sur une icône ni un widget, qui ont
            // le leur) : menu du bureau.
            let content: Element<'_, Msg> = if with_windows {
                Stack::with_children(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                mouse_area(
                    Stack::with_children(content)
                        .width(Length::Fill)
                        .height(Length::Fill),
                )
                .on_right_press(Msg::DeskContext(id))
                .into()
            };
            layers.push(
                container(content)
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
            .on_right_press(Msg::IconContext(win, idx))
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

    pub fn parse(s: &str) -> Option<DesktopMode> {
        Some(match s {
            "no-desktop" => DesktopMode::NoDesktop,
            "launcher-only" => DesktopMode::LauncherOnly,
            "full-desktop" => DesktopMode::FullDesktop,
            _ => return None,
        })
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
        Msg::Problem(..) => "Problem",
        Msg::Search(..) => "Search",
        Msg::IconContext(..) => "IconContext",
        Msg::WidgetContext(..) => "WidgetContext",
        Msg::DeskContext(..) => "DeskContext",
        Msg::IconMenu(..) => "IconMenu",
        Msg::Switcher(..) => "Switcher",
        Msg::CalendarMonth(..) => "CalendarMonth",
        Msg::MenuRefreshed(..) => "MenuRefreshed",
        Msg::Modifiers(..) => "Modifiers",
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
            base: String::new(),
            focused,
            reserved: (0.0, 0.0, 0.0, 0.0),
            x: 0.0,
            y: 0.0,
            scale: 1.0,
        }
    }

    #[test]
    fn graphite_drag_shared_by_settings_and_bar_reorders_and_cancels() {
        let mut app = app();
        app.settings.bar = Some(crate::settings::BarSettings {
            style: config::BarStyle::Graphite,
            zones: [
                vec![],
                vec![crate::settings::BarEntry::new("launcher")],
                vec![
                    crate::settings::BarEntry::new("volume"),
                    crate::settings::BarEntry::new("clock"),
                ],
            ],
            ..Default::default()
        });
        let initial = app.settings.bar.clone();
        let _ = app.update_inner(Msg::BarDragStart(1, 0));
        assert!(app.bar_drag.is_none(), "launcher stays fixed");
        let _ = app.update_inner(Msg::BarDragStart(2, 0));
        let _ = app.update_inner(Msg::BarDragOver(2, 2));
        let _ = app.update_inner(Msg::BarDragLeave(2));
        let _ = app.update_inner(Msg::Released);
        assert!(app.bar_drag.is_none());
        assert_eq!(app.settings.bar, initial, "release outside cancels");
        let _ = app.update_inner(Msg::BarDragStart(2, 0));
        let _ = app.update_inner(Msg::BarDragOver(2, 2));
        let _ = app.update_inner(Msg::Key(
            Id::unique(),
            keyboard::Key::Named(keyboard::key::Named::Escape),
        ));
        assert!(app.bar_drag.is_none());
        assert_eq!(app.settings.bar, initial, "Escape cancels");
        let _ = app.update_inner(Msg::BarDragStart(2, 0));
        let _ = app.update_inner(Msg::BarDragOver(2, 2));
        let _ = app.update_inner(Msg::BarDragEnd);
        assert!(app.bar_drag.is_none());
        let b = app.settings.bar.as_ref().unwrap();
        assert_eq!(b.zones[2][0].type_id, "clock");
        assert_eq!(b.zones[2][1].type_id, "volume");
        assert_eq!(b.zones[1][0].type_id, "launcher");
    }

    #[test]
    fn controls_sources_follow_widget_visibility_and_disable() {
        fn load(source: &str) -> config::lua::Loaded {
            config::lua::load_chunks(
                &[(source.into(), "test.lua".into(), config::Origin::User)],
                std::path::Path::new("/tmp/hyprdesk-control-tests"),
            )
            .unwrap()
        }
        let mut app = app();
        let desk = Id::unique();
        app.monitors = vec![monitor("A", true)];
        app.surfaces.insert(desk, "A".into());
        app.cfg = Arc::new(load("hd.widget('controls',{id='center',screen='all'})").config);
        let _ = app.poll_widgets();
        assert!(
            app.pending_widgets.is_empty(),
            "hidden desktop must not poll controls"
        );
        app.raised.insert(desk);
        let _ = app.poll_widgets();
        assert!(app.pending_widgets.contains("bar:volume"));
        for kind in widgets::controls::SOURCES {
            assert!(
                app.pending_widgets
                    .contains(&format!("bar:controls:{kind}"))
            );
        }
        app.pending_widgets.clear();
        app.last_poll.clear();
        Arc::make_mut(&mut app.cfg)
            .general
            .disabled_plugins
            .insert("widgets".into());
        let _ = app.poll_widgets();
        assert!(
            app.pending_widgets.is_empty(),
            "disabled desktop widgets must stop polling"
        );
        app.cfg = Arc::new(load("hd.widget('controls',{id='center'}) hd.disable('center')").config);
        let _ = app.poll_widgets();
        assert!(
            app.pending_widgets.is_empty(),
            "disabled controls must stop all owned polling"
        );
        app.cfg = Arc::new(load("hd.bar({right={'controls'}})").config);
        app.bar_surfaces.insert(
            Id::unique(),
            bar::BarSurface {
                monitor: "A".into(),
                position: config::BarPosition::Top,
                height: 44,
                margin: 0,
            },
        );
        let _ = app.poll_widgets();
        assert!(app.pending_widgets.contains("bar:volume"));
        app.pending_widgets.clear();
        app.last_poll.clear();
        Arc::make_mut(&mut app.cfg)
            .general
            .disabled_plugins
            .insert("bar".into());
        let _ = app.poll_widgets();
        assert!(
            app.pending_widgets.is_empty(),
            "disabled bar must stop its widgets and their dependencies"
        );
    }

    /// Modes imposés : arriver sur le workspace imposé change le mode, en
    /// repartir le rend ; un écran imposé en no-desktop n'a jamais de bureau.
    #[test]
    fn forced_modes_follow_the_focus() {
        let mut app = app();
        let mut cfg = (*app.cfg).clone();
        cfg.general
            .forced_modes
            .insert("ws:5".into(), "full-desktop".into());
        cfg.general
            .forced_modes
            .insert("B".into(), "no-desktop".into());
        app.cfg = std::sync::Arc::new(cfg);
        let on = |ws: &str| Monitor {
            workspace: ws.into(),
            ..monitor("A", true)
        };
        let _ = app.set_monitors(vec![on("1"), monitor("B", false)]);
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        let _ = app.set_monitors(vec![on("5"), monitor("B", false)]);
        assert_eq!(app.mode(), DesktopMode::FullDesktop);
        assert!(app.shows_on("A") && !app.shows_on("B"));
        // Sur place, un changement à la main reste.
        app.fade = None;
        let _ = app.set_mode(DesktopMode::LauncherOnly);
        let _ = app.set_monitors(vec![on("5"), monitor("B", false)]);
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
        // En repartant : le mode d'avant (no-desktop).
        app.fade = None;
        let _ = app.set_monitors(vec![on("2"), monitor("B", false)]);
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
    }

    /// Workspace 5 imposé en plein bureau, deux écrans : seul l'écran qui
    /// l'affiche passe en plein bureau ; l'autre garde le mode de son
    /// workspace, même quand il a le focus.
    #[test]
    fn forced_workspace_only_on_its_screen() {
        let mut app = app();
        let mut cfg = (*app.cfg).clone();
        cfg.general
            .forced_modes
            .insert("ws:5".into(), "full-desktop".into());
        app.cfg = std::sync::Arc::new(cfg);
        let at = |name: &str, ws: &str, focused: bool| Monitor {
            workspace: ws.into(),
            ..monitor(name, focused)
        };
        let _ = app.set_monitors(vec![at("DP-1", "1", true), at("HDMI-A-2", "6", false)]);
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        let _ = app.set_monitors(vec![at("DP-1", "5", true), at("HDMI-A-2", "6", false)]);
        assert_eq!(app.monitor_mode("DP-1"), DesktopMode::FullDesktop);
        assert_eq!(app.monitor_mode("HDMI-A-2"), DesktopMode::NoDesktop);
        assert!(app.shows_on("DP-1") && !app.shows_on("HDMI-A-2"));
        // Focus sur l'autre écran : chacun garde le sien.
        let _ = app.set_monitors(vec![at("DP-1", "5", false), at("HDMI-A-2", "6", true)]);
        assert_eq!(
            app.mode(),
            DesktopMode::NoDesktop,
            "écran actif : workspace 6"
        );
        assert_eq!(app.monitor_mode("DP-1"), DesktopMode::FullDesktop);
        // Changement à la main : le workspace de l'écran actif seulement.
        app.fade = None;
        let _ = app.set_mode(DesktopMode::LauncherOnly);
        assert_eq!(app.monitor_mode("HDMI-A-2"), DesktopMode::LauncherOnly);
        assert_eq!(app.monitor_mode("DP-1"), DesktopMode::FullDesktop);
        // DP-1 quitte le 5 : le workspace 2 n'a jamais été réglé.
        let _ = app.set_monitors(vec![at("DP-1", "2", true), at("HDMI-A-2", "6", false)]);
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        assert_eq!(app.monitor_mode("HDMI-A-2"), DesktopMode::LauncherOnly);
    }

    /// Chaque workspace garde son mode : on le retrouve en y revenant.
    #[test]
    fn each_workspace_keeps_its_mode() {
        let mut app = app();
        let on = |ws: &str| Monitor {
            workspace: ws.into(),
            ..monitor("A", true)
        };
        app.surfaces.insert(Id::unique(), "A".into());
        let _ = app.set_monitors(vec![on("1")]);
        let _ = app.set_mode(DesktopMode::FullDesktop);
        assert_eq!(app.mode(), DesktopMode::FullDesktop);
        let _ = app.set_monitors(vec![on("2")]);
        assert_eq!(
            app.mode(),
            DesktopMode::NoDesktop,
            "workspace 2 jamais réglé"
        );
        assert!(!app.visible);
        let _ = app.set_mode(DesktopMode::LauncherOnly);
        let _ = app.set_monitors(vec![on("1")]);
        assert_eq!(
            app.mode(),
            DesktopMode::FullDesktop,
            "le 1 retrouve le sien"
        );
        assert!(app.visible);
        let _ = app.set_monitors(vec![on("2")]);
        assert_eq!(app.mode(), DesktopMode::LauncherOnly, "le 2 aussi");
    }

    /// Scratchpad (workspace spécial) ouvert par-dessus : le sélecteur de mode
    /// règle le workspace normal de l'écran, sans attendre qu'il se referme.
    #[test]
    fn mode_selector_works_under_a_scratchpad() {
        let mut app = app();
        let magic = Monitor {
            workspace: "special:magic".into(),
            base: "1".into(),
            ..monitor("A", true)
        };
        app.surfaces.insert(Id::unique(), "A".into());
        let _ = app.set_monitors(vec![magic.clone()]);
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        let _ = app.set_mode(DesktopMode::LauncherOnly);
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
        assert!(app.visible);
        app.fade = None;
        let _ = app.set_mode(DesktopMode::NoDesktop);
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        // Le scratchpad refermé : le workspace 1 a le mode réglé dessous.
        app.fade = None;
        let _ = app.set_mode(DesktopMode::FullDesktop);
        let _ = app.set_monitors(vec![monitor("A", true)]);
        assert_eq!(app.mode(), DesktopMode::FullDesktop);
    }

    /// Plein bureau sous un scratchpad garni : la fenêtre du scratchpad garde
    /// le clavier et ses raccourcis (pas de submap) ; refermé sur un bureau
    /// vide, le bureau reprend la main.
    #[test]
    fn full_desktop_leaves_keyboard_to_a_scratchpad() {
        let mut app = app();
        let magic = Monitor {
            workspace: "special:magic".into(),
            base: "1".into(),
            ..monitor("A", true)
        };
        app.surfaces.insert(Id::unique(), "A".into());
        app.clients = vec![hypr::Client {
            address: "0x9".into(),
            class: "kitty".into(),
            title: String::new(),
            workspace: "special:magic".into(),
            monitor: "A".into(),
        }];
        let _ = app.set_monitors(vec![magic]);
        let _ = app.set_mode(DesktopMode::FullDesktop);
        assert_eq!(app.mode(), DesktopMode::FullDesktop);
        assert!(app.windows_shown_on("A"));
        assert!(!app.in_submap, "raccourcis gardés pour le scratchpad");
        app.clients.clear();
        let _ = app.set_monitors(vec![monitor("A", true)]);
        assert!(!app.windows_shown_on("A"));
        assert!(app.in_submap, "bureau vide : raccourcis suspendus");
    }

    /// Plein bureau : Alt+Tab vers une fenêtre cachée quitte le bureau ;
    /// vers une fenêtre ouverte sur le bureau, le bureau reste.
    #[test]
    fn alt_tab_in_full_desktop() {
        let w = |a: &str, stashed: bool| crate::switcher::Win {
            address: a.into(),
            class: "kitty".into(),
            title: String::new(),
            workspace: if stashed {
                hypr::STASH.into()
            } else {
                "1".into()
            },
            focus: 0,
            stashed,
        };
        let open = |app: &mut App, wins: Vec<crate::switcher::Win>| {
            app.switcher = Some(switcher::Switcher {
                win: Id::unique(),
                wins,
                selected: 0,
                loading: false,
                back: false,
            });
        };
        let mut app = app();
        app.test_mode(DesktopMode::FullDesktop);
        app.session_windows.insert("0xd".into());
        open(&mut app, vec![w("0xd", false)]);
        let _ = app.switcher_commit();
        assert_eq!(app.mode(), DesktopMode::FullDesktop, "fenêtre du bureau");
        open(&mut app, vec![w("0x1", true)]);
        let _ = app.switcher_commit();
        // Aucun changement de mode demandé : le bureau reste affiché.
        assert_eq!(
            app.mode(),
            DesktopMode::FullDesktop,
            "fenêtre hors du bureau : le mode ne change pas"
        );
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
            "raccourcis/hyprdesk".into(),
        )));
        assert_eq!(app.subtab("keys"), "hyprdesk");
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
    fn the_app_launcher_hears_its_keys_with_the_desktop_hidden() {
        let mut app = app();
        app.monitors = vec![monitor("A", true)];
        assert_eq!(app.mode(), DesktopMode::NoDesktop);
        assert!(!app.listens_to_events(), "barre seule : rien à écouter");
        // SUPER + R bureau masqué : flèches, Entrée et Échap doivent arriver.
        let _ = app.update_inner(Msg::Control(control::Command::Search));
        assert!(app.search.is_some());
        assert!(app.listens_to_events());
        let _ = app.update_inner(Msg::Key(
            Id::unique(),
            keyboard::Key::Named(keyboard::key::Named::Escape),
        ));
        assert!(app.search.is_none());
        assert!(!app.listens_to_events());
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
        assert!(
            app.desk_windows.contains("0x2"),
            "ramenée en plein bureau : flottante, rendue à la sortie"
        );
        assert!(!app.in_submap, "la fenêtre rendue garde ses raccourcis");
    }

    #[test]
    fn the_settings_button_toggles_the_drawer() {
        let mut app = app();
        let a = Id::unique();
        app.monitors = vec![monitor("A", true)];
        app.surfaces.insert(a, "A".into());
        app.sizes.insert(a, Size::new(1440.0, 900.0));
        let _ = app.update_inner(Msg::OpenPanel);
        assert!(app.panel_open(), "premier clic : ouvert");
        let _ = app.update_inner(Msg::OpenPanel);
        assert!(!app.panel_open(), "second clic : fermé");
        let _ = app.update_inner(Msg::OpenPanel);
        assert!(app.panel_open(), "troisième clic : rouvert");
        // Le raccourci et `hyprdesk settings` ouvrent toujours, sans refermer.
        let _ = app.update_inner(Msg::Control(control::Command::Settings("desktop".into())));
        assert!(app.panel_open());
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
                screen: Default::default(),
                if_missing: Default::default(),
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
            margin: 0,
            zones: Default::default(),
            portrait: None,
            compact: config::BarCompact::Auto,
            style: config::BarStyle::Continuite,
            colors: Default::default(),
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
        app.ws_modes.insert("1".into(), DesktopMode::FullDesktop);
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
        Arc::make_mut(&mut app.cfg).general.show_on = ShowOn::Focused;
        app.test_mode(DesktopMode::FullDesktop);
        let _ = app.set_monitors(vec![monitor("A", false), monitor("B", true)]);
        assert_eq!(app.raised, HashSet::from([b]));
    }

    /// Bureau masqué : ses fenêtres redeviennent ordinaires (configuration
    /// d'origine, MLG-55) ; celles qu'une version d'avant a rangées dans DESK
    /// reviennent au prochain affichage.
    #[test]
    fn desk_windows_follow_the_desktop() {
        let mut app = app();
        app.test_mode(DesktopMode::FullDesktop);
        app.session_windows.insert("0x1".into());
        app.desk_windows.insert("0x1".into());
        let _ = app.hide();
        assert!(app.desk_windows.is_empty() && app.session_windows.is_empty());
        // Restées dans DESK : elles reviennent au prochain affichage, du bureau à nouveau.
        app.clients = vec![hypr::Client {
            address: "0x1".into(),
            class: "kitty".into(),
            title: String::new(),
            workspace: hypr::DESK.into(),
            monitor: "A".into(),
        }];
        app.fade = None;
        let _ = app.show();
        assert!(app.desk_windows.contains("0x1") && app.session_windows.contains("0x1"));
    }

    /// Mode lanceur : les fenêtres du bureau redeviennent ordinaires ; une
    /// fenêtre restée dans DESK (version d'avant MLG-55) ne revient qu'avec
    /// le plein bureau.
    #[test]
    fn desk_windows_only_in_full_desktop() {
        let mut app = app();
        app.test_mode(DesktopMode::FullDesktop);
        app.session_windows.insert("0x1".into());
        app.desk_windows.insert("0x1".into());
        let _ = app.show_with_windows();
        assert_eq!(app.mode(), DesktopMode::LauncherOnly);
        assert!(app.desk_windows.is_empty() && !app.session_windows.contains("0x1"));
        app.clients = vec![hypr::Client {
            address: "0x1".into(),
            class: "kitty".into(),
            title: String::new(),
            workspace: hypr::DESK.into(),
            monitor: "A".into(),
        }];
        // Depuis le bureau masqué, le mode lanceur ne les ramène pas.
        let _ = app.hide();
        app.fade = None;
        let _ = app.show_with_windows();
        assert!(app.desk_windows.is_empty());
        // Le plein bureau, si.
        let _ = app.show();
        assert_eq!(app.mode(), DesktopMode::FullDesktop);
        assert!(app.desk_windows.contains("0x1"));
    }

    #[test]
    fn closing_the_last_launched_window_restores_shortcut_suspension() {
        let mut app = app();
        app.test_mode(DesktopMode::FullDesktop);
        app.session_windows.insert("0x1".into());
        let _ = app.update_inner(Msg::Hypr(hypr::Update::Clients(Vec::new())));
        assert!(app.session_windows.is_empty());
        assert!(app.in_submap);
    }
}
