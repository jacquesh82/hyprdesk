//! Panorama (MLG-164) : surface plein écran tactile des écrans bandeau.
//!
//! - une surface layer-shell par écran au rôle Panorama (calque Top, jamais
//!   de focus clavier : toucher le Panorama ne prend pas le clavier à la
//!   fenêtre de l'écran principal) ;
//! - des pages qu'on fait glisser (`swipe`), au doigt ou à la souris ;
//! - escamotable : une poignée sur le bord du bas la fait revenir ;
//! - proposée d'elle-même à la détection d'un écran candidat
//!   (`crate::panorama::detect`), une fois par écran.

mod dashboard;
mod fx;
mod instruments;
mod panes;
mod swipe;

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use iced::widget::{Space, button, column, container, image, row, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};
use crate::panorama::detect::{self, Offer, Screen};
use crate::panorama::layout::{self, Orientation, Zones};
use crate::widgets::media::Status;

/// Poignée de l'écran escamoté.
const HANDLE_W: u32 = 180;
const HANDLE_H: u32 = 26;
/// Clé de page et d'état de la fenêtre d'aperçu (pas un écran).
pub const PREVIEW: &str = "\u{0}aperçu";
/// app_id de la fenêtre d'aperçu (règle de Hyprland : flottante, centrée).
pub const PREVIEW_APP_ID: &str = "hyprdesk-panorama-preview";
/// Format simulé par défaut (Xeneon Edge et ses semblables).
pub const PREVIEW_SIZE: (u32, u32) = (2560, 720);
/// Densité simulée d'un écran bandeau de 14,5″ (≈ 183 ppp), en px/mm.
const PREVIEW_PX_PER_MM: f32 = 7.4;
/// Part maximale de l'écran courant occupée par l'aperçu.
const PREVIEW_FIT: f32 = 0.85;

/// Fenêtre d'aperçu : le Panorama dans une fenêtre flottante.
#[derive(Debug, Clone, Copy)]
pub struct Preview {
    pub win: Id,
    /// Format simulé (pixels logiques de l'écran bandeau).
    pub virtual_size: (u32, u32),
    /// Taille de la fenêtre / taille simulée (≤ 1 : l'aperçu tient à l'écran).
    pub scale: f32,
}

/// Taille de la fenêtre pour un format simulé sur un écran `screen` :
/// le format lui-même s'il tient dans 85 % de l'écran, réduit sinon.
pub fn preview_fit(virtual_size: (u32, u32), screen: (u32, u32)) -> ((u32, u32), f32) {
    let (vw, vh) = (virtual_size.0.max(1) as f32, virtual_size.1.max(1) as f32);
    let (sw, sh) = (screen.0.max(1) as f32, screen.1.max(1) as f32);
    let scale = (PREVIEW_FIT * sw / vw).min(PREVIEW_FIT * sh / vh).min(1.0);
    (
        ((vw * scale).round() as u32, (vh * scale).round() as u32),
        scale,
    )
}

/// `WxH` (`2560x720`, `720×2560`).
pub fn parse_size(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once(['x', 'X', '×'])?;
    let (w, h) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    ((200..=8000).contains(&w) && (200..=8000).contains(&h)).then_some((w, h))
}

fn preview_title(size: (u32, u32)) -> String {
    crate::tr!("Panorama — aperçu {w}×{h}", w = size.0, h = size.1)
}

/// Règle de Hyprland de la fenêtre d'aperçu (remplace la précédente).
fn preview_rule_lua(w: u32, h: u32) -> String {
    format!(
        "if __hyprdesk_panorama_preview then pcall(function() __hyprdesk_panorama_preview:set_enabled(false) end) end\n\
         __hyprdesk_panorama_preview = hl.window_rule({{ name = \"hyprdesk-panorama-preview\", \
         match = {{ class = \"^({PREVIEW_APP_ID})$\" }}, float = true, center = true, size = \"{w} {h}\" }})\n"
    )
}

/// Relevés (lecteur, système) tant qu'un Panorama est affiché.
pub const TICK: Duration = Duration::from_secs(1);

const ICONS: &[&str] = &[
    "media-playback-start-symbolic",
    "media-playback-pause-symbolic",
    "media-skip-forward-symbolic",
    "media-skip-backward-symbolic",
    "audio-x-generic-symbolic",
    "go-down-symbolic",
    "go-up-symbolic",
    "window-close-symbolic",
];

#[derive(Debug, Default)]
pub struct State {
    /// Surface → écran.
    pub surfaces: HashMap<Id, String>,
    /// Poignée (Panorama escamoté) → écran.
    pub handles: HashMap<Id, String>,
    /// Carte de proposition → offre.
    pub offers: HashMap<Id, Offer>,
    /// Écrans escamotés (cette session).
    pub hidden: HashSet<String>,
    /// Page affichée, par écran.
    pub page: HashMap<String, usize>,
    /// Écrans tels que Hyprland les décrit (EDID, taille physique).
    pub screens: Vec<Screen>,
    /// Périphériques tactiles.
    pub touch: Vec<String>,
    /// Écrans (identifiant EDID) déjà proposés cette session.
    pub offered: HashSet<String>,
    pub media: Option<Status>,
    pub sys: Vec<crate::widgets::SysLine>,
    /// Dernier toucher (la veille attend `sleep_minutes` sans toucher ni lecture).
    pub last_press: Option<Instant>,
    pub cover: Option<(String, image::Handle)>,
    /// Fenêtre d'aperçu ouverte.
    pub preview: Option<Preview>,
    /// Vidéo du widget Vidéo (fenêtre de navigateur posée dans sa zone).
    pub video: Option<panes::Video>,
    /// Lancements abandonnés avant que leur fenêtre n'apparaisse : elle sera
    /// fermée à son arrivée (une seule source à la fois).
    pub strays: Vec<panes::Stray>,
    /// Réglage du cadrage de la source ouverte (colonne des services).
    pub framing: bool,
    /// Fenêtres du lecteur laissées par un hyprdesk précédent : vérifiées
    /// une fois (reprises, ou fermées).
    pub orphans_checked: bool,
    /// Zone de saisie posée sur chaque surface : ses trous (vidéo).
    pub regions: HashMap<Id, Vec<layout::Rect>>,
    /// Position globale de la fenêtre d'aperçu (la vidéo s'y pose quand
    /// aucun écran Panorama n'a de zone Vidéo).
    pub preview_at: Option<(i32, i32)>,
    /// Réglages : nouvelle source (nom, adresse) en cours de saisie.
    pub draft: (String, String),
    /// Réglages : navigateur en cours de saisie.
    pub browser_draft: Option<String>,
    pub design_drafts: HashMap<String, String>,
    pub design_error: Option<String>,
    pub selected_tile: usize,
    pub shortcut_draft: (String, String, String),
    /// Réglages → Panorama : disposition dont on règle les zones.
    pub edit_layout: Option<crate::panorama::config::Layout>,
    /// Écran où le choix de la disposition est ouvert (geste de bas en haut).
    pub picker: Option<String>,
    /// Réglages : `hyprdesk panorama icons` en cours.
    pub icons_job: Option<String>,
    /// Compte rendu de la dernière génération d'icônes.
    pub icons_report: Option<Result<String, String>>,
    pub history: HashMap<String, std::collections::VecDeque<f32>>,
    pub wallpaper: Option<(String, image::Handle, iced::Size)>,
    pub wallpaper_requested: String,
}

#[derive(Debug, Clone)]
pub enum PMsg {
    Shortcut(usize),
    /// Profil du bloc de raccourcis (vidéo, audio, télécommande, système).
    Profile(String),
    /// Cadrage de la source ouverte : start (bascule), zoom+, zoom-, left,
    /// right, up, down, reset, done.
    Frame(&'static str),
    /// Fenêtres étiquetées du lecteur trouvées au démarrage : (adresse, classe).
    Orphans(Vec<(String, String)>),
    /// Plein écran relevé à la fermeture d'une source : (adresse, vidéo seule).
    RememberFull(String, bool),
    /// Vidéo seule mémorisée : la rétablir (fenêtre, essai).
    VideoFull(String, u8),
    ShortcutResult(Result<(), String>),
    Wallpaper(String, Option<iced::Size>),
    /// Écrans et tactiles relus.
    Detected(Vec<Screen>, Vec<String>),
    Swipe(String, i32),
    /// Geste sur l'autre axe : +1 de bas en haut (choix de la disposition),
    /// -1 de haut en bas (ferme le choix, sinon masque le Panorama).
    Cross(String, i32),
    /// Ferme le choix de la disposition.
    PickerClose,
    Goto(String, usize),
    Hide(String),
    Show(String),
    /// Basculer (ligne de commande : sans écran, le premier Panorama).
    Toggle(Option<String>),
    Accept(Id),
    Decline(Id),
    Later(Id),
    Tick,
    Media(Option<Status>),
    Sys(Vec<crate::widgets::SysLine>),
    /// Commande MPRIS (`PlayPause`, `Next`, `Previous`).
    Control(&'static str),
    Pressed,
    Touch(Result<(), String>),
    /// Ouvre l'aperçu au format donné (taille simulée), ou le ferme s'il
    /// est ouvert au même format.
    Preview((u32, u32)),
    ClosePreview,
    /// Disposition : une page, 2 ou 3 zones.
    Layout(crate::panorama::config::Layout),
    /// Widget suivant (+1) ou précédent (-1) de la zone.
    Pane(usize, i32),
    /// Ouvre la source vidéo n° (liste `sources`).
    VideoOpen(usize),
    /// Recherche de la fenêtre lancée (essai n°).
    VideoPoll(u32),
    VideoFound(Option<String>),
    VideoPlaced(Result<(), String>),
    VideoClose,
    PreviewAt(Option<(i32, i32)>),
}

fn wrap(m: PMsg) -> Msg {
    Msg::Panorama(m)
}

/// Écrans et tactiles décrits par Hyprland (bloquant).
fn detect_now() -> (Vec<Screen>, Vec<String>) {
    use hyprpanel_core::ipc::query_json;
    let screens = query_json("monitors")
        .map(|v| detect::parse_screens(&v))
        .unwrap_or_default();
    let touch = query_json("devices")
        .map(|v| detect::parse_touch(&v))
        .unwrap_or_default();
    (screens, touch)
}

pub fn detect_task() -> Task<Msg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(detect_now)
                .await
                .unwrap_or_default()
        },
        |(s, t)| wrap(PMsg::Detected(s, t)),
    )
}

fn fmt_time(secs: i64) -> String {
    let s = secs.max(0);
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

impl App {
    fn panorama_font(&self) -> iced::Font {
        let family = &self.cfg.general.panorama.design.font_family;
        if family.is_empty() {
            self.font
        } else {
            iced::Font::with_name(super::intern_font(family))
        }
    }
    fn panorama_palette(&self) -> (Color, Color, Color) {
        let th = &self.cfg.general.theme;
        let d = &self.cfg.general.panorama.design;
        let c = |s: &str, fallback| color(crate::config::Rgba::parse(s).unwrap_or(fallback), 1.0);
        (
            c(&d.background, th.bg),
            c(&d.foreground, th.fg),
            c(&d.accent, th.accent),
        )
    }
    fn panorama_motion(&self) -> Duration {
        if self.cfg.general.transition == crate::config::Transition::None {
            Duration::ZERO
        } else {
            Duration::from_millis(160)
        }
    }

    fn panorama_feedback<'a>(&self, content: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
        super::kinetic::magnet(
            content,
            super::bar_style::convexe::Edge::Bottom,
            self.panorama_palette().2,
            self.panorama_motion(),
        )
    }

    /// Configuration du Panorama en vigueur (réglages en cours, sinon chargée).
    pub(super) fn panorama_cfg(&self) -> crate::panorama::Config {
        self.settings
            .panorama
            .clone()
            .unwrap_or_else(|| self.cfg.general.panorama.clone())
    }

    /// Écrans branchés au rôle Panorama (connecteurs).
    pub(super) fn panorama_monitors(&self) -> Vec<String> {
        if !self.cfg.plugin_on("panorama") {
            return Vec::new();
        }
        let c = &self.cfg.general.panorama;
        self.monitors
            .iter()
            .filter(|m| {
                c.screens.iter().any(|e| e == &m.name)
                    || self
                        .panorama
                        .screens
                        .iter()
                        .find(|s| s.name == m.name)
                        .is_some_and(|s| c.has(s))
            })
            .map(|m| m.name.clone())
            .collect()
    }

    pub(super) fn is_panorama(&self, monitor: &str) -> bool {
        self.panorama_monitors().iter().any(|m| m == monitor)
    }

    /// Crée ou ferme les surfaces et poignées selon la config et les écrans.
    pub(super) fn sync_panorama(&mut self) -> Task<Msg> {
        let want = self.panorama_monitors();
        let mut tasks: Vec<Task<Msg>> = Vec::new();
        let wallpaper = self.cfg.general.panorama.design.wallpaper.clone();
        if wallpaper != self.panorama.wallpaper_requested {
            self.panorama.wallpaper_requested = wallpaper.clone();
            self.panorama.wallpaper = None;
            if !wallpaper.is_empty() {
                let path = crate::config::lua::expand_path(
                    &wallpaper,
                    self.config_path
                        .parent()
                        .unwrap_or(std::path::Path::new(".")),
                );
                tasks.push(Task::perform(
                    async move {
                        let size = tokio::task::spawn_blocking(move || {
                            ::image::image_dimensions(path).ok()
                        })
                        .await
                        .ok()
                        .flatten()
                        .map(|(w, h)| iced::Size::new(w as f32, h as f32));
                        (wallpaper, size)
                    },
                    |(p, s)| wrap(PMsg::Wallpaper(p, s)),
                ));
            }
        }
        let shown: Vec<String> = want
            .iter()
            .filter(|m| !self.panorama.hidden.contains(*m))
            .cloned()
            .collect();
        let hidden: Vec<String> = want
            .iter()
            .filter(|m| self.panorama.hidden.contains(*m))
            .cloned()
            .collect();
        for (id, m) in self.panorama.surfaces.clone() {
            if !shown.contains(&m) {
                self.panorama.surfaces.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        for (id, m) in self.panorama.handles.clone() {
            if !hidden.contains(&m) {
                self.panorama.handles.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        // Rôle donné autrement (Réglages, ligne de commande) ou écran parti :
        // la proposition n'a plus lieu d'être.
        for (id, o) in self.panorama.offers.clone() {
            if want.contains(&o.screen) || !self.monitors.iter().any(|m| m.name == o.screen) {
                self.panorama.offers.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        for m in shown {
            if self.panorama.surfaces.values().any(|s| s == &m) {
                continue;
            }
            let id = Id::unique();
            self.panorama.surfaces.insert(id, m.clone());
            self.panorama.last_press.get_or_insert_with(Instant::now);
            tracing::info!("{}", crate::tr!("Panorama sur {p0}", p0 = m.clone()));
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((0, 0)),
                    layer: Layer::Top,
                    anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(m),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
        }
        for m in hidden {
            if self.panorama.handles.values().any(|s| s == &m) {
                continue;
            }
            let id = Id::unique();
            self.panorama.handles.insert(id, m.clone());
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((HANDLE_W, HANDLE_H)),
                    layer: Layer::Top,
                    anchor: LsAnchor::Bottom,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(m),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
        }
        if !self.panorama.surfaces.is_empty() || self.panorama.preview.is_some() {
            for icon in self.panorama_icon_names() {
                tasks.push(self.want_icon(&icon));
            }
        }
        Task::batch(tasks)
    }

    /// Icônes à résoudre pour le bandeau : commandes, touches de chaque
    /// profil, raccourcis de la personne, sources vidéo et musique.
    fn panorama_icon_names(&self) -> Vec<String> {
        let c = &self.cfg.general.panorama;
        let mut out: Vec<String> = ICONS.iter().map(|n| n.to_string()).collect();
        // Cadrage du lecteur, Lecture/Pause.
        out.extend(
            [
                "zoom-in-symbolic",
                "zoom-out-symbolic",
                "view-refresh-symbolic",
                "object-select-symbolic",
                "media-playback-pause-symbolic",
            ]
            .map(String::from),
        );
        for id in crate::panorama::design::PROFILES {
            out.extend(
                crate::panorama::design::profile_shortcuts(id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| s.icon),
            );
        }
        out.extend(c.design.shortcuts.iter().map(|s| s.icon.clone()));
        out.extend(
            c.sources
                .iter()
                .chain(&c.music_sources)
                .map(|s| s.icon.clone())
                .filter(|i| !i.is_empty()),
        );
        out.sort();
        out.dedup();
        out
    }

    /// Propositions à afficher (écrans candidats nouveaux).
    fn panorama_offers(&mut self) -> Task<Msg> {
        if !self.cfg.plugin_on("panorama") {
            return Task::none();
        }
        let c = self.panorama_cfg();
        let list = detect::offers(
            &self.panorama.screens,
            &self.panorama.touch,
            &c.screens,
            &c.declined,
        );
        let mut tasks = Vec::new();
        for o in list {
            let key = if o.edid.is_empty() {
                o.screen.clone()
            } else {
                o.edid.clone()
            };
            if !self.panorama.offered.insert(key) {
                continue;
            }
            let Some(m) = self.monitors.iter().find(|m| m.name == o.screen) else {
                continue;
            };
            let w = (m.width.saturating_sub(32)).min(900);
            let h = (m.height.saturating_sub(32)).min(260);
            // Un tactile sûr : il est rattaché tout de suite, pour qu'on
            // puisse répondre au doigt.
            if let Some(_t) = &o.touch {
                let (path, out) = (self.config_path.clone(), o.screen.clone());
                tasks.push(Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::panorama::map_touch(&path, Some(&out))
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::Touch(r)),
                ));
            }
            let id = Id::unique();
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((w, h)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::empty(),
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(o.screen.clone()),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
            self.panorama.offers.insert(id, o);
        }
        Task::batch(tasks)
    }

    /// Enregistre une nouvelle configuration du Panorama.
    fn panorama_save(&mut self, c: crate::panorama::Config) -> Task<Msg> {
        self.settings.panorama = Some(c);
        self.commit()
    }

    /// Traite un message, puis remet zone de saisie et fenêtre vidéo à leur
    /// place (sans rien envoyer si rien n'a bougé).
    pub(super) fn panorama_update(&mut self, m: PMsg) -> Task<Msg> {
        let t = self.panorama_update_inner(m);
        Task::batch([t, self.sync_pip()])
    }

    fn panorama_update_inner(&mut self, m: PMsg) -> Task<Msg> {
        match m {
            PMsg::Wallpaper(path, size) => {
                if path == self.panorama.wallpaper_requested {
                    if let Some(size) = size {
                        let full = crate::config::lua::expand_path(
                            &path,
                            self.config_path
                                .parent()
                                .unwrap_or(std::path::Path::new(".")),
                        );
                        self.panorama.wallpaper =
                            Some((path, image::Handle::from_path(full), size));
                    } else {
                        self.panorama.design_error = Some(crate::tr!(
                            "Image introuvable ou illisible : {path}",
                            path = path
                        ));
                    }
                }
                Task::none()
            }
            PMsg::Shortcut(i) => {
                self.panorama.last_press = Some(Instant::now());
                let list = self.cfg.general.panorama.design.active_shortcuts();
                let Some(s) = list.get(i) else {
                    return Task::none();
                };
                let command = s.command.clone();
                drop(list);
                // Profils Vidéo et Musique : les touches vont au lecteur ; son
                // plein écran (vidéo seule) est mémorisé pour la source.
                let player = matches!(
                    self.cfg.general.panorama.design.shortcut_profile.as_str(),
                    "video" | "audio"
                );
                if player
                    && let Some(key) = command.trim().strip_prefix("hyprdesk key ")
                    && let Some(v) = self.panorama.video.as_ref()
                    && let Some(addr) = v.address.clone()
                {
                    // Plein écran (touche f) : mémorisé pour la source, rétabli
                    // à l'ouverture.
                    let key = key.trim().to_string();
                    let url = v.url.clone();
                    let mut save = Task::none();
                    if key == "f" {
                        let mut c = self.panorama_cfg();
                        if let Some(src) = c
                            .sources
                            .iter_mut()
                            .chain(c.music_sources.iter_mut())
                            .find(|s| s.url == url)
                        {
                            src.full = !src.full;
                            save = self.panorama_save(c);
                        }
                    }
                    return Task::batch([
                        save,
                        Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    crate::panorama::video::send_key(&addr, &key)
                                })
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()))
                            },
                            |r| wrap(PMsg::ShortcutResult(r)),
                        ),
                    ]);
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::hypr::exec(&command).map_err(|e| e.to_string())
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::ShortcutResult(r)),
                )
            }
            PMsg::ShortcutResult(r) => {
                self.panorama.design_error = r.err();
                Task::none()
            }
            m @ (PMsg::Layout(_)
            | PMsg::Profile(_)
            | PMsg::VideoFull(..)
            | PMsg::Frame(_)
            | PMsg::Orphans(_)
            | PMsg::RememberFull(..)
            | PMsg::Pane(..)
            | PMsg::VideoOpen(_)
            | PMsg::VideoPoll(_)
            | PMsg::VideoFound(_)
            | PMsg::VideoPlaced(_)
            | PMsg::VideoClose) => self.panorama_panes_update(m),
            PMsg::PreviewAt(at) => {
                self.panorama.preview_at = at;
                Task::none()
            }
            PMsg::Detected(screens, touch) => {
                self.panorama.screens = screens;
                self.panorama.touch = touch;
                let offers = self.panorama_offers();
                Task::batch([self.sync_bars(), self.sync_panorama(), offers])
            }
            PMsg::Cross(mon, dir) => {
                self.panorama.last_press = Some(Instant::now());
                if dir > 0 {
                    self.panorama.picker = Some(mon);
                    Task::none()
                } else if self.panorama.picker.take().is_some() {
                    Task::none()
                } else {
                    self.panorama_update(PMsg::Hide(mon))
                }
            }
            PMsg::PickerClose => {
                self.panorama.picker = None;
                Task::none()
            }
            PMsg::Swipe(mon, dir) => {
                let n = self.panorama_carousel() as i64;
                let p = self.panorama.page.entry(mon).or_insert(0);
                *p = (*p as i64 + dir as i64).rem_euclid(n) as usize;
                Task::none()
            }
            PMsg::Goto(mon, i) => {
                self.panorama.page.insert(mon, i);
                Task::none()
            }
            PMsg::Hide(mon) if mon == PREVIEW => self.panorama_update(PMsg::ClosePreview),
            PMsg::Hide(mon) => {
                self.panorama.hidden.insert(mon);
                self.sync_panorama()
            }
            PMsg::Show(mon) => {
                self.panorama.hidden.remove(&mon);
                self.panorama.last_press = Some(Instant::now());
                self.sync_panorama()
            }
            PMsg::Toggle(mon) => {
                let list = self.panorama_monitors();
                let Some(mon) = mon.or_else(|| list.first().cloned()) else {
                    return Task::none();
                };
                if self.panorama.hidden.contains(&mon) {
                    self.panorama_update(PMsg::Show(mon))
                } else {
                    self.panorama_update(PMsg::Hide(mon))
                }
            }
            PMsg::Accept(id) => {
                let Some(o) = self.panorama.offers.remove(&id) else {
                    return Task::none();
                };
                let mut c = self.panorama_cfg();
                let key = if o.edid.is_empty() {
                    o.screen.clone()
                } else {
                    o.edid.clone()
                };
                c.declined.retain(|d| d != &key);
                if !c.screens.contains(&key) {
                    c.screens.push(key);
                }
                // Tactile présent : il suit l'écran choisi (« Oui » touché ici).
                let touch = if self.panorama.touch.is_empty() {
                    Task::none()
                } else {
                    let (path, out) = (self.config_path.clone(), o.screen.clone());
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                crate::panorama::map_touch(&path, Some(&out))
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |r| wrap(PMsg::Touch(r)),
                    )
                };
                Task::batch([iced::window::close(id), self.panorama_save(c), touch])
            }
            PMsg::Decline(id) => {
                let Some(o) = self.panorama.offers.remove(&id) else {
                    return Task::none();
                };
                let mut c = self.panorama_cfg();
                let key = if o.edid.is_empty() { o.screen } else { o.edid };
                if !c.declined.contains(&key) {
                    c.declined.push(key);
                }
                Task::batch([iced::window::close(id), self.panorama_save(c)])
            }
            PMsg::Later(id) => {
                self.panorama.offers.remove(&id);
                iced::window::close(id)
            }
            PMsg::Tick => {
                if !self.panorama_ticking() {
                    return Task::none();
                }
                self.panorama_video_gone();
                let mut strays = self.panorama_close_strays();
                if !self.panorama.orphans_checked {
                    self.panorama.orphans_checked = true;
                    strays = Task::batch([
                        strays,
                        Task::perform(
                            async {
                                tokio::task::spawn_blocking(crate::panorama::video::tagged)
                                    .await
                                    .unwrap_or_default()
                            },
                            |l| wrap(PMsg::Orphans(l)),
                        ),
                    ]);
                }
                // Vidéo posée dans l'aperçu : la fenêtre suit la sienne.
                // (dès l'ouverture : la position est connue quand elle apparaît).
                let follow = if self.panorama.preview.is_some() && self.panorama.video.is_some() {
                    Task::perform(
                        async {
                            tokio::task::spawn_blocking(|| {
                                let c = hyprpanel_core::ipc::query_json("clients").ok()?;
                                c.as_array()?.iter().find_map(|w| {
                                    (w["class"].as_str() == Some(PREVIEW_APP_ID)).then(|| {
                                        (
                                            w["at"][0].as_i64().unwrap_or(0) as i32,
                                            w["at"][1].as_i64().unwrap_or(0) as i32,
                                        )
                                    })
                                })
                            })
                            .await
                            .ok()
                            .flatten()
                        },
                        |at| wrap(PMsg::PreviewAt(at)),
                    )
                } else {
                    Task::none()
                };
                let media = Task::batch([
                    follow,
                    strays,
                    Task::perform(crate::widgets::media::status(), |s| wrap(PMsg::Media(s))),
                ]);
                let wants_sys = (self.cfg.general.panorama.layout
                    == crate::panorama::config::Layout::Dashboard
                    && self
                        .cfg
                        .general
                        .panorama
                        .design
                        .active_tiles(1600.0, 450.0)
                        .iter()
                        .any(|t| t.widget == "system"))
                    || self
                        .panorama
                        .surfaces
                        .values()
                        .map(String::as_str)
                        .chain(self.panorama.preview.map(|_| PREVIEW))
                        .any(|m| self.panorama_page_id(m) == "system")
                    || (self.cfg.general.panorama.layout
                        != crate::panorama::config::Layout::Single
                        && self
                            .cfg
                            .general
                            .panorama
                            .panes_of(self.cfg.general.panorama.layout)
                            .iter()
                            .any(|p| p == "system"));
                if !wants_sys {
                    return media;
                }
                let metrics = self.cfg.general.panorama.design.metrics.clone();
                let sys = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::panorama::telemetry::sample(&metrics)
                        })
                        .await
                        .unwrap_or_default()
                    },
                    |l| wrap(PMsg::Sys(l)),
                );
                Task::batch([media, sys])
            }
            PMsg::Media(s) => {
                // Pochette locale (file://) : chargée une fois par adresse.
                let art = s
                    .as_ref()
                    .and_then(|s| s.art.strip_prefix("file://"))
                    .map(str::to_string);
                match art {
                    Some(path) if self.panorama.cover.as_ref().map(|c| &c.0) != Some(&path) => {
                        let h = image::Handle::from_path(&path);
                        self.panorama.cover = Some((path, h));
                    }
                    None => self.panorama.cover = None,
                    _ => {}
                }
                self.panorama.media = s;
                Task::none()
            }
            PMsg::Sys(l) => {
                let limit = self.cfg.general.panorama.design.history_seconds as usize;
                for v in &l {
                    if let Some(r) = v.ratio {
                        let h = self.panorama.history.entry(v.label.clone()).or_default();
                        h.push_back(r);
                        while h.len() > limit {
                            h.pop_front();
                        }
                    }
                }
                self.panorama.sys = l;
                Task::none()
            }
            PMsg::Control(method) => {
                let Some(bus) = self.panorama.media.as_ref().map(|s| s.bus.clone()) else {
                    return Task::none();
                };
                if method == "PlayPause"
                    && let Some(s) = self.panorama.media.as_mut()
                {
                    s.playing = !s.playing;
                }
                Task::perform(crate::widgets::media::media_control(bus, method), |_| {
                    wrap(PMsg::Tick)
                })
            }
            PMsg::Pressed => {
                self.panorama.last_press = Some(Instant::now());
                Task::none()
            }
            PMsg::Preview(size) => {
                if let Some(p) = self.panorama.preview.take() {
                    let close = iced::window::close(p.win);
                    if p.virtual_size == size {
                        return close;
                    }
                    return Task::batch([close, self.panorama_open_preview(size)]);
                }
                self.panorama_open_preview(size)
            }
            PMsg::ClosePreview => match self.panorama.preview.take() {
                Some(p) => iced::window::close(p.win),
                None => Task::none(),
            },
            PMsg::Touch(r) => {
                if let Err(e) = r {
                    tracing::warn!(
                        "{}",
                        crate::tr!("Panorama : tactile non rattaché : {e}", e = e)
                    );
                }
                Task::none()
            }
        }
    }

    /// `hyprdesk panorama <toggle|show|hide|page <n|id>|detect>`.
    pub(super) fn panorama_command(&mut self, a: &str) -> Task<Msg> {
        let mut it = a.split_whitespace();
        let first = self.panorama_monitors().into_iter().next();
        match (it.next().unwrap_or("toggle"), first) {
            ("detect", _) => {
                // Nouvelle passe : les écrans déjà proposés peuvent l'être encore.
                self.panorama.offered.clear();
                detect_task()
            }
            ("layout", _) => match it.next().and_then(crate::panorama::config::Layout::parse) {
                Some(l) => self.panorama_update(PMsg::Layout(l)),
                None => Task::none(),
            },
            ("video", _) => {
                let want = it.collect::<Vec<_>>().join(" ");
                if want == "close" || want == "stop" {
                    return self.panorama_update(PMsg::VideoClose);
                }
                let sources = self.cfg.general.panorama.active_sources();
                let i = want
                    .parse::<usize>()
                    .ok()
                    .map(|n| n.saturating_sub(1))
                    .or_else(|| {
                        let w = want.to_lowercase();
                        sources.iter().position(|s| {
                            s.name.to_lowercase() == w
                                || crate::panorama::video::host(&s.url).contains(&w)
                        })
                    });
                match i {
                    Some(i) if i < sources.len() => self.panorama_update(PMsg::VideoOpen(i)),
                    _ => Task::none(),
                }
            }
            ("preview", _) => {
                let size = it.next().and_then(parse_size).unwrap_or(PREVIEW_SIZE);
                self.panorama_update(PMsg::Preview(size))
            }
            ("toggle", m) => self.panorama_update(PMsg::Toggle(m)),
            ("show", Some(m)) => self.panorama_update(PMsg::Show(m)),
            ("hide", Some(m)) => self.panorama_update(PMsg::Hide(m)),
            ("page", Some(m)) => {
                let want = it.next().unwrap_or("");
                let pages = &self.cfg.general.panorama.pages;
                let i = want
                    .parse::<usize>()
                    .ok()
                    .map(|n| n.saturating_sub(1))
                    .or_else(|| pages.iter().position(|p| p == want));
                match i {
                    Some(i) if i < pages.len() => {
                        let i = i + self.panorama_offset();
                        self.panorama.hidden.remove(&m);
                        self.panorama.page.insert(m, i);
                        self.sync_panorama()
                    }
                    _ => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }

    /// Ouvre la fenêtre d'aperçu (règle flottante posée d'abord).
    fn panorama_open_preview(&mut self, size: (u32, u32)) -> Task<Msg> {
        let screen = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .or(self.monitors.first())
            .map(|m| (m.width, m.height))
            .unwrap_or((1920, 1080));
        let ((w, h), scale) = preview_fit(size, screen);
        let win = Id::unique();
        self.panorama.preview = Some(Preview {
            win,
            virtual_size: size,
            scale,
        });
        self.panorama.last_press = Some(Instant::now());
        let lua = preview_rule_lua(w, h);
        let mut tasks: Vec<Task<Msg>> = self
            .panorama_icon_names()
            .iter()
            .map(|n| self.want_icon(n))
            .collect();
        tasks.push(Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || hyprpanel_core::ipc::eval(&lua)).await;
            },
            move |_| Msg::NewBaseWindow {
                settings: iced_layershell::actions::IcedXdgWindowSettings {
                    title: Some(preview_title(size)),
                    app_id: Some(PREVIEW_APP_ID.into()),
                    size: Some((w, h)),
                    client_side_decorations: false,
                },
                id: win,
            },
        ));
        tasks.push(Task::done(wrap(PMsg::Tick)));
        Task::batch(tasks)
    }

    /// Titre de la fenêtre d'aperçu (aucun pour les autres surfaces).
    pub(super) fn panorama_title(&self, id: Id) -> Option<String> {
        self.panorama
            .preview
            .filter(|p| p.win == id)
            .map(|p| preview_title(p.virtual_size))
    }

    /// Relevés utiles : un Panorama ou son aperçu est affiché.
    pub(super) fn panorama_ticking(&self) -> bool {
        !self.panorama.surfaces.is_empty() || self.panorama.preview.is_some()
    }

    /// Surface fermée par le compositeur (écran débranché…) ou fenêtre
    /// d'aperçu fermée par l'utilisateur.
    pub(super) fn panorama_closed(&mut self, id: Id) -> bool {
        if self.panorama.preview.is_some_and(|p| p.win == id) {
            self.panorama.preview = None;
            return true;
        }
        self.panorama.surfaces.remove(&id).is_some()
            || self.panorama.handles.remove(&id).is_some()
            || self.panorama.offers.remove(&id).is_some()
    }

    pub(super) fn panorama_owns(&self, id: Id) -> bool {
        self.panorama.preview.is_some_and(|p| p.win == id)
            || self.panorama.surfaces.contains_key(&id)
            || self.panorama.handles.contains_key(&id)
            || self.panorama.offers.contains_key(&id)
    }

    /// Vue composée en tête du carrousel (découpage, composition libre) :
    /// 1, sinon 0 (une page à la fois).
    fn panorama_offset(&self) -> usize {
        usize::from(self.cfg.general.panorama.layout != crate::panorama::config::Layout::Single)
    }

    /// Entrées du carrousel d'un écran : la vue composée, puis les pages.
    pub(super) fn panorama_carousel(&self) -> usize {
        self.cfg.general.panorama.pages.len().max(1) + self.panorama_offset()
    }

    /// Disposition affichée sur l'écran `key` : la configurée sur la vue
    /// composée (entrée 0), une page à la fois sur les pages qui suivent.
    pub(super) fn panorama_layout_at(&self, key: &str) -> crate::panorama::config::Layout {
        let i = self.panorama.page.get(key).copied().unwrap_or(0);
        if i >= self.panorama_offset() {
            crate::panorama::config::Layout::Single
        } else {
            self.cfg.general.panorama.layout
        }
    }

    /// Page affichée sur un écran (la veille passe d'elle-même à l'horloge).
    fn panorama_page_id(&self, monitor: &str) -> &str {
        let pages = &self.cfg.general.panorama.pages;
        let i = self
            .panorama
            .page
            .get(monitor)
            .copied()
            .unwrap_or(0)
            .saturating_sub(self.panorama_offset());
        pages
            .get(i.min(pages.len().saturating_sub(1)))
            .map(String::as_str)
            .unwrap_or("media")
    }

    /// Veille : rien ne joue et personne n'a touché depuis `sleep_minutes`.
    fn panorama_asleep(&self) -> bool {
        let mins = self.cfg.general.panorama.sleep_minutes;
        if mins == 0 || self.panorama.media.as_ref().is_some_and(|m| m.playing) {
            return false;
        }
        self.panorama
            .last_press
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(mins as u64 * 60))
            && self.cfg.general.panorama.pages.iter().any(|p| p == "clock")
    }

    pub(super) fn view_panorama(&self, id: Id) -> Element<'_, Msg> {
        if let Some(m) = self.panorama.handles.get(&id) {
            return self.view_panorama_handle(m);
        }
        if let Some(o) = self.panorama.offers.get(&id) {
            return self.view_panorama_offer(id, o);
        }
        self.view_panorama_surface(id)
    }

    fn panorama_dots<'a>(
        &self,
        n: usize,
        cur: usize,
        mon: &str,
        fg: Color,
        accent: Color,
    ) -> Element<'a, Msg> {
        let items: Vec<Element<'a, Msg>> = (0..n)
            .map(|i| {
                let on = i == cur;
                button(
                    container(Space::new().width(if on { 26 } else { 10 }).height(10)).style(
                        move |_| container::Style {
                            background: Some(
                                if on { accent } else { Color { a: 0.35, ..fg } }.into(),
                            ),
                            border: Border::default().rounded(999),
                            ..Default::default()
                        },
                    ),
                )
                .padding(10)
                .on_press(wrap(PMsg::Goto(mon.to_string(), i)))
                .style(|_, _| button::Style::default())
                .into()
            })
            .collect();
        row(items).spacing(2).into()
    }

    fn panorama_button<'a>(&self, icon: &str, size: f32, fg: Color, msg: Msg) -> Element<'a, Msg> {
        let accent = self.panorama_palette().2;
        let primary = icon.starts_with("media-playback-");
        let ink = if primary {
            super::design::on_accent(accent, fg, color(self.cfg.general.theme.bg, 1.0))
        } else {
            fg
        };
        let glyph = self
            .icon_named(icon, size * 0.45, ink)
            .unwrap_or_else(|| text("•").size(size * 0.4).color(fg).into());
        self.panorama_feedback(
            button(
                container(glyph)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .width(size)
            .height(size)
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: Some(
                    Color {
                        a: if primary {
                            1.0
                        } else {
                            match st {
                                button::Status::Pressed => 0.28,
                                button::Status::Hovered => 0.16,
                                _ => 0.08,
                            }
                        },
                        ..if primary { accent } else { fg }
                    }
                    .into(),
                ),
                border: Border {
                    color: fx::alpha(if primary { accent } else { fg }, 0.22),
                    width: 1.0,
                    radius: (size / 2.0).into(),
                },
                shadow: iced::Shadow {
                    color: fx::alpha(accent, if primary { 0.24 } else { 0.0 }),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 18.0,
                },
                ..Default::default()
            }),
        )
    }

    fn panorama_media(&self, z: &Zones, fg: Color) -> Element<'_, Msg> {
        let accent = self.panorama_palette().2;
        let dim = Color { a: 0.6, ..fg };
        let art_side = z.art.w;
        let bg = self.panorama_palette().0;
        let face = (art_side - 16.0).max(1.0);
        let art: Element<'_, Msg> = match &self.panorama.cover {
            Some((_, h)) => image(h.clone())
                .width(face)
                .height(face)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(18.0)
                .into(),
            None => fx::album(face, bg, accent),
        };
        let art = self.panorama_feedback(
            container(art)
                .padding(8)
                .width(art_side)
                .height(art_side)
                .style(move |_| fx::glass(bg, fg, accent, 24.0))
                .clip(true),
        );
        let tgt = z.target;
        let short = z.unit;
        let (title, sub, playing, pos, len) = match &self.panorama.media {
            Some(s) => (
                if s.title.is_empty() {
                    s.player.clone()
                } else {
                    s.title.clone()
                },
                [s.artist.as_str(), s.album.as_str()]
                    .iter()
                    .filter(|x| !x.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" · "),
                s.playing,
                s.position,
                s.length,
            ),
            None => (
                crate::i18n::tr("Rien ne joue").to_string(),
                crate::i18n::tr("Lancez un lecteur (Spotify, Firefox, mpv…)").to_string(),
                false,
                None,
                None,
            ),
        };
        let progress: Element<'_, Msg> = match (pos, len) {
            // Un direct annonce parfois une durée absurde : rien plutôt que faux.
            (Some(p), Some(l)) if l > 0 && l < 48 * 3600 && (0..=l).contains(&p) => {
                let r = (p as f32 / l as f32).clamp(0.0, 1.0);
                let filled = (r * 1000.0) as u16;
                let bar = |portion: u16, c: Option<Color>| -> Element<'_, Msg> {
                    container(Space::new())
                        .width(Length::FillPortion(portion.max(1)))
                        .height(8)
                        .style(move |_| container::Style {
                            background: c.map(Into::into),
                            border: Border::default().rounded(4),
                            ..Default::default()
                        })
                        .into()
                };
                column![
                    row![
                        bar(filled, Some(accent)),
                        bar(1000 - filled, Some(Color { a: 0.15, ..fg }))
                    ],
                    row![
                        text(fmt_time(p)).size(short * 0.045).color(dim),
                        Space::new().width(Length::Fill),
                        text(fmt_time(l)).size(short * 0.045).color(dim)
                    ]
                ]
                .spacing(6)
                .into()
            }
            _ => Space::new().into(),
        };
        let controls = row![
            self.panorama_button(
                "media-skip-backward-symbolic",
                tgt,
                fg,
                wrap(PMsg::Control("Previous"))
            ),
            self.panorama_button(
                if playing {
                    "media-playback-pause-symbolic"
                } else {
                    "media-playback-start-symbolic"
                },
                tgt * 1.35,
                fg,
                wrap(PMsg::Control("PlayPause"))
            ),
            self.panorama_button(
                "media-skip-forward-symbolic",
                tgt,
                fg,
                wrap(PMsg::Control("Next"))
            ),
        ]
        .spacing(tgt * 0.4)
        .align_y(Alignment::Center);
        let info = column![
            container(
                row![
                    container(Space::new())
                        .width(6)
                        .height(6)
                        .style(move |_| container::Style {
                            background: Some(if playing { accent } else { dim }.into()),
                            border: Border::default().rounded(3),
                            ..Default::default()
                        }),
                    text(crate::i18n::tr(if playing {
                        "En lecture"
                    } else {
                        "Média"
                    }))
                    .size((short * 0.032).clamp(11.0, 20.0))
                    .color(dim)
                    .font(self.panorama_font())
                ]
                .spacing(9)
                .align_y(Alignment::Center)
            )
            .padding([6, 10])
            .style(move |_| fx::glass(bg, fg, accent, 20.0)),
            text(title)
                .size((short * 0.095).clamp(20.0, 64.0))
                .color(fg)
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..self.panorama_font()
                })
                .wrapping(text::Wrapping::None),
            text(sub)
                .size((short * 0.045).clamp(13.0, 30.0))
                .color(dim)
                .font(self.panorama_font())
                .wrapping(text::Wrapping::None),
            Space::new().height(Length::Fill),
            progress,
            Space::new().height(z.pad / 2.0),
            controls,
        ]
        .spacing(8)
        .width(z.main.w)
        .height(z.main.h)
        .clip(true);
        let side: Element<'_, Msg> = match (z.side, &self.panorama.media) {
            (Some(r), Some(s)) => column![
                text(crate::i18n::tr("Lecteur"))
                    .size(short * 0.04)
                    .color(dim)
                    .font(self.panorama_font()),
                text(s.player.split('.').next().unwrap_or(&s.player).to_string())
                    .size(short * 0.06)
                    .color(fg)
                    .font(self.panorama_font()),
            ]
            .spacing(6)
            .width(r.w)
            .height(r.h)
            .into(),
            (Some(r), None) => Space::new().width(r.w).height(r.h).into(),
            (None, _) => Space::new().width(0).into(),
        };
        let pad = z.pad;
        if z.compact {
            if z.art.w < 1.0 {
                return container(info).padding(pad).into();
            }
            return column![art, info].spacing(pad).padding(pad).into();
        }
        match z.orientation {
            Orientation::Landscape => row![art, info, side]
                .spacing(pad)
                .padding(pad)
                .align_y(Alignment::Start)
                .into(),
            Orientation::Portrait => column![art, info, side]
                .spacing(pad)
                .padding(pad)
                .align_x(Alignment::Center)
                .into(),
        }
    }

    fn panorama_system(
        &self,
        z: &Zones,
        fg: Color,
        size: iced::Size,
        reserve_controls: bool,
    ) -> Element<'_, Msg> {
        let accent = self.panorama_palette().2;
        let dim = Color { a: 0.6, ..fg };
        let short = z.unit;
        let columns = if z.compact {
            2
        } else if z.orientation == Orientation::Landscape {
            self.panorama.sys.len().max(1)
        } else {
            1
        };
        let rows = if z.compact {
            self.panorama.sys.len().div_ceil(2).max(1)
        } else if z.orientation == Orientation::Portrait {
            self.panorama.sys.len().max(1)
        } else {
            1
        };
        let footer = if reserve_controls { z.target } else { 0.0 };
        let card_width = (size.width - z.pad * (columns as f32 + 1.0)) / columns as f32;
        let card_height = (size.height - z.pad * (rows as f32 + 1.0) - footer) / rows as f32;
        let gauge_height = if card_height < 110.0 { 5.0 } else { 10.0 };
        let cards: Vec<Element<'_, Msg>> = self
            .panorama
            .sys
            .iter()
            .map(|l| {
                let bg = self.panorama_palette().0;
                let gauge: Element<'_, Msg> = match l.ratio {
                    Some(r) => {
                        let f = (r.clamp(0.0, 1.0) * 1000.0) as u16;
                        row![
                            container(Space::new())
                                .width(Length::FillPortion(f.max(1)))
                                .height(gauge_height)
                                .style(move |_| container::Style {
                                    background: Some(accent.into()),
                                    border: Border::default().rounded(5),
                                    ..Default::default()
                                }),
                            container(Space::new())
                                .width(Length::FillPortion((1000 - f).max(1)))
                                .height(gauge_height)
                                .style(move |_| container::Style {
                                    background: Some(Color { a: 0.15, ..fg }.into()),
                                    border: Border::default().rounded(5),
                                    ..Default::default()
                                }),
                        ]
                        .into()
                    }
                    None => Space::new().into(),
                };
                self.panorama_feedback(
                    container(
                        column![
                            text(l.label.clone())
                                .size((short * 0.05).min(card_height * 0.18).clamp(11.0, 24.0))
                                .color(dim)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Semibold,
                                    ..self.panorama_font()
                                }),
                            text(l.value.clone())
                                .size(
                                    (short * 0.09)
                                        .min(card_width * 0.11)
                                        .min(card_height * 0.28)
                                        .clamp(14.0, 42.0)
                                )
                                .color(fg)
                                .wrapping(text::Wrapping::None)
                                .font(self.panorama_font()),
                            Space::new().height(Length::Fill),
                            gauge,
                        ]
                        .spacing((card_height * 0.05).clamp(2.0, 6.0)),
                    )
                    .padding(z.pad.min((card_height * 0.14).max(4.0)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| fx::glass(bg, fg, accent, 20.0)),
                )
            })
            .collect();
        let pad = z.pad;
        let body: Element<'_, Msg> = if z.compact {
            // Zone presque carrée : une grille de deux colonnes.
            let mut col = column![].spacing(pad);
            let mut it = cards.into_iter().peekable();
            while it.peek().is_some() {
                let mut line = row![].spacing(pad);
                for _ in 0..2 {
                    line = line.push(
                        it.next()
                            .unwrap_or_else(|| Space::new().width(Length::Fill).into()),
                    );
                }
                col = col.push(line);
            }
            col.into()
        } else {
            match z.orientation {
                Orientation::Landscape => row(cards).spacing(pad).into(),
                Orientation::Portrait => column(cards).spacing(pad).into(),
            }
        };
        container(body)
            .padding(iced::Padding {
                top: pad,
                left: pad,
                right: pad,
                bottom: pad + footer,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn panorama_clock(&self, z: &Zones, fg: Color) -> Element<'_, Msg> {
        let d = &self.cfg.general.panorama.design;
        if d.clock_style != "digital"
            || self.cfg.general.panorama.layout == crate::panorama::config::Layout::Dashboard
            || d.seconds
            || !d.date
            || d.text_scale != 1.0
        {
            return instruments::clock(
                &d.clock_style,
                d.seconds,
                d.date,
                fg,
                self.panorama_palette().2,
                self.panorama_font(),
                d.text_scale,
            );
        }
        let dim = Color { a: 0.6, ..fg };
        let short = z.unit;
        let now = chrono::Local::now();
        let date = now
            .format_localized("%A %e %B", crate::i18n::chrono_locale())
            .to_string();
        let mut col = column![
            text(now.format("%H:%M").to_string())
                .size(short * 0.42)
                .color(fg)
                .font(self.panorama_font()),
            text(date)
                .size(short * 0.08)
                .color(dim)
                .font(self.panorama_font()),
        ]
        .align_x(Alignment::Center)
        .spacing(4);
        if let Some(s) = self.panorama.media.as_ref().filter(|s| !s.title.is_empty()) {
            col = col.push(
                text(if s.artist.is_empty() {
                    s.title.clone()
                } else {
                    format!("{} — {}", s.title, s.artist)
                })
                .size(short * 0.05)
                .color(dim)
                .font(self.panorama_font()),
            );
        }
        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_panorama_handle(&self, mon: &str) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let bg = color(th.bg, 0.92);
        button(
            container(
                row![
                    self.icon_named("go-up-symbolic", 14.0, fg)
                        .unwrap_or_else(|| text("▴").size(14).color(fg).into()),
                    text("Panorama")
                        .size(13)
                        .color(fg)
                        .font(self.panorama_font())
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(wrap(PMsg::Show(mon.to_string())))
        .style(move |_, _| button::Style {
            background: Some(bg.into()),
            border: Border {
                radius: iced::border::Radius {
                    top_left: 13.0,
                    top_right: 13.0,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }

    fn view_panorama_offer(&self, id: Id, o: &Offer) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let accent = self.panorama_palette().2;
        let bg = color(th.bg, 0.97);
        let dim = Color { a: 0.7, ..fg };
        let what = if o.touch.is_some() {
            crate::i18n::tr(
                "Écran tactile allongé détecté. En faire un Panorama : lecteur multimédia, système et horloge, pilotés au doigt ?",
            )
        } else if o.ambiguous {
            crate::i18n::tr(
                "Écran allongé détecté. En faire un Panorama ? Touchez « Oui » sur l'écran tactile : le tactile y sera rattaché.",
            )
        } else {
            crate::i18n::tr(
                "Écran allongé détecté. En faire un Panorama : lecteur multimédia, système et horloge, en plein écran ?",
            )
        };
        let btn = |label: &'static str, primary: bool, msg: Msg| -> Element<'_, Msg> {
            button(
                text(crate::i18n::tr(label))
                    .size(18)
                    .font(self.panorama_font())
                    .color(if primary { Color::BLACK } else { fg }),
            )
            .padding([14, 26])
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: Some(
                    if primary {
                        accent
                    } else {
                        Color {
                            a: if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                                0.18
                            } else {
                                0.08
                            },
                            ..fg
                        }
                    }
                    .into(),
                ),
                border: Border::default().rounded(999),
                ..Default::default()
            })
            .into()
        };
        container(
            column![
                text(crate::i18n::tr("Panorama"))
                    .size(26)
                    .font(self.panorama_font())
                    .color(fg),
                text(what).size(16).font(self.panorama_font()).color(dim),
                Space::new().height(Length::Fill),
                row![
                    btn("Oui, en faire un Panorama", true, wrap(PMsg::Accept(id))),
                    btn("Plus tard", false, wrap(PMsg::Later(id))),
                    btn("Non merci", false, wrap(PMsg::Decline(id))),
                ]
                .spacing(12),
            ]
            .spacing(10),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(bg.into()),
            border: Border::default()
                .rounded(22)
                .width(1)
                .color(Color { a: 0.15, ..fg }),
            ..Default::default()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apercu_tient_a_l_ecran() {
        // 2560×720 sur un 2560×1440 : réduit à 85 % de la largeur.
        let ((w, h), s) = preview_fit((2560, 720), (2560, 1440));
        assert_eq!((w, h), (2176, 612));
        assert!((s - 0.85).abs() < 1e-3);
        // Assez petit : taille réelle.
        assert_eq!(preview_fit((1280, 400), (2560, 1440)), ((1280, 400), 1.0));
        // Portrait : limité par la hauteur.
        let ((w, h), _) = preview_fit((720, 2560), (2560, 1440));
        assert_eq!(h, 1224);
        assert!((w as f32 / h as f32 - 720.0 / 2560.0).abs() < 0.01);
    }

    #[test]
    fn format_lu() {
        assert_eq!(parse_size("2560x720"), Some((2560, 720)));
        assert_eq!(parse_size("720×2560"), Some((720, 2560)));
        assert_eq!(parse_size("10x10"), None);
        assert_eq!(parse_size("abc"), None);
    }

    #[test]
    fn regle_flottante() {
        let l = preview_rule_lua(1280, 360);
        assert!(l.contains(r#"class = "^(hyprdesk-panorama-preview)$""#));
        assert!(l.contains(r#"float = true, center = true, size = "1280 360""#));
        assert!(l.contains("set_enabled(false)"));
    }
}
