//! Paquet intégré « Lanceur » : le dock en bas du bureau — widget autonome,
//! seul contrôle du mode `launcher-only` — et sa page de réglages.
//!
//! Le lanceur affiche les fenêtres ouvertes (aucune, celles du workspace
//! affiché, ou toutes), regroupées au choix : une entrée par fenêtre, une par
//! application (classe de fenêtre, donc par binaire), ou par workspace. Le
//! bouton ⚙ termine la rangée. L'utiliser ne change jamais le mode du bureau.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

mod motion;

use iced::widget::{Space, button, column, container, mouse_area, radio, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Task};

use super::super::panel::PanelMsg;
use super::super::{App, IconImg, Msg as UiMsg, color, design};
use super::{PluginInfo, PluginMsg};
use crate::config::{LauncherGroup, LauncherPosition, RunningApps};
use crate::hypr::{Client, Monitor};

pub const INFO: PluginInfo = PluginInfo {
    id: "launcher",
    label: "Lanceur",
    icon: "view-app-grid",
    summary: "Dock du bureau : fenêtres affichées, regroupement, bouton des réglages.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
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
}

/// État d'écran : recherche d'une application à épingler.
#[derive(Default)]
pub struct State {
    pub pin_filter: String,
}

/// Une entrée de la rangée du lanceur.
enum Entry<'a> {
    Window(&'a Client),
    /// Toutes les fenêtres d'une application (classe).
    App(String, Vec<&'a Client>),
    /// Titre d'un groupe : le nom du workspace.
    Label(String),
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
}

/// Épaisseur de la surface du lanceur : boutons, carte, points, marge.
pub(in crate::ui) fn thickness(size: u16) -> u32 {
    size as u32 + 12 + 20 + 8 + 16
}

/// Taille de la bulle selon son contenu.
fn popup_size(content: &PopupContent, lines: usize) -> (u32, u32) {
    match content {
        PopupContent::Label(t) => ((t.chars().count() as u32 * 8 + 32).clamp(80, 440), 36),
        PopupContent::Windows(_) => (400, lines.max(1) as u32 * 36 + 24),
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
                tasks.push(iced::window::close(id));
                changed = true;
            }
        }
        for mon in want {
            if self.launcher_surfaces.values().any(|s| s.monitor == mon) {
                continue;
            }
            let id = iced::window::Id::unique();
            let (pos, th) = spec;
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
                    keyboard_interactivity: KeyboardInteractivity::None,
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
        let monitor = m.name.clone();
        // Bouton d'une entrée, grossi au survol ; le survol ouvre la bulle.
        let item = |index: usize,
                    key: String,
                    face: Element<'a, UiMsg>,
                    badge: usize,
                    active: bool,
                    popup: PopupContent,
                    msg: UiMsg|
         -> Element<'a, UiMsg> {
            let over = self.hover_value(&format!("dock-wave:{monitor}:{index}"));
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
                } else if active {
                    12.0
                } else {
                    4.0
                })
                .height(if vertical && active { 12.0 } else { 4.0 })
                .style(move |_| container::Style {
                    background: (badge > 0).then_some(Background::Color(if active {
                        accent
                    } else {
                        Color { a: 0.5 * t, ..fg }
                    })),
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
            mouse_area(
                button(body)
                    .padding(6)
                    .on_press(msg)
                    .style(move |_, status| {
                        let alpha = match status {
                            button::Status::Pressed => 0.15,
                            button::Status::Hovered => 0.025 + 0.035 * over,
                            _ => 0.025 * over,
                        };
                        button::Style {
                            background: Some(Color { a: alpha * t, ..fg }.into()),
                            border: Border::default().rounded(12),
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
            })
            .into()
        };
        let extent = if vertical { m.height } else { m.width } as f32;
        let room = ((extent - 110.0) / (size + 16.0)).max(1.0) as usize;
        let wins = windows(&self.clients, g.running_apps, &m.workspace);
        let focused = |c: &Client| {
            m.focused
                && self
                    .active_window
                    .as_ref()
                    .is_some_and(|(class, title)| class == &c.class && title == &c.title)
        };
        let mut items: Vec<Element<'a, UiMsg>> = Vec::new();
        // Les épingles gardent leur rang, ouvertes ou fermées.
        for id in &l.pinned {
            if items.len() >= room {
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
            let (popup, action) = match open.as_slice() {
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
            items.push(item(
                items.len(),
                format!("dock-pin:{id}"),
                face(id, size),
                open.len(),
                open.iter().any(|c| focused(c)),
                popup,
                action,
            ));
        }
        let unpinned = wins
            .into_iter()
            .filter(|c| !l.pinned.iter().any(|id| self.app_matches(id, &c.class)))
            .collect();
        for e in entries(unpinned, g.launcher_group).into_iter() {
            if items.len() >= room {
                break;
            }
            items.push(match e {
                Entry::Window(c) => item(
                    items.len(),
                    format!("dock:{}", c.address),
                    face(&c.class, size),
                    1,
                    focused(c),
                    PopupContent::Label(if c.title.is_empty() {
                        c.class.clone()
                    } else {
                        c.title.clone()
                    }),
                    UiMsg::FocusWindow(c.address.clone()),
                ),
                Entry::App(class, l) => item(
                    items.len(),
                    format!("dock-app:{class}"),
                    face(&class, size),
                    l.len(),
                    l.iter().any(|c| focused(c)),
                    PopupContent::Windows(class.clone()),
                    UiMsg::LauncherApp(m.name.clone(), class),
                ),
                Entry::Label(ws) => container(text(ws).font(self.title_font()).size(12).color(dim))
                    .padding(Padding {
                        left: 6.0,
                        right: 2.0,
                        ..Padding::ZERO
                    })
                    .into(),
            });
        }
        let assistant = l.show_assistant
            && self.cfg.plugin_on("assistant")
            && crate::assistant::lyra_installed();
        if !items.is_empty() && (assistant || l.show_settings) {
            let sep = Color { a: 0.12 * t, ..fg };
            let line = if vertical {
                iced::widget::Space::new().width(size * 0.6).height(1)
            } else {
                iced::widget::Space::new().width(1).height(size * 0.6)
            };
            items.push(
                container(container(line).style(move |_| container::Style {
                    background: Some(sep.into()),
                    ..Default::default()
                }))
                .padding(if vertical { [4, 0] } else { [0, 4] })
                .into(),
            );
        }
        if assistant {
            let glyph: Element<'a, UiMsg> = super::super::icons::get("ai-assistant")
                .map(|h| {
                    iced::widget::svg(h)
                        .width(size * 0.6)
                        .height(size * 0.6)
                        .style(move |_, _| iced::widget::svg::Style { color: Some(fg) })
                        .into()
                })
                .unwrap_or_else(|| text("IA").font(self.font).size(13).color(fg).into());
            items.push(item(
                items.len(),
                "dock-btn:assistant".into(),
                glyph,
                0,
                false,
                PopupContent::Label("Assistant IA (LYRA)".into()),
                UiMsg::OpenAssistant,
            ));
        }
        if l.show_settings {
            let gear: Element<'a, UiMsg> = self
                .img_element(
                    self.ui_imgs
                        .get(super::super::ICON_SETTINGS)
                        .copied()
                        .unwrap_or(IconImg::None),
                    size * 0.6,
                    t,
                )
                .unwrap_or_else(|| text("Réglages").font(self.font).size(13).color(fg).into());
            items.push(item(
                items.len(),
                "dock-btn:settings".into(),
                gear,
                0,
                false,
                PopupContent::Label("Réglages".into()),
                UiMsg::OpenPanel,
            ));
        }
        if items.is_empty() {
            return iced::widget::Space::new().into();
        }
        let inner: Element<'a, UiMsg> = if vertical {
            column(items).spacing(4).align_x(Alignment::Center).into()
        } else {
            row(items).spacing(4).align_y(Alignment::Center).into()
        };
        let dock = container(inner)
            .padding(10)
            .style(move |_| design::launcher_surface(&self.theme, t, false));
        let dock = mouse_area(dock).on_exit(UiMsg::LauncherPointer(monitor, None));
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

    /// Survol d'une entrée : la bulle s'ouvre (sa propre surface, au-dessus de
    /// tout, au niveau du curseur) ; quitter l'entrée la referme après un court
    /// délai, le temps d'atteindre la bulle.
    pub(in crate::ui) fn launcher_hover(
        &mut self,
        spec: Option<(String, String, PopupContent, usize)>,
    ) -> Task<UiMsg> {
        use iced_layershell::reexport::{
            Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
        };
        self.launcher_popup_token = self.launcher_popup_token.wrapping_add(1);
        let Some((monitor, key, content, index)) = spec else {
            return self.schedule_popup_close();
        };
        let _ = self.launcher_pointer(&monitor, Some(index as f32));
        if self
            .launcher_popup
            .as_ref()
            .is_some_and(|p| p.key == key && p.monitor == monitor)
        {
            return Task::none();
        }
        let mut tasks = vec![self.close_launcher_popup()];
        let lines = match &content {
            PopupContent::Windows(class) => self.popup_windows(&monitor, class).len(),
            PopupContent::Label(_) => 1,
        };
        let (w, h) = popup_size(&content, lines);
        let l = &self.cfg.general.launcher;
        let Some((sid, surf)) = self
            .launcher_surfaces
            .iter()
            .find(|(_, s)| s.monitor == monitor)
            .map(|(id, s)| (*id, s.clone()))
        else {
            return Task::batch(tasks);
        };
        let m = self.monitors.iter().find(|m| m.name == monitor).cloned();
        let (mw, mh) = m.map_or((1920.0, 1080.0), |m| (m.width as f32, m.height as f32));
        let cursor = self.cursor.get(&sid).copied().unwrap_or_default();
        // Centrée sur le curseur le long du bord, collée au lanceur.
        let along = |c: f32, len: f32, span: u32| {
            (c - span as f32 / 2.0).clamp(8.0, (len - span as f32 - 8.0).max(8.0)) as i32
        };
        let gap = 6;
        let (anchor, margin) = match l.position {
            LauncherPosition::Bottom => (
                Anchor::Bottom | Anchor::Left,
                (0, 0, gap, along(cursor.x, mw, w)),
            ),
            LauncherPosition::Top => (
                Anchor::Top | Anchor::Left,
                (gap, 0, 0, along(cursor.x, mw, w)),
            ),
            LauncherPosition::Left => (
                Anchor::Left | Anchor::Top,
                (along(cursor.y, mh, h), 0, 0, gap),
            ),
            LauncherPosition::Right => (
                Anchor::Right | Anchor::Top,
                (along(cursor.y, mh, h), gap, 0, 0),
            ),
        };
        let _ = surf;
        let id = iced::window::Id::unique();
        self.launcher_popup = Some(Popup {
            id,
            monitor: monitor.clone(),
            key,
            content,
        });
        tasks.push(Task::done(UiMsg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((w, h)),
                layer: Layer::Overlay,
                anchor,
                exclusive_zone: Some(0),
                margin: Some(margin),
                keyboard_interactivity: KeyboardInteractivity::None,
                output_option: OutputOption::OutputName(monitor),
                namespace: Some(POPUP_NAMESPACE.into()),
                ..Default::default()
            },
            id,
        }));
        Task::batch(tasks)
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
        self.close_launcher_popup()
    }

    pub(in crate::ui) fn launcher_popup_hover(&mut self, on: bool) -> Task<UiMsg> {
        self.launcher_popup_hover = on;
        if on {
            self.launcher_popup_token = self.launcher_popup_token.wrapping_add(1);
            Task::none()
        } else {
            self.schedule_popup_close()
        }
    }

    pub(in crate::ui) fn close_launcher_popup(&mut self) -> Task<UiMsg> {
        self.launcher_popup_hover = false;
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
        let fg = color(g.theme.fg, 1.0);
        let dim = color(g.theme.fg, 0.55);
        let accent = color(g.theme.accent, 1.0);
        let hover = move |_: &iced::Theme, s: button::Status| button::Style {
            background: matches!(s, button::Status::Hovered)
                .then(|| Background::Color(Color { a: 0.15, ..fg })),
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
                        .img_element(img, 18.0, 1.0)
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
                                    a: if matches!(status, button::Status::Hovered) {
                                        0.18
                                    } else {
                                        0.10
                                    },
                                    ..accent
                                }
                                .into(),
                            );
                            style.border = style.border.width(1).color(Color { a: 0.18, ..accent });
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
        mouse_area(
            container(body)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8)
                .center_y(Length::Fill)
                .style(move |_| design::launcher_surface(&self.theme, 1.0, true)),
        )
        .on_enter(UiMsg::LauncherPopupHover(true))
        .on_exit(UiMsg::LauncherPopupHover(false))
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
        let def = crate::config::IconDef {
            origin: Default::default(),
            sel: Default::default(),
            id: id.to_string(),
            label: None,
            kind: crate::config::IconKind::App(id.to_string()),
            icon: None,
            pos: None,
        };
        let r = crate::launch::resolve(&def, None, 16, &self.cfg.general.terminal);
        if r.command.is_empty() {
            return Task::none();
        }
        self.run_from_desktop(r.command, true)
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
            radio(label, v, Some(g.running_apps), |v| wrap(Msg::Running(v))).font(self.font)
        };
        let group = |label: &'static str, v: LauncherGroup| {
            radio(label, v, Some(g.launcher_group), |v| wrap(Msg::Group(v))).font(self.font)
        };
        column![
            self.section("Fenêtres"),
            self.field(
                "Fenêtres ouvertes dans le lanceur",
                "running_apps",
                column![
                    running("Aucune (épingles et boutons uniquement)", RunningApps::Off),
                    running("Celles du workspace affiché", RunningApps::Workspace),
                    running("Toutes", RunningApps::All),
                ]
                .spacing(8),
            ),
            self.section("Regroupement"),
            self.field(
                "Regrouper les fenêtres",
                "launcher_group",
                column![
                    group("Aucun : une entrée par fenêtre", LauncherGroup::None),
                    group(
                        "Par application (binaire) : une entrée, un clic passe de l'une à l'autre",
                        LauncherGroup::App
                    ),
                    group(
                        "Par workspace : rangées sous leur numéro",
                        LauncherGroup::Workspace
                    ),
                ]
                .spacing(8),
            ),
            self.launcher_look(),
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

    /// Réglages d'apparence : bord, taille, applications épinglées, boutons.
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
        column![
            self.section("Apparence"),
            self.field(
                "Bord de l'écran",
                "launcher_position",
                row![
                    position("En bas", LauncherPosition::Bottom),
                    position("En haut", LauncherPosition::Top),
                    position("À gauche", LauncherPosition::Left),
                    position("À droite", LauncherPosition::Right),
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
                        .label("Assistant IA (LYRA)")
                        .on_toggle(|v| wrap(Msg::ShowAssistant(v))),
                    iced::widget::toggler(l.show_settings)
                        .label("Réglages")
                        .on_toggle(|v| wrap(Msg::ShowSettings(v))),
                ]
                .spacing(8),
            ),
            self.section("Applications épinglées"),
            text("Toujours dans le lanceur ; un clic les lance si elles ne sont pas ouvertes.")
                .font(self.font)
                .size(12)
                .color(dim),
            pins,
            iced::widget::text_input("Épingler une application (firefox, terminal…)", &filter)
                .font(self.font)
                .on_input(|v| wrap(Msg::PinFilter(v)))
                .padding(8),
            found,
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
