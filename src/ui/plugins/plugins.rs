//! Paquet intégré « Plugins » : ce qui est installé et actif, et comment en
//! ajouter. Deux familles :
//!
//! - les **paquets intégrés** (`BUILTIN`) : compilés avec hyprdesk ; on peut
//!   les couper (`disabled_plugins`), sauf ceux du cœur ;
//! - les **plugins Lua** de widgets et de barre : intégrés, livrés, ou tiers
//!   (installés dans `~/.config/hypr/hyprdesk/widgets/` depuis un fichier, un
//!   dossier ou un dépôt git ; retirés dans `.retired/`, sans destruction).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, container, row, text_input, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{BUILTIN, PluginInfo, PluginMsg};
use crate::config::{CORE_PLUGINS, WidgetSource};

pub const INFO: PluginInfo = PluginInfo {
    id: "plugins",
    label: "Plugins",
    icon: "application-x-addon",
    summary: "Paquets intégrés et plugins Lua : état, activation, ajout d'un plugin tiers.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Plugins(m)))
}

/// Onglet de la page Plugins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    /// Paquets intégrés (`BUILTIN`).
    #[default]
    Packages,
    /// Plugins Lua de widgets et de barre, et ajout d'un plugin tiers.
    Widgets,
}

#[derive(Default)]
pub struct State {
    /// Onglet affiché.
    pub tab: Tab,
    /// Source à installer : fichier `.lua`, dossier, ou URL git.
    pub source: String,
    /// Résultat de la dernière installation ou du dernier retrait.
    pub report: Option<Result<String, String>>,
    /// Plugin dont la configuration est dépliée.
    pub configuring: Option<String>,
    /// Saisies en cours (option ou commande → texte).
    pub drafts: std::collections::HashMap<String, String>,
    /// Sources de paquets de la machine (mises à jour) : id, libellé,
    /// relevé et mise à jour de hyprdesk.
    pub update_sources: Vec<(String, String, String, String)>,
    pub opt_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Change d'onglet.
    Tab(Tab),
    /// Active ou coupe un paquet intégré.
    Package(String, bool),
    Source(String),
    Install,
    Installed(Result<String, String>),
    /// Retire un plugin tiers (fichier du plugin).
    Retire(std::path::PathBuf),
    /// Déplie ou replie la configuration d'un plugin.
    Configure(String),
    /// Option booléenne d'un plugin.
    OptBool(String, String, bool),
    /// Saisie d'une option (clé de brouillon, texte).
    Draft(String, String),
    /// Option texte ou nombre validée (plugin, option).
    OptSubmit(String, String),
    /// Toutes les options d'un plugin reviennent aux valeurs du plugin.
    OptReset(String),
    /// Commande de mise à jour validée (source, `true` : mise à jour).
    UpdSubmit(String, bool),
    /// Commandes d'une source rendues à hyprdesk.
    UpdReset(String),
}

/// Dossier des plugins de l'utilisateur.
fn user_dir(app: &App) -> std::path::PathBuf {
    crate::widgets::plugins::dirs(
        app.config_path
            .parent()
            .unwrap_or(std::path::Path::new(".")),
    )
    .remove(0)
}

impl App {
    pub(in crate::ui) fn plugins_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = self.ink().dim;
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let badge = move |label: &'static str, on: bool| {
            container(
                text(label)
                    .font(self.font)
                    .size(12)
                    .color(if on { accent } else { dim }),
            )
            .padding([2, 8])
            .style(move |_| container::Style {
                border: Border::default().rounded(8).width(1).color(Color {
                    a: 0.5,
                    ..(if on { accent } else { dim })
                }),
                ..Default::default()
            })
        };

        // Paquets intégrés.
        let mut packages = column![].spacing(10);
        for info in BUILTIN {
            let core = CORE_PLUGINS.contains(&info.id);
            let on = self.cfg.plugin_on(info.id);
            let id = info.id.to_string();
            let switch: Element<'_, UiMsg> = if core {
                badge(crate::i18n::tr("cœur"), true).into()
            } else {
                toggler(on)
                    .on_toggle(move |v| wrap(Msg::Package(id.clone(), v)))
                    .into()
            };
            packages = packages.push(
                row![
                    column![
                        row![
                            text(info.label).font(self.title_font()).size(14),
                            badge(
                                if on {
                                    "actif"
                                } else {
                                    crate::i18n::tr("coupé")
                                },
                                on
                            ),
                            badge(crate::i18n::tr("intégré"), false),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        text(info.summary).font(self.font).size(12).color(dim),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                    switch,
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }

        // Plugins Lua (et widgets intégrés), avec leur origine et leur état.
        let user = user_dir(self);
        let mut lua = column![].spacing(10);
        for t in &self.cfg.catalog {
            let active = self.cfg.widgets.iter().any(|w| w.type_id == t.id);
            let (origin, path) = match &t.source {
                WidgetSource::Builtin => (crate::i18n::tr("intégré"), None),
                WidgetSource::Bundled => (crate::i18n::tr("livré"), None),
                WidgetSource::Plugin(path) => ("tiers", Some(path.clone())),
            };
            let mut head = row![
                text(t.name.clone()).font(self.title_font()).size(14),
                badge(if active { "actif" } else { "inactif" }, active),
                badge(origin, false),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            if t.error.is_some() {
                head = head.push(text("en erreur").font(self.font).size(12).color(warn));
            }
            let mut info = column![head].spacing(3).width(Length::Fill);
            let detail = match (&t.error, &path) {
                (Some(e), _) => e.clone(),
                (None, Some(p)) => format!("{} — {}", t.description, p.display()),
                (None, None) => t.description.clone(),
            };
            info = info.push(text(detail).font(self.font).size(12).color(dim));
            let mut line = row![info].spacing(12).align_y(Alignment::Center);
            if let Some(path) = path.filter(|p| p.starts_with(&user)) {
                line = line.push(
                    button(text("Retirer").font(self.font).size(12))
                        .padding([4, 10])
                        .style(button::secondary)
                        .on_press(wrap(Msg::Retire(path))),
                );
            }
            let configurable = !t.options.is_empty() || t.id == "majs";
            if configurable && t.error.is_none() {
                let open = p.plugins.configuring.as_deref() == Some(t.id.as_str());
                line = line.push(
                    button(
                        text(if open { "Fermer" } else { "Configurer" })
                            .font(self.font)
                            .size(12),
                    )
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(wrap(Msg::Configure(t.id.clone()))),
                );
            }
            if t.toggleable || active {
                let id = t.id.clone();
                line = line.push(toggler(active).on_toggle(move |v| {
                    super::widgets::wrap(super::widgets::Msg::WidgetToggle(id.clone(), v))
                }));
            }
            lua = lua.push(line);
            if p.plugins.configuring.as_deref() == Some(t.id.as_str()) {
                lua = lua.push(self.plugin_config(t));
            }
        }

        // Ajouter un plugin tiers.
        let mut add = column![
            row![
                text_input(
                    crate::i18n::tr("fichier .lua, dossier, ou https://…git"),
                    &p.plugins.source
                )
                .font(self.font)
                .on_input(|v| wrap(Msg::Source(v)))
                .on_submit(wrap(Msg::Install))
                .padding(8)
                .width(Length::Fill),
                button(text("Installer").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press_maybe(
                        (!p.plugins.source.trim().is_empty()).then(|| wrap(Msg::Install))
                    ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                button(
                    text("Ouvrir le dossier des plugins")
                        .font(self.font)
                        .size(12)
                )
                .padding([4, 10])
                .style(button::secondary)
                .on_press(super::widgets::wrap(super::widgets::Msg::OpenWidgetDir)),
                button(
                    text("Rechercher de nouveaux plugins")
                        .font(self.font)
                        .size(12)
                )
                .padding([4, 10])
                .style(button::secondary)
                .on_press(super::widgets::wrap(super::widgets::Msg::RescanWidgets)),
            ]
            .spacing(10),
        ]
        .spacing(10);
        if let Some(r) = &p.plugins.report {
            add = add.push(match r {
                Ok(m) => text(m.clone()).font(self.font).size(12).color(accent),
                Err(e) => text(e.clone()).font(self.font).size(12).color(warn),
            });
        }

        // Onglets : paquets intégrés | plugins de widget de barre.
        let tab_button = |label: String, tab: Tab| {
            let active = p.plugins.tab == tab;
            button(
                text(label)
                    .size(13)
                    .font(self.ui_font(iced::font::Weight::Medium)),
            )
            .padding([8, 14])
            .on_press(wrap(Msg::Tab(tab)))
            .style(move |_, status| button::Style {
                background: if active {
                    Some(Color { a: 0.16, ..accent }.into())
                } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    Some(Color { a: 0.07, ..fg }.into())
                } else {
                    None
                },
                text_color: if active { accent } else { fg },
                border: Border::default().rounded(11),
                ..Default::default()
            })
        };
        let tabs = row![
            tab_button(
                crate::tr!("Paquets intégrés ({n})", n = BUILTIN.len()),
                Tab::Packages
            ),
            tab_button(
                crate::tr!(
                    "Plugins de widget de barre ({n})",
                    n = self.cfg.catalog.len()
                ),
                Tab::Widgets
            ),
        ]
        .spacing(6);

        let body: Element<'_, UiMsg> = match p.plugins.tab {
            Tab::Packages => column![
                self.section("Paquets intégrés"),
                text(
                    "Compilés avec hyprdesk. Couper un paquet le retire des réglages et coupe sa \
                     fonction (barre, lanceur, règles de workspaces…)."
                )
                .font(self.font)
                .size(12)
                .color(dim),
                packages,
            ]
            .spacing(14)
            .into(),
            Tab::Widgets => column![
                self.section("Plugins de widget de barre"),
                lua,
                self.section("Ajouter un plugin tiers"),
                text(crate::tr!(
                    "Un plugin Lua (fichier, dossier avec init.lua, ou dépôt git) est installé \
                     dans {p0} ; il apparaît aussitôt ci-dessus et dans Widgets.",
                    p0 = user.display()
                ))
                .font(self.font)
                .size(12)
                .color(dim),
                add,
            ]
            .spacing(14)
            .into(),
        };

        column![tabs, body].spacing(6).into()
    }

    /// Configuration d'un plugin : ses options (valeur réglée, sinon celle
    /// du plugin) ; pour les mises à jour, les commandes de chaque source.
    fn plugin_config<'a>(&'a self, t: &'a crate::config::WidgetType) -> Element<'a, UiMsg> {
        use crate::tiling::OptValue;
        let Some(p) = &self.panel else {
            return column![].into();
        };
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = self.ink().dim;
        let set = self.cfg.general.plugin_options.get(&t.id);
        let mut col = column![].spacing(8);
        for (k, default) in &t.options {
            let cur = set.and_then(|m| m.get(k)).unwrap_or(default);
            let label = text(k.replace('_', " "))
                .font(self.font)
                .size(13)
                .width(Length::Fill);
            let field: Element<'_, UiMsg> = match cur {
                OptValue::Bool(b) => {
                    let (id, key) = (t.id.clone(), k.clone());
                    toggler(*b)
                        .on_toggle(move |v| wrap(Msg::OptBool(id.clone(), key.clone(), v)))
                        .into()
                }
                other => {
                    let dk = format!("opt:{}:{k}", t.id);
                    let shown = p
                        .plugins
                        .drafts
                        .get(&dk)
                        .cloned()
                        .unwrap_or_else(|| other.display());
                    let (id, key) = (t.id.clone(), k.clone());
                    text_input(&default.display(), &shown)
                        .font(self.font)
                        .size(13)
                        .padding(6)
                        .width(220)
                        .on_input(move |v| wrap(Msg::Draft(dk.clone(), v)))
                        .on_submit(wrap(Msg::OptSubmit(id, key)))
                        .into()
                }
            };
            col = col.push(row![label, field].spacing(10).align_y(Alignment::Center));
        }
        if !t.options.is_empty() {
            col = col.push(
                row![
                    text(crate::i18n::tr(
                        "Valider un texte avec Entrée. Ces réglages valent pour chaque \
                         instance du plugin, sauf les options écrites dans hyprdesk.lua."
                    ))
                    .font(self.font)
                    .size(12)
                    .color(dim)
                    .width(Length::Fill),
                    button(text("Valeurs du plugin").font(self.font).size(12))
                        .padding([3, 8])
                        .style(button::text)
                        .on_press(wrap(Msg::OptReset(t.id.clone()))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        if t.id == "majs" {
            col = col.push(
                text(crate::i18n::tr("Commandes utilisées"))
                    .font(self.title_font())
                    .size(13),
            );
            col = col.push(
                text(crate::i18n::tr(
                    "Relevé : sans droits, une ligne par paquet. Mise à jour : non interactive \
                     (pkexec pour les droits). Vide : la commande de hyprdesk. Journal : \
                     ~/.local/state/hyprdesk/updates.log.",
                ))
                .font(self.font)
                .size(12)
                .color(dim),
            );
            let over = &self.cfg.general.update_commands;
            for (id, label, check, upgrade) in &p.plugins.update_sources {
                let (oc, ou) = over.get(id).cloned().unwrap_or_default();
                let field = |upgrade_cmd: bool, default: &str, set: &str| {
                    let dk = format!("upd:{id}:{upgrade_cmd}");
                    let shown = p
                        .plugins
                        .drafts
                        .get(&dk)
                        .cloned()
                        .unwrap_or_else(|| set.to_string());
                    let hint = if default.is_empty() {
                        crate::i18n::tr("aucune : comprise dans une autre étape").to_string()
                    } else {
                        default.to_string()
                    };
                    text_input(&hint, &shown)
                        .font(iced::Font::MONOSPACE)
                        .size(12)
                        .padding(6)
                        .width(Length::Fill)
                        .on_input(move |v| wrap(Msg::Draft(dk.clone(), v)))
                        .on_submit(wrap(Msg::UpdSubmit(id.clone(), upgrade_cmd)))
                };
                col = col.push(
                    row![
                        text(crate::i18n::tr(label))
                            .font(self.font)
                            .size(13)
                            .width(Length::Fill),
                        button(text("Commandes de hyprdesk").font(self.font).size(12))
                            .padding([3, 8])
                            .style(button::text)
                            .on_press(wrap(Msg::UpdReset(id.clone()))),
                    ]
                    .align_y(Alignment::Center),
                );
                col = col.push(
                    row![
                        text(crate::i18n::tr("relevé"))
                            .font(self.font)
                            .size(12)
                            .color(dim)
                            .width(90),
                        field(false, check, &oc)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
                col = col.push(
                    row![
                        text(crate::i18n::tr("mise à jour"))
                            .font(self.font)
                            .size(12)
                            .color(dim)
                            .width(90),
                        field(true, upgrade, &ou)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
            if p.plugins.update_sources.is_empty() {
                col = col.push(
                    text(crate::i18n::tr("aucune source de paquets reconnue"))
                        .font(self.font)
                        .size(12)
                        .color(dim),
                );
            }
        }
        if let Some(e) = &p.plugins.opt_error {
            col = col.push(
                text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
            );
        }
        container(col)
            .padding(12)
            .width(Length::Fill)
            .style(move |_| container::Style {
                border: iced::Border::default()
                    .rounded(10)
                    .width(1)
                    .color(Color { a: 0.15, ..fg }),
                ..Default::default()
            })
            .into()
    }

    /// Enregistre une option de plugin (et recharge : les plugins relisent leurs options).
    fn set_plugin_option(
        &mut self,
        plugin: &str,
        key: &str,
        v: Option<crate::tiling::OptValue>,
    ) -> Task<UiMsg> {
        let mut all = self
            .settings
            .plugin_options
            .clone()
            .unwrap_or_else(|| self.cfg.general.plugin_options.clone());
        let m = all.entry(plugin.to_string()).or_default();
        match v {
            Some(v) => {
                m.insert(key.to_string(), v);
            }
            None => {
                m.remove(key);
            }
        }
        all.retain(|_, m| !m.is_empty());
        self.settings.plugin_options = Some(all);
        let applied = self.apply();
        Task::batch([applied, self.commit_reload()])
    }

    pub(in crate::ui) fn plugins_update(&mut self, msg: Msg) -> Task<UiMsg> {
        use crate::tiling::OptValue;
        match msg {
            Msg::Configure(id) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                p.plugins.drafts.clear();
                p.plugins.opt_error = None;
                if p.plugins.configuring.as_deref() == Some(id.as_str()) {
                    p.plugins.configuring = None;
                    return Task::none();
                }
                if id == "majs" {
                    p.plugins.update_sources = crate::updates::sources()
                        .into_iter()
                        .map(|s| {
                            (
                                s.id.to_string(),
                                s.label.to_string(),
                                crate::updates::shown(&s.check),
                                crate::updates::shown(&s.upgrade),
                            )
                        })
                        .collect();
                }
                p.plugins.configuring = Some(id);
                Task::none()
            }
            Msg::OptBool(plugin, key, v) => {
                self.set_plugin_option(&plugin, &key, Some(OptValue::Bool(v)))
            }
            Msg::Draft(k, v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.plugins.drafts.insert(k, v);
                    p.plugins.opt_error = None;
                }
                Task::none()
            }
            Msg::OptSubmit(plugin, key) => {
                let dk = format!("opt:{plugin}:{key}");
                let Some(raw) = self
                    .panel
                    .as_mut()
                    .and_then(|p| p.plugins.drafts.remove(&dk))
                else {
                    return Task::none();
                };
                let default = self
                    .cfg
                    .catalog
                    .iter()
                    .find(|t| t.id == plugin)
                    .and_then(|t| t.options.iter().find(|(k, _)| *k == key))
                    .map(|(_, v)| v.clone());
                let raw = raw.trim().to_string();
                let v = match default {
                    _ if raw.is_empty() => None,
                    Some(OptValue::Num(_)) => match raw.replace(',', ".").parse::<f64>() {
                        Ok(n) if n.is_finite() => Some(OptValue::Num(n)),
                        _ => {
                            if let Some(p) = self.panel.as_mut() {
                                p.plugins.opt_error =
                                    Some(crate::tr!("« {v} » : un nombre est attendu", v = raw));
                            }
                            return Task::none();
                        }
                    },
                    _ => Some(OptValue::Str(raw)),
                };
                self.set_plugin_option(&plugin, &key, v)
            }
            Msg::OptReset(plugin) => {
                let mut all = self
                    .settings
                    .plugin_options
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.plugin_options.clone());
                all.remove(&plugin);
                self.settings.plugin_options = Some(all);
                if let Some(p) = self.panel.as_mut() {
                    p.plugins.drafts.clear();
                }
                let applied = self.apply();
                Task::batch([applied, self.commit_reload()])
            }
            Msg::UpdSubmit(source, upgrade) => {
                let dk = format!("upd:{source}:{upgrade}");
                let Some(raw) = self
                    .panel
                    .as_mut()
                    .and_then(|p| p.plugins.drafts.remove(&dk))
                else {
                    return Task::none();
                };
                let mut all = self
                    .settings
                    .update_commands
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.update_commands.clone());
                let e = all.entry(source).or_default();
                if upgrade {
                    e.1 = raw.trim().to_string();
                } else {
                    e.0 = raw.trim().to_string();
                }
                all.retain(|_, (c, u)| !c.is_empty() || !u.is_empty());
                self.settings.update_commands = Some(all);
                self.commit()
            }
            Msg::UpdReset(source) => {
                let mut all = self
                    .settings
                    .update_commands
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.update_commands.clone());
                all.remove(&source);
                self.settings.update_commands = Some(all);
                if let Some(p) = self.panel.as_mut() {
                    p.plugins
                        .drafts
                        .retain(|k, _| !k.starts_with(&format!("upd:{source}:")));
                }
                self.commit()
            }
            Msg::Tab(tab) => {
                if let Some(p) = self.panel.as_mut() {
                    p.plugins.tab = tab;
                }
                Task::none()
            }
            Msg::Package(id, on) => {
                let mut set = self
                    .settings
                    .disabled_plugins
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.disabled_plugins.clone());
                if on {
                    set.remove(&id);
                } else {
                    set.insert(id);
                }
                self.settings.disabled_plugins = Some(set);
                self.commit()
            }
            Msg::Source(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.plugins.source = v;
                    p.plugins.report = None;
                }
                Task::none()
            }
            Msg::Install => {
                let Some(source) = self.panel.as_ref().map(|p| p.plugins.source.clone()) else {
                    return Task::none();
                };
                let dest = user_dir(self);
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::widgets::plugins::install(&source, &dest)
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |r| wrap(Msg::Installed(r)),
                )
            }
            Msg::Installed(r) => {
                let ok = r.is_ok();
                if let Some(p) = self.panel.as_mut() {
                    if ok {
                        p.plugins.source.clear();
                    }
                    p.plugins.report = Some(r.map(|n| crate::tr!("« {n} » installé.", n = n)));
                }
                // Relire la config : le nouveau plugin entre au catalogue.
                if ok {
                    Task::done(UiMsg::ConfigChanged)
                } else {
                    Task::none()
                }
            }
            Msg::Retire(path) => {
                let r = crate::widgets::plugins::retire(&path, &user_dir(self));
                let ok = r.is_ok();
                if let Some(p) = self.panel.as_mut() {
                    p.plugins.report =
                        Some(r.map(|t| crate::tr!("Retiré, gardé dans {dir}.", dir = t.display())));
                }
                if ok {
                    Task::done(UiMsg::ConfigChanged)
                } else {
                    Task::none()
                }
            }
        }
    }
}
