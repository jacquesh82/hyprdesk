//! Paquet intégré « Lanceur » : le dock en bas du bureau — widget autonome,
//! seul contrôle du mode `launcher-only` — et sa page de réglages.
//!
//! Le lanceur affiche les fenêtres ouvertes (aucune, celles du workspace
//! affiché, ou toutes), regroupées au choix : une entrée par fenêtre, une par
//! application (classe de fenêtre, donc par binaire), ou par workspace. Le
//! bouton ⚙ termine la rangée. L'utiliser ne change jamais le mode du bureau.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

mod contrast;
mod keyboard;
mod lifecycle;
mod motion;

pub(in crate::ui) use contrast::Backdrop;
pub(in crate::ui) use lifecycle::{Placement, Runtime, screen_anchor};

use crate::i18n::text;
use iced::widget::{Space, button, column, container, mouse_area, radio, row};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Size, Task,
};

use super::super::panel::PanelMsg;
use super::super::{App, IconImg, Msg as UiMsg, color, design};
use super::{PluginInfo, PluginMsg};
use crate::config::{LauncherGroup, LauncherPosition, RunningApps};
use crate::hypr::{Client, Monitor};

pub const INFO: PluginInfo = PluginInfo {
    id: "launcher",
    label: "Lanceur",
    icon: "view-app-grid",
    summary: "Dock du bureau et lanceur d'applications (grande feuille ou carte centrée, suggestions).",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("contenu", "Contenu"),
    ("apparence", "Apparence"),
    ("epinglees", "Épinglées"),
    ("applications", "Applications"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Launcher(m)))
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Fenêtres affichées : aucune, celles du workspace, toutes.
    Running(RunningApps),
    Group(LauncherGroup),
    Position(LauncherPosition),
    /// Taille des icônes (pendant le glissé), puis enregistrement.
    Size(u16),
    SizeCommit,
    PinFilter(String),
    Pin(String),
    Unpin(usize),
    /// Déplace une application épinglée (-1 avant, +1 après).
    MovePin(usize, i32),
    ShowSettings(bool),
    ShowAssistant(bool),
    /// Lanceur d'applications : présentation, suggestions, catégories.
    AppsStyle(crate::config::AppsStyle),
    AppsSuggestions(u8),
    AppsSuggestionsCommit,
    AppsCategories(bool),
    /// keys.search = SUPER + R (à la place d'un autre lanceur).
    UseSuperR,
    /// Le même lanceur dans tous les modes (apps_unified).
    AppsUnified(bool),
    /// Plein bureau : raccourci propre (keys.search_desktop), saisi puis validé.
    DeskKey(String),
    DeskKeySubmit,
}

/// État d'écran : recherche d'une application à épingler.
#[derive(Default)]
pub struct State {
    pub pin_filter: String,
    /// Raccourci du plein bureau en cours de saisie, et son erreur.
    pub desk_key: Option<String>,
    pub desk_error: Option<String>,
}

/// Une entrée de la rangée du lanceur.
enum Entry<'a> {
    Window(&'a Client),
    /// Toutes les fenêtres d'une application (classe).
    App(String, Vec<&'a Client>),
    /// Titre d'un groupe : le nom du workspace.
    Label(String),
}

/// Image d'un bouton du dock.
#[derive(Debug, Clone, PartialEq)]
enum Glyph {
    /// Icône d'une application (id `.desktop` ou classe de fenêtre).
    App(String),
    /// Symbole du thème : nom, échelle, texte de repli.
    Symbol(&'static str, f32, &'static str),
    /// Engrenage des réglages.
    Settings,
}

/// Bouton du dock : ce que la souris et le clavier activent.
#[derive(Debug, Clone)]
pub(in crate::ui) struct DockItem {
    pub key: String,
    glyph: Glyph,
    badge: usize,
    active: bool,
    pub popup: PopupContent,
    pub msg: UiMsg,
}

/// Case du dock, dans l'ordre affiché. Le rang d'une case sert d'index à la
/// vague du survol comme à la sélection au clavier.
#[derive(Debug, Clone)]
pub(in crate::ui) enum Slot {
    Item(Box<DockItem>),
    /// Nom d'un workspace (regroupement par workspace).
    Label(String),
    Separator,
}

/// Ordre des workspaces : numériques d'abord, dans l'ordre, puis par nom.
fn ws_key(name: &str) -> (i64, String) {
    (name.parse::<i64>().unwrap_or(i64::MAX), name.to_string())
}

/// Fenêtres que le lanceur montre sur un écran.
fn windows<'a>(clients: &'a [Client], running: RunningApps, workspace: &str) -> Vec<&'a Client> {
    match running {
        RunningApps::Off => Vec::new(),
        RunningApps::Workspace => clients
            .iter()
            .filter(|c| c.workspace == workspace)
            .collect(),
        // Fenêtres mises de côté comprises : le lanceur permet de les rappeler.
        RunningApps::All => clients.iter().collect(),
    }
}

/// Entrées du lanceur pour un regroupement donné (fonction pure, testée).
fn entries<'a>(wins: Vec<&'a Client>, group: LauncherGroup) -> Vec<Entry<'a>> {
    match group {
        LauncherGroup::None => wins.into_iter().map(Entry::Window).collect(),
        LauncherGroup::App => {
            let mut groups: Vec<(String, Vec<&Client>)> = Vec::new();
            for c in wins {
                match groups.iter_mut().find(|(k, _)| *k == c.class) {
                    Some((_, l)) => l.push(c),
                    None => groups.push((c.class.clone(), vec![c])),
                }
            }
            groups
                .into_iter()
                .map(|(k, l)| {
                    if l.len() == 1 {
                        Entry::Window(l[0])
                    } else {
                        Entry::App(k, l)
                    }
                })
                .collect()
        }
        LauncherGroup::Workspace => {
            let mut sorted = wins;
            sorted.sort_by_key(|c| ws_key(&c.workspace));
            let mut out = Vec::new();
            let mut last: Option<&str> = None;
            for c in sorted {
                if last != Some(c.workspace.as_str()) {
                    out.push(Entry::Label(c.workspace.clone()));
                    last = Some(c.workspace.as_str());
                }
                out.push(Entry::Window(c));
            }
            out
        }
    }
}

/// Espaces de noms des surfaces du lanceur et de sa bulle.
pub const LAUNCHER_NAMESPACE: &str = "hyprdesk-launcher";
pub const POPUP_NAMESPACE: &str = "hyprdesk-launcher-popup";

/// Surface du lanceur sur un écran : couche `Top`, au-dessus des fenêtres,
/// contre un bord, avec une zone réservée (les fenêtres lui laissent la place).
#[derive(Debug, Clone, PartialEq)]
pub struct LauncherSurface {
    pub monitor: String,
    pub position: LauncherPosition,
    pub thickness: u32,
}

/// Contenu de la bulle du lanceur.
#[derive(Debug, Clone, PartialEq)]
pub enum PopupContent {
    /// Nom d'une entrée (fenêtre, application épinglée, bouton).
    Label(String),
    /// Fenêtres d'une application regroupée, à choisir.
    Windows(String),
}

/// Bulle ouverte au survol d'une entrée : sa propre surface (`Overlay`).
#[derive(Debug, Clone)]
pub struct Popup {
    pub id: iced::window::Id,
    pub monitor: String,
    pub key: String,
    pub content: PopupContent,
    pub(in crate::ui) fade: crate::ui::Fade,
    pub(in crate::ui) placement: lifecycle::Placement,
    pub max_height: f32,
}

/// Épaisseur de la surface du lanceur : boutons, carte, points, marge.
pub(in crate::ui) fn thickness(size: u16) -> u32 {
    size as u32 + 12 + 20 + 8 + 16
}

/// Taille de la bulle selon son contenu.
fn popup_size(content: &PopupContent, lines: usize) -> (u32, u32) {
    match content {
        PopupContent::Label(t) => ((t.chars().count() as u32 * 8 + 48).clamp(96, 456), 50),
        PopupContent::Windows(_) => (416, lines.max(1) as u32 * 36 + 30),
    }
}

/// Influence bornée à deux voisins, continue même entre deux boutons.
fn wave_weight(distance: f32) -> f32 {
    if distance.abs() >= 2.0 {
        0.0
    } else {
        (distance * std::f32::consts::FRAC_PI_4).cos().powi(4)
    }
}

impl App {
    /// Réemploie les transitions interruptibles du bureau, sans minuterie au repos.
    pub(in crate::ui) fn launcher_pointer(
        &mut self,
        monitor: &str,
        position: Option<f32>,
    ) -> Task<UiMsg> {
        let prefix = format!("dock-wave:{monitor}:");
        let mut keys: Vec<usize> = self
            .hover_fades
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix)?.parse().ok())
            .collect();
        if let Some(p) = position {
            let center = p.round().max(0.0) as usize;
            keys.extend(center.saturating_sub(2)..=center + 2);
        }
        keys.sort_unstable();
        keys.dedup();
        let now = std::time::Instant::now();
        let dur = self
            .motion_duration()
            .min(std::time::Duration::from_millis(140));
        for index in keys {
            let key = format!("{prefix}{index}");
            let to = position.map_or(0.0, |p| wave_weight(index as f32 - p));
            if self.hover_fades.get(&key).is_some_and(|f| {
                f.to == to || (to > 0.0 && f.to > 0.0 && (f.to - to).abs() < 0.002)
            }) {
                continue;
            }
            let from = self.hover_value(&key);
            if from == 0.0 && to == 0.0 {
                self.hover_fades.remove(&key);
                continue;
            }
            self.hover_fades.insert(
                key,
                super::super::Fade {
                    from,
                    to,
                    start: now,
                    dur,
                },
            );
        }
        Task::none()
    }

    /// Une surface de lanceur par écran où le bureau est affiché.
    pub(in crate::ui) fn sync_launchers(&mut self) -> Task<UiMsg> {
        use iced_layershell::reexport::{
            Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
        };
        let l = &self.cfg.general.launcher;
        let want: Vec<String> = if self.visible && self.cfg.plugin_on("launcher") {
            self.surfaces
                .iter()
                .filter(|(id, _)| self.raised.contains(id))
                .map(|(_, m)| m.clone())
                .collect()
        } else {
            Vec::new()
        };
        let spec = (l.position, thickness(l.size));
        let mut tasks = Vec::new();
        let mut changed = false;
        for (id, s) in self.launcher_surfaces.clone() {
            if !(want.contains(&s.monitor) && (s.position, s.thickness) == spec) {
                self.launcher_surfaces.remove(&id);
                self.launcher_runtime.docks.remove(&s.monitor);
                if self
                    .launcher_runtime
                    .focus
                    .as_ref()
                    .is_some_and(|(m, _)| m == &s.monitor)
                {
                    self.launcher_runtime.focus = None;
                }
                self.launcher_runtime
                    .anchors
                    .retain(|(m, _), _| m != &s.monitor);
                let _ = self.launcher_pointer(&s.monitor, None);
                tasks.push(iced::window::close(id));
                changed = true;
            }
        }
        for mon in want {
            if self.launcher_surfaces.values().any(|s| s.monitor == mon) {
                continue;
            }
            let id = iced::window::Id::unique();
            let grabbed = self.launcher_runtime.focus.as_ref().map(|(m, _)| m.clone());
            let (pos, th) = spec;
            let fade = self.launcher_transition(0.0, 0.0, 0);
            self.launcher_runtime.docks.insert(mon.clone(), fade);
            let (size, anchor) = match pos {
                LauncherPosition::Bottom => {
                    ((0, th), Anchor::Bottom | Anchor::Left | Anchor::Right)
                }
                LauncherPosition::Top => ((0, th), Anchor::Top | Anchor::Left | Anchor::Right),
                LauncherPosition::Left => ((th, 0), Anchor::Left | Anchor::Top | Anchor::Bottom),
                LauncherPosition::Right => ((th, 0), Anchor::Right | Anchor::Top | Anchor::Bottom),
            };
            self.launcher_surfaces.insert(
                id,
                LauncherSurface {
                    monitor: mon.clone(),
                    position: pos,
                    thickness: th,
                },
            );
            tasks.push(Task::done(UiMsg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some(size),
                    layer: Layer::Top,
                    anchor,
                    exclusive_zone: Some(th as i32),
                    // Ouvert au clavier (launcher-only) : il prend le clavier d'emblée.
                    keyboard_interactivity: if grabbed.as_deref() == Some(mon.as_str()) {
                        KeyboardInteractivity::Exclusive
                    } else {
                        KeyboardInteractivity::None
                    },
                    output_option: OutputOption::OutputName(mon),
                    namespace: Some(LAUNCHER_NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
            changed = true;
        }
        if changed {
            tasks.push(self.close_launcher_popup());
            // La zone réservée a changé : Hyprland ne l'annonce pas, on relit les écrans.
            tasks.push(Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    crate::hypr::monitors_async().await
                },
                |m| UiMsg::Hypr(crate::hypr::Update::Monitors(m)),
            ));
        }
        Task::batch(tasks)
    }

    /// Le lanceur dans sa surface : applications épinglées, fenêtres, puis
    /// boutons de l'assistant et des réglages, contre le bord choisi.
    pub(in crate::ui) fn view_launcher<'a>(&'a self, m: &Monitor, t: f32) -> Element<'a, UiMsg> {
        let progress = self.launcher_progress(&m.name);
        let t = t * progress;
        let g = &self.cfg.general;
        let l = &g.launcher;
        let vertical = l.position.vertical();
        let fg = color(g.theme.fg, t);
        let dim = color(g.theme.fg, 0.55 * t);
        let size = l.size as f32;
        let accent = color(g.theme.accent, t);
        let face = |class: &str, px: f32| -> Element<'a, UiMsg> {
            let img = self.class_imgs.get(class).copied().unwrap_or(IconImg::None);
            self.img_element(img, px, t).unwrap_or_else(|| {
                container(
                    text(
                        class
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_uppercase()
                            .to_string(),
                    )
                    .font(self.font)
                    .size(px * 0.5)
                    .color(fg),
                )
                .center(px)
                .style(move |_| container::Style {
                    background: Some(
                        Color {
                            a: 0.16 * t,
                            ..accent
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(px * 0.24),
                    ..Default::default()
                })
                .into()
            })
        };
        let glyph = |g: &Glyph| -> Element<'a, UiMsg> {
            match g {
                Glyph::App(id) => face(id, size),
                Glyph::Symbol(name, scale, fallback) => super::super::icons::get(name)
                    .map(|h| {
                        iced::widget::svg(h)
                            .width(size * scale)
                            .height(size * scale)
                            .style(move |_, _| iced::widget::svg::Style { color: Some(fg) })
                            .into()
                    })
                    .unwrap_or_else(|| text(*fallback).font(self.font).size(13).color(fg).into()),
                Glyph::Settings => self
                    .img_element(
                        self.ui_imgs
                            .get(super::super::ICON_SETTINGS)
                            .copied()
                            .unwrap_or(IconImg::None),
                        size * 0.6,
                        t,
                    )
                    .unwrap_or_else(|| {
                        text(crate::i18n::tr("Réglages"))
                            .font(self.font)
                            .size(13)
                            .color(fg)
                            .into()
                    }),
            }
        };
        let monitor = m.name.clone();
        let selected = self
            .launcher_runtime
            .focus
            .as_ref()
            .filter(|(name, _)| name == &m.name)
            .map(|(_, i)| *i);
        // Bouton d'une entrée, grossi au survol ; le survol ouvre la bulle.
        let item = |index: usize, entry: DockItem| -> Element<'a, UiMsg> {
            let DockItem {
                key,
                glyph: look,
                badge,
                active,
                popup,
                msg,
            } = entry;
            let face = glyph(&look);
            // Sélection au clavier : anneau plein, au contraste du texte.
            let ring = selected == Some(index);
            let over = self.hover_value(&format!("dock-wave:{monitor}:{index}"));
            let launching = key
                .strip_prefix("dock-pin:")
                .and_then(|id| self.launcher_runtime.launches.get(id));
            let pulse = launching.map(|launch| {
                if g.transition == crate::config::Transition::None {
                    1.0
                } else {
                    0.5 + 0.5
                        * (launch.started.elapsed().as_secs_f32() * std::f32::consts::TAU / 1.1)
                            .sin()
                }
            });
            let motion = if g.transition == crate::config::Transition::None {
                0.0
            } else {
                over
            };
            let lift = match l.position {
                LauncherPosition::Bottom => iced::Vector::new(0.0, -2.5 * motion),
                LauncherPosition::Top => iced::Vector::new(0.0, 2.5 * motion),
                LauncherPosition::Left => iced::Vector::new(2.5 * motion, 0.0),
                LauncherPosition::Right => iced::Vector::new(-2.5 * motion, 0.0),
            };
            // Halo et icône partagent une boîte fixe ; aucun déplacement des voisins.
            let face = motion::face(
                container(face).center(size),
                1.0 + 0.09 * motion,
                lift,
                self.motion_duration(),
            );
            let icon: Element<'a, UiMsg> = container(face)
                .style(move |_| container::Style {
                    background: Some(
                        Color {
                            a: 0.07 * over * t,
                            ..accent
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(12),
                    ..Default::default()
                })
                .into();
            // La piste de 4 px est réservée même pour une application fermée.
            let mark = container(Space::new())
                .width(if vertical {
                    4.0
                } else if active || launching.is_some() {
                    12.0
                } else {
                    4.0
                })
                .height(if vertical && (active || launching.is_some()) {
                    12.0
                } else {
                    4.0
                })
                .style(move |_| container::Style {
                    background: if let Some(pulse) = pulse {
                        Some(
                            Color {
                                a: (0.4 + 0.6 * pulse) * t,
                                ..accent
                            }
                            .into(),
                        )
                    } else {
                        (badge > 0).then_some(Background::Color(if active {
                            accent
                        } else {
                            Color { a: 0.5 * t, ..fg }
                        }))
                    },
                    border: Border::default().rounded(2),
                    ..Default::default()
                });
            let indicator: Element<'a, UiMsg> = if vertical {
                container(mark).width(4).height(size).center_y(size).into()
            } else {
                container(mark).width(size).height(4).center_x(size).into()
            };
            let body: Element<'a, UiMsg> = match l.position {
                LauncherPosition::Bottom => column![icon, indicator].spacing(4).into(),
                LauncherPosition::Top => column![indicator, icon].spacing(4).into(),
                LauncherPosition::Left => row![indicator, icon].spacing(4).into(),
                LauncherPosition::Right => row![icon, indicator].spacing(4).into(),
            };
            let pointer_monitor = monitor.clone();
            let area = mouse_area(
                button(body)
                    .padding(6)
                    .on_press(UiMsg::LauncherActivate(Box::new(msg)))
                    .style(move |_, status| {
                        let alpha = match status {
                            button::Status::Pressed => 0.15,
                            button::Status::Hovered => 0.025 + 0.035 * over,
                            _ if ring => 0.06,
                            _ => 0.025 * over,
                        };
                        button::Style {
                            background: Some(Color { a: alpha * t, ..fg }.into()),
                            border: if ring {
                                Border::default().rounded(12).width(2).color(fg)
                            } else {
                                Border::default().rounded(12)
                            },
                            ..Default::default()
                        }
                    }),
            )
            .on_enter(UiMsg::LauncherHover(Some((
                monitor.clone(),
                key.clone(),
                popup,
                index,
            ))))
            .on_exit(UiMsg::LauncherHover(None))
            .on_move(move |p| {
                let along = if vertical { p.y } else { p.x };
                UiMsg::LauncherPointer(
                    pointer_monitor.clone(),
                    Some(index as f32 + (along - (size + 12.0) / 2.0) / (size + 16.0)),
                )
            });
            motion::anchor(area, monitor.clone(), key)
        };
        let mut items: Vec<Element<'a, UiMsg>> = Vec::new();
        for (index, slot) in self.dock_slots(m).into_iter().enumerate() {
            items.push(match slot {
                Slot::Item(entry) => item(index, *entry),
                Slot::Label(ws) => container(text(ws).font(self.title_font()).size(12).color(dim))
                    .padding(Padding {
                        left: 6.0,
                        right: 2.0,
                        ..Padding::ZERO
                    })
                    .into(),
                Slot::Separator => {
                    let sep = Color { a: 0.12 * t, ..fg };
                    let line = if vertical {
                        iced::widget::Space::new().width(size * 0.6).height(1)
                    } else {
                        iced::widget::Space::new().width(1).height(size * 0.6)
                    };
                    container(container(line).style(move |_| container::Style {
                        background: Some(sep.into()),
                        ..Default::default()
                    }))
                    .padding(if vertical { [4, 0] } else { [0, 4] })
                    .into()
                }
            });
        }
        if items.is_empty() {
            return iced::widget::Space::new().into();
        }
        let inner: Element<'a, UiMsg> = if vertical {
            column(items).spacing(4).align_x(Alignment::Center).into()
        } else {
            row(items).spacing(4).align_y(Alignment::Center).into()
        };
        let roi = self
            .launcher_runtime
            .anchors
            .iter()
            .filter(|((name, _), _)| name == &m.name)
            .map(|(_, r)| lifecycle::screen_anchor(m, l.position, l.size, *r))
            .reduce(|a, b| a.union(&b))
            .unwrap_or_else(|| match l.position {
                LauncherPosition::Bottom => Rectangle::new(
                    Point::new(0.0, m.height as f32 - thickness(l.size) as f32),
                    Size::new(m.width as f32, thickness(l.size) as f32),
                ),
                LauncherPosition::Top => Rectangle::new(
                    Point::ORIGIN,
                    Size::new(m.width as f32, thickness(l.size) as f32),
                ),
                LauncherPosition::Left => Rectangle::new(
                    Point::ORIGIN,
                    Size::new(thickness(l.size) as f32, m.height as f32),
                ),
                LauncherPosition::Right => Rectangle::new(
                    Point::new(m.width as f32 - thickness(l.size) as f32, 0.0),
                    Size::new(thickness(l.size) as f32, m.height as f32),
                ),
            });
        let base = self.launcher_material_opacity(&m.name, roi);
        let dock = container(inner)
            .padding(10)
            .style(move |_| design::launcher_surface(&self.theme, t, false, base));
        let dock = mouse_area(dock).on_exit(UiMsg::LauncherPointer(monitor, None));
        let dock = motion::surface(
            dock,
            1.0,
            lifecycle::edge_offset(l.position, (1.0 - progress) * 8.0),
            Point::new(0.5, 0.5),
        );
        let shown_monitor = m.name.clone();
        let dock = iced::widget::sensor(dock)
            .key(m.name.clone())
            .on_show(move |_| UiMsg::LauncherShown(shown_monitor.clone()));
        let (ax, ay) = match l.position {
            LauncherPosition::Bottom => (Alignment::Center, Alignment::End),
            LauncherPosition::Top => (Alignment::Center, Alignment::Start),
            LauncherPosition::Left => (Alignment::Start, Alignment::Center),
            LauncherPosition::Right => (Alignment::End, Alignment::Center),
        };
        container(dock)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(ax)
            .align_y(ay)
            .padding(8)
            .into()
    }

    /// Cases du dock d'un écran, dans l'ordre affiché : applications
    /// épinglées, fenêtres, séparateur, puis boutons de commande. Une seule
    /// source pour le rendu et pour la navigation au clavier.
    pub(in crate::ui) fn dock_slots(&self, m: &Monitor) -> Vec<Slot> {
        let g = &self.cfg.general;
        let l = &g.launcher;
        let vertical = l.position.vertical();
        let size = l.size as f32;
        let assistant = l.show_assistant
            && self.cfg.plugin_on("assistant")
            && crate::assistant::lyra_installed();
        let extent = if vertical { m.height } else { m.width } as f32;
        let commands = usize::from(assistant) + usize::from(l.show_settings);
        let command_width = (commands as f32 * (size + 16.0) - 4.0).max(0.0);
        // Deux marges de 8 px et deux de 10 px. Le séparateur occupe 9 px,
        // avec 4 px de chaque côté ; les boutons de commande gardent leur place.
        let room = ((extent - 36.0 - command_width - if commands > 0 { 13.0 } else { -4.0 })
            / (size + 16.0))
            .max(0.0) as usize;
        let wins = windows(&self.clients, g.running_apps, &m.workspace);
        let focused = |c: &Client| {
            m.focused
                && self
                    .active_window
                    .as_ref()
                    .is_some_and(|(class, title)| class == &c.class && title == &c.title)
        };
        let mut slots: Vec<Slot> = Vec::new();
        // Les épingles gardent leur rang, ouvertes ou fermées.
        for id in &l.pinned {
            if slots.len() >= room {
                break;
            }
            let name = self
                .apps
                .iter()
                .find(|a| &a.id == id)
                .map_or(id.clone(), |a| a.name.clone());
            let open: Vec<_> = wins
                .iter()
                .copied()
                .filter(|c| self.app_matches(id, &c.class))
                .collect();
            let (popup, msg) = match open.as_slice() {
                [] => (PopupContent::Label(name), UiMsg::LauncherLaunch(id.clone())),
                [c] => (
                    PopupContent::Label(name),
                    UiMsg::FocusWindow(c.address.clone()),
                ),
                [c, ..] => (
                    PopupContent::Windows(c.class.clone()),
                    UiMsg::LauncherApp(m.name.clone(), c.class.clone()),
                ),
            };
            slots.push(Slot::Item(Box::new(DockItem {
                key: format!("dock-pin:{id}"),
                glyph: Glyph::App(id.clone()),
                badge: open.len(),
                active: open.iter().any(|c| focused(c)),
                popup,
                msg,
            })));
        }
        let unpinned = wins
            .into_iter()
            .filter(|c| !l.pinned.iter().any(|id| self.app_matches(id, &c.class)))
            .collect();
        for e in entries(unpinned, g.launcher_group).into_iter() {
            if slots.len() >= room {
                break;
            }
            slots.push(match e {
                Entry::Window(c) => Slot::Item(Box::new(DockItem {
                    key: format!("dock:{}", c.address),
                    glyph: Glyph::App(c.class.clone()),
                    badge: 1,
                    active: focused(c),
                    popup: PopupContent::Label(if c.title.is_empty() {
                        c.class.clone()
                    } else {
                        c.title.clone()
                    }),
                    msg: UiMsg::FocusWindow(c.address.clone()),
                })),
                Entry::App(class, list) => Slot::Item(Box::new(DockItem {
                    key: format!("dock-app:{class}"),
                    glyph: Glyph::App(class.clone()),
                    badge: list.len(),
                    active: list.iter().any(|c| focused(c)),
                    popup: PopupContent::Windows(class.clone()),
                    msg: UiMsg::LauncherApp(m.name.clone(), class),
                })),
                Entry::Label(ws) => Slot::Label(ws),
            });
        }
        if !slots.is_empty() && (assistant || l.show_settings) {
            slots.push(Slot::Separator);
        }
        // Boutons de commande : la pastille d'accent dit ce qui est ouvert,
        // et le même bouton le referme.
        let (apps_open, assistant_open, settings_open) = (
            self.search.is_some(),
            self.assistant_shown(),
            self.panel_open(),
        );
        // Toutes les applications : la grille plein écran.
        slots.push(Slot::Item(Box::new(DockItem {
            key: "dock-btn:apps".into(),
            glyph: Glyph::Symbol("view-app-grid", 0.55, "⋯"),
            badge: usize::from(apps_open),
            active: apps_open,
            popup: PopupContent::Label(crate::i18n::tr("Toutes les applications").into()),
            msg: UiMsg::Control(crate::control::Command::Apps),
        })));
        if assistant {
            slots.push(Slot::Item(Box::new(DockItem {
                key: "dock-btn:assistant".into(),
                glyph: Glyph::Symbol("ai-assistant", 0.6, "IA"),
                badge: usize::from(assistant_open),
                active: assistant_open,
                popup: PopupContent::Label("Assistant IA (LYRA)".into()),
                msg: UiMsg::OpenAssistant,
            })));
        }
        if l.show_settings {
            slots.push(Slot::Item(Box::new(DockItem {
                key: "dock-btn:settings".into(),
                glyph: Glyph::Settings,
                badge: usize::from(settings_open),
                active: settings_open,
                popup: PopupContent::Label(crate::i18n::tr("Réglages").into()),
                msg: UiMsg::OpenPanel,
            })));
        }
        slots
    }

    /// Fenêtres d'une application regroupée, telles que le lanceur les montre.
    fn popup_windows(&self, monitor: &str, class: &str) -> Vec<&Client> {
        let ws = self
            .monitors
            .iter()
            .find(|m| m.name == monitor)
            .map(|m| m.workspace.clone())
            .unwrap_or_default();
        windows(&self.clients, self.cfg.general.running_apps, &ws)
            .into_iter()
            .filter(|c| c.class == class)
            .collect()
    }

    /// Le survol prépare la bulle ; le délai est annulable par un autre survol.
    pub(in crate::ui) fn launcher_hover(
        &mut self,
        spec: Option<(String, String, PopupContent, usize)>,
    ) -> Task<UiMsg> {
        self.launcher_popup_token = self.launcher_popup_token.wrapping_add(1);
        let token = self.launcher_popup_token;
        let Some((monitor, key, content, index)) = spec else {
            self.launcher_runtime.pending = None;
            return self.schedule_popup_close();
        };
        let _ = self.launcher_pointer(&monitor, Some(index as f32));
        self.launcher_popup_hover = false;
        if self
            .launcher_popup
            .as_ref()
            .is_some_and(|p| p.key == key && p.monitor == monitor && p.content == content)
        {
            self.launcher_runtime.pending = None;
            if self.launcher_popup.as_ref().unwrap().fade.to == 0.0 {
                let value = self
                    .launcher_popup
                    .as_ref()
                    .unwrap()
                    .fade
                    .value(std::time::Instant::now());
                let fade = self.launcher_transition(value, 1.0, 160);
                self.launcher_popup.as_mut().unwrap().fade = fade;
            }
            return Task::none();
        }
        self.launcher_runtime.pending = Some(lifecycle::Pending {
            monitor,
            key,
            content,
            token,
        });
        let close = self.dismiss_launcher_popup();
        let open = Task::perform(
            async { tokio::time::sleep(lifecycle::OPEN_DELAY).await },
            move |_| UiMsg::LauncherPopupOpen(token),
        );
        Task::batch([close, open])
    }

    pub(in crate::ui) fn launcher_popup_open(&mut self, token: u64) -> Task<UiMsg> {
        use iced_layershell::reexport::{
            Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
        };
        if !self.visible
            || !self.cfg.plugin_on("launcher")
            || token != self.launcher_popup_token
            || !self
                .launcher_runtime
                .pending
                .as_ref()
                .is_some_and(|p| p.token == token)
        {
            return Task::none();
        }
        let pending = self.launcher_runtime.pending.take().unwrap();
        let Some(local) = self
            .launcher_runtime
            .anchors
            .get(&(pending.monitor.clone(), pending.key.clone()))
            .copied()
        else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == pending.monitor) else {
            return Task::none();
        };
        let l = &self.cfg.general.launcher;
        let anchor = lifecycle::screen_anchor(m, l.position, l.size, local);
        let lines = match &pending.content {
            PopupContent::Windows(class) => self.popup_windows(&pending.monitor, class).len(),
            PopupContent::Label(_) => 1,
        };
        let (w, h) = popup_size(&pending.content, lines);
        let max_height = (m.height as f32 - m.reserved.1 - m.reserved.3 - 16.0).max(36.0);
        let placement = lifecycle::place_popup(
            l.position,
            lifecycle::workarea(m, l.position, l.size),
            anchor,
            Size::new(w as f32, (h as f32).min(max_height)),
        );
        let close = self.close_launcher_popup();
        let id = iced::window::Id::unique();
        self.launcher_popup = Some(Popup {
            id,
            monitor: pending.monitor.clone(),
            key: pending.key,
            content: pending.content,
            fade: self.launcher_transition(0.0, 1.0, 160),
            placement,
            max_height,
        });
        let bounds = placement.bounds;
        Task::batch([
            close,
            Task::done(UiMsg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((bounds.width as u32, bounds.height as u32)),
                    layer: Layer::Overlay,
                    anchor: Anchor::Top | Anchor::Left,
                    // Coordonnées absolues dans l'écran, indépendantes des zones exclusives.
                    exclusive_zone: Some(-1),
                    margin: Some((bounds.y as i32, 0, 0, bounds.x as i32)),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(pending.monitor),
                    namespace: Some(POPUP_NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }),
        ])
    }

    fn dismiss_launcher_popup(&mut self) -> Task<UiMsg> {
        if let Some(p) = &self.launcher_popup {
            if p.fade.to == 0.0 {
                return Task::none();
            }
            let value = p.fade.value(std::time::Instant::now());
            let fade = self.launcher_transition(value, 0.0, 125);
            self.launcher_popup.as_mut().unwrap().fade = fade;
            return Task::done(UiMsg::Frame);
        }
        Task::none()
    }

    fn schedule_popup_close(&self) -> Task<UiMsg> {
        let token = self.launcher_popup_token;
        Task::perform(
            async { tokio::time::sleep(std::time::Duration::from_millis(250)).await },
            move |_| UiMsg::LauncherPopupClose(token),
        )
    }

    /// Fin du délai : la bulle se ferme, sauf si la souris est dessus ou si
    /// une autre entrée a été survolée entre-temps.
    pub(in crate::ui) fn launcher_popup_timeout(&mut self, token: u64) -> Task<UiMsg> {
        if token != self.launcher_popup_token || self.launcher_popup_hover {
            return Task::none();
        }
        self.dismiss_launcher_popup()
    }

    pub(in crate::ui) fn launcher_popup_hover(&mut self, on: bool) -> Task<UiMsg> {
        self.launcher_popup_hover = on;
        if on {
            self.launcher_popup_token = self.launcher_popup_token.wrapping_add(1);
            self.launcher_runtime.pending = None;
            if let Some(p) = &self.launcher_popup {
                let fade =
                    self.launcher_transition(p.fade.value(std::time::Instant::now()), 1.0, 160);
                self.launcher_popup.as_mut().unwrap().fade = fade;
            }
            Task::none()
        } else {
            self.schedule_popup_close()
        }
    }

    pub(in crate::ui) fn close_launcher_popup(&mut self) -> Task<UiMsg> {
        self.launcher_popup_hover = false;
        self.launcher_runtime.pending = None;
        match self.launcher_popup.take() {
            Some(p) => iced::window::close(p.id),
            None => Task::none(),
        }
    }

    /// La bulle : un nom, ou la liste des fenêtres d'une application.
    pub(in crate::ui) fn view_launcher_popup(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.launcher_popup else {
            return iced::widget::Space::new().into();
        };
        let g = &self.cfg.general;
        let t = if g.transition == crate::config::Transition::None {
            p.fade.to
        } else {
            p.fade.value(std::time::Instant::now())
        };
        let fg = color(g.theme.fg, t);
        let dim = color(g.theme.fg, 0.55 * t);
        let accent = color(g.theme.accent, t);
        let hover = move |_: &iced::Theme, s: button::Status| button::Style {
            background: matches!(s, button::Status::Hovered)
                .then(|| Background::Color(Color { a: 0.15 * t, ..fg })),
            border: Border::default().rounded(8),
            ..Default::default()
        };
        let body: Element<'_, UiMsg> = match &p.content {
            PopupContent::Label(t) => text(t.clone()).font(self.font).size(13).color(fg).into(),
            PopupContent::Windows(class) => {
                column(self.popup_windows(&p.monitor, class).into_iter().map(|c| {
                    let active =
                        self.monitors
                            .iter()
                            .any(|m| m.name == p.monitor && m.focused)
                            && self.active_window.as_ref().is_some_and(|(class, title)| {
                                class == &c.class && title == &c.title
                            });
                    let title = if c.title.is_empty() {
                        c.class.clone()
                    } else {
                        c.title.clone()
                    };
                    let short: String = if title.chars().count() > 44 {
                        format!("{}…", title.chars().take(43).collect::<String>())
                    } else {
                        title
                    };
                    let img = self
                        .class_imgs
                        .get(&c.class)
                        .copied()
                        .unwrap_or(IconImg::None);
                    let icon = self
                        .img_element(img, 18.0, t)
                        .unwrap_or_else(|| iced::widget::Space::new().width(18).into());
                    button(
                        row![
                            icon,
                            text(short)
                                .font(self.font)
                                .size(13)
                                .color(fg)
                                .width(Length::Fill),
                            text(format!("ws {}", c.workspace))
                                .font(self.font)
                                .size(11)
                                .color(dim),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .height(34)
                    .padding([6, 10])
                    .style(move |theme, status| {
                        let mut style = hover(theme, status);
                        if active {
                            style.background = Some(
                                Color {
                                    a: t * if matches!(status, button::Status::Hovered) {
                                        0.18
                                    } else {
                                        0.10
                                    },
                                    ..accent
                                }
                                .into(),
                            );
                            style.border = style.border.width(1).color(Color {
                                a: 0.18 * t,
                                ..accent
                            });
                        }
                        style
                    })
                    .on_press(UiMsg::LauncherPick(c.address.clone()))
                    .into()
                }))
                .spacing(2)
                .into()
            }
        };
        let popup_id = p.id;
        let measured = iced::widget::sensor(body)
            .key(p.id)
            .on_resize(move |size| UiMsg::LauncherPopupMeasured(popup_id, size.height));
        let body = iced::widget::scrollable(measured).height(Length::Fill);
        let base = self.launcher_popup_opacity();
        let card = mouse_area(
            container(body)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8)
                .center_y(Length::Fill)
                .style(move |_| design::launcher_surface(&self.theme, t, true, base)),
        )
        .on_enter(UiMsg::LauncherPopupHover(true))
        .on_exit(UiMsg::LauncherPopupHover(false));
        let card = motion::surface(
            card,
            0.97 + 0.03 * t,
            lifecycle::edge_offset(g.launcher.position, (1.0 - t) * 6.0),
            p.placement.pivot,
        );
        container(card)
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl App {
    /// Une fenêtre de classe `class` appartient-elle à l'application `id` ?
    pub(in crate::ui) fn app_matches(&self, id: &str, class: &str) -> bool {
        let class = class.to_lowercase();
        id.to_lowercase() == class
            || self.apps.iter().any(|a| {
                a.id == id
                    && a.wm_class
                        .as_deref()
                        .is_some_and(|w| w.to_lowercase() == class)
            })
    }

    /// Lance une application épinglée ; le mode du bureau ne change pas.
    pub(in crate::ui) fn launcher_launch(&mut self, id: &str) -> Task<UiMsg> {
        if self.launcher_runtime.launches.contains_key(id) {
            return Task::none();
        }
        let feedback = self.launcher_launch_started(id.to_string());
        let token = self.launcher_runtime.launches[id].started;
        let def = crate::config::IconDef {
            origin: Default::default(),
            sel: Default::default(),
            id: id.to_string(),
            label: None,
            kind: crate::config::IconKind::App(id.to_string()),
            icon: None,
            pos: None,
        };
        let terminal = self.cfg.general.terminal.clone();
        let id = id.to_string();
        let resolve = Task::perform(
            async move {
                let command = tokio::task::spawn_blocking(move || {
                    crate::launch::resolve(&def, None, 16, &terminal).command
                })
                .await
                .unwrap_or_default();
                (id, command)
            },
            move |(id, command)| UiMsg::LauncherResolved(id, token, command),
        );
        Task::batch([feedback, resolve])
    }

    /// Clic sur une application regroupée : sa fenêtre suivante, à tour de
    /// rôle. Comme toute action du lanceur, le mode du bureau ne change pas.
    pub(in crate::ui) fn launcher_focus_app(&mut self, monitor: &str, class: &str) -> Task<UiMsg> {
        let workspace = self
            .monitors
            .iter()
            .find(|m| m.name == monitor)
            .map(|m| m.workspace.clone())
            .unwrap_or_default();
        let list: Vec<String> = windows(&self.clients, self.cfg.general.running_apps, &workspace)
            .into_iter()
            .filter(|c| c.class == class)
            .map(|c| c.address.clone())
            .collect();
        if list.is_empty() {
            return Task::none();
        }
        let n = self.launcher_cycle.entry(class.to_string()).or_insert(0);
        let address = list[*n % list.len()].clone();
        *n = n.wrapping_add(1);
        self.update_inner(UiMsg::FocusWindow(address))
    }

    pub(in crate::ui) fn launcher_page(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let running = |label: &'static str, v: RunningApps| {
            radio(crate::i18n::tr(label), v, Some(g.running_apps), |v| {
                wrap(Msg::Running(v))
            })
            .font(self.font)
        };
        let group = |label: &'static str, v: LauncherGroup| {
            radio(crate::i18n::tr(label), v, Some(g.launcher_group), |v| {
                wrap(Msg::Group(v))
            })
            .font(self.font)
        };
        if self.on_subtab(INFO.id, "applications") {
            return self.launcher_apps_tab();
        }
        if !self.on_subtab(INFO.id, "contenu") {
            return self.launcher_look();
        }
        column![
            self.section("Fenêtres"),
            self.field(
                "Fenêtres ouvertes dans le lanceur",
                "running_apps",
                column![
                    running(crate::i18n::tr("Aucune (épingles et boutons uniquement)"), RunningApps::Off),
                    running(crate::i18n::tr("Celles du workspace affiché"), RunningApps::Workspace),
                    running(crate::i18n::tr("Toutes"), RunningApps::All),
                ]
                .spacing(8),
            ),
            self.section("Regroupement"),
            self.field(
                "Regrouper les fenêtres",
                "launcher_group",
                column![
                    group(crate::i18n::tr("Aucun : une entrée par fenêtre"), LauncherGroup::None),
                    group(
                        crate::i18n::tr("Par application (binaire) : une entrée, un clic passe de l'une à l'autre"),
                        LauncherGroup::App
                    ),
                    group(
                        crate::i18n::tr("Par workspace : rangées sous leur numéro"),
                        LauncherGroup::Workspace
                    ),
                ]
                .spacing(8),
            ),
            text(
                "Le lanceur a sa propre couche, au-dessus des fenêtres : en launcher-only, \
                 les fenêtres lui laissent sa place. L'utiliser ne change jamais le mode du bureau."
            )
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(14)
        .into()
    }

    /// Ce qu'ouvre le raccourci du lanceur hors plein bureau, puis en plein bureau.
    fn launcher_keys_text(&self) -> (String, String) {
        let g = &self.cfg.general;
        let k = &g.keys;
        let r = &self.key_report;
        let held = |list: &[crate::keymap::Held]| {
            list.iter()
                .find(|h| h.action.id == "search")
                .map(|h| h.holder.describe())
        };
        let outside = if k.search.is_empty() {
            crate::i18n::tr("Hors plein bureau : aucun raccourci (le dock, ou hyprdesk apps).")
                .to_string()
        } else if let Some(h) = held(&r.left) {
            crate::tr!("Hors plein bureau : {k} reste à {h}.", k = k.search, h = h)
        } else if let Some(h) = held(&r.taken) {
            crate::tr!(
                "Hors plein bureau : {k} ouvre ce lanceur, à la place de {h}.",
                k = k.search,
                h = h
            )
        } else {
            crate::tr!("Hors plein bureau : {k} ouvre ce lanceur.", k = k.search)
        };
        let desk = if k.search_desktop.is_empty() {
            &k.search
        } else {
            &k.search_desktop
        };
        let inside = if desk.is_empty() {
            crate::i18n::tr("En plein bureau : aucun raccourci (le dock, ou hyprdesk apps).")
                .to_string()
        } else {
            crate::tr!("En plein bureau : {k} ouvre ce lanceur.", k = desk)
        };
        (outside, inside)
    }

    /// Réglages d'apparence : bord, taille, applications épinglées, boutons.
    /// Onglet Applications : le lanceur d'applications.
    fn launcher_apps_tab(&self) -> Element<'_, UiMsg> {
        use crate::config::AppsStyle;
        let g = &self.cfg.general;
        let a = g.apps_launcher;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let style = |label: &'static str, v: AppsStyle| {
            radio(crate::i18n::tr(label), v, Some(a.style), |v| {
                wrap(Msg::AppsStyle(v))
            })
            .font(self.font)
        };
        let (outside, inside) = self.launcher_keys_text();
        let desk_input = self
            .panel
            .as_ref()
            .and_then(|p| p.launcher.desk_key.clone())
            .unwrap_or_else(|| g.keys.search_desktop.clone());
        let mut col = column![
            self.section("Lanceur d'applications"),
            text("Un seul lanceur, quelle que soit la façon de l'ouvrir (raccourci, bouton du dock, hyprdesk apps) : en plein bureau, une grande feuille de toutes les applications ; dans les autres modes, une carte au centre de l'écran. Les applications que vous lancez le plus sont toujours mises en avant.")
                .font(self.font)
                .size(12)
                .color(dim),
            self.field(
                "Présentation",
                "apps_style",
                column![
                    style("Selon le mode (feuille en plein bureau, carte sinon)", AppsStyle::Auto),
                    style("Toujours la grande feuille", AppsStyle::Sheet),
                    style("Toujours la carte centrée", AppsStyle::Spotlight),
                ]
                .spacing(6),
            ),
            self.field(
                "Suggestions (les plus lancées)",
                "apps_suggestions",
                row![
                    iced::widget::slider(0.0..=16.0, a.suggestions as f32, |v| wrap(
                        Msg::AppsSuggestions(v.round() as u8)
                    ))
                    .step(1.0_f32)
                    .on_release(wrap(Msg::AppsSuggestionsCommit))
                    .width(220),
                    text(a.suggestions.to_string()).font(self.font).size(13),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            ),
            self.field(
                "Catégories dans la grande feuille",
                "apps_categories",
                iced::widget::toggler(a.categories).on_toggle(|v| wrap(Msg::AppsCategories(v))),
            ),
            self.section("Raccourci"),
            self.field(
                "Le même lanceur dans tous les modes",
                "apps_unified",
                iced::widget::toggler(a.unified).on_toggle(|v| wrap(Msg::AppsUnified(v))),
            ),
            text(outside).font(self.font).size(13).color(fg),
            text(inside).font(self.font).size(13).color(fg),
            self.field(
                "Plein bureau : un autre raccourci",
                "keys.search_desktop",
                iced::widget::text_input(crate::i18n::tr("vide : le même"), &desk_input)
                    .font(self.font)
                    .on_input(|v| wrap(Msg::DeskKey(v)))
                    .on_submit(wrap(Msg::DeskKeySubmit))
                    .padding(8)
                    .width(220),
            ),
            text("Les autres raccourcis, et tous les conflits : Réglages → Bureau → Raccourcis.")
                .font(self.font)
                .size(11)
                .color(dim),
        ]
        .spacing(12);
        if let Some(e) = self
            .panel
            .as_ref()
            .and_then(|p| p.launcher.desk_error.clone())
        {
            col = col.push(text(e).font(self.font).size(12).color(warn));
        }
        // Un raccourci du lanceur en conflit : hyprdesk le prend, ou LYRA.
        for i in ["keys.search", "keys.search_desktop"]
            .iter()
            .filter_map(|k| self.key_report.issue_for(k))
        {
            let mut line = row![
                text(i.message())
                    .font(self.font)
                    .size(12)
                    .color(warn)
                    .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            if i.holder().is_some() {
                line = line.push(
                    button(
                        text(crate::i18n::tr("hyprdesk le prend"))
                            .font(self.font)
                            .size(12),
                    )
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(super::desktop::wrap(
                        super::desktop::Msg::TakeKey(i.action.combo.clone()),
                    )),
                );
            }
            line = line.push(self.lyra_button(
                crate::i18n::tr("Lanceur d'applications"),
                crate::keymap::lyra_prompt(&self.key_report),
            ));
            col = col.push(line);
        }
        let mut actions = row![
            button(
                text(crate::i18n::tr("Ouvrir le lanceur"))
                    .font(self.font)
                    .size(13)
            )
            .padding([6, 14])
            .on_press(UiMsg::Control(crate::control::Command::Apps)),
        ]
        .spacing(10);
        if g.keys.search != "SUPER + R" {
            actions = actions.push(
                button(
                    text(crate::i18n::tr("Utiliser SUPER + R"))
                        .font(self.font)
                        .size(13),
                )
                .padding([6, 14])
                .style(button::secondary)
                .on_press(wrap(Msg::UseSuperR)),
            );
        }
        col.push(actions).into()
    }

    fn launcher_look(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let l = &g.launcher;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let filter = self
            .panel
            .as_ref()
            .map(|p| p.launcher.pin_filter.clone())
            .unwrap_or_default();
        let position = |label: &'static str, v: LauncherPosition| {
            radio(label, v, Some(l.position), |v| wrap(Msg::Position(v))).font(self.font)
        };
        let name = |id: &str| {
            self.apps
                .iter()
                .find(|a| a.id == id)
                .map_or(id.to_string(), |a| a.name.clone())
        };
        let mut pins = column![].spacing(6);
        if l.pinned.is_empty() {
            pins = pins.push(
                text("Aucune application épinglée.")
                    .font(self.font)
                    .size(13)
                    .color(dim),
            );
        }
        for (i, id) in l.pinned.iter().enumerate() {
            let small = |label: &'static str, msg: Option<UiMsg>| {
                button(text(label).font(self.font).size(12))
                    .padding([3, 8])
                    .style(button::secondary)
                    .on_press_maybe(msg)
            };
            pins = pins.push(
                row![
                    text(name(id)).font(self.font).size(14).width(Length::Fill),
                    small("↑", (i > 0).then(|| wrap(Msg::MovePin(i, -1)))),
                    small(
                        "↓",
                        (i + 1 < l.pinned.len()).then(|| wrap(Msg::MovePin(i, 1)))
                    ),
                    small("Retirer", Some(wrap(Msg::Unpin(i)))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        let f = filter.to_lowercase();
        let mut found = column![].spacing(2);
        if !f.is_empty() {
            for a in self
                .apps
                .iter()
                .filter(|a| {
                    !l.pinned.contains(&a.id)
                        && (a.name.to_lowercase().contains(&f) || a.id.to_lowercase().contains(&f))
                })
                .take(8)
            {
                found = found.push(
                    button(
                        row![
                            text(a.name.clone())
                                .font(self.font)
                                .size(14)
                                .width(Length::Fill),
                            text(a.id.clone()).font(self.font).size(11).color(dim),
                        ]
                        .spacing(8),
                    )
                    .width(Length::Fill)
                    .padding([6, 10])
                    .style(button::text)
                    .on_press(wrap(Msg::Pin(a.id.clone()))),
                );
            }
        }
        if self.on_subtab(INFO.id, "epinglees") {
            return column![
                self.section("Applications épinglées"),
                text("Toujours dans le lanceur ; un clic les lance si elles ne sont pas ouvertes.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
                pins,
                iced::widget::text_input(
                    crate::i18n::tr("Épingler une application (firefox, terminal…)"),
                    &filter
                )
                .font(self.font)
                .on_input(|v| wrap(Msg::PinFilter(v)))
                .padding(8),
                found,
            ]
            .spacing(14)
            .into();
        }
        column![
            self.section("Apparence"),
            self.field(
                "Bord de l'écran",
                "launcher_position",
                row![
                    position("En bas", LauncherPosition::Bottom),
                    position("En haut", LauncherPosition::Top),
                    position(crate::i18n::tr("À gauche"), LauncherPosition::Left),
                    position(crate::i18n::tr("À droite"), LauncherPosition::Right),
                ]
                .spacing(16),
            ),
            self.field(
                "Taille des icônes",
                "launcher_size",
                row![
                    iced::widget::slider(24..=96u16, l.size, |v| wrap(Msg::Size(v)))
                        .on_release(wrap(Msg::SizeCommit))
                        .width(260),
                    text(format!("{} px", l.size)).font(self.font).size(14),
                ]
                .spacing(12),
            ),
            self.field(
                "Boutons",
                "",
                column![
                    iced::widget::toggler(l.show_assistant)
                        .label(crate::i18n::tr("Assistant IA (LYRA)"))
                        .on_toggle(|v| wrap(Msg::ShowAssistant(v))),
                    iced::widget::toggler(l.show_settings)
                        .label(crate::i18n::tr("Réglages"))
                        .on_toggle(|v| wrap(Msg::ShowSettings(v))),
                ]
                .spacing(8),
            ),
            // SmartUI design : les icônes se créent et se valident dans
            // Réglages → IA → Autres services IA.
            self.field(
                "Icônes",
                "launcher_smartui",
                row![
                    iced::widget::toggler(l.smartui)
                        .label(crate::i18n::tr("SmartUI design (icônes unifiées par l'IA)"))
                        .on_toggle(
                            |v| super::assistant::wrap(super::assistant::Msg::SmartToggle(v))
                        ),
                    button(
                        text("Créer et valider les icônes…")
                            .font(self.font)
                            .size(12)
                    )
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(super::assistant::wrap(super::assistant::Msg::OpenServices)),
                ]
                .spacing(14)
                .align_y(Alignment::Center),
            ),
        ]
        .spacing(14)
        .into()
    }

    pub(in crate::ui) fn launcher_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Running(v) => {
                self.settings.running_apps = Some(v);
                self.commit()
            }
            Msg::Group(v) => {
                self.settings.launcher_group = Some(v);
                self.commit()
            }
            Msg::Position(v) => {
                self.settings.launcher_position = Some(v);
                self.commit()
            }
            Msg::Size(v) => {
                self.settings.launcher_size = Some(v);
                self.apply()
            }
            Msg::SizeCommit => self.commit(),
            Msg::ShowSettings(v) => {
                self.settings.launcher_show_settings = Some(v);
                self.commit()
            }
            Msg::ShowAssistant(v) => {
                self.settings.launcher_show_assistant = Some(v);
                self.commit()
            }
            Msg::AppsStyle(v) => {
                self.settings.apps_style = Some(v);
                self.commit()
            }
            Msg::AppsSuggestions(v) => {
                self.settings.apps_suggestions = Some(v);
                self.apply()
            }
            Msg::AppsSuggestionsCommit => self.commit(),
            Msg::AppsCategories(v) => {
                self.settings.apps_categories = Some(v);
                self.commit()
            }
            Msg::UseSuperR => {
                self.settings.key_search = Some("SUPER + R".into());
                self.commit()
            }
            Msg::AppsUnified(v) => {
                self.settings.apps_unified = Some(v);
                self.commit()
            }
            Msg::DeskKey(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.launcher.desk_key = Some(v);
                    p.launcher.desk_error = None;
                }
                Task::none()
            }
            Msg::DeskKeySubmit => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                let v = p
                    .launcher
                    .desk_key
                    .clone()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if !v.is_empty() && crate::hypr::parse_combo(&v).is_none() {
                    p.launcher.desk_error = Some(crate::tr!(
                        "« {v} » : raccourci illisible (ex. SUPER + SHIFT + D)",
                        v = v
                    ));
                    return Task::none();
                }
                let keys = crate::config::Keys {
                    search_desktop: v.clone(),
                    ..self.cfg.general.keys.clone()
                };
                if let Some(other) = crate::keymap::duplicate_of(
                    &keys,
                    self.cfg.general.apps_launcher.unified,
                    "keys.search_desktop",
                ) {
                    p.launcher.desk_error = Some(crate::tr!(
                        "« {v} » sert déjà à « {other} » : choisissez une autre combinaison.",
                        v = v,
                        other = crate::i18n::tr(other)
                    ));
                    return Task::none();
                }
                p.launcher.desk_key = None;
                // Saisi aussi dans Bureau → Raccourcis : le champ suit.
                p.desktop.keys[9] = v.clone();
                self.settings.key_search_desktop = Some(v);
                self.commit()
            }
            Msg::PinFilter(f) => {
                if let Some(p) = self.panel.as_mut() {
                    p.launcher.pin_filter = f;
                }
                Task::none()
            }
            Msg::Pin(id) => {
                let mut l = self.cfg.general.launcher.pinned.clone();
                if !l.contains(&id) {
                    l.push(id);
                }
                if let Some(p) = self.panel.as_mut() {
                    p.launcher.pin_filter.clear();
                }
                self.settings.launcher_pinned = Some(l);
                self.commit()
            }
            Msg::Unpin(i) => {
                let mut l = self.cfg.general.launcher.pinned.clone();
                if i < l.len() {
                    l.remove(i);
                }
                self.settings.launcher_pinned = Some(l);
                self.commit()
            }
            Msg::MovePin(i, d) => {
                let mut l = self.cfg.general.launcher.pinned.clone();
                let j = i as i64 + d as i64;
                if i < l.len() && j >= 0 && (j as usize) < l.len() {
                    l.swap(i, j as usize);
                }
                self.settings.launcher_pinned = Some(l);
                self.commit()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-launcher-test/config.lua"),
            true,
        )
        .0
    }

    #[test]
    fn hover_wave_retargets_without_jumping_and_stays_on_its_monitor() {
        use std::time::{Duration, Instant};
        let mut app = app();
        let _ = app.launcher_pointer("left", Some(1.0));
        let _ = app.launcher_pointer("right", Some(0.0));
        let key = "dock-wave:left:1";
        app.hover_fades.get_mut(key).unwrap().start = Instant::now() - Duration::from_millis(60);
        let before = app.hover_value(key);
        let _ = app.launcher_pointer("left", Some(1.8));
        let fade = app.hover_fades[key];
        assert!((fade.from - before).abs() < 0.02);
        assert!(fade.to < 1.0 && fade.to > 0.0);
        assert_eq!(app.hover_fades["dock-wave:right:0"].to, 1.0);
        let _ = app.launcher_pointer("left", None);
        assert!(
            app.hover_fades
                .iter()
                .filter(|(k, _)| k.starts_with("dock-wave:left:"))
                .all(|(_, f)| f.to == 0.0)
        );
        // Au-delà de la transition de sortie, plus aucune animation de survol.
        for fade in app.hover_fades.values_mut() {
            fade.start = Instant::now() - Duration::from_secs(1);
        }
        assert!(!app.animating());
    }

    #[test]
    fn disabled_motion_never_leaves_a_running_animation() {
        let mut app = app();
        std::sync::Arc::make_mut(&mut app.cfg).general.transition = crate::config::Transition::None;
        let _ = app.launcher_pointer("screen", Some(0.5));
        assert!(!app.animating());
        assert!(app.hover_fades.values().all(|f| f.dur.is_zero()));
        let _ = app.launcher_pointer("screen", None);
        assert!(app.hover_fades.keys().all(|k| app.hover_value(k) == 0.0));
    }

    fn client(address: &str, class: &str, workspace: &str) -> Client {
        Client {
            address: address.into(),
            class: class.into(),
            title: String::new(),
            workspace: workspace.into(),
        }
    }

    fn shape(e: &[Entry<'_>]) -> Vec<String> {
        e.iter()
            .map(|e| match e {
                Entry::Window(c) => c.address.clone(),
                Entry::App(k, l) => format!("{k}×{}", l.len()),
                Entry::Label(w) => format!("[{w}]"),
            })
            .collect()
    }

    #[test]
    fn regroupements() {
        let all = vec![
            client("a", "kitty", "2"),
            client("b", "brave", "1"),
            client("c", "kitty", "10"),
            client("d", "kitty", "2"),
        ];
        let wins = || windows(&all, RunningApps::All, "2");
        assert_eq!(
            shape(&entries(wins(), LauncherGroup::None)),
            ["a", "b", "c", "d"]
        );
        assert_eq!(
            shape(&entries(wins(), LauncherGroup::App)),
            ["kitty×3", "b"]
        );
        assert_eq!(
            shape(&entries(wins(), LauncherGroup::Workspace)),
            ["[1]", "b", "[2]", "a", "d", "[10]", "c"]
        );
        // Le workspace affiché seulement.
        let here = windows(&all, RunningApps::Workspace, "2");
        assert_eq!(shape(&entries(here, LauncherGroup::App)), ["kitty×2"]);
        assert!(windows(&all, RunningApps::Off, "2").is_empty());
    }
}
