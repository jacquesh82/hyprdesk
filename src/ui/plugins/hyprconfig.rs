//! Paquet intégré « Configuration de Hyprland » : ménage de hyprland.lua,
//! incohérences et conflits (`crate::hyprcheck`), avec la correction de
//! chacun, « Tout ranger » pour les corrections sûres, et LYRA pour le reste.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::hyprcheck::{Finding, Fix};
use crate::i18n::text;
use iced::widget::{button, column, container, row};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "hyprconfig",
    label: "Configuration de Hyprland",
    icon: "preferences-system",
    summary: "Ménage de hyprland.lua : erreurs, doublons, programmes absents ou faits par hyprdesk, vieilles copies.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::HyprConfig(m)))
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub loaded: bool,
    pub findings: Vec<Finding>,
    /// Compte rendu de la dernière correction.
    pub info: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(Vec<Finding>),
    Rescan,
    /// Corrige un point (son rang dans le relevé).
    Fix(usize),
    /// Toutes les corrections sûres.
    FixAll,
    Done(Result<usize, String>),
}

/// Relevé hors du fil de l'interface.
pub fn load(app: &App) -> Task<UiMsg> {
    let path = app.config_path.clone();
    let cfg = &app.cfg;
    let managed = crate::hyprcheck::managed(
        cfg.plugin_on("wallpaper"),
        cfg.plugin_on("bar") && cfg.bar.is_some(),
        app.toasts.active,
        cfg.general.idle.manage,
        cfg.general.clipboard_history,
    );
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || crate::hyprcheck::full(&path, &managed))
                .await
                .unwrap_or_default()
        },
        |f| wrap(Msg::Loaded(f)),
    )
}

fn apply(app: &App, fixes: Vec<Fix>) -> Task<UiMsg> {
    let path = app.config_path.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || crate::hyprcheck::apply(&path, &fixes))
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
        },
        |r| wrap(Msg::Done(r)),
    )
}

impl App {
    pub(in crate::ui) fn hyprconfig_page(&self) -> Element<'_, UiMsg> {
        let ink = self.ink();
        let (fg, dim, accent) = (ink.fg, ink.dim, ink.accent);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let Some(s) = self.panel.as_ref().map(|p| &p.hyprconfig).filter(|s| s.loaded) else {
            return text("Analyse de hyprland.lua…").font(self.font).size(13).color(dim).into();
        };
        let safe = s.findings.iter().filter(|f| f.safe && f.fix.is_some()).count();
        let mut head = row![
            button(text("Analyser à nouveau").font(self.font).size(13))
                .padding([6, 12])
                .style(button::secondary)
                .on_press(wrap(Msg::Rescan)),
            button(
                iced::widget::text(crate::tr!("Tout ranger ({n})", n = safe))
                    .font(self.font)
                    .size(13)
            )
            .padding([6, 12])
            .on_press_maybe((safe > 0).then(|| wrap(Msg::FixAll))),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if !s.findings.is_empty() {
            head = head.push(iced::widget::Space::new().width(Length::Fill));
            head = head.push(self.lyra_button(
                crate::i18n::tr("Configuration de Hyprland"),
                crate::hyprcheck::report(&s.findings),
            ));
        }
        let mut col = column![
            text("hyprland.lua est relu : ce qui se contredit, fait double emploi ou ne sert plus. « Tout ranger » applique les corrections sûres (commenter, supprimer) ; hyprland.lua est copié avant, et une ligne modifiée entre-temps n'est pas touchée.")
                .font(self.font)
                .size(12)
                .color(dim),
            head,
        ]
        .spacing(12);
        if let Some(m) = &s.info {
            col = col.push(iced::widget::text(m.clone()).font(self.font).size(12).color(accent));
        }
        if let Some(e) = &s.error {
            col = col.push(iced::widget::text(e.clone()).font(self.font).size(12).color(warn));
        }
        if s.findings.is_empty() {
            col = col.push(
                text("Rien à signaler : hyprland.lua est en ordre.")
                    .font(self.font)
                    .size(13)
                    .color(accent),
            );
            return col.into();
        }
        let mut last = None;
        for (i, f) in s.findings.iter().enumerate() {
            if last != Some(f.kind) {
                col = col.push(self.section(crate::i18n::tr(f.kind.label())));
                last = Some(f.kind);
            }
            let mut info = column![
                iced::widget::text(match f.line {
                    Some(l) => format!("{l} · {}", f.excerpt),
                    None => f.excerpt.clone(),
                })
                .font(self.font)
                .size(13)
                .color(fg)
            ]
            .spacing(2)
            .width(Length::Fill);
            if !f.detail.is_empty() {
                info = info.push(iced::widget::text(f.detail.clone()).font(self.font).size(12).color(dim));
            }
            let mut line = row![info].spacing(10).align_y(Alignment::Center);
            if let Some(fix) = &f.fix {
                let label = match fix {
                    Fix::Comment(..) => "Commenter",
                    Fix::Delete(..) => "Supprimer la ligne",
                    Fix::DeleteFiles(..) => "Supprimer les copies",
                };
                line = line.push(
                    button(text(label).font(self.font).size(12))
                        .padding([4, 10])
                        .on_press(wrap(Msg::Fix(i))),
                );
            }
            col = col.push(
                container(line)
                    .padding([8, 10])
                    .style(move |_| container::Style {
                        border: Border::default()
                            .rounded(10)
                            .width(1)
                            .color(Color { a: 0.12, ..fg }),
                        ..Default::default()
                    }),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn hyprconfig_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Loaded(f) => {
                if let Some(p) = self.panel.as_mut() {
                    p.hyprconfig.findings = f;
                    p.hyprconfig.loaded = true;
                }
                Task::none()
            }
            Msg::Rescan => {
                if let Some(p) = self.panel.as_mut() {
                    p.hyprconfig.info = None;
                    p.hyprconfig.error = None;
                }
                load(self)
            }
            Msg::Fix(i) => {
                let fix = self
                    .panel
                    .as_ref()
                    .and_then(|p| p.hyprconfig.findings.get(i))
                    .and_then(|f| f.fix.clone());
                match fix {
                    Some(f) => apply(self, vec![f]),
                    None => Task::none(),
                }
            }
            Msg::FixAll => {
                let fixes: Vec<Fix> = self
                    .panel
                    .as_ref()
                    .map(|p| {
                        p.hyprconfig
                            .findings
                            .iter()
                            .filter(|f| f.safe)
                            .filter_map(|f| f.fix.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                apply(self, fixes)
            }
            Msg::Done(r) => {
                if let Some(p) = self.panel.as_mut() {
                    match r {
                        Ok(n) => {
                            p.hyprconfig.error = None;
                            p.hyprconfig.info = Some(crate::tr!(
                                "{n} correction(s) faite(s) ; copie : hyprland.lua.avant-hyprdesk-menage.bak",
                                n = n
                            ));
                        }
                        Err(e) => {
                            p.hyprconfig.info = None;
                            p.hyprconfig.error = Some(e);
                        }
                    }
                }
                load(self)
            }
        }
    }
}
