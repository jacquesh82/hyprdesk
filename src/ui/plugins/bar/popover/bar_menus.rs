//! Listes de workspaces et débordement : navigation propre au conteneur de barre.
use iced::widget::{Space, column};
use iced::{Color, Element};

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
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                self.cfg.general.theme.fg
            },
            alpha,
        );
        let mut col =
            column![self.pop_header(alpha, crate::i18n::tr("Autres commandes").into())].spacing(8);
        for item in items {
            if matches!(item, BarItem::Workspaces) {
                col = col.push(self.pop_workspaces(alpha));
                continue;
            }
            let label = self.bar_item_label(item);
            col = col.push(
                column![
                    text(label.to_string())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(11.0))
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
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                self.cfg.general.theme.fg
            },
            alpha,
        );
        let mut col =
            column![self.pop_header(alpha, crate::i18n::tr("Workspaces").into())].spacing(4);
        for workspace in self.workspaces.iter().filter(|w| w.monitor == bar.monitor) {
            let active = workspace.name == current;
            // Le mode du bureau de cet espace, à droite : celui en place pour
            // l'espace affiché, sinon l'imposé, sinon le mémorisé.
            let mode = if active {
                self.monitor_mode(&bar.monitor)
            } else {
                match self.cfg.general.forced_mode(&bar.monitor, &workspace.name) {
                    Some("no-desktop") => crate::ui::DesktopMode::NoDesktop,
                    Some("launcher-only") => crate::ui::DesktopMode::LauncherOnly,
                    Some("full-desktop") => crate::ui::DesktopMode::FullDesktop,
                    _ => self.ws_mode(&workspace.name),
                }
            };
            let mode = crate::i18n::tr(match mode {
                crate::ui::DesktopMode::NoDesktop => "Fenêtres seules",
                crate::ui::DesktopMode::LauncherOnly => "Lanceur seul",
                crate::ui::DesktopMode::FullDesktop => "Bureau complet",
            })
            .to_string();
            col = col.push(
                self.pop_row(
                    alpha,
                    self.icon_named(
                        if active {
                            "object-select-symbolic"
                        } else {
                            "user-desktop-symbolic"
                        },
                        crate::ui::tokens::glyph(16.0),
                        fg,
                    )
                    .unwrap_or_else(|| Space::new().into()),
                    workspace.name.clone(),
                    Some(mode),
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
        // Une rangée du composant commun, après un séparateur qui la distingue
        // de la liste des espaces (HIG MENU-19).
        col = col.push(self.pop_separator(alpha));
        col.push(
            self.pop_row(
                alpha,
                self.icon_named("workspace-switcher", crate::ui::tokens::glyph(16.0), fg)
                    .unwrap_or_else(|| {
                        let side = crate::ui::tokens::glyph(16.0);
                        Space::new().width(side).height(side).into()
                    }),
                crate::i18n::tr("Organiser les fenêtres…").to_string(),
                None,
                true,
                true,
                Some(Msg::BarOrganize),
            ),
        )
        .into()
    }
}
