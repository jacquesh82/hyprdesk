//! Paquet intégré « Performance » : ce que le rendu de Hyprland coûte au
//! processeur et à la carte graphique, option par option, et trois profils
//! de priorité (Performance, Équilibré, Rendu graphique) qui les posent d'un
//! coup. Domaine : `crate::perf` ; les réglages sont ceux de `crate::fx`
//! (partagés avec le paquet Animations).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, row};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::perf::{self, Cost, Profile};
use crate::tiling::{Kind, OptValue};

pub const INFO: PluginInfo = PluginInfo {
    id: "performance",
    label: "Performance",
    icon: "power-profile-performance",
    summary: "Effets de Hyprland qui sollicitent le processeur et la carte graphique, et \
              profils Performance, Équilibré, Rendu graphique.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Performance(m)))
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Pose un profil.
    Profile(Profile),
    /// Retire les réglages du paquet : tout revient à hyprland.lua.
    Clear,
    /// Ouvre Énergie (profil du processeur).
    Energy,
}

/// Valeur lisible d'un profil pour une option (`None` : hyprland.lua).
fn shown(path: &str, v: Option<&OptValue>) -> String {
    match v {
        None => "hyprland.lua".into(),
        Some(OptValue::Bool(b)) => crate::i18n::tr(if *b { "oui" } else { "non" }).into(),
        Some(OptValue::Num(n)) => match crate::fx::opt(path).map(|o| &o.kind) {
            Some(Kind::IntChoice(list)) => list
                .iter()
                .find(|(x, _)| *x as f64 == *n)
                .map(|(_, l)| crate::i18n::tr(l).to_string())
                .unwrap_or_else(|| n.to_string()),
            _ => OptValue::Num(*n).display(),
        },
        Some(OptValue::Str(s)) => s.clone(),
    }
}

/// Ce que coûte l'option, selon sa nature (interrupteur, curseur, choix).
fn cost_caption(path: &str, c: Cost) -> &'static str {
    match (crate::fx::opt(path).map(|o| &o.kind), c) {
        (Some(Kind::Num { .. }), _) => "Plus élevé, plus coûteux pour la carte graphique",
        (Some(Kind::IntChoice(_)), _) => "Hors « Jamais », économise en plein écran",
        (_, Cost::Gpu) => "Activé, sollicite la carte graphique",
        (_, Cost::Frames) => "Activé, fait calculer plus d'images",
        (_, Cost::Saves) => "Activé, économise",
    }
}

impl App {
    fn perf_card(&self, p: Profile, on: bool) -> Element<'_, UiMsg> {
        let ink = self.ink();
        let accent = color(self.cfg.general.theme.accent, 1.0);
        let fg = ink.fg;
        button(
            column![
                text(p.label())
                    .font(self.title_font())
                    .size(15)
                    .color(if on { ink.accent } else { fg }),
                text(p.summary()).font(self.font).size(12).color(ink.dim),
            ]
            .spacing(6),
        )
        .padding(12)
        .width(Length::FillPortion(1))
        .on_press(wrap(Msg::Profile(p)))
        .style(move |_, status| button::Style {
            background: Some(
                if on {
                    Color { a: 0.16, ..accent }
                } else if matches!(status, button::Status::Hovered) {
                    Color { a: 0.06, ..fg }
                } else {
                    Color::TRANSPARENT
                }
                .into(),
            ),
            text_color: fg,
            border: iced::Border::default()
                .rounded(12)
                .width(if on { 2 } else { 1 })
                .color(if on { accent } else { Color { a: 0.15, ..fg } }),
            ..Default::default()
        })
        .into()
    }

    pub(in crate::ui) fn performance_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let options = &self.cfg.general.fx.options;
        let active = perf::active(options);
        let mut col = column![
            text(
                "Chaque effet de Hyprland se paie à chaque image : le flou et l'ombre sur la \
                 carte graphique, les animations en images à calculer. Un profil pose tous \
                 les réglages ci-dessous d'un coup ; chacun se retouche ensuite à la main."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.section("Profil"),
            row(Profile::ALL.map(|p| self.perf_card(p, active == Some(p))))
                .spacing(10)
                .width(Length::Fill),
        ]
        .spacing(14);
        let status = match active {
            Some(p) => crate::tr!("Profil en place : {p}.", p = crate::i18n::tr(p.label())),
            None if perf::untouched(options) => {
                crate::i18n::tr("Aucun profil : tout vient de hyprland.lua.").to_string()
            }
            None => crate::i18n::tr("Réglages retouchés à la main (aucun profil).").to_string(),
        };
        let mut line = row![text(status).font(self.font).size(12).color(dim)]
            .spacing(12)
            .align_y(Alignment::Center);
        if !perf::untouched(options) {
            line = line.push(
                button(text("Revenir à hyprland.lua").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press(wrap(Msg::Clear)),
            );
        }
        col = col.push(line);
        if self.hypr_blur_off() {
            col = col.push(
                text("Sans flou, le tiroir des Réglages devient opaque pour rester lisible.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for (title, items) in perf::GROUPS {
            col = col.push(self.section(title));
            for it in *items {
                let profiles: Vec<String> = Profile::ALL
                    .iter()
                    .map(|p| {
                        crate::tr!(
                            "{p} : {v}",
                            p = crate::i18n::tr(p.label()),
                            v = shown(it.path, p.value(it.path).as_ref())
                        )
                    })
                    .collect();
                col = col.push(
                    column![
                        self.fx_option(it.path),
                        text(format!(
                            "{} · {}",
                            crate::i18n::tr(cost_caption(it.path, it.cost)),
                            profiles.join(" · ")
                        ))
                        .font(self.font)
                        .size(11)
                        .color(dim),
                    ]
                    .spacing(4),
                );
            }
        }
        col = col.push(self.section("Processeur"));
        col = col.push(
            row![
                text(
                    "La fréquence du processeur suit le profil d'énergie du système \
                     (power-profiles-daemon)."
                )
                .font(self.font)
                .size(12)
                .color(dim)
                .width(Length::Fill),
                button(text("Profil d'énergie…").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press(wrap(Msg::Energy)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
        col.into()
    }

    pub(in crate::ui) fn performance_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Profile(p) => {
                self.edit_fx(|f| p.apply(&mut f.options));
                self.commit()
            }
            Msg::Clear => {
                self.edit_fx(|f| perf::clear(&mut f.options));
                self.commit()
            }
            Msg::Energy => self.panel_update(PanelMsg::Tab("energy")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_lisibles() {
        assert_eq!(shown("debug.vfr", None), "hyprland.lua");
        assert_eq!(
            shown("decoration.blur.passes", Some(&OptValue::Num(3.0))),
            "3"
        );
        let missing = crate::i18n::untranslated(
            "performance",
            [
                ("debug.vfr", Cost::Gpu),
                ("debug.vfr", Cost::Frames),
                ("debug.vfr", Cost::Saves),
                ("decoration.blur.passes", Cost::Gpu),
                ("render.direct_scanout", Cost::Saves),
            ]
            .map(|(p, c)| cost_caption(p, c)),
        );
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }
}
