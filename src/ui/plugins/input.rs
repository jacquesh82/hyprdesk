//! Paquet intégré « Clavier et souris » : dispositions et répétition du
//! clavier, vitesse et accélération du pointeur, pavé tactile. Les valeurs
//! sont lues dans Hyprland, appliquées à chaud, puis enregistrées dans
//! `inputs.lua`, que hyprdesk tient (`crate::inputs`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use crate::inputs::Inputs;
use iced::widget::{column, pick_list, radio, text_input, toggler};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::{PanelMsg, SliderLook, SliderSpec};
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "input",
    label: "Clavier et souris",
    icon: "input-keyboard",
    summary: "Dispositions et répétition du clavier, vitesse de la souris, pavé tactile, \
              curseur, gestes et réglages par périphérique.",
    builtin: true,
};

pub const TABS: &[super::SubTab] = &[
    ("clavier", "Clavier"),
    ("souris", "Souris"),
    ("pave", "Pavé tactile"),
    ("curseur", "Curseur"),
    ("gestes", "Gestes"),
    ("peripheriques", "Périphériques"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Input(m)))
}

/// Valeur du curseur « Vitesse de répétition », en frappes par seconde.
fn en_par_seconde(v: f64) -> String {
    crate::tr!("{n} / s", n = v.round() as i64)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub loaded: bool,
    pub cur: Option<Inputs>,
    pub layouts: Vec<(String, String)>,
    pub variants: Vec<(String, String, String)>,
    pub touchpad: bool,
    /// Périphériques branchés : (nom, sorte), pour l'onglet Périphériques.
    pub devices: Vec<(String, String)>,
    /// Saisies en cours (dispositions, options xkb), validées par Entrée.
    pub layout_draft: Option<String>,
    pub options_draft: Option<String>,
    /// Zone d'essai du clavier (répétition, disposition).
    pub try_text: String,
    pub error: Option<String>,
}

/// Choix d'une liste : (code, libellé affiché).
#[derive(Debug, Clone, PartialEq)]
pub struct Choice(pub String, pub String);

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(Box<State>),
    /// Réglage changé : appliqué à chaud et enregistré.
    Set(Box<Inputs>),
    /// Curseur déplacé : affiché ; appliqué au relâchement.
    Preview(Box<Inputs>),
    Commit,
    Saved(Result<(), String>),
    LayoutDraft(String),
    LayoutSubmit,
    AddLayout(Choice),
    OptionsDraft(String),
    OptionsSubmit,
    Try(String),
    /// Accès clavier complet (`general.keyboard_access`, HIG ENT-01).
    KeyboardAccess(bool),
}

pub fn load() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                let (layouts, variants) = crate::inputs::layouts();
                State {
                    loaded: true,
                    cur: Some(crate::inputs::read()),
                    layouts,
                    variants,
                    touchpad: crate::inputs::has_touchpad(),
                    devices: crate::inputs::devices(),
                    ..Default::default()
                }
            })
            .await
            .unwrap_or_default()
        },
        |s| wrap(Msg::Loaded(Box::new(s))),
    )
}

/// Interrupteur : une copie des réglages, `f` y pose la valeur, puis Set.
fn toggle(
    cur: &Inputs,
    f: impl Fn(&mut Inputs, bool) + 'static,
) -> impl Fn(bool) -> UiMsg + 'static {
    let c = cur.clone();
    move |v| {
        let mut n = c.clone();
        f(&mut n, v);
        wrap(Msg::Set(Box::new(n)))
    }
}

/// Applique à chaud puis enregistre `inputs.lua`, hors du fil de l'interface.
pub(in crate::ui) fn save(config_path: std::path::PathBuf, i: Inputs) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                crate::inputs::apply(&i)?;
                crate::inputs::save(&crate::inputs::inputs_path(&config_path), &i)
                    .map_err(|e| e.to_string())
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()))
        },
        |r| wrap(Msg::Saved(r)),
    )
}

impl App {
    pub(in crate::ui) fn input_page(&self) -> Element<'_, UiMsg> {
        // Onglets tenus par le registre `hyprset` (MLG-142).
        if self.on_subtab(INFO.id, "curseur") {
            return column![self.cursor_theme_editor(), self.hyprset_folded("curseur")]
                .spacing(14)
                .into();
        }
        if self.on_subtab(INFO.id, "gestes") {
            return column![self.gestures_page(), self.hyprset_folded("gestes")]
                .spacing(14)
                .into();
        }
        if self.on_subtab(INFO.id, "peripheriques") {
            return self.devices_page();
        }
        let dim = self.ink().dim;
        // Onglet Clavier : l'accès clavier complet est un réglage de hyprdesk,
        // montré même quand Hyprland ne répond pas (encore).
        let waiting = |note: &'static str| -> Element<'_, UiMsg> {
            let note = text(note)
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .color(dim);
            if self.on_subtab(INFO.id, "clavier") {
                column![self.keyboard_access_field(), note]
                    .spacing(12)
                    .into()
            } else {
                note.into()
            }
        };
        let Some(s) = self.panel.as_ref().map(|p| &p.input).filter(|s| s.loaded) else {
            return waiting("Lecture…");
        };
        let Some(cur) = s.cur.clone() else {
            return waiting("Hyprland ne répond pas.");
        };
        let mut col = column![].spacing(12);
        if self.on_subtab(INFO.id, "souris") {
            col = col.push(self.section("Pointeur"));
            // force_no_accel (Avancé) l'emporte sur le profil (MLG-140).
            if self.cfg.general.hypr.options.get("input.force_no_accel")
                == Some(&crate::tiling::OptValue::Bool(true))
            {
                col = col.push(
                    text(
                        "« Sans accélération » (configuration avancée, plus bas) est actif : le \
                         profil d'accélération ci-dessous n'a pas d'effet.",
                    )
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(crate::ui::tone::warning()),
                );
            }
            let c = cur.clone();
            col = col.push(self.field(
                "Vitesse",
                "",
                self.slider(
                    &SliderSpec::new(
                        "input_sensitivity",
                        (-1.0, 1.0),
                        0.05,
                        move |v| {
                            let mut n = c.clone();
                            n.sensitivity = v;
                            wrap(Msg::Preview(Box::new(n)))
                        },
                        wrap(Msg::Commit),
                    ),
                    cur.sensitivity,
                    |v| format!("{:+.2}", v),
                    SliderLook::fixed(13.0, 260, Alignment::Center),
                ),
            ));
            let accel = |label: &'static str, v: &'static str| {
                let c = cur.clone();
                self.focusable(
                    radio(
                        crate::i18n::tr(label),
                        v,
                        Some(cur.accel_profile.as_str()),
                        move |v| {
                            let mut n = c.clone();
                            n.accel_profile = v.to_string();
                            wrap(Msg::Set(Box::new(n)))
                        },
                    )
                    .font(self.font),
                )
            };
            col = col.push(
                self.field(
                    "Accélération",
                    "",
                    column![
                        accel("Celle du périphérique", ""),
                        accel("Adaptative (plus vite quand on va vite)", "adaptive"),
                        accel("Aucune (déplacement constant, jeux, dessin)", "flat"),
                    ]
                    .spacing(6),
                ),
            );
            col = col.push(
                self.field(
                    "Défilement naturel (comme sur un téléphone)",
                    "",
                    self.focusable(
                        toggler(cur.natural_scroll)
                            .on_toggle(toggle(&cur, |i, v| i.natural_scroll = v)),
                    ),
                ),
            );
            col = col.push(self.field(
                "Gaucher (boutons inversés)",
                "",
                self.focusable(
                    toggler(cur.left_handed).on_toggle(toggle(&cur, |i, v| i.left_handed = v)),
                ),
            ));
            let focus = [
                Choice("0".into(), crate::i18n::tr("Au clic").into()),
                Choice("1".into(), crate::i18n::tr("Suit la souris").into()),
                Choice(
                    "2".into(),
                    crate::i18n::tr("Clavier au clic, survol sans focus").into(),
                ),
                Choice("3".into(), crate::i18n::tr("Totalement séparés").into()),
            ];
            let current = focus
                .iter()
                .find(|c| c.0 == cur.follow_mouse.to_string())
                .cloned();
            col = col.push(
                self.field(
                    "Fenêtre active",
                    "",
                    self.choosable(
                        pick_list(focus, current, {
                            let c0 = cur.clone();
                            move |c: Choice| {
                                let mut n = c0.clone();
                                n.follow_mouse = c.0.parse().unwrap_or(1);
                                wrap(Msg::Set(Box::new(n)))
                            }
                        })
                        .font(self.font)
                        .width(320),
                    ),
                ),
            );
        } else if self.on_subtab(INFO.id, "pave") {
            col = col.push(self.section("Pavé tactile"));
            if !s.touchpad {
                col = col.push(
                    text("Aucun pavé tactile détecté : ces réglages s'appliqueront quand il y en aura un.")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim),
                );
            }
            col = col.push(
                self.field(
                    "Toucher pour cliquer",
                    "",
                    self.focusable(
                        toggler(cur.tp_tap_to_click)
                            .on_toggle(toggle(&cur, |i, v| i.tp_tap_to_click = v)),
                    ),
                ),
            );
            col = col.push(
                self.field(
                    "Défilement naturel",
                    "",
                    self.focusable(
                        toggler(cur.tp_natural_scroll)
                            .on_toggle(toggle(&cur, |i, v| i.tp_natural_scroll = v)),
                    ),
                ),
            );
            col = col.push(
                self.field(
                    "Désactiver pendant la frappe",
                    "",
                    self.focusable(
                        toggler(cur.tp_disable_while_typing)
                            .on_toggle(toggle(&cur, |i, v| i.tp_disable_while_typing = v)),
                    ),
                ),
            );
            col = col.push(
                self.field(
                    "Clic droit à deux doigts",
                    "",
                    self.focusable(
                        toggler(cur.tp_clickfinger)
                            .on_toggle(toggle(&cur, |i, v| i.tp_clickfinger = v)),
                    ),
                ),
            );
            let c = cur.clone();
            col = col.push(self.field(
                "Vitesse de défilement",
                "",
                self.slider(
                    &SliderSpec::new(
                        "input_tp_scroll_factor",
                        (0.1, 3.0),
                        0.1,
                        move |v| {
                            let mut n = c.clone();
                            n.tp_scroll_factor = v;
                            wrap(Msg::Preview(Box::new(n)))
                        },
                        wrap(Msg::Commit),
                    ),
                    cur.tp_scroll_factor,
                    |v| format!("× {:.1}", v),
                    SliderLook::fixed(13.0, 260, Alignment::Center),
                ),
            ));
        } else {
            // Clavier.
            col = col.push(self.keyboard_access_field());
            col = col.push(self.section("Disposition"));
            let first = cur
                .kb_layout
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            let layouts: Vec<Choice> = s
                .layouts
                .iter()
                .map(|(c, d)| Choice(c.clone(), format!("{d} ({c})")))
                .collect();
            col = col.push(
                self.field(
                    "Dispositions (la première par défaut)",
                    "",
                    text_input("fr,us", s.layout_draft.as_deref().unwrap_or(&cur.kb_layout))
                        .font(self.font)
                        .on_input(|v| wrap(Msg::LayoutDraft(v)))
                        .on_submit(wrap(Msg::LayoutSubmit))
                        .padding(8)
                        .width(320),
                ),
            );
            col = col.push(
                pick_list(layouts, None::<Choice>, |c| wrap(Msg::AddLayout(c)))
                    .placeholder(crate::i18n::tr("Ajouter une disposition…"))
                    .font(self.font)
                    .width(320),
            );
            let mut variants: Vec<Choice> =
                vec![Choice(String::new(), crate::i18n::tr("Standard").into())];
            variants.extend(
                s.variants
                    .iter()
                    .filter(|(l, _, _)| *l == first)
                    .map(|(_, c, d)| Choice(c.clone(), format!("{d} ({c})"))),
            );
            let current = variants.iter().find(|c| c.0 == cur.kb_variant).cloned();
            col = col.push(
                self.field(
                    "Variante",
                    "",
                    self.choosable(
                        pick_list(variants, current, {
                            let c0 = cur.clone();
                            move |c: Choice| {
                                let mut n = c0.clone();
                                n.kb_variant = c.0;
                                wrap(Msg::Set(Box::new(n)))
                            }
                        })
                        .font(self.font)
                        .width(320),
                    ),
                ),
            );
            col = col.push(
                self.adv(
                    "Options xkb (grp:alt_shift_toggle, caps:escape…)",
                    self.field(
                        "",
                        "",
                        text_input("", s.options_draft.as_deref().unwrap_or(&cur.kb_options))
                            .font(self.font)
                            .on_input(|v| wrap(Msg::OptionsDraft(v)))
                            .on_submit(wrap(Msg::OptionsSubmit))
                            .padding(8)
                            .width(320),
                    ),
                ),
            );
            col = col.push(self.section("Répétition des touches"));
            let c1 = cur.clone();
            col = col.push(self.field(
                "Délai avant répétition",
                "",
                self.slider(
                    &SliderSpec::new(
                        "input_repeat_delay",
                        (150.0, 1000.0),
                        10.0,
                        move |v| {
                            let mut n = c1.clone();
                            n.repeat_delay = (v / 10.0).round() as i64 * 10;
                            wrap(Msg::Preview(Box::new(n)))
                        },
                        wrap(Msg::Commit),
                    ),
                    cur.repeat_delay as f64,
                    |v| format!("{} ms", v as i64),
                    SliderLook::fixed(13.0, 260, Alignment::Center),
                ),
            ));
            let c2 = cur.clone();
            col = col.push(self.field(
                "Vitesse de répétition",
                "",
                self.slider(
                    &SliderSpec::new(
                        "input_repeat_rate",
                        (5.0, 80.0),
                        1.0,
                        move |v| {
                            let mut n = c2.clone();
                            n.repeat_rate = v.round() as i64;
                            wrap(Msg::Preview(Box::new(n)))
                        },
                        wrap(Msg::Commit),
                    ),
                    cur.repeat_rate as f64,
                    en_par_seconde,
                    SliderLook::fixed(13.0, 260, Alignment::Center),
                ),
            ));
            col = col.push(self.field(
                "Verr. Num. actif au démarrage",
                "",
                self.focusable(toggler(cur.numlock).on_toggle(toggle(&cur, |i, v| i.numlock = v))),
            ));
            col = col.push(
                text_input(crate::i18n::tr("Essayer ici…"), &s.try_text)
                    .font(self.font)
                    .on_input(|v| wrap(Msg::Try(v)))
                    .padding(8)
                    .width(Length::Fill),
            );
        }
        col = col.push(
            text("Appliqué aussitôt, et enregistré dans inputs.lua (tenu par hyprdesk).")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
        );
        if let Some(e) = &s.error {
            col = col.push(
                text(e.clone())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(crate::ui::tone::warning()),
            );
        }
        col.into()
    }

    /// Accès clavier complet (HIG ENT-01) : un réglage de hyprdesk, pas de
    /// Hyprland ; `keys.keyboard_access` le bascule aussi.
    fn keyboard_access_field(&self) -> Element<'_, UiMsg> {
        let combo = &self.cfg.general.keys.keyboard_access;
        let note = if combo.trim().is_empty() {
            crate::i18n::tr(
                "Tab atteint chaque commande des Réglages, Entrée ou Espace l'active. \
                 Coupé : Tab ne va qu'aux champs de saisie.",
            )
            .to_string()
        } else {
            crate::tr!(
                "Tab atteint chaque commande des Réglages, Entrée ou Espace l'active. \
                 Coupé : Tab ne va qu'aux champs de saisie. {k} le bascule partout.",
                k = combo
            )
        };
        column![
            self.section("Navigation au clavier"),
            self.field(
                "Accès clavier complet",
                "keyboard_access",
                self.focusable(
                    toggler(self.cfg.general.keyboard_access)
                        .on_toggle(|v| wrap(Msg::KeyboardAccess(v))),
                ),
            ),
            text(note)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(self.ink().dim),
        ]
        .spacing(12)
        .into()
    }

    pub(in crate::ui) fn input_update(&mut self, m: Msg) -> Task<UiMsg> {
        if let Msg::KeyboardAccess(v) = m {
            self.settings.keyboard_access = Some(v);
            return self.commit();
        }
        let path = self.config_path.clone();
        let Some(p) = self.panel.as_mut() else {
            return Task::none();
        };
        let s = &mut p.input;
        match m {
            Msg::Loaded(n) => {
                *s = *n;
                Task::none()
            }
            Msg::Set(i) => {
                s.cur = Some((*i).clone());
                s.error = None;
                save(path, *i)
            }
            Msg::Preview(i) => {
                s.cur = Some(*i);
                Task::none()
            }
            Msg::Commit => match s.cur.clone() {
                Some(i) => save(path, i),
                None => Task::none(),
            },
            Msg::Saved(r) => {
                s.error = r.err().map(|e| crate::tr!("enregistrement : {e}", e = e));
                Task::none()
            }
            Msg::LayoutDraft(v) => {
                s.layout_draft = Some(v);
                Task::none()
            }
            Msg::LayoutSubmit => {
                let (Some(v), Some(mut i)) = (s.layout_draft.take(), s.cur.clone()) else {
                    return Task::none();
                };
                let v: Vec<String> = v
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect();
                if v.is_empty() {
                    return Task::none();
                }
                i.kb_layout = v.join(",");
                s.cur = Some(i.clone());
                save(path, i)
            }
            Msg::AddLayout(c) => {
                let Some(mut i) = s.cur.clone() else {
                    return Task::none();
                };
                if !i.kb_layout.split(',').any(|l| l.trim() == c.0) {
                    i.kb_layout = format!("{},{}", i.kb_layout.trim_end_matches(','), c.0);
                    s.cur = Some(i.clone());
                    s.layout_draft = None;
                    return save(path, i);
                }
                Task::none()
            }
            Msg::OptionsDraft(v) => {
                s.options_draft = Some(v);
                Task::none()
            }
            Msg::OptionsSubmit => {
                let (Some(v), Some(mut i)) = (s.options_draft.take(), s.cur.clone()) else {
                    return Task::none();
                };
                i.kb_options = v.trim().to_string();
                s.cur = Some(i.clone());
                save(path, i)
            }
            Msg::Try(v) => {
                s.try_text = v;
                Task::none()
            }
            Msg::KeyboardAccess(_) => Task::none(),
        }
    }
}
