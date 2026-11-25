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

use crate::ui::search::{SMsg, Search};
use crate::ui::{App, Msg, color};

/// Applications au plus dans la grille.
pub const LIMIT: usize = 400;
/// Colonnes de la grille (les flèches haut et bas sautent d'une rangée).
/// Forme du lanceur (MLG-278) et hauteur de l'écran actif, posées à
/// chaque config installée.
pub(in crate::ui) static SHAPE: std::sync::RwLock<Option<(crate::config::LauncherShape, f32)>> =
    std::sync::RwLock::new(None);

/// Pose la forme du lanceur.
pub(in crate::ui) fn set_shape(shape: crate::config::LauncherShape, screen_height: f32) {
    if let Ok(mut w) = SHAPE.write() {
        *w = Some((shape, screen_height));
    }
}

/// Forme du lanceur en place.
pub(in crate::ui) fn shape() -> crate::config::LauncherShape {
    SHAPE
        .read()
        .ok()
        .and_then(|s| s.map(|(s, _)| s))
        .unwrap_or_default()
}

pub(in crate::ui) fn screen_height() -> f32 {
    SHAPE
        .read()
        .ok()
        .and_then(|s| s.map(|(_, h)| h))
        .unwrap_or(1080.0)
}

/// Colonnes de la grille d'applications.
pub fn columns() -> usize {
    usize::from(shape().columns.max(2))
}
pub(in crate::ui) const TILE: f32 = 128.0;
pub(in crate::ui) const ICON: f32 = 60.0;
/// Carte centrée : largeur, lignes de résultats.
/// Largeur de la carte centrée.
pub fn spot_width() -> f32 {
    f32::from(shape().width)
}

/// Hauteur d'une ligne de résultat.
pub(in crate::ui) fn spot_row() -> f32 {
    f32::from(shape().row_height)
}

/// Résultats visibles en tapant.
pub(in crate::ui) fn spot_rows() -> usize {
    usize::from(shape().max_results.max(1))
}
/// Suggestions de la carte centrée : tuile, icône.
pub(in crate::ui) const SPOT_TILE: f32 = 76.0;
pub(in crate::ui) const SPOT_ICON: f32 = 46.0;

/// Hauteur de la carte centrée (avec ou sans bandeau de suggestions).
pub fn spot_height(suggestions: bool) -> f32 {
    let field = 58.0;
    let list = spot_rows() as f32 * spot_row();
    let sugg = if suggestions {
        22.0 + SPOT_TILE + 30.0 + 18.0
    } else {
        0.0
    };
    // Sans requête : suggestions + 5 lignes ; en tapant : 8 lignes.
    let pad = f32::from(shape().padding);
    let body = (sugg + 22.0 + 5.0_f32.min(spot_rows() as f32) * spot_row()).max(list);
    let h = field + 16.0 + body + 2.0 * pad;
    // Hauteur plafonnée à une part de l'écran (MLG-278) : les résultats
    // défilent dans la carte.
    let cap = shape().max_height;
    if cap > 0.0 {
        h.min(screen_height() * cap).max(field + 2.0 * pad + 60.0)
    } else {
        h
    }
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
    pub(in crate::ui) fn launcher_palette(&self) -> (crate::theme::Readable, Color) {
        let th = &self.cfg.general.theme;
        (
            crate::theme::readable(th.fg, th.bg, th.accent),
            // Teinte du verre du lanceur (MLG-268).
            color(
                crate::ui::tokens::tinted(th.bg, crate::config::Place::Launcher),
                1.0,
            ),
        )
    }

    /// Champ du lanceur d'applications, avec son texte de substitution.
    fn apps_field<'a>(&'a self, s: &'a Search, width: f32, size: f32) -> Element<'a, Msg> {
        self.launcher_field(
            s,
            Length::Fixed(width),
            size,
            crate::i18n::tr("Rechercher une application… (:f, :clip, :2fa, :mdp, :r)"),
        )
    }

    /// Champ de recherche en pastille, seul propriétaire des quatre modes du
    /// lanceur : loupe à gauche, bouton Clear à droite dès que la requête n'est
    /// pas vide.
    pub(in crate::ui) fn launcher_field<'a>(
        &'a self,
        s: &'a Search,
        width: Length,
        size: f32,
        placeholder: &'a str,
    ) -> Element<'a, Msg> {
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
        let clear = !s.query.is_empty();
        let input = text_input(placeholder, &s.query)
            .id(crate::ui::search::INPUT_ID)
            .on_input(|q| Msg::Search(SMsg::Query(q)))
            .on_submit(Msg::Search(SMsg::Launch(s.selected)))
            .font(self.font)
            .size(crate::ui::tokens::ts(size))
            // À droite comme à gauche, le texte s'arrête avant le ou les glyphes.
            .padding(Padding {
                top: 14.0,
                bottom: 14.0,
                left: 44.0,
                right: if clear { 44.0 } else { 16.0 },
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
        let glass: Element<'_, Msg> = crate::ui::icons::get("edit-find")
            .or_else(|| crate::ui::icons::get("system-search"))
            .map(|h| {
                iced::widget::svg(h)
                    .width(18)
                    .height(18)
                    .style(move |_, _| iced::widget::svg::Style { color: Some(dim) })
                    .into()
            })
            .unwrap_or_else(|| {
                text("⌕")
                    .size(crate::ui::tokens::ts(17.0))
                    .color(dim)
                    .into()
            });
        let mut layers: Vec<Element<'a, Msg>> = vec![
            input.into(),
            container(glass)
                .padding(Padding {
                    left: 16.0,
                    ..Padding::ZERO
                })
                .height(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        ];
        if clear {
            let glyph: Element<'a, Msg> = crate::ui::icons::get("window-close")
                .map(|h| {
                    iced::widget::svg(h)
                        .width(20)
                        .height(20)
                        .style(move |_, _| iced::widget::svg::Style { color: Some(dim) })
                        .into()
                })
                .unwrap_or_else(|| {
                    text("×")
                        .size(crate::ui::tokens::ts(15.0))
                        .color(dim)
                        .into()
                });
            layers.push(
                container(iced::widget::tooltip(
                    button(glyph)
                        .padding(2)
                        .style(button::text)
                        .on_press(Msg::Search(SMsg::Clear)),
                    text(crate::i18n::tr("Effacer la recherche"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0)),
                    iced::widget::tooltip::Position::Top,
                ))
                // La couche occupe tout le champ : sans elle, l'empilement pose
                // le bouton à gauche, sur la loupe.
                .width(Length::Fill)
                .align_x(Alignment::End)
                .padding(Padding {
                    right: 10.0,
                    ..Padding::ZERO
                })
                .height(Length::Fill)
                .center_y(Length::Fill)
                .into(),
            );
        }
        iced::widget::Stack::with_children(layers).into()
    }

    /// Icône d'une application, sinon l'icône de remplacement de hyprdesk.
    pub(in crate::ui) fn app_icon<'a>(&'a self, i: usize, size: f32) -> Element<'a, Msg> {
        let (r, _) = self.launcher_palette();
        let a = &self.apps[i];
        // Clé d'application : le masque unique (HIG ICO-04).
        a.icon
            .as_deref()
            .and_then(|n| self.icon_named(&App::app_icon_key(n), size, color(r.fg, 1.0)))
            .unwrap_or_else(|| {
                crate::ui::icons::fallback_icon(
                    &a.name,
                    size,
                    color(r.button, 1.0),
                    color(r.on_button, 1.0),
                    self.title_font(),
                )
            })
    }

    /// Tuile : icône, nom dessous (deux lignes au plus) ; `msg` au clic.
    pub(in crate::ui) fn app_tile<'a>(
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
        let tile = button(
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
        .style(move |_, st| {
            // États du thème (MLG-273), anneau plein sur la tuile choisie.
            let mut style = crate::ui::tokens::item(accent, fg, selected, st, 20.0);
            style.border.color.a = style.border.color.a.max(if selected { 1.0 } else { 0.0 });
            style
        })
        .on_press(msg);
        // Clic droit : le menu de l'application (MLG-308).
        mouse_area(tile)
            .on_right_press(Msg::GridMenu(self.apps[i].id.clone()))
            .into()
    }

    /// Titre de section du lanceur (« Suggestions », « Toutes les applications »).
    pub(in crate::ui) fn launcher_heading<'a>(&'a self, label: &str) -> Element<'a, Msg> {
        let (r, _) = self.launcher_palette();
        text(crate::i18n::tr(label).to_string())
            .font(self.title_font())
            .size(crate::ui::tokens::ts(13.0))
            .color(color(r.dim, 1.0))
            .into()
    }

    /// Aucun résultat (HIG TYP-02) : ce qui manque, l'étape suivante et le
    /// bouton qui la fait — effacer la recherche, revenir à toutes les
    /// catégories.
    fn launcher_empty<'a>(&'a self, s: &'a Search) -> Element<'a, Msg> {
        let (r, _) = self.launcher_palette();
        let (fg, dim, edge) = (color(r.fg, 1.0), color(r.dim, 1.0), color(r.edge, 1.0));
        let query = s.query.trim();
        let (what, next) = match (query.is_empty(), s.category.is_some()) {
            (false, true) => (
                crate::tr!(
                    "Aucune application ne correspond à « {q} » dans cette catégorie.",
                    q = query
                ),
                crate::i18n::tr("Vérifiez l'orthographe, ou cherchez dans toutes les catégories.")
                    .to_string(),
            ),
            (false, false) => (
                crate::tr!("Aucune application ne correspond à « {q} ».", q = query),
                crate::i18n::tr("Vérifiez l'orthographe, ou essayez un autre mot.").to_string(),
            ),
            (true, true) => (
                crate::i18n::tr("Aucune application dans cette catégorie.").to_string(),
                crate::i18n::tr("Les autres catégories en ont peut-être.").to_string(),
            ),
            (true, false) => (
                crate::i18n::tr("Aucune application à afficher.").to_string(),
                crate::i18n::tr(
                    "Installez-en depuis le gestionnaire de paquets : elles apparaîtront ici.",
                )
                .to_string(),
            ),
        };
        // Même pastille que les catégories, non choisie.
        let action = |label: &str, msg: SMsg| -> Element<'a, Msg> {
            button(
                text(crate::i18n::tr(label).to_string())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(fg),
            )
            .padding([7, 16])
            .style(move |_, st| button::Style {
                background: Some(
                    if matches!(st, button::Status::Hovered) {
                        crate::ui::tokens::veil(fg, crate::ui::tokens::states().hover)
                    } else {
                        Color::TRANSPARENT
                    }
                    .into(),
                ),
                border: Border::default().rounded(999).width(1).color(edge),
                text_color: fg,
                ..Default::default()
            })
            .on_press(Msg::Search(msg))
            .into()
        };
        let mut actions = row![].spacing(8);
        if s.category.is_some() {
            actions = actions.push(action("Toutes les catégories", SMsg::Category(None)));
        }
        if !query.is_empty() {
            actions = actions.push(action("Effacer la recherche", SMsg::Clear));
        }
        column![
            text(what)
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .color(fg),
            text(next)
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .color(dim),
            actions,
        ]
        .spacing(8)
        .into()
    }

    /// Plein bureau : la grande feuille.
    pub(in crate::ui) fn view_apps_grid<'a>(&'a self, s: &'a Search) -> Element<'a, Msg> {
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
        let mut body = column![container(self.apps_field(s, 560.0, 17.0)).center_x(Length::Fill)]
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
                        .size(crate::ui::tokens::ts(13.0))
                        .color(if on { on_btn } else { fg }),
                )
                .padding([7, 16])
                .style(move |_, st| button::Style {
                    background: Some(
                        if on {
                            accent
                        } else if matches!(st, button::Status::Hovered) {
                            crate::ui::tokens::veil(fg, crate::ui::tokens::states().hover)
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
            grid = grid.push(self.launcher_empty(s));
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
            if (k + 1).is_multiple_of(columns()) {
                grid = grid.push(cur);
                cur = row![].spacing(6);
            }
        }
        if !s.results.len().is_multiple_of(columns()) {
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
                // La grille s'estompe sous les bords (MLG-282).
                crate::ui::tokens::scroll_edges(
                    scrollable(grid).height(Length::Fill),
                    if crate::ui::glass::refracts(crate::config::Place::Launcher) {
                        self.launcher_tint(surface)
                    } else {
                        surface
                    },
                )
            ]
            .spacing(10)
            .height(Length::Fill),
        );
        body = body.push(
            text(crate::i18n::tr(
                "Entrée lance · flèches pour choisir · Échap ferme",
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        );
        // La feuille : opaque, coins arrondis, ombre ; un clic autour referme.
        // Verre du thème, vraie réfraction du fond en plein bureau (MLG-275).
        let sheet = crate::ui::glass::card(
            container(body)
                .padding([28, 32])
                .max_width(columns() as f32 * (TILE + 6.0) + 64.0)
                .height(Length::Fill),
            crate::config::Place::Launcher,
            f32::from(crate::ui::apps_grid::shape().radius) + 4.0,
            fg,
            self.launcher_tint(surface),
            container::Style {
                background: Some(surface.into()),
                border: Border::default()
                    .rounded(f32::from(shape().radius) + 4.0)
                    .width(1)
                    .color(Color {
                        a: edge.a * shape().border,
                        ..edge
                    }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, shape().shadow),
                    offset: Vector::new(0.0, 18.0),
                    blur_radius: 48.0,
                },
                text_color: Some(fg),
                ..Default::default()
            },
            true,
            self.launcher_under(),
        );
        let page = container(mouse_area(sheet).on_press(Msg::Noop))
            .center_x(Length::Fill)
            .padding(Padding {
                top: 56.0,
                bottom: 40.0,
                left: 24.0,
                right: 24.0,
            })
            .height(Length::Fill);
        // Le voile flouté est posé dessous (`view_search`).
        mouse_area(page)
            .on_press(Msg::Control(crate::control::Command::Apps))
            .into()
    }

    /// Autres modes : la carte centrée.
    pub(in crate::ui) fn view_spotlight<'a>(&'a self, s: &'a Search) -> Element<'a, Msg> {
        let (r, bg) = self.launcher_palette();
        let (fg, dim, edge, accent) = (
            color(r.fg, 1.0),
            color(r.dim, 1.0),
            color(r.edge, 1.0),
            color(r.focus, 1.0),
        );
        let typing = !s.query.trim().is_empty();
        let convex = self.convex_options().is_some_and(|o| o.attached_panels);
        let mut body = if convex {
            column![]
        } else {
            column![self.apps_field(s, spot_width() - 44.0, 22.0)]
        }
        .spacing(16);
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
        let rows = if typing { spot_rows() } else { 5 };
        let mut list = column![].spacing(2);
        if s.results.is_empty() {
            list = list.push(self.launcher_empty(s));
        }
        for (k, &i) in s.results.iter().enumerate().take(rows) {
            let selected = k == s.selected;
            list = list.push(
                button(
                    row![
                        self.app_icon(i, 28.0),
                        text(self.apps[i].name.clone())
                            .font(self.font)
                            .size(crate::ui::tokens::ts(15.0))
                            .color(fg)
                            .width(Length::Fill),
                        if selected {
                            Element::from(
                                text("↵")
                                    .font(self.font)
                                    .size(crate::ui::tokens::ts(13.0))
                                    .color(dim),
                            )
                        } else {
                            Space::new().into()
                        },
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .height(spot_row())
                .style(move |_, st| {
                    crate::ui::tokens::item(
                        accent,
                        fg,
                        selected,
                        st,
                        f32::from(shape().selection_radius),
                    )
                })
                .on_press(Msg::Search(SMsg::Launch(k))),
            );
        }
        body = body.push(
            column![
                self.launcher_heading(if typing { "Résultats" } else { "Applications" }),
                // Résultats sur une surface de lecture stable (MLG-282).
                crate::ui::tokens::content(list, &self.cfg.general.theme, 16.0)
            ]
            .spacing(8),
        );
        if convex {
            let body = column![
                scrollable(body).height(Length::Fill),
                self.apps_field(s, spot_width() - 80.0, 15.0)
            ]
            .spacing(16);
            return self.convex_search_surface(
                s,
                body.into(),
                spot_width(),
                spot_height(!s.suggested.is_empty()),
            );
        }
        let card = crate::ui::glass::card(
            container(body)
                .padding(shape().padding)
                .width(spot_width())
                .height(spot_height(!s.suggested.is_empty())),
            crate::config::Place::Launcher,
            f32::from(shape().radius),
            fg,
            self.launcher_tint(bg),
            container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(f32::from(shape().radius))
                    .width(1)
                    .color(Color {
                        a: edge.a * shape().border,
                        ..edge
                    }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, shape().shadow),
                    offset: Vector::new(0.0, 12.0),
                    blur_radius: 32.0,
                },
                text_color: Some(fg),
                ..Default::default()
            },
            true,
            self.launcher_under(),
        );
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
        app.test_mode(crate::ui::DesktopMode::NoDesktop);
        assert!(!app.launcher_sheet(), "no-desktop : carte centrée");
        app.test_mode(crate::ui::DesktopMode::LauncherOnly);
        assert!(!app.launcher_sheet(), "launcher-only : carte centrée");
        app.test_mode(crate::ui::DesktopMode::FullDesktop);
        assert!(app.launcher_sheet(), "full-desktop : grande feuille");
        style(&mut app, AppsStyle::Spotlight);
        assert!(!app.launcher_sheet());
        style(&mut app, AppsStyle::Sheet);
        app.test_mode(crate::ui::DesktopMode::NoDesktop);
        assert!(app.launcher_sheet());
        assert!(spot_height(true) > spot_height(false));
    }
}
