//! Paquet intégré « Clavier et souris » : dispositions et répétition du
//! clavier, vitesse et accélération du pointeur, pavé tactile. Les valeurs
//! sont lues dans Hyprland, appliquées à chaud, puis enregistrées dans
//! `inputs.lua`, que hyprdesk tient (`crate::inputs`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use crate::inputs::Inputs;
use iced::widget::{column, pick_list, radio, row, slider, text_input, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "input",
    label: "Clavier et souris",
    icon: "input-keyboard",
    summary: "Dispositions et répétition du clavier, vitesse de la souris, pavé tactile.",
    builtin: true,
};

pub const TABS: &[super::SubTab] = &[
    ("clavier", "Clavier"),
    ("souris", "Souris"),
    ("pave", "Pavé tactile"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Input(m)))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub loaded: bool,
    pub cur: Option<Inputs>,
    pub layouts: Vec<(String, String)>,
    pub variants: Vec<(String, String, String)>,
    pub touchpad: bool,
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
fn save(config_path: std::path::PathBuf, i: Inputs) -> Task<UiMsg> {
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
        let dim = self.ink().dim;
        let Some(s) = self.panel.as_ref().map(|p| &p.input).filter(|s| s.loaded) else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let Some(cur) = s.cur.clone() else {
            return text("Hyprland ne répond pas.")
                .font(self.font)
                .size(13)
                .color(dim)
                .into();
        };
        let mut col = column![].spacing(12);
        if self.on_subtab(INFO.id, "souris") {
            col = col.push(self.section("Pointeur"));
            let c = cur.clone();
            col = col.push(
                self.field(
                    "Vitesse",
                    "",
                    row![
                        slider(-1.0..=1.0, cur.sensitivity as f32, move |v| {
                            let mut n = c.clone();
                            n.sensitivity = v as f64;
                            wrap(Msg::Preview(Box::new(n)))
                        })
                        .step(0.05_f32)
                        .on_release(wrap(Msg::Commit))
                        .width(260),
                        text(format!("{:+.2}", cur.sensitivity))
                            .font(self.font)
                            .size(13),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
            );
            let accel = |label: &'static str, v: &'static str| {
                let c = cur.clone();
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
                .font(self.font)
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
            col = col.push(self.field(
                "Défilement naturel (comme sur un téléphone)",
                "",
                toggler(cur.natural_scroll).on_toggle(toggle(&cur, |i, v| i.natural_scroll = v)),
            ));
            col = col.push(self.field(
                "Gaucher (boutons inversés)",
                "",
                toggler(cur.left_handed).on_toggle(toggle(&cur, |i, v| i.left_handed = v)),
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
            );
        } else if self.on_subtab(INFO.id, "pave") {
            col = col.push(self.section("Pavé tactile"));
            if !s.touchpad {
                col = col.push(
                    text("Aucun pavé tactile détecté : ces réglages s'appliqueront quand il y en aura un.")
                        .font(self.font)
                        .size(12)
                        .color(dim),
                );
            }
            col = col.push(self.field(
                "Toucher pour cliquer",
                "",
                toggler(cur.tp_tap_to_click).on_toggle(toggle(&cur, |i, v| i.tp_tap_to_click = v)),
            ));
            col = col.push(
                self.field(
                    "Défilement naturel",
                    "",
                    toggler(cur.tp_natural_scroll)
                        .on_toggle(toggle(&cur, |i, v| i.tp_natural_scroll = v)),
                ),
            );
            col = col.push(
                self.field(
                    "Désactiver pendant la frappe",
                    "",
                    toggler(cur.tp_disable_while_typing)
                        .on_toggle(toggle(&cur, |i, v| i.tp_disable_while_typing = v)),
                ),
            );
            col = col.push(self.field(
                "Clic droit à deux doigts",
                "",
                toggler(cur.tp_clickfinger).on_toggle(toggle(&cur, |i, v| i.tp_clickfinger = v)),
            ));
            let c = cur.clone();
            col = col.push(
                self.field(
                    "Vitesse de défilement",
                    "",
                    row![
                        slider(0.1..=3.0, cur.tp_scroll_factor as f32, move |v| {
                            let mut n = c.clone();
                            n.tp_scroll_factor = v as f64;
                            wrap(Msg::Preview(Box::new(n)))
                        })
                        .step(0.1_f32)
                        .on_release(wrap(Msg::Commit))
                        .width(260),
                        text(format!("× {:.1}", cur.tp_scroll_factor))
                            .font(self.font)
                            .size(13),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
            );
        } else {
            // Clavier.
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
            );
            col = col.push(
                self.adv(
                    self.field(
                        "Options xkb (grp:alt_shift_toggle, caps:escape…)",
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
            col = col.push(
                self.field(
                    "Délai avant répétition",
                    "",
                    row![
                        slider(150.0..=1000.0, cur.repeat_delay as f32, move |v| {
                            let mut n = c1.clone();
                            n.repeat_delay = (v / 10.0).round() as i64 * 10;
                            wrap(Msg::Preview(Box::new(n)))
                        })
                        .step(10.0_f32)
                        .on_release(wrap(Msg::Commit))
                        .width(260),
                        text(format!("{} ms", cur.repeat_delay))
                            .font(self.font)
                            .size(13),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
            );
            let c2 = cur.clone();
            col = col.push(
                self.field(
                    "Vitesse de répétition",
                    "",
                    row![
                        slider(5.0..=80.0, cur.repeat_rate as f32, move |v| {
                            let mut n = c2.clone();
                            n.repeat_rate = v.round() as i64;
                            wrap(Msg::Preview(Box::new(n)))
                        })
                        .step(1.0_f32)
                        .on_release(wrap(Msg::Commit))
                        .width(260),
                        text(crate::tr!("{n} / s", n = cur.repeat_rate))
                            .font(self.font)
                            .size(13),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
            );
            col = col.push(self.field(
                "Verr. Num. actif au démarrage",
                "",
                toggler(cur.numlock).on_toggle(toggle(&cur, |i, v| i.numlock = v)),
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
                .size(12)
                .color(dim),
        );
        if let Some(e) = &s.error {
            col = col.push(
                text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn input_update(&mut self, m: Msg) -> Task<UiMsg> {
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
        }
    }
}
