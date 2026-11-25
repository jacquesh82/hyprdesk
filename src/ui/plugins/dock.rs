//! Plugin « Dock » : le dock en bas du bureau — widget autonome, seul
//! contrôle du mode `launcher-only` — et sa page de réglages.
//!
//! - `surface` : le dock, posé sur chaque écran ou dans la barre, et la carte
//!   des fenêtres d'une application ;
//! - `lifecycle`, `keyboard`, `motion`, `contrast` : transitions et
//!   lancements, clavier, vague du survol, lisibilité sur le fond ;
//! - ce fichier : manifeste et page de Réglages (contenu, apparence et style
//!   — options `dock.*` du registre `design_options` —, épinglées).
//!
//! Le dock affiche les fenêtres ouvertes (aucune, celles du workspace
//! affiché, ou toutes), regroupées au choix : une entrée par fenêtre, une par
//! application (classe de fenêtre, donc par binaire), ou par workspace. Le
//! bouton ⚙ termine la rangée. L'utiliser ne change jamais le mode du bureau.
//! Le lanceur d'applications qu'ouvre son bouton est un autre plugin
//! (`super::launcher`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

pub(crate) mod contrast;
mod keyboard;
mod lifecycle;
mod motion;
mod surface;

pub(in crate::ui) use contrast::Backdrop;
pub(in crate::ui) use lifecycle::{LaunchEnd, Placement, Runtime};
pub(in crate::ui) use surface::{
    LauncherSurface, Popup, PopupContent, REVEAL_STRIP, Slot, thickness,
};

use crate::i18n::text;
use iced::widget::{button, column, radio, row};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::{PanelMsg, SliderLook, SliderSpec};
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};
use crate::config::{LauncherGroup, LauncherPosition, RunningApps};

pub const INFO: PluginInfo = PluginInfo {
    id: "dock",
    label: "Dock",
    icon: "view-app-grid",
    summary: "Dock du bureau : fenêtres ouvertes, applications épinglées, style.",
    builtin: true,
};

/// Le curseur « Taille des icônes », de 24 à 96 px : sa valeur se saisit au
/// clavier, de 1 px en 1 px (HIG SEL-08).
pub(in crate::ui) fn size_spec() -> SliderSpec {
    SliderSpec::new(
        "launcher_size",
        (24.0, 96.0),
        1.0,
        |v| wrap(Msg::Size(v.round() as u16)),
        wrap(Msg::SizeCommit),
    )
}

/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("contenu", "Contenu"),
    ("apparence", "Apparence"),
    ("epinglees", "Épinglées"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Dock(m)))
}

/// Message de ce paquet, enveloppé pour l'application.
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
}

/// État d'écran : recherche d'une application à épingler.
#[derive(Default)]
pub struct State {
    pub pin_filter: String,
}

impl App {
    pub(in crate::ui) fn dock_page(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let dim = self.ink().dim;
        let running = |label: &'static str, v: RunningApps| {
            self.focusable(
                radio(crate::i18n::tr(label), v, Some(g.running_apps), |v| {
                    wrap(Msg::Running(v))
                })
                .font(self.font),
            )
        };
        let group = |label: &'static str, v: LauncherGroup| {
            self.focusable(
                radio(crate::i18n::tr(label), v, Some(g.launcher_group), |v| {
                    wrap(Msg::Group(v))
                })
                .font(self.font),
            )
        };
        if !self.on_subtab(INFO.id, "contenu") {
            return self.dock_look();
        }
        column![
            self.section("Fenêtres"),
            self.field(
                "Fenêtres ouvertes dans le dock",
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
                "Le dock a sa propre couche, au-dessus des fenêtres : en launcher-only, \
                 les fenêtres lui laissent sa place. L'utiliser ne change jamais le mode du bureau."
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(14)
        .into()
    }

    fn dock_look(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let l = &g.launcher;
        let dim = self.ink().dim;
        let filter = self
            .panel
            .as_ref()
            .map(|p| p.dock.pin_filter.clone())
            .unwrap_or_default();
        let position = |label: &'static str, v: LauncherPosition| {
            self.focusable(
                radio(label, v, Some(l.position), |v| wrap(Msg::Position(v))).font(self.font),
            )
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
                let on = msg.is_some();
                self.focusable_when(
                    button(
                        text(label)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0)),
                    )
                    .padding([3, 8])
                    .style(button::secondary)
                    .on_press_maybe(msg),
                    on,
                )
            };
            pins = pins.push(
                row![
                    text(name(id))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
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
                    self.focusable(
                        button(
                            row![
                                text(a.name.clone())
                                    .font(self.font)
                                    .size(crate::ui::tokens::ts(13.0))
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
                    ),
                );
            }
        }
        if self.on_subtab(INFO.id, "epinglees") {
            return column![
                self.section("Applications épinglées"),
                text("Toujours dans le dock ; un clic les lance si elles ne sont pas ouvertes.")
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
                self.slider(
                    &size_spec(),
                    f64::from(l.size),
                    |v| format!("{} px", v.round() as u16),
                    SliderLook::fixed(13.0, 260, Alignment::Start),
                ),
            ),
            self.field(
                "Boutons",
                "",
                column![
                    self.focusable(
                        self.toggle(l.show_assistant)
                            .label(crate::i18n::tr("Assistant IA (LYRA)"))
                            .on_toggle(|v| wrap(Msg::ShowAssistant(v)))
                    ),
                    self.focusable(
                        self.toggle(l.show_settings)
                            .label(crate::i18n::tr("Réglages"))
                            .on_toggle(|v| wrap(Msg::ShowSettings(v)))
                    ),
                ]
                .spacing(8),
            ),
            // SmartUI design : les icônes se créent et se valident dans
            // Réglages → IA → Autres services IA.
            self.field(
                "Icônes",
                "launcher_smartui",
                row![
                    self.focusable(
                        self.toggle(l.smartui)
                            .label(crate::i18n::tr("SmartUI design (icônes unifiées par l'IA)"))
                            .on_toggle(|v| super::assistant::wrap(
                                super::assistant::Msg::SmartToggle(v)
                            ))
                    ),
                    self.focusable(
                        button(
                            text("Créer et valider les icônes…")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(12.0))
                        )
                        .padding([4, 10])
                        .style(button::secondary)
                        .on_press(super::assistant::wrap(super::assistant::Msg::OpenServices))
                    ),
                ]
                .spacing(14)
                .align_y(Alignment::Center),
            ),
        ]
        .spacing(14)
        .push(self.design_fields("dock", "Style"))
        .into()
    }

    pub(in crate::ui) fn dock_update(&mut self, msg: Msg) -> Task<UiMsg> {
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
            Msg::PinFilter(f) => {
                if let Some(p) = self.panel.as_mut() {
                    p.dock.pin_filter = f;
                }
                Task::none()
            }
            Msg::Pin(id) => {
                let mut l = self.cfg.general.launcher.pinned.clone();
                if !l.contains(&id) {
                    l.push(id);
                }
                if let Some(p) = self.panel.as_mut() {
                    p.dock.pin_filter.clear();
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
