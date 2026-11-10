//! Paquet intégré « Partage » : exporter son setup dans une archive, ou en
//! importer un. Domaine : `crate::share` (mêmes fonctions que `hyprdesk
//! export` / `hyprdesk import`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use iced::widget::{button, column, row, text, text_input, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "share",
    label: "Partage",
    icon: "document-send",
    summary: "Exporter son setup (configs, plugins, fonds d'écran) ou en importer un.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Share(m)))
}

pub struct State {
    pub export_to: String,
    pub import_from: String,
    pub screens: bool,
    /// Compte rendu du dernier export, de la dernière vérification ou du dernier import.
    pub report: Option<Result<String, String>>,
    /// L'archive à importer a été vérifiée (simulation) : l'import est permis.
    pub checked: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            export_to: format!(
                "~/hyprdesk-setup-{}.tar.gz",
                chrono::Local::now().format("%Y%m%d")
            ),
            import_from: String::new(),
            screens: false,
            report: None,
            checked: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    ExportTo(String),
    ImportFrom(String),
    Screens(bool),
    Export,
    /// Simulation (`dry_run`), puis import réel.
    Check,
    Import,
    Done(Result<String, String>, Op),
}

/// Opération dont vient un compte rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Export,
    Check,
    Import,
}

impl App {
    pub(in crate::ui) fn share_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let s = &p.share;
        let mut col = column![
            self.section("Exporter mon setup"),
            text(
                "Une archive avec hyprdesk.lua, les réglages, les plugins tiers et les fonds \
                 d'écran utilisés — à donner à quelqu'un, ou à garder."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            row![
                text_input("~/hyprdesk-setup.tar.gz", &s.export_to)
                    .font(self.font)
                    .on_input(|v| wrap(Msg::ExportTo(v)))
                    .padding(8)
                    .width(Length::Fill),
                button(text("Exporter").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press(wrap(Msg::Export)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            toggler(s.screens)
                .label("Inclure la disposition des écrans (monitors.lua, propre à cette machine)")
                .on_toggle(|v| wrap(Msg::Screens(v))),
            self.section("Importer un setup"),
            text(
                "Vérifiez d'abord : rien n'est modifié. À l'import, la configuration actuelle est \
                 sauvegardée dans hyprdesk/backups/ ; les plugins déjà présents sont gardés."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            row![
                text_input("archive .tar.gz", &s.import_from)
                    .font(self.font)
                    .on_input(|v| wrap(Msg::ImportFrom(v)))
                    .padding(8)
                    .width(Length::Fill),
                button(text("Vérifier").font(self.font).size(13))
                    .padding([6, 14])
                    .style(button::secondary)
                    .on_press_maybe((!s.import_from.trim().is_empty()).then(|| wrap(Msg::Check))),
                button(text("Importer").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press_maybe(s.checked.then(|| wrap(Msg::Import))),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(14);
        if let Some(r) = &s.report {
            col = col.push(match r {
                Ok(m) => text(m.clone()).font(self.font).size(12).color(accent),
                Err(e) => text(e.clone()).font(self.font).size(12).color(warn),
            });
        }
        col.into()
    }

    pub(in crate::ui) fn share_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let path = |p: &str| crate::config::lua::expand_path(p.trim(), std::path::Path::new("/"));
        let Some(st) = self.panel.as_mut().map(|p| &mut p.share) else {
            return Task::none();
        };
        match msg {
            Msg::ExportTo(v) => {
                st.export_to = v;
                Task::none()
            }
            Msg::ImportFrom(v) => {
                st.import_from = v;
                st.checked = false;
                st.report = None;
                Task::none()
            }
            Msg::Screens(v) => {
                st.screens = v;
                st.checked = false;
                Task::none()
            }
            Msg::Export => {
                let (out, screens, cfg) =
                    (path(&st.export_to), st.screens, self.config_path.clone());
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::share::export(&cfg, &out, screens).map(|m| {
                                format!(
                                    "Exporté : {} — {} fond(s), {} plugin(s).",
                                    out.display(),
                                    m.wallpapers.len(),
                                    m.plugins.len()
                                )
                            })
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |r| wrap(Msg::Done(r, Op::Export)),
                )
            }
            m @ (Msg::Check | Msg::Import) => {
                let dry = matches!(m, Msg::Check);
                let (file, screens, cfg) =
                    (path(&st.import_from), st.screens, self.config_path.clone());
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::share::import(&file, &cfg, screens, dry).map(|r| r.to_string())
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    move |r| wrap(Msg::Done(r, if dry { Op::Check } else { Op::Import })),
                )
            }
            Msg::Done(r, op) => {
                let ok = r.is_ok();
                st.report = Some(r);
                match op {
                    Op::Check => {
                        st.checked = ok;
                        Task::none()
                    }
                    // Import réel : la config a changé sur disque, on la relit.
                    Op::Import => {
                        st.checked = false;
                        if ok {
                            Task::done(UiMsg::ConfigChanged)
                        } else {
                            Task::none()
                        }
                    }
                    Op::Export => Task::none(),
                }
            }
        }
    }
}
