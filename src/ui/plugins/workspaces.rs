//! Paquet intégré « Workspaces » : Nombre de workspaces permanents, affectation des applications.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use iced::widget::{button, column, pick_list, row, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length, Task};

use super::super::panel::{Panel, PanelMsg};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::Origin;

pub const INFO: PluginInfo = PluginInfo {
    id: "workspaces",
    label: "Workspaces",
    icon: "workspace-switcher",
    summary: "Nombre de workspaces permanents, affectation des applications.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Workspaces(m)))
}

/// État propre à ce paquet, porté par le tiroir des réglages.
pub struct State {
    pub assign_filter: String,
    pub assign_target: String,
    pub assign_silent: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            assign_filter: String::new(),
            assign_target: "1".into(),
            assign_silent: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Nombre de workspaces permanents (`None` : laissé à Hyprland).
    WsCount(Option<u8>),
    AssignFilter(String),
    /// Workspace proposé pour la prochaine affectation.
    AssignTarget(String),
    AssignSilent(bool),
    AddAssign(crate::config::AppMatch),
    /// Change le workspace d'une affectation des réglages.
    AssignWorkspace(usize, String),
    AssignRuleSilent(usize, bool),
    RemoveAssign(usize),
    /// Écran d'un workspace : connecteur, `primary`, ou `None` (automatique).
    WsMonitor(String, Option<String>),
    /// Écran dédié : n'accueille que ses workspaces.
    Dedicated(String, bool),
}

/// Choix d'écran d'un workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenChoice {
    value: Option<String>,
    label: String,
}

impl std::fmt::Display for ScreenChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

impl App {
    /// Écran de chaque workspace : automatique, l'écran principal, ou un écran.
    fn workspace_screens(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let mut choices = vec![
            ScreenChoice {
                value: None,
                label: "Automatique".into(),
            },
            ScreenChoice {
                value: Some("primary".into()),
                label: "Écran principal".into(),
            },
        ];
        for m in &self.monitors {
            choices.push(ScreenChoice {
                value: Some(m.name.clone()),
                label: m.name.clone(),
            });
        }
        let upto = g.workspaces.unwrap_or(10).max(1) as usize;
        let mut names: Vec<String> = (1..=upto).map(|i| i.to_string()).collect();
        for w in g.workspace_monitors.values().flatten() {
            if !names.contains(w) {
                names.push(w.clone());
            }
        }
        let current = |ws: &str| {
            g.workspace_monitors
                .iter()
                .find(|(_, l)| l.iter().any(|w| w == ws))
                .map(|(m, _)| m.clone())
        };
        let rows = names.chunks(2).map(|pair| {
            row(pair.iter().map(|ws| {
                let now = current(ws);
                let selected = choices.iter().find(|c| c.value == now).cloned();
                let w = ws.clone();
                row![
                    text(format!("Workspace {ws}"))
                        .font(self.font)
                        .size(14)
                        .width(110),
                    pick_list(choices.clone(), selected, move |c: ScreenChoice| {
                        wrap(Msg::WsMonitor(w.clone(), c.value))
                    })
                    .font(self.font)
                    .width(170),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill)
                .into()
            }))
            .spacing(16)
            .into()
        });
        column![
            self.section("Écrans"),
            text(
                "Le premier workspace d'un écran est celui qu'il affiche par défaut. Un workspace \
                 permanent change d'écran aussitôt ; les autres, à leur création."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.field(
                "Écran de chaque workspace",
                "workspace_monitors",
                column(rows).spacing(8)
            ),
            self.field(
                "Écrans dédiés : seulement leurs workspaces, les autres vont sur l'écran principal",
                "workspace_dedicated",
                column(self.monitors.iter().map(|m| {
                    let name = m.name.clone();
                    iced::widget::toggler(g.workspace_dedicated.contains(&m.name))
                        .label(m.name.clone())
                        .on_toggle(move |v| wrap(Msg::Dedicated(name.clone(), v)))
                        .into()
                }))
                .spacing(8),
            ),
        ]
        .spacing(14)
        .into()
    }

    pub(in crate::ui) fn tab_workspaces<'a>(&'a self, p: &'a Panel) -> Element<'a, UiMsg> {
        use crate::config::{AppMatch, MAX_WORKSPACES};
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let small = |s: String| text(s).font(self.font).size(12).color(dim);

        // Nombre de workspaces : interrupteur, puis − N +.
        let count = self.settings.workspaces;
        let shown = g.workspaces.or(count).unwrap_or(5);
        let step = |n: u8| wrap(Msg::WsCount(Some(n)));
        let mut stepper = row![
            toggler(count.is_some())
                .label("Nombre fixe")
                .on_toggle(move |on| wrap(Msg::WsCount(on.then_some(shown)))),
        ]
        .spacing(14)
        .align_y(Alignment::Center);
        if count.is_some() {
            stepper = stepper.push(
                row![
                    button(text("−").font(self.font).size(16))
                        .padding([2, 12])
                        .on_press_maybe((shown > 1).then(|| step(shown - 1))),
                    text(shown.to_string())
                        .font(self.font)
                        .size(16)
                        .width(32)
                        .align_x(Alignment::Center),
                    button(text("+").font(self.font).size(16))
                        .padding([2, 12])
                        .on_press_maybe((shown < MAX_WORKSPACES).then(|| step(shown + 1))),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }
        let count_help = match g.workspaces {
            Some(n) => format!(
                "Les workspaces 1 à {n} existent toujours, même vides, et restent dans la barre. \
                 Leur écran se règle ci-dessous."
            ),
            None => {
                "Hyprland crée les workspaces à la demande et retire ceux qui sont vides.".into()
            }
        };

        // Workspaces proposés aux affectations.
        let upto = g.workspaces.unwrap_or(10).max(1);
        let mut targets: Vec<String> = (1..=upto).map(|i| i.to_string()).collect();
        for a in &self.cfg.assigns {
            if !targets.contains(&a.workspace) {
                targets.push(a.workspace.clone());
            }
        }
        let app_name = |app: &AppMatch| match app {
            AppMatch::App(id) => self
                .apps
                .iter()
                .find(|a| &a.id == id)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| id.clone()),
            AppMatch::Class(re) => format!("classe {re}"),
        };

        let mut rules = column![].spacing(8);
        if self.settings.assigns.is_empty() {
            rules = rules.push(small("Aucune affectation réglée ici.".into()));
        }
        for (i, a) in self.settings.assigns.iter().enumerate() {
            let overridden = self
                .cfg
                .assigns
                .iter()
                .any(|u| u.origin == Origin::User && u.app == a.app);
            let mut name = column![text(app_name(&a.app)).font(self.font).size(14)];
            if overridden {
                name = name.push(
                    text("surchargée dans hyprdesk.lua — sans effet ici")
                        .font(self.font)
                        .size(11)
                        .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
                );
            }
            rules = rules.push(
                row![
                    name.width(Length::Fill),
                    small("workspace".into()),
                    pick_list(targets.clone(), Some(a.workspace.clone()), move |w| {
                        wrap(Msg::AssignWorkspace(i, w))
                    })
                    .font(self.font)
                    .width(70),
                    toggler(a.silent)
                        .label("Sans y aller")
                        .on_toggle(move |v| wrap(Msg::AssignRuleSilent(i, v))),
                    button(text("Retirer").font(self.font).size(13))
                        .padding([4, 10])
                        .on_press(wrap(Msg::RemoveAssign(i))),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }
        let user_rules: Vec<String> = self
            .cfg
            .assigns
            .iter()
            .filter(|a| a.origin == Origin::User)
            .map(|a| format!("{} → {}", app_name(&a.app), a.workspace))
            .collect();
        if !user_rules.is_empty() {
            rules = rules.push(small(format!(
                "Dans hyprdesk.lua : {}",
                user_rules.join(" · ")
            )));
        }

        // Ajout : recherche parmi les applications installées, ou classe brute.
        let filter = p.workspaces.assign_filter.trim().to_lowercase();
        let mut picker = column![].spacing(2);
        if !filter.is_empty() {
            let pick = |label: String, detail: String, m: AppMatch| {
                button(
                    row![
                        text(label).font(self.font).size(14).width(Length::Fill),
                        text(detail).font(self.font).size(11).color(dim)
                    ]
                    .spacing(8),
                )
                .width(Length::Fill)
                .padding([6, 10])
                .on_press(wrap(Msg::AddAssign(m)))
                .style(move |_, s| button::Style {
                    background: matches!(s, button::Status::Hovered)
                        .then(|| Background::Color(Color { a: 0.1, ..fg })),
                    text_color: fg,
                    border: Border::default().rounded(6),
                    ..Default::default()
                })
            };
            for a in self
                .apps
                .iter()
                .filter(|a| {
                    a.name.to_lowercase().contains(&filter) || a.id.to_lowercase().contains(&filter)
                })
                .take(10)
            {
                picker = picker.push(pick(
                    a.name.clone(),
                    a.id.clone(),
                    AppMatch::App(a.id.clone()),
                ));
            }
            let raw = p.workspaces.assign_filter.trim().to_string();
            picker = picker.push(pick(
                format!("Classe de fenêtre « {raw} »"),
                "expression régulière".into(),
                AppMatch::Class(raw),
            ));
        }

        column![
            self.section("Nombre de workspaces"),
            self.field("Workspaces permanents", "workspaces", stepper),
            small(count_help),
            self.workspace_screens(),
            self.section("Applications"),
            small(
                "Les fenêtres d'une application s'ouvrent sur le workspace choisi. \
                 « Sans y aller » : elles s'y ouvrent sans vous y emmener."
                    .into()
            ),
            rules,
            self.field(
                "Affecter une application",
                "",
                column![
                    row![
                        text_input("Rechercher (firefox, code…)", &p.workspaces.assign_filter)
                            .font(self.font)
                            .on_input(|s| wrap(Msg::AssignFilter(s)))
                            .padding(8)
                            .width(Length::Fill),
                        small("vers".into()),
                        pick_list(
                            targets.clone(),
                            Some(p.workspaces.assign_target.clone()),
                            |w| { wrap(Msg::AssignTarget(w)) }
                        )
                        .font(self.font)
                        .width(70),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    toggler(p.workspaces.assign_silent)
                        .label("Sans y aller")
                        .on_toggle(|v| wrap(Msg::AssignSilent(v))),
                    picker,
                ]
                .spacing(8),
            ),
        ]
        .spacing(14)
        .into()
    }

    pub(in crate::ui) fn workspaces_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::WsCount(n) => {
                self.settings.workspaces = n;
                self.commit()
            }
            Msg::AssignFilter(f) => {
                panel.workspaces.assign_filter = f;
                Task::none()
            }
            Msg::AssignTarget(w) => {
                panel.workspaces.assign_target = w;
                Task::none()
            }
            Msg::AssignSilent(v) => {
                panel.workspaces.assign_silent = v;
                Task::none()
            }
            Msg::AddAssign(app) => {
                let (ws, silent) = (
                    panel.workspaces.assign_target.clone(),
                    panel.workspaces.assign_silent,
                );
                panel.workspaces.assign_filter.clear();
                self.settings.set_assign(app, &ws, silent);
                self.commit()
            }
            Msg::AssignWorkspace(i, ws) => {
                if let Some(a) = self.settings.assigns.get_mut(i) {
                    a.workspace = ws;
                }
                self.commit()
            }
            Msg::AssignRuleSilent(i, v) => {
                if let Some(a) = self.settings.assigns.get_mut(i) {
                    a.silent = v;
                }
                self.commit()
            }
            Msg::RemoveAssign(i) => {
                self.settings.remove_assign(i);
                self.commit()
            }
            Msg::Dedicated(mon, on) => {
                let mut set = self
                    .settings
                    .workspace_dedicated
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.workspace_dedicated.clone());
                if on {
                    set.insert(mon);
                } else {
                    set.remove(&mon);
                }
                self.settings.workspace_dedicated = Some(set);
                self.commit()
            }
            Msg::WsMonitor(ws, mon) => {
                let mut map = self
                    .settings
                    .workspace_monitors
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.workspace_monitors_declared.clone());
                for l in map.values_mut() {
                    l.retain(|w| *w != ws);
                }
                if let Some(mon) = mon {
                    let l = map.entry(mon).or_default();
                    l.push(ws);
                    l.sort_by_key(|w| w.parse::<i64>().unwrap_or(i64::MAX));
                }
                map.retain(|_, l| !l.is_empty());
                self.settings.workspace_monitors = Some(map);
                self.commit()
            }
        }
    }
}
