//! Paquet intégré « Applications » : applications par défaut par rôle
//! (navigateur, terminal, fichiers, éditeur, images, vidéo, audio, PDF, mail,
//! assistant IA). Un choix est enregistré dans `settings.lua` et devient aussi
//! la valeur du système (`xdg-mime`, `xdg-terminals.list`). Domaine : `crate::apps`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::{BTreeMap, HashMap};

use crate::i18n::text;
use iced::widget::{button, column, pick_list, row, text_input};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};
use crate::apps::{self, ROLES};

pub const INFO: PluginInfo = PluginInfo {
    id: "apps",
    label: "Applications",
    icon: "applications-other",
    summary: "Applications par défaut : navigateur, terminal, images, vidéo, assistant IA…",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Apps(m)))
}

/// État d'écran : valeurs du système, commandes en cours de saisie.
#[derive(Default)]
pub struct State {
    pub system: BTreeMap<String, Option<String>>,
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Choix d'une application (ou d'une commande) pour un rôle.
    Pick(String, String),
    Custom(String, String),
    CustomSubmit(String),
    /// Revenir à l'application du système pour ce rôle.
    UseSystem(String),
    SystemLoaded(BTreeMap<String, Option<String>>),
}

/// Choix d'une liste : valeur enregistrée, libellé affiché.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    value: String,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Valeurs du système pour tous les rôles (lancé hors du fil de l'interface).
pub fn load_system() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                ROLES
                    .iter()
                    .map(|r| (r.id.to_string(), apps::system_default(r)))
                    .collect::<BTreeMap<_, _>>()
            })
            .await
            .unwrap_or_default()
        },
        |m| wrap(Msg::SystemLoaded(m)),
    )
}

impl App {
    pub(in crate::ui) fn apps_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        let dim = self.ink().dim;
        let mut col = column![
            self.section("Applications par défaut"),
            text(
                "Un choix est aussi appliqué au système (xdg-open, xdg-mime). \
                 « hyprdesk open <rôle> » lance l'application, par exemple depuis un raccourci."
            )
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(14);
        for r in ROLES {
            let current = self.cfg.general.apps.get(r.id).cloned();
            let mut choices: Vec<Choice> = apps::candidates(r, &self.apps)
                .into_iter()
                .map(|(value, label)| Choice { value, label })
                .collect();
            // Choix en place hors de la liste : une commande, ou une
            // application désinstallée depuis (signalée, pas proposée ailleurs).
            if let Some(v) = &current
                && !choices.iter().any(|c| &c.value == v)
            {
                choices.insert(
                    0,
                    Choice {
                        value: v.clone(),
                        label: if apps::installed(v, &self.apps) {
                            crate::tr!("{v} (commande)", v = v)
                        } else {
                            crate::tr!("{v} (non installée)", v = v)
                        },
                    },
                );
            }
            let selected = current
                .as_ref()
                .and_then(|v| choices.iter().find(|c| &c.value == v).cloned());
            let system = p.apps.system.get(r.id).cloned().flatten();
            let missing = |s: &str| {
                if apps::installed(s, &self.apps) {
                    ""
                } else {
                    crate::i18n::tr(" (non installée)")
                }
            };
            let status = match (&current, &system) {
                (Some(_), Some(s)) => crate::tr!("système : {s}{m}", s = s, m = missing(s)),
                (None, Some(s)) => crate::tr!("celle du système : {s}{m}", s = s, m = missing(s)),
                (_, None) => crate::i18n::tr("aucune au système").into(),
            };
            let role = r.id.to_string();
            let role_custom = role.clone();
            let role_submit = role.clone();
            let mut controls = row![
                pick_list(choices, selected, move |c: Choice| {
                    wrap(Msg::Pick(role.clone(), c.value))
                })
                .placeholder(crate::i18n::tr("Celle du système"))
                .font(self.font)
                .width(240),
                text_input(
                    crate::i18n::tr("ou une commande"),
                    p.apps.custom.get(r.id).map(String::as_str).unwrap_or("")
                )
                .font(self.font)
                .on_input(move |v| wrap(Msg::Custom(role_custom.clone(), v)))
                .on_submit(wrap(Msg::CustomSubmit(role_submit)))
                .padding(6)
                .width(170),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            if current.is_some() {
                controls = controls.push(
                    button(text("Système").font(self.font).size(12))
                        .padding([4, 8])
                        .style(button::secondary)
                        .on_press(wrap(Msg::UseSystem(r.id.to_string()))),
                );
            }
            col = col.push(
                self.field(
                    r.label,
                    "",
                    column![controls, text(status).font(self.font).size(12).color(dim)]
                        .spacing(4)
                        .width(Length::Fill),
                ),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn apps_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::SystemLoaded(m) => {
                if let Some(p) = self.panel.as_mut() {
                    p.apps.system = m;
                }
                Task::none()
            }
            Msg::Custom(r, v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.apps.custom.insert(r, v);
                }
                Task::none()
            }
            Msg::CustomSubmit(r) => {
                let v = self
                    .panel
                    .as_mut()
                    .and_then(|p| p.apps.custom.remove(&r))
                    .unwrap_or_default();
                if v.trim().is_empty() {
                    return Task::none();
                }
                self.update_inner(wrap(Msg::Pick(r, v.trim().to_string())))
            }
            Msg::UseSystem(r) => {
                self.settings.apps.remove(&r);
                self.commit()
            }
            Msg::Pick(r, v) => {
                let v = v.trim_end_matches(".desktop").to_string();
                self.settings.apps.insert(r.clone(), v.clone());
                let saved = self.commit();
                let Some(role) = apps::role(&r) else {
                    return saved;
                };
                let system = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || apps::apply_system(role, &v))
                            .await
                            .map_err(|e| e.to_string())
                            .and_then(|r| r)
                    },
                    |res| {
                        if let Err(e) = res {
                            tracing::warn!("{}", crate::tr!("application par défaut : {e}", e = e));
                            crate::hypr::notify(3, 5000, &format!("hyprdesk : {e}"));
                        }
                        UiMsg::Noop
                    },
                )
                .chain(load_system());
                Task::batch([saved, system])
            }
        }
    }
}
