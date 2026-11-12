//! Paquet intégré « Thème des applications » : le thème de hyprdesk appliqué
//! à d'autres logiciels (kitty pour commencer), réappliqué à chaque
//! changement de thème. Plugin configurable : chaque application a ses
//! réglages (fichiers, rechargement), ici ou dans Réglages → Plugins →
//! Configurer. Un échec se propose à LYRA. Domaine : `crate::apptheme`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::BTreeSet;

use crate::i18n::text;
use iced::widget::{Space, button, column, container, row, text_input, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::Config;

pub const INFO: PluginInfo = PluginInfo {
    id: "apptheme",
    label: "Thème des applications",
    icon: "preferences-desktop-theme",
    summary: "Le thème de hyprdesk appliqué à d'autres logiciels : kitty pour commencer.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::AppTheme(m)))
}

#[derive(Debug, Clone, Default)]
pub struct State {
    /// Compte rendu du dernier « Appliquer maintenant ».
    pub report: Vec<(String, Result<String, String>)>,
    /// Application dont les réglages sont dépliés.
    pub editing: Option<&'static str>,
    /// Saisies en cours : `<cible>.<option>` → texte (validé par Entrée).
    pub drafts: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Toggle(&'static str, bool),
    ApplyNow,
    Report(Vec<(String, Result<String, String>)>),
    /// Déplie ou replie les réglages d'une application.
    Edit(&'static str),
    Draft(String, String),
    /// Enregistre une option (`<cible>.<option>`).
    Submit(String),
    /// Valeurs par défaut d'une application.
    Reset(&'static str),
}

/// Options effectives : celles en cours de réglage, sinon celles de la config.
fn current_opts(app: &App) -> crate::apptheme::Opts {
    let all = app
        .settings
        .plugin_options
        .clone()
        .unwrap_or_else(|| app.cfg.general.plugin_options.clone());
    all.get(crate::apptheme::PLUGIN)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.display())).collect())
        .unwrap_or_default()
}

/// Cibles effectives : celles de la config, ou aucune si le paquet est coupé.
pub fn effective(cfg: &Config) -> BTreeSet<String> {
    if cfg.plugin_on(INFO.id) {
        cfg.general.app_themes.clone()
    } else {
        BTreeSet::new()
    }
}

impl App {
    /// Applique le thème aux applications choisies (hors du fil de l'interface).
    pub(in crate::ui) fn sync_app_themes(&self, report: bool) -> Task<UiMsg> {
        let (targets, theme) = (effective(&self.cfg), self.cfg.general.theme.clone());
        let opts = current_opts(self);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || crate::apptheme::sync(&targets, &theme, &opts))
                    .await
                    .unwrap_or_default()
            },
            move |r| {
                for (id, res) in &r {
                    if let Err(e) = res {
                        tracing::warn!("{}", crate::tr!("thème de {id} : {e}", id = id, e = e));
                    }
                }
                if report {
                    wrap(Msg::Report(r))
                } else {
                    UiMsg::Noop
                }
            },
        )
    }

    pub(in crate::ui) fn apptheme_page(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let accent = color(g.theme.accent, 1.0);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let on = self
            .settings
            .app_themes
            .clone()
            .unwrap_or_else(|| g.app_themes.clone());
        let mut col = column![
            self.section("Applications"),
            text(crate::i18n::tr(
                "Chaque application cochée prend les couleurs du thème en place, et les reprend \
                 à chaque changement de thème. Décochée, elle retrouve son propre thème."
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(14);
        let opts = current_opts(self);
        let editing = self.panel.as_ref().and_then(|p| p.apptheme.editing);
        for t in crate::apptheme::TARGETS {
            let installed = (t.installed)();
            let checked = on.contains(t.id);
            let status = if installed {
                text(crate::i18n::tr("installé")).color(dim)
            } else {
                text(crate::i18n::tr("introuvable")).color(warn)
            };
            col = col.push(
                container(
                    column![
                        row![
                            column![
                                row![
                                    text(t.label).font(self.title_font()).size(14),
                                    status.font(self.font).size(12),
                                ]
                                .spacing(8)
                                .align_y(Alignment::Center),
                                text(crate::i18n::tr(t.summary))
                                    .font(self.font)
                                    .size(12)
                                    .color(dim),
                            ]
                            .spacing(4)
                            .width(Length::Fill),
                            toggler(checked).on_toggle(move |v| wrap(Msg::Toggle(t.id, v))),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center),
                        row![
                            text(crate::tr!(
                                "Fichier : {path}",
                                path = (t.file)(&opts).display().to_string()
                            ))
                            .font(self.font)
                            .size(12)
                            .color(dim)
                            .width(Length::Fill),
                            button(
                                text(if editing == Some(t.id) {
                                    "Fermer"
                                } else {
                                    "Réglages"
                                })
                                .font(self.font)
                                .size(12),
                            )
                            .padding([4, 10])
                            .style(button::secondary)
                            .on_press(wrap(Msg::Edit(t.id))),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                        self.apptheme_options(t, &opts, editing == Some(t.id)),
                    ]
                    .spacing(8),
                )
                .padding(12)
                .style(move |_| container::Style {
                    border: Border::default()
                        .rounded(10)
                        .width(1)
                        .color(Color { a: 0.15, ..fg }),
                    ..Default::default()
                }),
            );
        }
        // Aperçu : les couleurs que kitty recevra.
        let swatch = |c: crate::config::Rgba, label: &'static str| {
            let c = color(c, 1.0);
            row![
                container(Space::new().width(18).height(18)).style(move |_| container::Style {
                    background: Some(c.into()),
                    border: Border::default()
                        .rounded(4)
                        .width(1)
                        .color(Color { a: 0.3, ..fg }),
                    ..Default::default()
                }),
                text(crate::i18n::tr(label))
                    .font(self.font)
                    .size(12)
                    .color(dim),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        };
        col = col.push(
            row![
                swatch(g.theme.fg, "texte"),
                swatch(g.theme.bg, "fond"),
                swatch(g.theme.accent, "accent, sélection, curseur"),
            ]
            .spacing(16),
        );
        col = col.push(
            button(
                text(crate::i18n::tr("Appliquer maintenant"))
                    .font(self.font)
                    .size(13),
            )
            .padding([6, 14])
            .on_press(wrap(Msg::ApplyNow)),
        );
        if let Some(p) = &self.panel {
            for (id, r) in &p.apptheme.report {
                match r {
                    Ok(m) => col = col.push(text(m.clone()).color(accent).font(self.font).size(12)),
                    Err(e) => {
                        // Échec : le problème se propose à LYRA (bouton IA).
                        let ask = crate::apptheme::target(id)
                            .map(|t| crate::apptheme::lyra_prompt(t, &opts, e))
                            .map(|prompt| {
                                self.lyra_button(crate::i18n::tr("Thème des applications"), prompt)
                            });
                        let mut line = row![
                            text(crate::tr!("{id} : {e}", id = id, e = e))
                                .color(warn)
                                .font(self.font)
                                .size(12)
                                .width(Length::Fill)
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center);
                        if let Some(b) = ask {
                            line = line.push(b);
                        }
                        col = col.push(line);
                    }
                }
            }
        }
        col.into()
    }

    /// Réglages d'une application (dépliés) : un champ par option, validé
    /// par Entrée ; « Valeurs par défaut » les efface.
    fn apptheme_options<'a>(
        &'a self,
        t: &'static crate::apptheme::Target,
        opts: &crate::apptheme::Opts,
        open: bool,
    ) -> Element<'a, UiMsg> {
        if !open {
            return Space::new().into();
        }
        let dim = self.ink().dim;
        let drafts = self.panel.as_ref().map(|p| &p.apptheme.drafts);
        let mut col = column![].spacing(10);
        for o in t.options {
            let key = format!("{}.{}", t.id, o.key);
            let shown = drafts
                .and_then(|d| d.get(&key).cloned())
                .or_else(|| opts.get(&key).cloned())
                .unwrap_or_else(o.default);
            let k2 = key.clone();
            col = col.push(
                column![
                    text(crate::i18n::tr(o.label)).font(self.font).size(13),
                    text_input(&(o.default)(), &shown)
                        .font(self.font)
                        .size(13)
                        .padding(6)
                        .on_input(move |v| wrap(Msg::Draft(k2.clone(), v)))
                        .on_submit(wrap(Msg::Submit(key)))
                        .width(Length::Fill),
                    text(crate::i18n::tr(o.help)).font(self.font).size(12).color(dim),
                ]
                .spacing(4),
            );
        }
        col = col.push(
            row![
                text(crate::i18n::tr("Valider un champ avec Entrée ; le thème est réappliqué aussitôt."))
                    .font(self.font)
                    .size(12)
                    .color(dim)
                    .width(Length::Fill),
                button(text("Valeurs par défaut").font(self.font).size(12))
                    .padding([3, 8])
                    .style(button::text)
                    .on_press(wrap(Msg::Reset(t.id))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
        col.into()
    }

    /// Enregistre les options du paquet (toutes : `<cible>.<option>` → texte).
    fn set_apptheme_opts(&mut self, f: impl FnOnce(&mut std::collections::BTreeMap<String, crate::tiling::OptValue>)) -> Task<UiMsg> {
        let mut all = self
            .settings
            .plugin_options
            .clone()
            .unwrap_or_else(|| self.cfg.general.plugin_options.clone());
        let m = all.entry(crate::apptheme::PLUGIN.to_string()).or_default();
        f(m);
        all.retain(|_, m| !m.is_empty());
        self.settings.plugin_options = Some(all);
        let commit = self.commit();
        Task::batch([commit, self.sync_app_themes(true)])
    }

    pub(in crate::ui) fn apptheme_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Toggle(id, v) => {
                let mut set = self
                    .settings
                    .app_themes
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.app_themes.clone());
                if v {
                    set.insert(id.to_string());
                } else {
                    set.remove(id);
                }
                self.settings.app_themes = Some(set);
                self.commit()
            }
            Msg::ApplyNow => self.sync_app_themes(true),
            Msg::Edit(id) => {
                if let Some(p) = self.panel.as_mut() {
                    p.apptheme.drafts.clear();
                    p.apptheme.editing = (p.apptheme.editing != Some(id)).then_some(id);
                }
                Task::none()
            }
            Msg::Draft(k, v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.apptheme.drafts.insert(k, v);
                }
                Task::none()
            }
            Msg::Submit(key) => {
                let Some(v) = self.panel.as_mut().and_then(|p| p.apptheme.drafts.remove(&key)) else {
                    return Task::none();
                };
                self.set_apptheme_opts(move |m| {
                    m.insert(key, crate::tiling::OptValue::Str(v.trim().to_string()));
                })
            }
            Msg::Reset(id) => {
                if let Some(p) = self.panel.as_mut() {
                    p.apptheme.drafts.clear();
                }
                let prefix = format!("{id}.");
                self.set_apptheme_opts(move |m| m.retain(|k, _| !k.starts_with(&prefix)))
            }
            Msg::Report(r) => {
                if let Some(p) = self.panel.as_mut() {
                    p.apptheme.report = r;
                }
                Task::none()
            }
        }
    }
}
