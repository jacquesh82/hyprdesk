//! Paquet intégré « Configuration de Hyprland » : ménage de hyprland.lua,
//! incohérences et conflits (`crate::hyprcheck`), avec la correction de
//! chacun, « Tout ranger » pour les corrections sûres, et LYRA pour le reste ;
//! onglet « Historique » : instantanés de la configuration sur 30 jours et
//! retour arrière (`crate::history`, MLG-138).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::history::{Entry, FileDiff, Restored};
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
    summary: "Ménage de hyprland.lua : erreurs, doublons, programmes absents ou faits par hyprdesk, vieilles copies ; historique de la configuration sur 30 jours, avec retour arrière.",
    builtin: true,
};

/// Onglets : le ménage de hyprland.lua, puis l'historique de la configuration.
pub const TABS: &[super::SubTab] = &[("menage", "Ménage"), ("historique", "Historique")];

/// Lignes d'extrait du diff affichées sous un instantané déplié.
const EXCERPT_LINES: usize = 40;

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
    /// Historique : instantanés, du plus récent au plus ancien (`None` : en lecture).
    pub history: Option<Vec<Entry>>,
    /// Instantané déplié, et son écart avec l'état actuel.
    pub open: Option<String>,
    pub diff: Option<(String, Result<Vec<FileDiff>, String>)>,
    /// Instantané dont le retour arrière attend confirmation.
    pub armed: Option<String>,
    /// Retour arrière en cours.
    pub busy: bool,
    /// Compte rendu du dernier retour arrière.
    pub history_info: Option<String>,
    pub history_error: Option<String>,
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
    HistoryLoaded(Vec<Entry>),
    HistoryRefresh,
    /// Déplie ou replie un instantané.
    Toggle(String),
    DiffLoaded(String, Result<Vec<FileDiff>, String>),
    /// Arme, puis confirme, le retour à un instantané.
    Rollback(String),
    RolledBack(Result<Restored, String>),
}

/// Historique lu hors du fil de l'interface.
pub fn load_history(app: &App) -> Task<UiMsg> {
    let path = app.config_path.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || crate::history::entries(&path))
                .await
                .unwrap_or_default()
        },
        |e| wrap(Msg::HistoryLoaded(e)),
    )
}

fn load_diff(app: &App, id: String) -> Task<UiMsg> {
    let path = app.config_path.clone();
    Task::perform(
        async move {
            let q = id.clone();
            let r = tokio::task::spawn_blocking(move || {
                crate::history::diff(&path, &q).map(|(_, d)| d)
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
            (id, r)
        },
        |(id, r)| wrap(Msg::DiffLoaded(id, r)),
    )
}

fn rollback(app: &App, id: String) -> Task<UiMsg> {
    let path = app.config_path.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || crate::history::rollback(&path, &id))
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
        },
        |r| wrap(Msg::RolledBack(r)),
    )
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
    let session = crate::hyprcheck::Session {
        monitors: app.monitors.iter().map(|m| m.name.clone()).collect(),
        workspaces: cfg.plugin_on("workspaces"),
    };
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || crate::hyprcheck::full(&path, &managed, &session))
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
        if self.on_subtab(INFO.id, "historique") {
            return self.hyprconfig_history();
        }
        let ink = self.ink();
        let (fg, dim, accent) = (ink.fg, ink.dim, ink.accent);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let Some(s) = self
            .panel
            .as_ref()
            .map(|p| &p.hyprconfig)
            .filter(|s| s.loaded)
        else {
            return text("Analyse de hyprland.lua…")
                .font(self.font)
                .size(13)
                .color(dim)
                .into();
        };
        let safe = s
            .findings
            .iter()
            .filter(|f| f.safe && f.fix.is_some())
            .count();
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
            col = col.push(
                iced::widget::text(m.clone())
                    .font(self.font)
                    .size(12)
                    .color(accent),
            );
        }
        if let Some(e) = &s.error {
            col = col.push(
                iced::widget::text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(warn),
            );
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
                info = info.push(
                    iced::widget::text(f.detail.clone())
                        .font(self.font)
                        .size(12)
                        .color(dim),
                );
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
            col = col.push(container(line).padding([8, 10]).style(move |_| {
                container::Style {
                    border: Border::default()
                        .rounded(10)
                        .width(1)
                        .color(Color { a: 0.12, ..fg }),
                    ..Default::default()
                }
            }));
        }
        col.into()
    }

    /// Onglet « Historique » : instantanés groupés par jour, extrait du diff
    /// d'un instantané déplié, retour arrière confirmé en deux temps.
    fn hyprconfig_history(&self) -> Element<'_, UiMsg> {
        let ink = self.ink();
        let (fg, dim, accent) = (ink.fg, ink.dim, ink.accent);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let font = self.font;
        let Some(s) = self.panel.as_ref().map(|p| &p.hyprconfig) else {
            return iced::widget::Space::new().into();
        };
        let mut col = column![
            text("Avant chaque modification de la configuration (Réglages, ménage, raccourcis, démarrage, import, ligne de commande), et après chaque modification à la main, hyprdesk garde une copie des fichiers de ~/.config/hypr pendant 30 jours. « Revenir à cette version » remet ces fichiers, recharge Hyprland et annule tout si Hyprland signale une erreur nouvelle ; l'état d'avant reste dans l'historique.")
                .font(font)
                .size(12)
                .color(dim),
            row![
                button(text("Actualiser").font(font).size(13))
                    .padding([6, 12])
                    .style(button::secondary)
                    .on_press(wrap(Msg::HistoryRefresh)),
            ],
        ]
        .spacing(12);
        if let Some(m) = &s.history_info {
            col = col.push(
                iced::widget::text(m.clone())
                    .font(font)
                    .size(12)
                    .color(accent),
            );
        }
        if let Some(e) = &s.history_error {
            col = col.push(
                iced::widget::text(e.clone())
                    .font(font)
                    .size(12)
                    .color(warn),
            );
        }
        let Some(list) = &s.history else {
            return col
                .push(
                    text("Lecture de l'historique…")
                        .font(font)
                        .size(13)
                        .color(dim),
                )
                .into();
        };
        if list.is_empty() {
            return col
                .push(
                    text("Aucun instantané pour l'instant : le premier sera pris à la prochaine modification de la configuration.")
                        .font(font)
                        .size(13)
                        .color(dim),
                )
                .into();
        }
        let today = chrono::Local::now().date_naive();
        let mut day = None;
        for e in list {
            let id = e.snap.id.clone();
            let t = e.snap.time();
            let d = t.date_naive();
            if day != Some(d) {
                day = Some(d);
                let label = if d == today {
                    crate::i18n::tr("Aujourd'hui").to_string()
                } else if today.pred_opt() == Some(d) {
                    crate::i18n::tr("Hier").to_string()
                } else {
                    t.format_localized("%A %-d %B %Y", crate::i18n::chrono_locale())
                        .to_string()
                };
                col = col.push(
                    iced::widget::text(label)
                        .font(self.title_font())
                        .size(15)
                        .color(accent),
                );
            }
            let open = s.open.as_deref() == Some(id.as_str());
            let armed = s.armed.as_deref() == Some(id.as_str());
            let changed = if e.changed.is_empty() {
                crate::i18n::tr("aucun").to_string()
            } else {
                e.changed.join(", ")
            };
            let info = column![
                row![
                    iced::widget::text(t.format("%H:%M:%S").to_string())
                        .font(font)
                        .size(13)
                        .color(dim),
                    iced::widget::text(crate::history::reason_label(&e.snap.reason))
                        .font(font)
                        .size(13)
                        .color(fg),
                ]
                .spacing(10),
                iced::widget::text(crate::tr!("Fichiers changés ensuite : {f}", f = changed))
                    .font(font)
                    .size(12)
                    .color(dim),
            ]
            .spacing(2);
            let toggle = button(
                row![
                    iced::widget::text(if open { "▾" } else { "▸" })
                        .font(font)
                        .size(13)
                        .color(dim),
                    info,
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding(0)
            .width(Length::Fill)
            .style(button::text)
            .on_press(wrap(Msg::Toggle(id.clone())));
            let back = button(
                iced::widget::text(if armed {
                    crate::i18n::tr("Confirmer le retour ?")
                } else {
                    crate::i18n::tr("Revenir à cette version")
                })
                .font(font)
                .size(12),
            )
            .padding([4, 10])
            .style(move |th, st| {
                if armed {
                    button::Style {
                        background: Some(Color { a: 0.85, ..warn }.into()),
                        text_color: Color::BLACK,
                        border: Border::default().rounded(999),
                        ..Default::default()
                    }
                } else {
                    button::secondary(th, st)
                }
            })
            .on_press_maybe((!s.busy).then(|| wrap(Msg::Rollback(id.clone()))));
            let mut card =
                column![row![toggle, back].spacing(10).align_y(Alignment::Center)].spacing(8);
            if open {
                card = card.push(self.history_excerpt(s, &id));
            }
            col = col.push(container(card).padding([8, 10]).style(move |_| {
                container::Style {
                    border: Border::default()
                        .rounded(10)
                        .width(1)
                        .color(Color { a: 0.12, ..fg }),
                    ..Default::default()
                }
            }));
        }
        col.into()
    }

    /// Écart entre un instantané déplié et l'état actuel : un extrait du diff.
    fn history_excerpt<'a>(&self, s: &'a State, id: &str) -> Element<'a, UiMsg> {
        let ink = self.ink();
        let (fg, dim) = (ink.fg, ink.dim);
        let font = self.font;
        let Some((_, r)) = s.diff.as_ref().filter(|(d, _)| d == id) else {
            return text("Lecture…").font(font).size(12).color(dim).into();
        };
        let diffs = match r {
            Ok(d) => d,
            Err(e) => {
                return iced::widget::text(e.clone())
                    .font(font)
                    .size(12)
                    .color(dim)
                    .into();
            }
        };
        if diffs.is_empty() {
            return text("Identique à l'état actuel.")
                .font(font)
                .size(12)
                .color(dim)
                .into();
        }
        let mut col = column![
            text("Écart avec l'état actuel (− cette version, + maintenant) :")
                .font(font)
                .size(12)
                .color(dim)
        ]
        .spacing(4);
        let mut left = EXCERPT_LINES;
        let mut hidden = 0;
        for d in diffs {
            col = col.push(
                iced::widget::text(format!("{} — {}", d.file, d.change.label()))
                    .font(font)
                    .size(12)
                    .color(fg),
            );
            // En-têtes « --- / +++ » retirés : le nom du fichier suffit.
            let lines: Vec<&str> = d.patch.lines().skip(2).collect();
            let shown = lines.len().min(left);
            hidden += lines.len() - shown;
            left -= shown;
            if shown > 0 {
                col = col.push(
                    iced::widget::text(lines[..shown].join("\n"))
                        .font(iced::Font::MONOSPACE)
                        .size(11)
                        .color(fg),
                );
            }
        }
        if hidden > 0 {
            col = col.push(
                iced::widget::text(crate::tr!(
                    "… {n} ligne(s) de plus : hyprdesk config diff {id}",
                    n = hidden,
                    id = id
                ))
                .font(font)
                .size(12)
                .color(dim),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn hyprconfig_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::HistoryLoaded(list) => {
                if let Some(p) = self.panel.as_mut() {
                    let s = &mut p.hyprconfig;
                    let known = |id: &Option<String>| {
                        id.as_ref()
                            .is_some_and(|i| list.iter().any(|e| &e.snap.id == i))
                    };
                    if !known(&s.armed) {
                        s.armed = None;
                    }
                    if !known(&s.open) {
                        s.open = None;
                    }
                    s.history = Some(list);
                }
                // Écart de l'instantané déplié : relu (l'état actuel a pu changer).
                match self.panel.as_ref().and_then(|p| p.hyprconfig.open.clone()) {
                    Some(id) => load_diff(self, id),
                    None => Task::none(),
                }
            }
            Msg::HistoryRefresh => {
                if let Some(p) = self.panel.as_mut() {
                    p.hyprconfig.history_info = None;
                    p.hyprconfig.history_error = None;
                    p.hyprconfig.armed = None;
                }
                load_history(self)
            }
            Msg::Toggle(id) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                let s = &mut p.hyprconfig;
                s.armed = None;
                if s.open.as_deref() == Some(id.as_str()) {
                    s.open = None;
                    Task::none()
                } else {
                    s.open = Some(id.clone());
                    s.diff = None;
                    load_diff(self, id)
                }
            }
            Msg::DiffLoaded(id, r) => {
                if let Some(p) = self.panel.as_mut()
                    && p.hyprconfig.open.as_deref() == Some(id.as_str())
                {
                    p.hyprconfig.diff = Some((id, r));
                }
                Task::none()
            }
            Msg::Rollback(id) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                let s = &mut p.hyprconfig;
                if s.busy {
                    return Task::none();
                }
                // Premier appui : armé ; second appui sur le même : confirmé.
                if s.armed.as_deref() != Some(id.as_str()) {
                    s.armed = Some(id);
                    return Task::none();
                }
                s.armed = None;
                s.busy = true;
                s.history_info = None;
                s.history_error = None;
                rollback(self, id)
            }
            Msg::RolledBack(r) => {
                if let Some(p) = self.panel.as_mut() {
                    let s = &mut p.hyprconfig;
                    s.busy = false;
                    match r {
                        Ok(r) => {
                            s.history_error = None;
                            s.history_info = Some(crate::tr!(
                                "Retour à {id} fait : {n} fichier(s) restauré(s), {m} supprimé(s). L'état d'avant est gardé : {safety}.",
                                id = r.id,
                                n = r.plan.restore.len(),
                                m = r.plan.remove.len(),
                                safety = r.safety
                            ));
                        }
                        Err(e) => {
                            s.history_info = None;
                            s.history_error = Some(e);
                        }
                    }
                }
                // Config rechargée (la veille le ferait aussi), historique et
                // ménage relus.
                Task::batch([
                    Task::done(UiMsg::ConfigChanged),
                    load_history(self),
                    load(self),
                ])
            }
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
