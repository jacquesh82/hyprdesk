//! Plugin « Lanceur » : le lanceur d'applications — grande feuille en plein
//! bureau, carte centrée sinon (`grid`, MLG-32 ; `hyprdesk apps`,
//! `hyprdesk search`, bouton du dock, keys.search) — et sa page de réglages :
//! présentation et suggestions, forme (MLG-278), raccourci.
//!
//! Le dock est un autre plugin (`super::dock`). Le lanceur fait partie du
//! cœur : il reste ouvrable même dock coupé.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

pub(in crate::ui) mod grid;

use crate::i18n::text;
use iced::widget::{button, column, radio, row};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::{PanelMsg, SliderLook, SliderSpec};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "launcher",
    label: "Lanceur",
    icon: "hyprdesk-apps",
    summary: "Lanceur d'applications : grande feuille ou carte centrée, suggestions, raccourci.",
    builtin: true,
};

/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("applications", "Applications"),
    ("forme", "Forme"),
    ("raccourci", "Raccourci"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Launcher(m)))
}

/// Valeur d'un curseur de la forme du lanceur, en pixels ou en nombre seul.
fn en_px(v: f64) -> String {
    format!("{} px", v.round() as u16)
}

fn en_nombre(v: f64) -> String {
    v.round().to_string()
}

/// Message de ce paquet, enveloppé pour l'application.
#[derive(Debug, Clone)]
pub enum Msg {
    /// Présentation, suggestions, catégories.
    AppsStyle(crate::config::AppsStyle),
    AppsSuggestions(u8),
    /// Forme du lanceur (MLG-278) ; `true` : enregistrer.
    AppsShape(crate::config::LauncherShape, bool),
    AppsSuggestionsCommit,
    AppsCategories(bool),
    /// keys.search = SUPER + R (à la place d'un autre lanceur).
    UseSuperR,
    /// Le même lanceur dans tous les modes (apps_unified).
    AppsUnified(bool),
    /// Plein bureau : raccourci propre (keys.search_desktop), saisi puis validé.
    DeskKey(String),
    DeskKeySubmit,
}

/// État d'écran : raccourci du plein bureau en cours de saisie, et son erreur.
#[derive(Default)]
pub struct State {
    pub desk_key: Option<String>,
    pub desk_error: Option<String>,
}

impl App {
    pub(in crate::ui) fn launcher_page(&self) -> Element<'_, UiMsg> {
        if self.on_subtab(INFO.id, "forme") {
            return self.launcher_shape_fields(self.cfg.general.apps_launcher.shape);
        }
        if self.on_subtab(INFO.id, "raccourci") {
            return self.launcher_keys_tab();
        }
        self.launcher_apps_tab()
    }

    /// Ce qu'ouvre le raccourci du lanceur hors plein bureau, puis en plein bureau.
    fn launcher_keys_text(&self) -> (String, String) {
        let g = &self.cfg.general;
        let k = &g.keys;
        let r = &self.key_report;
        let held = |list: &[crate::keymap::Held]| {
            list.iter()
                .find(|h| h.action.id == "search")
                .map(|h| h.holder.describe())
        };
        let outside = if k.search.is_empty() {
            crate::i18n::tr("Hors plein bureau : aucun raccourci (le dock, ou hyprdesk apps).")
                .to_string()
        } else if let Some(h) = held(&r.left) {
            crate::tr!("Hors plein bureau : {k} reste à {h}.", k = k.search, h = h)
        } else if let Some(h) = held(&r.taken) {
            crate::tr!(
                "Hors plein bureau : {k} ouvre ce lanceur, à la place de {h}.",
                k = k.search,
                h = h
            )
        } else {
            crate::tr!("Hors plein bureau : {k} ouvre ce lanceur.", k = k.search)
        };
        let desk = if k.search_desktop.is_empty() {
            &k.search
        } else {
            &k.search_desktop
        };
        let inside = if desk.is_empty() {
            crate::i18n::tr("En plein bureau : aucun raccourci (le dock, ou hyprdesk apps).")
                .to_string()
        } else {
            crate::tr!("En plein bureau : {k} ouvre ce lanceur.", k = desk)
        };
        (outside, inside)
    }

    /// Forme du lanceur (MLG-278) : dimensions, état initial, ouverture,
    /// voile, place, fermeture, effets.
    fn launcher_shape_fields(&self, shape: crate::config::LauncherShape) -> Element<'_, UiMsg> {
        use crate::config::{EffectsQuality, EmptyView, OpenEffect};
        let font = self.font;
        let set = move |f: &dyn Fn(&mut crate::config::LauncherShape), commit: bool| {
            let mut s = shape;
            f(&mut s);
            wrap(Msg::AppsShape(s, commit))
        };
        // L'enregistrement d'un curseur de la forme ne porte pas la forme :
        // posée juste avant (glissé ou saisie), la voilà à jour.
        let slider_row =
            |label: &'static str,
             key: &'static str,
             range: (f64, f64),
             value: f64,
             affiche: fn(f64) -> String,
             apply: fn(&mut crate::config::LauncherShape, f32)| {
                self.field(
                    label,
                    "",
                    self.slider(
                        &SliderSpec::new(
                            key,
                            range,
                            1.0,
                            move |v| {
                                set(
                                    &|s: &mut crate::config::LauncherShape| apply(s, v as f32),
                                    false,
                                )
                            },
                            UiMsg::Panel(PanelMsg::Commit),
                        ),
                        value,
                        affiche,
                        SliderLook::fixed(13.0, 220, Alignment::Center),
                    ),
                )
            };
        let radios = |label: &'static str,
                      items: Vec<(&'static str, u8)>,
                      current: u8,
                      apply: fn(&mut crate::config::LauncherShape, u8)| {
            self.field(
                label,
                "",
                row(items.into_iter().map(|(l, v)| {
                    radio(crate::i18n::tr(l), v, Some(current), move |v| {
                        set(&|s: &mut crate::config::LauncherShape| apply(s, v), true)
                    })
                    .font(font)
                    .into()
                }))
                .spacing(14)
                .wrap(),
            )
        };
        column![
            self.section("Forme du lanceur"),
            slider_row(
                "Largeur de la carte",
                "apps_width",
                (480.0, 1200.0),
                f64::from(shape.width),
                en_px,
                |s, v| s.width = v.round() as u16
            ),
            slider_row(
                "Résultats visibles",
                "apps_max_results",
                (3.0, 16.0),
                f64::from(shape.max_results),
                en_nombre,
                |s, v| s.max_results = v.round() as u8
            ),
            slider_row(
                "Colonnes de la feuille",
                "apps_columns",
                (4.0, 10.0),
                f64::from(shape.columns),
                en_nombre,
                |s, v| s.columns = v.round() as u8
            ),
            slider_row(
                "Rayon des coins",
                "apps_radius",
                (0.0, 48.0),
                f64::from(shape.radius),
                en_px,
                |s, v| s.radius = v.round() as u16
            ),
            // En pour cent, comme la valeur affichée : le curseur avance de
            // 1 %, au lieu de sauter de 0 à 80 %.
            slider_row(
                "Voile derrière",
                "apps_backdrop_dim",
                (0.0, 80.0),
                f64::from(shape.backdrop_dim) * 100.0,
                |v| format!("{} %", v.round() as u32),
                |s, v| s.backdrop_dim = v / 100.0
            ),
            radios(
                "Avant la saisie",
                vec![
                    ("Les plus lancées", 0),
                    ("Les épinglées", 1),
                    ("Le champ seul", 2)
                ],
                match shape.empty_view {
                    EmptyView::Suggestions => 0,
                    EmptyView::Favorites => 1,
                    EmptyView::Field => 2,
                },
                |s, v| s.empty_view = match v {
                    1 => EmptyView::Favorites,
                    2 => EmptyView::Field,
                    _ => EmptyView::Suggestions,
                },
            ),
            radios(
                "Ouverture",
                vec![
                    ("Fondu", 0),
                    ("Agrandissement", 1),
                    ("Glissement", 2),
                    ("Immédiate", 3)
                ],
                match shape.open_effect {
                    OpenEffect::Fade => 0,
                    OpenEffect::Scale => 1,
                    OpenEffect::Slide => 2,
                    OpenEffect::None => 3,
                },
                |s, v| s.open_effect = match v {
                    1 => OpenEffect::Scale,
                    2 => OpenEffect::Slide,
                    3 => OpenEffect::None,
                    _ => OpenEffect::Fade,
                },
            ),
            radios(
                "Place de la carte",
                vec![("Au centre", 0), ("En haut", 1)],
                u8::from(shape.top),
                |s, v| s.top = v == 1,
            ),
            radios(
                "Effets du verre",
                vec![("Complets", 2), ("Moyens", 1), ("Réduits", 0)],
                match shape.quality {
                    EffectsQuality::Low => 0,
                    EffectsQuality::Medium => 1,
                    EffectsQuality::High => 2,
                },
                |s, v| s.quality = match v {
                    0 => EffectsQuality::Low,
                    1 => EffectsQuality::Medium,
                    _ => EffectsQuality::High,
                },
            ),
            self.field(
                "Un clic hors de la carte la ferme",
                "",
                iced::widget::toggler(shape.close_on_outside_click).on_toggle(move |v| set(
                    &|s: &mut crate::config::LauncherShape| s.close_on_outside_click = v,
                    true
                )),
            ),
        ]
        .spacing(14)
        .into()
    }

    /// Onglet Applications : le lanceur d'applications.
    fn launcher_apps_tab(&self) -> Element<'_, UiMsg> {
        use crate::config::AppsStyle;
        let g = &self.cfg.general;
        let a = g.apps_launcher;
        let dim = self.ink().dim;
        let style = |label: &'static str, v: AppsStyle| {
            radio(crate::i18n::tr(label), v, Some(a.style), |v| {
                wrap(Msg::AppsStyle(v))
            })
            .font(self.font)
        };
        column![
            self.section("Lanceur d'applications"),
            text("Un seul lanceur, quelle que soit la façon de l'ouvrir (raccourci, bouton du dock, hyprdesk apps) : en plein bureau, une grande feuille de toutes les applications ; dans les autres modes, une carte au centre de l'écran. Les applications que vous lancez le plus sont toujours mises en avant.")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            self.field(
                "Présentation",
                "apps_style",
                column![
                    style("Selon le mode (feuille en plein bureau, carte sinon)", AppsStyle::Auto),
                    style("Toujours la grande feuille", AppsStyle::Sheet),
                    style("Toujours la carte centrée", AppsStyle::Spotlight),
                ]
                .spacing(6),
            ),
            self.field(
                "Suggestions (les plus lancées)",
                "apps_suggestions",
                self.slider(
                    &SliderSpec::new("apps_suggestions", (0.0, 16.0), 1.0, |v| wrap(Msg::AppsSuggestions(v.round() as u8)), wrap(Msg::AppsSuggestionsCommit)),
                    f64::from(a.suggestions),
                    |v| format!("{}", v.round() as u8),
                    SliderLook::fixed(13.0, 220, Alignment::Center),
                ),
            ),
            self.field(
                "Catégories dans la grande feuille",
                "apps_categories",
                iced::widget::toggler(a.categories).on_toggle(|v| wrap(Msg::AppsCategories(v))),
            ),
            button(
                text(crate::i18n::tr("Ouvrir le lanceur"))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
            )
            .padding([6, 14])
            .on_press(UiMsg::Control(crate::control::Command::Apps)),
        ]
        .spacing(12)
        .into()
    }

    /// Onglet Raccourci : ce qu'ouvre le lanceur, et ses conflits.
    fn launcher_keys_tab(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let a = g.apps_launcher;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let warn = crate::ui::tone::warning();
        let (outside, inside) = self.launcher_keys_text();
        let desk_input = self
            .panel
            .as_ref()
            .and_then(|p| p.launcher.desk_key.clone())
            .unwrap_or_else(|| g.keys.search_desktop.clone());
        let mut col = column![
            self.section("Raccourci"),
            self.field(
                "Le même lanceur dans tous les modes",
                "apps_unified",
                iced::widget::toggler(a.unified).on_toggle(|v| wrap(Msg::AppsUnified(v))),
            ),
            text(outside)
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .color(fg),
            text(inside)
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .color(fg),
            self.field(
                "Plein bureau : un autre raccourci",
                "keys.search_desktop",
                iced::widget::text_input(crate::i18n::tr("vide : le même"), &desk_input)
                    .font(self.font)
                    .on_input(|v| wrap(Msg::DeskKey(v)))
                    .on_submit(wrap(Msg::DeskKeySubmit))
                    .padding(8)
                    .width(220),
            ),
            text("Les autres raccourcis, et tous les conflits : Réglages → Raccourcis.")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
        ]
        .spacing(12);
        if let Some(e) = self
            .panel
            .as_ref()
            .and_then(|p| p.launcher.desk_error.clone())
        {
            col = col.push(
                text(e)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(warn),
            );
        }
        // Un raccourci du lanceur en conflit : hyprdesk le prend, ou LYRA.
        for i in ["keys.search", "keys.search_desktop"]
            .iter()
            .filter_map(|k| self.key_report.issue_for(k))
        {
            let mut line = row![
                text(i.message())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(warn)
                    .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            if i.holder().is_some() {
                line = line.push(
                    button(
                        text(crate::i18n::tr("hyprdesk le prend"))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0)),
                    )
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(super::keys::wrap(super::keys::Msg::TakeKey(
                        i.action.combo.clone(),
                    ))),
                );
            }
            line = line.push(self.lyra_button(
                crate::i18n::tr("Lanceur d'applications"),
                crate::keymap::lyra_prompt(&self.key_report),
            ));
            col = col.push(line);
        }
        if g.keys.search != "SUPER + R" {
            col = col.push(
                button(
                    text(crate::i18n::tr("Utiliser SUPER + R"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0)),
                )
                .padding([6, 14])
                .style(button::secondary)
                .on_press(wrap(Msg::UseSuperR)),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn launcher_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::AppsStyle(v) => {
                self.settings.apps_style = Some(v);
                self.commit()
            }
            Msg::AppsSuggestions(v) => {
                self.settings.apps_suggestions = Some(v);
                self.apply()
            }
            Msg::AppsShape(shape, commit) => {
                self.settings.launcher_shape = Some(shape);
                if commit { self.commit() } else { self.apply() }
            }
            Msg::AppsSuggestionsCommit => self.commit(),
            Msg::AppsCategories(v) => {
                self.settings.apps_categories = Some(v);
                self.commit()
            }
            Msg::UseSuperR => {
                self.settings.key_search = Some("SUPER + R".into());
                self.commit()
            }
            Msg::AppsUnified(v) => {
                self.settings.apps_unified = Some(v);
                self.commit()
            }
            Msg::DeskKey(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.launcher.desk_key = Some(v);
                    p.launcher.desk_error = None;
                }
                Task::none()
            }
            Msg::DeskKeySubmit => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                let v = p
                    .launcher
                    .desk_key
                    .clone()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if !v.is_empty() && crate::hypr::parse_combo(&v).is_none() {
                    p.launcher.desk_error = Some(crate::tr!(
                        "« {v} » : raccourci illisible (ex. SHIFT + SUPER + D)",
                        v = v
                    ));
                    return Task::none();
                }
                let keys = crate::config::Keys {
                    search_desktop: v.clone(),
                    ..self.cfg.general.keys.clone()
                };
                if let Some(other) = crate::keymap::duplicate_of(
                    &keys,
                    self.cfg.general.apps_launcher.unified,
                    "keys.search_desktop",
                ) {
                    p.launcher.desk_error = Some(crate::tr!(
                        "« {v} » sert déjà à « {other} » : choisissez une autre combinaison.",
                        v = v,
                        other = crate::i18n::tr(other)
                    ));
                    return Task::none();
                }
                p.launcher.desk_key = None;
                // Saisi aussi dans Bureau → Raccourcis : le champ suit.
                p.keys.keys[9] = v.clone();
                self.settings.key_search_desktop = Some(v);
                self.commit()
            }
        }
    }
}
