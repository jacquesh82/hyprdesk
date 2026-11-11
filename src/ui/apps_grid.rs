//! Grille des applications (MLG-32, `hyprdesk apps`, bouton du dock) : en
//! plein écran sur l'écran actif, le fond du thème voilé, un champ de
//! recherche, des catégories, et les applications en grandes icônes — les
//! plus lancées d'abord (`crate::search::usage_bonus`). Même moteur que la
//! recherche (`super::search`) : Entrée, clic, flèches, Échap.

use iced::widget::{button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use super::search::{SMsg, Search};
use super::{App, Msg, color};

/// Applications au plus dans la grille.
pub const LIMIT: usize = 400;
/// Colonnes de la grille (les flèches haut et bas sautent d'une rangée).
pub const COLUMNS: usize = 7;
const TILE: f32 = 132.0;
const ICON: f32 = 56.0;

/// Catégories proposées : libellé, catégories freedesktop qui en relèvent.
pub const CATEGORIES: &[(&str, &[&str])] = &[
    ("Internet", &["Network", "WebBrowser", "Email", "Chat"]),
    ("Bureautique", &["Office", "TextEditor", "Documentation"]),
    (
        "Graphisme",
        &["Graphics", "Photography", "2DGraphics", "3DGraphics"],
    ),
    ("Multimédia", &["AudioVideo", "Audio", "Video", "Player"]),
    ("Développement", &["Development", "IDE"]),
    ("Jeux", &["Game"]),
    (
        "Système",
        &["System", "Settings", "Monitor", "TerminalEmulator"],
    ),
    (
        "Utilitaires",
        &["Utility", "Accessories", "FileManager", "Archiving"],
    ),
];

/// L'application (ses catégories) relève-t-elle de la catégorie `c` ?
pub fn in_category(cats: &[String], c: usize) -> bool {
    CATEGORIES
        .get(c)
        .is_some_and(|(_, keys)| cats.iter().any(|x| keys.contains(&x.as_str())))
}

impl App {
    pub(super) fn view_apps_grid<'a>(&'a self, s: &'a Search) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let accent = color(th.accent, 1.0);
        let veil = color(
            th.bg,
            th.effect(crate::config::Place::Launcher).opacity.max(0.82),
        );
        let input =
            iced::widget::text_input(crate::i18n::tr("Rechercher une application…"), &s.query)
                .id(super::search::INPUT_ID)
                .on_input(|q| Msg::Search(SMsg::Query(q)))
                .on_submit(Msg::Search(SMsg::Launch(s.selected)))
                .font(self.font)
                .size(20)
                .padding(14)
                .width(520)
                .style(move |_, _| iced::widget::text_input::Style {
                    background: Color { a: 0.08, ..fg }.into(),
                    border: Border::default()
                        .rounded(16)
                        .width(1)
                        .color(Color { a: 0.25, ..fg }),
                    icon: fg,
                    placeholder: dim,
                    value: fg,
                    selection: Color { a: 0.35, ..accent },
                });
        // Catégories : des pastilles, « Toutes » d'abord.
        let chip = |label: String, on: bool, c: Option<usize>| -> Element<'a, Msg> {
            button(text(label).font(self.font).size(13).color(fg))
                .padding([6, 14])
                .style(move |_, st| button::Style {
                    background: Some(
                        Color {
                            a: if on {
                                0.32
                            } else if matches!(st, button::Status::Hovered) {
                                0.14
                            } else {
                                0.06
                            },
                            ..accent
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(14),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::Search(SMsg::Category(c)))
                .into()
        };
        let mut chips = row![chip(
            crate::i18n::tr("Toutes").to_string(),
            s.category.is_none(),
            None
        )]
        .spacing(8);
        for (i, (label, _)) in CATEGORIES.iter().enumerate() {
            let any = self
                .apps
                .iter()
                .any(|a| a.available && in_category(&a.categories, i));
            if any {
                chips = chips.push(chip(
                    crate::i18n::tr(label).to_string(),
                    s.category == Some(i),
                    Some(i),
                ));
            }
        }
        // Tuiles : grande icône, nom sur deux lignes au plus.
        let tile = |k: usize, i: usize| -> Element<'a, Msg> {
            let a = &self.apps[i];
            let icon: Element<'_, Msg> = a
                .icon
                .as_deref()
                .and_then(|n| self.icon_named(n, ICON, fg))
                .unwrap_or_else(|| {
                    container(
                        text(
                            a.name
                                .chars()
                                .next()
                                .unwrap_or('?')
                                .to_uppercase()
                                .to_string(),
                        )
                        .font(self.title_font())
                        .size(26)
                        .color(fg),
                    )
                    .center_x(ICON)
                    .center_y(ICON)
                    .style(move |_| container::Style {
                        background: Some(Color { a: 0.3, ..accent }.into()),
                        border: Border::default().rounded(14),
                        ..Default::default()
                    })
                    .into()
                });
            let name: String = {
                let n: String = a.name.chars().take(28).collect();
                if a.name.chars().count() > 28 {
                    format!("{n}…")
                } else {
                    n
                }
            };
            let selected = k == s.selected;
            button(
                column![
                    icon,
                    text(name)
                        .font(self.font)
                        .size(13)
                        .color(fg)
                        .align_x(Alignment::Center)
                        .width(Length::Fill)
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            )
            .width(TILE)
            .height(TILE)
            .padding(Padding {
                top: 16.0,
                bottom: 8.0,
                left: 8.0,
                right: 8.0,
            })
            .style(move |_, st| button::Style {
                background: (selected || matches!(st, button::Status::Hovered)).then(|| {
                    Color {
                        a: if selected { 0.22 } else { 0.1 },
                        ..accent
                    }
                    .into()
                }),
                border: Border::default().rounded(18),
                text_color: fg,
                ..Default::default()
            })
            .on_press(Msg::Search(SMsg::Launch(k)))
            .into()
        };
        let mut grid = column![].spacing(10).align_x(Alignment::Center);
        if s.results.is_empty() {
            grid = grid.push(
                text(crate::i18n::tr("Aucune application ne correspond."))
                    .font(self.font)
                    .size(14)
                    .color(dim),
            );
        }
        let mut cur = row![].spacing(10);
        for (k, &i) in s.results.iter().enumerate() {
            cur = cur.push(tile(k, i));
            if (k + 1).is_multiple_of(COLUMNS) {
                grid = grid.push(cur);
                cur = row![].spacing(10);
            }
        }
        if !s.results.len().is_multiple_of(COLUMNS) {
            grid = grid.push(cur);
        }
        let body = column![
            input,
            chips,
            scrollable(container(grid).center_x(Length::Fill)).height(Length::Fill),
            text(crate::i18n::tr(
                "Entrée lance · flèches pour choisir · Échap ferme"
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(22)
        .align_x(Alignment::Center)
        .max_width(COLUMNS as f32 * (TILE + 10.0) + 40.0);
        // Un clic hors du contenu referme la grille.
        let page = container(mouse_area(container(body).padding(24)).on_press(Msg::Noop))
            .center_x(Length::Fill)
            .padding(Padding {
                top: 64.0,
                bottom: 32.0,
                left: 24.0,
                right: 24.0,
            })
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(veil.into()),
                ..Default::default()
            });
        mouse_area(page)
            .on_press(Msg::Control(super::control::Command::Apps))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories() {
        let cats = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let firefox = cats(&["Network", "WebBrowser"]);
        assert!(in_category(&firefox, 0));
        assert!(!in_category(&firefox, 5));
        assert!(in_category(&cats(&["Game", "ActionGame"]), 5));
        assert!(!in_category(&firefox, 99));
    }
}
