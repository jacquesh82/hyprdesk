//! Paquet intégré « Layouts » : disposition des fenêtres par Hyprland —
//! layout par défaut et par workspace, options de Dwindle, Master et
//! Scrolling, layouts personnalisés fournis par hyprdesk.
//! Domaine : `crate::tiling`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::BTreeMap;

use crate::i18n::text;
use iced::widget::{button, column, container, pick_list, row, slider, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::Config;
use crate::tiling::{self, Kind, OptValue, Tiling};

pub const INFO: PluginInfo = PluginInfo {
    id: "layouts",
    label: "Layouts",
    icon: "view-grid",
    summary: "Disposition des fenêtres : layout par défaut et par workspace, options de \
              Dwindle, Master et Scrolling, layouts personnalisés.",
    builtin: true,
};

/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("general", "Général"),
    ("dwindle", "Dwindle"),
    ("master", "Master"),
    ("scrolling", "Scrolling"),
    ("monocle", "Monocle"),
    ("custom", "Personnalisés"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Layouts(m)))
}

#[derive(Debug, Clone, Default)]
pub struct State {
    /// Valeurs en place dans Hyprland (`dwindle.preserve_split` → `true`).
    pub current: BTreeMap<String, String>,
    /// Raccourcis lus dans hyprland.lua.
    pub binds: Vec<tiling::Bind>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Layout par défaut (`None` : celui de hyprland.lua).
    Default(Option<String>),
    /// Layout d'un workspace (`None` : le layout par défaut).
    Workspace(String, Option<String>),
    /// Option (chemin, valeur ; `None` : celle de hyprland.lua).
    Opt(String, Option<OptValue>),
    /// Curseur en cours de glissé (aperçu), puis enregistrement.
    Drag(String, f64),
    Commit,
    Current(BTreeMap<String, String>),
    Binds(Vec<tiling::Bind>),
    /// Copie une ligne Lua (presse-papiers).
    Copy(String),
}

/// Réglages effectifs : ceux de la config, ou rien si le paquet est coupé.
pub fn effective(cfg: &Config) -> Tiling {
    if cfg.plugin_on(INFO.id) {
        cfg.general.tiling.clone()
    } else {
        Tiling::default()
    }
}

/// Lit les valeurs en place dans Hyprland, hors du fil de l'interface.
pub fn load_current() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(tiling::current)
                .await
                .unwrap_or_default()
        },
        |m| wrap(Msg::Current(m)),
    )
}

/// Lit les raccourcis de hyprland.lua.
pub fn load_binds(config_path: std::path::PathBuf) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || tiling::load_binds(&config_path))
                .await
                .unwrap_or_default()
        },
        |b| wrap(Msg::Binds(b)),
    )
}

/// Choix d'un layout dans une liste (avec « par défaut »).
#[derive(Debug, Clone, PartialEq)]
struct LayoutChoice {
    value: Option<String>,
    label: String,
}

impl std::fmt::Display for LayoutChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ValueChoice {
    value: OptValue,
    label: String,
}

impl std::fmt::Display for ValueChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

impl App {
    fn tiling_settings(&self) -> Tiling {
        self.settings
            .tiling
            .clone()
            .unwrap_or_else(|| self.cfg.general.tiling.clone())
    }

    fn edit_tiling(&mut self, f: impl FnOnce(&mut Tiling)) {
        let mut t = self.tiling_settings();
        f(&mut t);
        self.settings.tiling = Some(t);
    }

    /// Pose les layouts dans Hyprland (hors du fil de l'interface).
    pub(in crate::ui) fn sync_tiling(&self) -> Task<UiMsg> {
        let t = effective(&self.cfg);
        Task::perform(
            async move {
                let r = tokio::task::spawn_blocking(move || tiling::apply(&t)).await;
                if let Ok(Err(e)) = r {
                    tracing::warn!("{}", crate::tr!("layouts : {e}", e = e));
                }
            },
            |_| UiMsg::Noop,
        )
    }

    pub(in crate::ui) fn layouts_page(&self) -> Element<'_, UiMsg> {
        let tab = self.subtab(INFO.id);
        match tab {
            "custom" => self.layouts_custom(),
            // Les raccourcis d'abord : c'est ce qu'on cherche en ouvrant l'onglet.
            "dwindle" | "master" | "scrolling" => {
                column![self.layout_keys(tab), self.adv(self.layouts_options(tab))]
                    .spacing(14)
                    .into()
            }
            "monocle" => column![
                self.section("Monocle"),
                text(crate::i18n::tr(
                    "Une seule fenêtre visible à la fois, en plein écran ; les autres attendent \
                     derrière. Pas d'option : on passe de l'une à l'autre par raccourci."
                ))
                .font(self.font)
                .size(12)
                .color(Color {
                    a: 0.6,
                    ..color(self.cfg.general.theme.fg, 1.0)
                }),
                self.layout_keys("monocle"),
            ]
            .spacing(14)
            .into(),
            _ => self.layouts_general(),
        }
    }

    fn layout_choices(&self, none_label: &str) -> Vec<LayoutChoice> {
        let mut v = vec![LayoutChoice {
            value: None,
            label: none_label.to_string(),
        }];
        v.extend(tiling::BUILTIN.iter().map(|(n, l)| LayoutChoice {
            value: Some(n.to_string()),
            label: l.to_string(),
        }));
        v.extend(tiling::CUSTOM.iter().map(|c| LayoutChoice {
            value: Some(c.name.to_string()),
            label: crate::tr!("{label} (hyprdesk)", label = crate::i18n::tr(c.label)),
        }));
        // Un layout Lua réglé à la main reste proposé.
        let t = self.cfg.general.tiling.clone();
        for name in t.layout.iter().chain(t.workspaces.values()) {
            if !v.iter().any(|c| c.value.as_deref() == Some(name.as_str())) {
                v.push(LayoutChoice {
                    value: Some(name.clone()),
                    label: name.clone(),
                });
            }
        }
        v
    }

    fn layouts_general(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let dim = self.ink().dim;
        let small = |s: String| text(s).font(self.font).size(12).color(dim);
        let t = &g.tiling;
        let current = self
            .panel
            .as_ref()
            .and_then(|p| p.layouts.current.get("general.layout").cloned());
        let from_hypr = crate::tr!(
            "Celui de hyprland.lua ({layout})",
            layout = current.clone().unwrap_or_else(|| "dwindle".into())
        );
        let choices = self.layout_choices(&from_hypr);
        let selected = choices.iter().find(|c| c.value == t.layout).cloned();
        let default_pick = pick_list(choices.clone(), selected, |c: LayoutChoice| {
            wrap(Msg::Default(c.value))
        })
        .font(self.font)
        .width(320);

        // Un layout par workspace : ceux qui existent toujours, et ceux déjà réglés.
        let upto = g.workspaces.unwrap_or(10).max(1) as usize;
        let mut names: Vec<String> = (1..=upto).map(|i| i.to_string()).collect();
        for w in t.workspaces.keys() {
            if !names.contains(w) {
                names.push(w.clone());
            }
        }
        let ws_choices = self.layout_choices(crate::i18n::tr("Par défaut"));
        let rows = names.chunks(2).map(|pair| {
            row(pair.iter().map(|ws| {
                let now = t.workspaces.get(ws).cloned();
                let sel = ws_choices.iter().find(|c| c.value == now).cloned();
                let w = ws.clone();
                row![
                    text(crate::tr!("Workspace {ws}", ws = ws))
                        .font(self.font)
                        .size(14)
                        .width(110),
                    pick_list(ws_choices.clone(), sel, move |c: LayoutChoice| {
                        wrap(Msg::Workspace(w.clone(), c.value))
                    })
                    .font(self.font)
                    .width(180),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .into()
            }))
            .spacing(16)
            .into()
        });
        column![
            self.section("Layout par défaut"),
            self.field("Layout des workspaces", "layout", default_pick),
            small(
                crate::i18n::tr(
                    "Dwindle découpe en deux à chaque fenêtre ; Master garde une fenêtre principale \
                     et une pile ; Scrolling aligne des colonnes qui défilent ; Monocle montre une \
                     fenêtre à la fois, en plein écran."
                )
                .into()
            ),
            self.section("Par workspace"),
            self.field(
                "Layout de chaque workspace",
                "layout_workspaces",
                column(rows).spacing(8)
            ),
            small(
                crate::i18n::tr(
                    "S'applique tout de suite, y compris aux workspaces déjà ouverts. Les réglages \
                     faits ici l'emportent sur hyprland.lua ; revenir à « Celui de hyprland.lua » \
                     lui rend la main."
                )
                .into()
            ),
        ]
        .spacing(14)
        .into()
    }

    fn layouts_options(&self, section: &'static str) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let dim = self.ink().dim;
        let accent = color(g.theme.accent, 1.0);
        let set = &g.tiling.options;
        let current = self.panel.as_ref().map(|p| &p.layouts.current);
        let mut col = column![
            self.section(match section {
                "dwindle" => "Dwindle",
                "master" => "Master",
                _ => "Scrolling",
            }),
            text(match section {
                "dwindle" => crate::i18n::tr(
                    "Chaque nouvelle fenêtre partage en deux la place de la fenêtre active, \
                     en alternant le sens."
                ),
                "master" => crate::i18n::tr(
                    "Une fenêtre maître occupe une grande part de l'écran ; les autres \
                     s'empilent à côté."
                ),
                _ => crate::i18n::tr(
                    "Les fenêtres sont rangées en colonnes qui défilent, au-delà de l'écran \
                     si besoin."
                ),
            })
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(14);
        for o in tiling::OPTIONS.iter().filter(|o| o.section == section) {
            let path = o.path();
            let ours = set.get(&path).cloned();
            let theirs = current.and_then(|c| c.get(&path)).cloned();
            let control: Element<'_, UiMsg> = match &o.kind {
                Kind::Bool => {
                    let on = match &ours {
                        Some(OptValue::Bool(b)) => *b,
                        _ => matches!(theirs.as_deref(), Some("true") | Some("1")),
                    };
                    let p = path.clone();
                    toggler(on)
                        .on_toggle(move |v| wrap(Msg::Opt(p.clone(), Some(OptValue::Bool(v)))))
                        .into()
                }
                Kind::Num { min, max, step } => {
                    let v = match &ours {
                        Some(OptValue::Num(n)) => *n,
                        _ => theirs
                            .as_deref()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(*min),
                    };
                    let p = path.clone();
                    row![
                        slider((*min as f32)..=(*max as f32), v as f32, move |x| wrap(
                            Msg::Drag(p.clone(), x as f64)
                        ))
                        .step(*step as f32)
                        .on_release(wrap(Msg::Commit))
                        .width(260),
                        text(format!("{}", (v * 100.0).round() / 100.0))
                            .font(self.font)
                            .size(13),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .into()
                }
                Kind::IntChoice(list) => {
                    let choices: Vec<ValueChoice> = list
                        .iter()
                        .map(|(v, l)| ValueChoice {
                            value: OptValue::Num(*v as f64),
                            label: crate::i18n::tr(l).to_string(),
                        })
                        .collect();
                    let now = ours.clone().or_else(|| {
                        theirs
                            .as_deref()
                            .and_then(|s| s.parse::<f64>().ok())
                            .map(OptValue::Num)
                    });
                    let sel = choices
                        .iter()
                        .find(|c| Some(&c.value) == now.as_ref())
                        .cloned();
                    let p = path.clone();
                    pick_list(choices, sel, move |c: ValueChoice| {
                        wrap(Msg::Opt(p.clone(), Some(c.value)))
                    })
                    .font(self.font)
                    .width(320)
                    .into()
                }
                Kind::Choice(list) => {
                    let choices: Vec<ValueChoice> = list
                        .iter()
                        .map(|(v, l)| ValueChoice {
                            value: OptValue::Str(v.to_string()),
                            label: crate::i18n::tr(l).to_string(),
                        })
                        .collect();
                    let now = ours.clone().or_else(|| theirs.clone().map(OptValue::Str));
                    let sel = choices
                        .iter()
                        .find(|c| Some(&c.value) == now.as_ref())
                        .cloned();
                    let p = path.clone();
                    pick_list(choices, sel, move |c: ValueChoice| {
                        wrap(Msg::Opt(p.clone(), Some(c.value)))
                    })
                    .font(self.font)
                    .width(320)
                    .into()
                }
            };
            // Réglé ici : un bouton rend la main à hyprland.lua.
            let origin: Element<'_, UiMsg> = if ours.is_some() {
                let p = path.clone();
                row![
                    text(crate::i18n::tr("réglé ici"))
                        .font(self.font)
                        .size(12)
                        .color(accent),
                    button(text(crate::i18n::tr("Rétablir")).font(self.font).size(12))
                        .padding([2, 8])
                        .style(button::text)
                        .on_press(wrap(Msg::Opt(p, None))),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
            } else {
                text(crate::i18n::tr("hyprland.lua"))
                    .font(self.font)
                    .size(12)
                    .color(dim)
                    .into()
            };
            col = col.push(
                self.field(
                    o.label,
                    "",
                    row![container(control).width(Length::Fill), origin]
                        .spacing(10)
                        .align_y(Alignment::Center),
                ),
            );
        }
        col.into()
    }

    fn layouts_custom(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let accent = color(g.theme.accent, 1.0);
        let mut col = column![
            self.section("Layouts de hyprdesk"),
            text(crate::i18n::tr(
                "Écrits en Lua avec l'API de layouts de Hyprland, enregistrés par hyprdesk. \
                 À choisir comme layout par défaut ou pour un workspace (onglet Général)."
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(14);
        for c in tiling::CUSTOM {
            let active = g.tiling.layout.as_deref() == Some(c.name);
            col = col.push(
                container(
                    row![
                        column![
                            text(crate::i18n::tr(c.label)).font(self.font).size(14),
                            text(crate::i18n::tr(c.summary))
                                .font(self.font)
                                .size(12)
                                .color(dim),
                        ]
                        .spacing(4)
                        .width(Length::Fill),
                        if active {
                            Element::from(
                                text(crate::i18n::tr("par défaut"))
                                    .font(self.font)
                                    .size(12)
                                    .color(accent),
                            )
                        } else {
                            button(text(crate::i18n::tr("Par défaut")).font(self.font).size(12))
                                .padding([4, 10])
                                .style(button::secondary)
                                .on_press(wrap(Msg::Default(Some(c.name.to_string()))))
                                .into()
                        },
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding(10)
                .style(move |_| container::Style {
                    border: Border::default()
                        .rounded(10)
                        .width(1)
                        .color(Color { a: 0.15, ..fg }),
                    ..Default::default()
                }),
            );
        }
        col = col.push(self.section("Le vôtre"));
        col = col.push(
            text(crate::i18n::tr(
                "Dans hyprland.lua : hl.layout.register(\"mien\", { recalculate = function(ctx) \
                 for i, t in ipairs(ctx.targets) do t:place(ctx:column(i, #ctx.targets)) end \
                 end }) — puis, ici ou dans hyprdesk.lua : layout = \"lua:mien\". Outils de ctx : \
                 area, targets, grid_cell(i, colonnes, rangées), column(i, n), row(i, n), \
                 split(boîte, côté, part).",
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        );
        col.into()
    }

    /// Raccourcis d'un layout : ceux de hyprland.lua, puis ceux proposés.
    fn layout_keys(&self, layout: &str) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let accent = color(g.theme.accent, 1.0);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let binds = self
            .panel
            .as_ref()
            .map(|p| p.layouts.binds.as_slice())
            .unwrap_or_default();
        let lines = tiling::key_lines(layout, binds);
        let chip = |s: String, c: Color| {
            container(text(s).font(self.font).size(12).color(c))
                .padding([2, 8])
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.08, ..fg }.into()),
                    border: Border::default()
                        .rounded(6)
                        .width(1)
                        .color(Color { a: 0.2, ..c }),
                    ..Default::default()
                })
        };
        let mut col = column![
            self.section("Raccourcis"),
            text(match layout {
                "scrolling" => crate::i18n::tr(
                    "Changer de fenêtre à gauche ou à droite passe d'une colonne à l'autre ; \
                     déplacer la fenêtre à gauche ou à droite la change de colonne."
                ),
                _ => crate::i18n::tr(
                    "Lus dans hyprland.lua. Une action sans raccourci : celui proposé, à ajouter \
                     à hyprland.lua (bouton Copier)."
                ),
            })
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(8);
        if binds.is_empty() {
            col = col.push(
                text(crate::i18n::tr(
                    "Aucun raccourci lu (hyprland.lua introuvable à côté de hyprdesk.lua).",
                ))
                .font(self.font)
                .size(12)
                .color(dim),
            );
        }
        for l in lines {
            let mut keys = row![].spacing(6).align_y(Alignment::Center);
            for b in &l.bound {
                keys = keys.push(chip(b.clone(), accent));
            }
            if let Some((s, free)) = l.suggest {
                keys = keys.push(chip(s.to_string(), dim));
                keys = keys.push(
                    text(if free {
                        crate::i18n::tr("proposé")
                    } else {
                        crate::i18n::tr("proposé, déjà pris")
                    })
                    .font(self.font)
                    .size(12)
                    .color(if free { dim } else { warn }),
                );
                if let Some(lua) = l.lua.clone() {
                    keys = keys.push(
                        button(text(crate::i18n::tr("Copier")).font(self.font).size(12))
                            .padding([2, 8])
                            .style(button::text)
                            .on_press(wrap(Msg::Copy(lua))),
                    );
                }
            }
            col = col.push(
                row![
                    text(crate::i18n::tr(l.label))
                        .font(self.font)
                        .size(13)
                        .width(Length::FillPortion(5)),
                    container(keys).width(Length::FillPortion(6)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn layouts_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Binds(b) => {
                if let Some(p) = self.panel.as_mut() {
                    p.layouts.binds = b;
                }
                Task::none()
            }
            Msg::Copy(lua) => Task::perform(
                async move {
                    let _ = tokio::task::spawn_blocking(move || {
                        use std::io::Write;
                        if let Ok(mut c) = std::process::Command::new("wl-copy")
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                        {
                            if let Some(i) = c.stdin.as_mut() {
                                let _ = i.write_all(lua.as_bytes());
                            }
                            drop(c.stdin.take());
                            let _ = c.wait();
                        }
                        crate::hypr::notify(
                            1,
                            2500,
                            crate::i18n::tr("hyprdesk : ligne copiée — à coller dans hyprland.lua"),
                        );
                    })
                    .await;
                },
                |_| UiMsg::Noop,
            ),
            Msg::Default(l) => {
                self.edit_tiling(|t| t.layout = l);
                self.commit()
            }
            Msg::Workspace(ws, l) => {
                self.edit_tiling(|t| match l {
                    Some(l) => {
                        t.workspaces.insert(ws, l);
                    }
                    None => {
                        t.workspaces.remove(&ws);
                    }
                });
                self.commit()
            }
            Msg::Opt(path, v) => {
                self.edit_tiling(|t| match v {
                    Some(v) => {
                        t.options.insert(path, v);
                    }
                    None => {
                        t.options.remove(&path);
                    }
                });
                self.commit()
            }
            Msg::Drag(path, x) => {
                let x = (x * 100.0).round() / 100.0;
                self.edit_tiling(|t| {
                    t.options.insert(path, OptValue::Num(x));
                });
                self.apply()
            }
            Msg::Commit => self.commit(),
            Msg::Current(m) => {
                if let Some(p) = self.panel.as_mut() {
                    p.layouts.current = m;
                }
                Task::none()
            }
        }
    }
}
