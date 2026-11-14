//! Barre native : une surface layer-shell par écran, sur le calque `top`
//! (au-dessus des fenêtres), avec une zone exclusive — Hyprland range les
//! fenêtres à côté, et le bureau décale sa grille de la zone réservée.
//!
//! Dessin soigné, à la manière d'une barre de menus macOS : fond translucide
//! flouté, filet discret, icônes symboliques teintées, capsule au survol,
//! menus déroulants (voir `popover`). Ses modules sont des widgets.

use std::time::Duration;

use crate::i18n::text;
use iced::widget::{Space, button, container, mouse_area, row, tooltip};
use iced::window::Id;
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Task, mouse};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::popover::Request;
use super::{App, IconImg, Msg, color, format_now};
use super::{bar_motion, design};
use crate::config::{BarItem, BarPosition, RenderState, Rgba, WidgetKind};
use crate::{control, hypr, widgets};

pub const BAR_NAMESPACE: &str = "hyprdesk-bar";

/// Écran de barre désignant l'écran principal (`hd.bar({ monitor = "primary" })`).
pub const PRIMARY: &str = "primary";

/// Icône du bureau affiché avec les fenêtres (dessinée par hyprdesk).
pub const DESKTOP_WINDOWS_ICON: &str = "hyprdesk-desktop-windows-symbolic";

/// Icône du bureau masqué (no-desktop), dessinée par hyprdesk.
pub const DESKTOP_OFF_ICON: &str = "hyprdesk-desktop-off-symbolic";

/// Losange « fenêtre urgente » dessiné par hyprdesk, à côté du numéro d'un
/// espace qui porte une fenêtre demandant l'attention.
pub const URGENT_ICON: &str = "hyprdesk-urgent-symbolic";

/// Icônes symboliques demandées dès que la barre existe.
pub const STATIC_ICONS: &[&str] = &[
    "user-desktop-symbolic",
    DESKTOP_WINDOWS_ICON,
    DESKTOP_OFF_ICON,
    "audio-volume-muted-symbolic",
    "audio-volume-low-symbolic",
    "audio-volume-medium-symbolic",
    "audio-volume-high-symbolic",
    "media-playback-start-symbolic",
    "media-playback-pause-symbolic",
    "object-select-symbolic",
    "audio-input-microphone-symbolic",
    "microphone-sensitivity-muted-symbolic",
    // Élément de la zone de notification sans image.
    "application-x-executable-symbolic",
    "preferences-system-controls-symbolic",
    "view-more-symbolic",
    "display-brightness-symbolic",
    URGENT_ICON,
];

/// Surface de barre : écran, position, hauteur et marge au bord à sa création.
#[derive(Debug, Clone, PartialEq)]
pub struct BarSurface {
    pub monitor: String,
    pub position: BarPosition,
    pub height: u16,
    pub margin: u16,
}

/// Couleurs effectives de la barre.
#[derive(Debug, Clone, Copy)]
pub(super) struct BarPalette {
    pub fg: Rgba,
    pub bg: Rgba,
    pub accent: Rgba,
    pub warning: Rgba,
    pub critical: Rgba,
    /// Opacité du fond.
    pub opacity: f32,
}

/// Clé d'un module propre à la barre (couleur par module, `module_colors`).
pub fn bar_item_key(item: &BarItem) -> &str {
    match item {
        BarItem::Workspaces => "workspaces",
        BarItem::Window => "window",
        BarItem::Desktop => "desktop",
        BarItem::Volume => "volume",
        BarItem::Tray => "tray",
        BarItem::Brightness => "brightness",
        BarItem::Widget(id) => id,
    }
}

pub(super) fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

/// `wpctl get-volume` → (volume 0..1, coupé).
pub fn parse_volume(out: &str) -> Option<(f32, bool)> {
    let v: f32 = out.split_whitespace().nth(1)?.parse().ok()?;
    (v.is_finite() && v >= 0.0).then_some((v, out.contains("[MUTED]")))
}

pub async fn read_volume() -> Option<(f32, bool)> {
    let o = crate::processes::output(
        tokio::process::Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]),
        Duration::from_secs(2),
    )
    .await
    .ok()?;
    parse_volume(&String::from_utf8_lossy(&o.stdout))
}

/// Périphérique audio (sortie ou entrée) selon PipeWire.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    /// Périphérique par défaut (celui que règlent le curseur et « muet »).
    pub default: bool,
}

/// Luminosité réglable : rétroéclairage (écran, clavier) et écrans DDC/CI.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Light {
    pub screen: Option<crate::brightness::Device>,
    pub kbd: Option<crate::brightness::Device>,
    pub displays: Vec<crate::brightness::Display>,
}

/// Ce que règle un curseur de luminosité.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightTarget {
    Screen,
    Kbd,
    /// Écran DDC/CI, par son bus I²C.
    Ddc(u32),
}

impl Light {
    /// Niveau affiché dans la barre d'un écran : son rétroéclairage, sinon
    /// l'écran DDC/CI de ce connecteur, sinon le premier.
    pub fn level_for(&self, monitor: &str) -> Option<f32> {
        self.screen.as_ref().map(|d| d.level()).or_else(|| {
            self.displays
                .iter()
                .find(|d| d.connector == monitor)
                .or(self.displays.first())
                .map(|d| d.level())
        })
    }

    /// Cible du défilement sur la barre d'un écran.
    pub fn target_for(&self, monitor: &str) -> Option<LightTarget> {
        if self.screen.is_some() {
            return Some(LightTarget::Screen);
        }
        self.displays
            .iter()
            .find(|d| d.connector == monitor)
            .or(self.displays.first())
            .map(|d| LightTarget::Ddc(d.bus))
    }

    pub fn level(&self, t: LightTarget) -> Option<f32> {
        match t {
            LightTarget::Screen => self.screen.as_ref().map(|d| d.level()),
            LightTarget::Kbd => self.kbd.as_ref().map(|d| d.level()),
            LightTarget::Ddc(b) => self.displays.iter().find(|d| d.bus == b).map(|d| d.level()),
        }
    }

    /// Note un niveau 0..1 (curseur) ; rend la valeur brute à appliquer.
    pub fn set_level(&mut self, t: LightTarget, v: f32) -> Option<u32> {
        let v = v.clamp(0.0, 1.0);
        let raw = |max: u32, floor: u32| ((max as f32 * v).round() as u32).clamp(floor, max);
        match t {
            LightTarget::Screen => self.screen.as_mut().map(|d| {
                d.current = raw(d.max, 1);
                d.current
            }),
            LightTarget::Kbd => self.kbd.as_mut().map(|d| {
                d.current = raw(d.max, 0);
                d.current
            }),
            LightTarget::Ddc(b) => self.displays.iter_mut().find(|d| d.bus == b).map(|d| {
                d.current = raw(d.max, 0);
                d.current
            }),
        }
    }
}

/// Relit la luminosité ; `displays` : valeurs DDC/CI connues (relues seulement si `None`).
pub async fn read_light(displays: Option<Vec<crate::brightness::Display>>) -> Light {
    let displays = match displays {
        Some(d) => d,
        None => crate::brightness::ddc_displays().await,
    };
    Light {
        screen: crate::brightness::screen(),
        kbd: crate::brightness::keyboard(),
        displays,
    }
}

/// Applique un niveau, puis relit (valeurs DDC/CI gardées : la relecture est lente).
pub async fn apply_light(mut light: Light, t: LightTarget, raw: u32) -> Light {
    let r = match t {
        LightTarget::Screen => match light.screen.clone() {
            Some(d) => crate::brightness::set(&d, raw).await,
            None => Ok(()),
        },
        LightTarget::Kbd => match light.kbd.clone() {
            Some(d) => crate::brightness::set(&d, raw).await,
            None => Ok(()),
        },
        LightTarget::Ddc(b) => crate::brightness::ddc_set(b, raw).await,
    };
    if let Err(e) = r {
        tracing::warn!("{}", crate::tr!("luminosité : {e}", e = e));
    }
    light.screen = crate::brightness::screen();
    light.kbd = crate::brightness::keyboard();
    light
}

/// Ce que le menu Son affiche en plus du volume de sortie.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Audio {
    /// Micro par défaut : (volume 0..1, coupé), `None` sans entrée.
    pub mic: Option<(f32, bool)>,
    pub outputs: Vec<AudioDevice>,
    pub inputs: Vec<AudioDevice>,
}

/// Sorties et entrées de la section « Audio » de `wpctl status`.
pub fn parse_wpctl_status(out: &str) -> (Vec<AudioDevice>, Vec<AudioDevice>) {
    let (mut outputs, mut inputs) = (Vec::new(), Vec::new());
    let (mut in_audio, mut sub) = (false, "");
    for line in out.lines() {
        if !line.starts_with(' ') && !line.trim().is_empty() {
            in_audio = line.trim() == "Audio";
            sub = "";
            continue;
        }
        if !in_audio {
            continue;
        }
        let body = line.trim_start_matches(|c: char| c.is_whitespace() || "│├└─".contains(c));
        if let Some(name) = body.strip_suffix(':')
            && (line.contains('├') || line.contains('└'))
        {
            sub = match name {
                "Sinks" => "sinks",
                "Sources" => "sources",
                _ => "",
            };
            continue;
        }
        if sub.is_empty() {
            continue;
        }
        let (default, rest) = match body.strip_prefix('*') {
            Some(r) => (true, r.trim_start()),
            None => (false, body),
        };
        let Some((id, name)) = rest.split_once(". ") else {
            continue;
        };
        let Ok(id) = id.trim().parse::<u32>() else {
            continue;
        };
        let name = name
            .split(" [vol:")
            .next()
            .unwrap_or(name)
            .trim()
            .to_string();
        let dev = AudioDevice { id, name, default };
        if sub == "sinks" {
            outputs.push(dev);
        } else {
            inputs.push(dev);
        }
    }
    (outputs, inputs)
}

async fn wpctl(args: &[&str]) -> Option<String> {
    let o = crate::processes::output(
        tokio::process::Command::new("wpctl").args(args),
        Duration::from_secs(2),
    )
    .await
    .ok()?;
    Some(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Micro et périphériques, pour le menu Son.
pub async fn read_audio() -> Audio {
    let (status, mic) = tokio::join!(
        wpctl(&["status"]),
        wpctl(&["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
    );
    let (outputs, inputs) = status
        .as_deref()
        .map(parse_wpctl_status)
        .unwrap_or_default();
    Audio {
        mic: mic.as_deref().and_then(parse_volume),
        outputs,
        inputs,
    }
}

pub fn mic_icon(v: Option<(f32, bool)>) -> &'static str {
    match v {
        None | Some((_, true)) => "microphone-sensitivity-muted-symbolic",
        _ => "audio-input-microphone-symbolic",
    }
}

pub fn volume_icon(v: Option<(f32, bool)>) -> &'static str {
    match v {
        None | Some((_, true)) => "audio-volume-muted-symbolic",
        Some((v, false)) if v < 0.34 => "audio-volume-low-symbolic",
        Some((v, false)) if v < 0.67 => "audio-volume-medium-symbolic",
        _ => "audio-volume-high-symbolic",
    }
}

pub(super) fn scroll_dir(d: mouse::ScrollDelta) -> i64 {
    let y = match d {
        mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => y,
    };
    if y > 0.0 { 1 } else { -1 }
}

impl App {
    pub(super) fn bar_duration(&self, millis: u64) -> Duration {
        if self.cfg.general.transition == crate::config::Transition::None {
            Duration::ZERO
        } else {
            Duration::from_millis(millis)
        }
    }

    pub(super) fn bar_overflow_items(&self, id: Id) -> Vec<BarItem> {
        let (Some(surface), Some(bar)) = (self.bar_surfaces.get(&id), self.cfg.bar.as_ref()) else {
            return Vec::new();
        };
        let zones = if self.bar_vertical(&surface.monitor) {
            bar.portrait.as_ref().unwrap_or(&bar.zones)
        } else {
            &bar.zones
        };
        zones
            .iter()
            .flatten()
            .enumerate()
            .filter(|(i, _)| {
                self.bar_hidden
                    .get(&id)
                    .is_some_and(|hidden| hidden.contains(i))
            })
            .map(|(_, item)| item.clone())
            .collect()
    }

    fn bar_priority(&self, item: &BarItem) -> u8 {
        match item {
            BarItem::Workspaces => 95,
            BarItem::Window => 0,
            BarItem::Volume => 60,
            BarItem::Widget(id) => match self
                .cfg
                .widgets
                .iter()
                .find(|w| &w.id == id)
                .map(|w| &w.kind)
            {
                Some(WidgetKind::Controls) => 100,
                Some(WidgetKind::Launcher) => 101,
                Some(WidgetKind::Clock { .. }) => 90,
                _ => match self.values.get(id) {
                    Some(widgets::Value::Rich(r)) if r.state == RenderState::Critical => 100,
                    Some(widgets::Value::Rich(r)) if r.state == RenderState::Warning => 85,
                    _ => 20,
                },
            },
            _ => 30,
        }
    }

    /// Crée, recrée ou ferme les surfaces de barre selon la config et les écrans.
    pub(super) fn sync_bars(&mut self) -> Task<Msg> {
        // `"primary"` dans la liste : l'écran principal, quel que soit son nom.
        let primary = self.primary_monitor();
        let want: Vec<String> = match &self.cfg.bar {
            // Paquet « Barre » coupé : aucune barre.
            _ if !self.cfg.plugin_on("bar") => Vec::new(),
            None => Vec::new(),
            Some(b) => self
                .monitors
                .iter()
                .filter(|m| {
                    b.monitors.as_ref().is_none_or(|l| {
                        l.contains(&m.name)
                            || (l.iter().any(|x| x == PRIMARY) && primary.as_ref() == Some(&m.name))
                    })
                })
                .map(|m| m.name.clone())
                .collect(),
        };
        let spec = self
            .cfg
            .bar
            .as_ref()
            .map(|b| (b.position, b.height, b.margin));
        let mut tasks = Vec::new();
        let mut changed = false;
        for (id, s) in self.bar_surfaces.clone() {
            if !(want.contains(&s.monitor) && spec == Some((s.position, s.height, s.margin))) {
                self.bar_surfaces.remove(&id);
                tasks.push(iced::window::close(id));
                changed = true;
            }
        }
        if let Some((position, height, margin)) = spec {
            for mon in want {
                if self.bar_surfaces.values().any(|s| s.monitor == mon) {
                    continue;
                }
                let id = Id::unique();
                let edge = if position == BarPosition::Top {
                    LsAnchor::Top
                } else {
                    LsAnchor::Bottom
                };
                self.bar_surfaces.insert(
                    id,
                    BarSurface {
                        monitor: mon.clone(),
                        position,
                        height,
                        margin,
                    },
                );
                tasks.push(Task::done(Msg::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some((0, height as u32)),
                        layer: Layer::Top,
                        anchor: edge | LsAnchor::Left | LsAnchor::Right,
                        // La zone réservée aux fenêtres compte la hauteur et la
                        // marge : la lame flotte au-dessus du bord.
                        exclusive_zone: Some(height as i32),
                        margin: Some(if position == BarPosition::Top {
                            (margin as i32, 0, 0, 0)
                        } else {
                            (0, 0, margin as i32, 0)
                        }),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        output_option: OutputOption::OutputName(mon),
                        namespace: Some(BAR_NAMESPACE.into()),
                        ..Default::default()
                    },
                    id,
                }));
                changed = true;
            }
            for name in STATIC_ICONS {
                tasks.push(self.want_icon(name));
            }
        }
        if changed {
            self.bar_anchors
                .retain(|(id, _), _| self.bar_surfaces.contains_key(id));
            self.bar_hidden
                .retain(|id, _| self.bar_surfaces.contains_key(id));
            self.bar_full_width
                .retain(|id, _| self.bar_surfaces.contains_key(id));
            if self
                .popover
                .as_ref()
                .is_some_and(|p| !self.bar_surfaces.contains_key(&p.bar))
            {
                tasks.push(self.destroy_popover());
            }
            // La zone réservée a changé : Hyprland ne l'annonce pas, on relit les écrans.
            tasks.push(Task::perform(
                async {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    hypr::monitors_async().await
                },
                |m| Msg::Hypr(hypr::Update::Monitors(m)),
            ));
        }
        Task::batch(tasks)
    }

    /// Ids des widgets affichés dans la barre.
    pub(super) fn bar_widget_ids(&self) -> Vec<String> {
        if !self.cfg.plugin_on("bar") || self.bar_surfaces.is_empty() {
            return Vec::new();
        }
        self.cfg
            .bar
            .iter()
            .flat_map(|b| b.zones.iter().chain(b.portrait.iter().flatten()).flatten())
            .filter_map(|i| {
                if let BarItem::Widget(id) = i {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub(super) fn bar_has(&self, item: &BarItem) -> bool {
        if !self.cfg.plugin_on("bar") || self.bar_surfaces.is_empty() {
            return false;
        }
        self.cfg
            .bar
            .iter()
            .flat_map(|b| b.zones.iter().chain(b.portrait.iter().flatten()).flatten())
            .any(|i| i == item)
    }

    pub(super) fn volume_action(&self, args: Vec<String>) -> Task<Msg> {
        Task::perform(
            async move {
                let _ = crate::processes::output(
                    tokio::process::Command::new("wpctl").args(&args),
                    Duration::from_secs(2),
                )
                .await;
                read_volume().await
            },
            Msg::VolumeValue,
        )
    }

    /// Commande `wpctl` sur le micro, puis relecture du menu Son.
    pub(super) fn mic_action(&self, args: Vec<String>) -> Task<Msg> {
        Task::perform(
            async move {
                let _ = crate::processes::output(
                    tokio::process::Command::new("wpctl").args(&args),
                    Duration::from_secs(2),
                )
                .await;
                read_audio().await
            },
            Msg::AudioValue,
        )
    }

    /// Couleurs de la barre : les siennes, sinon celles du thème.
    pub(super) fn bar_palette(&self) -> BarPalette {
        use crate::config::BarStyle;
        let th = &self.cfg.general.theme;
        let c = self.cfg.bar.as_ref().map(|b| &b.colors);
        let pick =
            |f: fn(&crate::config::BarColors) -> Option<Rgba>, d: Rgba| c.and_then(f).unwrap_or(d);
        // La matière Graphite : le gris du thème, sans sa teinte — la lame
        // reste neutre quel que soit le fond d'écran (les couleurs posées
        // explicitement pour la barre restent celles de l'utilisateur).
        let graphite = self
            .cfg
            .bar
            .as_ref()
            .is_some_and(|b| b.style == BarStyle::Graphite);
        let neutre = |v: Rgba| {
            if graphite {
                super::bar_style::neutral(v)
            } else {
                v
            }
        };
        BarPalette {
            fg: pick(|c| c.text, neutre(th.fg)),
            bg: pick(|c| c.background, neutre(th.bg)),
            accent: pick(|c| c.accent, th.accent),
            warning: pick(|c| c.warning, crate::config::BarColors::WARNING),
            critical: pick(|c| c.critical, crate::config::BarColors::CRITICAL),
            // Graphite privilégie un fond stable sur tout fond d’écran.
            opacity: match c.and_then(|c| c.opacity) {
                Some(o) => o.clamp(0.0, 1.0),
                None if graphite => super::bar_style::ACRYLIC,
                None => th.effect(crate::config::Place::Bar).opacity.clamp(0.0, 1.0),
            },
        }
    }

    fn bar_item_is_launcher(&self, item: &BarItem) -> bool {
        matches!(item, BarItem::Widget(id) if self.cfg.widgets.iter().any(|w| &w.id == id && matches!(w.kind, WidgetKind::Launcher)))
    }

    /// Couleur propre d'un module (par son identifiant, puis par son type).
    pub(super) fn bar_item_color(&self, item: &BarItem) -> Option<Rgba> {
        let m = &self.cfg.bar.as_ref()?.colors.modules;
        match item {
            BarItem::Widget(id) => m.get(id).copied().or_else(|| {
                let w = self.cfg.widgets.iter().find(|w| &w.id == id)?;
                m.get(&w.type_id).copied()
            }),
            other => m.get(bar_item_key(other)).copied(),
        }
    }

    pub(super) fn state_color(&self, s: RenderState, normal: Color) -> Color {
        let p = self.bar_palette();
        match s {
            RenderState::Warning => color(p.warning, 1.0),
            RenderState::Critical => color(p.critical, 1.0),
            RenderState::Active => color(p.accent, 1.0),
            _ => normal,
        }
    }

    pub(super) fn view_bar(&self, id: Id) -> Element<'_, Msg> {
        let (Some(s), Some(bar)) = (self.bar_surfaces.get(&id), &self.cfg.bar) else {
            return Space::new().into();
        };
        let pal = self.bar_palette();
        let style = bar.style;
        let t = super::bar_style::durations(self);
        // Écran vertical : ses propres zones, si la barre en déclare.
        let (zones, zone_offset) = match (&bar.portrait, self.bar_vertical(&s.monitor)) {
            (Some(p), true) => (p, 3),
            _ => (&bar.zones, 0),
        };
        // Chaque zone est rognée : sur un écran étroit, elles ne se chevauchent pas.
        // Réglages ouverts : les modules se déplacent en les glissant (zones
        // ordinaires seulement ; les écrans verticaux gardent les leurs).
        let edit = self.panel.is_some();
        let graphite = style == crate::config::BarStyle::Graphite;
        let content: Element<'_, Msg> = if !edit || graphite {
            let mut slots: Vec<_> = zones
                .iter()
                .enumerate()
                .flat_map(|(zone, items)| {
                    items
                        .iter()
                        .enumerate()
                        .map(move |(i, item)| (zone, i, item))
                })
                .enumerate()
                .map(|(index, (zone, i, item))| super::bar_layout::Slot {
                    zone,
                    priority: self.bar_priority(item),
                    flexible: self.bar_item_is_launcher(item),
                    content: {
                        let content = self.bar_item(id, s, item);
                        let content = if edit && !self.bar_item_is_launcher(item) {
                            self.bar_edit_item(content, zone + zone_offset, i, s.height)
                        } else if edit {
                            mouse_area(content)
                                .on_enter(Msg::BarDragOver(zone + zone_offset, i))
                                .on_release(Msg::BarDragEnd)
                                .into()
                        } else {
                            content
                        };
                        let content = if self.bar_preview == Some((index, true)) {
                            bar_motion::surface(
                                content,
                                0.97,
                                iced::Vector::ZERO,
                                iced::Point::new(0.5, 0.5),
                            )
                        } else {
                            content
                        };
                        if self.bar_item_is_launcher(item) {
                            content
                        } else {
                            bar_motion::anchor(
                                content,
                                id,
                                bar_item_key(item).into(),
                                self.bar_duration(t.hover),
                            )
                        }
                    },
                })
                .collect();
            if edit {
                for (zone, items) in zones.iter().enumerate() {
                    let z = zone + zone_offset;
                    let i = items.len();
                    let accent = color(pal.accent, 1.0);
                    let over = self.bar_drag.is_some_and(|(_, over)| over == Some((z, i)));
                    let target = mouse_area(
                        container(
                            Space::new()
                                .width(if items.is_empty() { 48 } else { 16 })
                                .height((s.height as f32 - 10.0).max(12.0)),
                        )
                        .style(move |_| container::Style {
                            background: over.then(|| color(pal.accent, 0.18).into()),
                            border: Border::default().rounded(4).width(1).color(Color {
                                a: if over { 1.0 } else { 0.25 },
                                ..accent
                            }),
                            ..Default::default()
                        }),
                    )
                    .on_enter(Msg::BarDragOver(z, i))
                    .on_release(Msg::BarDragEnd);
                    slots.push(super::bar_layout::Slot {
                        zone,
                        priority: 102,
                        flexible: false,
                        content: target.into(),
                    });
                }
            }
            let mut eligible: Vec<_> = zones
                .iter()
                .flatten()
                .map(|item| {
                    !self.bar_item_is_launcher(item)
                        && !matches!(
                            item,
                            BarItem::Workspaces
                                | BarItem::Window
                                | BarItem::Desktop
                                | BarItem::Tray
                        )
                })
                .collect();
            if edit {
                eligible.extend([false; 3]);
            }
            eligible.push(true);
            let mut more_content = row![].spacing(5).align_y(Alignment::Center);
            if self.bar_overflow_items(id).contains(&BarItem::Workspaces)
                && let Some(m) = self.monitors.iter().find(|m| m.name == s.monitor)
            {
                more_content = more_content.push(self.label(
                    truncate(&m.workspace, 8),
                    13.0,
                    super::bar_style::ink(style, &pal),
                ));
            }
            more_content = more_content.push(
                self.icon_named("view-more-symbolic", 18.0, color(pal.fg, 1.0))
                    .unwrap_or_else(|| self.label("…", 18.0, color(pal.fg, 1.0))),
            );
            let more = bar_motion::anchor(
                self.module(
                    s,
                    1.0,
                    more_content.into(),
                    Some(Msg::OpenPopover(id, "overflow".into(), Request::Overflow)),
                    None,
                    None,
                    None,
                    Some(crate::i18n::tr("Autres commandes").into()),
                    self.popover
                        .as_ref()
                        .is_some_and(|p| p.key == "overflow" && !p.closing()),
                ),
                id,
                "overflow".into(),
                self.bar_duration(t.hover),
            );
            bar_motion::rail(
                super::bar_layout::strip(
                    id,
                    slots,
                    more,
                    self.bar_compact(&s.monitor),
                    style == crate::config::BarStyle::Graphite,
                ),
                self.bar_preview.map(|(i, _)| i),
                eligible,
                super::bar_style::rail_look(style, &pal),
                self.bar_duration(t.hover),
            )
        } else {
            row![
                container(self.bar_zone(id, s, &zones[0], edit.then_some(zone_offset)))
                    .width(Length::Fill)
                    .align_x(Alignment::Start)
                    .clip(true),
                container(self.bar_zone(id, s, &zones[1], edit.then_some(1 + zone_offset)))
                    .align_x(Alignment::Center)
                    .clip(true),
                container(self.bar_zone(id, s, &zones[2], edit.then_some(2 + zone_offset)))
                    .width(Length::Fill)
                    .align_x(Alignment::End)
                    .clip(true),
            ]
            .align_y(Alignment::Center)
            .height(Length::Fill)
            .into()
        };
        let content: Element<'_, Msg> = if edit {
            mouse_area(content)
                .on_exit(Msg::BarDragLeave(usize::MAX))
                .on_release(Msg::BarDragCancel)
                .into()
        } else {
            content
        };
        // La lame du style porte elle-même ses filets et son reflet.
        let top = s.position == BarPosition::Top;
        // Un seul filet, côté supérieur en Graphite.
        let mut pad = Padding::new(0.0);
        if graphite {
            pad.top = 1.0;
        } else {
            pad.top = if top { 0.0 } else { 1.0 };
            pad.bottom = if top { 1.0 } else { 0.0 };
        }
        pad.left = 8.0;
        pad.right = 8.0;
        let slab = container(content)
            .padding(pad)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| super::bar_style::surface(style, &pal, top));
        if graphite {
            container(slab)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(color(pal.fg, 0.12).into()),
                    ..Default::default()
                })
                .into()
        } else {
            slab.into()
        }
    }

    /// Un espace de cet écran porte-t-il une fenêtre urgente ? Hyprland
    /// l'annonce par `urgent>>`, on la retrouve dans la liste des fenêtres.
    pub(super) fn bar_urgent(&self, monitor: &str, ws: &str) -> bool {
        !self.urgent.is_empty()
            && self.clients.iter().any(|c| {
                self.urgent.contains(&c.address) && c.workspace == ws && c.monitor == monitor
            })
    }

    /// Une bulle de notification est-elle affichée pour une fenêtre de cet
    /// espace ? L'application d'origine (`desktop-entry`, sinon `app_name`)
    /// est rapprochée de la classe d'une fenêtre ; l'indicateur suit la vie
    /// de la bulle (fermée, expirée : l'espace redevient normal).
    pub(super) fn bar_notif(&self, monitor: &str, ws: &str) -> bool {
        !self.toasts.list.is_empty()
            && self.toasts.list.iter().any(|t| {
                let entry =
                    t.n.desktop_entry
                        .as_deref()
                        .map(|e| e.strip_suffix(".desktop").unwrap_or(e));
                self.clients.iter().any(|c| {
                    c.workspace == ws
                        && c.monitor == monitor
                        && (entry.is_some_and(|e| c.class.eq_ignore_ascii_case(e))
                            || (!t.n.app.is_empty() && c.class.eq_ignore_ascii_case(&t.n.app)))
                })
            })
    }

    /// Écran plus haut que large (rotation à 90° ou 270°).
    pub(super) fn bar_vertical(&self, monitor: &str) -> bool {
        self.monitors
            .iter()
            .find(|m| m.name == monitor)
            .is_some_and(|m| m.height > m.width)
    }

    /// Barre compacte sur cet écran : réglage, ou automatique (étroit ou vertical).
    pub(super) fn bar_compact(&self, monitor: &str) -> bool {
        use crate::config::BarCompact;
        match self.cfg.bar.as_ref().map(|b| b.compact).unwrap_or_default() {
            BarCompact::Always => true,
            BarCompact::Never => false,
            BarCompact::Auto => {
                self.bar_vertical(monitor)
                    || self
                        .monitors
                        .iter()
                        .find(|m| m.name == monitor)
                        .is_some_and(|m| {
                            self.bar_surfaces
                                .iter()
                                .filter(|(_, s)| s.monitor == monitor)
                                .any(|(id, _)| {
                                    self.bar_full_width
                                        .get(id)
                                        .is_some_and(|width| *width as f32 > m.width as f32 - 16.0)
                                })
                        })
            }
        }
    }

    fn bar_edit_item<'a>(
        &'a self,
        content: Element<'a, Msg>,
        zone: usize,
        index: usize,
        height: u16,
    ) -> Element<'a, Msg> {
        let accent = color(self.bar_palette().accent, 1.0);
        let dragged = self.bar_drag.is_some_and(|(from, _)| from == (zone, index));
        let h = (height as f32 - 10.0).max(12.0);
        let mark = |index| {
            container(Space::new().width(3).height(h)).style(move |_| container::Style {
                background: self
                    .bar_drag
                    .is_some_and(|(_, over)| over == Some((zone, index)))
                    .then(|| accent.into()),
                ..Default::default()
            })
        };
        let item = container(content).style(move |_| container::Style {
            border: Border::default().rounded(4).width(1).color(Color {
                a: if dragged { 0.9 } else { 0.35 },
                ..accent
            }),
            ..Default::default()
        });
        let handle = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .interaction(if dragged {
                mouse::Interaction::Grabbing
            } else {
                mouse::Interaction::Grab
            })
            .on_press(Msg::BarDragStart(zone, index))
            .on_enter(Msg::BarDragOver(zone, index))
            .on_release(Msg::BarDragEnd);
        let body = row![mark(index), iced::widget::stack![item, handle]].align_y(Alignment::Center);
        body.into()
    }

    fn bar_zone<'a>(
        &'a self,
        id: Id,
        s: &'a BarSurface,
        items: &'a [BarItem],
        edit: Option<usize>,
    ) -> Element<'a, Msg> {
        let compact = self.bar_compact(&s.monitor);
        let Some(z) = edit else {
            let content = row(items.iter().map(|it| {
                bar_motion::anchor(
                    self.bar_item(id, s, it),
                    id,
                    bar_item_key(it).into(),
                    self.bar_duration(140),
                )
            }))
            .spacing(if compact { 0 } else { 2 })
            .align_y(Alignment::Center);
            return bar_motion::rail(
                content,
                None,
                items
                    .iter()
                    .map(|it| {
                        !matches!(
                            it,
                            BarItem::Workspaces
                                | BarItem::Window
                                | BarItem::Desktop
                                | BarItem::Tray
                        )
                    })
                    .collect(),
                super::bar_style::rail_look(
                    self.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default(),
                    &self.bar_palette(),
                ),
                self.bar_duration(super::bar_style::durations(self).hover),
            );
        };
        // Mode réglage : chaque module est une poignée ; un repère d'accent
        // marque la place où il ira.
        let pal = self.bar_palette();
        let accent = color(pal.accent, 1.0);
        let h = (s.height as f32 - 10.0).max(12.0);
        let drag = self.bar_drag;
        let marker = move |at: (usize, usize)| -> Element<'a, Msg> {
            let on = drag.is_some_and(|(_, over)| over == Some(at));
            container(Space::new().width(3).height(h))
                .style(move |_| container::Style {
                    background: on.then(|| accent.into()),
                    border: Border::default().rounded(2),
                    ..Default::default()
                })
                .into()
        };
        let mut r = row![].spacing(0).align_y(Alignment::Center);
        for (i, it) in items.iter().enumerate() {
            r = r.push(marker((z, i)));
            let dragged = drag.is_some_and(|(from, _)| from == (z, i));
            let item = container(self.bar_item(id, s, it)).style(move |_| container::Style {
                border: Border::default().rounded(7).width(1).color(Color {
                    a: if dragged { 0.9 } else { 0.35 },
                    ..accent
                }),
                ..Default::default()
            });
            let handle = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .interaction(mouse::Interaction::Grab)
                .on_press(Msg::BarDragStart(z, i))
                .on_enter(Msg::BarDragOver(z, i))
                .on_release(Msg::BarDragEnd);
            r = r.push(iced::widget::stack![item, handle]);
        }
        r = r.push(marker((z, items.len())));
        // Bout de zone (et zone vide) : on peut y déposer.
        r.push(
            mouse_area(
                Space::new()
                    .width(if items.is_empty() { 48 } else { 16 })
                    .height(h),
            )
            .on_enter(Msg::BarDragOver(z, items.len()))
            .on_release(Msg::BarDragEnd),
        )
        .into()
    }

    /// Module de barre : capsule au survol, clics gauche/droit/milieu, molette,
    /// info-bulle.
    #[allow(clippy::too_many_arguments)]
    fn module<'a>(
        &self,
        bar: &BarSurface,
        alpha: f32,
        content: Element<'a, Msg>,
        left: Option<Msg>,
        right: Option<Msg>,
        middle: Option<Msg>,
        scroll: Option<Box<dyn Fn(i64) -> Msg + 'a>>,
        tip: Option<String>,
        active: bool,
    ) -> Element<'a, Msg> {
        let pal = self.bar_palette();
        let fg = color(pal.fg, alpha);
        let style = self.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default();
        let h = (bar.height as f32 - 8.0).max(16.0);
        let interactive = left.is_some() || right.is_some() || middle.is_some() || scroll.is_some();
        let b = button(
            container(content)
                .height(Length::Fill)
                .align_y(Alignment::Center),
        )
        .height(h)
        .padding([0, 9])
        .on_press_maybe(left.or(interactive.then_some(Msg::Noop)))
        .style(move |_, st| {
            super::bar_style::module_button(style, alpha, fg, color(pal.bg, 1.0), active, st)
        });
        let mut m = mouse_area(b);
        if let Some(r) = right {
            m = m.on_right_press(r);
        }
        if let Some(mid) = middle {
            m = m.on_middle_press(mid);
        }
        if let Some(f) = scroll {
            m = m.on_scroll(move |d| f(scroll_dir(d)));
        }
        let el: Element<'a, Msg> = m.into();
        match tip {
            Some(t) if !t.is_empty() => {
                let bg = color(pal.bg, 0.95);
                let edge = Color { a: 0.12, ..fg };
                tooltip(
                    el,
                    container(
                        text(t)
                            .font(self.font)
                            .size(12)
                            .font(self.ui_font(iced::font::Weight::Normal)),
                    )
                    .padding([5, 9])
                    .style(move |_| container::Style {
                        background: Some(bg.into()),
                        border: Border::default().rounded(7).width(1).color(edge),
                        text_color: Some(fg),
                        shadow: iced::Shadow {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                            offset: iced::Vector::new(0.0, 4.0),
                            blur_radius: 12.0,
                        },
                        ..Default::default()
                    }),
                    if bar.position == BarPosition::Top {
                        tooltip::Position::Bottom
                    } else {
                        tooltip::Position::Top
                    },
                )
                .gap(6)
                .delay(Duration::from_millis(500))
                .into()
            }
            _ => el,
        }
    }

    pub(super) fn label<'a>(&self, s: impl Into<String>, size: f32, c: Color) -> Element<'a, Msg> {
        text(s.into())
            .font(self.font)
            .size(size)
            .color(c)
            .font(self.ui_font(iced::font::Weight::Medium))
            .wrapping(iced::widget::text::Wrapping::None)
            .into()
    }

    pub(super) fn bar_item<'a>(
        &'a self,
        bar_id: Id,
        bar: &'a BarSurface,
        item: &'a BarItem,
    ) -> Element<'a, Msg> {
        self.bar_item_alpha(bar_id, bar, item, 1.0)
    }

    pub(super) fn bar_item_alpha<'a>(
        &'a self,
        bar_id: Id,
        bar: &'a BarSurface,
        item: &'a BarItem,
        alpha: f32,
    ) -> Element<'a, Msg> {
        let pal = self.bar_palette();
        let style = self.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default();
        let t = super::bar_style::durations(self);
        // Couleur du module, sinon le texte de la barre.
        let base = self.bar_item_color(item).unwrap_or(pal.fg);
        let fg = color(base, 0.92 * alpha);
        let dim = color(base, 0.65 * alpha);
        let size = (bar.height as f32 * 0.38).clamp(11.0, 18.0);
        let isz = (size * 1.2).round();
        let monitor = bar.monitor.clone();
        let compact = self.bar_compact(&monitor);
        match item {
            // Les touches d'espaces sont le geste du style : chaque rendu son
            // langage (capsule glissante en Continuité, touche soulevée en Graphite).
            BarItem::Workspaces => super::bar_style::workspaces(self, bar_id, bar, alpha),
            BarItem::Window => {
                if self
                    .monitors
                    .iter()
                    .find(|m| m.name == monitor)
                    .is_some_and(|m| m.width < 1000)
                {
                    return Space::new().into();
                }
                let t = self
                    .active_window
                    .as_ref()
                    .map(|(c, t)| if t.is_empty() { c.clone() } else { t.clone() })
                    .unwrap_or_default();
                container(
                    text(truncate(&t, if compact { 18 } else { 36 }))
                        .wrapping(iced::widget::text::Wrapping::None)
                        .font(self.font)
                        .size(size)
                        .color(dim)
                        .font(self.ui_font(iced::font::Weight::Normal)),
                )
                .padding([0, 8])
                .into()
            }
            // Interrupteur à trois états : no-desktop, launcher-only,
            // full-desktop ; le mode en cours en couleur d'accent, un clic
            // sur un autre y passe.
            BarItem::Desktop => {
                use super::DesktopMode as M;
                // Le mode de cet écran (celui du workspace qu'il affiche) ;
                // un clic le change pour cet écran.
                let mode = self.monitor_mode(&monitor);
                let accent = color(pal.accent, alpha);
                let on_accent =
                    design::on_accent(accent, color(pal.fg, alpha), color(pal.bg, alpha));
                let fgc = color(base, alpha);
                let h = (bar.height as f32 * 0.66).round();
                let seg = |m: M, icon: &str, cmd: control::Command, tip: &'static str| {
                    let active = mode == m;
                    let tint = if active { on_accent } else { fg };
                    let glyph = self.icon_named(icon, isz * 0.9, tint).unwrap_or_else(|| {
                        self.label(m.as_str().chars().take(1).collect::<String>(), size, tint)
                    });
                    iced::widget::tooltip(
                        bar_motion::press(
                            button(
                                container(glyph)
                                    .center_x(Length::Fill)
                                    .center_y(Length::Fill),
                            )
                            .width(h * 1.25)
                            .height(h)
                            .padding(0)
                            .on_press(Msg::On(monitor.clone(), Box::new(Msg::Control(cmd))))
                            .style(move |_, st| button::Style {
                                background: if active {
                                    Some(Background::Color(accent))
                                } else if matches!(st, button::Status::Hovered) {
                                    Some(Background::Color(Color {
                                        a: 0.14 * alpha,
                                        ..fgc
                                    }))
                                } else {
                                    None
                                },
                                text_color: tint,
                                border: Border::default().rounded(h / 2.0),
                                ..Default::default()
                            }),
                            self.bar_duration(90),
                        ),
                        container(
                            text(crate::tr!(
                                "{m} — {t}",
                                m = m.as_str(),
                                t = crate::i18n::tr(tip)
                            ))
                            .font(self.font)
                            .size(12),
                        )
                        .padding([4, 8])
                        .style(move |_| container::Style {
                            background: Some(Background::Color(color(pal.bg, 0.96))),
                            border: Border::default().rounded(8),
                            text_color: Some(fgc),
                            ..Default::default()
                        }),
                        if bar.position == BarPosition::Top {
                            tooltip::Position::Bottom
                        } else {
                            tooltip::Position::Top
                        },
                    )
                };
                container(
                    row![
                        seg(
                            M::NoDesktop,
                            DESKTOP_OFF_ICON,
                            control::Command::Hide,
                            "bureau masqué"
                        ),
                        seg(
                            M::LauncherOnly,
                            DESKTOP_WINDOWS_ICON,
                            control::Command::LauncherOnly,
                            "fenêtres et lanceur"
                        ),
                        seg(
                            M::FullDesktop,
                            "user-desktop-symbolic",
                            control::Command::Show,
                            "bureau complet"
                        ),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center),
                )
                .padding(2)
                .style(move |_| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.10 * alpha,
                        ..fgc
                    })),
                    border: Border::default().rounded(h / 2.0 + 2.0),
                    ..Default::default()
                })
                .into()
            }
            BarItem::Volume => {
                let label = match self.volume {
                    Some((v, false)) if !compact && style != crate::config::BarStyle::Graphite => {
                        format!("{:.0} %", v * 100.0)
                    }
                    _ => String::new(),
                };
                let mut content = row![
                    self.icon_named(volume_icon(self.volume), isz, fg)
                        .unwrap_or_else(|| self.label(crate::i18n::tr("Vol."), size, fg))
                ]
                .spacing(6)
                .align_y(Alignment::Center);
                if !label.is_empty() {
                    content = content.push(self.label(label, size, fg));
                }
                self.module(
                    bar,
                    alpha,
                    content.into(),
                    Some(Msg::OpenPopover(bar_id, "volume".into(), Request::Volume)),
                    None,
                    Some(Msg::VolumeMute),
                    Some(Box::new(|d| Msg::VolumeStep(d as i32 * 5))),
                    None,
                    self.popover
                        .as_ref()
                        .is_some_and(|p| p.key == "volume" && !p.closing()),
                )
            }
            BarItem::Brightness => {
                let Some(level) = self.light.level_for(&monitor) else {
                    return Space::new().into();
                };
                let mut content = row![
                    self.icon_named("display-brightness-symbolic", isz, fg)
                        .unwrap_or_else(|| self.label("☀".to_string(), size, fg))
                ]
                .spacing(6)
                .align_y(Alignment::Center);
                if !compact {
                    content = content.push(self.label(format!("{:.0} %", level * 100.0), size, fg));
                }
                let target = self.light.target_for(&monitor);
                self.module(
                    bar,
                    alpha,
                    content.into(),
                    Some(Msg::OpenPopover(
                        bar_id,
                        "brightness".into(),
                        Request::Brightness,
                    )),
                    None,
                    None,
                    target.map(|t| {
                        Box::new(move |d: i64| Msg::LightStep(t, d as i32 * 5))
                            as Box<dyn Fn(i64) -> Msg>
                    }),
                    Some(crate::i18n::tr("Luminosité").to_string()),
                    self.popover
                        .as_ref()
                        .is_some_and(|p| p.key == "brightness" && !p.closing()),
                )
            }
            BarItem::Tray => {
                let count = self.tray.len();
                let icons = self.tray.iter().map(|t| {
                    // Sans image : icône générique plutôt qu'un « ? ».
                    let img: Element<'_, Msg> = self
                        .tray_icon(t, isz, alpha)
                        .or_else(|| self.icon_named("application-x-executable-symbolic", isz, fg))
                        .unwrap_or_else(|| {
                            self.label(
                                t.title
                                    .chars()
                                    .next()
                                    .map(|c| c.to_uppercase().to_string())
                                    .unwrap_or("•".into()),
                                size,
                                fg,
                            )
                        });
                    let menu = t.menu.clone().map(|mp| {
                        Msg::OpenPopover(
                            bar_id,
                            format!("tray:{}", t.key),
                            Request::Tray {
                                bus: t.bus.clone(),
                                menu_path: mp,
                            },
                        )
                    });
                    let left = if t.item_is_menu {
                        menu.clone()
                    } else {
                        Some(Msg::TrayActivate(
                            bar_id,
                            t.bus.clone(),
                            t.path.clone(),
                            false,
                        ))
                    };
                    let module = self.module(
                        bar,
                        alpha,
                        img,
                        left,
                        menu,
                        Some(Msg::TrayActivate(
                            bar_id,
                            t.bus.clone(),
                            t.path.clone(),
                            true,
                        )),
                        None,
                        Some(t.title.clone()),
                        self.popover
                            .as_ref()
                            .is_some_and(|p| p.key == format!("tray:{}", t.key) && !p.closing()),
                    );
                    if self.bar_overflow_items(bar_id).contains(&BarItem::Tray) {
                        module
                    } else {
                        bar_motion::anchor(
                            module,
                            bar_id,
                            format!("tray:{}", t.key),
                            self.bar_duration(140),
                        )
                    }
                });
                bar_motion::rail(
                    row(icons).spacing(0).align_y(Alignment::Center),
                    None,
                    vec![true; count],
                    super::bar_style::rail_look(style, &pal),
                    self.bar_duration(t.hover),
                )
            }
            BarItem::Widget(id) => {
                let Some(w) = self.cfg.widgets.iter().find(|w| &w.id == id) else {
                    return Space::new().into();
                };
                let value = self.values.get(id);
                let open = self
                    .popover
                    .as_ref()
                    .is_some_and(|p| &p.key == id && !p.closing());
                match &w.kind {
                    WidgetKind::Launcher => self
                        .monitors
                        .iter()
                        .find(|m| m.name == monitor)
                        .map_or_else(|| Space::new().into(), |m| self.view_launcher_bar(m, alpha)),
                    WidgetKind::Controls => self.module(
                        bar,
                        alpha,
                        self.icon_named("preferences-system-controls-symbolic", isz, fg)
                            .unwrap_or_else(|| self.label(crate::i18n::tr("Réglages"), size, fg)),
                        Some(Msg::OpenPopover(
                            bar_id,
                            id.clone(),
                            Request::Widget(id.clone()),
                        )),
                        None,
                        None,
                        None,
                        Some(crate::i18n::tr("Centre de contrôle").into()),
                        open,
                    ),
                    WidgetKind::Clock { format } => self.module(
                        bar,
                        alpha,
                        if style == crate::config::BarStyle::Graphite {
                            iced::widget::column![
                                self.label(format_now("%H:%M"), 12.0, fg),
                                self.label(format_now("%d/%m/%Y"), 11.0, fg),
                            ]
                            .align_x(Alignment::End)
                            .spacing(0)
                            .into()
                        } else {
                            self.label(format_now(if compact { "%H:%M" } else { format }), size, fg)
                        },
                        Some(Msg::OpenPopover(bar_id, id.clone(), Request::Calendar)),
                        None,
                        None,
                        None,
                        None,
                        open,
                    ),
                    WidgetKind::Calendar => self.module(
                        bar,
                        alpha,
                        self.label(format_now("%a %-d %b"), size, fg),
                        Some(Msg::OpenPopover(bar_id, id.clone(), Request::Calendar)),
                        None,
                        None,
                        None,
                        None,
                        open,
                    ),
                    WidgetKind::System { .. } => match value {
                        Some(widgets::Value::Sys(lines)) => {
                            let parts = lines.iter().map(|l| {
                                let v = match l.ratio {
                                    Some(r) => format!("{:.0} %", r * 100.0),
                                    None => l.value.clone(),
                                };
                                row![
                                    self.label(l.label.clone(), size * 0.9, dim),
                                    self.label(v, size, fg)
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center)
                                .into()
                            });
                            self.module(
                                bar,
                                alpha,
                                row(parts).spacing(10).align_y(Alignment::Center).into(),
                                None,
                                None,
                                None,
                                None,
                                None,
                                false,
                            )
                        }
                        _ => Space::new().into(),
                    },
                    WidgetKind::Media { .. } => match value {
                        Some(widgets::Value::Media(Some(mi))) => {
                            let label = if mi.artist.is_empty() {
                                mi.title.clone()
                            } else {
                                format!("{} — {}", mi.title, mi.artist)
                            };
                            let icon = if mi.playing {
                                "media-playback-pause-symbolic"
                            } else {
                                "media-playback-start-symbolic"
                            };
                            self.module(
                                bar,
                                alpha,
                                row![
                                    self.icon_named(icon, isz * 0.85, fg).unwrap_or_else(
                                        || self.label(crate::i18n::tr("▶"), size, fg)
                                    ),
                                    self.label(
                                        if compact {
                                            String::new()
                                        } else {
                                            truncate(&label, 40)
                                        },
                                        size,
                                        fg
                                    )
                                ]
                                .spacing(7)
                                .align_y(Alignment::Center)
                                .into(),
                                Some(Msg::Media(mi.bus.clone(), "PlayPause")),
                                None,
                                None,
                                Some(Box::new({
                                    let bus = mi.bus.clone();
                                    move |d| {
                                        Msg::Media(
                                            bus.clone(),
                                            if d > 0 { "Previous" } else { "Next" },
                                        )
                                    }
                                })),
                                Some(label),
                                false,
                            )
                        }
                        _ => Space::new().into(),
                    },
                    WidgetKind::Command { .. } => match value {
                        Some(widgets::Value::Text(t)) if !t.is_empty() => self.module(
                            bar,
                            alpha,
                            self.label(truncate(t.lines().next().unwrap_or(""), 60), size, fg),
                            None,
                            None,
                            None,
                            None,
                            Some(t.clone()),
                            false,
                        ),
                        _ => Space::new().into(),
                    },
                    WidgetKind::Lua {
                        click,
                        scroll,
                        menu,
                        ..
                    } => {
                        let r = match value {
                            Some(widgets::Value::Rich(r)) => r.clone(),
                            Some(widgets::Value::Error(e)) => crate::config::Rendered {
                                text: "⚠".into(),
                                tooltip: Some(e.clone()),
                                state: RenderState::Warning,
                                ..Default::default()
                            },
                            _ => return Space::new().into(),
                        };
                        if r.state == RenderState::Hidden {
                            return Space::new().into();
                        }
                        // État « actif » : la couleur choisie pour le module l'emporte
                        // sur l'accent ; avertissement et critique restent visibles.
                        let tint = match r.state {
                            RenderState::Active if self.bar_item_color(item).is_some() => {
                                color(base, alpha)
                            }
                            s => self.state_color(s, color(base, 0.92)).scale_alpha(alpha),
                        };
                        let mut content = row![].spacing(6).align_y(Alignment::Center);
                        let mut has_icon = false;
                        if let Some(icon) = r
                            .icon
                            .as_deref()
                            .and_then(|name| self.icon_named(name, isz, tint))
                        {
                            content = content.push(icon);
                            has_icon = true;
                        }
                        if !r.text.is_empty() && !(compact && has_icon) {
                            content = content.push(self.label(truncate(&r.text, 48), size, tint));
                        }
                        let menu_msg = menu.map(|f| {
                            Msg::OpenPopover(
                                bar_id,
                                id.clone(),
                                Request::LuaMenu {
                                    widget: id.clone(),
                                    func: f,
                                },
                            )
                        });
                        let click_msg = click.map(|f| Msg::LuaCall(id.clone(), f, None));
                        let (left, right) = match (menu_msg, click_msg) {
                            (Some(m), Some(c)) => (Some(c), Some(m)),
                            (Some(m), None) => (Some(m), None),
                            (None, c) => (c, None),
                        };
                        let wid = id.clone();
                        let scroll_cb: Option<Box<dyn Fn(i64) -> Msg + 'a>> = scroll.map(|f| {
                            Box::new(move |d: i64| Msg::LuaCall(wid.clone(), f, Some(d)))
                                as Box<dyn Fn(i64) -> Msg + 'a>
                        });
                        self.module(
                            bar,
                            alpha,
                            content.into(),
                            left,
                            right,
                            None,
                            scroll_cb,
                            r.tooltip.clone(),
                            open,
                        )
                    }
                }
            }
        }
    }

    fn tray_icon(
        &self,
        t: &crate::tray::TrayItem,
        size: f32,
        alpha: f32,
    ) -> Option<Element<'_, Msg>> {
        if let Some((_, h)) = self.tray_imgs.get(&t.key) {
            return Some(
                iced::widget::image(h.clone())
                    .opacity(alpha)
                    .width(size)
                    .height(size)
                    .into(),
            );
        }
        let name = t.icon_name.as_deref()?;
        // Les icônes de la zone de notification gardent leurs couleurs.
        match self.named_icons.get(name).copied().unwrap_or(IconImg::None) {
            IconImg::None => None,
            img => self.img_element(img, size, alpha),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peripheriques_wpctl() {
        let status = "PipeWire 'pipewire-0' [1.4.9, jacques@pc, cookie:1]
 └─ Clients:
        33. WirePlumber                         [1.4.9, jacques@pc, pid:1]

Audio
 ├─ Devices:
 │      34. Built-in Audio                      [alsa]
 │  
 ├─ Sinks:
 │  *   61. Arctis Nova Pro Wireless Analog Stereo [vol: 0.50]
 │      71. Built-in Audio Digital Stereo (HDMI) [vol: 1.00 MUTED]
 │  
 ├─ Sources:
 │  *   54. Arctis Nova Pro Wireless Mono       [vol: 0.90]
 │      59. HD Webcam eMeet C960 Analog Stereo  [vol: 0.93]
 │  
 ├─ Filters:
 │  
 └─ Streams:
       110. Brave
             82. output_FL       > Arctis:playback_FL	[active]

Video
 ├─ Devices:
 │      50. HD Webcam                            [v4l2]
 ├─ Sinks:
 │  *   99. Faux                                 [vol: 1.00]
";
        let (outs, ins) = parse_wpctl_status(status);
        assert_eq!(
            outs,
            vec![
                AudioDevice {
                    id: 61,
                    name: "Arctis Nova Pro Wireless Analog Stereo".into(),
                    default: true
                },
                AudioDevice {
                    id: 71,
                    name: "Built-in Audio Digital Stereo (HDMI)".into(),
                    default: false
                },
            ]
        );
        assert_eq!(ins.len(), 2);
        assert_eq!((ins[1].id, ins[1].default), (59, false));
        assert_eq!(parse_wpctl_status(""), (vec![], vec![]));
    }

    #[test]
    fn volume_wpctl() {
        assert_eq!(parse_volume("Volume: 0.60"), Some((0.6, false)));
        assert_eq!(parse_volume("Volume: 0.35 [MUTED]"), Some((0.35, true)));
        assert_eq!(parse_volume("erreur"), None);
        assert_eq!(truncate("abcdef", 3), "abc…");
        assert_eq!(
            volume_icon(Some((0.9, false))),
            "audio-volume-high-symbolic"
        );
        assert_eq!(
            volume_icon(Some((0.9, true))),
            "audio-volume-muted-symbolic"
        );
    }
}
