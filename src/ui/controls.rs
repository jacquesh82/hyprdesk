//! Composition des réglages existants ; aucune source système supplémentaire.
use iced::widget::{Space, button, column, container, row, slider};
use iced::{Alignment, Border, Color, Element, Length};

use super::popover::Request;
use crate::config::{RenderState, WidgetDef, WidgetKind};
use crate::i18n::text;
use crate::ui::{App, DesktopMode, Msg, color, design};
use crate::widgets::Value;

impl App {
    fn control_source(&self, type_id: &str) -> Option<&WidgetDef> {
        self.cfg
            .widgets
            .iter()
            .find(|w| w.type_id == type_id && w.in_bar)
            .or_else(|| self.cfg.widgets.iter().find(|w| w.type_id == type_id))
    }

    fn control_tile(
        &self,
        alpha: f32,
        type_id: &str,
        label: &str,
        bar: Option<iced::window::Id>,
    ) -> Element<'_, Msg> {
        let Some(w) = self.control_source(type_id) else {
            return Space::new().into();
        };
        let rich = match self.values.get(&w.id) {
            Some(Value::Rich(r)) if r.state == RenderState::Hidden => return Space::new().into(),
            Some(Value::Rich(r)) => Some(r),
            _ => None,
        };
        let fg = color(self.cfg.general.theme.fg, alpha);
        let accent = color(self.cfg.general.theme.accent, alpha);
        let pending = self.control_actions.get(&w.id).is_some_and(Option::is_none);
        let error = self
            .control_actions
            .get(&w.id)
            .and_then(|e| e.as_deref())
            .or_else(|| match self.values.get(&w.id) {
                Some(Value::Error(e)) => Some(e.as_str()),
                _ => None,
            });
        let active = rich.is_some_and(|r| r.toggled == Some(true));
        let detail = if pending {
            crate::i18n::tr("En cours…").to_string()
        } else if error.is_some() {
            crate::i18n::tr("Indisponible").to_string()
        } else {
            rich.and_then(|r| r.tooltip.clone())
                .unwrap_or_else(|| crate::i18n::tr("Chargement…").to_string())
        };
        let glyph = rich
            .and_then(|r| r.icon.as_deref())
            .unwrap_or(if type_id == "reseau" {
                "network-offline-symbolic"
            } else {
                "bluetooth-symbolic"
            });
        let on_accent = design::on_accent(accent, fg, color(self.cfg.general.theme.bg, alpha));
        let orb = container(
            self.icon_named(glyph, 16.0, if active { on_accent } else { fg })
                .unwrap_or_else(|| Space::new().into()),
        )
        .center_x(32)
        .center_y(32)
        .style(move |_| container::Style {
            background: Some(
                if active {
                    accent
                } else {
                    Color {
                        a: 0.10 * alpha,
                        ..fg
                    }
                }
                .into(),
            ),
            border: Border::default().rounded(16),
            ..Default::default()
        });
        let menu = match (&w.kind, bar) {
            (
                WidgetKind::Lua {
                    menu: Some(func), ..
                },
                Some(bar),
            ) => Some(Msg::OpenPopover(
                bar,
                w.id.clone(),
                Request::LuaMenu {
                    widget: w.id.clone(),
                    func: *func,
                },
            )),
            _ => Some(Msg::Control(crate::control::Command::Settings(
                type_id.into(),
            ))),
        };
        let can_toggle = matches!(
            w.kind,
            WidgetKind::Lua {
                toggle: Some(_),
                ..
            }
        ) && rich.is_some_and(|r| r.toggled.is_some());
        let action = if can_toggle {
            Some(Msg::ControlToggle(w.id.clone()))
        } else {
            menu.clone()
        };
        let body = row![
            orb,
            column![
                text(label.to_string())
                    .font(self.ui_font(iced::font::Weight::Medium))
                    .size(12.5)
                    .color(fg),
                text(crate::ui::bar::truncate(&detail.replace('\n', " · "), 15))
                    .font(self.font)
                    .size(11)
                    .color(Color {
                        a: 0.65 * alpha,
                        ..fg
                    })
            ]
            .spacing(3)
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let main = button(body)
            .padding([10, 8])
            .width(Length::Fill)
            .on_press_maybe((!pending).then_some(action).flatten())
            .style(move |_, status| iced::widget::button::Style {
                background: Some(
                    Color {
                        a: (match status {
                            iced::widget::button::Status::Pressed => 0.13,
                            iced::widget::button::Status::Hovered => 0.09,
                            _ => 0.06,
                        }) * alpha,
                        ..fg
                    }
                    .into(),
                ),
                text_color: fg,
                border: Border::default().rounded(14),
                ..Default::default()
            });
        let details = button(text("›").font(self.font).size(19).color(fg))
            .padding([8, 4])
            .on_press_maybe(menu)
            .style(move |_, _| iced::widget::button::Style {
                text_color: fg,
                ..Default::default()
            });
        let mut tile = column![row![main, details].align_y(Alignment::Center)]
            .width(Length::Fill)
            .spacing(6);
        if let Some(error) = error {
            tile = tile.push(
                text(error.to_string())
                    .font(self.font)
                    .size(11)
                    .color(color(self.bar_palette().warning, alpha)),
            );
        }
        tile.into()
    }

    pub(super) fn controls_view(
        &self,
        alpha: f32,
        monitor: &str,
        bar: Option<iced::window::Id>,
    ) -> Element<'_, Msg> {
        let fg = color(self.cfg.general.theme.fg, alpha);
        let mut header = row![
            text("Centre de contrôle")
                .font(self.ui_font(iced::font::Weight::Semibold))
                .size(15)
                .color(fg)
                .width(Length::Fill)
        ]
        .align_y(Alignment::Center);
        if bar.is_some() {
            header = header.push(
                button(text("×").font(self.font).size(18).color(fg))
                    .padding([2, 7])
                    .on_press(Msg::ClosePopover)
                    .style(move |_, _| iced::widget::button::Style {
                        text_color: fg,
                        border: Border::default().rounded(12),
                        ..Default::default()
                    }),
            );
        }
        let menu_bar = bar.or_else(|| {
            self.bar_surfaces
                .iter()
                .find(|(_, b)| b.monitor == monitor)
                .map(|(id, _)| *id)
        });
        let mut col = column![
            header,
            row![
                self.control_tile(alpha, "reseau", crate::i18n::tr("Wi-Fi"), menu_bar),
                self.control_tile(alpha, "bluetooth", crate::i18n::tr("Bluetooth"), menu_bar)
            ]
            .spacing(8)
        ]
        .spacing(14);
        if let Some(target) = self.light.target_for(monitor) {
            col = col
                .push(self.pop_header(alpha, crate::i18n::tr("Luminosité").into()))
                .push(self.light_slider(alpha, target));
        }
        if let Some((volume, muted)) = self.volume {
            col = col
                .push(self.pop_header(alpha, crate::i18n::tr("Son").into()))
                .push(
                    row![
                        button(
                            self.icon_named(super::bar::volume_icon(self.volume), 17.0, fg)
                                .unwrap_or_else(|| text("Son").font(self.font).color(fg).into())
                        )
                        .on_press(Msg::VolumeMute)
                        .padding(4)
                        .style(move |_, _| iced::widget::button::Style {
                            text_color: fg,
                            ..Default::default()
                        }),
                        slider(0.0..=1.5f32, volume.min(1.5), Msg::VolumeSet)
                            .step(0.01f32)
                            .width(Length::Fill)
                            .style(move |theme, status| design::bar_slider_alpha(
                                theme, status, alpha
                            )),
                        text(if muted {
                            crate::i18n::tr("Muet").into()
                        } else {
                            format!("{:.0} %", volume * 100.0)
                        })
                        .font(self.font)
                        .size(12)
                        .color(fg)
                        .width(40)
                        .align_x(Alignment::End)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
        } else {
            col = col.push(
                text("Audio indisponible")
                    .font(self.font)
                    .size(12)
                    .color(Color {
                        a: 0.65 * alpha,
                        ..fg
                    }),
            );
        }
        {
            let mode = self.monitor_mode(monitor);
            let mut modes = row![].spacing(3);
            for (label, state, command) in [
                (
                    "Masqué",
                    DesktopMode::NoDesktop,
                    crate::control::Command::Hide,
                ),
                (
                    "Lanceur",
                    DesktopMode::LauncherOnly,
                    crate::control::Command::LauncherOnly,
                ),
                (
                    "Complet",
                    DesktopMode::FullDesktop,
                    crate::control::Command::Show,
                ),
            ] {
                let active = mode == state;
                modes = modes.push(
                    button(text(label).font(self.font).size(12).color(fg))
                        .padding([8, 6])
                        .width(Length::Fill)
                        .on_press(Msg::On(
                            monitor.to_string(),
                            Box::new(Msg::Control(command)),
                        ))
                        .style(move |_, status| iced::widget::button::Style {
                            background: (active
                                || matches!(status, iced::widget::button::Status::Hovered))
                            .then_some(
                                Color {
                                    a: (if active { 0.13 } else { 0.06 }) * alpha,
                                    ..fg
                                }
                                .into(),
                            ),
                            text_color: fg,
                            border: Border::default().rounded(design::SMALL_RADIUS),
                            ..Default::default()
                        }),
                );
            }
            col = col
                .push(self.pop_separator(alpha))
                .push(self.pop_header(alpha, crate::i18n::tr("Affichage du bureau").into()))
                .push(modes);
        }
        if let Some(w) = self.control_source("batterie")
            && let Some(Value::Rich(r)) = self.values.get(&w.id)
            && r.state != RenderState::Hidden
        {
            col = col.push(self.pop_separator(alpha)).push(
                text(r.tooltip.clone().unwrap_or_else(|| r.text.clone()))
                    .font(self.font)
                    .size(12)
                    .color(self.state_color(r.state, fg)),
            );
        }
        col.into()
    }
}
