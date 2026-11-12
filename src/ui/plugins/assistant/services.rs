//! Onglet « Autres services IA » du paquet IA : les fournisseurs
//! (`providers.rs`), la fonction → fournisseur, et SmartUI design.
//! Domaine : `crate::ai_services`, `crate::smartui`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use crate::i18n::text;
use iced::widget::{Space, button, column, container, row, svg, text_editor};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::super::{App, Msg as UiMsg, color};
use super::{Msg, wrap};

/// Une application du lanceur et ses icônes.
#[derive(Debug, Clone)]
pub struct RowData {
    /// Clé du thème SmartUI (nom d'icône) : identifie la ligne.
    pub key: String,
    pub class: String,
    pub name: String,
    /// Icône du thème (envoyée au modèle).
    pub original: Option<PathBuf>,
    pub proposal: Option<Vec<u8>>,
    pub validated: Option<Vec<u8>>,
}

pub struct Row {
    pub key: String,
    pub class: String,
    pub name: String,
    pub original: Option<PathBuf>,
    pub proposal: Option<svg::Handle>,
    pub validated: Option<svg::Handle>,
}

impl From<RowData> for Row {
    fn from(d: RowData) -> Self {
        Row {
            key: d.key,
            class: d.class,
            name: d.name,
            original: d.original,
            proposal: d.proposal.map(svg::Handle::from_memory),
            validated: d.validated.map(svg::Handle::from_memory),
        }
    }
}

/// État de l'onglet (dans le tiroir).
#[derive(Default)]
pub struct State {
    /// Fournisseurs : fiche en cours de modification, clés.
    pub providers: super::providers::State,
    /// Style de SmartUI, sur plusieurs lignes ; `style_dirty` : pas encore enregistré.
    pub style: text_editor::Content,
    pub style_dirty: bool,
    pub rows: Vec<Row>,
    /// Génération en cours (une à la fois), et celles qui attendent.
    pub busy: Option<String>,
    pub queue: VecDeque<String>,
    pub errors: BTreeMap<String, String>,
    pub clear_armed: bool,
}

impl App {
    /// Le style enregistré dans l'éditeur (à l'arrivée sur le paquet).
    pub(in crate::ui) fn services_init(&mut self) {
        let style = &self.cfg.general.launcher.smartui_style;
        let text = if style.trim().is_empty() {
            crate::smartui::DEFAULT_STYLE
        } else {
            style.as_str()
        };
        if let Some(p) = self.panel.as_mut()
            && !p.assistant.services.style_dirty
        {
            p.assistant.services.style = text_editor::Content::with_text(text);
        }
    }

    /// Applications montrées par le lanceur : épinglées, puis fenêtres ouvertes.
    fn smartui_classes(&self) -> Vec<String> {
        let g = &self.cfg.general;
        let mut seen = BTreeSet::new();
        g.launcher
            .pinned
            .iter()
            .cloned()
            .chain(
                self.clients
                    .iter()
                    .filter(|_| g.running_apps != crate::config::RunningApps::Off)
                    .map(|c| c.class.clone()),
            )
            .filter(|c| !c.is_empty() && seen.insert(c.to_lowercase()))
            .collect()
    }

    /// Relit les icônes (thème, propositions, validées) des applications du lanceur.
    pub(in crate::ui) fn smartui_load(&self) -> Task<UiMsg> {
        let classes = self.smartui_classes();
        let (apps, theme) = (self.apps.clone(), self.cfg.general.icon_theme.clone());
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let mut keys = BTreeSet::new();
                    classes
                        .into_iter()
                        // Deux classes, une même icône : une seule ligne.
                        .map(|class| (crate::smartui::key_for(&class, &apps), class))
                        .filter(|(key, _)| keys.insert(key.clone()))
                        .map(|(key, class)| {
                            let lc = class.to_lowercase();
                            let name = apps
                                .iter()
                                .find(|a| {
                                    a.id.to_lowercase() == lc
                                        || a.wm_class
                                            .as_deref()
                                            .is_some_and(|w| w.eq_ignore_ascii_case(&class))
                                })
                                .map(|a| a.name.clone())
                                .unwrap_or_else(|| class.clone());
                            RowData {
                                original: crate::smartui::original_icon(
                                    &class,
                                    &apps,
                                    theme.as_deref(),
                                ),
                                proposal: crate::smartui::proposal(&key)
                                    .and_then(|p| std::fs::read(p).ok()),
                                validated: crate::smartui::validated(&key)
                                    .and_then(|p| std::fs::read(p).ok()),
                                name,
                                class,
                                key,
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .await
                .unwrap_or_default()
            },
            |rows| wrap(Msg::SmartRows(rows)),
        )
    }

    /// Génération automatique : les applications du lanceur sans icône
    /// SmartUI en reçoivent une, validée d'office (une à la fois, chacune
    /// essayée une fois par session).
    pub(in crate::ui) fn smartui_auto(&mut self, classes: Vec<String>) -> Task<UiMsg> {
        if !crate::smartui::auto_on(&self.cfg) {
            return Task::none();
        }
        {
            let mut a = crate::smartui::auto();
            if a.stopped {
                return Task::none();
            }
            for c in classes {
                if !c.is_empty() && a.tried.insert(c.to_lowercase()) {
                    a.queue.push_back(c);
                }
            }
        }
        self.smartui_auto_step()
    }

    fn smartui_auto_step(&mut self) -> Task<UiMsg> {
        let class = {
            let mut a = crate::smartui::auto();
            if a.busy || a.stopped {
                return Task::none();
            }
            let Some(c) = a.queue.pop_front() else {
                if a.added > 0 {
                    crate::hypr::notify(
                        1,
                        6000,
                        &crate::tr!(
                            "SmartUI design : {n} icône(s) ajoutée(s) au thème",
                            n = a.added.to_string()
                        ),
                    );
                    a.added = 0;
                }
                return Task::none();
            };
            a.busy = true;
            c
        };
        let (cfg, apps) = (self.cfg.clone(), self.apps.clone());
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || -> Result<Option<String>, String> {
                    let key = crate::smartui::key_for(&class, &apps);
                    if crate::smartui::validated(&key).is_some() {
                        return Ok(None);
                    }
                    let theme = cfg.general.icon_theme.as_deref();
                    let lc = class.to_lowercase();
                    let name = apps
                        .iter()
                        .find(|a| {
                            a.id.to_lowercase() == lc
                                || a.wm_class
                                    .as_deref()
                                    .is_some_and(|w| w.eq_ignore_ascii_case(&class))
                        })
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| class.clone());
                    let original = crate::smartui::original_icon(&class, &apps, theme);
                    crate::smartui::generate(&cfg, &key, &class, &name, original.as_deref())?;
                    crate::smartui::validate(&key, theme)?;
                    Ok(Some(key))
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| wrap(Msg::SmartAutoDone(r)),
        )
    }

    /// Lance la génération suivante de la file, s'il n'y en a pas en cours.
    fn smartui_next(&mut self) -> Task<UiMsg> {
        let Some(p) = self.panel.as_mut() else {
            return Task::none();
        };
        let s = &mut p.assistant.services;
        if s.busy.is_some() {
            return Task::none();
        }
        let Some(class) = s.queue.pop_front() else {
            return Task::none();
        };
        let (app, name, original) = s
            .rows
            .iter()
            .find(|r| r.key == class)
            .map(|r| (r.class.clone(), r.name.clone(), r.original.clone()))
            .unwrap_or_else(|| (class.clone(), class.clone(), None));
        s.busy = Some(class.clone());
        s.errors.remove(&class);
        let cfg: Arc<crate::config::Config> = self.cfg.clone();
        Task::perform(
            async move {
                let c = class.clone();
                let r = tokio::task::spawn_blocking(move || {
                    crate::smartui::generate(&cfg, &c, &app, &name, original.as_deref())
                        .and_then(|p| std::fs::read(p).map_err(|e| e.to_string()))
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
                (class, r)
            },
            |(class, r)| wrap(Msg::SmartGenerated(class, r)),
        )
    }

    pub(in crate::ui) fn services_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::SmartToggle(v) => {
                self.settings.launcher_smartui = Some(v);
                self.commit()
            }
            Msg::SmartStyleEdit(a) => {
                if let Some(p) = self.panel.as_mut() {
                    let s = &mut p.assistant.services;
                    s.style_dirty |= a.is_edit();
                    s.style.perform(a);
                }
                Task::none()
            }
            Msg::SmartStyleSave => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                let s = &mut p.assistant.services;
                s.style_dirty = false;
                let v = s.style.text().trim().to_string();
                self.settings.smartui_style =
                    (!v.is_empty() && v != crate::smartui::DEFAULT_STYLE).then_some(v);
                self.commit()
            }
            Msg::SmartStyleReset => {
                if let Some(p) = self.panel.as_mut() {
                    let s = &mut p.assistant.services;
                    s.style = text_editor::Content::with_text(crate::smartui::DEFAULT_STYLE);
                    s.style_dirty = false;
                }
                self.settings.smartui_style = None;
                self.commit()
            }
            Msg::SmartRows(rows) => {
                if let Some(p) = self.panel.as_mut() {
                    p.assistant.services.rows = rows.into_iter().map(Row::from).collect();
                }
                Task::none()
            }
            Msg::SmartGenerate(class) => {
                if let Some(p) = self.panel.as_mut() {
                    let s = &mut p.assistant.services;
                    if s.busy.as_ref() != Some(&class) && !s.queue.contains(&class) {
                        s.queue.push_back(class);
                    }
                }
                self.smartui_next()
            }
            Msg::SmartGenerateMissing => {
                if let Some(p) = self.panel.as_mut() {
                    let s = &mut p.assistant.services;
                    let todo: Vec<String> = s
                        .rows
                        .iter()
                        .filter(|r| r.proposal.is_none() && r.validated.is_none())
                        .map(|r| r.key.clone())
                        .filter(|c| s.busy.as_ref() != Some(c) && !s.queue.contains(c))
                        .collect();
                    s.queue.extend(todo);
                }
                self.smartui_next()
            }
            Msg::SmartGenerated(class, r) => {
                if let Some(p) = self.panel.as_mut() {
                    let s = &mut p.assistant.services;
                    if s.busy.as_ref() == Some(&class) {
                        s.busy = None;
                    }
                    match r {
                        Ok(bytes) => {
                            if let Some(row) = s.rows.iter_mut().find(|r| r.key == class) {
                                row.proposal = Some(svg::Handle::from_memory(bytes));
                            }
                        }
                        Err(e) => {
                            // Même erreur pour toutes (clé absente…) : la file s'arrête.
                            s.queue.clear();
                            s.errors.insert(class, e);
                        }
                    }
                }
                self.smartui_next()
            }
            Msg::SmartValidate(class) => {
                if let Err(e) =
                    crate::smartui::validate(&class, self.cfg.general.icon_theme.as_deref())
                {
                    if let Some(p) = self.panel.as_mut() {
                        p.assistant.services.errors.insert(class, e);
                    }
                    return Task::none();
                }
                if let Some(p) = self.panel.as_mut()
                    && let Some(row) = p
                        .assistant
                        .services
                        .rows
                        .iter_mut()
                        .find(|r| r.key == class)
                {
                    row.validated = row.proposal.take();
                }
                Task::batch([self.reload_icons(), self.smartui_suggest_on()])
            }
            Msg::SmartReject(class) => {
                crate::smartui::reject(&class);
                if let Some(p) = self.panel.as_mut()
                    && let Some(row) = p
                        .assistant
                        .services
                        .rows
                        .iter_mut()
                        .find(|r| r.key == class)
                {
                    row.proposal = None;
                }
                Task::none()
            }
            Msg::SmartForget(class) => {
                crate::smartui::forget(&class);
                if let Some(p) = self.panel.as_mut()
                    && let Some(row) = p
                        .assistant
                        .services
                        .rows
                        .iter_mut()
                        .find(|r| r.key == class)
                {
                    row.validated = None;
                }
                self.reload_icons()
            }
            Msg::SmartClear => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                if !p.assistant.services.clear_armed {
                    p.assistant.services.clear_armed = true;
                    return Task::none();
                }
                p.assistant.services.clear_armed = false;
                if let Err(e) = crate::smartui::clear() {
                    tracing::warn!("{}", crate::tr!("SmartUI : {e}", e = e.to_string()));
                }
                for r in &mut p.assistant.services.rows {
                    r.proposal = None;
                    r.validated = None;
                }
                self.reload_icons()
            }
            Msg::SmartAutoDone(r) => {
                let mut tasks = Vec::new();
                {
                    let mut a = crate::smartui::auto();
                    a.busy = false;
                    match &r {
                        Ok(Some(_)) => a.added += 1,
                        Ok(None) => {}
                        Err(e) => {
                            // Même cause pour la suite (fournisseur absent, clé…) : arrêt.
                            a.stopped = true;
                            a.queue.clear();
                            crate::hypr::notify(
                                0,
                                8000,
                                &crate::tr!(
                                    "SmartUI design : génération automatique arrêtée — {e}",
                                    e = e.clone()
                                ),
                            );
                        }
                    }
                }
                if let Ok(Some(_)) = r {
                    tasks.push(self.reload_icons());
                    tasks.push(self.smartui_suggest_on());
                    if self.panel.is_some() {
                        tasks.push(self.smartui_load());
                    }
                }
                tasks.push(self.smartui_auto_step());
                Task::batch(tasks)
            }
            Msg::SmartAutoToggle(v) => {
                self.settings.smartui_auto = Some(v);
                let commit = self.commit();
                if !v {
                    let mut a = crate::smartui::auto();
                    a.queue.clear();
                    return commit;
                }
                // Réactivée : on repart de zéro (après une erreur aussi).
                {
                    let mut a = crate::smartui::auto();
                    a.stopped = false;
                    a.tried.clear();
                }
                let classes = self.smartui_classes();
                Task::batch([commit, self.smartui_auto(classes)])
            }
            Msg::OpenServices => {
                self.subtabs.insert(super::INFO.id, "smartui");
                Task::done(UiMsg::Panel(super::super::super::panel::PanelMsg::Tab(
                    super::INFO.id,
                )))
            }
            _ => Task::none(),
        }
    }

    /// Génération automatique : interrupteur et état de la file.
    fn smartui_auto_line(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0x95, 0x80);
        let on = crate::smartui::auto_on(&self.cfg);
        let (status, c) = {
            let a = crate::smartui::auto();
            if !on {
                (String::new(), dim)
            } else if a.stopped {
                (
                    crate::i18n::tr("Arrêtée après une erreur : la réactiver pour reprendre.")
                        .to_string(),
                    warn,
                )
            } else if a.busy {
                (
                    crate::tr!("En cours… ({n} en file)", n = a.queue.len().to_string()),
                    dim,
                )
            } else {
                (String::new(), dim)
            }
        };
        let mut col = column![
            iced::widget::toggler(on)
                .label(crate::i18n::tr(
                    "Génération automatique (icônes validées d'office)"
                ))
                .on_toggle(|v| wrap(Msg::SmartAutoToggle(v))),
            text(if self.cfg.general.launcher.smartui_auto.is_none() {
                crate::i18n::tr(
                    "Par défaut : active quand LYRA est installée. Chaque nouvelle application du lanceur reçoit son icône.",
                )
            } else {
                crate::i18n::tr("Chaque nouvelle application du lanceur reçoit son icône.")
            })
            .font(self.font)
            .size(11)
            .color(dim),
        ]
        .spacing(4);
        if !status.is_empty() {
            col = col.push(text(status).font(self.font).size(11).color(c));
        }
        col.into()
    }

    /// Première icône validée alors que l'option est coupée : on l'active,
    /// sinon la validation ne se verrait pas dans le lanceur.
    fn smartui_suggest_on(&mut self) -> Task<UiMsg> {
        if self.cfg.general.launcher.smartui || self.settings.launcher_smartui == Some(false) {
            return Task::none();
        }
        self.settings.launcher_smartui = Some(true);
        self.commit()
    }

    pub(in crate::ui) fn services_tab<'a>(&'a self, s: &'a State) -> Element<'a, UiMsg> {
        column![self.providers_section(&s.providers), self.uses_section()]
            .spacing(14)
            .into()
    }

    pub(in crate::ui) fn smartui_section<'a>(&'a self, s: &'a State) -> Element<'a, UiMsg> {
        let g = &self.cfg.general;
        let th = &g.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0x95, 0x80);
        let l = &g.launcher;
        let waiting = s.rows.iter().filter(|r| r.proposal.is_some()).count();
        let validated = s.rows.iter().filter(|r| r.validated.is_some()).count();
        let missing = s
            .rows
            .iter()
            .filter(|r| r.proposal.is_none() && r.validated.is_none())
            .count();
        let summary = match &s.busy {
            Some(c) => crate::tr!(
                "{v} validée(s) · {w} à valider · génération : {c} ({q} en file)",
                v = validated.to_string(),
                w = waiting.to_string(),
                c = c.clone(),
                q = s.queue.len().to_string()
            ),
            None => crate::tr!(
                "{v} validée(s) · {w} à valider · {m} sans proposition",
                v = validated.to_string(),
                w = waiting.to_string(),
                m = missing.to_string()
            ),
        };
        let icon = |h: Option<&svg::Handle>, path: Option<&PathBuf>| -> Element<'a, UiMsg> {
            const PX: f32 = 40.0;
            let e: Element<'a, UiMsg> = match (h, path) {
                (Some(h), _) => svg(h.clone()).width(PX).height(PX).into(),
                (None, Some(p)) if p.extension().is_some_and(|e| e == "svg" || e == "svgz") => {
                    svg(svg::Handle::from_path(p)).width(PX).height(PX).into()
                }
                (None, Some(p)) => iced::widget::image(iced::widget::image::Handle::from_path(p))
                    .width(PX)
                    .height(PX)
                    .into(),
                (None, None) => Space::new().width(PX).height(PX).into(),
            };
            container(e)
                .center(PX + 8.0)
                .style(move |_| container::Style {
                    border: iced::Border::default()
                        .rounded(10)
                        .width(1)
                        .color(Color { a: 0.18, ..fg }),
                    ..Default::default()
                })
                .into()
        };
        let small_btn = |label: &'a str, msg: Option<UiMsg>, primary: bool| {
            button(text(label).font(self.font).size(12))
                .padding([4, 10])
                .style(if primary {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press_maybe(msg)
        };
        let mut list = column![].spacing(10);
        for r in &s.rows {
            let busy = s.busy.as_ref() == Some(&r.key);
            let queued = s.queue.contains(&r.key);
            let shown = r.proposal.as_ref().or(r.validated.as_ref());
            let state = if busy {
                crate::i18n::tr("Génération…").to_string()
            } else if queued {
                crate::i18n::tr("En file").to_string()
            } else if r.proposal.is_some() {
                crate::i18n::tr("Proposition à valider").to_string()
            } else if r.validated.is_some() {
                crate::i18n::tr("Validée").to_string()
            } else {
                crate::i18n::tr("Icône du thème").to_string()
            };
            let c = r.key.clone();
            let actions: Element<'a, UiMsg> = if busy || queued {
                Space::new().into()
            } else if r.proposal.is_some() {
                row![
                    small_btn("Valider", Some(wrap(Msg::SmartValidate(c.clone()))), true),
                    small_btn("Refaire", Some(wrap(Msg::SmartGenerate(c.clone()))), false),
                    small_btn("Rejeter", Some(wrap(Msg::SmartReject(c))), false),
                ]
                .spacing(6)
                .into()
            } else if r.validated.is_some() {
                row![
                    small_btn("Refaire", Some(wrap(Msg::SmartGenerate(c.clone()))), false),
                    small_btn("Retirer", Some(wrap(Msg::SmartForget(c))), false),
                ]
                .spacing(6)
                .into()
            } else {
                small_btn("Générer", Some(wrap(Msg::SmartGenerate(c))), false).into()
            };
            let mut info = column![
                text(&r.name).font(self.font).size(14).color(fg),
                text(state).font(self.font).size(11).color(dim),
            ]
            .spacing(2);
            if let Some(e) = s.errors.get(&r.key) {
                info = info.push(text(e).font(self.font).size(11).color(warn));
            }
            list = list.push(
                row![
                    icon(None, r.original.as_ref()),
                    text("→").font(self.font).size(14).color(dim),
                    icon(shown, None),
                    info.width(Length::Fill),
                    actions,
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        if s.rows.is_empty() {
            list = list.push(
                text("Aucune application dans le lanceur : épinglez-en, ou ouvrez des fenêtres.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        column![
            self.section("SmartUI design"),
            text(
                "Les icônes des applications redessinées dans un même style par le fournisseur \
                 choisi pour SmartUI (Autres services IA → Utilisation). Les icônes validées \
                 forment le thème d'icônes « HyprDesk SmartUI » : lanceur, bureau, SUPER+R, \
                 barre et notifications."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            iced::widget::toggler(l.smartui)
                .label(crate::i18n::tr(
                    "Utiliser le thème SmartUI (lanceur, bureau, SUPER+R, barre, notifications)"
                ))
                .on_toggle(|v| wrap(Msg::SmartToggle(v))),
            text(crate::tr!(
                "Fournisseur : {name} (Autres services IA → Utilisation)",
                name = g.ai.for_consumer("smartui").label()
            ))
            .font(self.font)
            .size(12)
            .color(accent),
            self.smartui_auto_line(),
            self.field(
                "Style",
                "smartui_style",
                column![
                    text_editor(&s.style)
                        .placeholder(crate::i18n::tr("vide : le style HyprDesk"))
                        .font(self.font)
                        .size(13)
                        .padding(8)
                        .height(240)
                        .on_action(|a| wrap(Msg::SmartStyleEdit(a))),
                    row![
                        button(text("Enregistrer le style").font(self.font).size(12))
                            .padding([4, 10])
                            .on_press_maybe(s.style_dirty.then(|| wrap(Msg::SmartStyleSave))),
                        button(text("Style HyprDesk").font(self.font).size(12))
                            .padding([4, 10])
                            .style(button::secondary)
                            .on_press_maybe(
                                (!l.smartui_style.is_empty() || s.style_dirty)
                                    .then(|| wrap(Msg::SmartStyleReset)),
                            ),
                        text(if s.style_dirty {
                            crate::i18n::tr("modifié, non enregistré")
                        } else {
                            ""
                        })
                        .font(self.font)
                        .size(11)
                        .color(dim),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(6),
            ),
            row![
                button(
                    text(crate::tr!(
                        "Générer les icônes manquantes ({n})",
                        n = missing.to_string()
                    ))
                    .font(self.font)
                    .size(13)
                )
                .padding([6, 14])
                .on_press_maybe((missing > 0).then(|| wrap(Msg::SmartGenerateMissing))),
                button(
                    text(if s.clear_armed {
                        crate::i18n::tr("Vider le cache ?")
                    } else {
                        crate::i18n::tr("Vider le cache")
                    })
                    .font(self.font)
                    .size(13)
                )
                .padding([6, 14])
                .style(if s.clear_armed {
                    button::danger
                } else {
                    button::secondary
                })
                .on_press(wrap(Msg::SmartClear)),
            ]
            .spacing(10),
            text(summary).font(self.font).size(12).color(accent),
            list,
        ]
        .spacing(14)
        .into()
    }
}
