//! Assistant de configuration (MLG-145) : une grande carte au-dessus du
//! bureau assombri, en dix étapes visuelles — langue, écrans, clavier,
//! apparence, barre, bureau, performance, veille, applications, puis un
//! récapitulatif. Chaque choix s'applique aussitôt par les réglages
//! existants (`App::settings` puis `App::commit`, ou le message du paquet de
//! Réglages concerné) ; un instantané d'historique « assistant », pris à
//! l'ouverture, permet de tout annuler.
//!
//! Proposé au premier démarrage, relancé par le bouton « Assistant » de
//! l'en-tête des Réglages ou `hyprdesk wizard`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use iced::widget::{
    Space, button, column, container, image, mouse_area, pick_list, pin, row, scrollable, slider,
    stack, svg, text_input, toggler, tooltip,
};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Size, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::plugins::{self, input::Choice, lock::IdleStep};
use super::{App, Msg, color, design};
use crate::config::{BarPosition, BarStyle, Config, Mode, Selector, WallSource};
use crate::i18n::text;
use crate::inputs::Inputs;
use crate::perf::Profile;
use crate::render::Renderer;

pub const NAMESPACE: &str = "hyprdesk-wizard";
/// Carte : taille visée, réduite sur un petit écran.
const CARD_W: f32 = 960.0;
const CARD_H: f32 = 640.0;
const GAP: f32 = 14.0;

/// Étape : nom court (pastille, titre) et phrase d'introduction.
pub struct StepInfo {
    pub label: &'static str,
    pub summary: &'static str,
    pub icon: &'static str,
}

pub const STEPS: [StepInfo; 10] = [
    StepInfo {
        label: "Bienvenue",
        summary: "Quelques choix pour un bureau à votre goût. Chacun s'applique aussitôt ; tout se retouche ensuite dans les Réglages.",
        icon: "hyprdesk-start",
    },
    StepInfo {
        label: "Écrans",
        summary: "Cliquez l'écran principal : il accueille la barre et le bureau quand ils ne s'affichent que sur un écran.",
        icon: "video-display",
    },
    StepInfo {
        label: "Clavier",
        summary: "La disposition de votre clavier. Essayez-la dans le champ du bas.",
        icon: "input-keyboard",
    },
    StepInfo {
        label: "Apparence",
        summary: "Un thème, un fond d'écran, et des bordures de fenêtres assorties.",
        icon: "preferences-desktop-theme",
    },
    StepInfo {
        label: "Barre",
        summary: "Le style de la barre et son bord de l'écran.",
        icon: "view-app-grid",
    },
    StepInfo {
        label: "Bureau",
        summary: "Les workspaces toujours présents, l'allure du bureau affiché et les fichiers du dossier Bureau.",
        icon: "user-desktop",
    },
    StepInfo {
        label: "Performance",
        summary: "Ce que les effets de Hyprland demandent à la machine, et comment hyprdesk dessine.",
        icon: "power-profile-balanced",
    },
    StepInfo {
        label: "Veille et sécurité",
        summary: "Après combien de minutes sans activité l'écran s'assombrit, se verrouille, s'éteint, puis la machine se met en veille.",
        icon: "system-lock-screen",
    },
    StepInfo {
        label: "Applications",
        summary: "Le terminal, le navigateur et le gestionnaire de fichiers que hyprdesk ouvre.",
        icon: "applications-other",
    },
    StepInfo {
        label: "Récapitulatif",
        summary: "Vos choix, déjà en place. Une étape se rouvre d'un clic.",
        icon: "object-select",
    },
];

/// Index des étapes, pour la lisibilité.
pub const WELCOME: usize = 0;
pub const SCREENS: usize = 1;
pub const KEYBOARD: usize = 2;
pub const LOOK: usize = 3;
pub const BAR: usize = 4;
pub const DESKTOP: usize = 5;
pub const PERFORMANCE: usize = 6;
pub const IDLE: usize = 7;
pub const APPS: usize = 8;
pub const SUMMARY: usize = 9;

/// Rôles proposés à l'étape Applications.
const ROLES: [&str; 3] = ["terminal", "browser", "files"];
/// Applications montrées par rôle.
const MAX_APPS: usize = 6;
/// Fonds d'écran proposés.
const MAX_WALLS: usize = 8;

/// Dispositions courantes : code xkb, nom, rangée du haut.
const LAYOUTS: [(&str, &str, &str); 8] = [
    ("fr", "Français", "AZERTY"),
    ("be", "Belge", "AZERTY"),
    ("us", "Anglais (États-Unis)", "QWERTY"),
    ("gb", "Anglais (Royaume-Uni)", "QWERTY"),
    ("de", "Allemand", "QWERTZ"),
    ("ch", "Suisse", "QWERTZ"),
    ("es", "Espagnol", "QWERTY"),
    ("it", "Italien", "QWERTY"),
];

/// Clavier : dispositions connues, réglage en vigueur, celui d'avant.
#[derive(Debug, Clone, Default)]
pub struct Keyboard {
    pub layouts: Vec<(String, String)>,
    pub cur: Option<Inputs>,
    pub before: Option<Inputs>,
    pub try_text: String,
}

pub struct Wizard {
    pub win: Id,
    pub step: usize,
    /// Étapes déjà vues (pastilles cochées).
    pub seen: [bool; 10],
    /// Instantané d'historique pris à l'ouverture (`None` : pas encore, ou rien à garder).
    pub snapshot: Option<String>,
    /// Réglages et configuration à l'ouverture : récapitulatif (« modifié »)
    /// et annulation sans historique.
    pub before_settings: crate::settings::Settings,
    pub before: Arc<Config>,
    pub kb: Keyboard,
    pub walls: Vec<PathBuf>,
    pub thumbs: HashMap<PathBuf, image::Handle>,
    /// Annulation en cours ; message d'erreur.
    pub busy: bool,
    pub error: Option<String>,
}

impl Wizard {
    pub fn new(win: Id, settings: crate::settings::Settings, cfg: Arc<Config>) -> Self {
        let mut seen = [false; 10];
        seen[0] = true;
        Wizard {
            win,
            step: 0,
            seen,
            snapshot: None,
            before_settings: settings,
            before: cfg,
            kb: Keyboard::default(),
            walls: Vec::new(),
            thumbs: HashMap::new(),
            busy: false,
            error: None,
        }
    }

    /// Va à l'étape `i` (bornée).
    pub fn go(&mut self, i: usize) {
        self.step = i.min(STEPS.len() - 1);
        self.seen[self.step] = true;
    }

    /// Étape suivante ; `false` sur la dernière (il reste à terminer).
    pub fn next(&mut self) -> bool {
        if self.step + 1 >= STEPS.len() {
            return false;
        }
        self.go(self.step + 1);
        true
    }

    pub fn back(&mut self) {
        self.go(self.step.saturating_sub(1));
    }
}

#[derive(Debug, Clone)]
pub enum WMsg {
    Go(usize),
    Next,
    Back,
    /// Étape laissée telle quelle.
    Skip,
    /// Fin : les choix restent.
    Finish,
    /// Fin, puis la visite guidée des Réglages.
    FinishTour,
    /// Rétablit la configuration d'avant l'assistant.
    Cancel,
    Snapshot(Option<String>),
    Restored(Result<String, String>),
    Language(&'static str),
    Primary(String),
    DetectScreens,
    KbLoaded(Box<Keyboard>),
    Layout(String),
    LayoutPick(Choice),
    Try(String),
    Images(Vec<PathBuf>),
    Thumb(PathBuf, Option<image::Handle>),
    Wallpaper(PathBuf),
    Theme(usize),
    Borders(bool),
    BarStyle(BarStyle),
    BarPos(BarPosition),
    Workspaces(Option<u8>),
    Mode(Mode),
    DesktopFiles(bool),
    Profile(Profile),
    Renderer(Renderer),
    IdleManage(bool),
    Idle(IdleStep, u32),
    IdleCommit,
    App(&'static str, String),
    AppSystem(&'static str),
}

fn wrap(m: WMsg) -> Msg {
    Msg::Wizard(m)
}

/// Met `code` en tête des dispositions (`fr,us`), variantes alignées.
pub fn with_first_layout(layout: &str, variant: &str, code: &str) -> (String, String) {
    let layouts: Vec<&str> = layout
        .split(',')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let variants: Vec<&str> = variant.split(',').map(str::trim).collect();
    let mut pairs: Vec<(String, String)> = layouts
        .iter()
        .enumerate()
        .map(|(i, l)| {
            (
                l.to_string(),
                variants.get(i).copied().unwrap_or("").to_string(),
            )
        })
        .collect();
    let first = match pairs.iter().position(|(l, _)| l == code) {
        Some(i) => pairs.remove(i),
        None => (code.to_string(), String::new()),
    };
    pairs.insert(0, first);
    let l = pairs
        .iter()
        .map(|(l, _)| l.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let v = if pairs.iter().all(|(_, v)| v.is_empty()) {
        String::new()
    } else {
        pairs
            .iter()
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };
    (l, v)
}

/// Vignettes des styles de barre (captures de l'aperçu), décodées une fois.
fn bar_thumb(style: BarStyle) -> Option<image::Handle> {
    static THUMBS: OnceLock<Vec<Option<image::Handle>>> = OnceLock::new();
    let all = THUMBS.get_or_init(|| {
        [
            &include_bytes!("../../assets/wizard/bar-continuite.jpg")[..],
            &include_bytes!("../../assets/wizard/bar-graphite.jpg")[..],
            &include_bytes!("../../assets/wizard/bar-island.jpg")[..],
            &include_bytes!("../../assets/wizard/bar-convexe.jpg")[..],
        ]
        .iter()
        .map(|b| {
            let img = ::image::load_from_memory(b).ok()?.to_rgba8();
            Some(image::Handle::from_rgba(
                img.width(),
                img.height(),
                img.into_raw(),
            ))
        })
        .collect()
    });
    let i = match style {
        BarStyle::Continuite => 0,
        BarStyle::Graphite => 1,
        BarStyle::Island => 2,
        BarStyle::Convexe => 3,
    };
    all.get(i).cloned().flatten()
}

/// Instantané « assistant » (hors du fil de l'interface).
fn mark(path: PathBuf) -> Task<Msg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || match crate::history::mark(&path, "assistant") {
                Ok(id) => id,
                Err(e) => {
                    tracing::warn!("{}", crate::tr!("historique : {e}", e = e));
                    None
                }
            })
            .await
            .ok()
            .flatten()
        },
        |id| wrap(WMsg::Snapshot(id)),
    )
}

fn load_keyboard() -> Task<Msg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                let (layouts, _) = crate::inputs::layouts();
                let cur = crate::inputs::read();
                Keyboard {
                    layouts,
                    before: Some(cur.clone()),
                    cur: Some(cur),
                    try_text: String::new(),
                }
            })
            .await
            .unwrap_or_default()
        },
        |k| wrap(WMsg::KbLoaded(Box::new(k))),
    )
}

fn load_walls() -> Task<Msg> {
    Task::perform(
        async {
            let dir = plugins::wallpaper::expand(&plugins::wallpaper::default_folder());
            tokio::task::spawn_blocking(move || {
                super::list_images(&dir, false)
                    .into_iter()
                    .take(MAX_WALLS)
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default()
        },
        |l| wrap(WMsg::Images(l)),
    )
}

/// Vignette 16:9 d'un fond (hors du fil de l'interface) : rognée au
/// décodage, une image en hauteur ne déborde pas de sa carte.
pub(super) fn scene_thumb(p: &std::path::Path) -> Option<image::Handle> {
    let img = ::image::open(p)
        .ok()?
        .resize_to_fill(384, 216, ::image::imageops::FilterType::Triangle)
        .to_rgba8();
    Some(image::Handle::from_rgba(
        img.width(),
        img.height(),
        img.into_raw(),
    ))
}

fn load_thumb(p: PathBuf) -> Task<Msg> {
    Task::perform(
        async move {
            let p2 = p.clone();
            let h = tokio::task::spawn_blocking(move || scene_thumb(&p2))
                .await
                .ok()
                .flatten();
            (p, h)
        },
        |(p, h)| wrap(WMsg::Thumb(p, h)),
    )
}

/// Couleurs de la carte : texte, texte secondaire, accent, fond, encre sur l'accent.
#[derive(Clone, Copy)]
struct Pal {
    fg: Color,
    dim: Color,
    accent: Color,
    bg: Color,
    on_accent: Color,
    warn: Color,
}

/// Bouton-carte : accent quand il est choisi, reflet au survol.
fn tile_style(on: bool, p: Pal) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let hover = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(
                if on {
                    Color {
                        a: 0.16,
                        ..p.accent
                    }
                } else if hover {
                    Color { a: 0.08, ..p.fg }
                } else {
                    Color { a: 0.035, ..p.fg }
                }
                .into(),
            ),
            text_color: p.fg,
            border: Border::default()
                .rounded(14)
                .width(if on { 2 } else { 1 })
                .color(if on {
                    p.accent
                } else if hover {
                    Color { a: 0.30, ..p.fg }
                } else {
                    Color { a: 0.12, ..p.fg }
                }),
            ..Default::default()
        }
    }
}

impl App {
    // -- Ouverture, fermeture -------------------------------------------------

    /// Ouvre l'assistant sur l'écran visé ; déjà ouvert : rien.
    pub(super) fn open_wizard(&mut self) -> Task<Msg> {
        if self.wizard.is_some() {
            return Task::none();
        }
        let monitor = self.target_monitor().unwrap_or_default();
        let win = Id::unique();
        self.wizard = Some(Wizard::new(win, self.settings.clone(), self.cfg.clone()));
        // Les Réglages laissent la place : les choix se voient sur le bureau.
        let panel = if self.panel.is_some() {
            self.close_panel()
        } else {
            Task::none()
        };
        let output = if monitor.is_empty() {
            OutputOption::Active
        } else {
            OutputOption::OutputName(monitor)
        };
        let mut tasks = vec![
            panel,
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    // Plein écran, par-dessus tout ; le fond assombri
                    // reçoit le flou des Réglages (`Place::Settings`).
                    size: Some((0, 0)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option: output,
                    namespace: Some(NAMESPACE.into()),
                    ..Default::default()
                },
                id: win,
            }),
            self.wizard_icons(),
        ];
        if !self.preview {
            tasks.push(mark(self.config_path.clone()));
            tasks.push(load_keyboard());
            tasks.push(load_walls());
            if self.screens.is_empty() {
                tasks.push(Self::detect_screens());
            }
        }
        Task::batch(tasks)
    }

    fn close_wizard(&mut self) -> Task<Msg> {
        match self.wizard.take() {
            Some(w) => iced::window::close(w.win),
            None => Task::none(),
        }
    }

    /// Icônes des applications proposées (une fois, hors du fil de l'interface).
    fn wizard_icons(&mut self) -> Task<Msg> {
        let missing: Vec<String> = ROLES
            .iter()
            .filter_map(|r| crate::apps::role(r))
            .flat_map(|r| crate::apps::candidates(r, &self.apps))
            .map(|(id, _)| id)
            .filter(|c| !self.class_imgs.contains_key(c) && !self.class_pending.contains(c))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if missing.is_empty() || self.preview {
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
                            let p = crate::launch::icon_for_class(&c, &apps, theme.as_deref(), 48);
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

    // -- Messages ---------------------------------------------------------------

    pub(super) fn wizard_update(&mut self, m: WMsg) -> Task<Msg> {
        let Some(w) = self.wizard.as_mut() else {
            return Task::none();
        };
        match m {
            WMsg::Go(i) => {
                w.go(i);
                self.wizard_entered()
            }
            WMsg::Next | WMsg::Skip => {
                if w.next() {
                    self.wizard_entered()
                } else {
                    self.close_wizard()
                }
            }
            WMsg::Back => {
                w.back();
                Task::none()
            }
            WMsg::Finish => self.close_wizard(),
            WMsg::FinishTour => {
                let close = self.close_wizard();
                Task::batch([
                    close,
                    Task::done(Msg::Control(crate::control::Command::Tour)),
                ])
            }
            WMsg::Cancel => {
                if w.busy {
                    return Task::none();
                }
                if let Some(id) = w.snapshot.clone() {
                    w.busy = true;
                    w.error = None;
                    let path = self.config_path.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                crate::history::rollback(&path, &id).map(|r| r.id)
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |r| wrap(WMsg::Restored(r)),
                    );
                }
                // Sans instantané (rien n'était encore écrit) : les réglages
                // d'avant, et le clavier d'avant.
                self.settings = w.before_settings.clone();
                let kb = match (&w.kb.before, &w.kb.cur) {
                    (Some(b), Some(c)) if b != c && !self.preview => {
                        plugins::input::save(self.config_path.clone(), b.clone())
                    }
                    _ => Task::none(),
                };
                let saved = self.commit();
                let close = self.close_wizard();
                Task::batch([saved, kb, close])
            }
            WMsg::Restored(r) => match r {
                Ok(_) => {
                    let close = self.close_wizard();
                    crate::hypr::notify(
                        5,
                        4000,
                        crate::i18n::tr("hyprdesk : configuration d'avant l'assistant rétablie"),
                    );
                    Task::batch([close, Task::done(Msg::ConfigChanged)])
                }
                Err(e) => {
                    w.busy = false;
                    w.error = Some(e);
                    Task::none()
                }
            },
            WMsg::Snapshot(id) => {
                w.snapshot = id;
                Task::none()
            }
            WMsg::Language(l) => {
                self.settings.language = Some(l.to_string());
                self.commit()
            }
            WMsg::Primary(name) => {
                // L'écran principal seul : la disposition ne bouge pas (elle
                // demanderait une confirmation, Réglages → Écrans).
                self.settings.primary = Some(name);
                self.commit()
            }
            WMsg::DetectScreens => Self::detect_screens(),
            WMsg::KbLoaded(k) => {
                w.kb = *k;
                Task::none()
            }
            WMsg::LayoutPick(c) => self.wizard_update(WMsg::Layout(c.0)),
            WMsg::Layout(code) => {
                let Some(mut i) = w.kb.cur.clone() else {
                    return Task::none();
                };
                let (l, v) = with_first_layout(&i.kb_layout, &i.kb_variant, &code);
                i.kb_layout = l;
                i.kb_variant = v;
                w.kb.cur = Some(i.clone());
                if let Some(p) = self.panel.as_mut() {
                    p.input.cur = Some(i.clone());
                }
                if self.preview {
                    return Task::none();
                }
                plugins::input::save(self.config_path.clone(), i)
            }
            WMsg::Try(t) => {
                w.kb.try_text = t;
                Task::none()
            }
            WMsg::Images(list) => {
                let missing: Vec<PathBuf> = list
                    .iter()
                    .filter(|p| !w.thumbs.contains_key(*p))
                    .cloned()
                    .collect();
                w.walls = list;
                Task::batch(missing.into_iter().map(load_thumb))
            }
            WMsg::Thumb(p, h) => {
                if let Some(h) = h {
                    w.thumbs.insert(p, h);
                }
                Task::none()
            }
            WMsg::Wallpaper(p) => {
                // Pour tous les bureaux : une règle par workspace reste prioritaire.
                let sel = Selector::default();
                let fit = self.current_fit(&sel);
                self.settings.set_wallpaper(sel, WallSource::Path(p), fit);
                self.commit()
            }
            WMsg::Theme(i) => self.theme_update(plugins::theme::Msg::Preset(i)),
            WMsg::Borders(v) => self.hyprset_update(super::hyprset::Msg::ThemeBorders(v)),
            WMsg::BarStyle(s) => self.bar_update(plugins::bar::Msg::Style(s)),
            WMsg::BarPos(p) => self.bar_update(plugins::bar::Msg::Pos(p)),
            WMsg::Workspaces(n) => {
                self.settings.workspaces = n;
                self.commit()
            }
            WMsg::Mode(m) => {
                self.settings.mode = Some(m);
                self.commit()
            }
            WMsg::DesktopFiles(v) => {
                // Les icônes du dossier sont ajoutées au chargement : on recharge.
                self.settings.desktop_files = Some(v);
                self.commit_reload()
            }
            WMsg::Profile(p) => self.performance_update(plugins::performance::Msg::Profile(p)),
            WMsg::Renderer(r) => self.performance_update(plugins::performance::Msg::Renderer(r)),
            WMsg::IdleManage(v) => self.lock_update(plugins::lock::Msg::IdleManage(v)),
            WMsg::Idle(s, m) => self.lock_update(plugins::lock::Msg::IdleDelay(s, m)),
            WMsg::IdleCommit => self.lock_update(plugins::lock::Msg::IdleCommit),
            WMsg::App(role, v) => self.apps_update(plugins::apps::Msg::Pick(role.to_string(), v)),
            WMsg::AppSystem(role) => {
                self.apps_update(plugins::apps::Msg::UseSystem(role.to_string()))
            }
        }
    }

    /// Une étape s'affiche : ce dont elle a besoin.
    fn wizard_entered(&mut self) -> Task<Msg> {
        match self.wizard.as_ref().map(|w| w.step) {
            Some(APPS) => self.wizard_icons(),
            _ => Task::none(),
        }
    }

    // -- Vue ----------------------------------------------------------------------

    fn wizard_pal(&self) -> Pal {
        let th = &self.cfg.general.theme;
        let ink = self.ink();
        let accent = color(th.accent, 1.0);
        let bg = color(th.bg, 1.0);
        Pal {
            fg: ink.fg,
            dim: ink.dim,
            accent,
            bg,
            on_accent: design::on_accent(accent, color(th.fg, 1.0), bg),
            warn: Color::from_rgb8(0xff, 0xb8, 0x6c),
        }
    }

    /// Taille de la surface de l'assistant.
    fn wizard_surface(&self) -> Size {
        let Some(w) = &self.wizard else {
            return Size::new(1440.0, 900.0);
        };
        self.sizes.get(&w.win).copied().unwrap_or_else(|| {
            self.monitors
                .iter()
                .find(|m| m.focused)
                .or_else(|| self.monitors.first())
                .map(|m| {
                    let s = if m.scale > 0.0 { m.scale } else { 1.0 };
                    Size::new(m.width as f32 / s, m.height as f32 / s)
                })
                .unwrap_or(Size::new(1440.0, 900.0))
        })
    }

    /// Carte : (largeur, hauteur, marge intérieure).
    fn wizard_card(&self) -> (f32, f32, f32) {
        let s = self.wizard_surface();
        let w = (s.width - 32.0).clamp(300.0, CARD_W);
        let h = (s.height - 32.0).clamp(360.0, CARD_H);
        (w, h, if w < 700.0 { 18.0 } else { 28.0 })
    }

    /// Largeur utile du contenu d'une étape.
    fn wizard_inner(&self) -> f32 {
        let (w, _, pad) = self.wizard_card();
        // La barre de défilement garde sa place à droite.
        w - 2.0 * pad - 14.0
    }

    /// Largeur d'une case d'une grille de `cols` colonnes au plus.
    fn wizard_tile(&self, cols: usize) -> f32 {
        let inner = self.wizard_inner();
        let cols = if inner < 560.0 {
            cols.min(2)
        } else if inner < 820.0 {
            cols.min(3)
        } else {
            cols
        }
        .max(1);
        ((inner - GAP * (cols as f32 - 1.0)) / cols as f32).floor()
    }

    fn wz_icon<'a>(&self, name: &str, size: f32, c: Color) -> Element<'a, Msg> {
        match super::icons::get(name) {
            Some(h) => svg(h)
                .width(size)
                .height(size)
                .style(move |_, _| svg::Style { color: Some(c) })
                .into(),
            None => Space::new().width(size).height(size).into(),
        }
    }

    /// Pastille ronde teintée d'accent, une icône au centre.
    fn wz_badge<'a>(&self, icon: &str, size: f32) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        container(self.wz_icon(icon, size * 0.5, p.accent))
            .center(size)
            .style(move |_| container::Style {
                background: Some(
                    Color {
                        a: 0.16,
                        ..p.accent
                    }
                    .into(),
                ),
                border: Border::default().rounded(size / 2.0),
                ..Default::default()
            })
            .into()
    }

    /// Carte à choisir : contenu, coche en haut à droite quand elle l'est.
    fn wz_choice<'a>(
        &self,
        content: Element<'a, Msg>,
        on: bool,
        msg: Msg,
        width: f32,
    ) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        let check: Element<'a, Msg> = if on {
            container(self.wz_icon("object-select", 12.0, p.on_accent))
                .center(22)
                .style(move |_| container::Style {
                    background: Some(p.accent.into()),
                    border: Border::default().rounded(11),
                    ..Default::default()
                })
                .into()
        } else {
            Space::new().into()
        };
        button(stack![
            container(content).width(Length::Fill),
            container(check).width(Length::Fill).align_x(Alignment::End)
        ])
        .padding(12)
        .width(width)
        .style(tile_style(on, p))
        .on_press(msg)
        .into()
    }

    fn wz_section<'a>(&self, title: &'a str) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        text(crate::i18n::tr(title))
            .font(self.title_font())
            .size(15)
            .color(p.fg)
            .into()
    }

    fn wz_note<'a>(&self, s: impl Into<String>) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        text(s.into())
            .font(self.font)
            .size(12)
            .color(p.dim)
            .width(Length::Fill)
            .into()
    }

    fn wz_wrap<'a>(&self, items: Vec<Element<'a, Msg>>) -> Element<'a, Msg> {
        row(items).spacing(GAP).wrap().into()
    }

    /// Surface plein écran : fond assombri, carte centrée.
    pub(super) fn view_wizard(&self) -> Element<'_, Msg> {
        let Some(w) = &self.wizard else {
            return Space::new().into();
        };
        let p = self.wizard_pal();
        let (cw, ch, pad) = self.wizard_card();
        let narrow = cw < 700.0;
        let info = &STEPS[w.step];

        // En-tête : étape, titre, pastilles des étapes.
        let dots = row(STEPS.iter().enumerate().map(|(i, s)| {
            let current = i == w.step;
            let seen = w.seen[i];
            let size = if narrow { 24.0 } else { 30.0 };
            let inner: Element<'_, Msg> = if seen && !current {
                self.wz_icon("object-select", size * 0.45, p.accent)
            } else {
                text((i + 1).to_string())
                    .font(self.title_font())
                    .size(if narrow { 11 } else { 13 })
                    .color(if current { p.on_accent } else { p.dim })
                    .into()
            };
            tooltip(
                button(container(inner).center(size))
                    .padding(0)
                    .on_press(wrap(WMsg::Go(i)))
                    .style(move |_, status| {
                        let hover = matches!(status, button::Status::Hovered);
                        button::Style {
                            background: Some(
                                if current {
                                    p.accent
                                } else if hover {
                                    Color { a: 0.14, ..p.fg }
                                } else {
                                    Color { a: 0.06, ..p.fg }
                                }
                                .into(),
                            ),
                            text_color: p.fg,
                            border: Border::default()
                                .rounded(size / 2.0)
                                .width(1)
                                .color(if seen {
                                    Color {
                                        a: 0.55,
                                        ..p.accent
                                    }
                                } else {
                                    Color::TRANSPARENT
                                }),
                            ..Default::default()
                        }
                    }),
                text(crate::i18n::tr(s.label)).font(self.font).size(12),
                tooltip::Position::Bottom,
            )
            .into()
        }))
        .spacing(if narrow { 4 } else { 8 })
        .align_y(Alignment::Center);
        let close = button(self.wz_icon("window-close", 16.0, p.fg))
            .padding(8)
            .style(button::text)
            .on_press(wrap(WMsg::Finish));
        let title = row![
            self.wz_badge(info.icon, 44.0),
            column![
                text(crate::tr!(
                    "ASSISTANT DE CONFIGURATION · {n} / {total}",
                    n = w.step + 1,
                    total = STEPS.len()
                ))
                .font(self.title_font())
                .size(11)
                .color(p.accent),
                text(crate::i18n::tr(info.label))
                    .font(self.title_font())
                    .size(if narrow { 22 } else { 26 })
                    .color(p.fg),
            ]
            .spacing(2)
            .width(Length::Fill),
            close,
        ]
        .spacing(14)
        .align_y(Alignment::Center);
        let header = column![
            title,
            text(crate::i18n::tr(info.summary))
                .font(self.font)
                .size(13)
                .color(p.dim),
            dots,
        ]
        .spacing(12);

        let body: Element<'_, Msg> = match w.step {
            WELCOME => self.wz_welcome(),
            SCREENS => self.wz_screens(),
            KEYBOARD => self.wz_keyboard(w),
            LOOK => self.wz_look(w),
            BAR => self.wz_bar(),
            DESKTOP => self.wz_desktop(w),
            PERFORMANCE => self.wz_performance(),
            IDLE => self.wz_idle(),
            APPS => self.wz_apps(),
            _ => self.wz_summary(w),
        };

        // Pied : retour, passer, suivant (terminer à la fin).
        let last = w.step == SUMMARY;
        let mut foot = row![
            button(text("Retour").font(self.font).size(14))
                .padding([9, 18])
                .style(button::secondary)
                .on_press_maybe((w.step > 0).then(|| wrap(WMsg::Back))),
            Space::new().width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if last {
            foot = foot.push(
                button(text("Annuler l'assistant").font(self.font).size(14))
                    .padding([9, 16])
                    .style(move |_, status| button::Style {
                        background: Some(
                            Color {
                                a: if matches!(status, button::Status::Hovered) {
                                    0.22
                                } else {
                                    0.10
                                },
                                ..p.warn
                            }
                            .into(),
                        ),
                        text_color: p.fg,
                        border: Border::default()
                            .rounded(8)
                            .width(1)
                            .color(Color { a: 0.6, ..p.warn }),
                        ..Default::default()
                    })
                    .on_press_maybe((!w.busy).then(|| wrap(WMsg::Cancel))),
            );
        } else {
            foot = foot.push(
                button(text("Passer").font(self.font).size(14))
                    .padding([9, 14])
                    .style(button::text)
                    .on_press(wrap(WMsg::Skip)),
            );
        }
        foot = foot.push(
            button(
                text(if last { "Terminer" } else { "Suivant" })
                    .font(self.title_font())
                    .size(14),
            )
            .padding([9, 22])
            .on_press(wrap(if last { WMsg::Finish } else { WMsg::Next })),
        );

        let card = container(
            column![
                header,
                scrollable(container(body).padding(iced::Padding {
                    right: 14.0,
                    ..iced::Padding::ZERO
                }))
                .height(Length::Fill),
                foot,
            ]
            .spacing(18),
        )
        .padding(pad)
        .width(cw)
        .height(ch)
        .style(move |theme| {
            let mut s = design::surface(theme, design::Surface::Floating);
            s.background = Some(Color { a: 0.97, ..p.bg }.into());
            s.shadow = Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                offset: Vector::new(0.0, 14.0),
                blur_radius: 40.0,
            };
            s
        });
        container(mouse_area(card).on_press(Msg::Noop))
            .center(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.02, 0.02, 0.05, 0.55).into()),
                ..Default::default()
            })
            .into()
    }

    // -- Étapes -------------------------------------------------------------------

    fn wz_welcome(&self) -> Element<'_, Msg> {
        let p = self.wizard_pal();
        let lang = match self.cfg.general.language.as_str() {
            "fr" => "fr",
            "en" => "en",
            _ => "auto",
        };
        let detected = match crate::i18n::from_env() {
            crate::i18n::Lang::Fr => "Français",
            crate::i18n::Lang::En => "English",
        };
        let tw = self.wizard_tile(3);
        let card = |code: &'static str, glyph: &'static str, name: String, sub: String| {
            let mark: Element<'_, Msg> =
                container(text(glyph).font(self.title_font()).size(20).color(p.accent))
                    .center(56)
                    .style(move |_| container::Style {
                        background: Some(
                            Color {
                                a: 0.14,
                                ..p.accent
                            }
                            .into(),
                        ),
                        border: Border::default().rounded(28),
                        ..Default::default()
                    })
                    .into();
            self.wz_choice(
                column![
                    mark,
                    text(name).font(self.title_font()).size(16).color(p.fg),
                    text(sub).font(self.font).size(12).color(p.dim),
                ]
                .spacing(8)
                .align_x(Alignment::Center)
                .width(Length::Fill)
                .into(),
                lang == code,
                wrap(WMsg::Language(code)),
                tw,
            )
        };
        column![
            row![
                self.wz_badge("user-desktop", 64.0),
                column![
                    text("Bienvenue dans hyprdesk")
                        .font(self.title_font())
                        .size(24)
                        .color(p.fg),
                    self.wz_note(crate::i18n::tr(
                        "Le bureau qui apparaît d'une touche sous vos fenêtres en tuiles. \
                         Neuf étapes, une minute : vous pouvez en passer, et revenir à \
                         chacune depuis les pastilles."
                    )),
                ]
                .spacing(6)
                .width(Length::Fill),
            ]
            .spacing(18)
            .align_y(Alignment::Center),
            self.wz_backup_note(),
            self.wz_section("Langue de l'interface"),
            self.wz_wrap(vec![
                card(
                    "fr",
                    "Fr",
                    "Français".into(),
                    crate::i18n::tr("Interface en français").into()
                ),
                card(
                    "en",
                    "En",
                    "English".into(),
                    crate::i18n::tr("Interface en anglais").into()
                ),
                card(
                    "auto",
                    "A",
                    crate::i18n::tr("Celle du système").into(),
                    crate::tr!("Détectée : {l}", l = detected)
                ),
            ]),
        ]
        .spacing(18)
        .into()
    }

    /// Sauvegarde et marche arrière (MLG-210) : ce qui a été gardé avant
    /// hyprdesk, et comment revenir en arrière.
    fn wz_backup_note<'a>(&self) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        let saved = match origine_date() {
            Some(d) => crate::tr!(
                "Votre configuration d'avant hyprdesk a été sauvegardée le {d} : rien n'est perdu.",
                d = d
            ),
            None => crate::i18n::tr(
                "Chaque modification de la configuration est d'abord sauvegardée (Réglages → Historique) : rien n'est perdu.",
            )
            .to_string(),
        };
        container(
            column![
                text(crate::i18n::tr("Sauvegarde et marche arrière"))
                    .font(self.font)
                    .size(14)
                    .color(p.fg),
                self.wz_note(saved),
                self.wz_note(crate::i18n::tr(
                    "Chaque choix s'applique aussitôt ; « Annuler l'assistant » remet la \
                     configuration d'avant son ouverture."
                )),
                self.wz_note(crate::i18n::tr(
                    "Pour tout remettre comme avant hyprdesk, à tout moment : \
                     ./scripts/uninstall.sh --restore (essai à blanc : --dry-run)."
                )),
            ]
            .spacing(4),
        )
        .padding([10, 14])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Color { a: 0.06, ..p.fg }.into()),
            border: Border::default().rounded(12),
            ..Default::default()
        })
        .into()
    }

    fn wz_screens(&self) -> Element<'_, Msg> {
        let p = self.wizard_pal();
        let on: Vec<&crate::screens::Screen> = self.screens.iter().filter(|s| s.enabled).collect();
        if on.is_empty() {
            return column![
                self.wz_badge("video-display", 64.0),
                self.wz_note(crate::i18n::tr("Aucun écran détecté pour l'instant.")),
                button(text("Détecter").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press(wrap(WMsg::DetectScreens)),
            ]
            .spacing(14)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .into();
        }
        let primary = self
            .cfg
            .general
            .primary
            .clone()
            .or_else(|| self.primary_monitor());
        let map_w = self.wizard_inner();
        let map_h = 300.0_f32.min(map_w * 0.42);
        let rect = |s: &crate::screens::Screen| {
            let (w, h) = s.logical();
            (s.x as f32, s.y as f32, w as f32, h as f32)
        };
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for s in &on {
            let (x, y, w, h) = rect(s);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + w), y1.max(y + h));
        }
        let k = ((map_w - 24.0) / (x1 - x0).max(1.0)).min((map_h - 24.0) / (y1 - y0).max(1.0));
        let (ox, oy) = ((map_w - (x1 - x0) * k) / 2.0, (map_h - (y1 - y0) * k) / 2.0);
        let mut layers: Vec<Element<'_, Msg>> =
            vec![Space::new().width(map_w).height(map_h).into()];
        for s in &on {
            let (x, y, w, h) = rect(s);
            let main = primary.as_deref() == Some(s.name.as_str());
            let (pw, ph) = if s.transform % 2 == 1 {
                (s.height, s.width)
            } else {
                (s.width, s.height)
            };
            let mut label = column![
                text(s.name.clone())
                    .font(self.title_font())
                    .size(15)
                    .color(p.fg),
                text(s.description.clone())
                    .font(self.font)
                    .size(12)
                    .color(p.dim),
                text(format!("{pw} × {ph}"))
                    .font(self.font)
                    .size(12)
                    .color(p.dim),
            ]
            .spacing(3)
            .align_x(Alignment::Center);
            if main {
                label = label.push(
                    container(
                        text("★ Principal")
                            .font(self.title_font())
                            .size(12)
                            .color(p.on_accent),
                    )
                    .padding([3, 10])
                    .style(move |_| container::Style {
                        background: Some(p.accent.into()),
                        border: Border::default().rounded(999),
                        ..Default::default()
                    }),
                );
            }
            let tile = button(container(label).center(Length::Fill))
                .width((w * k - 8.0).max(40.0))
                .height((h * k - 8.0).max(40.0))
                .padding(6)
                .on_press(wrap(WMsg::Primary(s.name.clone())))
                .style(move |_, st| {
                    let hover = matches!(st, button::Status::Hovered);
                    button::Style {
                        background: Some(
                            if main {
                                Color {
                                    a: 0.22,
                                    ..p.accent
                                }
                            } else if hover {
                                Color { a: 0.14, ..p.fg }
                            } else {
                                Color { a: 0.07, ..p.fg }
                            }
                            .into(),
                        ),
                        text_color: p.fg,
                        border: Border::default()
                            .rounded(10)
                            .width(if main { 2 } else { 1 })
                            .color(if main {
                                p.accent
                            } else {
                                Color { a: 0.3, ..p.fg }
                            }),
                        ..Default::default()
                    }
                });
            layers.push(
                pin(tile)
                    .x(ox + (x - x0) * k + 4.0)
                    .y(oy + (y - y0) * k + 4.0)
                    .into(),
            );
        }
        let map = container(iced::widget::Stack::with_children(layers))
            .width(map_w)
            .height(map_h)
            .style(move |_| container::Style {
                background: Some(Color { a: 0.04, ..p.fg }.into()),
                border: Border::default()
                    .rounded(14)
                    .width(1)
                    .color(Color { a: 0.08, ..p.fg }),
                ..Default::default()
            });
        let off: Vec<String> = self
            .screens
            .iter()
            .filter(|s| !s.enabled)
            .map(|s| s.name.clone())
            .collect();
        let mut col = column![map].spacing(12);
        if !off.is_empty() {
            col = col.push(self.wz_note(crate::tr!("Désactivé(s) : {l}.", l = off.join(", "))));
        }
        col.push(self.wz_note(crate::i18n::tr(
            "Placer les écrans, changer leur définition ou leur orientation : Réglages → Écrans.",
        )))
        .into()
    }

    fn wz_keyboard<'a>(&'a self, w: &'a Wizard) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        let Some(cur) = &w.kb.cur else {
            return self.wz_note(crate::i18n::tr("Lecture du clavier…"));
        };
        let first = cur
            .kb_layout
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let tw = self.wizard_tile(4);
        let known =
            |code: &str| w.kb.layouts.is_empty() || w.kb.layouts.iter().any(|(c, _)| c == code);
        let cards: Vec<Element<'_, Msg>> = LAYOUTS
            .iter()
            .filter(|(c, _, _)| known(c))
            .map(|(code, name, keys)| {
                let caps = row(keys.chars().map(|k| {
                    container(
                        text(k.to_string())
                            .font(self.title_font())
                            .size(12)
                            .color(p.fg),
                    )
                    .center(24)
                    .style(move |_| container::Style {
                        background: Some(Color { a: 0.08, ..p.fg }.into()),
                        border: Border::default()
                            .rounded(5)
                            .width(1)
                            .color(Color { a: 0.18, ..p.fg }),
                        shadow: Shadow {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
                            offset: Vector::new(0.0, 2.0),
                            blur_radius: 0.0,
                        },
                        ..Default::default()
                    })
                    .into()
                }))
                .spacing(4);
                self.wz_choice(
                    column![
                        caps,
                        text(crate::i18n::tr(name))
                            .font(self.title_font())
                            .size(14)
                            .color(p.fg),
                        text(code.to_string()).font(self.font).size(12).color(p.dim),
                    ]
                    .spacing(8)
                    .into(),
                    first == *code,
                    wrap(WMsg::Layout(code.to_string())),
                    tw,
                )
            })
            .collect();
        let all: Vec<Choice> =
            w.kb.layouts
                .iter()
                .map(|(c, d)| Choice(c.clone(), format!("{d} ({c})")))
                .collect();
        let selected = all.iter().find(|c| c.0 == first).cloned();
        column![
            self.wz_wrap(cards),
            row![
                text("Autre disposition")
                    .font(self.font)
                    .size(13)
                    .color(p.fg),
                pick_list(all, selected, |c| wrap(WMsg::LayoutPick(c)))
                    .placeholder(crate::i18n::tr("Choisir…"))
                    .font(self.font)
                    .width(300),
                text(crate::tr!("En place : {l}", l = cur.kb_layout))
                    .font(self.font)
                    .size(12)
                    .color(p.dim),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            text_input(
                crate::i18n::tr("Essayez la disposition ici : tapez quelques lettres…"),
                &w.kb.try_text
            )
            .on_input(|t| wrap(WMsg::Try(t)))
            .font(self.font)
            .size(18)
            .padding(14)
            .width(Length::Fill),
            self.wz_note(crate::i18n::tr(
                "Plusieurs dispositions, variantes et raccourci de bascule : Réglages → Clavier et souris."
            )),
        ]
        .spacing(16)
        .into()
    }

    /// Bureau en miniature : fond (vignette ou dégradé), voile de verre.
    fn wz_scene<'a>(
        &self,
        thumb: Option<image::Handle>,
        w: f32,
        h: f32,
        glass: bool,
    ) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        let base: Element<'a, Msg> = match thumb {
            Some(t) => image(t)
                .width(w)
                .height(h)
                .content_fit(iced::ContentFit::Cover)
                .into(),
            None => container(Space::new().width(w).height(h))
                .style(|_| design::wallpaper())
                .into(),
        };
        let veil: Element<'a, Msg> = if glass {
            container(Space::new().width(w).height(h))
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.55, ..p.bg }.into()),
                    ..Default::default()
                })
                .into()
        } else {
            Space::new().into()
        };
        let icon = move || {
            container(Space::new().width(14).height(14)).style(move |_| container::Style {
                background: Some(Color { a: 0.85, ..p.fg }.into()),
                border: Border::default().rounded(4),
                ..Default::default()
            })
        };
        let icons = column![icon(), icon(), icon()].spacing(8).padding(10);
        stack![base, veil, icons].into()
    }

    fn wz_look<'a>(&'a self, w: &'a Wizard) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        let th = &self.cfg.general.theme;
        let tw = self.wizard_tile(4);
        // Thèmes livrés : une fenêtre et une barre aux couleurs du thème.
        let themes: Vec<Element<'_, Msg>> = crate::theme::PRESETS
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let (f, b, a) = (color(t.fg, 1.0), color(t.bg, 1.0), color(t.accent, 1.0));
                let line = move |wd: f32, c: Color| {
                    container(Space::new().width(wd).height(5)).style(move |_| container::Style {
                        background: Some(c.into()),
                        border: Border::default().rounded(3),
                        ..Default::default()
                    })
                };
                let mock = container(
                    column![
                        row![
                            line(10.0, a),
                            Space::new().width(Length::Fill),
                            line(22.0, Color { a: 0.45, ..f })
                        ]
                        .align_y(Alignment::Center),
                        container(
                            column![
                                line(50.0, Color { a: 0.9, ..f }),
                                line(74.0, Color { a: 0.35, ..f }),
                                line(30.0, a),
                            ]
                            .spacing(6)
                        )
                        .padding(9)
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            background: Some(Color { a: 0.06, ..f }.into()),
                            border: Border::default()
                                .rounded(7)
                                .width(1)
                                .color(Color { a: 0.14, ..f }),
                            ..Default::default()
                        }),
                    ]
                    .spacing(6),
                )
                .padding(8)
                .width(Length::Fill)
                .height(74)
                .style(move |_| container::Style {
                    background: Some(b.into()),
                    border: Border::default().rounded(9),
                    ..Default::default()
                });
                let on =
                    th.name == t.name && th.fg == t.fg && th.bg == t.bg && th.accent == t.accent;
                self.wz_choice(
                    column![
                        mock,
                        text(t.name).font(self.title_font()).size(14).color(p.fg)
                    ]
                    .spacing(8)
                    .into(),
                    on,
                    wrap(WMsg::Theme(i)),
                    tw,
                )
            })
            .collect();
        // Fonds d'écran du dossier.
        let current = match self.settings.wallpaper(&Selector::default()) {
            Some(r) => match &r.source {
                WallSource::Path(p) => Some(p.clone()),
                _ => None,
            },
            None => None,
        };
        let walls: Element<'_, Msg> = if w.walls.is_empty() {
            self.wz_note(crate::i18n::tr(
                "Aucune image dans ~/.wallpapers : ajoutez-en, ou choisissez un autre dossier dans Réglages → Fond d'écran.",
            ))
        } else {
            let ww = self.wizard_tile(4);
            self.wz_wrap(
                w.walls
                    .iter()
                    .map(|path| {
                        let th = w.thumbs.get(path).cloned();
                        // Nom sur une ligne, raccourci à la largeur de la carte.
                        let full = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let max = ((ww - 26.0) / 6.5) as usize;
                        let name = if full.chars().count() > max {
                            let cut: String = full.chars().take(max.saturating_sub(1)).collect();
                            format!("{}…", cut.trim_end())
                        } else {
                            full
                        };
                        self.wz_choice(
                            column![
                                self.wz_scene(th, ww - 26.0, ((ww - 26.0) * 0.5625).floor(), false),
                                text(name)
                                    .font(self.font)
                                    .size(12)
                                    .color(p.dim)
                                    .wrapping(iced::widget::text::Wrapping::None),
                            ]
                            .spacing(6)
                            .into(),
                            current.as_ref() == Some(path),
                            wrap(WMsg::Wallpaper(path.clone())),
                            ww,
                        )
                    })
                    .collect(),
            )
        };
        // Bordures aux couleurs du thème : deux fenêtres en miniature.
        let borders = self.cfg.general.hypr.theme_borders;
        let win = move |active: bool| {
            let edge = if borders {
                if active {
                    p.accent
                } else {
                    Color { a: 0.9, ..p.bg }
                }
            } else {
                Color::from_rgb8(0x8a, 0x8f, 0x99)
            };
            container(Space::new().width(64).height(44)).style(move |_| container::Style {
                background: Some(Color { a: 0.08, ..p.fg }.into()),
                border: Border::default().rounded(6).width(2).color(edge),
                ..Default::default()
            })
        };
        let border_card = button(
            row![
                row![win(true), win(false)].spacing(8),
                column![
                    text("Bordures aux couleurs du thème")
                        .font(self.title_font())
                        .size(14)
                        .color(p.fg),
                    text(
                        "Accent du thème pour la fenêtre active, fond pour les autres ; suit les \
                         changements de thème."
                    )
                    .font(self.font)
                    .size(12)
                    .color(p.dim),
                ]
                .spacing(4)
                .width(Length::Fill),
                toggler(borders).on_toggle(|v| wrap(WMsg::Borders(v))),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        )
        .padding(12)
        .width(Length::Fill)
        .style(tile_style(borders, p))
        .on_press(wrap(WMsg::Borders(!borders)));
        column![
            self.wz_section("Thème"),
            self.wz_wrap(themes),
            self.wz_section("Fond d'écran"),
            walls,
            self.wz_section("Fenêtres"),
            border_card,
        ]
        .spacing(14)
        .into()
    }

    fn wz_bar(&self) -> Element<'_, Msg> {
        let p = self.wizard_pal();
        let bar = self.cfg.bar.as_ref();
        let style = bar.map(|b| b.style);
        let pos = bar.map(|b| b.position);
        let tw = self.wizard_tile(4);
        let styles: Vec<Element<'_, Msg>> = [
            (
                BarStyle::Continuite,
                "Continuité",
                "Groupes arrondis et sélection glissante.",
            ),
            (
                BarStyle::Graphite,
                "Graphite",
                "Barre des tâches en bas, lanceur centré.",
            ),
            (
                BarStyle::Island,
                "Island",
                "Capsule centrée, discrète au repos.",
            ),
            (
                BarStyle::Convexe,
                "Convex",
                "Lame sculptée de trois niches.",
            ),
        ]
        .into_iter()
        .map(|(s, name, sub)| {
            let iw = tw - 26.0;
            let thumb: Element<'_, Msg> = match bar_thumb(s) {
                Some(h) => image(h).width(iw).height(iw * 0.625).into(),
                None => Space::new().width(iw).height(iw * 0.625).into(),
            };
            self.wz_choice(
                column![
                    thumb,
                    text(crate::i18n::tr(name))
                        .font(self.title_font())
                        .size(14)
                        .color(p.fg),
                    text(crate::i18n::tr(sub))
                        .font(self.font)
                        .size(12)
                        .color(p.dim),
                ]
                .spacing(6)
                .into(),
                style == Some(s),
                wrap(WMsg::BarStyle(s)),
                tw,
            )
        })
        .collect();
        let screen = move |top: bool| {
            let strip = container(Space::new().width(Length::Fill).height(9)).style(move |_| {
                container::Style {
                    background: Some(p.accent.into()),
                    border: Border::default().rounded(3),
                    ..Default::default()
                }
            });
            let col = if top {
                column![strip, Space::new().height(Length::Fill)]
            } else {
                column![Space::new().height(Length::Fill), strip]
            };
            container(col.padding(5))
                .width(120)
                .height(72)
                .style(move |_| {
                    let mut s = design::wallpaper();
                    s.border = Border::default()
                        .rounded(8)
                        .width(1)
                        .color(Color { a: 0.2, ..p.fg });
                    s
                })
        };
        let pw = self.wizard_tile(4);
        let positions = self.wz_wrap(
            [
                (BarPosition::Top, "En haut"),
                (BarPosition::Bottom, "En bas"),
            ]
            .into_iter()
            .map(|(v, label)| {
                self.wz_choice(
                    row![
                        screen(v == BarPosition::Top),
                        text(crate::i18n::tr(label))
                            .font(self.title_font())
                            .size(14)
                            .color(p.fg),
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center)
                    .into(),
                    pos == Some(v),
                    wrap(WMsg::BarPos(v)),
                    pw,
                )
            })
            .collect(),
        );
        column![
            self.wz_section("Style"),
            self.wz_wrap(styles),
            self.wz_section("Position"),
            positions,
            self.wz_note(crate::i18n::tr(
                "Chaque style pose aussi sa position habituelle ; elle se change ici même. Les \
                 modules affichés se choisissent dans Réglages → Barre."
            )),
        ]
        .spacing(14)
        .into()
    }

    fn wz_desktop<'a>(&'a self, w: &'a Wizard) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        let g = &self.cfg.general;
        let count = g.workspaces;
        // Workspaces : dix cases, les N premières pleines.
        let mut tiles: Vec<Element<'_, Msg>> = (1..=10u8)
            .map(|n| {
                let full = count.is_some_and(|c| n <= c);
                button(
                    container(
                        text(n.to_string())
                            .font(self.title_font())
                            .size(14)
                            .color(if full { p.on_accent } else { p.dim }),
                    )
                    .center(40),
                )
                .padding(0)
                .on_press(wrap(WMsg::Workspaces(Some(n))))
                .style(move |_, st| button::Style {
                    background: Some(
                        if full {
                            p.accent
                        } else if matches!(st, button::Status::Hovered) {
                            Color { a: 0.14, ..p.fg }
                        } else {
                            Color { a: 0.06, ..p.fg }
                        }
                        .into(),
                    ),
                    text_color: p.fg,
                    border: Border::default().rounded(9),
                    ..Default::default()
                })
                .into()
            })
            .collect();
        tiles.push(
            button(
                text("À la demande")
                    .font(self.font)
                    .size(13)
                    .color(if count.is_none() { p.on_accent } else { p.fg }),
            )
            .padding([11, 14])
            .on_press(wrap(WMsg::Workspaces(None)))
            .style(move |_, st| button::Style {
                background: Some(
                    if count.is_none() {
                        p.accent
                    } else if matches!(st, button::Status::Hovered) {
                        Color { a: 0.14, ..p.fg }
                    } else {
                        Color { a: 0.06, ..p.fg }
                    }
                    .into(),
                ),
                text_color: p.fg,
                border: Border::default().rounded(9),
                ..Default::default()
            })
            .into(),
        );
        let ws_note = match count {
            Some(n) => crate::tr!(
                "{n} workspaces existent toujours, même vides : la barre les montre tous.",
                n = n
            ),
            None => crate::i18n::tr("Hyprland crée les workspaces à la demande.").to_string(),
        };
        // Mode du bureau : opaque ou verre, sur le fond en place.
        let wall = match self.settings.wallpaper(&Selector::default()) {
            Some(r) => match &r.source {
                WallSource::Path(path) => w.thumbs.get(path).cloned(),
                _ => None,
            },
            None => None,
        }
        .or_else(|| w.walls.first().and_then(|p| w.thumbs.get(p).cloned()));
        let mw = self.wizard_tile(3);
        let modes = self.wz_wrap(
            [
                (
                    Mode::Opaque,
                    "Opaque",
                    "Fond d'écran net ; les fenêtres sont mises de côté.",
                ),
                (
                    Mode::Glass,
                    "Verre",
                    "Fond flouté et teinté, comme un verre dépoli.",
                ),
            ]
            .into_iter()
            .map(|(m, name, sub)| {
                self.wz_choice(
                    column![
                        self.wz_scene(
                            wall.clone(),
                            mw - 26.0,
                            ((mw - 26.0) * 0.5).floor(),
                            m == Mode::Glass
                        ),
                        text(crate::i18n::tr(name))
                            .font(self.title_font())
                            .size(14)
                            .color(p.fg),
                        text(crate::i18n::tr(sub))
                            .font(self.font)
                            .size(12)
                            .color(p.dim),
                    ]
                    .spacing(6)
                    .into(),
                    g.mode == m,
                    wrap(WMsg::Mode(m)),
                    mw,
                )
            })
            .collect(),
        );
        let files = g.desktop_files;
        let files_card = button(
            row![
                row![
                    self.wz_icon("folder", 28.0, p.accent),
                    self.wz_icon("image-x-generic", 28.0, p.accent),
                    self.wz_icon("application-x-executable", 28.0, p.accent),
                ]
                .spacing(8),
                column![
                    text("Fichiers du dossier Bureau et Corbeille")
                        .font(self.title_font())
                        .size(14)
                        .color(p.fg),
                    text("Le contenu de ~/Bureau apparaît en icônes, avec la Corbeille.")
                        .font(self.font)
                        .size(12)
                        .color(p.dim),
                ]
                .spacing(4)
                .width(Length::Fill),
                toggler(files).on_toggle(|v| wrap(WMsg::DesktopFiles(v))),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        )
        .padding(12)
        .width(Length::Fill)
        .style(tile_style(files, p))
        .on_press(wrap(WMsg::DesktopFiles(!files)));
        column![
            self.wz_section("Workspaces permanents"),
            row(tiles).spacing(8).wrap(),
            self.wz_note(ws_note),
            self.wz_section("Bureau affiché"),
            modes,
            self.wz_section("Icônes"),
            files_card,
        ]
        .spacing(14)
        .into()
    }

    fn wz_performance(&self) -> Element<'_, Msg> {
        let p = self.wizard_pal();
        let active = crate::perf::active(&self.cfg.general.fx.options);
        let tw = self.wizard_tile(3);
        // Jauge de 1 à 3 : effets, ou charge de la carte graphique.
        let gauge = move |n: usize| {
            row((0..3).map(|i| {
                container(Space::new().width(18).height(6))
                    .style(move |_| container::Style {
                        background: Some(
                            if i < n {
                                p.accent
                            } else {
                                Color { a: 0.15, ..p.fg }
                            }
                            .into(),
                        ),
                        border: Border::default().rounded(3),
                        ..Default::default()
                    })
                    .into()
            }))
            .spacing(4)
        };
        let profiles: Vec<Element<'_, Msg>> = Profile::ALL
            .into_iter()
            .map(|pr| {
                let (icon, level) = match pr {
                    Profile::Performance => ("power-profile-performance", 1),
                    Profile::Balanced => ("power-profile-balanced", 2),
                    Profile::Quality => ("preferences-desktop-animation", 3),
                };
                self.wz_choice(
                    column![
                        self.wz_badge(icon, 44.0),
                        text(crate::i18n::tr(pr.label()))
                            .font(self.title_font())
                            .size(15)
                            .color(p.fg),
                        row![
                            text("Effets").font(self.font).size(12).color(p.dim),
                            gauge(level)
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        text(crate::i18n::tr(pr.summary()))
                            .font(self.font)
                            .size(12)
                            .color(p.dim),
                    ]
                    .spacing(8)
                    .into(),
                    active == Some(pr),
                    wrap(WMsg::Profile(pr)),
                    tw,
                )
            })
            .collect();
        let renderer = self.cfg.general.renderer;
        let rw = self.wizard_tile(2);
        let engines = self.wz_wrap(
            [
                (
                    Renderer::Hardware,
                    "video-display",
                    "La carte graphique dessine la barre, le dock et le bureau. Conseillé.",
                ),
                (
                    Renderer::Software,
                    "application-x-executable",
                    "Le processeur dessine tout : pour un pilote graphique défaillant.",
                ),
            ]
            .into_iter()
            .map(|(r, icon, sub)| {
                self.wz_choice(
                    row![
                        self.wz_badge(icon, 40.0),
                        column![
                            text(crate::i18n::tr(r.label()))
                                .font(self.title_font())
                                .size(14)
                                .color(p.fg),
                            text(crate::i18n::tr(sub))
                                .font(self.font)
                                .size(12)
                                .color(p.dim),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .into(),
                    renderer == r,
                    wrap(WMsg::Renderer(r)),
                    rw,
                )
            })
            .collect(),
        );
        let status = match active {
            Some(_) => String::new(),
            None if crate::perf::untouched(&self.cfg.general.fx.options) => {
                crate::i18n::tr("Aucun profil : tout vient de hyprland.lua.").to_string()
            }
            None => crate::i18n::tr("Réglages retouchés à la main (aucun profil).").to_string(),
        };
        column![
            self.wz_section("Effets de Hyprland"),
            self.wz_wrap(profiles),
            self.wz_note(status),
            self.wz_section("Moteur de rendu de hyprdesk"),
            engines,
            self.wz_note(crate::i18n::tr(
                "Le moteur de rendu prend effet à la relance de hyprdesk."
            )),
        ]
        .spacing(14)
        .into()
    }

    fn wz_idle(&self) -> Element<'_, Msg> {
        let p = self.wizard_pal();
        let i = &self.cfg.general.idle;
        let manage_card = button(
            row![
                self.wz_badge("system-lock-screen", 40.0),
                column![
                    text("hyprdesk gère la veille")
                        .font(self.title_font())
                        .size(14)
                        .color(p.fg),
                    text(
                        "hyprdesk écrit la configuration de hypridle et le lance ; une \
                         application qui demande à rester éveillée (vidéo, appel) l'en empêche."
                    )
                    .font(self.font)
                    .size(12)
                    .color(p.dim),
                ]
                .spacing(4)
                .width(Length::Fill),
                toggler(i.manage).on_toggle(|v| wrap(WMsg::IdleManage(v))),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
        )
        .padding(12)
        .width(Length::Fill)
        .style(tile_style(i.manage, p))
        .on_press(wrap(WMsg::IdleManage(!i.manage)));
        let steps = [
            (IdleStep::Dim, "Assombrir", "display-brightness", i.dim),
            (IdleStep::Lock, "Verrouiller", "system-lock-screen", i.lock),
            (IdleStep::Dpms, "Éteindre", "system-shutdown", i.dpms),
            (
                IdleStep::Suspend,
                "Mettre en veille",
                "weather-clear-night",
                i.suspend,
            ),
        ];
        let minutes = |s: u32| s.div_ceil(60);
        let alpha = if i.manage { 1.0 } else { 0.45 };
        let fg = Color {
            a: p.fg.a * alpha,
            ..p.fg
        };
        let accent = Color {
            a: alpha,
            ..p.accent
        };
        // Frise : chaque étape à sa minute, sur une échelle commune.
        let tw = self.wizard_inner();
        let max = steps
            .iter()
            .map(|s| minutes(s.3))
            .max()
            .unwrap_or(0)
            .max(10) as f32;
        let track_y = 52.0;
        let dot = 30.0;
        let mut layers: Vec<Element<'_, Msg>> = vec![
            Space::new().width(tw).height(112).into(),
            pin(
                container(Space::new().width(tw - dot).height(4)).style(move |_| {
                    container::Style {
                        background: Some(Color { a: 0.15, ..fg }.into()),
                        border: Border::default().rounded(2),
                        ..Default::default()
                    }
                }),
            )
            .x(dot / 2.0)
            .y(track_y - 2.0)
            .into(),
        ];
        // Libellés en quinconce, dans l'ordre de la frise : deux étapes
        // voisines ne se chevauchent pas.
        let mut placed: Vec<_> = steps.iter().filter(|s| s.3 > 0).collect();
        placed.sort_by_key(|s| s.3);
        let mut row_end = [0.0_f32; 2];
        for (k, (_, label, icon, secs)) in placed.into_iter().enumerate() {
            let m = minutes(*secs);
            let x = (m as f32 / max) * (tw - dot);
            layers.push(
                pin(container(self.wz_icon(icon, 15.0, p.on_accent))
                    .center(dot)
                    .style(move |_| container::Style {
                        background: Some(accent.into()),
                        border: Border::default().rounded(dot / 2.0),
                        ..Default::default()
                    }))
                .x(x)
                .y(track_y - dot / 2.0)
                .into(),
            );
            let above = k % 2 == 0;
            // Une rangée : le libellé suivant se pousse après le précédent.
            let edge = &mut row_end[usize::from(!above)];
            let lx = (x + dot / 2.0 - 80.0).max(*edge).clamp(0.0, tw - 160.0);
            *edge = lx + 160.0 + 6.0;
            layers.push(
                pin(container(
                    text(crate::tr!(
                        "{s} · {n} min",
                        s = crate::i18n::tr(label),
                        n = m
                    ))
                    .font(self.font)
                    .size(12)
                    .color(fg)
                    .align_x(Alignment::Center)
                    .width(160),
                ))
                .x(lx)
                .y(if above {
                    6.0
                } else {
                    track_y + dot / 2.0 + 8.0
                })
                .into(),
            );
        }
        let timeline = iced::widget::Stack::with_children(layers)
            .width(tw)
            .height(112);
        let never: Vec<String> = steps
            .iter()
            .filter(|s| s.3 == 0)
            .map(|s| crate::i18n::tr(s.1).to_string())
            .collect();
        // Un curseur par étape.
        let cw = self.wizard_tile(4);
        let cards: Vec<Element<'_, Msg>> = steps
            .iter()
            .enumerate()
            .map(|(k, (step, label, icon, secs))| {
                let m = minutes(*secs);
                let step = *step;
                let value = if m == 0 {
                    crate::i18n::tr("jamais").to_string()
                } else {
                    crate::tr!("{n} min", n = m)
                };
                container(
                    column![
                        row![
                            text((k + 1).to_string())
                                .font(self.title_font())
                                .size(12)
                                .color(p.accent),
                            self.wz_icon(icon, 16.0, fg),
                            text(crate::i18n::tr(label))
                                .font(self.title_font())
                                .size(13)
                                .color(fg),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                        text(value).font(self.title_font()).size(22).color(fg),
                        slider(0..=120u32, m, move |v| wrap(WMsg::Idle(step, v)))
                            .on_release(wrap(WMsg::IdleCommit))
                            .style(design::bar_slider),
                    ]
                    .spacing(8),
                )
                .padding(12)
                .width(cw)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.035, ..p.fg }.into()),
                    border: Border::default()
                        .rounded(14)
                        .width(1)
                        .color(Color { a: 0.12, ..p.fg }),
                    ..Default::default()
                })
                .into()
            })
            .collect();
        let mut col = column![manage_card].spacing(14);
        if i.manage && !crate::launch::executable("hypridle") && !self.preview {
            col = col.push(
                text("hypridle est absent (paquet hypridle) : rien ne se passera à l'inactivité.")
                    .font(self.font)
                    .size(12)
                    .color(p.warn),
            );
        }
        col = col.push(timeline);
        if !never.is_empty() {
            col = col.push(self.wz_note(crate::tr!("Jamais : {l}.", l = never.join(", "))));
        }
        if let Some(warning) = crate::idle::exposure(i) {
            col = col.push(
                container(
                    row![
                        self.wz_icon("hyprdesk-urgent", 20.0, p.warn),
                        column![
                            text(warning).font(self.font).size(13).color(p.fg),
                            text(
                                "Vos délais sont gardés. Pour l'éviter, verrouillez avant \
                                 d'éteindre l'écran."
                            )
                            .font(self.font)
                            .size(12)
                            .color(p.dim),
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding(12)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.12, ..p.warn }.into()),
                    border: Border::default()
                        .rounded(12)
                        .width(1)
                        .color(Color { a: 0.6, ..p.warn }),
                    ..Default::default()
                }),
            );
        }
        col.push(self.wz_wrap(cards)).into()
    }

    /// Icône d'une application (résolue), sinon son initiale.
    fn wz_app_icon<'a>(&'a self, id: &str, name: &str, size: f32) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        if let Some(img) = self.class_imgs.get(id).copied()
            && let Some(e) = self.img_element(img, size, 1.0)
        {
            return e;
        }
        container(
            text(
                name.chars()
                    .next()
                    .unwrap_or('?')
                    .to_uppercase()
                    .to_string(),
            )
            .font(self.title_font())
            .size(size * 0.45)
            .color(p.accent),
        )
        .center(size)
        .style(move |_| container::Style {
            background: Some(
                Color {
                    a: 0.16,
                    ..p.accent
                }
                .into(),
            ),
            border: Border::default().rounded(size * 0.25),
            ..Default::default()
        })
        .into()
    }

    fn wz_apps(&self) -> Element<'_, Msg> {
        let p = self.wizard_pal();
        let mut col = column![].spacing(18);
        for role_id in ROLES {
            let Some(role) = crate::apps::role(role_id) else {
                continue;
            };
            let current = self.cfg.general.apps.get(role_id).cloned();
            let mut cands = crate::apps::candidates(role, &self.apps);
            cands.truncate(MAX_APPS);
            let tile = 104.0;
            let mut tiles: Vec<Element<'_, Msg>> = vec![
                self.wz_choice(
                    column![
                        container(self.wz_icon("preferences-system", 26.0, p.accent))
                            .center(44)
                            .style(move |_| container::Style {
                                background: Some(
                                    Color {
                                        a: 0.12,
                                        ..p.accent
                                    }
                                    .into()
                                ),
                                border: Border::default().rounded(11),
                                ..Default::default()
                            }),
                        text("Celle du système")
                            .font(self.font)
                            .size(12)
                            .color(p.fg)
                            .align_x(Alignment::Center),
                    ]
                    .spacing(6)
                    .align_x(Alignment::Center)
                    .width(Length::Fill)
                    .into(),
                    current.is_none(),
                    wrap(WMsg::AppSystem(role_id)),
                    tile,
                ),
            ];
            for (id, name) in cands {
                let on = current.as_deref() == Some(id.trim_end_matches(".desktop"));
                tiles.push(
                    self.wz_choice(
                        column![
                            self.wz_app_icon(&id, &name, 44.0),
                            text(name)
                                .font(self.font)
                                .size(12)
                                .color(p.fg)
                                .align_x(Alignment::Center),
                        ]
                        .spacing(6)
                        .align_x(Alignment::Center)
                        .width(Length::Fill)
                        .into(),
                        on,
                        wrap(WMsg::App(role_id, id.clone())),
                        tile,
                    ),
                );
            }
            col = col.push(
                column![self.wz_section(role.label), row(tiles).spacing(10).wrap(),].spacing(10),
            );
        }
        col.push(self.wz_note(crate::i18n::tr(
            "Un choix vaut aussi pour le système (xdg-open). Images, vidéo, PDF, mail, assistant IA : Réglages → Applications.",
        )))
        .into()
    }

    /// Ce qui est choisi à une étape, selon une configuration donnée.
    pub(super) fn wizard_value(&self, step: usize, cfg: &Config) -> String {
        let g = &cfg.general;
        let tr = crate::i18n::tr;
        match step {
            WELCOME => match g.language.as_str() {
                "fr" => "Français".into(),
                "en" => "English".into(),
                _ => tr("Celle du système").into(),
            },
            SCREENS => match &g.primary {
                Some(p) => crate::tr!("Principal : {p}", p = p),
                None => tr("Écran principal automatique").into(),
            },
            KEYBOARD => self
                .wizard
                .as_ref()
                .and_then(|w| {
                    let k = if std::ptr::eq(cfg, &*w.before) {
                        w.kb.before.as_ref()
                    } else {
                        w.kb.cur.as_ref()
                    };
                    k.map(|i| i.kb_layout.clone())
                })
                .unwrap_or_else(|| "—".into()),
            LOOK => {
                let wall = cfg
                    .wallpapers
                    .iter()
                    .find(|w| w.sel == Selector::default())
                    .and_then(|w| match &w.source {
                        WallSource::Path(p) => {
                            p.file_stem().map(|s| s.to_string_lossy().to_string())
                        }
                        _ => None,
                    });
                let mut s = g.theme.name.clone();
                if let Some(w) = wall {
                    s = format!("{s} · {w}");
                }
                if g.hypr.theme_borders {
                    s = format!("{s} · {}", tr("bordures du thème"));
                }
                s
            }
            BAR => match &cfg.bar {
                Some(b) => {
                    let name = crate::config::BAR_STYLES
                        .iter()
                        .find(|(s, _, _)| *s == b.style)
                        .map_or("", |(_, _, l)| *l);
                    format!(
                        "{} · {}",
                        tr(name),
                        tr(if b.position == BarPosition::Top {
                            "En haut"
                        } else {
                            "En bas"
                        })
                    )
                }
                None => tr("Sans barre").into(),
            },
            DESKTOP => {
                let ws = match g.workspaces {
                    Some(n) => crate::tr!("{n} workspaces", n = n),
                    None => tr("Workspaces à la demande").into(),
                };
                let mode = tr(if g.mode == Mode::Glass {
                    "Verre"
                } else {
                    "Opaque"
                });
                let files = if g.desktop_files {
                    format!(" · {}", tr("dossier Bureau"))
                } else {
                    String::new()
                };
                format!("{ws} · {mode}{files}")
            }
            PERFORMANCE => {
                let profile = match crate::perf::active(&g.fx.options) {
                    Some(p) => tr(p.label()).to_string(),
                    None => tr("Sans profil").into(),
                };
                format!("{profile} · {}", tr(g.renderer.label()))
            }
            IDLE => {
                if !g.idle.manage {
                    return tr("Veille non gérée").into();
                }
                let i = &g.idle;
                [
                    ("Assombrir", i.dim),
                    ("Verrouiller", i.lock),
                    ("Éteindre", i.dpms),
                    ("Mettre en veille", i.suspend),
                ]
                .iter()
                .filter(|(_, s)| *s > 0)
                .map(|(l, s)| crate::tr!("{s} {n} min", s = tr(l), n = s.div_ceil(60)))
                .collect::<Vec<_>>()
                .join(" → ")
            }
            APPS => ROLES
                .iter()
                .map(|r| {
                    let v = g.apps.get(*r).map(|id| {
                        self.apps
                            .iter()
                            .find(|a| a.id.trim_end_matches(".desktop") == id)
                            .map_or(id.clone(), |a| a.name.clone())
                    });
                    v.unwrap_or_else(|| tr("système").into())
                })
                .collect::<Vec<_>>()
                .join(" · "),
            _ => String::new(),
        }
    }

    fn wz_summary<'a>(&'a self, w: &'a Wizard) -> Element<'a, Msg> {
        let p = self.wizard_pal();
        let tw = self.wizard_tile(3);
        let tiles: Vec<Element<'_, Msg>> = (0..SUMMARY)
            .map(|i| {
                let now = self.wizard_value(i, &self.cfg);
                let changed = now != self.wizard_value(i, &w.before);
                let mut head = row![
                    self.wz_icon(STEPS[i].icon, 18.0, p.accent),
                    text(crate::i18n::tr(STEPS[i].label))
                        .font(self.title_font())
                        .size(14)
                        .color(p.fg)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .align_y(Alignment::Center);
                if changed {
                    head = head.push(
                        container(text("modifié").font(self.font).size(11).color(p.on_accent))
                            .padding([2, 8])
                            .style(move |_| container::Style {
                                background: Some(p.accent.into()),
                                border: Border::default().rounded(999),
                                ..Default::default()
                            }),
                    );
                }
                button(column![head, text(now).font(self.font).size(12).color(p.dim)].spacing(6))
                    .padding(12)
                    .width(tw)
                    .height(86)
                    .style(tile_style(false, p))
                    .on_press(wrap(WMsg::Go(i)))
                    .into()
            })
            .collect();
        let mut col = column![self.wz_wrap(tiles)].spacing(14);
        if let Some(e) = &w.error {
            col = col.push(
                text(crate::tr!("Annulation impossible : {e}", e = e))
                    .font(self.font)
                    .size(12)
                    .color(p.warn),
            );
        }
        col.push(
            row![
                self.wz_note(crate::i18n::tr(
                    "« Annuler l'assistant » rétablit la configuration d'avant son ouverture \
                     (historique : « Assistant de configuration ») ; tout remettre comme avant \
                     hyprdesk : ./scripts/uninstall.sh --restore."
                )),
                button(
                    text("Terminer et visiter les Réglages")
                        .font(self.font)
                        .size(13)
                )
                .padding([7, 14])
                .style(button::secondary)
                .on_press(wrap(WMsg::FinishTour)),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
        )
        .into()
    }
}

/// Date (AAAA-MM-JJ) de l'état d'origine gardé par install.sh (MLG-210),
/// lue une fois.
fn origine_date() -> Option<String> {
    static DATE: OnceLock<Option<String>> = OnceLock::new();
    DATE.get_or_init(|| {
        let base = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
        let m = std::fs::read_to_string(base.join("hyprdesk/origine/manifest")).ok()?;
        m.lines()
            .find_map(|l| l.strip_prefix("date "))
            .map(|d| d.chars().take(10).collect())
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        app.surfaces
            .insert(iced::window::Id::unique(), "DP-1".into());
        let _ = app.open_wizard();
        app
    }

    fn step(app: &App) -> usize {
        app.wizard.as_ref().unwrap().step
    }

    /// Suivant, Passer, Retour et les pastilles restent dans les bornes ;
    /// Suivant sur le récapitulatif ferme l'assistant.
    #[test]
    fn navigation() {
        let mut app = app();
        assert_eq!(step(&app), 0);
        let _ = app.wizard_update(WMsg::Back);
        assert_eq!(step(&app), 0);
        let _ = app.wizard_update(WMsg::Next);
        let _ = app.wizard_update(WMsg::Skip);
        assert_eq!(step(&app), 2);
        assert!(app.wizard.as_ref().unwrap().seen[2]);
        assert!(!app.wizard.as_ref().unwrap().seen[3]);
        let _ = app.wizard_update(WMsg::Back);
        assert_eq!(step(&app), 1);
        let _ = app.wizard_update(WMsg::Go(42));
        assert_eq!(step(&app), SUMMARY);
        let _ = app.wizard_update(WMsg::Next);
        assert!(app.wizard.is_none());
    }

    /// Un choix passe par les réglages et s'applique aussitôt.
    #[test]
    fn choix_appliques() {
        let mut app = app();
        let _ = app.wizard_update(WMsg::Language("en"));
        assert_eq!(app.settings.language.as_deref(), Some("en"));
        assert_eq!(app.cfg.general.language, "en");
        let _ = app.wizard_update(WMsg::Workspaces(Some(4)));
        assert_eq!(app.cfg.general.workspaces, Some(4));
        let _ = app.wizard_update(WMsg::Mode(Mode::Glass));
        assert_eq!(app.cfg.general.mode, Mode::Glass);
        let _ = app.wizard_update(WMsg::Theme(1));
        assert_eq!(app.cfg.general.theme.name, crate::theme::PRESETS[1].name);
        let _ = app.wizard_update(WMsg::BarStyle(BarStyle::Graphite));
        let bar = app.settings.bar.as_ref().unwrap();
        assert_eq!(bar.style, BarStyle::Graphite);
        assert_eq!(bar.position, BarPosition::Bottom);
        let _ = app.wizard_update(WMsg::Idle(IdleStep::Lock, 9));
        assert_eq!(app.settings.idle_lock, Some(540));
        // Récapitulatif : l'étape Bureau est marquée modifiée.
        let w = app.wizard.as_ref().unwrap();
        assert_ne!(
            app.wizard_value(DESKTOP, &app.cfg),
            app.wizard_value(DESKTOP, &w.before)
        );
        assert_eq!(
            app.wizard_value(SCREENS, &app.cfg),
            app.wizard_value(SCREENS, &w.before)
        );
    }

    /// Annuler sans instantané : les réglages d'avant reviennent.
    #[test]
    fn annuler_sans_historique() {
        let mut app = app();
        let before = app.settings.clone();
        let _ = app.wizard_update(WMsg::Workspaces(Some(3)));
        assert_ne!(app.settings, before);
        let _ = app.wizard_update(WMsg::Cancel);
        assert_eq!(app.settings, before);
        assert!(app.wizard.is_none());
    }

    /// Avertissement de l'étape Veille : verrouillage après l'extinction.
    #[test]
    fn verrouillage_apres_extinction() {
        let mut app = app();
        let _ = app.wizard_update(WMsg::IdleManage(true));
        let _ = app.wizard_update(WMsg::Idle(IdleStep::Dpms, 5));
        let _ = app.wizard_update(WMsg::Idle(IdleStep::Lock, 4));
        assert!(crate::idle::exposure(&app.cfg.general.idle).is_none());
        let _ = app.wizard_update(WMsg::Idle(IdleStep::Lock, 8));
        assert!(crate::idle::exposure(&app.cfg.general.idle).is_some());
        // Les délais restent ceux choisis.
        assert_eq!(app.cfg.general.idle.lock, 480);
        assert_eq!(app.cfg.general.idle.dpms, 300);
    }

    /// Textes tenus dans des tables (hors des appels que repère le catalogue).
    #[test]
    fn textes_traduits() {
        let mut all: Vec<&str> = LAYOUTS.iter().map(|l| l.1).collect();
        all.extend([
            "Groupes arrondis et sélection glissante.",
            "Barre des tâches en bas, lanceur centré.",
            "Capsule centrée, discrète au repos.",
            "Lame sculptée de trois niches.",
            "Fond d'écran net ; les fenêtres sont mises de côté.",
            "Fond flouté et teinté, comme un verre dépoli.",
            "La carte graphique dessine la barre, le dock et le bureau. Conseillé.",
            "Le processeur dessine tout : pour un pilote graphique défaillant.",
            "Assombrir",
            "Verrouiller",
            "Éteindre",
            "Mettre en veille",
            "Moteur de rendu de hyprdesk",
        ]);
        let missing = crate::i18n::untranslated("assistant", all);
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }

    #[test]
    fn disposition_en_tete() {
        assert_eq!(
            with_first_layout("fr,us", "", "us"),
            ("us,fr".into(), "".into())
        );
        assert_eq!(
            with_first_layout("fr,us", "azerty,intl", "us"),
            ("us,fr".into(), "intl,azerty".into())
        );
        assert_eq!(
            with_first_layout("us", "", "de"),
            ("de,us".into(), "".into())
        );
        assert_eq!(with_first_layout("", "", "fr"), ("fr".into(), "".into()));
    }
}
