//! Paquet intégré « Barre » : Barre native : position, hauteur, écrans, modules par zone.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{
    Space, button, column, container, pick_list, radio, row, slider, toggler, tooltip,
};
use iced::{Alignment, Border, Color, Element, Length, Task};

use crate::config::{BAR_COLOR_SLOTS, BarColors, Rgba};
use crate::ui::color_picker::{Picker, PickerMsg};

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
/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("general", "Général"),
    ("modules", "Modules"),
    ("colors", "Couleurs"),
];

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
    /// Couleur d'un emplacement (`text`, `background`…) ou d'un module
    /// (`mod:clock`) ; `None` : celle du thème.
    SetColor(String, Option<Rgba>),
    /// Ouvre le sélecteur de couleur sous la ligne `clé` (le referme si déjà ouvert).
    OpenPicker(String),
    Picker(PickerMsg),
    PickerApply,
    PickerCancel,
    /// Transparence du fond (glissé), `None` : celle du thème.
    Opacity(Option<f32>),
}

/// État d'écran : quelles zones sont éditées.
#[derive(Default)]
pub struct State {
    pub portrait: bool,
    /// Sélecteur de couleur ouvert (onglet Couleurs) : la clé qu'il règle.
    pub picker: Option<(String, Picker)>,
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
        let dim = self.ink().dim;
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
                .map(|(_, l)| crate::i18n::tr(l).to_string())
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
                label: crate::i18n::tr(l).to_string(),
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
        if self.on_subtab(INFO.id, "colors") {
            // Modules présents dans la barre (toutes zones), sans doublon.
            let mut modules: Vec<(String, String)> = Vec::new();
            for e in b.zones.iter().chain(b.portrait.iter().flatten()).flatten() {
                if !modules.iter().any(|(k, _)| *k == e.type_id) {
                    modules.push((e.type_id.clone(), label_of(&e.type_id)));
                }
            }
            return self.bar_colors_page(&b, modules);
        }
        let general = self.on_subtab(INFO.id, "general");
        let mut col = if general {
            column![
                self.section("Barre de hyprdesk"),
                text("Au-dessus des fenêtres, avec sa zone réservée : les fenêtres et la grille du bureau se rangent à côté. Ses modules sont des widgets.").font(self.font).size(12).color(dim),
                self.field("Afficher la barre", "bar", toggler(b.enabled).on_toggle(|v| wrap(Msg::Enable(v)))),
            ]
        } else {
            column![self.section("Modules de la barre")]
        }
        .spacing(12);
        if self.cfg.keys_user.contains("bar") {
            col = col.push(text("hd.bar est aussi déclaré dans hyprdesk.lua, qui l'emporte : ces réglages sont sans effet.").font(self.font).size(12).color(warn));
        }
        if !b.enabled {
            col = col.push(
                text(if general {
                    crate::i18n::tr("Pensez ensuite à masquer la barre de wayle (sa configuration) pour éviter deux barres.")
                } else {
                    crate::i18n::tr("La barre est masquée : affichez-la dans l'onglet Général.")
                })
                .font(self.font)
                .size(12)
                .color(dim),
            );
            return col.into();
        }
        let small = |label: &'static str, msg: UiMsg| {
            button(text(label).font(self.font).size(12))
                .padding([2, 8])
                .on_press(msg)
        };
        if general {
            col = col
                .push(
                    self.field(
                        "Position",
                        "",
                        row![
                            radio(
                                crate::i18n::tr("En haut"),
                                BarPosition::Top,
                                Some(b.position),
                                |v| { wrap(Msg::Pos(v)) }
                            )
                            .font(self.font),
                            radio(
                                crate::i18n::tr("En bas"),
                                BarPosition::Bottom,
                                Some(b.position),
                                |v| { wrap(Msg::Pos(v)) }
                            )
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
                        radio(crate::i18n::tr("Tous"), 0u8, Some(which), pick).font(self.font),
                        radio(crate::i18n::tr("Écran principal"), 1u8, Some(which), pick)
                            .font(self.font),
                        radio(
                            crate::tr!("Seulement {mon}", mon = mon),
                            2u8,
                            Some(which),
                            pick
                        )
                        .font(self.font),
                    ]
                    .spacing(16)
                }));
            use crate::config::BarCompact;
            let compact = |label: &'static str, v: BarCompact| {
                radio(crate::i18n::tr(label), v, Some(b.compact), |v| {
                    wrap(Msg::Compact(v))
                })
                .font(self.font)
            };
            col = col.push(
                self.field(
                    "Barre compacte (titres courts, heure seule, icônes sans libellé)",
                    "",
                    row![
                        compact(
                            crate::i18n::tr("Auto : écrans étroits et verticaux"),
                            BarCompact::Auto
                        ),
                        compact("Toujours", BarCompact::Always),
                        compact("Jamais", BarCompact::Never),
                    ]
                    .spacing(16),
                ),
            );
            return col.into();
        }
        let portrait = self.panel.as_ref().is_some_and(|p| p.bar.portrait);
        let mut which = row![
            radio(
                crate::i18n::tr("Écrans horizontaux"),
                false,
                Some(portrait),
                |v| wrap(Msg::EditPortrait(v))
            )
            .font(self.font),
            radio(
                crate::i18n::tr("Écrans verticaux"),
                true,
                Some(portrait),
                |v| wrap(Msg::EditPortrait(v))
            )
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
        col = col.push(self.adv(self.field("Modules", "", which)));
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
                .placeholder(crate::i18n::tr("Ajouter un module…"))
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
            Msg::SetColor(key, c) => {
                if let Some(p) = self.panel.as_mut() {
                    p.bar.picker = None;
                }
                self.set_bar_color(&key, c);
                self.commit_reload()
            }
            Msg::OpenPicker(key) => {
                let open = self
                    .panel
                    .as_ref()
                    .and_then(|p| p.bar.picker.as_ref())
                    .is_some_and(|(k, _)| *k == key);
                let initial = self.bar_color_shown(&key);
                if let Some(p) = self.panel.as_mut() {
                    p.bar.picker = (!open).then(|| (key, Picker::new(initial)));
                }
                Task::none()
            }
            Msg::Picker(m) => {
                if let Some((_, pk)) = self.panel.as_mut().and_then(|p| p.bar.picker.as_mut()) {
                    pk.update(m);
                }
                Task::none()
            }
            Msg::PickerCancel => {
                if let Some(p) = self.panel.as_mut() {
                    p.bar.picker = None;
                }
                Task::none()
            }
            Msg::PickerApply => {
                let Some((key, pk)) = self.panel.as_mut().and_then(|p| p.bar.picker.take()) else {
                    return Task::none();
                };
                self.set_bar_color(&key, Some(pk.color()));
                self.commit_reload()
            }
            Msg::Opacity(o) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .colors
                    .opacity = o;
                // Glissé : enregistré au relâché (Commit) ; « Thème » : tout de suite.
                if o.is_none() {
                    self.commit_reload()
                } else {
                    Task::none()
                }
            }
        }
    }

    /// Couleur que la barre emploie pour cette clé : la sienne, sinon celle du thème.
    fn bar_color_shown(&self, key: &str) -> Rgba {
        let b = self
            .settings
            .bar
            .clone()
            .unwrap_or_else(|| crate::settings::BarSettings::from_config(&self.cfg));
        let th = &self.cfg.general.theme;
        match key.strip_prefix("mod:") {
            Some(m) => b
                .colors
                .modules
                .get(m)
                .copied()
                .or(b.colors.text)
                .unwrap_or(th.fg),
            None => b.colors.slot(key).unwrap_or(match key {
                "background" => th.bg,
                "accent" => th.accent,
                "warning" => BarColors::WARNING,
                "critical" => BarColors::CRITICAL,
                _ => th.fg,
            }),
        }
    }

    fn set_bar_color(&mut self, key: &str, c: Option<Rgba>) {
        let colors = &mut self
            .settings
            .bar
            .get_or_insert_with(Default::default)
            .colors;
        match key.strip_prefix("mod:") {
            Some(m) => match c {
                Some(c) => {
                    colors.modules.insert(m.to_string(), c);
                }
                None => {
                    colors.modules.remove(m);
                }
            },
            None => colors.set_slot(key, c),
        }
    }

    /// Onglet Couleurs : couleurs de la barre et de chaque module, transparence.
    fn bar_colors_page(
        &self,
        b: &crate::settings::BarSettings,
        modules: Vec<(String, String)>,
    ) -> Element<'_, UiMsg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = self.ink().dim;
        let font = self.font;
        let st = self.panel.as_ref().map(|p| &p.bar);
        let edge = Color { a: 0.3, ..fg };
        let dot = move |c: Rgba, size: f32, selected: bool| {
            let c = color(c, 1.0);
            container(Space::new().width(size).height(size)).style(move |_| container::Style {
                background: Some(c.into()),
                border: Border::default()
                    .rounded(size / 2.0)
                    .width(if selected { 2.5 } else { 1.0 })
                    .color(if selected { fg } else { edge }),
                ..Default::default()
            })
        };
        // Couleurs proposées : celles du thème en place (pas de palette figée).
        let mut proposals: Vec<(Rgba, &'static str)> = Vec::new();
        for (c, l) in [
            (th.fg, "Texte du thème"),
            (th.bg, "Fond du thème"),
            (th.accent, "Accent du thème"),
            (self.cfg.general.background, "Couleur de fond du bureau"),
            (BarColors::WARNING, "Avertissement"),
            (BarColors::CRITICAL, "Critique"),
        ] {
            if !proposals.iter().any(|(p, _)| *p == c) {
                proposals.push((c, l));
            }
        }
        let open = st.and_then(|s| s.picker.as_ref());
        // Une ligne : couleurs du thème, couleur en place, sélecteur, retour au thème.
        let line = |key: String, value: Option<Rgba>, default: Rgba| -> Element<'_, UiMsg> {
            let shown = value.unwrap_or(default);
            let chips = row(proposals.iter().map(|&(c, l)| {
                let k = key.clone();
                tooltip(
                    button(dot(c, 20.0, value == Some(c)))
                        .padding(0)
                        .style(|_, _| button::Style::default())
                        .on_press(wrap(Msg::SetColor(k, Some(c)))),
                    container(
                        text(format!(
                            "{} — {}",
                            crate::i18n::tr(l),
                            crate::settings::hex(c)
                        ))
                        .font(font)
                        .size(12),
                    )
                    .padding([4, 8])
                    .style(container::rounded_box),
                    tooltip::Position::Top,
                )
                .delay(std::time::Duration::from_millis(400))
                .into()
            }))
            .spacing(6);
            let is_open = open.is_some_and(|(k, _)| *k == key);
            let reset: Element<'_, UiMsg> = if value.is_some() {
                button(text(crate::i18n::tr("Thème")).font(font).size(12))
                    .padding([3, 8])
                    .on_press(wrap(Msg::SetColor(key.clone(), None)))
                    .into()
            } else {
                text(crate::i18n::tr("thème"))
                    .font(font)
                    .size(12)
                    .color(dim)
                    .width(52)
                    .into()
            };
            let mut c = column![
                row![
                    chips,
                    Space::new().width(Length::Fill),
                    dot(shown, 22.0, false),
                    text(crate::settings::hex(shown))
                        .font(font)
                        .size(12)
                        .color(if value.is_some() { fg } else { dim })
                        .width(64),
                    button(
                        text(crate::i18n::tr(if is_open {
                            "Fermer"
                        } else {
                            "Choisir…"
                        }))
                        .font(font)
                        .size(12)
                    )
                    .padding([3, 8])
                    .style(if is_open {
                        button::primary
                    } else {
                        button::secondary
                    })
                    .on_press(wrap(Msg::OpenPicker(key.clone()))),
                    reset,
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            ]
            .spacing(8);
            if let Some((_, pk)) = open.filter(|_| is_open) {
                c = c.push(pk.view(
                    font,
                    fg,
                    |m| wrap(Msg::Picker(m)),
                    wrap(Msg::PickerApply),
                    wrap(Msg::PickerCancel),
                ));
            }
            c.into()
        };
        let pal = self.bar_palette();
        let defaults = [
            ("text", th.fg),
            ("background", th.bg),
            ("accent", th.accent),
            ("warning", BarColors::WARNING),
            ("critical", BarColors::CRITICAL),
        ];
        let mut col = column![
            self.section("Couleurs de la barre"),
            text(crate::i18n::tr(
                "Sans couleur choisie, la barre suit le thème (et en change avec lui). Les pastilles proposent les couleurs du thème en place ; « Choisir… » ouvre le sélecteur."
            ))
            .font(font)
            .size(12)
            .color(dim),
        ]
        .spacing(12);
        for (k, label) in BAR_COLOR_SLOTS {
            let d = defaults
                .iter()
                .find(|(d, _)| d == k)
                .map_or(th.fg, |(_, c)| *c);
            col = col.push(self.field(label, "", line(k.to_string(), b.colors.slot(k), d)));
        }
        // Transparence du fond.
        let theme_opacity = th.effect(crate::config::Place::Bar).opacity;
        let o = b.colors.opacity.unwrap_or(theme_opacity);
        col = col.push(
            self.field(
                "Opacité du fond",
                "",
                row![
                    slider(0..=100u32, (o * 100.0).round() as u32, |v| wrap(
                        Msg::Opacity(Some(v as f32 / 100.0))
                    ))
                    .on_release(wrap(Msg::Commit))
                    .width(Length::Fill),
                    text(format!("{} %", (o * 100.0).round() as i32))
                        .font(font)
                        .size(13)
                        .width(48),
                    if b.colors.opacity.is_some() {
                        Element::from(
                            button(text(crate::i18n::tr("Thème")).font(font).size(12))
                                .padding([3, 8])
                                .on_press(wrap(Msg::Opacity(None))),
                        )
                    } else {
                        text(crate::i18n::tr("thème"))
                            .font(font)
                            .size(12)
                            .color(dim)
                            .width(52)
                            .into()
                    },
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ),
        );
        col = col.push(
            text(crate::i18n::tr(
                "0 % : barre transparente (seuls les modules restent) ; 100 % : opaque. Le flou d'arrière-plan se règle dans Thème → Effets.",
            ))
            .font(font)
            .size(12)
            .color(dim),
        );
        // Couleur de chaque module.
        col = col.push(self.section("Couleur de chaque module"));
        col = col.push(
            text(crate::i18n::tr(
                "Texte et icônes du module, y compris quand il est actif ; avertissement et critique gardent leurs couleurs.",
            ))
            .font(font)
            .size(12)
            .color(dim),
        );
        if modules.is_empty() {
            col = col.push(
                text(crate::i18n::tr("Aucun module dans la barre."))
                    .font(font)
                    .size(12)
                    .color(dim),
            );
        }
        for (k, label) in modules {
            let v = b.colors.modules.get(&k).copied();
            col = col.push(
                column![
                    text(label).font(self.title_font()).size(13),
                    line(format!("mod:{k}"), v, pal.fg),
                ]
                .spacing(6),
            );
        }
        col.into()
    }
}
