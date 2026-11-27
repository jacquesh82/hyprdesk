//! Barre native : une surface layer-shell par écran, sur le calque `top`
//! (au-dessus des fenêtres), avec une zone exclusive — Hyprland range les
//! fenêtres à côté, et le bureau décale sa grille de la zone réservée.
//!
//! Dessin soigné, à la manière d'une barre de menus macOS : fond translucide
//! flouté, filet discret, icônes symboliques teintées, capsule au survol,
//! menus déroulants (voir `popover`). Ses modules sont des widgets.

use crate::ui::tokens::Shown as _;
use std::time::Duration;

use crate::i18n::text;
use iced::widget::{Space, button, container, mouse_area, row, tooltip};
use iced::window::Id;
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Task, mouse};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::motion;
use crate::config::{BarItem, BarPosition, RenderState, Rgba, WidgetKind};
use crate::ui::design;
use crate::ui::plugins::bar::popover::{self, Request};
use crate::ui::{App, IconImg, Msg, color, format_now};
use crate::{control, hypr, widgets};

/// Graisse des libellés de la barre, que suivent les glyphes voisins
/// (HIG ICO-12).
const LABEL_WEIGHT: iced::font::Weight = iced::font::Weight::Medium;

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
/// Chevron qui marque le titre de la fenêtre active comme un menu (HIG MENU-07).
pub const MENU_CHEVRON: &str = "go-down-symbolic";

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
    // Chevron du titre de la fenêtre active : c'est un titre de menu.
    MENU_CHEVRON,
    URGENT_ICON,
];

/// Surface de barre : écran, position, hauteur et marge au bord à sa création.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarSurface {
    pub monitor: String,
    pub position: BarPosition,
    pub height: u16,
    pub margin: u16,
    /// Forme posée à la création (MLG-277).
    pub shape: crate::config::BarShape,
    /// Largeur de la surface (0 : tout le bord).
    pub width: u32,
}

/// Une barre passe par-dessus un plein écran, sauf réglée déjà sur
/// `Overlay` (visible par-dessus, au choix de l'utilisateur) ou en Convex
/// masqué hors survol, qui a son propre geste (HIG PAT-03).
fn bar_covers(layer: crate::config::BarLayer, convex_auto_hide: bool, fullscreen: bool) -> bool {
    fullscreen && layer != crate::config::BarLayer::Overlay && !convex_auto_hide
}

/// Couleurs effectives de la barre.
#[derive(Debug, Clone, Copy)]
pub(in crate::ui) struct BarPalette {
    pub fg: Rgba,
    pub bg: Rgba,
    pub accent: Rgba,
    pub warning: Rgba,
    pub critical: Rgba,
    /// Opacité du fond.
    pub opacity: f32,
}

impl BarPalette {
    pub(in crate::ui) fn resolve(
        theme: &crate::config::Theme,
        colors: Option<&crate::config::BarColors>,
    ) -> Self {
        use crate::config::{BarColors, Place};

        let light = light_theme(theme);
        let pick = |f: fn(&BarColors) -> Option<&crate::config::ColorRef>, fallback| {
            colors
                .and_then(f)
                .and_then(|c| resolve_color(c, light))
                .unwrap_or(fallback)
        };
        Self {
            fg: pick(|c| c.text.as_ref(), theme.fg),
            // Teinte du verre de la barre (MLG-268).
            bg: crate::ui::tokens::tinted(pick(|c| c.background.as_ref(), theme.bg), Place::Bar),
            accent: pick(|c| c.accent.as_ref(), theme.accent),
            // États : slots de la palette du moteur, lisibles sur le fond.
            warning: pick(|c| c.warning.as_ref(), crate::ui::tone::rgba("warning")),
            critical: pick(|c| c.critical.as_ref(), crate::ui::tone::rgba("danger")),
            opacity: colors
                .and_then(|c| c.opacity)
                .unwrap_or_else(|| theme.shown(Place::Bar).opacity)
                .clamp(0.0, 1.0),
        }
    }
}

/// Thème clair : fond lumineux (variante `light` d'un pack, préréglage
/// Clair) — c'est ce que voit l'utilisateur, quel que soit le pack.
pub(in crate::ui) fn light_theme(theme: &crate::config::Theme) -> bool {
    crate::theme_engine::color::luminance(theme.bg) > 0.4
}

/// Couleur réglée résolue dans le mode du thème (MLG-263) : un slot se lit
/// dans la palette du pack actif, une paire selon le mode.
pub(in crate::ui) fn resolve_color(c: &crate::config::ColorRef, light: bool) -> Option<Rgba> {
    c.resolve(light, &crate::ui::tone::get)
}

/// Clé d'un module propre à la barre (couleur par module, `module_colors`).
pub fn bar_item_key(item: &BarItem) -> &str {
    match item {
        BarItem::Menus => "menus",
        BarItem::Workspaces => "workspaces",
        BarItem::Window => "window",
        BarItem::Desktop => "desktop",
        BarItem::Volume => "volume",
        BarItem::Tray => "tray",
        BarItem::Brightness => "brightness",
        BarItem::Widget(id) => id,
    }
}

impl App {
    /// Libellé d'un module de la barre : celui du catalogue des widgets, ou
    /// celui des modules propres à la barre (`BAR_ITEMS`).
    pub(in crate::ui) fn bar_item_label(&self, item: &BarItem) -> String {
        match item {
            BarItem::Widget(id) => self
                .cfg
                .widgets
                .iter()
                .find(|w| &w.id == id)
                .map(|w| {
                    self.cfg
                        .catalog
                        .iter()
                        .find(|t| t.id == w.type_id)
                        .map_or(w.type_id.as_str(), |t| t.name.as_str())
                })
                .unwrap_or(id)
                .to_string(),
            _ => crate::config::BAR_ITEMS
                .iter()
                .find(|(id, _)| *id == bar_item_key(item))
                .map_or("", |(_, label)| crate::i18n::tr(label))
                .to_string(),
        }
    }
}

pub(in crate::ui) fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

/// Clic sur l'horloge de la barre (HIG SYS-13) : en style Island, la bulle
/// vivante qui occupe la pilule centrale ouvre sa vue étendue — corps,
/// actions, carte des notifications ; dans tous les autres cas, le calendrier.
pub(in crate::ui) fn clock_press(
    bar: Id,
    module: &str,
    style: crate::config::BarStyle,
    toast: Option<&crate::ui::toasts::Toast>,
) -> Msg {
    match (style, toast) {
        (crate::config::BarStyle::Island, Some(t)) => {
            Msg::OpenPopover(bar, module.to_string(), Request::Activity { id: t.n.id })
        }
        _ => Msg::OpenPopover(bar, module.to_string(), Request::Calendar),
    }
}

/// Clic droit sur un module de barre : son menu contextuel (HIG ENT-07). La
/// carte s'ancre au module, sous la clé `menu_key(module)` : un clic droit ne
/// bascule pas la carte du clic gauche.
/// Cible d'un module de barre : la hauteur de la barre moins 8 px, sans
/// descendre sous le minimum recommandé par la HIG (20 × 20 px), ni dépasser
/// la barre (HIG ACC-09).
const MODULE_TARGET: f32 = 20.0;

/// Marge horizontale du cadre d'un module, de part et d'autre du contenu.
const MODULE_PAD_X: f32 = 9.0;
/// Écart entre l'ovale d'un compte et le contenu du module (HIG SYS-02).
const BADGE_GAP: f32 = 1.0;

fn module_target(bar_height: u16) -> f32 {
    (bar_height as f32 - 8.0)
        .max(MODULE_TARGET)
        .min(bar_height as f32)
}

fn module_menu(bar: Id, module: &str) -> Msg {
    Msg::OpenPopover(
        bar,
        popover::menu_key(module),
        Request::ModuleMenu(module.to_string()),
    )
}

/// `wpctl get-volume` → (volume 0..1, coupé).
pub fn parse_volume(out: &str) -> Option<(f32, bool)> {
    let v: f32 = out.split_whitespace().nth(1)?.parse().ok()?;
    (v.is_finite() && v >= 0.0).then_some((v, out.contains("[MUTED]")))
}

/// Le volume suit les événements de PipeWire (`pactl subscribe`) : vrai tant
/// que la veille tourne ; le relevé toutes les 2 s (un `wpctl` à chaque fois)
/// ne sert alors plus (MLG-134).
pub static VOLUME_WATCH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Ligne de `pactl subscribe` qui peut changer le volume de la sortie : la
/// sortie elle-même, ou le serveur (sortie par défaut changée).
pub fn volume_event(line: &str) -> bool {
    line.contains(" on sink #") || line.contains(" on server")
}

/// Volume de la sortie, relu à chaque événement de PipeWire. Sans `pactl`
/// (pipewire-pulse absent), le flux se tait et le relevé périodique reste.
pub fn volume_stream() -> impl iced::futures::Stream<Item = Option<(f32, bool)>> {
    use iced::futures::SinkExt;
    use std::sync::atomic::Ordering;
    use tokio::io::AsyncBufReadExt;
    iced::stream::channel(
        4,
        async move |mut out: iced::futures::channel::mpsc::Sender<Option<(f32, bool)>>| {
            loop {
                let spawned = tokio::process::Command::new("pactl")
                    .arg("subscribe")
                    .env("LC_ALL", "C")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .kill_on_drop(true)
                    .spawn();
                let Ok(mut child) = spawned else {
                    // Pas de pactl : le relevé périodique fait le travail.
                    std::future::pending::<()>().await;
                    return;
                };
                let Some(stdout) = child.stdout.take() else {
                    return;
                };
                VOLUME_WATCH.store(true, Ordering::Relaxed);
                let _ = out.send(read_volume().await).await;
                let mut lines = tokio::io::BufReader::new(stdout).lines();
                loop {
                    match lines.next_line().await {
                        Ok(Some(l)) if volume_event(&l) => {
                            // Rafale (molette, curseur) : un seul relevé.
                            while let Ok(Ok(Some(_))) =
                                tokio::time::timeout(Duration::from_millis(60), lines.next_line())
                                    .await
                            {}
                            let _ = out.send(read_volume().await).await;
                        }
                        Ok(Some(_)) => {}
                        _ => break,
                    }
                }
                // PipeWire relancé : relevé périodique le temps de se réabonner.
                VOLUME_WATCH.store(false, Ordering::Relaxed);
                let _ = child.kill().await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        },
    )
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

pub(in crate::ui) fn scroll_dir(d: mouse::ScrollDelta) -> i64 {
    let y = match d {
        mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => y,
    };
    if y > 0.0 { 1 } else { -1 }
}

impl App {
    pub(in crate::ui) fn bar_input(&self, id: Id, bounds: Option<iced::Rectangle>) -> Task<Msg> {
        let Some(surface) = self.bar_surfaces.get(&id) else {
            return Task::none();
        };
        let width = self
            .monitors
            .iter()
            .find(|m| m.name == surface.monitor)
            .map_or(16384.0, |m| m.width as f32);
        let bounds = bounds.unwrap_or(iced::Rectangle::new(
            iced::Point::ORIGIN,
            iced::Size::new(width, surface.height as f32),
        ));
        Task::done(Msg::SetInputRegion {
            id,
            callback: iced_layershell::actions::ActionCallback::new(move |region| {
                region.add(
                    bounds.x.floor() as i32,
                    bounds.y.floor() as i32,
                    bounds.width.ceil() as i32,
                    bounds.height.ceil() as i32,
                );
            }),
        })
    }

    pub(in crate::ui) fn bar_duration(&self, millis: u64) -> Duration {
        if self.cfg.general.transition == crate::config::Transition::None {
            Duration::ZERO
        } else {
            // Durée du thème, fondu court en mouvement réduit (MLG-271).
            crate::ui::tokens::duration(millis)
        }
    }

    pub(in crate::ui) fn bar_overflow_items(&self, id: Id) -> Vec<BarItem> {
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
            // Les menus du shell ne débordent jamais (HIG MENU-01).
            BarItem::Menus => crate::ui::plugins::bar::layout::PINNED,
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
    pub(in crate::ui) fn sync_bars(&mut self) -> Task<Msg> {
        // `"primary"` dans la liste : l'écran principal, quel que soit son nom.
        let primary = self.primary_monitor();
        let want: Vec<String> = match &self.cfg.bar {
            // Paquet « Barre » coupé : aucune barre.
            _ if !self.cfg.plugin_on("bar") => Vec::new(),
            None => Vec::new(),
            Some(b) => self
                .monitors
                .iter()
                // Un écran Panorama n'a pas de barre (MLG-164).
                .filter(|m| !self.is_panorama(&m.name))
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
            .map(|b| (b.position, b.height, b.margin, b.shape));
        let mut tasks = Vec::new();
        let mut changed = false;
        for (id, s) in self.bar_surfaces.clone() {
            let height = spec.map(|(_, h, _, _)| self.bar_height_on(&s.monitor, h));
            // Plein écran venu ou parti : le calque change, la surface est
            // recréée (HIG PAT-03).
            if !(want.contains(&s.monitor)
                && spec.map(|(p, _, m, sh)| (p, height.unwrap_or(0), m, sh))
                    == Some((s.position, s.height, s.margin, s.shape))
                && self.bar_over_fullscreen.contains(&id) == self.bar_covered(&s.monitor, &s.shape))
            {
                self.bar_surfaces.remove(&id);
                self.bar_over_fullscreen.remove(&id);
                tasks.push(iced::window::close(id));
                changed = true;
            }
        }
        if let Some((position, base_height, margin, shape)) = spec {
            for mon in want {
                if self.bar_surfaces.values().any(|s| s.monitor == mon) {
                    continue;
                }
                // Hauteur propre à cet écran (MLG-281).
                let height = self.bar_height_on(&mon, base_height);
                let id = Id::unique();
                let vertical = position.vertical();
                let edge = match position {
                    BarPosition::Top => LsAnchor::Top,
                    BarPosition::Bottom => LsAnchor::Bottom,
                    BarPosition::Left => LsAnchor::Left,
                    BarPosition::Right => LsAnchor::Right,
                };
                // Le long du bord : gauche/droite pour une barre horizontale,
                // haut/bas pour une verticale.
                let (along_start, along_end) = if vertical {
                    (LsAnchor::Top, LsAnchor::Bottom)
                } else {
                    (LsAnchor::Left, LsAnchor::Right)
                };
                // Forme (MLG-277) : longueur, alignement, marges latérales,
                // place laissée par les fenêtres, couche.
                let screen = self
                    .monitors
                    .iter()
                    .find(|m| m.name == mon)
                    .map_or(1920.0, |m| if vertical { m.height } else { m.width } as f32);
                let content = self.bar_full_width.values().max().map(|w| *w as f32 + 16.0);
                let full = shape.length == crate::config::BarLength::Full;
                let width = if full {
                    0
                } else {
                    shape.length_on(screen, content).round() as u32
                };
                let side = shape.side_margin as i32;
                let anchor = if full {
                    edge | along_start | along_end
                } else {
                    match shape.alignment {
                        crate::config::BarAlign::Start => edge | along_start,
                        crate::config::BarAlign::Center => edge,
                        crate::config::BarAlign::End => edge | along_end,
                    }
                };
                // Par-dessus un plein écran : calque `Overlay`, sans zone
                // réservée, réduite d'emblée à sa bande de rappel (HIG PAT-03).
                let cover = self.bar_covered(&mon, &shape);
                let autohide = shape.visibility == crate::config::BarVisibility::AutoHide;
                let thick = if cover {
                    shape.reveal_zone.max(1)
                } else {
                    height
                };
                if cover {
                    self.bar_over_fullscreen.insert(id);
                    self.bar_reveal.insert(id, (false, 0));
                }
                self.bar_surfaces.insert(
                    id,
                    BarSurface {
                        monitor: mon.clone(),
                        position,
                        height,
                        margin,
                        shape,
                        width,
                    },
                );
                tasks.push(Task::done(Msg::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some(if vertical {
                            (thick as u32, width)
                        } else {
                            (width, thick as u32)
                        }),
                        layer: match shape.layer {
                            _ if cover => Layer::Overlay,
                            crate::config::BarLayer::Top => Layer::Top,
                            crate::config::BarLayer::Overlay => Layer::Overlay,
                            crate::config::BarLayer::Bottom => Layer::Bottom,
                        },
                        anchor,
                        // La zone réservée aux fenêtres compte la hauteur et la
                        // marge : la lame flotte au-dessus du bord. Masquée
                        // hors survol ou sans réserve : les fenêtres passent
                        // dessous.
                        exclusive_zone: Some(if shape.reserve_space && !autohide && !cover {
                            height as i32
                        } else {
                            0
                        }),
                        margin: Some(match position {
                            BarPosition::Top => (margin as i32, side, 0, side),
                            BarPosition::Bottom => (0, side, margin as i32, side),
                            BarPosition::Left => (side, 0, side, margin as i32),
                            BarPosition::Right => (side, margin as i32, side, 0),
                        }),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        output_option: OutputOption::OutputName(mon),
                        namespace: Some(BAR_NAMESPACE.into()),
                        ..Default::default()
                    },
                    id,
                }));
                if autohide && !cover {
                    // Montrée à l'apparition, masquée après le délai.
                    tasks.push(Task::done(Msg::BarReveal(id, false)));
                }
                changed = true;
            }
            for name in STATIC_ICONS {
                tasks.push(self.want_icon(name));
            }
        }
        tasks.push(self.sync_convex_frames());
        if self
            .cfg
            .bar
            .as_ref()
            .is_none_or(|b| b.style != crate::config::BarStyle::Island)
        {
            tasks.extend(self.bar_surfaces.keys().map(|id| self.bar_input(*id, None)));
        }
        if changed {
            self.bar_anchors
                .retain(|(id, _), _| self.bar_surfaces.contains_key(id));
            self.bar_hidden
                .retain(|id, _| self.bar_surfaces.contains_key(id));
            // Le dock partage la bande de rappel (HIG PAT-01).
            self.bar_reveal.retain(|id, _| {
                self.bar_surfaces.contains_key(id) || self.launcher_surfaces.contains_key(id)
            });
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
    pub(in crate::ui) fn bar_widget_ids(&self) -> Vec<String> {
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

    pub(in crate::ui) fn bar_has(&self, item: &BarItem) -> bool {
        if !self.cfg.plugin_on("bar") || self.bar_surfaces.is_empty() {
            return false;
        }
        self.cfg
            .bar
            .iter()
            .flat_map(|b| b.zones.iter().chain(b.portrait.iter().flatten()).flatten())
            .any(|i| i == item)
    }

    pub(in crate::ui) fn volume_action(&self, args: Vec<String>) -> Task<Msg> {
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
    pub(in crate::ui) fn mic_action(&self, args: Vec<String>) -> Task<Msg> {
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

    pub(in crate::ui) fn bar_palette(&self) -> BarPalette {
        let mut pal = BarPalette::resolve(
            &self.cfg.general.theme,
            self.cfg.bar.as_ref().map(|bar| &bar.colors),
        );
        // Transparence adaptative (MLG-261) : le verre se densifie sur un
        // fond qui gênerait la lecture — la plus forte des barres, pour un
        // même verre partout. Une opacité réglée pour la barre l'emporte.
        let fixed = self
            .cfg
            .bar
            .as_ref()
            .is_some_and(|b| b.colors.opacity.is_some());
        if !fixed
            && self
                .cfg
                .general
                .theme
                .shown(crate::config::Place::Bar)
                .adaptive
                .is_some()
        {
            let base = pal.opacity;
            pal.opacity = self
                .bar_surfaces
                .values()
                .map(|s| {
                    self.adaptive_opacity(
                        crate::config::Place::Bar,
                        base,
                        &s.monitor,
                        self.bar_roi(s),
                    )
                })
                .fold(base, f32::max);
        }
        // Barre presque transparente (matériau `transparent`, MLG-282) : le
        // texte suit le fond local, clair sur fond sombre et inversement,
        // sauf couleur de texte réglée pour la barre.
        let own_text = self
            .cfg
            .bar
            .as_ref()
            .is_some_and(|b| b.colors.text.is_some());
        if pal.opacity < 0.3 && !own_text && !self.bar_surfaces.is_empty() {
            let bright = self
                .bar_surfaces
                .values()
                .map(|s| {
                    let (lo, hi) = self.wall_range(&s.monitor, self.bar_roi(s));
                    (lo + hi) / 2.0
                })
                .fold(0.0, f32::max);
            let light_wall = bright > 0.45;
            let fg_dark = crate::theme_engine::color::luminance(pal.fg) < 0.4;
            if light_wall != fg_dark {
                pal.fg = if light_wall {
                    Rgba(0x12, 0x14, 0x18, 0xff)
                } else {
                    Rgba(0xf4, 0xf6, 0xfb, 0xff)
                };
            }
        }
        pal
    }

    /// Barre verticale (bord gauche ou droit, MLG-277) : les trois zones
    /// en colonne (début, centre, fin), les espaces de travail empilés.
    fn view_bar_vertical<'a>(&'a self, id: Id, s: &'a BarSurface) -> Element<'a, Msg> {
        let Some(bar) = self.cfg.bar.as_ref() else {
            return Space::new().into();
        };
        let pal = self.bar_palette();
        let shape = bar.shape;
        let gap = shape.item_gap.map_or(6.0, f32::from);
        let fg = color(pal.fg, 0.92);
        let accent = color(pal.accent, 1.0);
        let item = |item: &'a BarItem| -> Element<'a, Msg> {
            if *item == BarItem::Workspaces {
                let ctx = crate::ui::plugins::bar::style::ws_ctx(self, s);
                return iced::widget::Column::with_children(
                    ctx.rows
                        .into_iter()
                        .map(|w| {
                            let active = w.name == ctx.current;
                            let label = truncate(&w.name, 3);
                            button(
                                container(
                                    text(label)
                                        .size(crate::ui::tokens::ts_label(12.0))
                                        .font(self.ui_font(iced::font::Weight::Semibold))
                                        .color(if active { color(pal.bg, 1.0) } else { fg }),
                                )
                                .center_x(Length::Fill),
                            )
                            .padding([4, 0])
                            .width(Length::Fill)
                            .on_press(Msg::BarWorkspace(ctx.monitor.clone(), w.name.clone()))
                            .style(move |_, st| {
                                let mut b = crate::ui::tokens::item(accent, fg, false, st, 8.0);
                                if active {
                                    b.background = Some(accent.into());
                                } else if w.windows > 0 && b.background.is_none() {
                                    b.background = Some(Color { a: 0.10, ..fg }.into());
                                }
                                b
                            })
                            .into()
                        })
                        .collect::<Vec<Element<'a, Msg>>>(),
                )
                .spacing(4)
                .width(Length::Fill)
                .into();
            }
            container(self.bar_item(id, s, item))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .clip(true)
                .into()
        };
        let zone = |items: &'a [BarItem]| {
            iced::widget::Column::with_children(items.iter().map(item).collect::<Vec<_>>())
                .spacing(gap)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        };
        let pad = shape.padding.map_or(8.0, f32::from);
        let content = iced::widget::Column::with_children(vec![
            zone(&bar.zones[0]).into(),
            Space::new().height(Length::Fill).into(),
            zone(&bar.zones[1]).into(),
            Space::new().height(Length::Fill).into(),
            zone(&bar.zones[2]).into(),
        ])
        .padding(Padding {
            top: pad + 4.0,
            bottom: pad + 4.0,
            left: 4.0,
            right: 4.0,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center);
        // Coins arrondis côté fenêtres ; tous si la barre flotte.
        let r = shape.radius.map_or(11.0, f32::from);
        let floating = s.margin > 0 || shape.side_margin > 0 || s.width > 0;
        let radius = if floating {
            iced::border::radius(r)
        } else if s.position == BarPosition::Left {
            iced::border::right(r)
        } else {
            iced::border::left(r)
        };
        let mut surface = crate::ui::plugins::bar::style::surface(bar.style, &pal, true);
        surface.border.radius = radius;
        let bg = match surface.background {
            Some(iced::Background::Color(c)) => c,
            _ => color(pal.bg, pal.opacity),
        };
        let under = crate::ui::lens::available()
            .then(|| {
                let roi = self.bar_roi(s);
                self.bare_backdrop(&s.monitor, roi)
            })
            .flatten();
        let el = crate::ui::glass::card(
            content,
            crate::config::Place::Bar,
            radius,
            color(pal.fg, 1.0),
            bg,
            surface,
            true,
            under,
        );
        if self.bar_hides(id, &shape) {
            mouse_area(el)
                .on_enter(Msg::BarReveal(id, true))
                .on_exit(Msg::BarReveal(id, false))
                .into()
        } else {
            el
        }
    }

    /// Hauteur de la barre sur un écran (MLG-281) : la sienne réglée, sinon
    /// celle du pack pour cet écran, sinon `base`.
    pub(in crate::ui) fn bar_height_on(&self, monitor: &str, base: u16) -> u16 {
        let size = self
            .monitors
            .iter()
            .find(|m| m.name == monitor)
            .map_or((0, 0), |m| (m.width, m.height));
        crate::design_options::screen_num(&self.cfg.general, monitor, size, "bar_height")
            .map(|v| v.clamp(24.0, 64.0) as u16)
            .or_else(|| {
                crate::ui::tokens::design()
                    .screen(monitor, Some(size))
                    .and_then(|s| s.bar_height)
            })
            .unwrap_or(base)
    }

    /// Barre masquée hors survol : réglée ainsi (MLG-277), ou par-dessus un
    /// plein écran (HIG PAT-03).
    pub(in crate::ui) fn bar_hides(&self, id: Id, shape: &crate::config::BarShape) -> bool {
        shape.visibility == crate::config::BarVisibility::AutoHide
            || self.bar_over_fullscreen.contains(&id)
    }

    /// La barre de l'écran `monitor` passe par-dessus un plein écran
    /// (HIG PAT-03).
    fn bar_covered(&self, monitor: &str, shape: &crate::config::BarShape) -> bool {
        bar_covers(
            shape.layer,
            self.convex_auto_hide(),
            self.fullscreen_on(monitor),
        )
    }

    /// Une barre d'écran n'est plus sur le bon calque : plein écran venu ou
    /// parti sur son espace affiché (HIG PAT-03).
    pub(in crate::ui) fn bar_cover_changed(&self) -> bool {
        self.bar_surfaces.iter().any(|(id, s)| {
            self.bar_over_fullscreen.contains(id) != self.bar_covered(&s.monitor, &s.shape)
        })
    }

    /// Dock masqué hors survol (`dock.autohide`, MLG-279), hors mode convexe
    /// qui a le sien.
    pub(in crate::ui) fn dock_autohide(&self) -> bool {
        !self.convex_auto_hide() && crate::design_options::flag(&self.cfg.general, "dock.autohide")
    }

    /// Barre visible : toujours, ou montrée par le survol, ou tenue par son
    /// menu ouvert (MLG-277).
    pub(in crate::ui) fn bar_revealed(&self, id: Id) -> bool {
        self.bar_reveal.get(&id).is_none_or(|r| r.0)
            || self.popover.as_ref().is_some_and(|p| p.bar == id)
    }

    /// Pointeur entré sur la barre ou sorti : la montrer ou la masquer
    /// après le délai réglé, si rien n'a changé entre-temps.
    pub(in crate::ui) fn bar_hover(&mut self, id: Id, hover: bool) -> Task<Msg> {
        // Le dock masqué hors survol suit le même geste (MLG-279).
        let delay = if self.launcher_surfaces.contains_key(&id) && self.dock_hides(id) {
            if hover { 120 } else { 600 }
        } else {
            let Some(s) = self.bar_surfaces.get(&id) else {
                return Task::none();
            };
            if !self.bar_hides(id, &s.shape) {
                return Task::none();
            }
            u64::from(if hover {
                s.shape.show_delay
            } else {
                s.shape.hide_delay
            })
        };
        let e = self.bar_reveal.entry(id).or_insert((true, 0));
        e.1 += 1;
        let generation = e.1;
        Task::perform(
            tokio::time::sleep(Duration::from_millis(delay)),
            move |_| Msg::BarRevealApply(id, generation, hover),
        )
    }

    pub(in crate::ui) fn bar_reveal_apply(
        &mut self,
        id: Id,
        generation: u64,
        show: bool,
    ) -> Task<Msg> {
        if let Some(d) = self.launcher_surfaces.get(&id).cloned() {
            let Some(e) = self.bar_reveal.get_mut(&id) else {
                return Task::none();
            };
            if e.1 != generation || e.0 == show {
                return Task::none();
            }
            e.0 = show;
            let th = if show {
                d.thickness
            } else {
                crate::ui::plugins::dock::REVEAL_STRIP
            };
            let vertical = matches!(
                d.position,
                crate::config::LauncherPosition::Left | crate::config::LauncherPosition::Right
            );
            return Task::done(Msg::SizeChange {
                id,
                size: if vertical { (th, 0) } else { (0, th) },
            });
        }
        let Some(s) = self.bar_surfaces.get(&id).cloned() else {
            return Task::none();
        };
        let menu = self.popover.as_ref().is_some_and(|p| p.bar == id);
        let Some(e) = self.bar_reveal.get_mut(&id) else {
            return Task::none();
        };
        if e.1 != generation || (!show && menu) || e.0 == show {
            return Task::none();
        }
        e.0 = show;
        let h = if show {
            u32::from(s.height)
        } else {
            u32::from(s.shape.reveal_zone)
        };
        Task::done(Msg::SizeChange {
            id,
            size: if s.position.vertical() {
                (h, s.width)
            } else {
                (s.width, h)
            },
        })
    }

    /// Barre « selon le contenu » : sa largeur suit celle de ses modules
    /// une fois mesurée (MLG-277).
    pub(in crate::ui) fn bar_fit_content(&mut self, id: Id, full: u32) -> Task<Msg> {
        let Some(s) = self.bar_surfaces.get(&id).cloned() else {
            return Task::none();
        };
        if s.shape.length != crate::config::BarLength::Content || s.position.vertical() {
            return Task::none();
        }
        let screen = self.monitor_size(&s.monitor).width;
        let w = s.shape.length_on(screen, Some(full as f32 + 16.0)).round() as u32;
        if w.abs_diff(s.width) < 4 {
            return Task::none();
        }
        if let Some(b) = self.bar_surfaces.get_mut(&id) {
            b.width = w;
        }
        let h = if self.bar_revealed(id) {
            u32::from(s.height)
        } else {
            u32::from(s.shape.reveal_zone)
        };
        Task::done(Msg::SizeChange {
            id,
            size: if s.position.vertical() {
                (h, w)
            } else {
                (w, h)
            },
        })
    }

    /// Rectangle de la barre sur son écran (pour lire le fond derrière).
    pub(in crate::ui) fn bar_roi(&self, s: &BarSurface) -> iced::Rectangle {
        let screen = self.monitor_size(&s.monitor);
        let h = f32::from(s.height);
        // Barre verticale (MLG-277) : `height` est sa largeur.
        if s.position.vertical() {
            let side = f32::from(s.shape.side_margin);
            let x = if s.position == BarPosition::Left {
                f32::from(s.margin)
            } else {
                (screen.width - h - f32::from(s.margin)).max(0.0)
            };
            let len = if s.width == 0 {
                (screen.height - 2.0 * side).max(1.0)
            } else {
                s.width as f32
            };
            let y = match s.shape.alignment {
                crate::config::BarAlign::Start => side,
                crate::config::BarAlign::Center => (screen.height - len) / 2.0,
                crate::config::BarAlign::End => screen.height - len - side,
            };
            let y = if s.width == 0 { side } else { y };
            return iced::Rectangle::new(iced::Point::new(x, y), iced::Size::new(h, len));
        }
        let y = match s.position {
            BarPosition::Bottom => (screen.height - h - f32::from(s.margin)).max(0.0),
            _ => f32::from(s.margin),
        };
        // Forme (MLG-277) : longueur, alignement, marges latérales.
        let side = f32::from(s.shape.side_margin);
        let (x, w) = if s.width == 0 {
            (side, (screen.width - 2.0 * side).max(1.0))
        } else {
            let w = s.width as f32;
            let x = match s.shape.alignment {
                crate::config::BarAlign::Start => side,
                crate::config::BarAlign::Center => (screen.width - w) / 2.0,
                crate::config::BarAlign::End => screen.width - w - side,
            };
            (x, w)
        };
        iced::Rectangle::new(iced::Point::new(x, y), iced::Size::new(w, h))
    }

    pub(in crate::ui) fn bar_item_is_launcher(&self, item: &BarItem) -> bool {
        matches!(item, BarItem::Widget(id) if self.cfg.widgets.iter().any(|w| &w.id == id && matches!(w.kind, WidgetKind::Launcher)))
    }

    /// Couleur propre d'un module (par son identifiant, puis par son type).
    pub(in crate::ui) fn bar_item_color(&self, item: &BarItem) -> Option<Rgba> {
        let m = &self.cfg.bar.as_ref()?.colors.modules;
        let spec = match item {
            BarItem::Widget(id) => m.get(id).or_else(|| {
                let w = self.cfg.widgets.iter().find(|w| &w.id == id)?;
                m.get(&w.type_id)
            }),
            other => m.get(bar_item_key(other)),
        }?;
        resolve_color(spec, light_theme(&self.cfg.general.theme))
    }

    pub(in crate::ui) fn state_color(&self, s: RenderState, normal: Color) -> Color {
        let p = self.bar_palette();
        match s {
            RenderState::Warning => color(p.warning, 1.0),
            RenderState::Critical => color(p.critical, 1.0),
            RenderState::Active => color(p.accent, 1.0),
            _ => normal,
        }
    }

    pub(in crate::ui) fn view_bar(&self, id: Id) -> Element<'_, Msg> {
        let (Some(s), Some(bar)) = (self.bar_surfaces.get(&id), &self.cfg.bar) else {
            return Space::new().into();
        };
        let pal = self.bar_palette();
        let style = bar.style;
        let continuity = style == crate::config::BarStyle::Continuite;
        // Forme (MLG-277) : espacement des modules.
        let shape = bar.shape;
        crate::ui::plugins::bar::layout::set_gap(shape.item_gap.map_or(2.0, f32::from));
        // Masquée hors survol : une fine bande au bord la fait revenir.
        if self.bar_hides(id, &shape) && !self.bar_revealed(id) {
            return mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_enter(Msg::BarReveal(id, true))
                .into();
        }
        if s.position.vertical() {
            return self.view_bar_vertical(id, s);
        }
        let t = crate::ui::plugins::bar::style::durations(self);
        // Écran vertical : ses propres zones, si la barre en déclare.
        let (zones, zone_offset) = match (&bar.portrait, self.bar_vertical(&s.monitor)) {
            (Some(p), true) => (p, 3),
            _ => (&bar.zones, 0),
        };
        // Chaque zone est rognée : sur un écran étroit, elles ne se chevauchent pas.
        // Réglages ouverts sur la page Barre : les modules se déplacent en les
        // glissant (zones ordinaires seulement ; les écrans verticaux gardent
        // les leurs). Sur les autres pages, la barre garde son apparence.
        let edit = self.panel.as_ref().is_some_and(|p| p.tab == "bar");
        let graphite = style == crate::config::BarStyle::Graphite;
        let content: Element<'_, Msg> = if !edit
            || graphite
            || matches!(
                style,
                crate::config::BarStyle::Convexe | crate::config::BarStyle::Island
            ) {
            // Aperçu d'un module : son rang parmi les modules, ceux du bandeau.
            let preview = self.bar_preview;
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
                .map(
                    |(index, (zone, i, item))| crate::ui::plugins::bar::layout::Slot {
                        zone,
                        priority: self.bar_priority(item),
                        flexible: self.bar_item_is_launcher(item),
                        content: {
                            let content = self.bar_item(id, s, item);
                            let content = if zone == 1
                                && let Some(p) = self.popover.as_ref().filter(|p| p.bar == id)
                                && let Some(a) = p.attached.filter(|a| {
                                    matches!(
                                        a.edge,
                                        crate::ui::plugins::bar::style::convexe::Edge::Top
                                            | crate::ui::plugins::bar::style::convexe::Edge::Bottom
                                    )
                                }) {
                                let progress = p.visibility.value(std::time::Instant::now());
                                let dy = if s.position == BarPosition::Top {
                                    -1.0
                                } else {
                                    1.0
                                };
                                motion::inert(motion::surface(
                                    content,
                                    1.0,
                                    iced::Vector::new(0.0, dy * a.bar_height * progress),
                                    iced::Point::ORIGIN,
                                ))
                            } else {
                                content
                            };
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
                            let content = if preview == Some((index, true)) {
                                motion::surface(
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
                                motion::anchor(
                                    content,
                                    id,
                                    bar_item_key(item).into(),
                                    self.bar_duration(t.hover),
                                )
                            }
                        },
                    },
                )
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
                    slots.push(crate::ui::plugins::bar::layout::Slot {
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
                            BarItem::Menus
                                | BarItem::Workspaces
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
            if style == crate::config::BarStyle::Island {
                eligible.fill(false);
            }
            let more_open = self.bar_menu_open(id, "overflow");
            let more_fg = if continuity && more_open {
                crate::ui::plugins::bar::style::continuite::selected_ink(&pal, 1.0)
            } else {
                color(pal.fg, 1.0)
            };
            let mut more_content = row![].spacing(5).align_y(Alignment::Center);
            if self.bar_overflow_items(id).contains(&BarItem::Workspaces)
                && let Some(m) = self.monitors.iter().find(|m| m.name == s.monitor)
            {
                more_content = more_content.push(self.label(
                    truncate(&m.workspace, 8),
                    13.0,
                    if more_open && continuity {
                        more_fg
                    } else {
                        crate::ui::plugins::bar::style::ink(style, &pal)
                    },
                ));
            }
            more_content = more_content.push(
                self.icon_named("view-more-symbolic", 18.0, more_fg)
                    .unwrap_or_else(|| self.label("…", 18.0, more_fg)),
            );
            let more = motion::anchor(
                self.module(
                    s,
                    1.0,
                    more_content.into(),
                    Some(Msg::OpenPopover(id, "overflow".into(), Request::Overflow)),
                    None,
                    None,
                    None,
                    Some(crate::i18n::tr("Autres commandes").into()),
                    more_open,
                ),
                id,
                "overflow".into(),
                self.bar_duration(t.hover),
            );
            motion::rail(
                crate::ui::plugins::bar::layout::strip(
                    id,
                    slots,
                    more,
                    self.bar_compact(&s.monitor),
                    style == crate::config::BarStyle::Graphite,
                    crate::ui::plugins::bar::style::zone_pill(style, &pal).map(|mut skin| {
                        skin.pinned = edit
                            || self.bar_preview.is_some()
                            || self.popover.as_ref().is_some_and(|p| p.bar == id);
                        skin.notification = !self.toasts.list.is_empty()
                            && zones[1].iter().any(|item| {
                                matches!(item,
                                    BarItem::Widget(id) if self.cfg.widgets.iter().any(|w|
                                        &w.id == id && matches!(w.kind, WidgetKind::Clock { .. })))
                            });
                        skin.duration = self.bar_duration(180);
                        skin
                    }),
                    continuity,
                ),
                preview
                    .filter(|_| style != crate::config::BarStyle::Island)
                    .map(|(i, _)| i),
                eligible,
                crate::ui::plugins::bar::style::rail_look(style, &pal),
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
        match style {
            crate::config::BarStyle::Graphite => pad.top = 1.0,
            // Pilules : retraits horizontaux pour leur débord de verre ; la
            // fenêtre reste transparente au-dessus et en-dessous d'elles.
            crate::config::BarStyle::Island => {
                pad.left = 12.0;
                pad.right = 12.0;
            }
            // Convex : bord à bord, les niches partent des bords de l'écran.
            crate::config::BarStyle::Convexe => {}
            crate::config::BarStyle::Continuite => {
                let inset =
                    crate::ui::plugins::bar::style::continuite::vertical_padding(s.height as f32);
                let side = shape.padding.map_or(8.0, f32::from);
                pad.top = inset;
                pad.bottom = inset;
                pad.left = side;
                pad.right = side;
            }
        }
        // Fond de zones à path (Convex) : canvas sous le contenu, aux
        // étendues mesurées des zones. Continuité ne peint ses groupes que
        // détachés : en lame continue, la barre reste d'un seul tenant, comme
        // celle de macOS, sans îlot sous ses zones.
        let underlay = (!continuity || (shape.groups && !edit))
            .then(|| {
                crate::ui::plugins::bar::style::zone_underlay(
                    style,
                    &pal,
                    s.height as f32,
                    self.bar_zone_extents.get(&id).copied().unwrap_or_default(),
                    top,
                )
            })
            .flatten();
        let content = match underlay {
            Some(underlay) => iced::widget::stack([underlay, content]).into(),
            None => content,
        };
        // Verre du thème (MLG-268) sur les lames pleines ; les capsules et
        // les niches dessinent leur propre verre. Vraie réfraction (MLG-275)
        // : la lame ne voit pas le fond derrière elle, la tranche de fond de
        // son écran est peinte dessous pour la lentille.
        // Groupes détachés (MLG-277) : pas de lame, les groupes seuls.
        let detached = continuity && shape.groups;
        let slab: Element<'_, Msg> = if matches!(
            style,
            crate::config::BarStyle::Continuite | crate::config::BarStyle::Graphite
        ) && !detached
        {
            // Continuité arrondit les coins côté fenêtres ; une barre
            // flottante (décollée du bord ou plus courte que lui) les quatre.
            let r = shape
                .radius
                .map_or(if continuity { 11.0 } else { 0.0 }, f32::from);
            let floating = s.margin > 0 || shape.side_margin > 0 || s.width > 0;
            let radius = if floating {
                iced::border::radius(shape.radius.map_or(11.0, f32::from))
            } else {
                match (style, top) {
                    (crate::config::BarStyle::Continuite, true) => iced::border::bottom(r),
                    (crate::config::BarStyle::Continuite, false) => iced::border::top(r),
                    _ => iced::border::radius(r),
                }
            };
            let mut surface = crate::ui::plugins::bar::style::surface(style, &pal, top);
            surface.border.radius = radius;
            let bg = match surface.background {
                Some(iced::Background::Color(c)) => c,
                _ => color(pal.bg, pal.opacity),
            };
            let under = crate::ui::lens::available()
                .then(|| {
                    let roi = self.bar_roi(s);
                    self.bare_backdrop(&s.monitor, roi)
                })
                .flatten();
            crate::ui::glass::card(
                container(content)
                    .padding(pad)
                    .width(Length::Fill)
                    .height(Length::Fill),
                crate::config::Place::Bar,
                radius,
                color(pal.fg, 1.0),
                bg,
                surface,
                true,
                under,
            )
        } else {
            container(content)
                .padding(pad)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| {
                    let mut st = crate::ui::plugins::bar::style::surface(style, &pal, top);
                    if detached {
                        st.background = None;
                    }
                    st
                })
                .into()
        };
        let bar_el: Element<'_, Msg> = if graphite {
            container(slab)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(color(pal.fg, 0.12).into()),
                    ..Default::default()
                })
                .into()
        } else {
            slab
        };
        // Masquée hors survol (MLG-277) : le survol la garde, la sortie la
        // masque après le délai.
        if self.bar_hides(id, &shape) {
            mouse_area(bar_el)
                .on_enter(Msg::BarReveal(id, true))
                .on_exit(Msg::BarReveal(id, false))
                .into()
        } else {
            bar_el
        }
    }

    /// Un espace de cet écran porte-t-il une fenêtre urgente ? Hyprland
    /// l'annonce par `urgent>>`, on la retrouve dans la liste des fenêtres.
    pub(in crate::ui) fn bar_urgent(&self, monitor: &str, ws: &str) -> bool {
        !self.urgent.is_empty()
            && self.clients.iter().any(|c| {
                self.urgent.contains(&c.address) && c.workspace == ws && c.monitor == monitor
            })
    }

    /// Une bulle de notification est-elle affichée pour une fenêtre de cet
    /// espace ? L'application d'origine (`desktop-entry`, sinon `app_name`)
    /// est rapprochée de la classe d'une fenêtre ; l'indicateur suit la vie
    /// de la bulle (fermée, expirée : l'espace redevient normal).
    pub(in crate::ui) fn bar_notif(&self, monitor: &str, ws: &str) -> bool {
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
    pub(in crate::ui) fn bar_vertical(&self, monitor: &str) -> bool {
        self.monitors
            .iter()
            .find(|m| m.name == monitor)
            .is_some_and(|m| m.height > m.width)
    }

    /// Barre compacte sur cet écran : réglage, ou automatique (étroit ou vertical).
    pub(in crate::ui) fn bar_compact(&self, monitor: &str) -> bool {
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
                motion::anchor(
                    self.bar_item(id, s, it),
                    id,
                    bar_item_key(it).into(),
                    self.bar_duration(140),
                )
            }))
            .spacing(if compact { 0 } else { 2 })
            .align_y(Alignment::Center);
            return motion::rail(
                content,
                None,
                items
                    .iter()
                    .map(|it| {
                        !matches!(
                            it,
                            BarItem::Menus
                                | BarItem::Workspaces
                                | BarItem::Window
                                | BarItem::Desktop
                                | BarItem::Tray
                        )
                    })
                    .collect(),
                crate::ui::plugins::bar::style::rail_look(
                    self.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default(),
                    &self.bar_palette(),
                ),
                self.bar_duration(crate::ui::plugins::bar::style::durations(self).hover),
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
        let row = r.push(
            mouse_area(
                Space::new()
                    .width(if items.is_empty() { 48 } else { 16 })
                    .height(h),
            )
            .on_enter(Msg::BarDragOver(z, items.len()))
            .on_release(Msg::BarDragEnd),
        );
        // Pas d'îlot sous la zone : la lame reste d'un seul tenant, les cadres
        // des modules suffisent à dire qu'on peut les déplacer.
        container(row)
            .padding([0, 3])
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    }

    /// Ovale au compte `count` (HIG SYS-02) accolé au cadre `frame` d'un
    /// module : non-lues des notifications, paquets à mettre à jour… Il se
    /// pose après le contenu (sous lui dans une barre verticale), sur la marge
    /// du cadre et au-delà, à ras du bord de l'écran : l'icône reste entière,
    /// le module s'élargit de ce qui dépasse. `icon` : taille de l'icône du
    /// module, pour la marge d'une barre verticale. À zéro, le cadre seul.
    fn with_badge<'a>(
        &self,
        frame: Element<'a, Msg>,
        bar: &BarSurface,
        alpha: f32,
        count: usize,
        icon: f32,
    ) -> Element<'a, Msg> {
        let Some(badge) = self.count_badge(count, alpha) else {
            return frame;
        };
        let (x, y) = Self::badge_corner(bar.position);
        let thickness = bar.height as f32;
        // L'ovale empiète sur la marge du cadre, jamais sur son contenu.
        let margin = if bar.position.vertical() {
            ((module_target(bar.height) - icon) / 2.0).max(0.0)
        } else {
            MODULE_PAD_X
        };
        let spacing = -(margin - BADGE_GAP).max(0.0);
        if bar.position.vertical() {
            let base = container(frame)
                .width(Length::Fixed(thickness))
                .align_x(Alignment::Center);
            let over = container(badge).width(Length::Fill).align_x(x);
            iced::widget::column![base, over].spacing(spacing).into()
        } else {
            let base = container(frame)
                .height(Length::Fixed(thickness))
                .align_y(Alignment::Center);
            let over = container(badge).height(Length::Fill).align_y(y);
            iced::widget::row![base, over].spacing(spacing).into()
        }
    }

    /// Coin de l'ovale des non-lues sur le cadre d'un module (HIG SYS-02) :
    /// celui qui touche le bord de l'écran où la barre est posée, côté fin
    /// (bas d'une barre verticale).
    fn badge_corner(position: BarPosition) -> (Alignment, Alignment) {
        match position {
            BarPosition::Top => (Alignment::End, Alignment::Start),
            BarPosition::Bottom => (Alignment::End, Alignment::End),
            BarPosition::Left => (Alignment::Start, Alignment::End),
            BarPosition::Right => (Alignment::End, Alignment::End),
        }
    }

    /// Chiffre de l'ovale des non-lues (HIG SYS-02) : « 99+ » au-delà de 99.
    fn badge_label(unread: usize) -> String {
        if unread > 99 {
            "99+".to_string()
        } else {
            unread.to_string()
        }
    }

    /// Ovale d'un compte, posé sur le cadre d'un module (HIG SYS-02) : petit
    /// ovale plein au chiffre, au ton « attention » de la barre comme la bulle
    /// critique (HIG FON-02) — l'accent reste à la sélection et aux actions.
    /// `None` à zéro.
    fn count_badge<'a>(&self, unread: usize, alpha: f32) -> Option<Element<'a, Msg>> {
        if unread == 0 {
            return None;
        }
        let pal = self.bar_palette();
        // Encre du thème qui reste lisible sur ce ton, clair ou sombre.
        let ink = design::on_accent(
            color(pal.warning, 1.0),
            color(pal.fg, 1.0),
            color(pal.bg, 1.0),
        );
        let warning = color(pal.warning, alpha);
        let ink = Color { a: alpha, ..ink };
        // 18 px au moins ; grandit avec son chiffre (échelle du texte).
        let h = 18.0_f32.max(crate::ui::tokens::ts_label(11.0) + 7.0);
        Some(
            container(self.label_numeric(Self::badge_label(unread), 11.0, ink))
                .padding([0, 6])
                .height(h)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .style(move |_| container::Style {
                    background: Some(Background::Color(warning)),
                    border: Border::default().rounded(h / 2.0),
                    text_color: Some(ink),
                    ..Default::default()
                })
                .into(),
        )
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
        let h = module_target(bar.height);
        let interactive = left.is_some() || right.is_some() || middle.is_some() || scroll.is_some();
        // La barre qui tient le clavier (HIG ENT-02) : ses modules activables,
        // ceux qu'ouvre un clic gauche, prennent l'anneau de focus, activé par
        // Entrée ou Espace. Les autres barres, et celle-ci au repos, n'en ont
        // pas : le focus des autres surfaces ne s'y arrête pas.
        let keyboard = self
            .bar_keyboard
            .and_then(|id| self.bar_surfaces.get(&id))
            .is_some_and(|s| s.monitor == bar.monitor);
        let activate = left
            .clone()
            .filter(|m| keyboard && matches!(m, Msg::OpenPopover(..)));
        let b = button(
            container(content)
                .height(Length::Fill)
                .align_y(Alignment::Center),
        )
        .height(h)
        .padding([0.0, MODULE_PAD_X])
        .on_press_maybe(left.or(interactive.then_some(Msg::Noop)))
        .style(move |_, st| {
            crate::ui::plugins::bar::style::module_button(style, &pal, alpha, active, st)
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
        let mut el: Element<'a, Msg> = m.into();
        if let Some(msg) = activate {
            let style = self.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default();
            el = crate::ui::focus::press(
                el,
                Some(msg),
                crate::ui::plugins::bar::style::ink(style, &pal),
            );
        }
        match tip {
            Some(t) if !t.is_empty() => {
                let bg = color(pal.bg, 0.95);
                let edge = Color { a: 0.12, ..fg };
                tooltip(
                    el,
                    container(
                        text(t)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
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

    /// Libellé de chiffres qui défilent (horloge) : chiffres tabulaires du
    /// thème (MLG-270).
    pub(in crate::ui) fn label_numeric<'a>(
        &self,
        s: impl Into<String>,
        size: f32,
        c: Color,
    ) -> Element<'a, Msg> {
        text(s.into())
            .size(crate::ui::tokens::ts_label(size))
            .color(c)
            .font(self.numeric_font(iced::font::Weight::Medium))
            .wrapping(iced::widget::text::Wrapping::None)
            .into()
    }

    pub(in crate::ui) fn label<'a>(
        &self,
        s: impl Into<String>,
        size: f32,
        c: Color,
    ) -> Element<'a, Msg> {
        text(s.into())
            .font(self.font)
            .size(crate::ui::tokens::ts_label(size))
            .color(c)
            .font(self.ui_font(LABEL_WEIGHT))
            .wrapping(iced::widget::text::Wrapping::None)
            .into()
    }

    pub(in crate::ui) fn bar_item<'a>(
        &'a self,
        bar_id: Id,
        bar: &'a BarSurface,
        item: &'a BarItem,
    ) -> Element<'a, Msg> {
        self.bar_item_alpha(bar_id, bar, item, 1.0)
    }

    /// Titres des menus du shell (HIG MENU-01), dans l'ordre de la barre :
    /// le contenu du module `menus`, qui ne déborde jamais.
    fn bar_menu_titles<'a>(&'a self, id: Id, s: &'a BarSurface) -> Vec<Element<'a, Msg>> {
        let pal = self.bar_palette();
        let style = self.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default();
        let t = crate::ui::plugins::bar::style::durations(self);
        let shape = self.cfg.bar.as_ref().map(|b| b.shape).unwrap_or_default();
        let size = shape
            .font_size
            .unwrap_or((s.height as f32 * 0.38).clamp(11.0, 18.0));
        super::menus::titles()
            .into_iter()
            .map(|(key, label, request)| {
                let open = self.bar_menu_open(id, key);
                let fg = if open && style == crate::config::BarStyle::Continuite {
                    crate::ui::plugins::bar::style::continuite::selected_ink(&pal, 1.0)
                } else {
                    color(pal.fg, 0.92)
                };
                let actionable = self.bar_title_live(&request);
                let title = text(truncate(label, 16))
                    .wrapping(iced::widget::text::Wrapping::None)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(size))
                    .color(fg)
                    // Titres de menus : semibold (DESIGN.md, HIG MENU-07).
                    .font(self.ui_font(iced::font::Weight::Semibold));
                // Le menu hyprdesk porte le logo, comme la pomme de macOS ;
                // son nom reste à l'arbre d'accessibilité (`titles`).
                let title: Element<'a, Msg> = match (key == "menus:hyprdesk")
                    .then(|| crate::ui::icons::get("hyprdesk-logo"))
                    .flatten()
                {
                    Some(logo) => iced::widget::svg(logo)
                        .width((size * 1.15).round())
                        .height((size * 1.15).round())
                        .style(move |_, _| iced::widget::svg::Style { color: Some(fg) })
                        .into(),
                    None => title.into(),
                };
                // Clic droit sur « Fenêtres » : le menu du clic gauche, sous la
                // même clé — un contenu, une carte (HIG ENT-07).
                let right = (actionable && matches!(request, Request::Windows))
                    .then(|| Msg::OpenPopover(id, key.into(), request.clone()));
                motion::anchor(
                    self.module(
                        s,
                        1.0,
                        title,
                        actionable.then(|| Msg::OpenPopover(id, key.into(), request)),
                        right,
                        None,
                        None,
                        None,
                        open,
                    ),
                    id,
                    key.into(),
                    self.bar_duration(t.hover),
                )
            })
            .collect()
    }

    /// Clic gauche d'un module qui ouvre une carte, partagé par la vue et par
    /// l'arbre d'accessibilité (`DoAction`, HIG ACC-01) : fenêtre, volume,
    /// luminosité, centre de contrôle, horloge (vue étendue de la bulle en
    /// style Island), calendrier ; aucun quand la vue n'affiche rien à
    /// cliquer (fenêtre absente, luminosité inconnue). Les autres modules — espaces, bureau, zone
    /// de notification, média, système, commandes, Lua — : aucun.
    pub(in crate::ui) fn bar_item_press(&self, bar_id: Id, item: &BarItem) -> Option<Msg> {
        let open = |key: &str, request| Some(Msg::OpenPopover(bar_id, key.into(), request));
        let monitor = self.bar_surfaces.get(&bar_id).map(|b| b.monitor.as_str());
        match item {
            // Le titre de la fenêtre active n'est qu'un libellé : ses
            // commandes sont au menu « Fenêtres », comme sur macOS.
            BarItem::Volume => open("volume", Request::Volume),
            // Sans niveau sur cet écran, le module n'est pas affiché.
            BarItem::Brightness => {
                self.light.level_for(monitor?)?;
                open("brightness", Request::Brightness)
            }
            BarItem::Widget(id) => match &self.cfg.widgets.iter().find(|w| &w.id == id)?.kind {
                WidgetKind::Controls => open(id, Request::Widget(id.clone())),
                WidgetKind::Calendar => open(id, Request::Calendar),
                WidgetKind::Clock { .. } => {
                    let style = self.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default();
                    let toast = (style == crate::config::BarStyle::Island)
                        .then(|| self.toasts.list.first())
                        .flatten();
                    Some(clock_press(bar_id, id, style, toast))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Titre de menu du shell actif. Les titres de la barre le sont tous :
    /// « Fenêtres » sans fenêtre ouverte le dit dans sa carte. Le menu de la
    /// fenêtre active, lui, n'a rien à montrer sans elle (désactiver plutôt
    /// que retirer).
    pub(in crate::ui) fn bar_title_live(&self, request: &Request) -> bool {
        !matches!(request, Request::Window)
            || self
                .active_window
                .as_ref()
                .is_some_and(|(_, t)| !t.is_empty())
    }

    pub(in crate::ui) fn bar_item_alpha<'a>(
        &'a self,
        bar_id: Id,
        bar: &'a BarSurface,
        item: &'a BarItem,
        alpha: f32,
    ) -> Element<'a, Msg> {
        // Menus du shell (HIG MENU-01) : leurs titres en rangée, chacun sa
        // carte. Une barre verticale n'a pas la place de titres écrits.
        if matches!(item, BarItem::Menus) {
            if bar.position.vertical() {
                return Space::new().into();
            }
            let compact = self.bar_compact(&bar.monitor);
            return row(self.bar_menu_titles(bar_id, bar))
                .spacing(if compact { 0 } else { 2 })
                .align_y(Alignment::Center)
                .into();
        }
        let pal = self.bar_palette();
        let style = self.cfg.bar.as_ref().map(|b| b.style).unwrap_or_default();
        let t = crate::ui::plugins::bar::style::durations(self);
        // Couleur du module, sinon le texte de la barre.
        let base = self.bar_item_color(item).unwrap_or(pal.fg);
        let open = self.bar_menu_open(bar_id, bar_item_key(item));
        let selected = (style == crate::config::BarStyle::Continuite && open)
            .then(|| crate::ui::plugins::bar::style::continuite::selected_ink(&pal, alpha));
        let fg = selected.unwrap_or_else(|| color(base, 0.92 * alpha));
        let dim = color(base, 0.65 * alpha);
        let shape = self.cfg.bar.as_ref().map(|b| b.shape).unwrap_or_default();
        let size = shape
            .font_size
            .unwrap_or((bar.height as f32 * 0.38).clamp(11.0, 18.0));
        let isz = shape.icon_size.map_or((size * 1.2).round(), f32::from);
        let monitor = bar.monitor.clone();
        let compact = self.bar_compact(&monitor);
        match item {
            // Rendus plus haut, avant les couleurs d'un module.
            BarItem::Menus => Space::new().into(),
            // Les touches d'espaces sont le geste du style : chaque rendu son
            // langage (capsule glissante en Continuité, touche soulevée en Graphite).
            BarItem::Workspaces => {
                crate::ui::plugins::bar::style::workspaces(self, bar_id, bar, alpha)
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
                // Un libellé, pas un titre de menu : les commandes de la
                // fenêtre sont au menu « Fenêtres », comme sur macOS. Texte
                // normal, à l'encre secondaire de la barre.
                let label = text(truncate(&t, if compact { 18 } else { 36 }))
                    .wrapping(iced::widget::text::Wrapping::None)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(size))
                    .color(dim)
                    .font(self.ui_font(iced::font::Weight::Normal));
                container(label)
                    .padding([0, 8])
                    .height(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            }
            // Interrupteur à trois états : no-desktop, launcher-only,
            // full-desktop ; le mode en cours en couleur d'accent, un clic
            // sur un autre y passe.
            BarItem::Desktop => {
                use crate::ui::DesktopMode as M;
                // Le mode de cet écran (celui du workspace qu'il affiche) ;
                // un clic le change pour cet écran.
                let mode = self.monitor_mode(&monitor);
                let fgc = color(base, alpha);
                // Continuité : ni piste ni accent, la barre reste unie ; le
                // mode en cours garde une touche neutre, comme un menu ouvert.
                let uni = style == crate::config::BarStyle::Continuite;
                let accent = if uni {
                    Color {
                        a: 0.18 * alpha,
                        ..fgc
                    }
                } else {
                    color(pal.accent, alpha)
                };
                let on_accent = if uni {
                    fg
                } else {
                    design::on_accent(accent, color(pal.fg, alpha), color(pal.bg, alpha))
                };
                let h = (bar.height as f32 * 0.66).round();
                let seg = |m: M, icon: &str, cmd: control::Command, tip: &'static str| {
                    let active = mode == m;
                    let tint = if active { on_accent } else { fg };
                    let glyph = self.icon_named(icon, isz * 0.9, tint).unwrap_or_else(|| {
                        self.label(m.as_str().chars().take(1).collect::<String>(), size, tint)
                    });
                    iced::widget::tooltip(
                        motion::press(
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
                            .size(crate::ui::tokens::ts(12.0)),
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
                    background: (!uni).then_some(Background::Color(Color {
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
                    // Trait du libellé voisin (HIG ICO-12).
                    self.icon_named_weighted(volume_icon(self.volume), isz, fg, LABEL_WEIGHT)
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
                    self.bar_item_press(bar_id, item),
                    Some(module_menu(bar_id, "volume")),
                    Some(Msg::VolumeMute),
                    Some(Box::new(|d| Msg::VolumeStep(d as i32 * 5))),
                    None,
                    open,
                )
            }
            BarItem::Brightness => {
                let Some(level) = self.light.level_for(&monitor) else {
                    return Space::new().into();
                };
                let mut content = row![
                    self.icon_named_weighted("display-brightness-symbolic", isz, fg, LABEL_WEIGHT)
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
                    self.bar_item_press(bar_id, item),
                    Some(module_menu(bar_id, "brightness")),
                    None,
                    target.map(|t| {
                        Box::new(move |d: i64| Msg::LightStep(t, d as i32 * 5))
                            as Box<dyn Fn(i64) -> Msg>
                    }),
                    Some(crate::i18n::tr("Luminosité").to_string()),
                    open,
                )
            }
            BarItem::Tray => {
                let count = self.tray.len();
                let icons = self.tray.iter().map(|t| {
                    let key = format!("tray:{}", t.key);
                    let open = self.bar_menu_open(bar_id, &key);
                    let fg = if open && style == crate::config::BarStyle::Continuite {
                        crate::ui::plugins::bar::style::continuite::selected_ink(&pal, alpha)
                    } else {
                        fg
                    };
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
                            key.clone(),
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
                        open,
                    );
                    if self.bar_overflow_items(bar_id).contains(&BarItem::Tray) {
                        module
                    } else {
                        motion::anchor(module, bar_id, key, self.bar_duration(140))
                    }
                });
                motion::rail(
                    row(icons).spacing(0).align_y(Alignment::Center),
                    None,
                    vec![true; count],
                    crate::ui::plugins::bar::style::rail_look(style, &pal),
                    self.bar_duration(t.hover),
                )
            }
            BarItem::Widget(id) => {
                let Some(w) = self.cfg.widgets.iter().find(|w| &w.id == id) else {
                    return Space::new().into();
                };
                let value = self.values.get(id);
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
                        self.bar_item_press(bar_id, item),
                        None,
                        None,
                        None,
                        Some(crate::i18n::tr("Centre de contrôle").into()),
                        open,
                    ),
                    WidgetKind::Clock { format } => {
                        // « Dynamic island » : en style Island, la dernière
                        // bulle vivante prend la main sur l'horloge de la
                        // pilule centrale, et lui rend à sa disparition. Le
                        // clic ouvre sa vue étendue (HIG SYS-13) ; le
                        // calendrier reste en pied de cette vue.
                        let toast = (style == crate::config::BarStyle::Island)
                            .then(|| self.toasts.list.first())
                            .flatten();
                        let shown: Element<'_, Msg> = if let Some(t) = toast {
                            // Un glyphe monochrome de l'activité, jamais l'icône
                            // entière de l'application (HIG SYS-14).
                            let icon = crate::ui::icons::glyph("notifications", isz, fg)
                                .unwrap_or_else(|| self.label("•", size, fg));
                            row![icon, self.label(truncate(&t.n.summary, 26), size, fg),]
                                .spacing(6)
                                .align_y(Alignment::Center)
                                .into()
                        } else if style == crate::config::BarStyle::Graphite {
                            iced::widget::column![
                                self.label_numeric(format_now("%H:%M"), 12.0, fg),
                                self.label(format_now("%d/%m/%Y"), 11.0, fg),
                            ]
                            .align_x(Alignment::End)
                            .spacing(0)
                            .into()
                        } else {
                            self.label_numeric(
                                format_now(if compact { "%H:%M" } else { format }),
                                size,
                                fg,
                            )
                        };
                        // L'ovale des non-lues est sur le module des
                        // notifications, pas sur l'horloge.
                        self.module(
                            bar,
                            alpha,
                            shown,
                            self.bar_item_press(bar_id, item),
                            Some(module_menu(bar_id, id)),
                            None,
                            None,
                            None,
                            open,
                        )
                    }
                    WidgetKind::Calendar => self.module(
                        bar,
                        alpha,
                        self.label(format_now("%a %-d %b"), size, fg),
                        self.bar_item_press(bar_id, item),
                        Some(module_menu(bar_id, id)),
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
                                    // Trait du libellé voisin (HIG ICO-12).
                                    self.icon_named_weighted(icon, isz * 0.85, fg, LABEL_WEIGHT)
                                        .unwrap_or_else(|| self.label(
                                            crate::i18n::tr("▶"),
                                            size,
                                            fg
                                        )),
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
                        // Un module allumé (Wi-Fi, Bluetooth) ne se remplit pas en
                        // Continuité : son icône dit l'état, la barre reste unie.
                        let tint = selected.unwrap_or_else(|| match r.state {
                            RenderState::Active if self.bar_item_color(item).is_some() => {
                                color(base, alpha)
                            }
                            s => self.state_color(s, color(base, 0.92)).scale_alpha(alpha),
                        });
                        let mut content = row![].spacing(6).align_y(Alignment::Center);
                        let mut has_icon = false;
                        if let Some(icon) = r
                            .icon
                            .as_deref()
                            // Trait du libellé voisin (HIG ICO-12).
                            .and_then(|name| {
                                self.icon_named_weighted(name, isz, tint, LABEL_WEIGHT)
                            })
                        {
                            content = content.push(icon);
                            has_icon = true;
                        }
                        // Avancement d'une tâche (MLG-161) : une jauge fine, de la
                        // couleur du module, sur une piste discrète.
                        if let Some(p) = r.progress {
                            let track = tint.scale_alpha(0.25);
                            content = content.push(
                                iced::widget::progress_bar(0.0..=1.0, p)
                                    .girth(4)
                                    .length(Length::Fixed((isz * 2.5).round()))
                                    .style(move |_| iced::widget::progress_bar::Style {
                                        background: track.into(),
                                        bar: tint.into(),
                                        border: iced::Border::default().rounded(2),
                                    }),
                            );
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
                        // Widget de notifications sans fonction `menu` (HIG
                        // ENT-07) : son menu court à lui ; avec elle, le sien.
                        let notif = menu.is_none() && w.type_id == "notifications";
                        let (left, right) = match (menu_msg, click_msg) {
                            (Some(m), Some(c)) => (Some(c), Some(m)),
                            (Some(m), None) => (Some(m), None),
                            (None, c) => (c, notif.then(|| module_menu(bar_id, id))),
                        };
                        let wid = id.clone();
                        let scroll_cb: Option<Box<dyn Fn(i64) -> Msg + 'a>> = scroll.map(|f| {
                            Box::new(move |d: i64| Msg::LuaCall(wid.clone(), f, Some(d)))
                                as Box<dyn Fn(i64) -> Msg + 'a>
                        });
                        let frame = self.module(
                            bar,
                            alpha,
                            content.into(),
                            left,
                            right,
                            None,
                            scroll_cb,
                            r.tooltip.clone(),
                            open,
                        );
                        // Ovale des non-lues sur le module des notifications,
                        // ou du compte que rend le widget (HIG SYS-02).
                        let count = if w.type_id == "notifications" {
                            self.toasts.unread()
                        } else {
                            r.badge.unwrap_or(0) as usize
                        };
                        self.with_badge(frame, bar, alpha, count, isz)
                    }
                }
            }
        }
    }

    /// Carte de `key` ouverte sur la barre. Le menu contextuel d'un module
    /// (HIG ENT-07) porte la clé `menu_key(module)` : il allume son module.
    fn bar_menu_open(&self, bar: Id, key: &str) -> bool {
        self.popover
            .as_ref()
            .is_some_and(|p| p.bar == bar && popover::module_key(&p.key) == key && !p.closing())
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
    /// Par-dessus un plein écran, la barre se retire au bord, sauf déjà sur
    /// `Overlay` ou en Convex masqué (HIG PAT-03).
    #[test]
    fn barre_par_dessus_le_plein_ecran() {
        use super::bar_covers;
        use crate::config::BarLayer;
        assert!(bar_covers(BarLayer::Top, false, true));
        assert!(bar_covers(BarLayer::Bottom, false, true));
        assert!(!bar_covers(BarLayer::Top, false, false));
        assert!(!bar_covers(BarLayer::Overlay, false, true));
        assert!(!bar_covers(BarLayer::Top, true, true));
    }

    /// Cible d'un module de barre : 20 px au moins, jamais plus haut que la
    /// barre (HIG ACC-09).
    #[test]
    fn cible_d_un_module_de_barre() {
        assert_eq!(module_target(36), 28.0);
        assert_eq!(module_target(30), 22.0);
        assert_eq!(module_target(28), MODULE_TARGET);
        assert_eq!(module_target(24), MODULE_TARGET, "24 px : 20, plus 16");
        assert_eq!(module_target(20), MODULE_TARGET);
        assert_eq!(module_target(18), 18.0, "jamais plus haut que la barre");
    }

    #[test]
    fn evenements_du_volume() {
        assert!(volume_event("Event 'change' on sink #57"));
        assert!(volume_event("Event 'change' on server #4294967295"));
        assert!(!volume_event("Event 'change' on sink-input #120"));
        assert!(!volume_event("Event 'new' on client #300"));
    }

    use super::*;

    #[test]
    fn chaque_style_suit_le_theme_actif_et_les_couleurs_explicites() {
        use crate::config::{BAR_STYLES, Place, Theme};
        use std::sync::Arc;

        let path = std::path::PathBuf::from("/tmp/hyprdesk-bar-themes/config.lua");
        let (mut app, _) = App::boot_with(path, true);
        let loaded = crate::config::lua::load_source(
            "hd.bar({})",
            "bar-themes.lua",
            std::path::Path::new("/tmp"),
        )
        .unwrap();
        app.cfg = Arc::new(loaded.config);
        for &(style, _, _) in BAR_STYLES {
            let cfg = Arc::make_mut(&mut app.cfg);
            let bar = cfg.bar.as_mut().unwrap();
            bar.style = style;
            bar.colors = Default::default();
            for preset in crate::theme::PRESETS {
                Arc::make_mut(&mut app.cfg).general.theme = Theme {
                    name: preset.name.into(),
                    fg: preset.fg,
                    bg: preset.bg,
                    accent: preset.accent,
                    effects: preset.effects(),
                };
                let pal = app.bar_palette();
                assert_eq!(
                    (pal.fg, pal.bg, pal.accent),
                    (preset.fg, preset.bg, preset.accent)
                );
                assert_eq!(
                    pal.opacity,
                    app.cfg.general.theme.effect(Place::Bar).opacity
                );
            }
            let cfg = Arc::make_mut(&mut app.cfg);
            let colors = &mut cfg.bar.as_mut().unwrap().colors;
            let explicit_bg = Rgba(31, 92, 75, 255);
            colors.background = Some(crate::config::ColorRef::Fixed(explicit_bg));
            colors.opacity = Some(0.55);
            let pal = app.bar_palette();
            assert_eq!(pal.bg, explicit_bg);
            assert_eq!(pal.opacity, 0.55);
            assert_eq!(pal.accent, app.cfg.general.theme.accent);
            // Paire sombre/clair (MLG-263) : la moitié du mode du thème.
            let (sombre, clair) = (Rgba(0x2d, 0xd4, 0xbf, 255), Rgba(0x00, 0x6b, 0x5f, 255));
            let cfg = Arc::make_mut(&mut app.cfg);
            cfg.bar.as_mut().unwrap().colors.accent = Some(crate::config::ColorRef::Pair {
                dark: sombre,
                light: clair,
            });
            let attendu = if light_theme(&app.cfg.general.theme) {
                clair
            } else {
                sombre
            };
            assert_eq!(app.bar_palette().accent, attendu);
        }
    }

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

    /// HIG SYS-02 : l'ovale des non-lues se pose au coin du cadre de l'horloge
    /// qui touche le bord de l'écran ; « 99+ » au-delà de 99.
    #[test]
    fn ovale_des_non_lues_au_bord_de_l_ecran() {
        use BarPosition::{Bottom, Left, Right, Top};
        assert_eq!(App::badge_corner(Top), (Alignment::End, Alignment::Start));
        assert_eq!(App::badge_corner(Bottom), (Alignment::End, Alignment::End));
        assert_eq!(App::badge_corner(Left), (Alignment::Start, Alignment::End));
        assert_eq!(App::badge_corner(Right), (Alignment::End, Alignment::End));
        assert_eq!(App::badge_label(1), "1");
        assert_eq!(App::badge_label(99), "99");
        assert_eq!(App::badge_label(100), "99+");
    }

    /// HIG SYS-13 : en style Island, le clic sur la pilule occupée par une
    /// bulle vivante ouvre la vue étendue de cette bulle ; sans bulle, ou dans
    /// un autre style, l'horloge garde le calendrier.
    #[test]
    fn le_clic_sur_la_pilule_occupee_ouvre_la_vue_etendue() {
        use crate::config::BarStyle;
        let bulle = |id: u32| crate::ui::toasts::Toast {
            n: crate::notifications::build(id, "a", "", "s", "", &[], &Default::default(), -1),
            since: std::time::Instant::now(),
            request: None,
            alert: false,
            summary: false,
        };
        let calendrier = |style, toast| {
            matches!(
                clock_press(Id::unique(), "clock", style, toast),
                Msg::OpenPopover(_, key, Request::Calendar) if key == "clock"
            )
        };
        let sept = bulle(7);
        assert!(
            matches!(
                clock_press(Id::unique(), "clock", BarStyle::Island, Some(&sept)),
                Msg::OpenPopover(_, key, Request::Activity { id }) if key == "clock" && id == 7
            ),
            "Island, bulle vivante : la vue étendue de cette bulle"
        );
        for style in [BarStyle::Continuite, BarStyle::Graphite, BarStyle::Convexe] {
            assert!(
                calendrier(style, Some(&sept)),
                "un autre style garde le calendrier : {style:?}"
            );
        }
        assert!(
            calendrier(BarStyle::Island, None),
            "sans bulle vivante, l'horloge garde le calendrier"
        );
    }
}
