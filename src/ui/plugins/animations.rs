//! Paquet intégré « Animations » : animations et effets de Hyprland —
//! animation par élément (durée, courbe, style), arrondis, opacité, flou et
//! ses variantes, ombre, lueur, flou de mouvement, ondulation.
//! Domaine : `crate::fx`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::BTreeMap;

use crate::i18n::text;
use iced::widget::{button, column, container, pick_list, row};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::{PanelMsg, SliderLook, SliderSpec, beside_slider};
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};
use crate::config::Config;
use crate::fx::{self, Anim, Fx};
use crate::tiling::OptValue;

pub const INFO: PluginInfo = PluginInfo {
    id: "animations",
    label: "Animations",
    icon: "preferences-desktop-animation",
    summary: "Animations et effets de Hyprland : fenêtres, workspaces, fondus, flou et ses \
              variantes, ombre, lueur, ondulation ; transitions du bureau de hyprdesk.",
    builtin: true,
};

/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("animations", "Animations"),
    ("courbes", "Courbes"),
    ("effets", "Effets"),
    ("bureau", "Bureau"),
];

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
    /// Réglage d'un élément animé (`None` : celui de hyprland.lua).
    Anim(String, Option<Anim>),
    /// Durée d'un élément en cours de glissé.
    Speed(String, f64),
    Commit,
    Current(BTreeMap<String, String>),
    /// Fait défiler les valeurs d'un effet à l'écran (MLG-262).
    Compare(&'static str),
}

/// Réglages effectifs : ceux de la config, ou rien si le paquet est coupé
/// (ainsi que Performance, qui écrit dans les mêmes réglages) ; le mouvement
/// réduit coupe en plus glissements et agrandissements (HIG FON-08).
pub fn effective(cfg: &Config) -> Fx {
    let fx = if cfg.plugin_on(INFO.id) || cfg.plugin_on(super::performance::INFO.id) {
        cfg.general.fx.clone()
    } else {
        Fx::default()
    };
    fx::with_reduced_motion(fx, cfg.general.reduced_motion)
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

    /// Le flou de Hyprland est coupé : par un réglage d'ici (profil
    /// Performance…), sinon par hyprland.lua (lu à l'ouverture des Réglages).
    /// Sans flou, une surface translucide devient illisible.
    pub(in crate::ui) fn hypr_blur_off(&self) -> bool {
        match effective(&self.cfg).options.get("decoration.blur.enabled") {
            Some(OptValue::Bool(on)) => !on,
            _ => self
                .panel
                .as_ref()
                .and_then(|p| p.animations.current.get("decoration.blur.enabled"))
                .is_some_and(|v| matches!(v.as_str(), "false" | "0")),
        }
    }

    pub(in crate::ui) fn edit_fx(&mut self, f: impl FnOnce(&mut Fx)) {
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
            "courbes" => self.curves_page(),
            // Transitions de hyprdesk lui-même (ancien paquet Effets).
            "bureau" => self.tab_effects(),
            _ => self.fx_animations(),
        }
    }

    /// « réglé ici » + Rétablir, ou l'origine hyprland.lua.
    fn fx_origin(&self, ours: bool, reset: UiMsg) -> Element<'_, UiMsg> {
        if ours {
            row![
                text(crate::i18n::tr("réglé ici"))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(self.ink().accent),
                self.focusable(
                    button(
                        text(crate::i18n::tr("Rétablir"))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                    )
                    .padding([2, 8])
                    .style(button::text)
                    .on_press(reset)
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        } else {
            text(crate::i18n::tr("hyprland.lua"))
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(self.ink().dim)
                .into()
        }
    }

    /// Contrôle d'un effet selon sa nature (rendu unique, `ui::hyprset`).
    pub(in crate::ui) fn fx_option(&self, path: &'static str) -> Element<'_, UiMsg> {
        self.opt_field(crate::ui::hyprset::Store::Fx, path)
    }

    /// Comparer les variantes de flou à l'œil : Hyprland n'a pas de
    /// réfraction et une capture ne montre pas le flou (MLG-262).
    fn blur_compare(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        row![
            self.focusable(button(
                text(crate::i18n::tr("Comparer à l'écran"))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
            )
            .padding([4, 10])
            .on_press(wrap(Msg::Compare("decoration.blur.variant")))),
            text(crate::i18n::tr(
                "Chaque variante quatre secondes, son nom en haut de l'écran, puis la vôtre revient. \
                 Gardez une fenêtre transparente devant un fond détaillé."
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim)
            .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    }

    fn fx_effects(&self) -> Element<'_, UiMsg> {
        let mut col = column![
            text(crate::i18n::tr(
                "Effets dessinés par Hyprland autour et derrière les fenêtres. Seuls les \
                 réglages faits ici sont posés ; le reste vient de hyprland.lua. Les variantes \
                 de flou animées sollicitent davantage la carte graphique."
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(self.ink().dim),
        ]
        .spacing(14);
        for (title, keys) in fx::GROUPS {
            col = col.push(self.section(title));
            let mut fine = column![].spacing(14);
            for k in *keys {
                if fx::ADVANCED.contains(k) {
                    fine = fine.push(self.fx_option(k));
                    if *k == "decoration.blur.variant" {
                        fine = fine.push(self.blur_compare());
                    }
                } else {
                    col = col.push(self.fx_option(k));
                }
            }
            // Réglages fins du groupe : configuration avancée.
            if keys.iter().any(|k| fx::ADVANCED.contains(k)) {
                col = col.push(self.adv(
                    &crate::tr!("Réglages fins : {t}", t = crate::i18n::tr(title)),
                    || fine,
                ));
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
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(14);
        // Mouvement réduit : il l'emporte sur les réglages ci-dessous (HIG FON-08).
        if self.cfg.general.reduced_motion {
            col = col.push(
                text(crate::i18n::tr(
                    "Mouvement réduit est activé (Thème → Accessibilité) : fenêtres et \
                     calques ne glissent ni ne grossissent plus, les workspaces changent \
                     en fondu, quels que soient les réglages ci-dessous.",
                ))
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            );
        }
        // Catalogue de hyprdesk, puis courbes de l'utilisateur (MLG-143).
        let curves: Vec<Choice<String>> = fx::CURVES
            .iter()
            .map(|c| Choice {
                value: c.name.to_string(),
                label: crate::i18n::tr(c.label).to_string(),
            })
            .chain(self.cfg.general.fx.curves.keys().map(|n| Choice {
                value: n.clone(),
                label: crate::tr!("{n} (la vôtre)", n = n),
            }))
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
                                self.focusable(
                                    button(
                                        text(crate::i18n::tr("Personnaliser"))
                                            .font(self.font)
                                            .size(crate::ui::tokens::ts(12.0))
                                    )
                                    .padding([4, 10])
                                    .on_press(wrap(Msg::Anim(name.into(), Some(Anim::new_for(l)))))
                                ),
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
            let speed = SliderSpec::new(
                format!("anim_speed.{name}"),
                (1.0, 20.0),
                0.5,
                move |x| wrap(Msg::Speed(name.into(), x)),
                wrap(Msg::Commit),
            );
            let interrupteur = self.focusable(self.toggle(a.enabled).on_toggle({
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
            }));
            // Avec son curseur, l'interrupteur reste sur la ligne de celui-ci,
            // au-dessus de son échelle (HIG SEL-09).
            let mut line = if a.enabled {
                row![beside_slider(&speed, interrupteur)]
            } else {
                row![interrupteur]
            }
            .spacing(12)
            .align_y(Alignment::Center);
            // Seconde ligne : courbe et style.
            let mut second = row![].spacing(10);
            if a.enabled {
                line = line.push(self.slider(
                    &speed,
                    a.speed,
                    |x| format!("{x}"),
                    SliderLook::fill(13.0, 30, Alignment::Center),
                ));
                let sel = curves.iter().find(|c| c.value == a.curve).cloned();
                second = second.push(
                    self.choosable(
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
                    ),
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
                        self.choosable(
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
                        ),
                    );
                }
            }
            let origine = self.fx_origin(true, wrap(Msg::Anim(name.into(), None)));
            line = line.push(if a.enabled {
                beside_slider(&speed, origine)
            } else {
                origine
            });
            col = col.push(self.field(l.label, "", column![line, second].spacing(8)));
        }
        col = col.push(
            text(crate::i18n::tr(
                "Autres éléments (fondus détaillés, calques, zoom, écran branché…) : \
                 hd.config({ fx = { animations = { … } } }) dans hyprdesk.lua.",
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        );
        col.into()
    }

    pub(in crate::ui) fn animations_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
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
            Msg::Compare(path) => {
                let exe = std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "hyprdesk".into());
                let cmd = format!(
                    "{} fx compare {path} all --seconds 4",
                    crate::launch::sh_quote(&exe)
                );
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || crate::hypr::exec(&cmd)).await;
                    },
                    |_| UiMsg::Noop,
                )
            }
            Msg::Current(m) => {
                if let Some(p) = self.panel.as_mut() {
                    p.animations.current = m;
                }
                Task::none()
            }
        }
    }
}
