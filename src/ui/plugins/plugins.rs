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
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let badge = move |label: &'static str, on: bool| {
            container(
                text(label)
                    .font(self.font)
                    .size(11)
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
                head = head.push(text("en erreur").font(self.font).size(11).color(warn));
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
            if t.toggleable || active {
                let id = t.id.clone();
                line = line.push(toggler(active).on_toggle(move |v| {
                    super::widgets::wrap(super::widgets::Msg::WidgetToggle(id.clone(), v))
                }));
            }
            lua = lua.push(line);
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

    pub(in crate::ui) fn plugins_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
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
