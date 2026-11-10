//! Paquet intégré « Barre » : Barre native : position, hauteur, écrans, modules par zone.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use iced::widget::{button, column, pick_list, radio, row, slider, text, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "bar",
    label: "Barre",
    icon: "computer",
    summary: "Barre native : position, hauteur, écrans, modules par zone.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Bar(m)))
}

#[derive(Debug, Clone)]
pub enum Msg {
    Enable(bool),
    Pos(crate::config::BarPosition),
    Height(u16),
    AllMonitors(bool),
    /// Barre seulement sur l'écran principal.
    Primary,
    /// Ajoute un module à une zone (0 gauche, 1 centre, 2 droite).
    Add(usize, BarChoice),
    /// Déplace un module dans sa zone (-1 avant, +1 après).
    Move(usize, usize, i32),
    /// Change un module de zone (-1 vers la gauche, +1 vers la droite).
    Zone(usize, usize, i32),
    Remove(usize, usize),
    /// Fin d'un glissé de curseur de la barre : enregistrer puis recharger.
    Commit,
    /// Barre compacte : auto (écrans étroits ou verticaux), toujours, jamais.
    Compact(crate::config::BarCompact),
    /// Éditer les modules des écrans verticaux (zones 3..=5) plutôt que l'ordinaire.
    EditPortrait(bool),
    /// Écrans verticaux : revenir aux mêmes modules que les autres.
    PortraitReset,
}

/// État d'écran : quelles zones sont éditées.
#[derive(Default)]
pub struct State {
    pub portrait: bool,
}

/// Choix « ajouter un module » de la barre.
#[derive(Debug, Clone, PartialEq)]
pub struct BarChoice {
    pub type_id: String,
    pub label: String,
}

impl std::fmt::Display for BarChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

impl App {
    pub(in crate::ui) fn tab_bar(&self) -> Element<'_, UiMsg> {
        use crate::config::BarPosition;
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let (mon, _) = self.panel_ctx().unwrap_or_default();
        let b = self
            .settings
            .bar
            .clone()
            .unwrap_or_else(|| crate::settings::BarSettings::from_config(&self.cfg));
        let label_of = |type_id: &str| -> String {
            crate::config::BAR_ITEMS
                .iter()
                .find(|(id, _)| *id == type_id)
                .map(|(_, l)| l.to_string())
                .or_else(|| {
                    self.cfg
                        .catalog
                        .iter()
                        .find(|t| t.id == type_id)
                        .map(|t| t.name.clone())
                })
                .unwrap_or_else(|| type_id.to_string())
        };
        let choices: Vec<BarChoice> = crate::config::BAR_ITEMS
            .iter()
            .map(|(id, l)| BarChoice {
                type_id: id.to_string(),
                label: l.to_string(),
            })
            .chain(
                self.cfg
                    .catalog
                    .iter()
                    .filter(|t| t.toggleable && t.error.is_none())
                    .map(|t| BarChoice {
                        type_id: t.id.clone(),
                        label: format!("{} (widget)", t.name),
                    }),
            )
            .collect();
        let mut col = column![
            self.section("Barre de hyprdesk"),
            text("Au-dessus des fenêtres, avec sa zone réservée : les fenêtres et la grille du bureau se rangent à côté. Ses modules sont des widgets.").font(self.font).size(12).color(dim),
            self.field("Afficher la barre", "bar", toggler(b.enabled).on_toggle(|v| wrap(Msg::Enable(v)))),
        ]
        .spacing(12);
        if self.cfg.keys_user.contains("bar") {
            col = col.push(text("hd.bar est aussi déclaré dans hyprdesk.lua, qui l'emporte : ces réglages sont sans effet.").font(self.font).size(12).color(warn));
        }
        if !b.enabled {
            col = col.push(text("Pensez ensuite à masquer la barre de wayle (sa configuration) pour éviter deux barres.").font(self.font).size(12).color(dim));
            return col.into();
        }
        col = col
            .push(
                self.field(
                    "Position",
                    "",
                    row![
                        radio("En haut", BarPosition::Top, Some(b.position), |v| {
                            wrap(Msg::Pos(v))
                        })
                        .font(self.font),
                        radio("En bas", BarPosition::Bottom, Some(b.position), |v| {
                            wrap(Msg::Pos(v))
                        })
                        .font(self.font),
                    ]
                    .spacing(16),
                ),
            )
            .push(
                self.field(
                    "Hauteur",
                    "",
                    row![
                        slider(24..=64u16, b.height.clamp(24, 64), |v| wrap(Msg::Height(v)))
                            .on_release(wrap(Msg::Commit))
                            .width(260),
                        text(format!("{} px", b.height)).font(self.font).size(13),
                    ]
                    .spacing(12),
                ),
            )
            .push(self.field("Écrans", "", {
                // 0 : tous ; 1 : l'écran principal ; 2 : cet écran.
                let which = match &b.monitors {
                    None => 0u8,
                    Some(l) if l.len() == 1 && l[0] == super::super::bar::PRIMARY => 1,
                    Some(_) => 2,
                };
                let pick = |v: u8| {
                    wrap(match v {
                        0 => Msg::AllMonitors(true),
                        1 => Msg::Primary,
                        _ => Msg::AllMonitors(false),
                    })
                };
                row![
                    radio("Tous", 0u8, Some(which), pick).font(self.font),
                    radio("Écran principal", 1u8, Some(which), pick).font(self.font),
                    radio(format!("Seulement {mon}"), 2u8, Some(which), pick).font(self.font),
                ]
                .spacing(16)
            }));
        let small = |label: &'static str, msg: UiMsg| {
            button(text(label).font(self.font).size(12))
                .padding([2, 8])
                .on_press(msg)
        };
        use crate::config::BarCompact;
        let compact = |label: &'static str, v: BarCompact| {
            radio(label, v, Some(b.compact), |v| wrap(Msg::Compact(v))).font(self.font)
        };
        col = col.push(
            self.field(
                "Barre compacte (titres courts, heure seule, icônes sans libellé)",
                "",
                row![
                    compact("Auto : écrans étroits et verticaux", BarCompact::Auto),
                    compact("Toujours", BarCompact::Always),
                    compact("Jamais", BarCompact::Never),
                ]
                .spacing(16),
            ),
        );
        let portrait = self.panel.as_ref().is_some_and(|p| p.bar.portrait);
        let mut which = row![
            radio("Écrans horizontaux", false, Some(portrait), |v| wrap(
                Msg::EditPortrait(v)
            ))
            .font(self.font),
            radio("Écrans verticaux", true, Some(portrait), |v| wrap(
                Msg::EditPortrait(v)
            ))
            .font(self.font),
        ]
        .spacing(16)
        .align_y(Alignment::Center);
        if portrait && b.portrait.is_some() {
            which = which.push(
                button(
                    text("Mêmes modules que les autres écrans")
                        .font(self.font)
                        .size(12),
                )
                .padding([3, 10])
                .style(button::secondary)
                .on_press(wrap(Msg::PortraitReset)),
            );
        }
        col = col.push(self.field("Modules", "", which));
        if portrait && b.portrait.is_none() {
            col = col.push(
                text(
                    "Les écrans verticaux reprennent les modules des autres écrans ; \
                     un changement ici leur donne leurs propres modules.",
                )
                .font(self.font)
                .size(12)
                .color(dim),
            );
        }
        let (zones, off) = match (&b.portrait, portrait) {
            (Some(p), true) => (p, 3),
            (None, true) => (&b.zones, 3),
            _ => (&b.zones, 0),
        };
        for (zi, zname) in ["Gauche", "Centre", "Droite"].iter().enumerate() {
            let z = zi + off;
            col = col.push(
                text(*zname)
                    .font(self.font)
                    .size(15)
                    .color(color(g.theme.accent, 1.0)),
            );
            if zones[zi].is_empty() {
                col = col.push(text("vide").font(self.font).size(12).color(dim));
            }
            for (i, e) in zones[zi].iter().enumerate() {
                let mut r = row![
                    text(label_of(&e.type_id))
                        .font(self.font)
                        .size(14)
                        .width(Length::Fill)
                ]
                .spacing(4)
                .align_y(Alignment::Center);
                if i > 0 {
                    r = r.push(small("↑", wrap(Msg::Move(z, i, -1))));
                }
                if i + 1 < zones[zi].len() {
                    r = r.push(small("↓", wrap(Msg::Move(z, i, 1))));
                }
                if zi > 0 {
                    r = r.push(small("←", wrap(Msg::Zone(z, i, -1))));
                }
                if zi < 2 {
                    r = r.push(small("→", wrap(Msg::Zone(z, i, 1))));
                }
                r = r.push(small("×", wrap(Msg::Remove(z, i))));
                col = col.push(r);
            }
            col = col.push(
                pick_list(choices.clone(), None::<BarChoice>, move |c| {
                    wrap(Msg::Add(z, c))
                })
                .font(self.font)
                .placeholder("Ajouter un module…")
                .width(260),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn bar_update(&mut self, msg: Msg) -> Task<UiMsg> {
        if self.settings.bar.is_none() && !matches!(msg, Msg::Commit) {
            self.settings.bar = Some(crate::settings::BarSettings::from_config(&self.cfg));
        }
        match msg {
            Msg::Enable(on) => {
                let b = self
                    .settings
                    .bar
                    .get_or_insert_with(crate::settings::BarSettings::default);
                b.enabled = on;
                self.commit_reload()
            }
            Msg::Pos(p) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .position = p;
                self.commit_reload()
            }
            Msg::Height(h) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .height = h;
                Task::none()
            }
            Msg::AllMonitors(all) => {
                let mon = self.panel_ctx().map(|c| c.0).unwrap_or_default();
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .monitors = if all { None } else { Some(vec![mon]) };
                self.commit_reload()
            }
            Msg::Primary => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .monitors = Some(vec![super::super::bar::PRIMARY.to_string()]);
                self.commit_reload()
            }
            Msg::Add(z, choice) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .zone_mut(z)
                    .push(crate::settings::BarEntry::new(&choice.type_id));
                self.commit_reload()
            }
            Msg::Move(z, i, d) => {
                let zone = self
                    .settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .zone_mut(z);
                let j = i as i32 + d;
                if i < zone.len() && j >= 0 && (j as usize) < zone.len() {
                    zone.swap(i, j as usize);
                }
                self.commit_reload()
            }
            Msg::Zone(z, i, d) => {
                let b = self.settings.bar.get_or_insert_with(Default::default);
                // Même jeu de zones : ordinaire (0..=2) ou écrans verticaux (3..=5).
                let (base, local) = (z / 3 * 3, (z % 3) as i32);
                let to = local + d;
                if (0..3).contains(&to) && i < b.zone_mut(z).len() {
                    let e = b.zone_mut(z).remove(i);
                    b.zone_mut(base + to as usize).push(e);
                }
                self.commit_reload()
            }
            Msg::Remove(z, i) => {
                let zone = self
                    .settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .zone_mut(z);
                if i < zone.len() {
                    zone.remove(i);
                }
                self.commit_reload()
            }
            Msg::Commit => self.commit_reload(),
            Msg::Compact(c) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .compact = c;
                self.commit_reload()
            }
            Msg::EditPortrait(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.bar.portrait = v;
                }
                Task::none()
            }
            Msg::PortraitReset => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .portrait = None;
                self.commit_reload()
            }
        }
    }
}
