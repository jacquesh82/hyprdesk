//! Paquet intégré « Animations » : animations et effets de Hyprland —
//! animation par élément (durée, courbe, style), arrondis, opacité, flou et
//! ses variantes, ombre, lueur, flou de mouvement, ondulation.
//! Domaine : `crate::fx`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::BTreeMap;

use crate::i18n::text;
use iced::widget::{button, column, container, pick_list, row, slider, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::Config;
use crate::fx::{self, Anim, Fx};
use crate::tiling::{Kind, OptValue};

pub const INFO: PluginInfo = PluginInfo {
    id: "animations",
    label: "Animations",
    icon: "preferences-desktop-animation",
    summary: "Animations et effets de Hyprland : fenêtres, workspaces, fondus, flou et ses \
              variantes, ombre, lueur, ondulation.",
    builtin: true,
};

/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[("animations", "Animations"), ("effets", "Effets")];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Animations(m)))
}

#[derive(Debug, Clone, Default)]
pub struct State {
    /// Valeurs en place dans Hyprland (`decoration.rounding` → `5`).
    pub current: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Effet (chemin, valeur ; `None` : celle de hyprland.lua).
    Opt(String, Option<OptValue>),
    /// Curseur d'un effet en cours de glissé (aperçu), puis enregistrement.
    Drag(String, f64),
    /// Réglage d'un élément animé (`None` : celui de hyprland.lua).
    Anim(String, Option<Anim>),
    /// Durée d'un élément en cours de glissé.
    Speed(String, f64),
    Commit,
    Current(BTreeMap<String, String>),
}

/// Réglages effectifs : ceux de la config, ou rien si le paquet est coupé.
pub fn effective(cfg: &Config) -> Fx {
    if cfg.plugin_on(INFO.id) {
        cfg.general.fx.clone()
    } else {
        Fx::default()
    }
}

/// Lit les valeurs en place dans Hyprland, hors du fil de l'interface.
pub fn load_current() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(fx::current)
                .await
                .unwrap_or_default()
        },
        |m| wrap(Msg::Current(m)),
    )
}

/// Entrée d'une liste déroulante : valeur et libellé affiché.
#[derive(Debug, Clone, PartialEq)]
struct Choice<T> {
    value: T,
    label: String,
}

impl<T> std::fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

impl App {
    fn fx_settings(&self) -> Fx {
        self.settings
            .fx
            .clone()
            .unwrap_or_else(|| self.cfg.general.fx.clone())
    }

    fn edit_fx(&mut self, f: impl FnOnce(&mut Fx)) {
        let mut fx = self.fx_settings();
        f(&mut fx);
        self.settings.fx = Some(fx);
    }

    /// Pose animations et effets dans Hyprland (hors du fil de l'interface).
    pub(in crate::ui) fn sync_fx(&self) -> Task<UiMsg> {
        let fx = effective(&self.cfg);
        Task::perform(
            async move {
                let r = tokio::task::spawn_blocking(move || fx::apply(&fx)).await;
                if let Ok(Err(e)) = r {
                    tracing::warn!("{}", crate::tr!("animations : {e}", e = e));
                }
            },
            |_| UiMsg::Noop,
        )
    }

    pub(in crate::ui) fn animations_page(&self) -> Element<'_, UiMsg> {
        match self.subtab(INFO.id) {
            "effets" => self.fx_effects(),
            _ => self.fx_animations(),
        }
    }

    /// « réglé ici » + Rétablir, ou l'origine hyprland.lua.
    fn fx_origin(&self, ours: bool, reset: UiMsg) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        if ours {
            row![
                text(crate::i18n::tr("réglé ici"))
                    .font(self.font)
                    .size(12)
                    .color(color(self.cfg.general.theme.accent, 1.0)),
                button(text(crate::i18n::tr("Rétablir")).font(self.font).size(12))
                    .padding([2, 8])
                    .style(button::text)
                    .on_press(reset),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        } else {
            text(crate::i18n::tr("hyprland.lua"))
                .font(self.font)
                .size(12)
                .color(Color { a: 0.6, ..fg })
                .into()
        }
    }

    /// Contrôle d'un effet selon sa nature.
    fn fx_option(&self, path: &'static str) -> Element<'_, UiMsg> {
        let Some(o) = fx::opt(path) else {
            return iced::widget::Space::new().into();
        };
        let ours = self.cfg.general.fx.options.get(path).cloned();
        let theirs = self
            .panel
            .as_ref()
            .and_then(|p| p.animations.current.get(path))
            .cloned();
        let control: Element<'_, UiMsg> = match &o.kind {
            Kind::Bool => {
                let on = match &ours {
                    Some(OptValue::Bool(b)) => *b,
                    _ => matches!(theirs.as_deref(), Some("true") | Some("1")),
                };
                toggler(on)
                    .on_toggle(move |v| wrap(Msg::Opt(path.into(), Some(OptValue::Bool(v)))))
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
                row![
                    slider((*min as f32)..=(*max as f32), v as f32, move |x| wrap(
                        Msg::Drag(path.into(), x as f64)
                    ))
                    .step(*step as f32)
                    .on_release(wrap(Msg::Commit))
                    .width(240),
                    text(format!("{}", (v * 100.0).round() / 100.0))
                        .font(self.font)
                        .size(13),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
            }
            Kind::IntChoice(list) => {
                let choices: Vec<Choice<i64>> = list
                    .iter()
                    .map(|(v, l)| Choice {
                        value: *v,
                        label: crate::i18n::tr(l).to_string(),
                    })
                    .collect();
                let now = match &ours {
                    Some(OptValue::Num(n)) => Some(*n as i64),
                    _ => theirs.as_deref().and_then(|s| s.parse::<i64>().ok()),
                };
                let sel = choices.iter().find(|c| Some(c.value) == now).cloned();
                pick_list(choices, sel, move |c: Choice<i64>| {
                    wrap(Msg::Opt(path.into(), Some(OptValue::Num(c.value as f64))))
                })
                .font(self.font)
                .width(240)
                .into()
            }
            Kind::Choice(_) => iced::widget::Space::new().into(),
        };
        self.field(
            o.label,
            "",
            row![
                container(control).width(Length::Fill),
                self.fx_origin(ours.is_some(), wrap(Msg::Opt(path.into(), None)))
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
    }

    fn fx_effects(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let mut col = column![
            text(crate::i18n::tr(
                "Effets dessinés par Hyprland autour et derrière les fenêtres. Seuls les \
                 réglages faits ici sont posés ; le reste vient de hyprland.lua. Les variantes \
                 de flou animées sollicitent davantage la carte graphique."
            ))
            .font(self.font)
            .size(12)
            .color(Color { a: 0.6, ..fg }),
        ]
        .spacing(14);
        for (title, keys) in fx::GROUPS {
            col = col.push(self.section(title));
            for k in *keys {
                col = col.push(self.fx_option(k));
            }
        }
        col.into()
    }

    fn fx_animations(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let set = &self.cfg.general.fx.anims;
        let mut col = column![
            self.fx_option("animations.enabled"),
            text(crate::i18n::tr(
                "Chaque élément hérite de son parent (Fenêtres règle l'ouverture et la \
                 fermeture qu'on ne règle pas à part). Durée en dixièmes de seconde : plus \
                 petite, plus rapide."
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(14);
        let curves: Vec<Choice<String>> = fx::CURVES
            .iter()
            .map(|c| Choice {
                value: c.name.to_string(),
                label: crate::i18n::tr(c.label).to_string(),
            })
            .collect();
        for l in fx::LEAVES
            .iter()
            .filter(|l| l.main || set.contains_key(l.name))
        {
            let name = l.name;
            let Some(a) = set.get(name) else {
                col = col.push(
                    self.field(
                        l.label,
                        "",
                        row![
                            container(
                                button(
                                    text(crate::i18n::tr("Personnaliser"))
                                        .font(self.font)
                                        .size(12)
                                )
                                .padding([4, 10])
                                .on_press(wrap(Msg::Anim(name.into(), Some(Anim::new_for(l))))),
                            )
                            .width(Length::Fill),
                            self.fx_origin(false, UiMsg::Noop),
                        ]
                        .align_y(Alignment::Center),
                    ),
                );
                continue;
            };
            let base = a.clone();
            let mut line = row![toggler(a.enabled).on_toggle({
                let base = base.clone();
                move |v| {
                    wrap(Msg::Anim(
                        name.into(),
                        Some(Anim {
                            enabled: v,
                            ..base.clone()
                        }),
                    ))
                }
            })]
            .spacing(12)
            .align_y(Alignment::Center);
            // Seconde ligne : courbe et style.
            let mut second = row![].spacing(10);
            if a.enabled {
                line = line.push(
                    slider(1.0f32..=20.0, a.speed as f32, move |x| {
                        wrap(Msg::Speed(name.into(), x as f64))
                    })
                    .step(0.5f32)
                    .on_release(wrap(Msg::Commit))
                    .width(Length::Fill),
                );
                line = line.push(
                    text(format!("{}", a.speed))
                        .font(self.font)
                        .size(13)
                        .width(30),
                );
                let sel = curves.iter().find(|c| c.value == a.curve).cloned();
                second = second.push(
                    pick_list(curves.clone(), sel, {
                        let base = base.clone();
                        move |c: Choice<String>| {
                            wrap(Msg::Anim(
                                name.into(),
                                Some(Anim {
                                    curve: c.value,
                                    ..base.clone()
                                }),
                            ))
                        }
                    })
                    .font(self.font)
                    .width(Length::FillPortion(1)),
                );
                if !l.styles.is_empty() {
                    let styles: Vec<Choice<String>> = l
                        .styles
                        .iter()
                        .map(|s| Choice {
                            value: s.to_string(),
                            label: crate::i18n::tr(fx::style_label(s)).to_string(),
                        })
                        .collect();
                    let sel = styles
                        .iter()
                        .find(|c| Some(&c.value) == a.style.as_ref())
                        .cloned();
                    second = second.push(
                        pick_list(styles, sel, {
                            let base = base.clone();
                            move |c: Choice<String>| {
                                wrap(Msg::Anim(
                                    name.into(),
                                    Some(Anim {
                                        style: Some(c.value),
                                        ..base.clone()
                                    }),
                                ))
                            }
                        })
                        .font(self.font)
                        .width(Length::FillPortion(1)),
                    );
                }
            }
            line = line.push(self.fx_origin(true, wrap(Msg::Anim(name.into(), None))));
            col = col.push(self.field(l.label, "", column![line, second].spacing(8)));
        }
        col = col.push(
            text(crate::i18n::tr(
                "Autres éléments (fondus détaillés, calques, zoom, écran branché…) : \
                 hd.config({ fx = { animations = { … } } }) dans hyprdesk.lua.",
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        );
        col.into()
    }

    pub(in crate::ui) fn animations_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Opt(path, v) => {
                self.edit_fx(|f| match v {
                    Some(v) => {
                        f.options.insert(path, v);
                    }
                    None => {
                        f.options.remove(&path);
                    }
                });
                self.commit()
            }
            Msg::Drag(path, x) => {
                let x = (x * 100.0).round() / 100.0;
                self.edit_fx(|f| {
                    f.options.insert(path, OptValue::Num(x));
                });
                self.apply()
            }
            Msg::Anim(leaf, a) => {
                self.edit_fx(|f| match a {
                    Some(a) => {
                        f.anims.insert(leaf, a);
                    }
                    None => {
                        f.anims.remove(&leaf);
                    }
                });
                self.commit()
            }
            Msg::Speed(leaf, x) => {
                self.edit_fx(|f| {
                    if let Some(a) = f.anims.get_mut(&leaf) {
                        a.speed = (x * 2.0).round() / 2.0;
                    }
                });
                self.apply()
            }
            Msg::Commit => self.commit(),
            Msg::Current(m) => {
                if let Some(p) = self.panel.as_mut() {
                    p.animations.current = m;
                }
                Task::none()
            }
        }
    }
}
