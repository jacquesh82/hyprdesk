//! Barre native : une surface layer-shell par écran, sur le calque `top`
//! (au-dessus des fenêtres), avec une zone exclusive — Hyprland range les
//! fenêtres à côté, et le bureau décale sa grille de la zone réservée.
//!
//! Ses modules sont des widgets : modules propres (workspaces, fenêtre active,
//! volume, bouton bureau) et n'importe quel widget intégré ou plugin, rendu en
//! version compacte.

use std::time::Duration;

use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::window::Id;
use iced::{Alignment, Background, Border, Color, Element, Length, Task, mouse};
use iced_layershell::reexport::{Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption};

use super::{App, IconImg, Msg, color, format_now};
use crate::config::{BarItem, BarPosition, WidgetKind};
use crate::{control, hypr, widgets};

pub const BAR_NAMESPACE: &str = "hyprdesk-bar";

/// Icônes de la barre dans `App::ui_imgs` (voir `UI_ICONS`).
pub const ICON_DESKTOP: usize = 5;
pub const ICON_VOLUME: [usize; 4] = [6, 7, 8, 9]; // coupé, faible, moyen, fort

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
            let keep = want.contains(&s.monitor) && spec == Some((s.position, s.height));
            if !keep {
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

    pub(super) fn bar_has_volume(&self) -> bool {
        self.cfg.bar.iter().flat_map(|b| b.zones.iter().flatten()).any(|i| *i == BarItem::Volume)
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

    pub(super) fn view_bar(&self, id: Id) -> Element<'_, Msg> {
        let (Some(s), Some(bar)) = (self.bar_surfaces.get(&id), &self.cfg.bar) else { return Space::new().into() };
        let th = &self.cfg.general.theme;
        let bg = color(th.bg, 0.92);
        let size = (s.height as f32 * 0.38).max(11.0);
        let zone = |items: &[BarItem]| -> Element<'_, Msg> {
            row(items.iter().map(|it| self.bar_item(it, &s.monitor, size))).spacing(12).align_y(Alignment::Center).into()
        };
        container(
            row![
                container(zone(&bar.zones[0])).width(Length::Fill).align_x(Alignment::Start),
                container(zone(&bar.zones[1])).align_x(Alignment::Center),
                container(zone(&bar.zones[2])).width(Length::Fill).align_x(Alignment::End),
            ]
            .align_y(Alignment::Center)
            .height(Length::Fill),
        )
        .padding([0, 12])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style { background: Some(bg.into()), ..Default::default() })
        .into()
    }

    fn bar_button<'a>(&self, content: impl Into<Element<'a, Msg>>, msg: Msg, active: bool) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        button(content)
            .padding([2, 8])
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: if active {
                    Some(Background::Color(Color { a: 0.55, ..accent }))
                } else if matches!(st, button::Status::Hovered) {
                    Some(Background::Color(Color { a: 0.12, ..fg }))
                } else {
                    None
                },
                text_color: fg,
                border: Border::default().rounded(6),
                ..Default::default()
            })
            .into()
    }

    fn bar_item(&self, item: &BarItem, monitor: &str, size: f32) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let (fg, dim) = (color(th.fg, 1.0), color(th.fg, 0.55));
        let font = self.font;
        let icon = |i: usize, fallback: &'static str| -> Element<'_, Msg> {
            self.img_element(self.ui_imgs.get(i).copied().unwrap_or(IconImg::None), size * 1.2, 1.0)
                .unwrap_or_else(|| text(fallback).size(size).color(fg).font(font).into())
        };
        match item {
            BarItem::Workspaces => {
                let current = self.monitors.iter().find(|m| m.name == monitor).map(|m| m.workspace.clone()).unwrap_or_default();
                let buttons = self.workspaces.iter().filter(|w| w.monitor == monitor).map(|w| {
                    let c = if w.windows > 0 || w.name == current { fg } else { dim };
                    self.bar_button(text(w.name.clone()).size(size).color(c).font(font), Msg::BarWorkspace(monitor.to_string(), w.name.clone()), w.name == current)
                });
                let mon = monitor.to_string();
                mouse_area(row(buttons).spacing(4).align_y(Alignment::Center))
                    .on_scroll(move |d| {
                        let y = match d {
                            mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => y,
                        };
                        Msg::BarScroll(mon.clone(), if y > 0.0 { -1 } else { 1 })
                    })
                    .into()
            }
            BarItem::Window => {
                let t = self.active_window.as_ref().map(|(c, t)| if t.is_empty() { c.clone() } else { t.clone() }).unwrap_or_default();
                text(truncate(&t, 70)).size(size).color(dim).font(font).into()
            }
            BarItem::Desktop => self.bar_button(icon(ICON_DESKTOP, "Bureau"), Msg::Control(control::Command::Toggle), self.visible),
            BarItem::Volume => {
                let (label, i) = match self.volume {
                    None => ("—".to_string(), ICON_VOLUME[0]),
                    Some((_, true)) => ("coupé".to_string(), ICON_VOLUME[0]),
                    Some((v, false)) => (format!("{:.0} %", v * 100.0), ICON_VOLUME[if v < 0.34 { 1 } else if v < 0.67 { 2 } else { 3 }]),
                };
                mouse_area(row![icon(i, "Vol."), text(label).size(size).color(fg).font(font)].spacing(4).align_y(Alignment::Center))
                    .on_press(Msg::VolumeMute)
                    .on_scroll(|d| {
                        let y = match d {
                            mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => y,
                        };
                        Msg::VolumeStep(if y > 0.0 { 5 } else { -5 })
                    })
                    .interaction(mouse::Interaction::Pointer)
                    .into()
            }
            BarItem::Widget(id) => {
                let Some(w) = self.cfg.widgets.iter().find(|w| &w.id == id) else { return Space::new().into() };
                let value = self.values.get(id);
                match &w.kind {
                    WidgetKind::Clock { format } => text(format_now(format)).size(size).color(fg).font(font).into(),
                    WidgetKind::Calendar => text(format_now("%a %d %b")).size(size).color(fg).font(font).into(),
                    WidgetKind::System { .. } => match value {
                        Some(widgets::Value::Sys(lines)) => row(lines.iter().map(|l| {
                            let v = match l.ratio {
                                Some(r) => format!("{:.0} %", r * 100.0),
                                None => l.value.clone(),
                            };
                            row![text(l.label.clone()).size(size).color(dim).font(font), text(v).size(size).color(fg).font(font)].spacing(4).into()
                        }))
                        .spacing(10)
                        .into(),
                        _ => Space::new().into(),
                    },
                    WidgetKind::Media { .. } => match value {
                        Some(widgets::Value::Media(Some(mi))) => {
                            let label = if mi.artist.is_empty() { mi.title.clone() } else { format!("{} — {}", mi.title, mi.artist) };
                            let play = if mi.playing { icon(1, "||") } else { icon(2, ">") };
                            row![
                                self.bar_button(play, Msg::Media(mi.bus.clone(), "PlayPause"), false),
                                text(truncate(&label, 45)).size(size).color(fg).font(font),
                            ]
                            .spacing(4)
                            .align_y(Alignment::Center)
                            .into()
                        }
                        _ => Space::new().into(),
                    },
                    WidgetKind::Command { .. } | WidgetKind::Lua { .. } => match value {
                        Some(widgets::Value::Text(t)) => text(truncate(t.lines().next().unwrap_or(""), 60)).size(size).color(fg).font(font).into(),
                        Some(widgets::Value::Error(e)) => text(format!("⚠ {}", truncate(e, 40))).size(size).color(dim).font(font).into(),
                        _ => Space::new().into(),
                    },
                }
            }
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
    }
}
