//! Listes de workspaces et débordement : navigation propre au conteneur de barre.
use iced::widget::{Space, column, row};
use iced::{Color, Element, Length};

use crate::config::BarItem;
use crate::i18n::text;
use crate::ui::{App, Msg, color};

impl App {
    pub(super) fn pop_overflow<'a>(&'a self, alpha: f32, items: &'a [BarItem]) -> Element<'a, Msg> {
        let Some(p) = &self.popover else {
            return Space::new().into();
        };
        let Some(bar) = self.bar_surfaces.get(&p.bar) else {
            return Space::new().into();
        };
        let fg = color(self.cfg.general.theme.fg, alpha);
        let mut col =
            column![self.pop_header(alpha, crate::i18n::tr("Autres commandes").into())].spacing(8);
        for item in items {
            if matches!(item, BarItem::Workspaces) {
                col = col.push(self.pop_workspaces(alpha));
                continue;
            }
            let label = match item {
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
                    .unwrap_or(id),
                _ => crate::config::BAR_ITEMS
                    .iter()
                    .find(|(id, _)| *id == super::super::bar::bar_item_key(item))
                    .map_or("", |(_, label)| crate::i18n::tr(label)),
            };
            col = col.push(
                column![
                    text(label.to_string())
                        .font(self.font)
                        .size(11)
                        .color(Color {
                            a: 0.65 * alpha,
                            ..fg
                        }),
                    self.bar_item_alpha(p.bar, bar, item, alpha)
                ]
                .spacing(4),
            );
        }
        col.into()
    }

    pub(super) fn pop_workspaces(&self, alpha: f32) -> Element<'_, Msg> {
        let Some(p) = &self.popover else {
            return Space::new().into();
        };
        let Some(bar) = self.bar_surfaces.get(&p.bar) else {
            return Space::new().into();
        };
        let current = self
            .monitors
            .iter()
            .find(|m| m.name == bar.monitor)
            .map(|m| m.workspace.as_str())
            .unwrap_or("");
        let fg = color(self.cfg.general.theme.fg, alpha);
        let mut col =
            column![self.pop_header(alpha, crate::i18n::tr("Workspaces").into())].spacing(4);
        for workspace in self.workspaces.iter().filter(|w| w.monitor == bar.monitor) {
            let active = workspace.name == current;
            col = col.push(
                self.pop_row(
                    alpha,
                    self.icon_named(
                        if active {
                            "object-select-symbolic"
                        } else {
                            "user-desktop-symbolic"
                        },
                        16.0,
                        fg,
                    )
                    .unwrap_or_else(|| Space::new().into()),
                    workspace.name.clone(),
                    None,
                    true,
                    true,
                    Some(Msg::BarWorkspace(
                        bar.monitor.clone(),
                        workspace.name.clone(),
                    )),
                ),
            );
        }
        // L'écran d'organisation : les fenêtres à glisser d'un espace à l'autre.
        col = col.push(
            iced::widget::button(
                row![
                    self.icon_named("workspace-switcher", 16.0, fg)
                        .unwrap_or_else(|| Space::new().width(16).height(16).into()),
                    text(crate::i18n::tr("Organiser les fenêtres…"))
                        .font(self.font)
                        .size(13)
                        .color(fg),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
            .padding([6, 10])
            .width(Length::Fill)
            .on_press(Msg::BarOrganize)
            .style(move |_, st| iced::widget::button::Style {
                background: matches!(
                    st,
                    iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
                )
                .then_some(
                    Color {
                        a: 0.10 * alpha,
                        ..fg
                    }
                    .into(),
                ),
                text_color: fg,
                border: iced::Border::default().rounded(6),
                ..Default::default()
            }),
        );
        col.into()
    }
}
