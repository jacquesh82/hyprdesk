//! Lanceur d'applications (MLG-32 ; `hyprdesk apps`, `hyprdesk search`,
//! bouton du dock, keys.search) : un seul lanceur, deux présentations.
//!
//! - Plein bureau : une grande feuille, façon Launchpad / tiroir One UI —
//!   recherche en haut, « Suggestions » (les plus lancées) dans un bandeau,
//!   catégories, puis toutes les applications en grandes icônes.
//! - Autres modes : une carte centrée, façon Spotlight — recherche, puis les
//!   suggestions en icônes ; en tapant, les résultats en liste.
//!
//! Surfaces opaques et couleurs à contraste garanti (`theme::readable`) :
//! rien de translucide sous le texte. Même moteur que la recherche
//! (`super::search`) : Entrée, clic, flèches, Échap.

use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Length, Padding, Shadow, Vector};

use super::search::{SMsg, Search};
use super::{App, Msg, color};

/// Applications au plus dans la grille.
pub const LIMIT: usize = 400;
/// Colonnes de la grille (les flèches haut et bas sautent d'une rangée).
pub const COLUMNS: usize = 7;
const TILE: f32 = 128.0;
const ICON: f32 = 60.0;
/// Carte centrée : largeur, lignes de résultats.
pub const SPOT_WIDTH: f32 = 680.0;
const SPOT_ROW: f32 = 48.0;
const SPOT_ROWS: usize = 8;
/// Suggestions de la carte centrée : tuile, icône.
const SPOT_TILE: f32 = 76.0;
const SPOT_ICON: f32 = 46.0;

/// Hauteur de la carte centrée (avec ou sans bandeau de suggestions).
pub fn spot_height(suggestions: bool) -> f32 {
    let field = 58.0;
    let list = SPOT_ROWS as f32 * SPOT_ROW;
    let sugg = if suggestions {
        22.0 + SPOT_TILE + 30.0 + 18.0
    } else {
        0.0
    };
    // Sans requête : suggestions + 5 lignes ; en tapant : 8 lignes.
    let body = (sugg + 22.0 + 5.0 * SPOT_ROW).max(list);
    field + 16.0 + body + 2.0 * 22.0
}

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
    /// Couleurs lisibles du lanceur (thème, contrastes WCAG AA).
    fn launcher_palette(&self) -> (crate::theme::Readable, Color) {
        let th = &self.cfg.general.theme;
        (
            crate::theme::readable(th.fg, th.bg, th.accent),
            color(th.bg, 1.0),
        )
    }

    /// Champ de recherche en pastille, loupe à gauche.
    fn launcher_field<'a>(&'a self, s: &'a Search, width: f32, size: f32) -> Element<'a, Msg> {
        let (r, bg) = self.launcher_palette();
        let (fg, dim, edge, focus) = (
            color(r.fg, 1.0),
            color(r.dim, 1.0),
            color(r.edge, 1.0),
            color(r.focus, 1.0),
        );
        let field_bg = color(
            crate::theme::mix(self.cfg.general.theme.bg, r.fg, 0.06),
            1.0,
        );
        let input = text_input(
            crate::i18n::tr("Rechercher une application… (:clip, :2fa, :mdp)"),
            &s.query,
        )
        .id(super::search::INPUT_ID)
        .on_input(|q| Msg::Search(SMsg::Query(q)))
        .on_submit(Msg::Search(SMsg::Launch(s.selected)))
        .font(self.font)
        .size(size)
        .padding(Padding {
            top: 14.0,
            bottom: 14.0,
            left: 44.0,
            right: 16.0,
        })
        .width(width)
        .style(move |_, st| iced::widget::text_input::Style {
            background: field_bg.into(),
            border: Border::default().rounded(999).width(2).color(
                if matches!(st, iced::widget::text_input::Status::Focused { .. }) {
                    focus
                } else {
                    edge
                },
            ),
            icon: fg,
            placeholder: dim,
            value: fg,
            selection: Color { a: 0.35, ..focus },
        });
        let _ = bg;
        let glass: Element<'_, Msg> = super::icons::get("edit-find")
            .or_else(|| super::icons::get("system-search"))
            .map(|h| {
                iced::widget::svg(h)
                    .width(18)
                    .height(18)
                    .style(move |_, _| iced::widget::svg::Style { color: Some(dim) })
                    .into()
            })
            .unwrap_or_else(|| text("⌕").size(18).color(dim).into());
        iced::widget::stack![
            input,
            container(glass)
                .padding(Padding {
                    left: 16.0,
                    ..Padding::ZERO
                })
                .height(Length::Fill)
                .center_y(Length::Fill),
        ]
        .into()
    }

    /// Icône d'une application, sinon son initiale sur une pastille d'accent.
    fn app_icon<'a>(&'a self, i: usize, size: f32) -> Element<'a, Msg> {
        let (r, _) = self.launcher_palette();
        let a = &self.apps[i];
        a.icon
            .as_deref()
            .and_then(|n| self.icon_named(n, size, color(r.fg, 1.0)))
            .unwrap_or_else(|| {
                let tile = color(r.button, 1.0);
                let on = color(r.on_button, 1.0);
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
                    .size(size * 0.45)
                    .color(on),
                )
                .center_x(size)
                .center_y(size)
                .style(move |_| container::Style {
                    background: Some(tile.into()),
                    border: Border::default().rounded(size * 0.28),
                    ..Default::default()
                })
                .into()
            })
    }

    /// Tuile : icône, nom dessous (deux lignes au plus) ; `msg` au clic.
    fn app_tile<'a>(
        &'a self,
        i: usize,
        tile: f32,
        icon: f32,
        text_size: f32,
        selected: bool,
        msg: Msg,
    ) -> Element<'a, Msg> {
        let (r, _) = self.launcher_palette();
        let (fg, accent) = (color(r.fg, 1.0), color(r.focus, 1.0));
        let name = &self.apps[i].name;
        let max = if tile < 100.0 { 12 } else { 26 };
        let short: String = if name.chars().count() > max {
            format!("{}…", name.chars().take(max - 1).collect::<String>())
        } else {
            name.clone()
        };
        button(
            column![
                self.app_icon(i, icon),
                text(short)
                    .font(self.font)
                    .size(text_size)
                    .color(fg)
                    .align_x(Alignment::Center)
                    .width(Length::Fill)
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .width(tile)
        .padding(Padding {
            top: 12.0,
            bottom: 10.0,
            left: 6.0,
            right: 6.0,
        })
        .style(move |_, st| button::Style {
            background: (selected || matches!(st, button::Status::Hovered)).then(|| {
                Color {
                    a: if selected { 0.24 } else { 0.12 },
                    ..accent
                }
                .into()
            }),
            border: Border::default()
                .rounded(20)
                .width(if selected { 2.0 } else { 0.0 })
                .color(accent),
            text_color: fg,
            ..Default::default()
        })
        .on_press(msg)
        .into()
    }

    /// Titre de section du lanceur (« Suggestions », « Toutes les applications »).
    fn launcher_heading<'a>(&'a self, label: &str) -> Element<'a, Msg> {
        let (r, _) = self.launcher_palette();
        text(crate::i18n::tr(label).to_string())
            .font(self.title_font())
            .size(13)
            .color(color(r.dim, 1.0))
            .into()
    }

    /// Plein bureau : la grande feuille.
    pub(super) fn view_apps_grid<'a>(&'a self, s: &'a Search) -> Element<'a, Msg> {
        let (r, bg) = self.launcher_palette();
        let (fg, dim, edge, accent) = (
            color(r.fg, 1.0),
            color(r.dim, 1.0),
            color(r.edge, 1.0),
            color(r.focus, 1.0),
        );
        let surface = bg;
        let tray = color(
            crate::theme::mix(self.cfg.general.theme.bg, r.focus, 0.10),
            1.0,
        );
        let mut body =
            column![container(self.launcher_field(s, 560.0, 18.0)).center_x(Length::Fill)]
                .spacing(22)
                .align_x(Alignment::Center);
        // Suggestions : les plus lancées, dans un bandeau teinté (sans requête).
        if s.query.trim().is_empty() && s.category.is_none() && !s.suggested.is_empty() {
            let tiles = row(s.suggested.iter().map(|&i| {
                self.app_tile(i, TILE, 64.0, 13.0, false, Msg::Search(SMsg::LaunchApp(i)))
            }))
            .spacing(6);
            body = body.push(
                container(
                    column![self.launcher_heading("Suggestions"), tiles]
                        .spacing(10)
                        .align_x(Alignment::Start),
                )
                .padding([14, 18])
                .style(move |_| container::Style {
                    background: Some(tray.into()),
                    border: Border::default().rounded(24),
                    ..Default::default()
                }),
            );
        }
        // Catégories : des pastilles, « Toutes » d'abord.
        if self.cfg.general.apps_launcher.categories {
            let on_btn = color(r.on_button, 1.0);
            let chip = |label: String, on: bool, c: Option<usize>| -> Element<'a, Msg> {
                button(
                    text(label)
                        .font(self.font)
                        .size(13)
                        .color(if on { on_btn } else { fg }),
                )
                .padding([7, 16])
                .style(move |_, st| button::Style {
                    background: Some(
                        if on {
                            accent
                        } else if matches!(st, button::Status::Hovered) {
                            Color { a: 0.12, ..fg }
                        } else {
                            Color::TRANSPARENT
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(999).width(1).color(if on {
                        accent
                    } else {
                        edge
                    }),
                    text_color: if on { on_btn } else { fg },
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
                if self
                    .apps
                    .iter()
                    .any(|a| a.available && in_category(&a.categories, i))
                {
                    chips = chips.push(chip(
                        crate::i18n::tr(label).to_string(),
                        s.category == Some(i),
                        Some(i),
                    ));
                }
            }
            body = body.push(chips.wrap());
        }
        // Toutes les applications (ou les résultats).
        let mut grid = column![].spacing(6).align_x(Alignment::Start);
        if s.results.is_empty() {
            grid = grid.push(
                text(crate::i18n::tr("Aucune application ne correspond."))
                    .font(self.font)
                    .size(14)
                    .color(dim),
            );
        }
        let mut cur = row![].spacing(6);
        for (k, &i) in s.results.iter().enumerate() {
            cur = cur.push(self.app_tile(
                i,
                TILE,
                ICON,
                13.0,
                k == s.selected,
                Msg::Search(SMsg::Launch(k)),
            ));
            if (k + 1).is_multiple_of(COLUMNS) {
                grid = grid.push(cur);
                cur = row![].spacing(6);
            }
        }
        if !s.results.len().is_multiple_of(COLUMNS) {
            grid = grid.push(cur);
        }
        let heading = if s.query.trim().is_empty() {
            "Toutes les applications"
        } else {
            "Résultats"
        };
        body = body.push(
            column![
                self.launcher_heading(heading),
                scrollable(grid).height(Length::Fill)
            ]
            .spacing(10)
            .height(Length::Fill),
        );
        body = body.push(
            text(crate::i18n::tr(
                "Entrée lance · flèches pour choisir · Échap ferme",
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        );
        // La feuille : opaque, coins arrondis, ombre ; un clic autour referme.
        let sheet = container(body)
            .padding([28, 32])
            .max_width(COLUMNS as f32 * (TILE + 6.0) + 64.0)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(surface.into()),
                border: Border::default().rounded(32).width(1).color(edge),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    offset: Vector::new(0.0, 18.0),
                    blur_radius: 48.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        let page = container(mouse_area(sheet).on_press(Msg::Noop))
            .center_x(Length::Fill)
            .padding(Padding {
                top: 56.0,
                bottom: 40.0,
                left: 24.0,
                right: 24.0,
            })
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.45).into()),
                ..Default::default()
            });
        mouse_area(page)
            .on_press(Msg::Control(super::control::Command::Apps))
            .into()
    }

    /// Autres modes : la carte centrée.
    pub(super) fn view_spotlight<'a>(&'a self, s: &'a Search) -> Element<'a, Msg> {
        let (r, bg) = self.launcher_palette();
        let (fg, dim, edge, accent) = (
            color(r.fg, 1.0),
            color(r.dim, 1.0),
            color(r.edge, 1.0),
            color(r.focus, 1.0),
        );
        let typing = !s.query.trim().is_empty();
        let mut body = column![self.launcher_field(s, SPOT_WIDTH - 44.0, 20.0)].spacing(16);
        if !typing && !s.suggested.is_empty() {
            let tiles = row(s.suggested.iter().take(8).map(|&i| {
                self.app_tile(
                    i,
                    SPOT_TILE,
                    SPOT_ICON,
                    11.0,
                    false,
                    Msg::Search(SMsg::LaunchApp(i)),
                )
            }))
            .spacing(4);
            body = body.push(column![self.launcher_heading("Suggestions"), tiles].spacing(8));
        }
        let rows = if typing { SPOT_ROWS } else { 5 };
        let mut list = column![].spacing(2);
        if s.results.is_empty() {
            list = list.push(
                text(crate::i18n::tr("Aucune application ne correspond."))
                    .font(self.font)
                    .size(14)
                    .color(dim),
            );
        }
        for (k, &i) in s.results.iter().enumerate().take(rows) {
            let selected = k == s.selected;
            list = list.push(
                button(
                    row![
                        self.app_icon(i, 28.0),
                        text(self.apps[i].name.clone())
                            .font(self.font)
                            .size(15)
                            .color(fg)
                            .width(Length::Fill),
                        if selected {
                            Element::from(text("↵").font(self.font).size(14).color(dim))
                        } else {
                            Space::new().into()
                        },
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .height(SPOT_ROW)
                .style(move |_, st| button::Style {
                    background: (selected || matches!(st, button::Status::Hovered)).then(|| {
                        Color {
                            a: if selected { 0.22 } else { 0.1 },
                            ..accent
                        }
                        .into()
                    }),
                    border: Border::default().rounded(12),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::Search(SMsg::Launch(k))),
            );
        }
        body = body.push(
            column![
                self.launcher_heading(if typing { "Résultats" } else { "Applications" }),
                list
            ]
            .spacing(8),
        );
        let card = container(body)
            .padding(22)
            .width(SPOT_WIDTH)
            .height(spot_height(!s.suggested.is_empty()))
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default().rounded(28).width(1).color(edge),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    offset: Vector::new(0.0, 12.0),
                    blur_radius: 32.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        container(card).padding(18).into()
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

    /// Présentation : selon le réglage, ou selon le mode (feuille en plein bureau).
    #[test]
    fn presentation_selon_le_mode() {
        use crate::config::AppsStyle;
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        let style = |app: &mut App, st: AppsStyle| {
            std::sync::Arc::make_mut(&mut app.cfg)
                .general
                .apps_launcher
                .style = st;
        };
        style(&mut app, AppsStyle::Auto);
        app.visible = false;
        assert!(!app.launcher_sheet(), "no-desktop : carte centrée");
        app.visible = true;
        app.with_windows = true;
        assert!(!app.launcher_sheet(), "launcher-only : carte centrée");
        app.with_windows = false;
        assert!(app.launcher_sheet(), "full-desktop : grande feuille");
        style(&mut app, AppsStyle::Spotlight);
        assert!(!app.launcher_sheet());
        style(&mut app, AppsStyle::Sheet);
        app.visible = false;
        assert!(app.launcher_sheet());
        assert!(spot_height(true) > spot_height(false));
    }
}
