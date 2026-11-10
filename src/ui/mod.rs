//! Interface : un démon iced_layershell, UNE surface par écran.
//!
//! Au repos, chaque surface vit sur le calque `background` et dessine le fond
//! d'écran du workspace affiché (elle remplace hyprpaper). Appelée, elle passe
//! sur le calque `top` (au-dessus des fenêtres, sous la barre), prend le
//! clavier, et affiche icônes et widgets en fondu. Renvoyée, elle redescend.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Local;
use iced::futures::{SinkExt, Stream};
mod bar;
mod panel;
mod popover;
mod widget_view;

use iced::widget::{Space, Stack, button, column, container, image, mouse_area, pin, row, svg, text, tooltip};
use iced::window::Id;
use iced::{Alignment, Background, Border, Color, Element, Event, Font, Length, Padding, Point, Size, Subscription, Task, keyboard, mouse};
use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::{Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::config::lua::LuaHandle;
use crate::config::resolve::{Desktop, resolve};
use crate::config::{
    self, Config, Fit, HookEvent, IconDef, Mode, Rgba, RunningApps, ShowOn, Transition, WallSource, WidgetKind,
};
use crate::hypr::{self, Client, Monitor};
use crate::launch::AppInfo;
use crate::settings::Settings as DeskSettings;
use panel::{Panel, PanelMsg};
use crate::layout::{self, Cell, Layout};
use crate::{control, launch, widgets};

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
type WallKey = (PathBuf, u32, u32, Fit, bool);

/// Mode verre : l'image est décodée à 1/BLUR_DOWNSCALE de sa taille puis
/// légèrement floutée ; l'agrandissement par le GPU achève le flou, pour un
/// coût négligeable.
const BLUR_DOWNSCALE: u32 = 8;

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
    SettingsLoaded(DeskSettings),
    /// Barre : aller au workspace (écran, nom).
    BarWorkspace(String, String),
    /// Barre : molette sur les workspaces (écran, ±1).
    BarScroll(String, i32),
    VolumeMute,
    VolumeStep(i32),
    VolumeSet(f32),
    VolumeValue(Option<(f32, bool)>),
    /// Ouvre (ou referme) un menu sous un module de la barre : (barre, clé du module, contenu).
    OpenPopover(Id, String, popover::Request),
    PopoverReady(Id, String, popover::Kind),
    ClosePopover,
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
    KeysInstalled(Result<(Vec<(String, String)>, Vec<String>), String>),
    ConfigChanged,
    ConfigLoaded(Result<Arc<Config>, String>),
    /// Génération, icônes du bureau, puis icônes des commandes média.
    IconsResolved(u64, Vec<launch::Resolved>, Vec<Option<PathBuf>>),
    WallLoaded(WallKey, Result<image::Handle, String>),
    Rotate,
    Frame,
    Poll,
    WidgetValue(String, widgets::Value),
    Resized(Id, Size),
    Closed(Id),
    Cursor(Id, Point),
    Released,
    Key(Id, keyboard::Key),
    IconPress(Id, usize),
    /// Clic sur un widget, réglages ouverts : sélection et début de glissé.
    WidgetPress(Id, String),
    IconHover(Option<(Id, usize)>),
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
        let p = (now.saturating_duration_since(self.start).as_secs_f32() / self.dur.as_secs_f32()).clamp(0.0, 1.0);
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
    /// Surfaces actuellement sur le calque `top`.
    raised: HashSet<Id>,
    fade: Option<Fade>,
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
    layout: Layout,
    selected: Option<(Id, usize)>,
    hovered: Option<(Id, usize)>,
    cursor: HashMap<Id, Point>,
    drag: Option<Drag>,
    wdrag: Option<WidgetDrag>,
    /// Réglages de l'écran ⚙ (relus depuis settings.lua seul).
    settings: DeskSettings,
    panel: Option<Panel>,
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
    stash: Vec<hypr::Stashed>,
    /// Fenêtres ouvertes pendant la session bureau : le bureau les contient.
    session_windows: HashSet<String>,
    /// Surfaces de la barre native.
    bar_surfaces: HashMap<Id, bar::BarSurface>,
    workspaces: Vec<hypr::Workspace>,
    active_window: Option<(String, String)>,
    volume: Option<(f32, bool)>,
    popover: Option<popover::Popover>,
    /// Icônes du thème par nom (symboliques teintées), résolues à la demande.
    named_icons: HashMap<String, IconImg>,
    named_pending: HashSet<String>,
    tray: Vec<crate::tray::TrayItem>,
    /// Images fournies par les applications de la zone de notification : clé → (empreinte, image).
    tray_imgs: HashMap<String, (u64, image::Handle)>,
    /// Police de l'interface (barre, menus) : celle de la config, sinon Noto Sans.
    ui_family: &'static str,
}

pub fn run(config_path: PathBuf) -> Result<(), iced_layershell::Error> {
    daemon(move || App::boot(config_path.clone()), || NAMESPACE.to_string(), App::update, App::view)
        .subscription(App::subscription)
        .style(|_, _| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: Color::WHITE })
        .theme(|app: &App, _| Some(app.theme.clone()))
        .settings(Settings {
            layer_settings: LayerShellSettings { start_mode: StartMode::Background, ..Default::default() },
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

fn chrono_locale() -> chrono::Locale {
    let lang = std::env::var("LC_TIME").or_else(|_| std::env::var("LANG")).unwrap_or_default();
    let base = lang.split('.').next().unwrap_or("");
    chrono::Locale::try_from(base).unwrap_or(chrono::Locale::POSIX)
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
                    p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| matches!(e.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp"))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Décode et met à l'échelle une image de fond (hors thread UI).
fn decode_wall(key: &WallKey) -> Result<image::Handle, String> {
    let (path, w, h, fit, blur) = key;
    let img = ::image::open(path).map_err(|e| format!("{} : {e}", path.display()))?;
    let img = if *blur { img.thumbnail((img.width() / BLUR_DOWNSCALE).max(1), (img.height() / BLUR_DOWNSCALE).max(1)) } else { img };
    let (w, h) = if *blur { (&((*w / BLUR_DOWNSCALE).max(1)), &((*h / BLUR_DOWNSCALE).max(1))) } else { (w, h) };
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let (tw, th) = (*w as f32, *h as f32);
    use ::image::imageops::FilterType;
    let img = match fit {
        Fit::Fill => img.resize_exact(*w, *h, FilterType::Triangle),
        Fit::Center => img,
        Fit::Cover | Fit::Contain => {
            let s = if *fit == Fit::Cover { (tw / iw).max(th / ih) } else { (tw / iw).min(th / ih) };
            let (nw, nh) = ((iw * s).round().max(1.0) as u32, (ih * s).round().max(1.0) as u32);
            img.resize_exact(nw, nh, FilterType::Triangle)
        }
    };
    let img = if *blur { img.fast_blur(2.5) } else { img };
    let rgba = img.to_rgba8();
    Ok(image::Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw()))
}

fn watch_stream(path: &PathBuf) -> impl Stream<Item = ()> + use<> {
    let path = path.clone();
    iced::stream::channel(4, async move |mut out: iced::futures::channel::mpsc::Sender<()>| {
        use notify::{RecursiveMode, Watcher};
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        let dir = path.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let modules = dir.join("hyprdesk");
        let (target, mods) = (path.clone(), modules.clone());
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res
                && !matches!(ev.kind, notify::EventKind::Access(_))
                && ev.paths.iter().any(|p| p == &target || p.starts_with(&mods))
            {
                let _ = tx.send(());
            }
        });
        let mut watcher = match watcher {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("surveillance de la config : {e}");
                return;
            }
        };
        if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
            tracing::error!("surveillance de {} : {e}", dir.display());
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
    })
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
        let app = App {
            lua: LuaHandle::spawn(),
            config_path,
            cfg: Arc::new(Config::default()),
            config_error: None,
            font: Font::DEFAULT,
            monitors: Vec::new(),
            surfaces: HashMap::new(),
            sizes: HashMap::new(),
            visible: false,
            raised: HashSet::new(),
            fade: None,
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
            layout: Layout::load(),
            selected: None,
            hovered: None,
            cursor: HashMap::new(),
            drag: None,
            wdrag: None,
            settings: DeskSettings::default(),
            panel: None,
            apps: Arc::new(Vec::new()),
            clients: Vec::new(),
            class_imgs: HashMap::new(),
            class_pending: HashSet::new(),
            theme: make_theme(&Config::default()),
            key_conflicts: Vec::new(),
            universal_binds: Vec::new(),
            in_submap: false,
            stash: Vec::new(),
            session_windows: HashSet::new(),
            bar_surfaces: HashMap::new(),
            workspaces: Vec::new(),
            active_window: None,
            volume: None,
            popover: None,
            named_icons: HashMap::new(),
            named_pending: HashSet::new(),
            tray: Vec::new(),
            tray_imgs: HashMap::new(),
            ui_family: "Noto Sans",
        };
        let apps = Task::perform(async { tokio::task::spawn_blocking(launch::installed_apps).await.unwrap_or_default() }, Msg::Apps);
        let t = Task::batch([app.load_config(), apps]);
        (app, t)
    }

    fn load_config(&self) -> Task<Msg> {
        let (lua, path) = (self.lua.clone(), self.config_path.clone());
        let p2 = path.clone();
        let settings = Task::perform(
            async move { tokio::task::spawn_blocking(move || config::lua::load_settings_only(&p2)).await },
            |r| match r {
                Ok(Ok(c)) => Msg::SettingsLoaded(DeskSettings::from_config(&c)),
                Ok(Err(e)) => {
                    tracing::warn!("settings.lua : {e}");
                    Msg::Noop
                }
                Err(_) => Msg::Noop,
            },
        );
        Task::batch([Task::perform(async move { lua.load(path).await.map(Arc::new) }, Msg::ConfigLoaded), settings])
    }

    /// Remplace la config effective en ne refaisant que ce qui a changé : un
    /// réglage modifié en direct, ou un rechargement identique, ne fait ni
    /// clignoter les icônes ni repartir le diaporama.
    fn install_config(&mut self, cfg: Arc<Config>) -> Task<Msg> {
        let old = std::mem::replace(&mut self.cfg, cfg);
        let n_owned = self.cfg.general.clone();
        let (o, n) = (&old.general, &n_owned);
        if o.font != n.font || self.icons_gen == 0 {
            self.font = match &n.font {
                // Les noms de police doivent vivre aussi longtemps que le programme.
                Some(f) => Font::with_name(Box::leak(f.clone().into_boxed_str())),
                None => Font::DEFAULT,
            };
            self.ui_family = match &n.font {
                Some(f) => Box::leak(f.clone().into_boxed_str()),
                None => "Noto Sans",
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
            || old.widgets.iter().zip(&self.cfg.widgets).any(|(a, b)| !a.same_content(b));
        if widgets_changed {
            self.values.clear();
            self.last_poll.clear();
            if self.visible {
                tasks.push(Task::done(Msg::Poll));
            }
        }
        // La taille d'icône ne demande pas de nouvelle recherche : l'image est mise à l'échelle.
        let icons_changed =
            old.icons != self.cfg.icons || o.icon_theme != n.icon_theme || o.terminal != n.terminal || self.icons_gen == 0;
        if icons_changed {
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
                            .map(|d| launch::resolve(d, g.icon_theme.as_deref(), g.grid.icon_size, &g.terminal))
                            .collect::<Vec<_>>();
                        let media = UI_ICONS.iter().map(|n| launch::lookup_icon(n, g.icon_theme.as_deref(), 24)).collect::<Vec<_>>();
                        (icons, media)
                    })
                    .await
                    .unwrap_or_default()
                },
                move |(icons, media)| Msg::IconsResolved(gen_, icons, media),
            ));
        } else if o.running_apps != n.running_apps {
            tasks.push(self.resolve_class_icons());
        }
        if o.show_on != n.show_on {
            tasks.push(self.sync_raised());
        }
        // Premier chargement (catalogue vide avant) ou raccourcis modifiés.
        if o.keys != n.keys || old.catalog.is_empty() {
            tasks.push(self.install_keys());
        }
        if old.bar != self.cfg.bar {
            tasks.push(self.sync_bars());
            self.last_poll.retain(|k, _| !k.starts_with("bar:"));
            tasks.push(Task::done(Msg::Poll));
        }
        if old.components != self.cfg.components || old.catalog.is_empty() {
            // Composants de session : lancés, arrêtés ou relancés selon la config.
            let defs = self.cfg.components.clone();
            tasks.push(Task::perform(async move { crate::session::sync(&defs) }, |_| Msg::Noop));
        }
        if o.suspend_binds != n.suspend_binds {
            tasks.push(self.sync_submap());
        }
        tasks.push(self.refresh_walls());
        Task::batch(tasks)
    }

    /// (Ré)installe les raccourcis de hyprdesk dans Hyprland.
    fn install_keys(&self) -> Task<Msg> {
        let keys = self.cfg.general.keys.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || hypr::install_keys(&keys).map_err(|e| e.to_string()))
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
            },
            Msg::KeysInstalled,
        )
    }

    /// Submap « hyprdesk » active si et seulement si le bureau est affiché
    /// et que la suspension des raccourcis est demandée.
    fn sync_submap(&mut self) -> Task<Msg> {
        // Dès qu'une fenêtre est ouverte depuis le bureau, ses raccourcis
        // habituels doivent pouvoir agir sur elle.
        let want = self.visible && self.cfg.general.suspend_binds && self.session_windows.is_empty();
        if want == self.in_submap {
            return Task::none();
        }
        self.in_submap = want;
        Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || hypr::set_submap(want.then_some(hypr::SUBMAP))).await;
            },
            |_| Msg::Noop,
        )
    }

    /// Bureau affiché : aligne les surfaces actives sur `show_on` (écran actif ou tous).
    fn sync_raised(&mut self) -> Task<Msg> {
        if !self.visible {
            return Task::none();
        }
        let focused = self.monitors.iter().find(|m| m.focused).map(|m| m.name.clone());
        let all = self.cfg.general.show_on == ShowOn::All;
        let panel_win = self.panel.as_ref().map(|p| p.win);
        let mut tasks = Vec::new();
        for (id, mon) in self.surfaces.clone() {
            let want = all || Some(&mon) == focused.as_ref() || Some(id) == panel_win;
            let has = self.raised.contains(&id);
            if want && !has {
                tasks.extend(self.activate_surface(id, Some(&mon) == focused.as_ref()));
            } else if !want && has {
                self.raised.remove(&id);
                tasks.push(Task::done(Msg::KeyboardInteractivityChange { id, keyboard_interactivity: KeyboardInteractivity::None }));
                tasks.extend(self.lower_surface(id));
                tasks.push(self.restore_stash(Some(&mon)));
            }
        }
        Task::batch(tasks)
    }

    /// Active le bureau sur une surface. Le bureau est la RACINE des fenêtres :
    /// il reste toujours dessous. Les fenêtres du workspace sont mises de côté
    /// et celles qu'on ouvre depuis le bureau s'ouvrent dessus. Le mode verre
    /// n'est qu'un effet sur le fond (fond d'écran flouté et teinté).
    fn activate_surface(&mut self, id: Id, has_focus: bool) -> Vec<Task<Msg>> {
        let Some(m) = self.monitor(id).cloned() else { return Vec::new() };
        self.raised.insert(id);
        let mut tasks = vec![self.stash_workspace(&m)];
        let kb = if has_focus { KeyboardInteractivity::Exclusive } else { KeyboardInteractivity::None };
        tasks.push(Task::done(Msg::KeyboardInteractivityChange { id, keyboard_interactivity: kb }));
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
    /// celles ouvertes depuis le bureau). Fait de façon SYNCHRONE (quelques
    /// requêtes IPC) : aucune sortie ne peut s'intercaler et perdre une fenêtre.
    fn stash_workspace(&mut self, m: &Monitor) -> Task<Msg> {
        let picked: Vec<hypr::Stashed> = self
            .clients
            .iter()
            .filter(|c| c.workspace == m.workspace && !self.session_windows.contains(&c.address))
            .filter(|c| !self.stash.iter().any(|s| s.address == c.address))
            .map(|c| hypr::Stashed { address: c.address.clone(), workspace: c.workspace.clone(), monitor: m.name.clone() })
            .collect();
        if picked.is_empty() {
            return Task::none();
        }
        self.stash.extend(picked.iter().cloned());
        // La liste est écrite AVANT de déplacer : un arrêt brutal ne perd rien.
        hypr::save_stash(&self.stash);
        for s in &picked {
            if let Err(e) = hypr::move_window(&s.address, hypr::STASH) {
                tracing::warn!("mise de côté de {} : {e}", s.address);
            }
        }
        hypr::close_stash_view();
        Task::none()
    }

    /// Rend les fenêtres mises de côté (d'un écran, ou toutes), de façon synchrone.
    fn restore_stash(&mut self, monitor: Option<&str>) -> Task<Msg> {
        let (back, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.stash).into_iter().partition(|s| monitor.is_none_or(|m| s.monitor == m));
        self.stash = keep;
        if !back.is_empty() {
            hypr::restore(&back);
            hypr::save_stash(&self.stash);
        }
        Task::none()
    }

    /// Rejoue l'apparition du bureau (aperçu d'une transition).
    fn replay_fade(&mut self) {
        if self.visible {
            self.fade = Some(Fade { from: 0.0, to: 1.0, start: Instant::now(), dur: self.fade_duration() });
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
        let focused = self.monitors.iter().find(|m| m.focused).map(|m| m.name.clone());
        let Some(bar_id) = self
            .bar_surfaces
            .iter()
            .find(|(_, s)| Some(&s.monitor) == focused.as_ref())
            .or_else(|| self.bar_surfaces.iter().next())
            .map(|(id, _)| *id)
        else {
            return Task::none();
        };
        let items: Vec<config::BarItem> = self.cfg.bar.iter().flat_map(|b| b.zones.iter().flatten().cloned()).collect();
        for it in items {
            let req = match &it {
                config::BarItem::Volume if kind == "volume" => Some(("volume".to_string(), popover::Request::Volume)),
                config::BarItem::Tray if kind == "tray" => self.tray.iter().find_map(|t| {
                    t.menu.clone().map(|m| (format!("tray:{}", t.key), popover::Request::Tray { bus: t.bus.clone(), menu_path: m }))
                }),
                config::BarItem::Widget(id) => self.cfg.widgets.iter().find(|w| &w.id == id && w.type_id == kind).and_then(|w| match &w.kind {
                    config::WidgetKind::Clock { .. } | config::WidgetKind::Calendar => Some((id.clone(), popover::Request::Calendar)),
                    config::WidgetKind::Lua { menu: Some(f), .. } => Some((id.clone(), popover::Request::LuaMenu { widget: id.clone(), func: *f })),
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
        if self.cfg.general.running_apps == RunningApps::Off {
            return Task::none();
        }
        let missing: Vec<String> = self
            .clients
            .iter()
            .map(|c| c.class.clone())
            .filter(|c| !self.class_imgs.contains_key(c) && !self.class_pending.contains(c))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if missing.is_empty() {
            return Task::none();
        }
        self.class_pending.extend(missing.iter().cloned());
        let (apps, theme, gen_) = (self.apps.clone(), self.cfg.general.icon_theme.clone(), self.icons_gen);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    missing.into_iter().map(|c| {
                        let p = launch::icon_for_class(&c, &apps, theme.as_deref(), 48);
                        (c, p)
                    }).collect::<Vec<_>>()
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
            .or_else(|| self.monitor(id).map(|m| Size::new(m.width as f32, m.height as f32)))
            .unwrap_or(Size::new(1920.0, 1080.0));
        let (l, t, r, b) = self.reserved(id);
        Size::new((full.width - l - r).max(1.0), (full.height - t - b).max(1.0))
    }

    fn fade_value(&self) -> f32 {
        match self.fade {
            Some(f) => f.value(Instant::now()),
            None if self.visible => 1.0,
            None => 0.0,
        }
    }

    fn animating(&self) -> bool {
        self.fade.is_some() || self.shown.values().any(|s| s.prev.is_some())
    }

    fn hooks(&self, ev: HookEvent) -> Vec<usize> {
        self.cfg.hooks.iter().filter(|(e, _)| *e == ev).map(|(_, f)| *f).collect()
    }

    fn fire(&self, ev: HookEvent, name: &str, monitor: &Monitor) {
        self.lua.hook(self.hooks(ev), name, &monitor.name, &monitor.workspace);
    }

    fn publish_status(&self) {
        let screens: Vec<String> = self.monitors.iter().map(|m| format!("{}:{}", m.name, m.workspace)).collect();
        control::set_status(format!(
            "{} (pid {}) — écrans {} — config {} ({})",
            if self.visible { "affiché" } else { "caché" },
            std::process::id(),
            screens.join(" "),
            match &self.config_error {
                None => "ok".to_string(),
                Some(e) => format!("en erreur : {e}"),
            },
            self.config_path.display()
        ));
    }

    // -- Affichage / masquage ------------------------------------------------

    fn show(&mut self) -> Task<Msg> {
        if self.visible && self.fade.is_none_or(|f| f.to >= 1.0) {
            return Task::none();
        }
        self.visible = true;
        let focused = self.monitors.iter().find(|m| m.focused).map(|m| m.name.clone());
        let targets: Vec<(Id, bool)> = self
            .surfaces
            .iter()
            .filter(|(_, mon)| self.cfg.general.show_on == ShowOn::All || Some(*mon) == focused.as_ref())
            .map(|(id, mon)| (*id, Some(mon) == focused.as_ref()))
            .collect();
        let from = self.fade_value();
        self.fade = Some(Fade { from, to: 1.0, start: Instant::now(), dur: self.fade_duration() });
        self.selected = None;
        let mut tasks = Vec::new();
        for (id, has_focus) in targets {
            if !self.raised.contains(&id) {
                tasks.extend(self.activate_surface(id, has_focus));
            }
        }
        self.last_poll.clear();
        tasks.push(Task::done(Msg::Poll));
        tasks.push(self.refresh_walls());
        // Les raccourcis de Hyprland ne doivent pas agir sur les fenêtres cachées.
        tasks.push(self.sync_submap());
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

    fn hide(&mut self) -> Task<Msg> {
        if !self.visible {
            return Task::none();
        }
        self.visible = false;
        self.drag = None;
        self.panel = None;
        self.hovered = None;
        let from = self.fade_value();
        self.fade = Some(Fade { from, to: 0.0, start: Instant::now(), dur: self.fade_duration() });
        let mut tasks: Vec<Task<Msg>> = Vec::new();
        for id in self.raised.iter().copied().collect::<Vec<_>>() {
            tasks.push(Task::done(Msg::KeyboardInteractivityChange { id, keyboard_interactivity: KeyboardInteractivity::None }));
            if let Some(m) = self.monitor(id) {
                self.fire(HookEvent::Hide, "hide", m);
            }
        }
        // Les fenêtres mises de côté reviennent, à côté de celles ouvertes depuis le bureau.
        tasks.push(self.restore_stash(None));
        tasks.push(self.refresh_walls());
        self.session_windows.clear();
        tasks.push(self.sync_submap());
        tasks.push(self.finish_fade());
        Task::batch(tasks)
    }

    /// En fin de fondu sortant, les surfaces redescendent sur `background`.
    fn finish_fade(&mut self) -> Task<Msg> {
        let now = Instant::now();
        let Some(f) = self.fade else { return Task::none() };
        if !f.done(now) {
            return Task::none();
        }
        self.fade = None;
        if self.visible {
            return Task::none();
        }
        let ids: Vec<Id> = self.raised.drain().collect();
        let tasks: Vec<Task<Msg>> = ids.into_iter().flat_map(|id| self.lower_surface(id)).collect();
        Task::batch(tasks)
    }

    // -- Écrans ----------------------------------------------------------------

    fn set_monitors(&mut self, list: Vec<Monitor>) -> Task<Msg> {
        if list.is_empty() {
            return Task::none();
        }
        let old = std::mem::replace(&mut self.monitors, list);
        let mut tasks = Vec::new();
        for m in &self.monitors {
            if !self.surfaces.values().any(|n| n == &m.name) {
                let id = Id::unique();
                self.surfaces.insert(id, m.name.clone());
                tracing::info!("surface pour {} ({}x{})", m.name, m.width, m.height);
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
        let gone: Vec<Id> =
            self.surfaces.iter().filter(|(_, n)| !self.monitors.iter().any(|m| &m.name == *n)).map(|(id, _)| *id).collect();
        for id in gone {
            self.surfaces.remove(&id);
            self.raised.remove(&id);
            tasks.push(iced::window::close(id));
        }
        // Changement de workspace pendant que le bureau est affiché.
        for m in &self.monitors {
            let changed = old.iter().find(|o| o.name == m.name).is_some_and(|o| o.workspace != m.workspace);
            let raised = self.surfaces.iter().any(|(id, n)| n == &m.name && self.raised.contains(id));
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
                old.iter().find(|o| o.name == m.name).is_some_and(|o| o.workspace != m.workspace)
                    && self.surfaces.iter().any(|(id, n)| n == &m.name && self.raised.contains(id))
            })
            .cloned()
            .collect();
        for m in to_stash {
            tasks.push(self.stash_workspace(&m));
        }
        tasks.push(self.refresh_walls());
        tasks.push(self.sync_bars());
        if self.visible {
            tasks.push(Task::done(Msg::Poll));
        }
        Task::batch(tasks)
    }

    // -- Fonds d'écran ---------------------------------------------------------

    fn rot_file(&mut self, idx: usize, dir: &std::path::Path, interval: Duration, shuffle: bool) -> Option<PathBuf> {
        let r = self.rot.entry(idx).or_insert_with(|| {
            let mut files = list_images(dir);
            if shuffle {
                use rand::seq::SliceRandom;
                files.shuffle(&mut rand::rng());
            }
            if files.is_empty() {
                tracing::warn!("rotation : aucune image dans {}", dir.display());
            }
            Rot { files, idx: 0, next: Instant::now() + interval }
        });
        r.files.get(r.idx % r.files.len().max(1)).cloned()
    }

    fn refresh_walls(&mut self) -> Task<Msg> {
        let mut tasks = Vec::new();
        let cfg = self.cfg.clone();
        let monitors = self.monitors.clone();
        for m in &monitors {
            let d = resolve(&cfg, &m.name, &m.workspace);
            // Mode verre, bureau affiché : le même fond, flouté.
            let blur = self.visible
                && d.mode == Mode::Glass
                && self.surfaces.iter().any(|(id, n)| n == &m.name && self.raised.contains(id));
            let want = match (d.wallpaper, d.wallpaper_idx) {
                (None, _) | (_, None) => Wall::Color(cfg.general.background),
                (Some(rule), Some(idx)) => match &rule.source {
                    WallSource::Color(c) => Wall::Color(*c),
                    WallSource::Path(p) => Wall::Image((p.clone(), m.width, m.height, rule.fit, blur)),
                    WallSource::Rotate { dir, interval, shuffle } => match self.rot_file(idx, dir, *interval, *shuffle) {
                        Some(p) => Wall::Image((p, m.width, m.height, rule.fit, blur)),
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
                            let r = tokio::task::spawn_blocking(move || decode_wall(&k2)).await.unwrap_or_else(|e| Err(e.to_string()));
                            (k, r)
                        },
                        |(k, r)| Msg::WallLoaded(k, r),
                    ));
                }
                continue;
            }
            let prev = self.shown.remove(&m.name).map(|s| s.current);
            self.shown.insert(m.name.clone(), Shown { current: want, prev, since: Instant::now() });
        }
        // Le cache ne garde que les images affichées (courantes ou en fondu).
        let used: HashSet<&WallKey> = self
            .shown
            .values()
            .flat_map(|s| [Some(&s.current), s.prev.as_ref()])
            .flatten()
            .filter_map(|w| if let Wall::Image(k) = w { Some(k) } else { None })
            .collect();
        let keep: HashSet<WallKey> = used.into_iter().cloned().collect();
        self.walls.retain(|k, _| keep.contains(k) || self.loading.contains(k));
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
        if self.bar_has(&config::BarItem::Volume) && self.last_poll.get("bar:volume").is_none_or(|t| now.duration_since(*t) >= Duration::from_secs(2)) {
            self.last_poll.insert("bar:volume".into(), now);
            tasks.push(Task::perform(bar::read_volume(), Msg::VolumeValue));
        }
        if self.bar_has(&config::BarItem::Tray) && self.last_poll.get("bar:tray").is_none_or(|t| now.duration_since(*t) >= Duration::from_secs(2)) {
            self.last_poll.insert("bar:tray".into(), now);
            tasks.push(Task::perform(crate::tray::items(), Msg::TrayItems));
        }
        {
            for w in todo {
                if !seen.insert(w.id.clone()) {
                    continue;
                }
                if self.last_poll.get(&w.id).is_some_and(|t| now.duration_since(*t) < w.interval) {
                    continue;
                }
                let wid = w.id.clone();
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
                            move |v| Msg::WidgetValue(wid.clone(), v),
                        ))
                    }
                    WidgetKind::Media { player } => {
                        let p = player.clone();
                        Some(Task::perform(async move { widgets::media(&p).await }, move |v| Msg::WidgetValue(wid.clone(), v)))
                    }
                    WidgetKind::Command { exec, timeout } => {
                        let (c, t) = (exec.clone(), *timeout);
                        Some(Task::perform(async move { widgets::run_command(&c, t).await }, move |v| Msg::WidgetValue(wid.clone(), v)))
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
                            move |v| Msg::WidgetValue(wid.clone(), v),
                        ))
                    }
                };
                if let Some(t) = task {
                    self.last_poll.insert(w.id.clone(), now);
                    tasks.push(t);
                }
            }
        }
        Task::batch(tasks)
    }

    // -- Icônes ----------------------------------------------------------------

    /// Icônes d'une surface avec leur cellule : (index dans cfg.icons, cellule).
    fn placed(&self, id: Id) -> Vec<(usize, Cell)> {
        let Some((m, d)) = self.desktop_of(id) else { return Vec::new() };
        let g = &self.cfg.general.grid;
        let size = self.size_of(id);
        let (cols, rows) = layout::grid_dims(size.width, size.height, g.cell, g.gap, g.margin);
        let wanted: Vec<Option<Cell>> =
            d.icons.iter().map(|i| self.layout.get(&layout::key(&m.name, &m.workspace, &i.id)).or(i.pos)).collect();
        let cells = layout::place(&wanted, cols, rows);
        d.icons.iter().zip(cells).filter_map(|(i, c)| Some((index_of(&self.cfg.icons, *i), c?))).collect()
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
    pub(super) fn ui_font(&self, weight: iced::font::Weight) -> Font {
        Font { family: iced::font::Family::Name(self.ui_family), weight, ..Font::DEFAULT }
    }

    /// Icône du thème par nom ; les icônes symboliques sont teintées (`tint`).
    pub(super) fn icon_named<'a>(&self, name: &str, size: f32, tint: Color) -> Option<Element<'a, Msg>> {
        match self.named_icons.get(name).copied()? {
            IconImg::Vector(i) => {
                let symbolic = name.ends_with("-symbolic");
                Some(
                    svg(self.vectors[i].clone())
                        .width(size)
                        .height(size)
                        .style(move |_, _| svg::Style { color: symbolic.then_some(tint) })
                        .into(),
                )
            }
            IconImg::Raster(i) => Some(image(self.rasters[i].clone()).width(size).height(size).into()),
            IconImg::None => None,
        }
    }

    /// Résout une icône par nom (une fois), hors du thread UI.
    pub(super) fn want_icon(&mut self, name: &str) -> Task<Msg> {
        if self.named_icons.contains_key(name) || !self.named_pending.insert(name.to_string()) {
            return Task::none();
        }
        let (n, theme, gen_) = (name.to_string(), self.cfg.general.icon_theme.clone(), self.icons_gen);
        Task::perform(
            async move {
                let n2 = n.clone();
                let p = tokio::task::spawn_blocking(move || {
                    launch::lookup_icon(&n2, theme.as_deref(), 32)
                        // Repli : la version non symbolique, puis le thème par défaut.
                        .or_else(|| n2.strip_suffix("-symbolic").and_then(|b| launch::lookup_icon(b, theme.as_deref(), 32)))
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
            IconImg::Raster(i) => Some(image(self.rasters[i].clone()).width(size).height(size).opacity(alpha).into()),
            IconImg::Vector(i) => Some(svg(self.vectors[i].clone()).width(size).height(size).opacity(alpha).into()),
            IconImg::None => None,
        }
    }

    fn launch(&mut self, idx: usize) -> Task<Msg> {
        let Some(r) = self.icons.get(idx) else { return Task::none() };
        let cmd = r.command.clone();
        let stays = !self.cfg.general.close_on_launch;
        let t = self.run_from_desktop(cmd, stays);
        if stays { t } else { Task::batch([t, self.hide()]) }
    }

    /// Lance une commande depuis le bureau. S'il reste affiché, la fenêtre
    /// s'y ouvre flottante et centrée (option `float_launched`) : le bureau
    /// la contient.
    pub(super) fn run_from_desktop(&self, cmd: String, stays: bool) -> Task<Msg> {
        let size = (stays && self.cfg.general.float_launched)
            .then(|| self.monitors.iter().find(|m| m.focused).map(|m| ((m.width as f32 * 0.6) as u32, (m.height as f32 * 0.7) as u32)))
            .flatten();
        tracing::info!("lancement : {cmd}");
        Task::perform(
            async move {
                let r = tokio::task::spawn_blocking(move || match size {
                    Some((w, h)) => hypr::exec_floating(&cmd, w, h),
                    None => hypr::exec(&cmd),
                })
                .await;
                match r {
                    Ok(Err(e)) => tracing::warn!("lancement : {e}"),
                    Err(e) => tracing::warn!("lancement : {e}"),
                    Ok(Ok(())) => {}
                }
            },
            |_| Msg::Noop,
        )
    }

    fn navigate(&mut self, id: Id, key: &keyboard::Key) -> Task<Msg> {
        use keyboard::key::Named;
        let placed = self.placed(id);
        if placed.is_empty() {
            return Task::none();
        }
        let cur = self.selected.filter(|(w, _)| *w == id).and_then(|(_, i)| placed.iter().find(|(x, _)| *x == i).copied());
        let pick = |pred: &dyn Fn(Cell) -> Option<u32>| placed.iter().filter_map(|(i, c)| pred(*c).map(|k| (k, *i))).min().map(|(_, i)| i);
        let next = match (key, cur) {
            (keyboard::Key::Named(Named::Enter), Some((i, _))) => return self.launch(i),
            (keyboard::Key::Named(Named::Enter), None) => None,
            (_, None) => placed.iter().min_by_key(|(_, c)| (c.0, c.1)).map(|(i, _)| *i),
            (keyboard::Key::Named(n), Some((_, (c, r)))) => {
                let dist = |a: u16, b: u16| (a as i32 - b as i32).unsigned_abs();
                match n {
                    Named::ArrowDown => pick(&|(cc, rr)| (cc == c && rr > r).then(|| dist(rr, r))),
                    Named::ArrowUp => pick(&|(cc, rr)| (cc == c && rr < r).then(|| dist(rr, r))),
                    Named::ArrowRight => pick(&|(cc, rr)| (cc > c).then(|| dist(cc, c) * 1000 + dist(rr, r))),
                    Named::ArrowLeft => pick(&|(cc, rr)| (cc < c).then(|| dist(cc, c) * 1000 + dist(rr, r))),
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

    fn drop_icon(&mut self, win: Id, icon: usize) {
        let Some(p) = self.cursor.get(&win).copied() else { return };
        let g = self.cfg.general.grid.clone();
        let size = self.size_of(win);
        let (cols, rows) = layout::grid_dims(size.width, size.height, g.cell, g.gap, g.margin);
        // Le curseur est en coordonnées de surface ; la grille commence après la zone réservée.
        let (rl, rt, _, _) = self.reserved(win);
        let Some(target) = layout::cell_at(p.x - rl, p.y - rt, g.cell, g.gap, g.margin, cols, rows) else { return };
        let placed = self.placed(win);
        let Some(&(_, from)) = placed.iter().find(|(i, _)| *i == icon) else { return };
        let Some(m) = self.monitor(win).cloned() else { return };
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
        self.count_msg(&msg);
        let t = self.update_inner(msg);
        self.publish_status();
        t
    }

    fn update_inner(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Control(c) => match c {
                control::Command::Toggle => {
                    if self.visible {
                        self.hide()
                    } else {
                        self.show()
                    }
                }
                control::Command::Show => self.show(),
                control::Command::Hide => self.hide(),
                control::Command::Reload => self.load_config(),
                control::Command::Quit => {
                    if self.in_submap {
                        hypr::set_submap(None);
                    }
                    // Rien ne doit rester de côté après l'arrêt.
                    let _ = self.restore_stash(None);
                    crate::session::stop_all();
                    hypr::remove_keys();
                    iced::exit()
                }
                control::Command::Status => Task::none(),
                control::Command::Menu(kind) => self.open_menu_by_type(&kind),
                control::Command::Settings(tab) => {
                    let s = self.show();
                    let o = self.open_panel();
                    let tab = match tab {
                        1 => panel::Tab::Icons,
                        2 => panel::Tab::Wallpaper,
                        3 => panel::Tab::Effects,
                        4 => panel::Tab::Widgets,
                        _ => panel::Tab::Desktop,
                    };
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
            Msg::VolumeMute => self.volume_action(vec!["set-mute".into(), "@DEFAULT_AUDIO_SINK@".into(), "toggle".into()]),
            Msg::VolumeStep(d) => self.volume_action(vec![
                "set-volume".into(),
                "-l".into(),
                "1.5".into(),
                "@DEFAULT_AUDIO_SINK@".into(),
                format!("{}%{}", d.abs(), if d > 0 { "+" } else { "-" }),
            ]),
            Msg::VolumeValue(v) => {
                self.volume = v;
                Task::none()
            }
            Msg::Hypr(hypr::Update::Reloaded) => {
                // Une relecture de la config Hyprland efface nos raccourcis et notre submap.
                let mut t = vec![self.install_keys()];
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
                            let list: Vec<String> = conflicts.iter().map(|(k, w)| format!("{k} (déjà lié à {w})")).collect();
                            tracing::warn!("raccourcis non installés : {}", list.join(", "));
                            hypr::notify(0, 8000, &format!("hyprdesk : raccourci non installé — {}", list.join(", ")));
                        }
                        self.key_conflicts = conflicts;
                        self.universal_binds = universal;
                    }
                    Err(e) => tracing::warn!("raccourcis : {e}"),
                }
                Task::none()
            }
            Msg::Hypr(hypr::Update::Clients(list)) => {
                let mut tasks = Vec::new();
                if self.visible {
                    // Fenêtre ouverte sur un bureau affiché : le bureau la contient ;
                    // elle reçoit le clavier et ses raccourcis habituels.
                    let fresh: Vec<Client> = list
                        .iter()
                        .filter(|c| !self.clients.iter().any(|o| o.address == c.address))
                        .filter(|c| c.workspace != hypr::STASH)
                        .cloned()
                        .collect();
                    for c in fresh {
                        let surfaces: Vec<Id> = self
                            .surfaces
                            .iter()
                            .filter(|(id, n)| self.raised.contains(id) && self.monitors.iter().any(|m| &m.name == *n && m.workspace == c.workspace))
                            .map(|(id, _)| *id)
                            .collect();
                        if surfaces.is_empty() {
                            continue;
                        }
                        self.session_windows.insert(c.address.clone());
                        for id in surfaces {
                            tasks.push(Task::done(Msg::KeyboardInteractivityChange { id, keyboard_interactivity: KeyboardInteractivity::OnDemand }));
                        }
                    }
                    tasks.push(self.sync_submap());
                }
                self.clients = list;
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
            Msg::FocusWindow(addr) => {
                let t = Task::perform(
                    async move {
                        if let Ok(Err(e)) = tokio::task::spawn_blocking(move || hypr::focus_window(&addr)).await {
                            tracing::warn!("focus : {e}");
                        }
                    },
                    |_| Msg::Noop,
                );
                Task::batch([t, self.hide()])
            }
            Msg::OpenPanel => {
                let s = self.show();
                Task::batch([s, self.open_panel()])
            }
            Msg::Panel(m) => self.panel_update(m),
            Msg::SettingsLoaded(s) => {
                self.settings = s;
                Task::none()
            }
            Msg::ConfigChanged => self.load_config(),
            Msg::ConfigLoaded(Ok(cfg)) => {
                tracing::info!(
                    "config chargée : {} fonds, {} icônes, {} widgets",
                    cfg.wallpapers.len(),
                    cfg.icons.len(),
                    cfg.widgets.len()
                );
                if self.config_error.take().is_some() {
                    hypr::notify(3, 2500, "hyprdesk : configuration rechargée");
                }
                self.install_config(cfg)
            }
            Msg::ConfigLoaded(Err(e)) => {
                tracing::error!("config : {e}");
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
                self.icon_imgs = list.iter().map(|r| self.handle_for(r.image.as_deref())).collect();
                self.ui_imgs = media.iter().map(|p| self.handle_for(p.as_deref())).collect();
                self.icons = list;
                // Les poignées des fenêtres et des icônes nommées vivaient dans les vecteurs vidés plus haut.
                self.class_imgs.clear();
                self.class_pending.clear();
                self.named_icons.clear();
                self.named_pending.clear();
                let mut again: Vec<Task<Msg>> = bar::STATIC_ICONS.iter().map(|n| self.want_icon(n)).collect();
                let names: Vec<String> = self
                    .values
                    .values()
                    .filter_map(|v| if let widgets::Value::Rich(r) = v { r.icon.clone() } else { None })
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
                    Ok(h) => {
                        self.walls.insert(key, h);
                    }
                    Err(e) => {
                        tracing::warn!("fond d'écran : {e}");
                        hypr::notify(0, 5000, &format!("hyprdesk : fond d'écran illisible — {e}"));
                        return Task::none();
                    }
                }
                self.refresh_walls()
            }
            Msg::Rotate => {
                let now = Instant::now();
                let cfg = self.cfg.clone();
                let mut changed = false;
                for (idx, r) in self.rot.iter_mut() {
                    if now >= r.next
                        && let Some(WallSource::Rotate { interval, .. }) = cfg.wallpapers.get(*idx).map(|w| &w.source)
                    {
                        r.idx = (r.idx + 1) % r.files.len().max(1);
                        r.next = now + *interval;
                        changed = true;
                    }
                }
                if changed { self.refresh_walls() } else { Task::none() }
            }
            Msg::Frame => {
                let now = Instant::now();
                let cf = self.crossfade();
                for s in self.shown.values_mut() {
                    if s.prev.is_some() && now.duration_since(s.since) >= cf {
                        s.prev = None;
                    }
                }
                self.finish_fade()
            }
            Msg::Poll => self.poll(),
            Msg::WidgetValue(id, v) => {
                let t = match &v {
                    widgets::Value::Rich(r) => r.icon.clone().map(|i| self.want_icon(&i)).unwrap_or_else(Task::none),
                    _ => Task::none(),
                };
                self.values.insert(id, v);
                t
            }
            Msg::IconResolved(gen_, name, path) => {
                if gen_ == self.icons_gen {
                    self.named_pending.remove(&name);
                    let img = self.handle_for(path.as_deref());
                    self.named_icons.insert(name, img);
                }
                Task::none()
            }
            Msg::OpenPopover(bar_id, key, req) => match req {
                popover::Request::Volume => self.open_popover(bar_id, key, popover::Kind::Volume),
                popover::Request::Calendar => self.open_popover(bar_id, key, popover::Kind::Calendar),
                popover::Request::LuaMenu { widget, func } => {
                    if self.popover.as_ref().is_some_and(|p| p.key == key) {
                        return self.close_popover();
                    }
                    let lua = self.lua.clone();
                    Task::perform(async move { lua.menu(func).await }, move |r| match r {
                        Ok(entries) => {
                            Msg::PopoverReady(bar_id, key.clone(), popover::Kind::Menu { entries, source: popover::MenuSource::Lua { widget: widget.clone() } })
                        }
                        Err(e) => {
                            tracing::warn!("menu de {widget} : {e}");
                            hypr::notify(0, 5000, &format!("hyprdesk : menu de « {widget} » — {e}"));
                            Msg::Noop
                        }
                    })
                }
                popover::Request::Tray { bus, menu_path } => {
                    if self.popover.as_ref().is_some_and(|p| p.key == key) {
                        return self.close_popover();
                    }
                    Task::perform(
                        {
                            let (b, m) = (bus.clone(), menu_path.clone());
                            async move { crate::tray::menu(&b, &m).await }
                        },
                        move |r| match r {
                            Ok(entries) => Msg::PopoverReady(
                                bar_id,
                                key.clone(),
                                popover::Kind::Menu { entries, source: popover::MenuSource::Tray { bus: bus.clone(), menu_path: menu_path.clone() } },
                            ),
                            Err(e) => {
                                tracing::warn!("menu de la zone de notification : {e}");
                                Msg::Noop
                            }
                        },
                    )
                }
            },
            Msg::PopoverReady(bar_id, key, kind) => self.open_popover(bar_id, key, kind),
            Msg::ClosePopover => self.close_popover(),
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
                self.volume_action(vec!["set-volume".into(), "@DEFAULT_AUDIO_SINK@".into(), format!("{v:.2}")])
            }
            Msg::TrayItems(items) => {
                let mut tasks = Vec::new();
                for t in &items {
                    if let Some((w, h, data)) = &t.pixmap {
                        use std::hash::{Hash, Hasher};
                        let mut hs = std::collections::hash_map::DefaultHasher::new();
                        data.hash(&mut hs);
                        let fp = hs.finish();
                        if self.tray_imgs.get(&t.key).is_none_or(|(f, _)| *f != fp) {
                            self.tray_imgs.insert(t.key.clone(), (fp, image::Handle::from_rgba(*w, *h, data.clone())));
                        }
                    } else if let Some(n) = &t.icon_name {
                        self.tray_imgs.remove(&t.key);
                        tasks.push(self.want_icon(n));
                    }
                }
                self.tray = items;
                Task::batch(tasks)
            }
            Msg::TrayActivate(_bar, bus, path, secondary) => {
                // Les applications attendent des coordonnées globales.
                let (x, y) = hypr::cursor_pos().unwrap_or_default();
                Task::perform(crate::tray::activate(bus, path, secondary, x as i32, y as i32), |_| Msg::Noop)
            }
            Msg::Resized(id, size) => {
                self.sizes.insert(id, size);
                Task::none()
            }
            Msg::Closed(id) => {
                if self.popover.as_ref().is_some_and(|p| p.win == id || p.catcher == id) {
                    // Fermé par le compositeur : on referme l'autre surface aussi.
                    return self.close_popover();
                }
                self.bar_surfaces.remove(&id);
                self.surfaces.remove(&id);
                self.raised.remove(&id);
                self.sizes.remove(&id);
                Task::none()
            }
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
                if moved { self.panel_update(PanelMsg::Commit) } else { Task::none() }
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
                        return self.close_popover();
                    }
                    if self.panel.is_some() {
                        self.panel = None;
                        return Task::none();
                    }
                    return self.hide();
                }
                // Le panneau a ses propres champs de saisie : pas de navigation dessous.
                if self.panel.is_some() {
                    return Task::none();
                }
                self.navigate(id, &key)
            }
            Msg::IconPress(win, icon) => {
                let start = self.cursor.get(&win).copied().unwrap_or(Point::ORIGIN);
                self.selected = Some((win, icon));
                self.drag = Some(Drag { win, icon, start, active: false });
                Task::none()
            }
            Msg::WidgetPress(win, wid) => {
                let Some(from) = self.cfg.widgets.iter().find(|w| w.id == wid).map(|w| w.placement()) else { return Task::none() };
                let start = self.cursor.get(&win).copied().unwrap_or(Point::ORIGIN);
                if let Some(p) = self.panel.as_mut() {
                    p.select_widget(&wid);
                }
                self.wdrag = Some(WidgetDrag { win, id: wid, start, from, moved: false });
                Task::none()
            }
            Msg::IconHover(h) => {
                self.hovered = h;
                Task::none()
            }
            Msg::Launch(i) => self.launch(i),
            Msg::Media(bus, method) => {
                for w in &self.cfg.widgets {
                    if matches!(w.kind, WidgetKind::Media { .. }) {
                        self.last_poll.remove(&w.id);
                    }
                }
                Task::perform(widgets::media_control(bus, method), |_| Msg::Noop)
                    .chain(Task::perform(tokio::time::sleep(Duration::from_millis(150)), |_| Msg::Poll))
            }
            Msg::Noop => Task::none(),
            // Messages layer-shell : traités par iced_layershell.
            _ => Task::none(),
        }
    }

    fn subscription(&self) -> Subscription<Msg> {
        let mut subs = vec![
            Subscription::run(control::server_stream).map(Msg::Control),
            Subscription::run(control::signal_stream).map(Msg::Control),
            Subscription::run(hypr::stream).map(Msg::Hypr),
            Subscription::run_with(self.config_path.clone(), watch_stream).map(|_| Msg::ConfigChanged),
            iced::window::resize_events().map(|(id, s)| Msg::Resized(id, s)),
            iced::window::close_events().map(Msg::Closed),
        ];
        if !self.raised.is_empty() || self.popover.is_some() {
            // Bureau affiché (glisser-déposer, clavier) ou menu ouvert (Échap).
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
            subs.push(iced::time::every(Duration::from_millis(16)).map(|_| Msg::Frame));
        }
        if !self.rot.is_empty() {
            subs.push(iced::time::every(Duration::from_secs(1)).map(|_| Msg::Rotate));
        }
        Subscription::batch(subs)
    }

    // -- Vue ---------------------------------------------------------------------

    fn view(&self, id: Id) -> Element<'_, Msg> {
        if self.bar_surfaces.contains_key(&id) {
            return self.view_bar(id);
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
        layers.push(self.view_wall(&m.name, 1.0));
        if raised && d.mode == Mode::Glass {
            // Mode verre : un voile teinté sur le fond flouté.
            let veil = color(self.cfg.general.theme.bg, self.cfg.general.glass_opacity * t);
            layers.push(container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| bg(veil)).into());
        }
        let panel_here = self.panel.as_ref().is_some_and(|p| p.win == id);
        if raised {
            let mut content: Vec<Element<'_, Msg>> = vec![self.view_icons(id, t)];
            for w in &d.widgets {
                content.push(self.view_widget(id, w, t));
            }
            content.push(self.view_dock(m, t));
            let shift = if self.cfg.general.transition == Transition::Slide { (1.0 - t) * SLIDE_PX } else { 0.0 };
            // Le tiroir des réglages occupe la droite : le bureau reste visible
            // à côté et montre chaque réglage en direct.
            let (rl, rt, rr, rb) = self.reserved(id);
            let right = rr + if panel_here { panel::DRAWER_W } else { 0.0 };
            layers.push(
                container(Stack::with_children(content).width(Length::Fill).height(Length::Fill))
                    .padding(Padding { top: rt + shift, right, bottom: rb, left: rl })
                    .into(),
            );
        }
        if panel_here {
            let (_, rt, _, rb) = self.reserved(id);
            layers.push(container(self.view_panel()).padding(Padding { top: rt, bottom: rb, ..Padding::ZERO }).into());
        }
        Stack::with_children(layers).width(Length::Fill).height(Length::Fill).into()
    }

    fn view_wall(&self, monitor: &str, t: f32) -> Element<'_, Msg> {
        let Some(s) = self.shown.get(monitor) else {
            let c = color(self.cfg.general.background, t);
            return container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| bg(c)).into();
        };
        let base = color(self.cfg.general.background, t);
        let cf = self.crossfade().as_secs_f32();
        let p = if cf <= 0.0 { 1.0 } else { (Instant::now().duration_since(s.since).as_secs_f32() / cf).clamp(0.0, 1.0) };
        let mut layers = vec![container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| bg(base)).into()];
        if let Some(prev) = &s.prev {
            layers.push(self.wall_element(prev, t));
            layers.push(self.wall_element(&s.current, t * p));
        } else {
            layers.push(self.wall_element(&s.current, t));
        }
        Stack::with_children(layers).width(Length::Fill).height(Length::Fill).into()
    }

    fn wall_element(&self, w: &Wall, alpha: f32) -> Element<'_, Msg> {
        match w {
            Wall::Color(c) => {
                let c = color(*c, alpha);
                container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| bg(c)).into()
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

    fn icon_cell(&self, win: Id, idx: usize, def: &IconDef, t: f32, ghost: bool) -> Element<'_, Msg> {
        let g = &self.cfg.general;
        let th = &g.theme;
        let size = g.grid.icon_size as f32;
        let alpha = if ghost { t * 0.5 } else { t };
        let img: Element<'_, Msg> = match self.img_element(self.icon_imgs.get(idx).copied().unwrap_or(IconImg::None), size, alpha) {
            Some(e) => e,
            None => {
                let c = color(th.accent, alpha);
                container(text(def.label.clone().unwrap_or_default().chars().next().unwrap_or('?').to_string()).size(size * 0.5))
                    .center(size)
                    .style(move |_| container::Style { background: Some(c.into()), border: Border::default().rounded(size / 4.0), ..Default::default() })
                    .into()
            }
        };
        let label = self.icons.get(idx).map(|r| r.label.clone()).unwrap_or_else(|| def.label.clone().unwrap_or_default());
        let fg = color(th.fg, alpha);
        let selected = self.selected == Some((win, idx));
        let hovered = self.hovered == Some((win, idx));
        let hl = if selected {
            Some(color(th.accent, 0.45 * t))
        } else if hovered {
            Some(color(th.fg, 0.12 * t))
        } else {
            None
        };
        let cell = g.grid.cell as f32;
        let body = container(
            column![img, text(label).size(g.font_size).color(fg).font(self.font).align_x(Alignment::Center).width(Length::Fill)]
                .spacing(6)
                .align_x(Alignment::Center),
        )
        .width(cell)
        .height(cell)
        .padding(6)
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: hl.map(Background::Color),
            border: Border::default().rounded(10),
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
        let dragging = self.drag.as_ref().filter(|d| d.active && d.win == win).map(|d| d.icon);
        let mut layers: Vec<Element<'_, Msg>> = Vec::new();
        for (idx, (c, r)) in self.placed(win) {
            let def = &self.cfg.icons[idx];
            let cell = self.icon_cell(win, idx, def, t, dragging == Some(idx));
            layers.push(pin(cell).x(g.margin as f32 + c as f32 * step).y(g.margin as f32 + r as f32 * step).into());
        }
        if let (Some(idx), Some(p)) = (dragging, self.cursor.get(&win)) {
            let half = g.cell as f32 / 2.0;
            let def = &self.cfg.icons[idx];
            let (rl, rt, _, _) = self.reserved(win);
            layers.push(pin(self.icon_cell(win, idx, def, t, false)).x(p.x - rl - half).y(p.y - rt - half).into());
        }
        Stack::with_children(layers).width(Length::Fill).height(Length::Fill).into()
    }

}

impl App {
    /// Dock en bas au centre : fenêtres ouvertes (si activé), puis le bouton ⚙.
    fn view_dock(&self, m: &Monitor, t: f32) -> Element<'_, Msg> {
        let g = &self.cfg.general;
        let wins: Vec<&Client> = match g.running_apps {
            RunningApps::Off => Vec::new(),
            RunningApps::Workspace => self.clients.iter().filter(|c| c.workspace == m.workspace).collect(),
            RunningApps::All => self.clients.iter().collect(),
        };
        let fg = color(g.theme.fg, t);
        let size = 40.0;
        let hover = move |_: &iced::Theme, s: button::Status| button::Style {
            background: matches!(s, button::Status::Hovered).then(|| Background::Color(Color { a: 0.15 * t, ..fg })),
            border: Border::default().rounded(10),
            ..Default::default()
        };
        let tip_box = move |label: String| {
            container(text(label).size(13).color(fg)).padding([4, 8]).style(move |_| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.8 * t).into()),
                border: Border::default().rounded(6),
                ..Default::default()
            })
        };
        let mut items: Vec<Element<'_, Msg>> = wins
            .into_iter()
            .take(24)
            .map(|c| {
                let img = self.class_imgs.get(&c.class).copied().unwrap_or(IconImg::None);
                let face: Element<'_, Msg> = self.img_element(img, size, t).unwrap_or_else(|| {
                    text(c.class.chars().next().unwrap_or('?').to_uppercase().to_string()).size(size * 0.5).color(fg).into()
                });
                tooltip(
                    button(container(face).center(size)).padding(6).on_press(Msg::FocusWindow(c.address.clone())).style(hover),
                    tip_box(if c.title.is_empty() { c.class.clone() } else { c.title.clone() }),
                    tooltip::Position::Top,
                )
                .into()
            })
            .collect();
        if !items.is_empty() {
            let sep = Color { a: 0.25 * t, ..fg };
            items.push(
                container(Space::new().width(1).height(size))
                    .style(move |_| container::Style { background: Some(sep.into()), ..Default::default() })
                    .padding([0, 4])
                    .into(),
            );
        }
        let gear: Element<'_, Msg> = self
            .img_element(self.ui_imgs.get(ICON_SETTINGS).copied().unwrap_or(IconImg::None), size * 0.6, t)
            .unwrap_or_else(|| text("Réglages").size(13).color(fg).into());
        items.push(
            tooltip(button(container(gear).center(size)).padding(6).on_press(Msg::OpenPanel).style(hover), tip_box("Réglages".into()), tooltip::Position::Top)
                .into(),
        );
        let card_bg = color(g.theme.bg, 0.72 * t);
        container(
            container(row(items).spacing(4).align_y(Alignment::Center))
                .padding(8)
                .style(move |_| container::Style { background: Some(card_bg.into()), border: Border::default().rounded(16), ..Default::default() }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::End)
        .padding(Padding { bottom: 24.0, ..Padding::ZERO })
        .into()
    }
}

/// Diagnostic : messages traités par type, relevé toutes les 5 s ; un trafic
/// anormal (> 1500 messages, soit 300/s) est signalé dans le journal.
static MSG_STATS: std::sync::Mutex<Option<(Instant, HashMap<&'static str, u32>)>> = std::sync::Mutex::new(None);

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
        Msg::WidgetValue(..) => "WidgetValue",
        Msg::Cursor(..) => "Cursor",
        Msg::Resized(..) => "Resized",
        Msg::IconResolved(..) => "IconResolved",
        Msg::TrayItems(_) => "TrayItems",
        Msg::VolumeValue(_) => "VolumeValue",
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
                tracing::warn!("trafic élevé : {total} messages en 5 s — {:?}", &v[..v.len().min(6)]);
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
    container::Style { background: Some(c.into()), ..Default::default() }
}

/// Vérifie la config sans lancer le démon (`hyprdesk check`).
pub fn check(path: &std::path::Path) -> Result<Config, String> {
    config::lua::load_file(path).map(|l| l.config)
}
