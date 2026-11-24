//! Composition du centre de contrôle de Continuité dans la surface de menu existante.
use crate::ui::tokens::Shown as _;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use chrono::Datelike;
use iced::widget::{
    Space, button, canvas, column, container, row, scrollable, slider, tooltip, vertical_slider,
};
use iced::{Alignment, Border, Color, Element, Length, Point, Rectangle, Size, Theme, mouse};

use super::{PMsg, Popover, Request, SHADOW};
use crate::config::{RenderState, WidgetKind};
use crate::control::Command;
use crate::i18n::{text, tr};
use crate::ui::plugins::bar::{style::continuite, surface::BarPalette};
use crate::ui::{App, DesktopMode, Msg, color, format_now};
use crate::widgets::{SysLine, Value, media::Status};

mod keyboard;
pub(in crate::ui) use keyboard::handle as handle_key;

const GAP: f32 = 12.0;
const PADDING: f32 = 12.0;
const HISTORY: usize = 30;

pub(in crate::ui) const ICONS: &[&str] = &[
    "user-home-symbolic",
    "applications-graphics-symbolic",
    "power-profile-performance-symbolic",
    "video-display-symbolic",
    "view-grid-symbolic",
    "preferences-system-symbolic",
    "system-lock-screen-symbolic",
    "weather-overcast-symbolic",
    "display-brightness-symbolic",
    "notifications-disabled-symbolic",
    "preferences-system-notifications-symbolic",
    "edit-paste-symbolic",
    "media-skip-backward-symbolic",
    "media-playback-pause-symbolic",
    "media-playback-start-symbolic",
    "media-skip-forward-symbolic",
    "window-close-symbolic",
    "edit-clear-symbolic",
    "go-previous-symbolic",
    "go-next-symbolic",
];

pub(in crate::ui) struct Dashboard {
    pub id: iced::window::Id,
    pub media: Option<Status>,
    pub system: Vec<SysLine>,
    pub network: Option<(f32, f32)>,
    cpu: VecDeque<f32>,
    gpu: VecDeque<f32>,
    traffic: VecDeque<f32>,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            id: iced::window::Id::unique(),
            media: None,
            system: Vec::new(),
            network: None,
            cpu: VecDeque::new(),
            gpu: VecDeque::new(),
            traffic: VecDeque::new(),
        }
    }

    pub fn update(&mut self, sample: Sample) {
        self.media = sample.media;
        for (label, history) in [("CPU", &mut self.cpu), ("GPU", &mut self.gpu)] {
            if let Some(value) = sample
                .system
                .iter()
                .find(|s| s.label == label)
                .and_then(|s| s.ratio)
            {
                push_sample(history, value);
            }
        }
        self.system = sample.system;
        self.network = sample.network;
        if let Some((rx, tx)) = sample.network {
            push_sample(&mut self.traffic, rx + tx);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub media: Option<Status>,
    pub system: Vec<SysLine>,
    pub network: Option<(f32, f32)>,
}

fn push_sample(history: &mut VecDeque<f32>, value: f32) {
    if history.len() == HISTORY {
        history.pop_front();
    }
    history.push_back(value);
}

/// Le relevé s'arrête à la fermeture du panneau et ses résultats portent son identifiant.
pub(in crate::ui) fn samples(
    id: &iced::window::Id,
) -> impl iced::futures::Stream<Item = Msg> + use<> {
    use iced::futures::SinkExt;

    let id = *id;
    iced::stream::channel(1, async move |mut out| {
        let mut networks = sysinfo::Networks::new().ok();
        let mut last = None::<Instant>;
        loop {
            let metrics = tokio::task::spawn_blocking(move || {
                let now = Instant::now();
                let rates = networks.as_mut().and_then(|networks| {
                    networks.refresh(true);
                    let elapsed = now.duration_since(last?).as_secs_f32().max(0.001);
                    let mut interfaces = networks
                        .iter()
                        .filter(|(name, _)| name.as_os_str() != "lo")
                        .peekable();
                    interfaces.peek()?;
                    let (rx, tx) = interfaces.fold((0.0, 0.0), |(rx, tx), (_, data)| {
                        (rx + data.received() as f32, tx + data.transmitted() as f32)
                    });
                    Some((rx / elapsed, tx / elapsed))
                });
                let system = crate::panorama::telemetry::sample("cpu,gpu,ram");
                (networks, now, system, rates)
            });
            let (media, result) = tokio::join!(
                tokio::time::timeout(Duration::from_secs(2), crate::widgets::media::status()),
                metrics,
            );
            let Ok((next, now, system, network)) = result else {
                break;
            };
            networks = next;
            last = Some(now);
            let sample = Sample {
                media: media.ok().flatten(),
                system,
                network,
            };
            if out
                .send(Msg::DashboardSample(id, Box::new(sample)))
                .await
                .is_err()
            {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}

fn columns(width: f32) -> usize {
    if width >= 1010.0 {
        3
    } else if width >= 620.0 {
        2
    } else {
        1
    }
}

fn main_height(width: f32) -> f32 {
    (width * 0.28).clamp(312.0, 426.0)
}

fn detail_height(width: f32) -> f32 {
    (width * 0.107).clamp(132.0, 164.0)
}

#[derive(Clone, Copy)]
struct Palette {
    alpha: f32,
    frame: Color,
    dark: Color,
    card: Color,
    inset: Color,
    hover: Color,
    fg: Color,
    muted: Color,
    accent: Color,
    ink: Color,
}

impl Palette {
    fn new(bar: &BarPalette, alpha: f32, opacity: f32) -> Self {
        Self {
            alpha,
            frame: continuite::frame(bar, alpha * opacity),
            dark: color(bar.bg, alpha),
            card: continuite::tone(bar, 0.055, alpha),
            inset: continuite::tone(bar, 0.12, alpha),
            hover: continuite::tone(bar, 0.22, alpha),
            fg: color(bar.fg, alpha),
            muted: continuite::tone(bar, 0.74, alpha),
            accent: color(bar.accent, alpha),
            ink: continuite::selected_ink(bar, alpha),
        }
    }

    fn surface(self, bg: Color, radius: f32) -> container::Style {
        container::Style {
            background: Some(bg.into()),
            border: Border::default().rounded(radius),
            text_color: Some(self.fg),
            ..Default::default()
        }
    }

    fn button(self, selected: bool, radius: f32, status: button::Status) -> button::Style {
        let bg = match (selected, status) {
            (true, button::Status::Pressed) => self.accent.scale_alpha(0.86),
            (true, _) => self.accent,
            (false, button::Status::Hovered | button::Status::Pressed) => self.hover,
            (false, button::Status::Disabled) => self.dark,
            (false, _) => self.inset,
        };
        button::Style {
            background: Some(bg.into()),
            border: Border::default().rounded(radius),
            text_color: if selected {
                self.ink
            } else if status == button::Status::Disabled {
                self.muted
            } else {
                self.fg
            },
            ..Default::default()
        }
    }

    fn scroll(self, theme: &Theme, status: scrollable::Status) -> scrollable::Style {
        let mut style = scrollable::default(theme, status);
        style.vertical_rail.background = None;
        style.vertical_rail.scroller.background = self.hover.into();
        style
    }

    fn slider(self, style: slider::Style) -> slider::Style {
        let mut style = style.with_circular_handle(6.0);
        style.rail.backgrounds = (self.accent.into(), self.hover.into());
        style.rail.border = Border::default().rounded(4);
        style.rail.width = 5.0;
        style.handle.background = self.accent.into();
        style.handle.border_width = 0.0;
        style
    }
}

impl App {
    pub(super) fn dashboard_height(&self, width: f32) -> f32 {
        if columns(width) < 3 {
            return 850.0;
        }
        let extra = self.control_source("meteo").is_some()
            || self.dashboard_has_clipboard()
            || self.dashboard_battery_value().is_some();
        let rows = if extra { 2.0 } else { 1.0 };
        2.0 * PADDING + 58.0 + main_height(width) + rows * detail_height(width) + (rows + 1.0) * GAP
    }

    fn dashboard_has_clipboard(&self) -> bool {
        self.cfg.general.clipboard_history && crate::policy::get().clipboard_allowed()
    }

    pub(super) fn continuity_dashboard(
        &self,
        popover: &Popover,
        dashboard: &Dashboard,
        alpha: f32,
    ) -> Element<'_, Msg> {
        let opacity = self
            .cfg
            .general
            .theme
            .shown(crate::config::Place::Menus)
            .opacity;
        let pal = Palette::new(&self.bar_palette(), alpha, opacity);
        let width = popover.size.width - 2.0 * SHADOW;
        let cols = columns(width);
        let main_height = main_height(width);
        let detail_height = detail_height(width);
        let monitor = self
            .bar_surfaces
            .get(&popover.bar)
            .map_or("", |b| b.monitor.as_str());
        let side = cols == 3;
        let central_width =
            width - 2.0 * PADDING - if side { 54.0 + 68.0 + 2.0 * GAP } else { 0.0 };
        let card_width = (central_width - (cols - 1) as f32 * GAP) / cols as f32;
        let height = popover.size.height - 2.0 * SHADOW - 2.0 * PADDING;
        let shortcuts = self.dashboard_shortcuts(pal, monitor, side);
        let controls = self.dashboard_controls(pal, popover.bar, monitor, cols);
        let sound = self.dashboard_sound(pal, monitor, side);
        let primary = [
            self.dashboard_media(pal, dashboard, card_width, main_height),
            self.dashboard_calendar(pal, main_height),
            self.dashboard_notifications(pal, main_height),
        ];
        let mut cards = column![].spacing(GAP);
        let mut line = row![].spacing(GAP);
        for (i, card) in primary.into_iter().enumerate() {
            line = line.push(card);
            if (i + 1) % cols == 0 || i == 2 {
                // Une carte seule sur la dernière ligne prend toute la largeur.
                cards = cards.push(line);
                line = row![].spacing(GAP);
            }
        }
        let mut secondary = vec![
            self.dashboard_system(pal, dashboard, detail_height),
            self.dashboard_network(pal, dashboard, detail_height),
        ];
        if let Some(weather) = self.dashboard_weather(pal, detail_height) {
            secondary.push(weather);
        }
        secondary.push(self.dashboard_workspaces(pal, popover.bar, monitor, detail_height));
        if self.dashboard_has_clipboard() {
            secondary.push(self.dashboard_clipboard(pal, detail_height));
        }
        if let Some(battery) = self.dashboard_battery(pal, detail_height) {
            secondary.push(battery);
        }
        let mut cells = secondary.into_iter();
        loop {
            let elements: Vec<_> = cells.by_ref().take(cols).collect();
            if elements.is_empty() {
                break;
            }
            cards = cards.push(row(elements).spacing(GAP));
        }

        let body: Element<'_, Msg> = if side {
            let main = column![
                controls,
                scrollable(cards)
                    .id(keyboard::SCROLL)
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .style(move |theme, status| pal.scroll(theme, status)),
            ]
            .spacing(GAP)
            .width(Length::Fill)
            .height(Length::Fill);
            row![shortcuts, main, sound].spacing(GAP).into()
        } else {
            scrollable(column![controls, shortcuts, sound, cards].spacing(GAP))
                .id(keyboard::SCROLL)
                .style(move |theme, status| pal.scroll(theme, status))
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };
        container(body)
            .id(keyboard::PANE)
            .padding(PADDING)
            .width(Length::Fill)
            .height(height + 2.0 * PADDING)
            .style(move |_| pal.surface(pal.frame, 26.0))
            .into()
    }

    fn dash_label(&self, label: impl Into<String>, size: f32, fg: Color) -> iced::widget::Text<'_> {
        text(label.into())
            .font(self.font)
            .size(crate::ui::tokens::ts(size))
            .color(fg)
    }

    fn dash_icon(&self, name: &str, fallback: &str, size: f32, fg: Color) -> Element<'_, Msg> {
        self.icon_named(name, size, fg)
            .unwrap_or_else(|| self.dash_label(fallback, size, fg).into())
    }

    fn dash_button<'a>(
        &self,
        pal: Palette,
        content: Element<'a, Msg>,
        action: Option<&Msg>,
        selected: bool,
        square: Option<f32>,
    ) -> button::Button<'a, Msg> {
        let mut b = button(content)
            .padding([7, 11])
            .on_press_maybe(action.cloned())
            .style(move |_, status| pal.button(selected, 15.0, status));
        if let Some(size) = square {
            b = b.width(size).height(size).padding(0);
        }
        b
    }

    fn dash_icon_button(
        &self,
        pal: Palette,
        glyph: (&str, &str, &str),
        action: Option<Msg>,
        selected: bool,
        size: f32,
    ) -> Element<'_, Msg> {
        let (icon, fallback, label) = glyph;
        let fg = if selected {
            pal.ink
        } else if action.is_some() {
            pal.fg
        } else {
            pal.muted
        };
        let centered = container(self.dash_icon(icon, fallback, 21.0, fg))
            .center_x(Length::Fill)
            .center_y(Length::Fill);
        let b = self.dash_button(pal, centered.into(), action.as_ref(), selected, Some(size));
        self.dash_tooltip(
            pal,
            keyboard::press(b, action, if selected { pal.ink } else { pal.fg }),
            label,
        )
    }

    /// Info-bulle d'une commande du tableau de bord : son libellé suit le curseur.
    fn dash_tooltip<'a>(
        &'a self,
        pal: Palette,
        content: Element<'a, Msg>,
        label: impl Into<String>,
    ) -> Element<'a, Msg> {
        tooltip(
            content,
            self.dash_label(label, 12.0, pal.fg),
            tooltip::Position::FollowCursor,
        )
        .style(move |_| pal.surface(pal.dark, 6.0))
        .into()
    }

    fn dash_title(&self, pal: Palette, title: &str) -> Element<'_, Msg> {
        container(
            text(tr(title).to_uppercase())
                .font(self.title_font())
                .size(crate::ui::tokens::ts(15.0))
                .color(pal.fg),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(move |_| pal.surface(pal.dark, 15.0))
        .into()
    }

    fn dash_card<'a>(
        &self,
        pal: Palette,
        content: impl Into<Element<'a, Msg>>,
        height: f32,
    ) -> Element<'a, Msg> {
        container(content)
            .width(Length::Fill)
            .height(height)
            .padding(8)
            .clip(true)
            .style(move |_| pal.surface(pal.card, 22.0))
            .into()
    }

    fn dashboard_shortcuts(&self, pal: Palette, monitor: &str, vertical: bool) -> Element<'_, Msg> {
        let command = |cmd| Msg::On(monitor.into(), Box::new(Msg::DashboardCommand(cmd)));
        let items = [
            self.dash_icon_button(
                pal,
                ("user-home-symbolic", "⌂", tr("Centre de contrôle")),
                None,
                true,
                48.0,
            ),
            self.dash_icon_button(
                pal,
                ("applications-graphics-symbolic", "◈", tr("Apparence")),
                Some(command(Command::Settings("theme".into()))),
                false,
                46.0,
            ),
            self.dash_icon_button(
                pal,
                ("power-profile-performance-symbolic", "∿", tr("Performance")),
                Some(command(Command::Settings("performance".into()))),
                false,
                46.0,
            ),
            self.dash_icon_button(
                pal,
                ("video-display-symbolic", "▣", tr("Écrans")),
                Some(command(Command::Settings("screens".into()))),
                false,
                46.0,
            ),
            self.dash_icon_button(
                pal,
                ("view-grid-symbolic", "▦", tr("Espaces de travail")),
                Some(command(Command::Overview)),
                false,
                46.0,
            ),
        ];
        let settings = self.dash_icon_button(
            pal,
            ("preferences-system-symbolic", "⚙", tr("Réglages")),
            Some(command(Command::Settings("desktop".into()))),
            false,
            48.0,
        );
        if vertical {
            column(items)
                .push(Space::new().height(Length::Fill))
                .push(settings)
                .height(Length::Fill)
                .width(54)
                .align_x(Alignment::Center)
                .spacing(10)
                .into()
        } else {
            row(items)
                .push(Space::new().width(Length::Fill))
                .push(settings)
                .align_y(Alignment::Center)
                .spacing(6)
                .into()
        }
    }

    fn dashboard_controls(
        &self,
        pal: Palette,
        bar: iced::window::Id,
        monitor: &str,
        cols: usize,
    ) -> Element<'_, Msg> {
        let clock = self
            .dash_label(format_now("%H:%M"), 27.0, pal.fg)
            .font(self.title_font());
        let date = self.dash_label(format_now("%A %-d %B"), 14.0, pal.muted);
        let close = self.dash_icon_button(
            pal,
            ("window-close-symbolic", "×", tr("Fermer")),
            Some(Msg::ClosePopover),
            false,
            36.0,
        );
        let mut title = row![
            self.dash_label("Hyprdesk", 22.0, pal.fg)
                .font(self.title_font()),
            clock
        ]
        .align_y(Alignment::Center)
        .spacing(14);
        if cols > 1 {
            title = title.push(date);
        }
        let wifi = self.dashboard_connection(
            pal,
            bar,
            monitor,
            "network",
            "Wi-Fi",
            "network-wireless-symbolic",
        );
        let bluetooth = self.dashboard_connection(
            pal,
            bar,
            monitor,
            "bluetooth",
            "Bluetooth",
            "bluetooth-symbolic",
        );
        // Allumée, la tuile nomme sa fin et un clic l'éteint ; éteinte, elle
        // ouvre le choix de la fin — choisir, c'est allumer (HIG PAT-04).
        let silence = self.toasts.dnd;
        let silence_icon = if silence {
            "notifications-disabled-symbolic"
        } else {
            "preferences-system-notifications-symbolic"
        };
        let silence_label = match self.toasts.dnd_until {
            Some(until) => crate::tr!(
                "Silence · jusqu'à {hour}",
                hour = crate::notifications::silence_end_label(until)
            ),
            None => tr("Silence").to_string(),
        };
        let silence_button = if self.toasts.active || self.preview {
            let action = if silence {
                Msg::Control(Command::Notifications("dnd off".into()))
            } else {
                Msg::OpenPopover(bar, "silence".into(), Request::Silence)
            };
            self.dashboard_pill(pal, silence_icon, &silence_label, silence, Some(action))
        } else if let Some(action) = self
            .control_source("notifications")
            .and_then(|widget| Self::dashboard_widget_menu(bar, widget))
        {
            self.dashboard_pill(pal, silence_icon, tr("Notifications"), false, Some(action))
        } else {
            Space::new().into()
        };
        let lock = self.dash_icon_button(
            pal,
            ("system-lock-screen-symbolic", "◇", tr("Verrouiller")),
            Some(Msg::Popover(PMsg::Spawn("hyprdesk lock".into()))),
            false,
            38.0,
        );
        let actions = row![wifi, bluetooth, silence_button, lock]
            .spacing(6)
            .align_y(Alignment::Center);
        let header: Element<'_, Msg> = if cols == 3 {
            title
                .push(Space::new().width(Length::Fill))
                .push(actions)
                .push(close)
                .into()
        } else {
            let top = title.push(Space::new().width(Length::Fill)).push(close);
            column![top, actions.wrap()].spacing(8).into()
        };
        container(header)
            .padding([8, 10])
            .width(Length::Fill)
            .style(move |_| pal.surface(pal.dark, 22.0))
            .into()
    }

    fn dashboard_pill(
        &self,
        pal: Palette,
        icon: &str,
        label: &str,
        selected: bool,
        action: Option<Msg>,
    ) -> Element<'_, Msg> {
        let fg = if selected {
            pal.ink
        } else if action.is_some() {
            pal.fg
        } else {
            pal.muted
        };
        let body = row![
            self.dash_icon(icon, "•", 17.0, fg),
            self.dash_label(label, 13.0, fg)
                .font(self.ui_font(iced::font::Weight::Semibold))
        ]
        .align_y(Alignment::Center)
        .spacing(7);
        let button = self
            .dash_button(pal, body.into(), action.as_ref(), selected, None)
            .height(39);
        keyboard::press(button, action, if selected { pal.ink } else { pal.fg })
    }

    fn dashboard_connection(
        &self,
        pal: Palette,
        bar: iced::window::Id,
        monitor: &str,
        type_id: &str,
        label: &str,
        fallback: &str,
    ) -> Element<'_, Msg> {
        let Some(widget) = self.control_source(type_id) else {
            return Space::new().into();
        };
        let value = self.values.get(&widget.id);
        let rich = match value {
            Some(Value::Rich(r)) if r.state == RenderState::Hidden => return Space::new().into(),
            Some(Value::Rich(r)) => Some(r),
            _ => None,
        };
        let pending = self
            .control_actions
            .get(&widget.id)
            .is_some_and(Option::is_none);
        let error = self
            .control_actions
            .get(&widget.id)
            .and_then(|e| e.as_deref());
        let active = rich.is_some_and(|r| r.toggled == Some(true));
        let menu = Self::dashboard_widget_menu(bar, widget);
        let can_toggle = matches!(
            widget.kind,
            WidgetKind::Lua {
                toggle: Some(_),
                ..
            }
        ) && rich.is_some_and(|r| r.toggled.is_some());
        let action = if can_toggle {
            Msg::ControlToggle(widget.id.clone())
        } else {
            menu.clone().unwrap_or_else(|| {
                Msg::On(
                    monitor.into(),
                    Box::new(Msg::DashboardCommand(Command::Settings(type_id.into()))),
                )
            })
        };
        let label = if pending {
            format!("{} …", tr(label))
        } else {
            tr(label).into()
        };
        let pill = self.dashboard_pill(
            pal,
            rich.and_then(|r| r.icon.as_deref()).unwrap_or(fallback),
            &label,
            active,
            (!pending).then_some(action),
        );
        let detail = error.map(str::to_owned).or_else(|| match value {
            Some(Value::Error(e)) => Some(e.clone()),
            _ => rich.and_then(|r| r.tooltip.clone()),
        });
        let pill = match detail {
            Some(detail) => tooltip(
                pill,
                self.dash_label(detail, 12.0, pal.fg),
                tooltip::Position::Bottom,
            )
            .style(move |_| pal.surface(pal.dark, 8.0))
            .into(),
            None => pill,
        };
        if can_toggle && let Some(menu) = menu {
            let detail = format!("{} · {label}", tr("Détails"));
            row![
                pill,
                self.dash_icon_button(
                    pal,
                    ("go-next-symbolic", "›", &detail),
                    Some(menu),
                    false,
                    29.0,
                )
            ]
            .spacing(3)
            .align_y(Alignment::Center)
            .into()
        } else {
            pill
        }
    }

    fn dashboard_widget_menu(
        bar: iced::window::Id,
        widget: &crate::config::WidgetDef,
    ) -> Option<Msg> {
        let WidgetKind::Lua {
            menu: Some(func), ..
        } = widget.kind
        else {
            return None;
        };
        Some(Msg::OpenPopover(
            bar,
            widget.id.clone(),
            Request::LuaMenu {
                widget: widget.id.clone(),
                func,
            },
        ))
    }

    fn dashboard_sound(&self, pal: Palette, monitor: &str, vertical: bool) -> Element<'_, Msg> {
        let light: Option<Element<'_, Msg>> = self.light.target_for(monitor).map(|target| {
            let level = self.light.level(target).unwrap_or_default();
            let set = move |v| Msg::LightSet(target, v);
            let commit = Msg::LightApply(target);
            let label = self.dash_label(format!("{:.0} %", 100.0 * level), 12.0, pal.fg);
            let icon = self.dash_icon("display-brightness-symbolic", "☼", 23.0, pal.fg);
            if vertical {
                let control = vertical_slider(0.0..=1.0, level, set)
                    .on_release(commit.clone())
                    .step(0.01_f32)
                    .width(28)
                    .height(Length::Fill)
                    .style(move |theme, status| {
                        pal.slider(iced::widget::vertical_slider::default(theme, status))
                    });
                column![
                    container(icon)
                        .center_x(53)
                        .center_y(47)
                        .style(move |_| pal.surface(pal.dark, 16.0)),
                    keyboard::adjust(control, level, 1.0, set, Some(commit), pal.fg),
                    label
                ]
                .align_x(Alignment::Center)
                .spacing(8)
                .height(Length::Fill)
                .into()
            } else {
                let control = slider(0.0..=1.0, level, set)
                    .on_release(commit.clone())
                    .step(0.01_f32)
                    .width(Length::Fill)
                    .style(move |theme, status| pal.slider(slider::default(theme, status)));
                column![
                    self.dash_label(tr("Luminosité"), 12.0, pal.fg),
                    row![
                        icon,
                        keyboard::adjust(control, level, 1.0, set, Some(commit), pal.fg),
                        label
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                ]
                .spacing(2)
                .into()
            }
        });
        let output = self.dashboard_audio(
            pal,
            self.volume,
            super::super::surface::volume_icon(self.volume),
            "Son",
            (Msg::VolumeSet, Msg::VolumeMute),
            vertical,
        );
        let input = self.dashboard_audio(
            pal,
            self.audio.mic,
            super::super::surface::mic_icon(self.audio.mic),
            "Micro",
            (Msg::MicSet, Msg::MicMute),
            vertical,
        );
        if vertical {
            let mut col = column![]
                .width(68)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .spacing(8);
            col = if let Some(light) = light {
                col.push(light)
            } else {
                col.push(Space::new().height(Length::Fill))
            };
            col.extend(output.into_iter().chain(input)).into()
        } else {
            let col = column(light.into_iter().chain(output).chain(input)).spacing(8);
            container(col)
                .width(Length::Fill)
                .padding([8, 12])
                .style(move |_| pal.surface(pal.card, 18.0))
                .into()
        }
    }

    fn dashboard_audio(
        &self,
        pal: Palette,
        value: Option<(f32, bool)>,
        icon: &str,
        name: &str,
        (set, mute): (fn(f32) -> Msg, Msg),
        vertical: bool,
    ) -> Option<Element<'_, Msg>> {
        let (value, muted) = value?;
        let title = if muted {
            format!("{} · {}", tr(name), tr("Muet"))
        } else {
            format!("{} · {:.0} %", tr(name), value * 100.0)
        };
        if vertical {
            let gauge = iced::widget::stack![
                canvas(ArcGauge {
                    pal,
                    value: if muted { 0.0 } else { value.min(1.0) },
                    disc: false
                })
                .width(57)
                .height(57),
                container(self.dash_icon(icon, "♪", 19.0, if muted { pal.muted } else { pal.fg }))
                    .center_x(57)
                    .center_y(57),
            ];
            let action = self
                .dash_button(pal, gauge.into(), Some(&mute), false, Some(57.0))
                .style(move |_, status| {
                    let mut style = pal.button(false, 29.0, status);
                    if status == button::Status::Active {
                        style.background = Some(pal.dark.into());
                    }
                    style
                });
            let adjust = slider(0.0..=1.5, value.min(1.5), set)
                .step(0.01_f32)
                .width(48)
                .style(move |theme, status| pal.slider(slider::default(theme, status)));
            Some(
                column![
                    tooltip(
                        keyboard::press(action, Some(mute), pal.fg),
                        self.dash_label(title, 12.0, pal.fg),
                        tooltip::Position::Left
                    )
                    .style(move |_| pal.surface(pal.dark, 8.0)),
                    keyboard::adjust(adjust, value, 1.5, set, None, pal.fg)
                ]
                .align_x(Alignment::Center)
                .spacing(2)
                .into(),
            )
        } else {
            let action = self.dash_icon_button(pal, (icon, "♪", &title), Some(mute), muted, 32.0);
            let adjust = slider(0.0..=1.5, value.min(1.5), set)
                .step(0.01_f32)
                .style(move |theme, status| pal.slider(slider::default(theme, status)));
            Some(
                row![
                    action,
                    self.dash_label(title, 12.0, pal.fg).width(112),
                    keyboard::adjust(adjust, value, 1.5, set, None, pal.fg)
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            )
        }
    }

    fn dashboard_media(
        &self,
        pal: Palette,
        dashboard: &Dashboard,
        width: f32,
        height: f32,
    ) -> Element<'_, Msg> {
        let media = dashboard.media.as_ref();
        let title = media
            .map(|m| {
                if m.title.is_empty() {
                    m.player.as_str()
                } else {
                    m.title.as_str()
                }
            })
            .unwrap_or(tr("Aucune lecture"));
        let artist = media.map(|m| m.artist.as_str()).unwrap_or("");
        let playing = media.is_some_and(|m| m.playing);
        let time = media
            .and_then(|m| Some((m.position?, m.length?)))
            .filter(|&(_, len)| len > 0);
        let ratio = time.map_or(0.0, |(pos, len)| (pos as f32 / len as f32).clamp(0.0, 1.0));
        let format_time =
            |seconds: i64| format!("{}:{:02}", seconds.max(0) / 60, seconds.max(0) % 60);
        let time = time.map_or("–:–– / –:––".into(), |(pos, len)| {
            format!("{} / {}", format_time(pos), format_time(len))
        });
        let action = |method| media.map(|m| Msg::Media(m.bus.clone(), method));
        let play = if playing {
            ("media-playback-pause-symbolic", "Ⅱ", tr("Pause"))
        } else {
            ("media-playback-start-symbolic", "▶", tr("Lecture"))
        };
        let center = row![
            self.dash_icon_button(
                pal,
                ("media-skip-backward-symbolic", "‹", tr("Précédent")),
                action("Previous"),
                false,
                36.0
            ),
            self.dash_icon_button(pal, play, action("PlayPause"), media.is_some(), 44.0),
            self.dash_icon_button(
                pal,
                ("media-skip-forward-symbolic", "›", tr("Suivant")),
                action("Next"),
                false,
                36.0
            ),
        ]
        .spacing(9)
        .align_y(Alignment::Center);
        let size = (width - 40.0).min(height * 0.56).clamp(126.0, 238.0);
        let body = column![
            canvas(ArcGauge {
                pal,
                value: ratio,
                disc: true
            })
            .width(size)
            .height(size),
            self.dash_label(super::super::surface::truncate(title, 32), 21.0, pal.fg)
                .font(self.title_font()),
            self.dash_label(super::super::surface::truncate(artist, 40), 13.0, pal.muted),
            center,
            self.dash_label(time, 13.0, pal.muted),
        ]
        .padding([8, 0])
        .spacing(6)
        .align_x(Alignment::Center)
        .width(Length::Fill);
        self.dash_card(
            Palette {
                card: pal.dark,
                ..pal
            },
            body,
            height,
        )
    }

    fn dashboard_calendar(&self, pal: Palette, height: f32) -> Element<'_, Msg> {
        let today = chrono::Local::now().date_naive();
        let first = crate::ui::widget_view::month_start(today, self.cal_months);
        let monday =
            first - chrono::Duration::days(i64::from(first.weekday().num_days_from_monday()));
        let month = first
            .format_localized("%B %Y", crate::i18n::chrono_locale())
            .to_string();
        let prev = self.dash_icon_button(
            pal,
            ("go-previous-symbolic", "‹", tr("Mois précédent")),
            Some(Msg::CalendarMonth(-1)),
            false,
            36.0,
        );
        let next = self.dash_icon_button(
            pal,
            ("go-next-symbolic", "›", tr("Mois suivant")),
            Some(Msg::CalendarMonth(1)),
            false,
            36.0,
        );
        let month = self
            .dash_label(month.to_uppercase(), 14.0, pal.fg)
            .font(self.title_font())
            .width(Length::Fill)
            .align_x(Alignment::Center);
        let header = container(row![prev, month, next].align_y(Alignment::Center))
            .style(move |_| pal.surface(pal.dark, 16.0));
        let head = row((0..7).map(|i| {
            let day = (monday + chrono::Duration::days(i))
                .format_localized("%a", crate::i18n::chrono_locale())
                .to_string();
            let day: String = day.trim_end_matches('.').chars().take(3).collect();
            self.dash_label(day.to_uppercase(), 12.0, pal.muted)
                .font(self.title_font())
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .into()
        }))
        .height(27);
        let mut grid = column![head].spacing(3);
        let cell_height = ((height - 131.0) / 6.0).clamp(28.0, 40.0);
        for week in 0..6 {
            grid = grid.push(row((0..7).map(|day| {
                let date = monday + chrono::Duration::days(week * 7 + day);
                let current = date == today;
                let fg = if current {
                    pal.ink
                } else if date.month() == first.month() {
                    pal.fg
                } else {
                    Color {
                        a: 0.72 * pal.alpha,
                        ..pal.muted
                    }
                };
                let label = self.dash_label(date.day().to_string(), 14.0, fg);
                container(
                    container(label)
                        .center_x(cell_height)
                        .center_y(cell_height)
                        .style(move |_| {
                            if current {
                                pal.surface(pal.accent, cell_height / 2.0)
                            } else {
                                container::Style::default()
                            }
                        }),
                )
                .center_x(Length::Fill)
                .into()
            })));
        }
        let grid = container(grid)
            .padding([12, 5])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| pal.surface(pal.dark, 16.0));
        self.dash_card(pal, column![header, grid].spacing(7), height)
    }

    fn dashboard_notifications(&self, pal: Palette, height: f32) -> Element<'_, Msg> {
        let dnd = self.toasts.dnd;
        let header = row![
            self.dash_title(pal, "Notifications"),
            self.dash_icon_button(
                pal,
                ("edit-clear-symbolic", "×", tr("Effacer l’historique")),
                (!self.toasts.history.is_empty())
                    .then(|| Msg::Control(Command::Notifications("clear".into()))),
                false,
                34.0
            )
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        // Aperçus masqués (HIG SYS-05) : un texte générique au lieu du corps.
        let masque = crate::design_options::flag(&self.cfg.general, "notifications.hide_previews");
        let history = &self.toasts.history;
        let mut items = column![].spacing(8);
        // Une boîte par application, son nom en en-tête (HIG LAY-02). La boîte
        // porte le seul cadre : les tuiles n'ont plus de fond au repos.
        for (app, ranks) in crate::ui::toasts::app_groups(&history[..history.len().min(12)]) {
            let mut tiles = column![].width(Length::Fill);
            for i in ranks {
                let n = &history[i];
                let title = super::super::surface::truncate(&n.summary, 42);
                // Détail : le corps seul ; le nom de l'application est en en-tête.
                let detail = if n.body.is_empty() {
                    None
                } else if masque {
                    Some(crate::i18n::tr("Notification").to_string())
                } else {
                    Some(super::super::surface::truncate(
                        &n.body.replace('\n', " "),
                        74,
                    ))
                };
                // Tuile cliquable quand elle mène quelque part : l'action de
                // l'application, ou l'application elle-même. Une tuile du centre
                // comme les autres, au clavier aussi.
                let press = crate::ui::toasts::card_press(n);
                let mut caption =
                    column![self.dash_label(title, 14.0, pal.fg).font(self.title_font())]
                        .spacing(2);
                if let Some(detail) = detail {
                    caption = caption.push(self.dash_label(detail, 12.0, pal.muted));
                }
                let card = button(
                    row![
                        self.notif_icon(n, crate::ui::toasts::CARD_ICON, pal.muted),
                        caption
                    ]
                    .align_y(Alignment::Center)
                    .spacing(12),
                )
                .width(Length::Fill)
                .padding([8, 10])
                .on_press_maybe(press.clone())
                .style(move |_, status: button::Status| {
                    // Survol et appui se voient sur le fond de la boîte.
                    let bg = matches!(status, button::Status::Hovered | button::Status::Pressed)
                        .then(|| pal.hover.into());
                    button::Style {
                        background: bg,
                        border: Border::default().rounded(12.0),
                        text_color: pal.fg,
                        ..Default::default()
                    }
                });
                tiles = tiles.push(keyboard::press(card, press, pal.fg));
            }
            items = items.push(
                container(
                    column![
                        container(
                            self.dash_label(app, 11.0, pal.muted)
                                .font(self.title_font()),
                        )
                        .padding([2, 10]),
                        tiles,
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                )
                .padding(4)
                .width(Length::Fill)
                .style(move |_| pal.surface(pal.inset, 12.0)),
            );
        }
        let body: Element<'_, Msg> = if self.toasts.history.is_empty() {
            let (icon, label) = if dnd {
                // Le Silence à fin programmée dit quand il s'éteint (HIG PAT-04).
                let label = match self.toasts.dnd_until {
                    Some(until) => crate::tr!(
                        "Ne pas déranger · jusqu'à {hour}",
                        hour = crate::notifications::silence_end_label(until)
                    ),
                    None => tr("Ne pas déranger").to_string(),
                };
                ("notifications-disabled-symbolic", label)
            } else {
                (
                    "preferences-system-notifications-symbolic",
                    tr("Aucune notification").to_string(),
                )
            };
            container(
                column![
                    self.dash_icon(icon, "•", 38.0, pal.muted),
                    self.dash_label(label, 13.0, pal.muted)
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            scrollable(items)
                .style(move |theme, status| pal.scroll(theme, status))
                .height(Length::Fill)
                .into()
        };
        self.dash_card(pal, column![header, body].spacing(9), height)
    }

    fn dashboard_system(
        &self,
        pal: Palette,
        dashboard: &Dashboard,
        height: f32,
    ) -> Element<'_, Msg> {
        let mut lines = column![self.dash_title(pal, "Système")].spacing(6);
        for stat in &dashboard.system {
            let samples = match stat.label.as_str() {
                "CPU" => Some(&dashboard.cpu),
                "GPU" => Some(&dashboard.gpu),
                _ => None,
            };
            let gauge = match (samples.filter(|s| s.len() >= 2), stat.ratio) {
                (Some(s), _) => self.dashboard_trace(pal, s, 1.0, 14.0),
                (_, Some(v)) => self.dashboard_meter(pal, v),
                _ => Space::new().width(Length::Fill).into(),
            };
            lines = lines.push(
                row![
                    self.dash_label(stat.label.clone(), 12.0, pal.fg).width(32),
                    gauge,
                    self.dash_label(stat.value.clone(), 12.0, pal.fg)
                        .width(108)
                        .align_x(Alignment::End)
                ]
                .align_y(Alignment::Center)
                .spacing(7)
                .padding([0, 6]),
            );
        }
        self.dash_card(pal, lines, height)
    }

    fn dashboard_network(
        &self,
        pal: Palette,
        dashboard: &Dashboard,
        height: f32,
    ) -> Element<'_, Msg> {
        let content: Element<'_, Msg> = if let Some((rx, tx)) = dashboard.network {
            let rate = |n: f32| {
                if n >= 1_000_000.0 {
                    format!("{:.1} {}", n / 1_000_000.0, tr("Mo/s"))
                } else {
                    format!("{:.0} {}", n / 1000.0, tr("ko/s"))
                }
            };
            let max = dashboard.traffic.iter().fold(1.0f32, |max, &v| max.max(v));
            column![
                row![
                    self.dash_label(format!("↓ {}", rate(rx)), 14.0, pal.fg)
                        .width(Length::Fill),
                    self.dash_label(format!("↑ {}", rate(tx)), 14.0, pal.fg)
                        .align_x(Alignment::End),
                ],
                self.dashboard_trace(pal, &dashboard.traffic, max, 37.0)
            ]
            .spacing(6)
            .padding([4, 6])
            .into()
        } else {
            self.dash_label(tr("Débit indisponible"), 13.0, pal.muted)
                .into()
        };
        self.dash_card(
            pal,
            column![self.dash_title(pal, "Réseau"), content].spacing(8),
            height,
        )
    }

    fn dashboard_weather(&self, pal: Palette, height: f32) -> Option<Element<'_, Msg>> {
        let value = self.values.get(&self.control_source("meteo")?.id)?;
        let summary = match value {
            Value::Text(t) => t.clone(),
            Value::Rich(r) if r.state != RenderState::Hidden => {
                r.tooltip.clone().unwrap_or_else(|| r.text.clone())
            }
            _ => return None,
        };
        let body = row![
            self.dash_icon("weather-overcast-symbolic", "☁", 33.0, pal.fg),
            self.dash_label(summary.trim(), 15.0, pal.fg)
        ]
        .align_y(Alignment::Center)
        .spacing(10)
        .padding([5, 6]);
        Some(self.dash_card(
            pal,
            column![self.dash_title(pal, "Météo"), body].spacing(7),
            height,
        ))
    }

    fn dashboard_workspaces(
        &self,
        pal: Palette,
        bar: iced::window::Id,
        monitor: &str,
        height: f32,
    ) -> Element<'_, Msg> {
        let current = self
            .monitors
            .iter()
            .find(|m| m.name == monitor)
            .map_or("", |m| m.workspace.as_str());
        let mut spaces: Vec<_> = self
            .workspaces
            .iter()
            .filter(|w| w.monitor == monitor)
            .collect();
        spaces.sort_by_key(|w| w.id);
        let mut line = row![].spacing(5).align_y(Alignment::Center);
        let focus = spaces
            .iter()
            .position(|w| w.name == current)
            .unwrap_or_default();
        let start = focus.saturating_sub(3).min(spaces.len().saturating_sub(5));
        for w in spaces.iter().skip(start).take(5) {
            let active = w.name == current;
            let action = Msg::BarWorkspace(monitor.into(), w.name.clone());
            let cell = self
                .dash_label(
                    super::super::surface::truncate(&w.name, 4),
                    14.0,
                    if active { pal.ink } else { pal.fg },
                )
                .width(Length::Fill)
                .align_x(Alignment::Center);
            let button = self
                .dash_button(pal, cell.into(), Some(&action), active, None)
                .width(Length::Fill)
                .height(43)
                .padding([4, 3]);
            // La cellule ne garde que quatre signes du nom : l'info-bulle dit
            // lequel (HIG SEL-05).
            line = line.push(self.dash_tooltip(
                pal,
                keyboard::press(button, Some(action), if active { pal.ink } else { pal.fg }),
                w.name.clone(),
            ));
        }
        if spaces.len() > 5 || spaces.is_empty() {
            line = line.push(self.dash_icon_button(
                pal,
                ("view-grid-symbolic", "…", tr("Espaces de travail")),
                Some(Msg::OpenPopover(
                    bar,
                    "workspaces".into(),
                    Request::Workspaces,
                )),
                false,
                38.0,
            ));
        }
        let desktop = self.monitor_mode(monitor);
        let mut modes = row![].spacing(4);
        for (label, state, command) in [
            ("Masqué", DesktopMode::NoDesktop, Command::Hide),
            ("Lanceur", DesktopMode::LauncherOnly, Command::LauncherOnly),
            ("Complet", DesktopMode::FullDesktop, Command::Show),
        ] {
            let active = desktop == state;
            let content = self.dash_label(tr(label), 10.0, if active { pal.ink } else { pal.fg });
            let action = Msg::On(monitor.into(), Box::new(Msg::Control(command)));
            let button = self
                .dash_button(pal, content.into(), Some(&action), active, None)
                .padding([3, 7]);
            // Un segment, une info-bulle : son libellé (HIG SEL-05).
            modes = modes.push(self.dash_tooltip(
                pal,
                keyboard::press(button, Some(action), if active { pal.ink } else { pal.fg }),
                tr(label),
            ));
        }
        self.dash_card(
            pal,
            column![self.dash_title(pal, "Espaces"), line, modes].spacing(6),
            height,
        )
    }

    fn dashboard_clipboard(&self, pal: Palette, height: f32) -> Element<'_, Msg> {
        let open = self.dashboard_pill(
            pal,
            "edit-paste-symbolic",
            tr("Ouvrir l’historique"),
            false,
            Some(Msg::DashboardCommand(Command::Clipboard)),
        );
        self.dash_card(
            pal,
            column![self.dash_title(pal, "Presse-papiers"), open].spacing(12),
            height,
        )
    }

    fn dashboard_battery(&self, pal: Palette, height: f32) -> Option<Element<'_, Msg>> {
        let r = self.dashboard_battery_value()?;
        let detail = r.tooltip.as_deref().unwrap_or(&r.text);
        let detail = detail.lines().next().unwrap_or("");
        let percent = detail
            .rsplit_once('—')
            .map(|(_, value)| value)
            .unwrap_or(detail)
            .trim()
            .strip_suffix('%')
            .and_then(|v| v.trim().parse::<f32>().ok());
        let label = match percent {
            Some(n) => format!("{n:.0} %"),
            None => detail.into(),
        };
        let mut body = column![
            self.dash_title(pal, "Batterie"),
            self.dash_label(label, 17.0, self.state_color(r.state, pal.fg))
                .width(Length::Fill)
                .align_x(Alignment::End)
        ]
        .spacing(8);
        if let Some(value) = percent {
            body = body.push(self.dashboard_meter(pal, value / 100.0));
        }
        Some(self.dash_card(pal, body, height))
    }

    fn dashboard_battery_value(&self) -> Option<&crate::config::Rendered> {
        match self.values.get(&self.control_source("battery")?.id)? {
            Value::Rich(r) if r.state != RenderState::Hidden => Some(r),
            _ => None,
        }
    }

    fn dashboard_meter(&self, pal: Palette, fraction: f32) -> Element<'_, Msg> {
        let fraction = fraction.clamp(0.0, 1.0);
        let filled = (fraction * 1000.0).round() as u16;
        let mut line = row![].height(8).width(Length::Fill);
        if filled > 0 {
            line = line.push(
                container(Space::new())
                    .width(Length::FillPortion(filled))
                    .height(8)
                    .style(move |_| pal.surface(pal.fg, 4.0)),
            );
        }
        if filled < 1000 {
            line = line.push(Space::new().width(Length::FillPortion(1000 - filled)));
        }
        container(line)
            .style(move |_| pal.surface(pal.hover, 4.0))
            .into()
    }

    fn dashboard_trace(
        &self,
        pal: Palette,
        history: &VecDeque<f32>,
        max: f32,
        height: f32,
    ) -> Element<'_, Msg> {
        canvas(Trace {
            fg: pal.fg,
            values: history.iter().copied().collect(),
            max,
        })
        .width(Length::Fill)
        .height(height)
        .into()
    }
}

struct ArcGauge {
    pal: Palette,
    value: f32,
    disc: bool,
}

impl canvas::Program<Msg> for ArcGauge {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use std::f32::consts::PI;
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let size = bounds.width.min(bounds.height);
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = size * if self.disc { 0.46 } else { 0.40 };
        let (start, sweep, line_width) = if self.disc {
            (PI, PI, 6.0)
        } else {
            (PI * 0.75, PI * 1.5, 3.0)
        };
        let arc = |value: f32| {
            canvas::Path::new(|path| {
                path.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: iced::Radians(start),
                    end_angle: iced::Radians(start + sweep * value),
                })
            })
        };
        let stroke = |fg, width| {
            canvas::Stroke::default()
                .with_color(fg)
                .with_width(width)
                .with_line_cap(canvas::LineCap::Round)
        };
        if self.disc {
            frame.fill(&canvas::Path::circle(center, radius * 0.78), self.pal.inset);
            let width = radius * 0.9;
            let path = canvas::Path::new(|path| {
                path.move_to(Point::new(center.x - width / 2.0, center.y));
                for i in 1..=38 {
                    let x = i as f32 / 38.0;
                    let y = (x * 4.0 * PI).sin() * radius * 0.07;
                    path.line_to(Point::new(center.x + (x - 0.5) * width, center.y - y));
                }
            });
            frame.stroke(&path, stroke(self.pal.fg, 2.5));
        }
        frame.stroke(&arc(1.0), stroke(self.pal.hover, line_width));
        if self.value > 0.0 {
            frame.stroke(&arc(self.value), stroke(self.pal.accent, line_width));
        } else if self.disc {
            frame.stroke(&arc(0.025), stroke(self.pal.accent, line_width));
        }
        vec![frame.into_geometry()]
    }
}

struct Trace {
    fg: Color,
    values: Vec<f32>,
    max: f32,
}

impl canvas::Program<Msg> for Trace {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if self.values.len() < 2 {
            return Vec::new();
        }
        let size = Size::new(bounds.width, bounds.height);
        let mut frame = canvas::Frame::new(renderer, size);
        let point = |i, n: f32| {
            Point::new(
                i as f32 * (size.width - 2.0).max(0.0) / (self.values.len() - 1) as f32 + 1.0,
                1.0 + (size.height - 2.0) * (1.0 - (n / self.max).clamp(0.0, 1.0)),
            )
        };
        let line = canvas::Path::new(|p| {
            p.move_to(point(0, self.values[0]));
            for (i, value) in self.values.iter().enumerate().skip(1) {
                p.line_to(point(i, *value));
            }
        });
        frame.stroke(
            &line,
            canvas::Stroke::default()
                .with_color(self.fg)
                .with_width(1.5)
                .with_line_join(canvas::LineJoin::Round)
                .with_line_cap(canvas::LineCap::Round),
        );
        vec![frame.into_geometry()]
    }
}
