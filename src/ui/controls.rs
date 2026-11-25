//! Composition des réglages existants ; aucune source système supplémentaire.
use iced::widget::{Space, button, column, container, row, slider, tooltip};
use iced::{Alignment, Border, Color, Element, Length};

use crate::config::{RenderState, WidgetDef, WidgetKind};
use crate::i18n::text;
use crate::ui::plugins::bar::popover::Request;
use crate::ui::{App, DesktopMode, Msg, color, design};
use crate::widgets::Value;

/// Symbole d'une commande à bascule quand il dépend de son état : pour le
/// Wi-Fi, celui de sa radio — les ondes allumées, barrées éteintes —, l'état
/// de la connexion se lisant dans la valeur. Sinon rien : le symbole que
/// donne le widget fait foi (HIG SYS-08).
pub(in crate::ui) fn radio_symbol(type_id: &str, toggled: Option<bool>) -> Option<&'static str> {
    match (type_id, toggled) {
        ("network", Some(on)) => Some(wifi_radio(on)),
        _ => None,
    }
}

fn wifi_radio(on: bool) -> &'static str {
    if on {
        "network-wireless-symbolic"
    } else {
        "network-wireless-disabled-symbolic"
    }
}

impl App {
    pub(in crate::ui) fn control_source(&self, type_id: &str) -> Option<&WidgetDef> {
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
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                self.cfg.general.theme.fg
            },
            alpha,
        );
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
        // Passage d'un état à l'autre en fondu (HIG SYS-06) ; pendant
        // l'action, la pastille pulse au lieu d'afficher « En cours… »
        // (HIG SYS-07), sauf sans transition.
        let on = self.toggle_progress(&w.id, active);
        let pulse = if pending {
            self.pending_pulse(&w.id)
        } else {
            None
        };
        let detail = if pending && pulse.is_none() {
            crate::i18n::tr("En cours…").to_string()
        } else if error.is_some() {
            crate::i18n::tr("Indisponible").to_string()
        } else {
            rich.and_then(|r| r.tooltip.clone())
                .unwrap_or_else(|| crate::i18n::tr("Chargement…").to_string())
        };
        let glyph = radio_symbol(type_id, rich.and_then(|r| r.toggled))
            .or_else(|| rich.and_then(|r| r.icon.as_deref()))
            .unwrap_or(if type_id == "network" {
                "network-offline-symbolic"
            } else {
                "bluetooth-symbolic"
            });
        let on_accent = design::on_accent(accent, fg, color(self.cfg.general.theme.bg, alpha));
        let beat = pulse.unwrap_or(1.0);
        let ink = design::mix(fg, on_accent, on).scale_alpha(beat);
        let orb = container(
            // Trait du libellé voisin, en Medium (HIG ICO-12).
            self.icon_named_weighted(glyph, 16.0, ink, iced::font::Weight::Medium)
                .unwrap_or_else(|| Space::new().into()),
        )
        .center_x(32)
        .center_y(32)
        .style(move |_| container::Style {
            background: Some(
                design::mix(
                    Color {
                        a: 0.10 * alpha,
                        ..fg
                    },
                    accent,
                    on,
                )
                .scale_alpha(beat)
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
        // Taille compacte : le symbole seul, qui garde son action ; titre et
        // état en infobulle (HIG SYS-12).
        if crate::design_options::flag(&self.cfg.general, "controls.compact") {
            // L'erreur exacte, que la taille compacte ne montre pas sous la
            // tuile, passe dans l'infobulle.
            let tip = match error {
                Some(e) => format!("{label} · {e}"),
                None => format!("{label} · {}", detail.replace('\n', " · ")),
            };
            let pastille = button(orb)
                .padding(4)
                .on_press_maybe((!pending).then_some(action).flatten())
                .style(move |_, status| iced::widget::button::Style {
                    background: matches!(
                        status,
                        iced::widget::button::Status::Hovered
                            | iced::widget::button::Status::Pressed
                    )
                    .then(|| {
                        Color {
                            a: 0.09 * alpha,
                            ..fg
                        }
                        .into()
                    }),
                    text_color: fg,
                    border: Border::default().rounded(design::SMALL_RADIUS),
                    ..Default::default()
                });
            let bg = color(self.cfg.general.theme.bg, 1.0);
            // Le menu de la commande, sans le bouton « › » : clic droit.
            let mut pastille = iced::widget::mouse_area(pastille);
            if can_toggle && let Some(menu) = menu {
                pastille = pastille.on_right_press(menu);
            }
            let compact: Element<'_, Msg> = tooltip(
                pastille,
                text(tip)
                    .font(self.font)
                    .size(crate::ui::tokens::ts_tooltip(11.0))
                    .color(fg),
                iced::widget::tooltip::Position::Bottom,
            )
            .style(move |_| container::Style {
                text_color: Some(Color { a: 1.0, ..fg }),
                background: Some(bg.into()),
                border: Border::default().rounded(design::SMALL_RADIUS),
                ..Default::default()
            })
            .into();
            return if self.convex_options().is_some() {
                super::kinetic::feedback(compact, fg, self.bar_duration(130))
            } else {
                compact
            };
        }
        let body = row![
            orb,
            column![
                text(label.to_string())
                    .font(self.ui_font(iced::font::Weight::Medium))
                    .size(crate::ui::tokens::ts(12.0))
                    .color(fg),
                text(crate::ui::plugins::bar::surface::truncate(
                    &detail.replace('\n', " · "),
                    15
                ))
                .font(self.font)
                .size(crate::ui::tokens::ts(11.0))
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
        let details = button(
            text("›")
                .font(self.font)
                .size(crate::ui::tokens::ts(17.0))
                .color(fg),
        )
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
                    .size(crate::ui::tokens::ts(11.0))
                    .color(color(self.bar_palette().warning, alpha)),
            );
        }
        if self.convex_options().is_some() {
            super::kinetic::feedback(tile, fg, self.bar_duration(130))
        } else {
            tile.into()
        }
    }

    pub(super) fn controls_view(
        &self,
        alpha: f32,
        monitor: &str,
        bar: Option<iced::window::Id>,
    ) -> Element<'_, Msg> {
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                self.cfg.general.theme.fg
            },
            alpha,
        );
        let mut header = row![
            text("Centre de contrôle")
                .font(self.ui_font(iced::font::Weight::Semibold))
                .size(crate::ui::tokens::ts(15.0))
                .color(fg)
                .width(Length::Fill)
        ]
        .align_y(Alignment::Center);
        if bar.is_some() {
            header = header.push(
                button(
                    // Fermeture : le glyphe du jeu, comme les menus (HIG ICO-07).
                    crate::ui::icons::glyph("window-close", crate::ui::tokens::ts(16.0), fg)
                        .expect("glyphe de fermeture"),
                )
                .padding([5, 7])
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
        // Tuiles en colonnes réglées (MLG-279) : une seule colonne les empile.
        let tiles = [
            self.control_tile(alpha, "network", crate::i18n::tr("Wi-Fi"), menu_bar),
            self.control_tile(alpha, "bluetooth", crate::i18n::tr("Bluetooth"), menu_bar),
        ];
        let tiles: Element<'_, Msg> =
            if crate::design_options::num(&self.cfg.general, "controls.columns") <= 1.0 {
                iced::widget::Column::with_children(tiles).spacing(8).into()
            } else {
                iced::widget::Row::with_children(tiles).spacing(8).into()
            };
        let mut col = column![header, tiles].spacing(14);
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
                            self.icon_named(
                                crate::ui::plugins::bar::surface::volume_icon(self.volume),
                                17.0,
                                fg
                            )
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
                        .size(crate::ui::tokens::ts(12.0))
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
                    .size(crate::ui::tokens::ts(12.0))
                    .color(Color {
                        a: 0.65 * alpha,
                        ..fg
                    }),
            );
        }
        {
            let mode = self.monitor_mode(monitor);
            let mut modes = row![].spacing(3);
            // Chaque segment nomme ce que le mode montre, d'un nom (HIG
            // SEL-06) ; son info-bulle, le nom entier du mode (HIG SEL-05).
            for (label, name, state, command) in [
                (
                    "Fenêtres",
                    "Fenêtres seules",
                    DesktopMode::NoDesktop,
                    crate::control::Command::Hide,
                ),
                (
                    "Lanceur",
                    "Lanceur seul",
                    DesktopMode::LauncherOnly,
                    crate::control::Command::LauncherOnly,
                ),
                (
                    "Bureau",
                    "Bureau complet",
                    DesktopMode::FullDesktop,
                    crate::control::Command::Show,
                ),
            ] {
                let active = mode == state;
                let (label, name) = (crate::i18n::tr(label), crate::i18n::tr(name));
                modes = modes.push(
                    tooltip(
                        button(
                            // Segments de largeur égale, libellé centré
                            // (HIG SEL-13).
                            text(label)
                                .font(self.font)
                                .size(crate::ui::tokens::ts(12.0))
                                .color(fg)
                                .width(Length::Fill)
                                .align_x(iced::alignment::Horizontal::Center),
                        )
                        .padding([8, 6])
                        .width(Length::Fill)
                        .on_press(Msg::On(
                            monitor.to_string(),
                            Box::new(Msg::Control(command)),
                        ))
                        .style(move |_, status| {
                            iced::widget::button::Style {
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
                            }
                        }),
                        text(name)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                            .color(fg),
                        iced::widget::tooltip::Position::Bottom,
                    )
                    .style(move |_| container::Style {
                        text_color: Some(Color { a: 1.0, ..fg }),
                        background: Some(color(self.cfg.general.theme.bg, 1.0).into()),
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
        if let Some(w) = self.control_source("battery")
            && let Some(Value::Rich(r)) = self.values.get(&w.id)
            && r.state != RenderState::Hidden
        {
            col = col.push(self.pop_separator(alpha)).push(
                text(r.tooltip.clone().unwrap_or_else(|| r.text.clone()))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(self.state_color(r.state, fg)),
            );
        }
        col.into()
    }
}

#[cfg(test)]
mod tests {
    use super::radio_symbol;

    /// La tuile « Wi-Fi » a un symbole par état de sa radio ; les autres
    /// commandes gardent celui de leur widget (HIG SYS-08).
    #[test]
    fn symbole_de_la_radio_wifi() {
        assert_eq!(
            radio_symbol("network", Some(true)),
            Some("network-wireless-symbolic")
        );
        assert_eq!(
            radio_symbol("network", Some(false)),
            Some("network-wireless-disabled-symbolic")
        );
        assert_eq!(
            radio_symbol("network", None),
            None,
            "état inconnu : le widget décide"
        );
        assert_eq!(radio_symbol("bluetooth", Some(true)), None);
    }
}
