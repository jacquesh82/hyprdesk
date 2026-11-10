//! Barre native : une surface layer-shell par écran, sur le calque `top`
//! (au-dessus des fenêtres), avec une zone exclusive — Hyprland range les
//! fenêtres à côté, et le bureau décale sa grille de la zone réservée.
//!
//! Dessin soigné, à la manière d'une barre de menus macOS : fond translucide
//! flouté, filet discret, icônes symboliques teintées, capsule au survol,
//! menus déroulants (voir `popover`). Ses modules sont des widgets.

use std::time::Duration;

use iced::widget::{Space, button, container, mouse_area, row, text, tooltip};
use iced::window::Id;
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Task, mouse};
use iced_layershell::reexport::{Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption};

use super::popover::Request;
use super::{App, IconImg, Msg, color, format_now};
use crate::config::{BarItem, BarPosition, RenderState, WidgetKind};
use crate::{control, hypr, widgets};

pub const BAR_NAMESPACE: &str = "hyprdesk-bar";

/// Icônes symboliques demandées dès que la barre existe.
pub const STATIC_ICONS: &[&str] = &[
    "user-desktop-symbolic",
    "audio-volume-muted-symbolic",
    "audio-volume-low-symbolic",
    "audio-volume-medium-symbolic",
    "audio-volume-high-symbolic",
    "media-playback-start-symbolic",
    "media-playback-pause-symbolic",
    "object-select-symbolic",
];

/// Surface de barre : écran, position et hauteur à sa création.
#[derive(Debug, Clone, PartialEq)]
pub struct BarSurface {
    pub monitor: String,
    pub position: BarPosition,
    pub height: u16,
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n).collect::<String>()) }
}

/// `wpctl get-volume` → (volume 0..1, coupé).
pub fn parse_volume(out: &str) -> Option<(f32, bool)> {
    let v: f32 = out.split_whitespace().nth(1)?.parse().ok()?;
    Some((v, out.contains("[MUTED]")))
}

pub async fn read_volume() -> Option<(f32, bool)> {
    let o = tokio::process::Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]).output().await.ok()?;
    parse_volume(&String::from_utf8_lossy(&o.stdout))
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
        let want: Vec<String> = match &self.cfg.bar {
            None => Vec::new(),
            Some(b) => self
                .monitors
                .iter()
                .filter(|m| b.monitors.as_ref().is_none_or(|l| l.contains(&m.name)))
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
                let edge = if position == BarPosition::Top { LsAnchor::Top } else { LsAnchor::Bottom };
                self.bar_surfaces.insert(id, BarSurface { monitor: mon.clone(), position, height });
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
            .filter_map(|i| if let BarItem::Widget(id) = i { Some(id.clone()) } else { None })
            .collect()
    }

    pub(super) fn bar_has(&self, item: &BarItem) -> bool {
        self.cfg.bar.iter().flat_map(|b| b.zones.iter().flatten()).any(|i| i == item)
    }

    pub(super) fn volume_action(&self, args: Vec<String>) -> Task<Msg> {
        Task::perform(
            async move {
                let _ = tokio::process::Command::new("wpctl").args(&args).status().await;
                read_volume().await
            },
            Msg::VolumeValue,
        )
    }

    fn state_color(&self, s: RenderState) -> Color {
        let th = &self.cfg.general.theme;
        match s {
            RenderState::Warning => Color::from_rgb8(0xff, 0xb8, 0x6c),
            RenderState::Critical => Color::from_rgb8(0xff, 0x55, 0x55),
            RenderState::Active => color(th.accent, 1.0),
            _ => color(th.fg, 0.92),
        }
    }

    pub(super) fn view_bar(&self, id: Id) -> Element<'_, Msg> {
        let (Some(s), Some(bar)) = (self.bar_surfaces.get(&id), &self.cfg.bar) else { return Space::new().into() };
        let th = &self.cfg.general.theme;
        let bg = color(th.bg, 0.72);
        let hair = color(th.fg, 0.07);
        let content = row![
            container(self.bar_zone(id, s, &bar.zones[0])).width(Length::Fill).align_x(Alignment::Start),
            container(self.bar_zone(id, s, &bar.zones[1])).align_x(Alignment::Center),
            container(self.bar_zone(id, s, &bar.zones[2])).width(Length::Fill).align_x(Alignment::End),
        ]
        .align_y(Alignment::Center)
        .height(Length::Fill);
        // Filet d'un pixel du côté des fenêtres, comme une barre de menus.
        let top = s.position == BarPosition::Top;
        container(content)
            .padding(Padding { top: if top { 0.0 } else { 1.0 }, bottom: if top { 1.0 } else { 0.0 }, left: 8.0, right: 8.0 })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border { color: hair, width: 0.0, radius: 0.0.into() },
                shadow: iced::Shadow { color: hair, offset: iced::Vector::new(0.0, if top { 1.0 } else { -1.0 }), blur_radius: 0.0 },
                ..Default::default()
            })
            .into()
    }

    fn bar_zone<'a>(&'a self, id: Id, s: &BarSurface, items: &'a [BarItem]) -> Element<'a, Msg> {
        row(items.iter().map(|it| self.bar_item(id, s, it))).spacing(2).align_y(Alignment::Center).into()
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
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let h = (bar.height as f32 - 8.0).max(16.0);
        let interactive = left.is_some() || right.is_some() || middle.is_some() || scroll.is_some();
        let b = button(container(content).height(Length::Fill).align_y(Alignment::Center))
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
                let bg = color(th.bg, 0.95);
                let edge = Color { a: 0.12, ..fg };
                tooltip(
                    el,
                    container(text(t).size(12).font(self.ui_font(iced::font::Weight::Normal))).padding([5, 9]).style(move |_| container::Style {
                        background: Some(bg.into()),
                        border: Border::default().rounded(7).width(1).color(edge),
                        text_color: Some(fg),
                        shadow: iced::Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.35), offset: iced::Vector::new(0.0, 4.0), blur_radius: 12.0 },
                        ..Default::default()
                    }),
                    if bar.position == BarPosition::Top { tooltip::Position::Bottom } else { tooltip::Position::Top },
                )
                .gap(6)
                .delay(Duration::from_millis(500))
                .into()
            }
            _ => el,
        }
    }

    fn label<'a>(&self, s: impl Into<String>, size: f32, c: Color) -> Element<'a, Msg> {
        text(s.into()).size(size).color(c).font(self.ui_font(iced::font::Weight::Medium)).wrapping(iced::widget::text::Wrapping::None).into()
    }

    fn bar_item<'a>(&'a self, bar_id: Id, bar: &BarSurface, item: &'a BarItem) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 0.92);
        let dim = color(th.fg, 0.5);
        let size = (bar.height as f32 * 0.38).clamp(11.0, 18.0);
        let isz = (size * 1.2).round();
        let monitor = bar.monitor.clone();
        match item {
            BarItem::Workspaces => {
                let current = self.monitors.iter().find(|m| m.name == monitor).map(|m| m.workspace.clone()).unwrap_or_default();
                let accent = color(th.accent, 1.0);
                let on_accent = color(th.bg, 1.0);
                let pills = self.workspaces.iter().filter(|w| w.monitor == monitor).map(|w| {
                    let active = w.name == current;
                    let (txt, fill) = if active {
                        (on_accent, Some(accent))
                    } else if w.windows > 0 {
                        (fg, None)
                    } else {
                        (dim, None)
                    };
                    let h = (bar.height as f32 * 0.6).round();
                    let fgc = color(th.fg, 1.0);
                    button(container(self.label(w.name.clone(), size * 0.92, txt)).center_x(Length::Shrink).center_y(Length::Fill))
                        .height(h)
                        .padding([0, if w.name.chars().count() > 1 { 8 } else { 0 }])
                        .width(if w.name.chars().count() > 1 { Length::Shrink } else { Length::Fixed(h) })
                        .on_press(Msg::BarWorkspace(monitor.clone(), w.name.clone()))
                        .style(move |_, st| button::Style {
                            background: match (fill, st) {
                                (Some(f), _) => Some(Background::Color(f)),
                                (None, button::Status::Hovered) => Some(Background::Color(Color { a: 0.12, ..fgc })),
                                _ => None,
                            },
                            text_color: txt,
                            border: Border::default().rounded(h / 2.0),
                            ..Default::default()
                        })
                        .into()
                });
                let mon = monitor.clone();
                mouse_area(container(row(pills).spacing(3).align_y(Alignment::Center)).padding([0, 4]))
                    .on_scroll(move |d| Msg::BarScroll(mon.clone(), -scroll_dir(d) as i32))
                    .into()
            }
            BarItem::Window => {
                let t = self.active_window.as_ref().map(|(c, t)| if t.is_empty() { c.clone() } else { t.clone() }).unwrap_or_default();
                container(text(truncate(&t, 64)).size(size).color(dim).font(self.ui_font(iced::font::Weight::Normal))).padding([0, 8]).into()
            }
            BarItem::Desktop => self.module(
                bar,
                self.icon_named("user-desktop-symbolic", isz, fg).unwrap_or_else(|| self.label("Bureau", size, fg)),
                Some(Msg::Control(control::Command::Toggle)),
                None,
                None,
                None,
                Some(if self.visible { "Masquer le bureau".into() } else { "Afficher le bureau".into() }),
                self.visible,
            ),
            BarItem::Volume => {
                let label = match self.volume {
                    Some((v, false)) => format!("{:.0} %", v * 100.0),
                    _ => String::new(),
                };
                let mut content = row![self.icon_named(volume_icon(self.volume), isz, fg).unwrap_or_else(|| self.label("Vol.", size, fg))]
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
            BarItem::Tray => {
                let icons = self.tray.iter().map(|t| {
                    let img: Element<'_, Msg> = self
                        .tray_icon(t, isz)
                        .unwrap_or_else(|| self.label(t.title.chars().next().map(|c| c.to_string()).unwrap_or("?".into()), size, fg));
                    let menu = t.menu.clone().map(|mp| Msg::OpenPopover(bar_id, format!("tray:{}", t.key), Request::Tray { bus: t.bus.clone(), menu_path: mp }));
                    let left = if t.item_is_menu { menu.clone() } else { Some(Msg::TrayActivate(bar_id, t.bus.clone(), t.path.clone(), false)) };
                    self.module(
                        bar,
                        img,
                        left,
                        menu,
                        Some(Msg::TrayActivate(bar_id, t.bus.clone(), t.path.clone(), true)),
                        None,
                        Some(t.title.clone()),
                        self.popover.as_ref().is_some_and(|p| p.key == format!("tray:{}", t.key)),
                    )
                });
                row(icons).spacing(0).align_y(Alignment::Center).into()
            }
            BarItem::Widget(id) => {
                let Some(w) = self.cfg.widgets.iter().find(|w| &w.id == id) else { return Space::new().into() };
                let value = self.values.get(id);
                let open = self.popover.as_ref().is_some_and(|p| &p.key == id);
                match &w.kind {
                    WidgetKind::Clock { format } => self.module(
                        bar,
                        self.label(format_now(format), size, fg),
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
                                row![self.label(l.label.clone(), size * 0.9, dim), self.label(v, size, fg)].spacing(4).align_y(Alignment::Center).into()
                            });
                            self.module(bar, row(parts).spacing(10).align_y(Alignment::Center).into(), None, None, None, None, None, false)
                        }
                        _ => Space::new().into(),
                    },
                    WidgetKind::Media { .. } => match value {
                        Some(widgets::Value::Media(Some(mi))) => {
                            let label = if mi.artist.is_empty() { mi.title.clone() } else { format!("{} — {}", mi.title, mi.artist) };
                            let icon = if mi.playing { "media-playback-pause-symbolic" } else { "media-playback-start-symbolic" };
                            self.module(
                                bar,
                                row![self.icon_named(icon, isz * 0.85, fg).unwrap_or_else(|| self.label("▶", size, fg)), self.label(truncate(&label, 40), size, fg)]
                                    .spacing(7)
                                    .align_y(Alignment::Center)
                                    .into(),
                                Some(Msg::Media(mi.bus.clone(), "PlayPause")),
                                None,
                                None,
                                Some(Box::new({
                                    let bus = mi.bus.clone();
                                    move |d| Msg::Media(bus.clone(), if d > 0 { "Previous" } else { "Next" })
                                })),
                                Some(label),
                                false,
                            )
                        }
                        _ => Space::new().into(),
                    },
                    WidgetKind::Command { .. } => match value {
                        Some(widgets::Value::Text(t)) if !t.is_empty() => {
                            self.module(bar, self.label(truncate(t.lines().next().unwrap_or(""), 60), size, fg), None, None, None, None, Some(t.clone()), false)
                        }
                        _ => Space::new().into(),
                    },
                    WidgetKind::Lua { click, scroll, menu, .. } => {
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
                        let tint = self.state_color(r.state);
                        let mut content = row![].spacing(6).align_y(Alignment::Center);
                        if let Some(icon) = r.icon.as_deref().and_then(|name| self.icon_named(name, isz, tint)) {
                            content = content.push(icon);
                        }
                        if !r.text.is_empty() {
                            content = content.push(self.label(truncate(&r.text, 48), size, tint));
                        }
                        let menu_msg = menu.map(|f| Msg::OpenPopover(bar_id, id.clone(), Request::LuaMenu { widget: id.clone(), func: f }));
                        let click_msg = click.map(|f| Msg::LuaCall(id.clone(), f, None));
                        let (left, right) = match (menu_msg, click_msg) {
                            (Some(m), Some(c)) => (Some(c), Some(m)),
                            (Some(m), None) => (Some(m), None),
                            (None, c) => (c, None),
                        };
                        let wid = id.clone();
                        let scroll_cb: Option<Box<dyn Fn(i64) -> Msg + 'a>> =
                            scroll.map(|f| Box::new(move |d: i64| Msg::LuaCall(wid.clone(), f, Some(d))) as Box<dyn Fn(i64) -> Msg + 'a>);
                        self.module(bar, content.into(), left, right, None, scroll_cb, r.tooltip.clone(), open)
                    }
                }
            }
        }
    }

    fn tray_icon(&self, t: &crate::tray::TrayItem, size: f32) -> Option<Element<'_, Msg>> {
        if let Some((_, h)) = self.tray_imgs.get(&t.key) {
            return Some(iced::widget::image(h.clone()).width(size).height(size).into());
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
    fn volume_wpctl() {
        assert_eq!(parse_volume("Volume: 0.60"), Some((0.6, false)));
        assert_eq!(parse_volume("Volume: 0.35 [MUTED]"), Some((0.35, true)));
        assert_eq!(parse_volume("erreur"), None);
        assert_eq!(truncate("abcdef", 3), "abc…");
        assert_eq!(volume_icon(Some((0.9, false))), "audio-volume-high-symbolic");
        assert_eq!(volume_icon(Some((0.9, true))), "audio-volume-muted-symbolic");
    }
}
