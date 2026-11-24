//! Menus de hyprdesk (MLG-306) : une seule façon de dessiner un menu,
//! partagée par les menus de la barre et par ceux du clic droit (bureau,
//! icônes, widgets, dock, grille du Lanceur, champs de saisie).
//!
//! Règles suivies (Apple HIG, « Menus » et « Context menus ») : au plus trois
//! groupes séparés par un trait, les entrées les plus utiles en tête ; une
//! icône pour chaque entrée d'un groupe qui en montre ; survol et appui
//! distincts ; action destructive en rouge ; « … » quand l'entrée demande une
//! suite ; entrée indisponible grisée. Au clavier, les flèches parcourent les
//! entrées actives (Début, Fin aux extrémités) et Entrée choisit.

use iced::widget::{Space, button, column, container, row, rule, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};

use super::{App, Msg};

/// Hauteurs des lignes : une surface layer-shell a une taille fixe, la carte
/// se calcule d'avance.
pub(in crate::ui) const ROW: f32 = 30.0;
/// Ligne dont le détail passe sous le libellé.
pub(in crate::ui) const ROW_TALL: f32 = 44.0;
pub(in crate::ui) const SEPARATOR: f32 = 11.0;
pub(in crate::ui) const HEADER: f32 = 26.0;

/// Encre d'un menu : couleur du texte, accent, opacité du fondu d'apparition.
#[derive(Debug, Clone, Copy)]
pub(in crate::ui) struct Ink {
    pub fg: Color,
    pub accent: Color,
    pub alpha: f32,
}

impl Ink {
    fn fg(&self, a: f32) -> Color {
        Color {
            a: a * self.alpha,
            ..self.fg
        }
    }
}

/// État d'une ligne de menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui) struct RowState {
    /// Faux : entrée indisponible, grisée.
    pub enabled: bool,
    /// Faux : ligne d'information, lisible mais sans survol.
    pub clickable: bool,
    /// Action destructive : texte et icône en rouge.
    pub danger: bool,
    /// Choisie au clavier : surlignée comme au survol.
    pub selected: bool,
    /// Ligne d'un sous-menu d'un niveau (HIG menus › Submenus) : un chevron
    /// suit son libellé.
    pub submenu: bool,
}

impl Default for RowState {
    fn default() -> Self {
        Self {
            enabled: true,
            clickable: true,
            danger: false,
            selected: false,
            submenu: false,
        }
    }
}

/// Chevron d'une ligne de sous-menu : icône du jeu de hyprdesk, repli « › ».
pub(in crate::ui) const CHEVRON: &str = "go-next-symbolic";

/// Fond d'une ligne : rien au repos, accent léger au survol (ou choisie au
/// clavier), plus marqué pendant l'appui (HIG Buttons : un état pressé).
pub(in crate::ui) fn row_background(
    ink: Ink,
    st: RowState,
    status: button::Status,
) -> Option<Background> {
    if !(st.enabled && st.clickable) {
        return None;
    }
    let a = match status {
        button::Status::Pressed => 0.24,
        button::Status::Hovered => 0.12,
        _ if st.selected => 0.12,
        _ => return None,
    };
    Some(Background::Color(Color {
        a: a * ink.alpha,
        ..ink.accent
    }))
}

/// Texte d'une ligne : rouge si destructive, estompé si indisponible.
pub(in crate::ui) fn row_text(ink: Ink, st: RowState) -> Color {
    let base = if st.danger {
        super::tone::danger()
    } else {
        ink.fg
    };
    Color {
        a: (if st.enabled { 1.0 } else { 0.35 }) * ink.alpha,
        ..base
    }
}

/// Détail trop long pour tenir à droite du libellé : il passe dessous.
pub(in crate::ui) fn stacked(label: &str, detail: Option<&str>) -> bool {
    detail.is_some_and(|d| d.chars().count() > 22 || label.chars().count() + d.chars().count() > 32)
}

/// Entrée d'un menu construit dans le code (clic droit).
#[derive(Debug, Clone)]
pub(in crate::ui) struct Item<M> {
    pub label: String,
    /// Icône nommée (thème, jeu de hyprdesk) ou chemin d'image.
    pub icon: Option<String>,
    pub detail: Option<String>,
    pub msg: M,
    pub enabled: bool,
    pub danger: bool,
    /// Coche : état en cours (fenêtre au premier plan…), à la place de l'icône.
    pub checked: bool,
}

impl<M> Item<M> {
    pub fn new(label: impl Into<String>, icon: &str, msg: M) -> Self {
        Self {
            label: label.into(),
            icon: (!icon.is_empty()).then(|| icon.to_string()),
            detail: None,
            msg,
            enabled: true,
            danger: false,
            checked: false,
        }
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail;
        self
    }
}

/// Ligne d'un menu construit dans le code.
#[derive(Debug, Clone)]
pub(in crate::ui) enum Line<M> {
    Item(Item<M>),
    Separator,
}

impl<M> From<Item<M>> for Line<M> {
    fn from(i: Item<M>) -> Self {
        Line::Item(i)
    }
}

/// Groupes joints par un trait ; un groupe vide disparaît.
pub(in crate::ui) fn groups<M>(groups: Vec<Vec<Line<M>>>) -> Vec<Line<M>> {
    let mut out = Vec::new();
    for g in groups.into_iter().filter(|g| !g.is_empty()) {
        if !out.is_empty() {
            out.push(Line::Separator);
        }
        out.extend(g);
    }
    out
}

/// Hauteur des lignes, sans les marges de la carte.
pub(in crate::ui) fn height<M>(lines: &[Line<M>]) -> f32 {
    lines
        .iter()
        .map(|l| match l {
            Line::Item(i) if stacked(&i.label, i.detail.as_deref()) => ROW_TALL,
            Line::Item(_) => ROW,
            Line::Separator => SEPARATOR,
        })
        .sum()
}

/// Rang d'une entrée qu'on peut choisir.
fn choosable<M>(l: &Line<M>) -> bool {
    matches!(l, Line::Item(i) if i.enabled)
}

/// Entrée suivante (ou précédente) choisissable au clavier, en boucle ; sans
/// entrée choisie, la première (ou la dernière).
pub(in crate::ui) fn step<M>(
    lines: &[Line<M>],
    from: Option<usize>,
    forward: bool,
) -> Option<usize> {
    let n = lines.len();
    if n == 0 {
        return None;
    }
    let start = match (from, forward) {
        (Some(i), true) => i + 1,
        (Some(i), false) => i + n - 1,
        (None, true) => 0,
        (None, false) => n - 1,
    };
    (0..n)
        .map(|k| {
            if forward {
                (start + k) % n
            } else {
                (start + n - k) % n
            }
        })
        .find(|&i| choosable(&lines[i]))
}

/// Première (ou dernière) entrée choisissable.
pub(in crate::ui) fn edge<M>(lines: &[Line<M>], first: bool) -> Option<usize> {
    step(lines, None, first)
}

impl App {
    /// Encre des menus du bureau : couleurs du thème.
    pub(in crate::ui) fn menu_ink(&self, alpha: f32) -> Ink {
        let th = &self.cfg.general.theme;
        Ink {
            fg: super::color(th.fg, 1.0),
            accent: super::color(th.accent, 1.0),
            alpha,
        }
    }

    /// Titre de section, estompé.
    pub(in crate::ui) fn menu_header(&self, ink: Ink, label: String) -> Element<'_, Msg> {
        container(
            text(label)
                .font(self.font)
                .size(crate::ui::tokens::ts(11.0))
                .color(ink.fg(0.65))
                .font(self.ui_font(iced::font::Weight::Semibold)),
        )
        .padding(Padding {
            top: 6.0,
            right: 10.0,
            bottom: 4.0,
            left: 10.0,
        })
        .height(HEADER)
        .into()
    }

    /// Trait entre deux groupes.
    pub(in crate::ui) fn menu_separator(&self, ink: Ink) -> Element<'_, Msg> {
        let line = ink.fg(0.10);
        container(rule::horizontal(1).style(move |_| rule::Style {
            color: line,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        }))
        .padding([5, 10])
        .height(SEPARATOR)
        .into()
    }

    /// Tête d'une ligne : coche, sinon icône ; la colonne reste alignée sans.
    pub(in crate::ui) fn menu_lead<'a>(
        &self,
        ink: Ink,
        st: RowState,
        icon: Option<&str>,
        checked: bool,
    ) -> Element<'a, Msg> {
        let tint = row_text(ink, st);
        let name = if checked {
            Some("object-select-symbolic")
        } else {
            icon
        };
        name.and_then(|n| {
            self.icon_named(
                n,
                crate::ui::tokens::glyph(if checked { 14.0 } else { 16.0 }),
                tint,
            )
        })
        .unwrap_or_else(|| {
            if checked {
                text("•")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(tint)
                    .into()
            } else {
                Space::new().into()
            }
        })
    }

    /// Ligne de menu : tête (icône), libellé, détail à droite (ou dessous
    /// s'il est long). Sans action ou indisponible : pas de survol.
    pub(in crate::ui) fn menu_row<'a>(
        &'a self,
        ink: Ink,
        lead: Element<'a, Msg>,
        label: String,
        detail: Option<String>,
        st: RowState,
        msg: Option<Msg>,
    ) -> Element<'a, Msg> {
        let dim = ink.fg(if st.enabled { 0.65 } else { 0.3 });
        let font = self.ui_font(iced::font::Weight::Normal);
        let long = stacked(&label, detail.as_deref());
        let active = st.enabled && st.clickable;
        let mut label_el = text(label)
            .font(self.font)
            .size(crate::ui::tokens::ts(13.0))
            .font(font)
            .wrapping(iced::widget::text::Wrapping::None);
        // Sous-menu : le libellé occupe la ligne, le chevron vient au bord.
        if st.submenu {
            label_el = label_el.width(Length::Fill);
        }
        let body: Element<'_, Msg> = match detail {
            Some(d) if long => column![
                label_el,
                text(d)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(11.0))
                    .color(dim)
                    .font(font)
                    .wrapping(iced::widget::text::Wrapping::None)
            ]
            .spacing(1)
            .into(),
            Some(d) => row![
                label_el.width(Length::Fill),
                text(d)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim)
                    .font(font)
            ]
            .align_y(Alignment::Center)
            .into(),
            None => label_el.into(),
        };
        let body: Element<'_, Msg> = if st.submenu {
            row![body, self.menu_chevron(ink, st)]
                .spacing(4)
                .align_y(Alignment::Center)
                .into()
        } else {
            body
        };
        let fg = row_text(ink, st);
        button(
            row![
                container(lead)
                    .width(crate::ui::tokens::glyph(18.0))
                    .align_x(Alignment::Center),
                body
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(if long { ROW_TALL } else { ROW })
        .padding([4, 10])
        .on_press_maybe(if active { msg } else { None })
        .style(move |_, status| button::Style {
            background: row_background(ink, st, status),
            text_color: fg,
            border: Border::default().rounded(crate::ui::design::SMALL_RADIUS),
            ..Default::default()
        })
        .into()
    }

    /// Chevron d'une ligne de sous-menu, à la couleur du texte de la ligne
    /// (rouge si destructive, estompé si indisponible).
    pub(in crate::ui) fn menu_chevron<'a>(&self, ink: Ink, st: RowState) -> Element<'a, Msg> {
        let tint = row_text(ink, st);
        self.icon_named(CHEVRON, crate::ui::tokens::glyph(12.0), tint)
            .unwrap_or_else(|| {
                text("›")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(15.0))
                    .color(tint)
                    .into()
            })
    }

    /// Lignes d'un menu construit dans le code ; `selected` : entrée choisie
    /// au clavier.
    pub(in crate::ui) fn menu_lines<'a, M: Clone>(
        &'a self,
        ink: Ink,
        lines: &[Line<M>],
        selected: Option<usize>,
        wrap: impl Fn(M) -> Msg,
    ) -> iced::widget::Column<'a, Msg> {
        let mut col = column![].spacing(0);
        for (i, l) in lines.iter().enumerate() {
            col = col.push(match l {
                Line::Separator => self.menu_separator(ink),
                Line::Item(it) => {
                    let st = RowState {
                        enabled: it.enabled,
                        clickable: true,
                        danger: it.danger,
                        selected: selected == Some(i),
                        ..Default::default()
                    };
                    self.menu_row(
                        ink,
                        self.menu_lead(ink, st, it.icon.as_deref(), it.checked),
                        it.label.clone(),
                        it.detail.clone(),
                        st,
                        Some(wrap(it.msg.clone())),
                    )
                }
            });
        }
        col
    }
}

/// Icônes à résoudre avant d'ouvrir le menu.
pub(in crate::ui) fn icons<M>(lines: &[Line<M>]) -> Vec<String> {
    let mut v: Vec<String> = lines
        .iter()
        .filter_map(|l| match l {
            Line::Item(i) => i.icon.clone(),
            _ => None,
        })
        .collect();
    v.push("object-select-symbolic".into());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> Line<u8> {
        Item::new(label, "", 0).into()
    }

    #[test]
    fn groupes_separes_sans_groupe_vide() {
        let l = groups(vec![vec![item("a"), item("b")], vec![], vec![item("c")]]);
        assert_eq!(l.len(), 4);
        assert!(matches!(l[2], Line::Separator));
        assert!(groups::<u8>(vec![vec![], vec![]]).is_empty());
        assert!((height(&l) - (3.0 * ROW + SEPARATOR)).abs() < 0.01);
    }

    #[test]
    fn clavier_saute_traits_et_entrees_grisees() {
        let l: Vec<Line<u8>> = vec![
            Line::Separator,
            item("a"),
            Line::Separator,
            Item::new("b", "", 0).enabled(false).into(),
            item("c"),
        ];
        assert_eq!(edge(&l, true), Some(1));
        assert_eq!(edge(&l, false), Some(4));
        assert_eq!(step(&l, Some(1), true), Some(4));
        assert_eq!(step(&l, Some(4), true), Some(1));
        assert_eq!(step(&l, Some(1), false), Some(4));
        assert_eq!(step::<u8>(&[Line::Separator], None, true), None);
    }

    #[test]
    fn appui_plus_marque_que_survol() {
        let ink = Ink {
            fg: Color::WHITE,
            accent: Color::from_rgb(0.2, 0.4, 1.0),
            alpha: 1.0,
        };
        let a = |s| match row_background(ink, RowState::default(), s) {
            Some(Background::Color(c)) => c.a,
            _ => 0.0,
        };
        assert!(a(button::Status::Pressed) > a(button::Status::Hovered));
        assert_eq!(a(button::Status::Active), 0.0);
        let off = RowState {
            enabled: false,
            ..RowState::default()
        };
        assert!(row_background(ink, off, button::Status::Hovered).is_none());
        let sel = RowState {
            selected: true,
            ..RowState::default()
        };
        assert!(row_background(ink, sel, button::Status::Active).is_some());
    }
}
