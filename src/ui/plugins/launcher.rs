//! Plugin « Lanceur » : le dock en bas du bureau — widget autonome, seul
//! contrôle du mode `launcher-only` — et sa page de réglages.
//!
//! - `surface` : le dock, posé sur chaque écran ou dans la barre, et la carte
//!   des fenêtres d'une application ;
//! - `lifecycle`, `keyboard`, `motion`, `contrast` : transitions et
//!   lancements, clavier, vague du survol, lisibilité sur le fond ;
//! - ce fichier : manifeste et page de Réglages.
//!
//! Le lanceur affiche les fenêtres ouvertes (aucune, celles du workspace
//! affiché, ou toutes), regroupées au choix : une entrée par fenêtre, une par
//! application (classe de fenêtre, donc par binaire), ou par workspace. Le
//! bouton ⚙ termine la rangée. L'utiliser ne change jamais le mode du bureau.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

pub(crate) mod contrast;
mod keyboard;
mod lifecycle;
mod motion;
mod surface;

pub(in crate::ui) use contrast::Backdrop;
pub(in crate::ui) use lifecycle::{Placement, Runtime};
pub(in crate::ui) use surface::{LauncherSurface, Popup, PopupContent, thickness};

use crate::i18n::text;
use iced::widget::{button, column, radio, row};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::{LauncherGroup, LauncherPosition, RunningApps};

pub const INFO: PluginInfo = PluginInfo {
    id: "launcher",
    label: "Lanceur",
    icon: "view-app-grid",
    summary: "Dock du bureau et lanceur d'applications (grande feuille ou carte centrée, suggestions).",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("contenu", "Contenu"),
    ("apparence", "Apparence"),
    ("epinglees", "Épinglées"),
    ("applications", "Applications"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Launcher(m)))
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Fenêtres affichées : aucune, celles du workspace, toutes.
    Running(RunningApps),
    Group(LauncherGroup),
    Position(LauncherPosition),
    /// Taille des icônes (pendant le glissé), puis enregistrement.
    Size(u16),
    SizeCommit,
    PinFilter(String),
    Pin(String),
    Unpin(usize),
    /// Déplace une application épinglée (-1 avant, +1 après).
    MovePin(usize, i32),
    ShowSettings(bool),
    ShowAssistant(bool),
    /// Lanceur d'applications : présentation, suggestions, catégories.
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

/// État d'écran : recherche d'une application à épingler.
#[derive(Default)]
pub struct State {
    pub pin_filter: String,
    /// Raccourci du plein bureau en cours de saisie, et son erreur.
    pub desk_key: Option<String>,
    pub desk_error: Option<String>,
}

impl App {
    pub(in crate::ui) fn launcher_page(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let dim = self.ink().dim;
        let running = |label: &'static str, v: RunningApps| {
            radio(crate::i18n::tr(label), v, Some(g.running_apps), |v| {
                wrap(Msg::Running(v))
            })
            .font(self.font)
        };
        let group = |label: &'static str, v: LauncherGroup| {
            radio(crate::i18n::tr(label), v, Some(g.launcher_group), |v| {
                wrap(Msg::Group(v))
            })
            .font(self.font)
        };
        if self.on_subtab(INFO.id, "applications") {
            return self.launcher_apps_tab();
        }
        if !self.on_subtab(INFO.id, "contenu") {
            return self.launcher_look();
        }
        column![
            self.section("Fenêtres"),
            self.field(
                "Fenêtres ouvertes dans le lanceur",
                "running_apps",
                column![
                    running(crate::i18n::tr("Aucune (épingles et boutons uniquement)"), RunningApps::Off),
                    running(crate::i18n::tr("Celles du workspace affiché"), RunningApps::Workspace),
                    running(crate::i18n::tr("Toutes"), RunningApps::All),
                ]
                .spacing(8),
            ),
            self.section("Regroupement"),
            self.field(
                "Regrouper les fenêtres",
                "launcher_group",
                column![
                    group(crate::i18n::tr("Aucun : une entrée par fenêtre"), LauncherGroup::None),
                    group(
                        crate::i18n::tr("Par application (binaire) : une entrée, un clic passe de l'une à l'autre"),
                        LauncherGroup::App
                    ),
                    group(
                        crate::i18n::tr("Par workspace : rangées sous leur numéro"),
                        LauncherGroup::Workspace
                    ),
                ]
                .spacing(8),
            ),
            text(
                "Le lanceur a sa propre couche, au-dessus des fenêtres : en launcher-only, \
                 les fenêtres lui laissent sa place. L'utiliser ne change jamais le mode du bureau."
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(14)
        .into()
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
        let commit = wrap(Msg::AppsShape(shape, true));
        let slider_row =
            |label: &'static str,
             range: std::ops::RangeInclusive<f32>,
             value: f32,
             shown: String,
             apply: fn(&mut crate::config::LauncherShape, f32)| {
                self.field(
                    label,
                    "",
                    row![
                        iced::widget::slider(range, value, move |v| set(
                            &|s: &mut crate::config::LauncherShape| apply(s, v),
                            false
                        ))
                        .on_release(commit.clone())
                        .width(220),
                        text(shown).font(font).size(crate::ui::tokens::ts(13.0)),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
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
                480.0..=1200.0,
                f32::from(shape.width),
                format!("{} px", shape.width),
                |s, v| s.width = v.round() as u16
            ),
            slider_row(
                "Résultats visibles",
                3.0..=16.0,
                f32::from(shape.max_results),
                shape.max_results.to_string(),
                |s, v| s.max_results = v.round() as u8
            ),
            slider_row(
                "Colonnes de la feuille",
                4.0..=10.0,
                f32::from(shape.columns),
                shape.columns.to_string(),
                |s, v| s.columns = v.round() as u8
            ),
            slider_row(
                "Rayon des coins",
                0.0..=48.0,
                f32::from(shape.radius),
                format!("{} px", shape.radius),
                |s, v| s.radius = v.round() as u16
            ),
            slider_row(
                "Voile derrière",
                0.0..=0.8,
                shape.backdrop_dim,
                format!("{} %", (shape.backdrop_dim * 100.0).round()),
                |s, v| s.backdrop_dim = v
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

    /// Réglages d'apparence : bord, taille, applications épinglées, boutons.
    /// Onglet Applications : le lanceur d'applications.
    fn launcher_apps_tab(&self) -> Element<'_, UiMsg> {
        use crate::config::AppsStyle;
        let g = &self.cfg.general;
        let a = g.apps_launcher;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let warn = crate::ui::tone::warning();
        let style = |label: &'static str, v: AppsStyle| {
            radio(crate::i18n::tr(label), v, Some(a.style), |v| {
                wrap(Msg::AppsStyle(v))
            })
            .font(self.font)
        };
        let (outside, inside) = self.launcher_keys_text();
        let desk_input = self
            .panel
            .as_ref()
            .and_then(|p| p.launcher.desk_key.clone())
            .unwrap_or_else(|| g.keys.search_desktop.clone());
        let mut col = column![
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
                row![
                    iced::widget::slider(0.0..=16.0, a.suggestions as f32, |v| wrap(
                        Msg::AppsSuggestions(v.round() as u8)
                    ))
                    .step(1.0_f32)
                    .on_release(wrap(Msg::AppsSuggestionsCommit))
                    .width(220),
                    text(a.suggestions.to_string()).font(self.font).size(crate::ui::tokens::ts(13.0)),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            ),
            self.field(
                "Catégories dans la grande feuille",
                "apps_categories",
                iced::widget::toggler(a.categories).on_toggle(|v| wrap(Msg::AppsCategories(v))),
            ),
            self.launcher_shape_fields(a.shape),
            self.section("Raccourci"),
            self.field(
                "Le même lanceur dans tous les modes",
                "apps_unified",
                iced::widget::toggler(a.unified).on_toggle(|v| wrap(Msg::AppsUnified(v))),
            ),
            text(outside).font(self.font).size(crate::ui::tokens::ts(13.0)).color(fg),
            text(inside).font(self.font).size(crate::ui::tokens::ts(13.0)).color(fg),
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
        let mut actions = row![
            button(
                text(crate::i18n::tr("Ouvrir le lanceur"))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
            )
            .padding([6, 14])
            .on_press(UiMsg::Control(crate::control::Command::Apps)),
        ]
        .spacing(10);
        if g.keys.search != "SUPER + R" {
            actions = actions.push(
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
        col.push(actions).into()
    }

    fn launcher_look(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let l = &g.launcher;
        let dim = self.ink().dim;
        let filter = self
            .panel
            .as_ref()
            .map(|p| p.launcher.pin_filter.clone())
            .unwrap_or_default();
        let position = |label: &'static str, v: LauncherPosition| {
            radio(label, v, Some(l.position), |v| wrap(Msg::Position(v))).font(self.font)
        };
        let name = |id: &str| {
            self.apps
                .iter()
                .find(|a| a.id == id)
                .map_or(id.to_string(), |a| a.name.clone())
        };
        let mut pins = column![].spacing(6);
        if l.pinned.is_empty() {
            pins = pins.push(
                text("Aucune application épinglée.")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(dim),
            );
        }
        for (i, id) in l.pinned.iter().enumerate() {
            let small = |label: &'static str, msg: Option<UiMsg>| {
                button(
                    text(label)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0)),
                )
                .padding([3, 8])
                .style(button::secondary)
                .on_press_maybe(msg)
            };
            pins = pins.push(
                row![
                    text(name(id))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(14.0))
                        .width(Length::Fill),
                    small("↑", (i > 0).then(|| wrap(Msg::MovePin(i, -1)))),
                    small(
                        "↓",
                        (i + 1 < l.pinned.len()).then(|| wrap(Msg::MovePin(i, 1)))
                    ),
                    small("Retirer", Some(wrap(Msg::Unpin(i)))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        let f = filter.to_lowercase();
        let mut found = column![].spacing(2);
        if !f.is_empty() {
            for a in self
                .apps
                .iter()
                .filter(|a| {
                    !l.pinned.contains(&a.id)
                        && (a.name.to_lowercase().contains(&f) || a.id.to_lowercase().contains(&f))
                })
                .take(8)
            {
                found = found.push(
                    button(
                        row![
                            text(a.name.clone())
                                .font(self.font)
                                .size(crate::ui::tokens::ts(14.0))
                                .width(Length::Fill),
                            text(a.id.clone())
                                .font(self.font)
                                .size(crate::ui::tokens::ts(12.0))
                                .color(dim),
                        ]
                        .spacing(8),
                    )
                    .width(Length::Fill)
                    .padding([6, 10])
                    .style(button::text)
                    .on_press(wrap(Msg::Pin(a.id.clone()))),
                );
            }
        }
        if self.on_subtab(INFO.id, "epinglees") {
            return column![
                self.section("Applications épinglées"),
                text("Toujours dans le lanceur ; un clic les lance si elles ne sont pas ouvertes.")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
                pins,
                iced::widget::text_input(
                    crate::i18n::tr("Épingler une application (firefox, terminal…)"),
                    &filter
                )
                .font(self.font)
                .on_input(|v| wrap(Msg::PinFilter(v)))
                .padding(8),
                found,
            ]
            .spacing(14)
            .into();
        }
        column![
            self.section("Apparence"),
            self.field(
                "Bord de l'écran",
                "launcher_position",
                row![
                    position("En bas", LauncherPosition::Bottom),
                    position("En haut", LauncherPosition::Top),
                    position(crate::i18n::tr("À gauche"), LauncherPosition::Left),
                    position(crate::i18n::tr("À droite"), LauncherPosition::Right),
                ]
                .spacing(16),
            ),
            self.field(
                "Taille des icônes",
                "launcher_size",
                row![
                    iced::widget::slider(24..=96u16, l.size, |v| wrap(Msg::Size(v)))
                        .on_release(wrap(Msg::SizeCommit))
                        .width(260),
                    text(format!("{} px", l.size))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(14.0)),
                ]
                .spacing(12),
            ),
            self.field(
                "Boutons",
                "",
                column![
                    iced::widget::toggler(l.show_assistant)
                        .label(crate::i18n::tr("Assistant IA (LYRA)"))
                        .on_toggle(|v| wrap(Msg::ShowAssistant(v))),
                    iced::widget::toggler(l.show_settings)
                        .label(crate::i18n::tr("Réglages"))
                        .on_toggle(|v| wrap(Msg::ShowSettings(v))),
                ]
                .spacing(8),
            ),
            // SmartUI design : les icônes se créent et se valident dans
            // Réglages → IA → Autres services IA.
            self.field(
                "Icônes",
                "launcher_smartui",
                row![
                    iced::widget::toggler(l.smartui)
                        .label(crate::i18n::tr("SmartUI design (icônes unifiées par l'IA)"))
                        .on_toggle(
                            |v| super::assistant::wrap(super::assistant::Msg::SmartToggle(v))
                        ),
                    button(
                        text("Créer et valider les icônes…")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                    )
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(super::assistant::wrap(super::assistant::Msg::OpenServices)),
                ]
                .spacing(14)
                .align_y(Alignment::Center),
            ),
        ]
        .spacing(14)
        .into()
    }

    pub(in crate::ui) fn launcher_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Running(v) => {
                self.settings.running_apps = Some(v);
                self.commit()
            }
            Msg::Group(v) => {
                self.settings.launcher_group = Some(v);
                self.commit()
            }
            Msg::Position(v) => {
                self.settings.launcher_position = Some(v);
                self.commit()
            }
            Msg::Size(v) => {
                self.settings.launcher_size = Some(v);
                self.apply()
            }
            Msg::SizeCommit => self.commit(),
            Msg::ShowSettings(v) => {
                self.settings.launcher_show_settings = Some(v);
                self.commit()
            }
            Msg::ShowAssistant(v) => {
                self.settings.launcher_show_assistant = Some(v);
                self.commit()
            }
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
                        "« {v} » : raccourci illisible (ex. SUPER + SHIFT + D)",
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
            Msg::PinFilter(f) => {
                if let Some(p) = self.panel.as_mut() {
                    p.launcher.pin_filter = f;
                }
                Task::none()
            }
            Msg::Pin(id) => {
                let mut l = self.cfg.general.launcher.pinned.clone();
                if !l.contains(&id) {
                    l.push(id);
                }
                if let Some(p) = self.panel.as_mut() {
                    p.launcher.pin_filter.clear();
                }
                self.settings.launcher_pinned = Some(l);
                self.commit()
            }
            Msg::Unpin(i) => {
                let mut l = self.cfg.general.launcher.pinned.clone();
                if i < l.len() {
                    l.remove(i);
                }
                self.settings.launcher_pinned = Some(l);
                self.commit()
            }
            Msg::MovePin(i, d) => {
                let mut l = self.cfg.general.launcher.pinned.clone();
                let j = i as i64 + d as i64;
                if i < l.len() && j >= 0 && (j as usize) < l.len() {
                    l.swap(i, j as usize);
                }
                self.settings.launcher_pinned = Some(l);
                self.commit()
            }
        }
    }
}
