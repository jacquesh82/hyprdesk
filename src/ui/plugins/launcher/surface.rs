//! Dock du Lanceur : rangée de fenêtres et d'applications épinglées, posée
//! sur chaque écran ou dans la barre, et la carte des fenêtres d'une
//! application. Les réglages sont dans le fichier parent.

use crate::i18n::text;
use iced::widget::{Space, button, column, container, mouse_area, row};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Size, Task,
};

use super::{lifecycle, motion};
use crate::config::{LauncherGroup, LauncherPosition, RunningApps};
use crate::hypr::{Client, Monitor};
use crate::ui::{App, IconImg, Msg as UiMsg, color, design};

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
    /// Pastille d'état propre au bouton (loupe : état de l'index), rendue
    /// même pour un bouton de commande.
    pub status: Option<crate::ui::plugins::index::IndexInk>,
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
        // Fenêtres réduites comprises, d'où qu'elles viennent : le dock est
        // le seul moyen de les rappeler.
        RunningApps::Workspace => clients
            .iter()
            .filter(|c| c.workspace == workspace || c.workspace == crate::hypr::MINIMIZED)
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
    pub(in crate::ui) shown: bool,
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
    /// Le même plugin est hébergé par la barre Graphite de cet écran.
    pub(in crate::ui) fn launcher_in_bar(&self, monitor: &str) -> bool {
        self.cfg.plugin_on("bar")
            && self.cfg.plugin_on("launcher")
            && self
                .cfg
                .bar
                .as_ref()
                .is_some_and(|b| b.style == crate::config::BarStyle::Graphite)
            && self.bar_surfaces.values().any(|s| s.monitor == monitor)
    }

    pub(in crate::ui) fn launcher_bar_position(&self, monitor: &str) -> LauncherPosition {
        self.bar_surfaces
            .values()
            .find(|s| s.monitor == monitor)
            .filter(|_| self.launcher_in_bar(monitor))
            .map_or(self.cfg.general.launcher.position, |s| match s.position {
                crate::config::BarPosition::Top => LauncherPosition::Top,
                crate::config::BarPosition::Bottom => LauncherPosition::Bottom,
            })
    }

    pub(in crate::ui) fn launcher_screen_anchor(&self, m: &Monitor, local: Rectangle) -> Rectangle {
        if self.launcher_in_bar(&m.name)
            && let Some(bar) = self.bar_surfaces.values().find(|s| s.monitor == m.name)
        {
            let y = match bar.position {
                crate::config::BarPosition::Top => bar.margin as f32,
                crate::config::BarPosition::Bottom => {
                    m.height as f32 - bar.height as f32 - bar.margin as f32
                }
            };
            return local + iced::Vector::new(0.0, y);
        }
        let l = &self.cfg.general.launcher;
        lifecycle::screen_anchor(m, l.position, l.size, local)
    }

    pub(in crate::ui) fn launcher_popup_area(&self, m: &Monitor) -> Rectangle {
        if self.launcher_in_bar(&m.name) {
            let (left, top, right, bottom) = m.reserved;
            Rectangle::new(
                Point::new(left, top),
                Size::new(
                    (m.width as f32 - left - right).max(1.0),
                    (m.height as f32 - top - bottom).max(1.0),
                ),
            )
        } else {
            let l = &self.cfg.general.launcher;
            lifecycle::workarea(m, l.position, l.size)
        }
    }

    /// Réemploie les transitions interruptibles du bureau, sans minuterie au repos.
    pub(in crate::ui) fn launcher_pointer(
        &mut self,
        monitor: &str,
        position: Option<f32>,
    ) -> Task<UiMsg> {
        if self.convex_options().is_some() {
            return Task::none();
        }
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
                crate::ui::Fade {
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
                .filter(|(id, mon)| self.raised.contains(id) && !self.launcher_in_bar(mon))
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
                    exclusive_zone: Some(if self.convex_auto_hide() {
                        0
                    } else {
                        th as i32
                    }),
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
        for (id, s) in &self.launcher_surfaces {
            tasks.push(Task::done(UiMsg::ExclusiveZoneChange {
                id: *id,
                zone_size: if self.convex_auto_hide() {
                    0
                } else {
                    s.thickness as i32
                },
            }));
            tasks.push(self.convex_dock_input(*id));
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
        self.view_launcher_content(m, t, false)
    }

    pub(in crate::ui) fn view_launcher_bar<'a>(
        &'a self,
        m: &Monitor,
        t: f32,
    ) -> Element<'a, UiMsg> {
        if !self.cfg.plugin_on("launcher") {
            return Space::new().into();
        }
        self.view_launcher_content(m, t, true)
    }

    fn view_launcher_content<'a>(
        &'a self,
        m: &Monitor,
        t: f32,
        embedded: bool,
    ) -> Element<'a, UiMsg> {
        let progress = if embedded {
            1.0
        } else {
            self.launcher_progress(&m.name)
        };
        let local_motion = !embedded && self.convex_options().is_some();
        let t = if self.convex_auto_hide() && !embedded {
            t
        } else {
            t * progress
        };
        let g = &self.cfg.general;
        let l = &g.launcher;
        let vertical = !embedded && l.position.vertical();
        let fg = color(
            if embedded {
                self.bar_palette().fg
            } else {
                g.theme.fg
            },
            t,
        );
        let dim = color(g.theme.fg, 0.55 * t);
        let size = if embedded {
            self.cfg
                .bar
                .as_ref()
                .map_or(28.0, |b| (b.height as f32 - 20.0).clamp(16.0, 32.0))
        } else {
            l.size as f32
        };
        let accent = color(
            if embedded {
                self.bar_palette().accent
            } else {
                g.theme.accent
            },
            t,
        );
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
                Glyph::Symbol(name, scale, fallback) => crate::ui::icons::get(name)
                    .map(|h| {
                        iced::widget::svg(h)
                            .width(size * scale)
                            .height(size * scale)
                            .style({
                                let ink = if *name == "hyprdesk-start" {
                                    None
                                } else {
                                    Some(fg)
                                };
                                move |_, _| iced::widget::svg::Style { color: ink }
                            })
                            .into()
                    })
                    .unwrap_or_else(|| text(*fallback).font(self.font).size(13).color(fg).into()),
                Glyph::Settings if embedded => iced::widget::svg(
                    crate::ui::icons::get("preferences-system").expect("settings icon"),
                )
                .width(size * 0.85)
                .height(size * 0.85)
                .style(move |_, _| iced::widget::svg::Style { color: Some(fg) })
                .into(),
                Glyph::Settings => self
                    .img_element(
                        self.ui_imgs
                            .get(crate::ui::ICON_SETTINGS)
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
                status,
            } = entry;
            // Loupe : la pastille porte l'état de l'index, pas un « ouvert ».
            let pret = status == Some(crate::ui::plugins::index::IndexInk::Ready);
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
            let motion = if embedded || g.transition == crate::config::Transition::None {
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
            let face = if local_motion {
                use crate::ui::{kinetic, plugins::bar::style::convexe::Edge};
                let edge = match l.position {
                    LauncherPosition::Bottom => Edge::Bottom,
                    LauncherPosition::Top => Edge::Top,
                    LauncherPosition::Left => Edge::Left,
                    LauncherPosition::Right => Edge::Right,
                };
                kinetic::magnet(
                    container(face).center(size),
                    edge,
                    accent,
                    self.bar_duration(135),
                )
            } else {
                motion::face(
                    container(face).center(size),
                    1.0 + 0.09 * motion,
                    lift,
                    self.motion_duration(),
                )
            };
            let icon: Element<'a, UiMsg> = container(face)
                .style(move |_| container::Style {
                    background: Some(
                        Color {
                            a: if embedded { 0.0 } else { 0.07 * over * t },
                            ..accent
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(if embedded { 4 } else { 12 }),
                    ..Default::default()
                })
                .into();
            let command = embedded && key.starts_with("dock-btn:");
            // La piste est réservée même pour une application fermée.
            let mark = container(Space::new())
                .width(if vertical {
                    4.0
                } else if active || launching.is_some() || pret {
                    if embedded { 16.0 } else { 12.0 }
                } else {
                    if embedded { 6.0 } else { 4.0 }
                })
                .height(if vertical && (active || launching.is_some() || pret) {
                    12.0
                } else {
                    if embedded { 3.0 } else { 4.0 }
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
                    } else if let Some(encre) = status {
                        // état du démon d'index : accent quand prêt, ambre en scan
                        Some(Background::Color(match encre {
                            crate::ui::plugins::index::IndexInk::Ready => accent,
                            crate::ui::plugins::index::IndexInk::Scan => {
                                Color::from_rgb8(0xff, 0xb8, 0x6c)
                            }
                        }))
                    } else {
                        (badge > 0 && !command).then_some(Background::Color(if active {
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
                container(mark)
                    .width(size)
                    .height(if embedded { 3 } else { 4 })
                    .center_x(size)
                    .into()
            };
            let position = if embedded {
                self.launcher_bar_position(&m.name)
            } else {
                l.position
            };
            let body: Element<'a, UiMsg> = match position {
                LauncherPosition::Bottom => column![icon, indicator].spacing(4).into(),
                LauncherPosition::Top => column![indicator, icon].spacing(4).into(),
                LauncherPosition::Left => row![indicator, icon].spacing(4).into(),
                LauncherPosition::Right => row![icon, indicator].spacing(4).into(),
            };
            let body: Element<'a, UiMsg> = if embedded {
                container(body).center(size + 8.0).into()
            } else {
                body
            };
            let pointer_monitor = monitor.clone();
            let area = mouse_area(
                button(body)
                    .padding(if embedded { 4 } else { 6 })
                    .width(if embedded {
                        Length::Fixed(size + 16.0)
                    } else {
                        Length::Shrink
                    })
                    .height(if embedded {
                        Length::Fixed(size + 16.0)
                    } else {
                        Length::Shrink
                    })
                    .on_press(UiMsg::LauncherActivate(Box::new(msg)))
                    .style(move |_, status| {
                        let alpha = match status {
                            button::Status::Pressed => 0.15,
                            button::Status::Hovered => 0.025 + 0.035 * over,
                            _ if ring || (embedded && active) => 0.07,
                            _ => 0.025 * over,
                        };
                        button::Style {
                            background: Some(Color { a: alpha * t, ..fg }.into()),
                            border: if ring {
                                Border::default()
                                    .rounded(if embedded { 4 } else { 12 })
                                    .width(2)
                                    .color(fg)
                            } else {
                                Border::default().rounded(if embedded { 4 } else { 12 })
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
            .on_exit(UiMsg::LauncherHover(None));
            let area = if local_motion {
                area
            } else {
                area.on_move(move |p| {
                    let along = if vertical { p.y } else { p.x };
                    UiMsg::LauncherPointer(
                        pointer_monitor.clone(),
                        Some(index as f32 + (along - (size + 12.0) / 2.0) / (size + 16.0)),
                    )
                })
            };
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
        if embedded {
            return iced::widget::scrollable(inner)
                .id(format!("bar-launcher:{}", m.name))
                .direction(iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(0)
                        .scroller_width(0),
                ))
                .width(Length::Shrink)
                .height(size + 16.0)
                .into();
        }
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
        let convex = self
            .convex_options()
            .filter(|o| o.attached_panels)
            .filter(|_| self.bar_surfaces.values().any(|b| b.monitor == m.name));
        let dock: Element<'_, UiMsg> = if convex.is_some() {
            use crate::ui::plugins::bar::style::convexe::{self, Edge};
            let edge = match l.position {
                LauncherPosition::Top => Edge::Top,
                LauncherPosition::Bottom => Edge::Bottom,
                LauncherPosition::Left => Edge::Left,
                LauncherPosition::Right => Edge::Right,
            };
            let padding = if l.position.vertical() {
                Padding {
                    top: 36.0,
                    bottom: 36.0,
                    left: 10.0,
                    right: 10.0,
                }
            } else {
                Padding {
                    top: 10.0,
                    bottom: 10.0,
                    left: 36.0,
                    right: 36.0,
                }
            };
            let pal = self.bar_palette();
            if self.convex_auto_hide() {
                use crate::ui::kinetic::{self, Track};
                let spring = if self.search.is_some() {
                    crate::ui::kinetic::Spring::rest(0.0)
                } else {
                    self.convex_motion(&m.name, crate::ui::convex::Part::Dock)
                };
                kinetic::reveal(
                    container(inner).padding(padding),
                    Track::Spring(spring),
                    edge,
                    color(pal.bg, pal.opacity),
                    Some(UiMsg::MotionSettled),
                )
            } else {
                iced::widget::stack![container(inner).padding(padding)]
                    .push_under(convexe::drawer(&pal, edge, progress))
                    .into()
            }
        } else {
            container(inner)
                .padding(10)
                .style(move |_| design::launcher_surface(&self.theme, t, false, base))
                .into()
        };
        let dock = mouse_area(dock).on_exit(UiMsg::LauncherPointer(monitor.clone(), None));
        let dock: Element<'_, UiMsg> = if self.convex_auto_hide() {
            let enter = m.name.clone();
            let leave = m.name.clone();
            mouse_area(dock)
                .on_enter(UiMsg::ConvexHover(
                    enter,
                    crate::ui::convex::Part::Dock,
                    true,
                ))
                .on_exit(UiMsg::ConvexHover(
                    leave,
                    crate::ui::convex::Part::Dock,
                    false,
                ))
                .into()
        } else {
            dock.into()
        };
        let dock = motion::surface(
            dock,
            1.0,
            lifecycle::edge_offset(
                l.position,
                if self.convex_auto_hide() {
                    0.0
                } else {
                    (1.0 - progress) * 8.0
                },
            ),
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
        let mut padding = Padding::new(8.0);
        if convex.is_some() {
            match l.position {
                LauncherPosition::Top => padding.top = 0.0,
                LauncherPosition::Bottom => padding.bottom = 0.0,
                LauncherPosition::Left => padding.left = 0.0,
                LauncherPosition::Right => padding.right = 0.0,
            }
        }
        let view: Element<'_, UiMsg> = container(dock)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(ax)
            .align_y(ay)
            .padding(padding)
            .into();
        if self.convex_auto_hide() {
            let width = (self.dock_slots(m).len() as f32 * (l.size as f32 + 16.0) + 100.0)
                .min(m.width as f32);
            let enter = m.name.clone();
            let leave = m.name.clone();
            let wake = mouse_area(Space::new().width(width).height(6))
                .on_enter(UiMsg::ConvexWake(enter, true))
                .on_exit(UiMsg::ConvexWake(leave, false));
            iced::widget::stack![
                view,
                container(wake)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::End)
            ]
            .into()
        } else {
            view
        }
    }

    /// Cases du dock d'un écran, dans l'ordre affiché : applications
    /// épinglées, fenêtres, séparateur, puis boutons de commande. Une seule
    /// source pour le rendu et pour la navigation au clavier.
    pub(in crate::ui) fn dock_slots(&self, m: &Monitor) -> Vec<Slot> {
        let embedded = self.launcher_in_bar(&m.name);
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
        let room = if embedded {
            usize::MAX
        } else {
            ((extent - 36.0 - command_width - if commands > 0 { 13.0 } else { -4.0 })
                / (size + 16.0))
                .max(0.0) as usize
        };
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
                status: None,
            })));
        }
        let unpinned = wins
            .into_iter()
            .filter(|c| !l.pinned.iter().any(|id| self.app_matches(id, &c.class)))
            .collect();
        for e in entries(
            unpinned,
            if embedded {
                LauncherGroup::App
            } else {
                g.launcher_group
            },
        )
        .into_iter()
        {
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
                    status: None,
                })),
                Entry::App(class, list) => Slot::Item(Box::new(DockItem {
                    key: format!("dock-app:{class}"),
                    glyph: Glyph::App(class.clone()),
                    badge: list.len(),
                    active: list.iter().any(|c| focused(c)),
                    popup: PopupContent::Windows(class.clone()),
                    msg: UiMsg::LauncherApp(m.name.clone(), class),
                    status: None,
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
            // Ouvert sur l'écran de ce dock.
            msg: UiMsg::On(
                m.name.clone(),
                Box::new(UiMsg::Control(crate::control::Command::Apps)),
            ),
            status: None,
        })));
        if assistant {
            slots.push(Slot::Item(Box::new(DockItem {
                key: "dock-btn:assistant".into(),
                glyph: Glyph::Symbol("ai-assistant", 0.6, "IA"),
                badge: usize::from(assistant_open),
                active: assistant_open,
                popup: PopupContent::Label("Assistant IA (LYRA)".into()),
                msg: UiMsg::On(m.name.clone(), Box::new(UiMsg::OpenAssistant)),
                status: None,
            })));
        }
        if l.show_settings {
            slots.push(Slot::Item(Box::new(DockItem {
                key: "dock-btn:settings".into(),
                glyph: Glyph::Settings,
                badge: usize::from(settings_open),
                active: settings_open,
                popup: PopupContent::Label(crate::i18n::tr("Réglages").into()),
                msg: UiMsg::On(m.name.clone(), Box::new(UiMsg::OpenPanel)),
                status: None,
            })));
        }
        if embedded {
            slots.retain(|s| matches!(s, Slot::Item(_)));
            if let Some(i) = slots
                .iter()
                .position(|s| matches!(s, Slot::Item(item) if item.key == "dock-btn:apps"))
            {
                let mut start = slots.remove(i);
                if let Slot::Item(item) = &mut start {
                    item.glyph = Glyph::Symbol("hyprdesk-start", 0.85, "h");
                }
                slots.insert(0, start);
            }
            // La loupe ouvre la recherche de fichiers ; sa pastille dit
            // l'état de l'index (accent : prêt, ambre : scan, rien : arrêté).
            let (etiquette, encre) = match self.index_status.as_ref() {
                Some(st) if st.phase == "scan" => (
                    crate::tr!("Recherche de fichiers — indexation en cours…").to_string(),
                    Some(crate::ui::plugins::index::IndexInk::Scan),
                ),
                Some(st) if st.phase == "chargement" => (
                    crate::tr!("Recherche de fichiers — chargement de l'index…").to_string(),
                    Some(crate::ui::plugins::index::IndexInk::Scan),
                ),
                Some(st) => (
                    crate::tr!(
                        "Recherche de fichiers — index prêt ({n} fichiers)",
                        n = st.files
                    )
                    .to_string(),
                    Some(crate::ui::plugins::index::IndexInk::Ready),
                ),
                None => (
                    crate::tr!("Recherche de fichiers — index arrêté (démarre au premier « :f »)")
                        .to_string(),
                    None,
                ),
            };
            slots.insert(
                1,
                Slot::Item(Box::new(DockItem {
                    key: "dock-btn:search".into(),
                    glyph: Glyph::Symbol("system-search", 0.85, "…"),
                    badge: 0,
                    active: false,
                    popup: PopupContent::Label(etiquette),
                    msg: UiMsg::On(
                        m.name.clone(),
                        Box::new(UiMsg::Control(crate::control::Command::Files)),
                    ),
                    status: encre,
                })),
            );
            slots.insert(
                2,
                Slot::Item(Box::new(DockItem {
                    key: "dock-btn:tasks".into(),
                    glyph: Glyph::Symbol("hyprdesk-task-view", 0.85, "…"),
                    badge: 0,
                    active: false,
                    popup: PopupContent::Label(crate::i18n::tr("Vue des tâches").into()),
                    msg: UiMsg::On(
                        m.name.clone(),
                        Box::new(UiMsg::Control(crate::control::Command::Overview)),
                    ),
                    status: None,
                })),
            );
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
        let delay = if self.convex_options().is_some() {
            std::time::Duration::from_millis(400)
        } else {
            lifecycle::OPEN_DELAY
        };
        let open = Task::perform(async move { tokio::time::sleep(delay).await }, move |_| {
            UiMsg::LauncherPopupOpen(token)
        });
        Task::batch([close, open])
    }

    pub(in crate::ui) fn launcher_popup_open(&mut self, token: u64) -> Task<UiMsg> {
        use iced_layershell::reexport::{
            Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
        };
        if (!self.visible
            && !self
                .launcher_runtime
                .pending
                .as_ref()
                .is_some_and(|p| self.launcher_in_bar(&p.monitor)))
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
        let anchor = self.launcher_screen_anchor(m, local);
        let position = self.launcher_bar_position(&m.name);
        let lines = match &pending.content {
            PopupContent::Windows(class) => self.popup_windows(&pending.monitor, class).len(),
            PopupContent::Label(_) => 1,
        };
        let (w, h) = popup_size(&pending.content, lines);
        let max_height = (m.height as f32 - m.reserved.1 - m.reserved.3 - 16.0).max(36.0);
        let placement = lifecycle::place_popup(
            position,
            self.launcher_popup_area(m),
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
            shown: cfg!(test),
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
            return if self.convex_options().is_some() {
                Task::none()
            } else {
                Task::done(UiMsg::Frame)
            };
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

    pub(in crate::ui) fn launcher_popup_shown(&mut self, id: iced::window::Id) -> Task<UiMsg> {
        if let Some(p) = self
            .launcher_popup
            .as_mut()
            .filter(|p| p.id == id && !p.shown)
        {
            p.shown = true;
            if p.fade.to == 1.0 {
                p.fade.from = 0.0;
                p.fade.start = std::time::Instant::now();
            }
        }
        Task::none()
    }

    /// La bulle : un nom, ou la liste des fenêtres d'une application.
    pub(in crate::ui) fn view_launcher_popup(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.launcher_popup else {
            return iced::widget::Space::new().into();
        };
        let g = &self.cfg.general;
        let local = self.convex_options().is_some();
        let t = if local {
            1.0
        } else if g.transition == crate::config::Transition::None {
            p.fade.to
        } else {
            p.fade.value(std::time::Instant::now())
        };
        let fg = color(
            if local {
                self.bar_palette().fg
            } else {
                g.theme.fg
            },
            t,
        );
        let dim = color(
            if local {
                self.bar_palette().fg
            } else {
                g.theme.fg
            },
            0.55 * t,
        );
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
                                .size(12)
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
        let pal = self.bar_palette();
        let card = mouse_area(
            container(body)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8)
                .center_y(Length::Fill)
                .style(move |_| {
                    if local {
                        design::bar_card(color(pal.bg, pal.opacity), color(pal.fg, 1.0))
                    } else {
                        design::launcher_surface(&self.theme, t, true, base)
                    }
                }),
        )
        .on_enter(UiMsg::LauncherPopupHover(true))
        .on_exit(UiMsg::LauncherPopupHover(false));
        if local {
            use crate::ui::{
                kinetic::{self, Track},
                plugins::bar::style::convexe::Edge,
            };
            let edge = match self.launcher_bar_position(&p.monitor) {
                LauncherPosition::Top => Edge::Top,
                LauncherPosition::Bottom => Edge::Bottom,
                LauncherPosition::Left => Edge::Left,
                LauncherPosition::Right => Edge::Right,
            };
            let fade = if p.shown {
                p.fade
            } else {
                crate::ui::Fade {
                    from: 0.0,
                    to: 0.0,
                    start: p.fade.start,
                    dur: std::time::Duration::ZERO,
                }
            };
            return iced::widget::sensor(kinetic::floating(
                container(card)
                    .padding(8)
                    .width(Length::Fill)
                    .height(Length::Fill),
                Track::Ease(fade),
                edge,
                p.placement.pivot,
                p.shown.then_some(UiMsg::MotionSettled),
            ))
            .key(p.id)
            .on_show(move |_| UiMsg::LauncherPopupShown(popup_id))
            .into();
        }
        let card = motion::surface(
            card,
            0.97 + 0.03 * t,
            lifecycle::edge_offset(self.launcher_bar_position(&p.monitor), (1.0 - t) * 6.0),
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
            monitor: "A".into(),
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
