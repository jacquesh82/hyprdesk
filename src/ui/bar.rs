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
use crate::config::{BarItem, BarPosition, RenderState, Rgba, WidgetKind};
use crate::{control, hypr, widgets};

pub const BAR_NAMESPACE: &str = "hyprdesk-bar";

/// Écran de barre désignant l'écran principal (`hd.bar({ monitor = "primary" })`).
pub const PRIMARY: &str = "primary";

/// Icône du bureau affiché avec les fenêtres (dessinée par hyprdesk).
pub const DESKTOP_WINDOWS_ICON: &str = "hyprdesk-desktop-windows-symbolic";

/// Icônes symboliques demandées dès que la barre existe.
pub const STATIC_ICONS: &[&str] = &[
    "user-desktop-symbolic",
    DESKTOP_WINDOWS_ICON,
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
];

/// Surface de barre : écran, position et hauteur à sa création.
#[derive(Debug, Clone, PartialEq)]
pub struct BarSurface {
    pub monitor: String,
    pub position: BarPosition,
    pub height: u16,
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

fn scroll_dir(d: mouse::ScrollDelta) -> i64 {
    let y = match d {
        mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => y,
    };
    if y > 0.0 { 1 } else { -1 }
}

impl App {
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
        let spec = self.cfg.bar.as_ref().map(|b| (b.position, b.height));
        let mut tasks = Vec::new();
        let mut changed = false;
        for (id, s) in self.bar_surfaces.clone() {
            if !(want.contains(&s.monitor) && spec == Some((s.position, s.height))) {
                self.bar_surfaces.remove(&id);
                tasks.push(iced::window::close(id));
                changed = true;
            }
        }
        if let Some((position, height)) = spec {
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
                    },
                );
                tasks.push(Task::done(Msg::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some((0, height as u32)),
                        layer: Layer::Top,
                        anchor: edge | LsAnchor::Left | LsAnchor::Right,
                        exclusive_zone: Some(height as i32),
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
        self.cfg
            .bar
            .iter()
            .flat_map(|b| b.zones.iter().flatten())
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
        self.cfg
            .bar
            .iter()
            .flat_map(|b| b.zones.iter().flatten())
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
        let th = &self.cfg.general.theme;
        let c = self.cfg.bar.as_ref().map(|b| &b.colors);
        let pick =
            |f: fn(&crate::config::BarColors) -> Option<Rgba>, d: Rgba| c.and_then(f).unwrap_or(d);
        BarPalette {
            fg: pick(|c| c.text, th.fg),
            bg: pick(|c| c.background, th.bg),
            accent: pick(|c| c.accent, th.accent),
            warning: pick(|c| c.warning, crate::config::BarColors::WARNING),
            critical: pick(|c| c.critical, crate::config::BarColors::CRITICAL),
            opacity: c
                .and_then(|c| c.opacity)
                .unwrap_or_else(|| th.effect(crate::config::Place::Bar).opacity)
                .clamp(0.0, 1.0),
        }
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

    fn state_color(&self, s: RenderState, normal: Color) -> Color {
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
        let bg = color(pal.bg, pal.opacity);
        let hair = color(pal.fg, 0.07 * pal.opacity.max(0.3));
        // Écran vertical : ses propres zones, si la barre en déclare.
        let zones = match (&bar.portrait, self.bar_vertical(&s.monitor)) {
            (Some(p), true) => p,
            _ => &bar.zones,
        };
        // Chaque zone est rognée : sur un écran étroit, elles ne se chevauchent pas.
        // Réglages ouverts : les modules se déplacent en les glissant (zones
        // ordinaires seulement ; les écrans verticaux gardent les leurs).
        let edit = self.panel.is_some() && std::ptr::eq(zones, &bar.zones);
        let content = row![
            container(self.bar_zone(id, s, &zones[0], edit.then_some(0)))
                .width(Length::Fill)
                .align_x(Alignment::Start)
                .clip(true),
            container(self.bar_zone(id, s, &zones[1], edit.then_some(1)))
                .align_x(Alignment::Center)
                .clip(true),
            container(self.bar_zone(id, s, &zones[2], edit.then_some(2)))
                .width(Length::Fill)
                .align_x(Alignment::End)
                .clip(true),
        ]
        .align_y(Alignment::Center)
        .height(Length::Fill);
        // Filet d'un pixel du côté des fenêtres, comme une barre de menus.
        let top = s.position == BarPosition::Top;
        container(content)
            .padding(Padding {
                top: if top { 0.0 } else { 1.0 },
                bottom: if top { 1.0 } else { 0.0 },
                left: 8.0,
                right: 8.0,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border {
                    color: hair,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                shadow: iced::Shadow {
                    color: hair,
                    offset: iced::Vector::new(0.0, if top { 1.0 } else { -1.0 }),
                    blur_radius: 0.0,
                },
                ..Default::default()
            })
            .into()
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
                        .is_some_and(|m| m.width < 1600)
            }
        }
    }

    fn bar_zone<'a>(
        &'a self,
        id: Id,
        s: &BarSurface,
        items: &'a [BarItem],
        edit: Option<usize>,
    ) -> Element<'a, Msg> {
        let compact = self.bar_compact(&s.monitor);
        let Some(z) = edit else {
            return row(items.iter().map(|it| self.bar_item(id, s, it)))
                .spacing(if compact { 0 } else { 2 })
                .align_y(Alignment::Center)
                .into();
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
        content: Element<'a, Msg>,
        left: Option<Msg>,
        right: Option<Msg>,
        middle: Option<Msg>,
        scroll: Option<Box<dyn Fn(i64) -> Msg + 'a>>,
        tip: Option<String>,
        active: bool,
    ) -> Element<'a, Msg> {
        let pal = self.bar_palette();
        let fg = color(pal.fg, 1.0);
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
            let a = if active {
                0.16
            } else {
                match st {
                    button::Status::Hovered => 0.11,
                    button::Status::Pressed => 0.18,
                    _ => 0.0,
                }
            };
            button::Style {
                background: (a > 0.0).then_some(Background::Color(Color { a, ..fg })),
                text_color: fg,
                border: Border::default().rounded(7),
                ..Default::default()
            }
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

    fn label<'a>(&self, s: impl Into<String>, size: f32, c: Color) -> Element<'a, Msg> {
        text(s.into())
            .font(self.font)
            .size(size)
            .color(c)
            .font(self.title_font())
            .wrapping(iced::widget::text::Wrapping::None)
            .into()
    }

    fn bar_item<'a>(&'a self, bar_id: Id, bar: &BarSurface, item: &'a BarItem) -> Element<'a, Msg> {
        let pal = self.bar_palette();
        // Couleur du module, sinon le texte de la barre.
        let base = self.bar_item_color(item).unwrap_or(pal.fg);
        let fg = color(base, 0.92);
        let dim = color(base, 0.5);
        let size = (bar.height as f32 * 0.38).clamp(11.0, 18.0);
        let isz = (size * 1.2).round();
        let monitor = bar.monitor.clone();
        let compact = self.bar_compact(&monitor);
        match item {
            BarItem::Workspaces => {
                let current = self
                    .monitors
                    .iter()
                    .find(|m| m.name == monitor)
                    .map(|m| m.workspace.clone())
                    .unwrap_or_default();
                let accent = color(pal.accent, 1.0);
                let on_accent = color(pal.bg, 1.0);
                let pills = self
                    .workspaces
                    .iter()
                    .filter(|w| w.monitor == monitor)
                    .map(|w| {
                        let active = w.name == current;
                        let (txt, fill) = if active {
                            (on_accent, Some(accent))
                        } else if w.windows > 0 {
                            (fg, None)
                        } else {
                            (dim, None)
                        };
                        let h = (bar.height as f32 * 0.6).round();
                        let fgc = color(base, 1.0);
                        button(
                            container(self.label(w.name.clone(), size * 0.92, txt))
                                .center_x(Length::Shrink)
                                .center_y(Length::Fill),
                        )
                        .height(h)
                        .padding([0, if w.name.chars().count() > 1 { 8 } else { 0 }])
                        .width(if w.name.chars().count() > 1 {
                            Length::Shrink
                        } else {
                            Length::Fixed(h)
                        })
                        .on_press(Msg::BarWorkspace(monitor.clone(), w.name.clone()))
                        .style(move |_, st| button::Style {
                            background: match (fill, st) {
                                (Some(f), _) => Some(Background::Color(f)),
                                (None, button::Status::Hovered) => {
                                    Some(Background::Color(Color { a: 0.12, ..fgc }))
                                }
                                _ => None,
                            },
                            text_color: txt,
                            border: Border::default().rounded(h / 2.0),
                            ..Default::default()
                        })
                        .into()
                    });
                let mon = monitor.clone();
                mouse_area(
                    container(row(pills).spacing(3).align_y(Alignment::Center)).padding([0, 4]),
                )
                .on_scroll(move |d| Msg::BarScroll(mon.clone(), -scroll_dir(d) as i32))
                .into()
            }
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
                        .font(self.title_font()),
                )
                .padding([0, 8])
                .into()
            }
            // Trois modes, dans l'ordre des clics : no-desktop → launcher-only →
            // full-desktop → no-desktop. Clic droit : no-desktop tout de suite.
            BarItem::Desktop => {
                let mode = self.mode();
                let icon = match mode {
                    super::DesktopMode::LauncherOnly => DESKTOP_WINDOWS_ICON,
                    _ => "user-desktop-symbolic",
                };
                let tip = crate::tr!(
                    "{mode} — clic : {next}{right}",
                    mode = mode.as_str(),
                    next = mode.next().as_str(),
                    right = if self.visible {
                        crate::i18n::tr(" · clic droit : no-desktop")
                    } else {
                        ""
                    }
                );
                self.module(
                    bar,
                    self.icon_named(icon, isz, fg)
                        .unwrap_or_else(|| self.label(crate::i18n::tr("Bureau"), size, fg)),
                    Some(Msg::Control(control::Command::Cycle)),
                    self.visible.then_some(Msg::Control(control::Command::Hide)),
                    None,
                    None,
                    Some(tip),
                    self.visible,
                )
            }
            BarItem::Volume => {
                let label = match self.volume {
                    Some((v, false)) if !compact => format!("{:.0} %", v * 100.0),
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
                    content.into(),
                    Some(Msg::OpenPopover(bar_id, "volume".into(), Request::Volume)),
                    None,
                    Some(Msg::VolumeMute),
                    Some(Box::new(|d| Msg::VolumeStep(d as i32 * 5))),
                    None,
                    self.popover.as_ref().is_some_and(|p| p.key == "volume"),
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
                    self.popover.as_ref().is_some_and(|p| p.key == "brightness"),
                )
            }
            BarItem::Tray => {
                let icons = self.tray.iter().map(|t| {
                    // Sans image : icône générique plutôt qu'un « ? ».
                    let img: Element<'_, Msg> = self
                        .tray_icon(t, isz)
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
                    self.module(
                        bar,
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
                            .is_some_and(|p| p.key == format!("tray:{}", t.key)),
                    )
                });
                row(icons).spacing(0).align_y(Alignment::Center).into()
            }
            BarItem::Widget(id) => {
                let Some(w) = self.cfg.widgets.iter().find(|w| &w.id == id) else {
                    return Space::new().into();
                };
                let value = self.values.get(id);
                let open = self.popover.as_ref().is_some_and(|p| &p.key == id);
                match &w.kind {
                    WidgetKind::Clock { format } => self.module(
                        bar,
                        self.label(format_now(if compact { "%H:%M" } else { format }), size, fg),
                        Some(Msg::OpenPopover(bar_id, id.clone(), Request::Calendar)),
                        None,
                        None,
                        None,
                        None,
                        open,
                    ),
                    WidgetKind::Calendar => self.module(
                        bar,
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
                                color(base, 1.0)
                            }
                            s => self.state_color(s, fg),
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

    fn tray_icon(&self, t: &crate::tray::TrayItem, size: f32) -> Option<Element<'_, Msg>> {
        if let Some((_, h)) = self.tray_imgs.get(&t.key) {
            return Some(
                iced::widget::image(h.clone())
                    .width(size)
                    .height(size)
                    .into(),
            );
        }
        let name = t.icon_name.as_deref()?;
        // Les icônes de la zone de notification gardent leurs couleurs.
        match self.named_icons.get(name).copied().unwrap_or(IconImg::None) {
            IconImg::None => None,
            img => self.img_element(img, size, 1.0),
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
