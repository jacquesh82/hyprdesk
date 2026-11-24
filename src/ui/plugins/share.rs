//! Paquet intégré « Partage » : exporter son setup (tout, ou le thème seul)
//! dans une archive, ou en importer un. Domaine : `crate::share` (mêmes fonctions que `hyprdesk
//! export` / `hyprdesk import`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{Space, button, column, radio, row, text_input, toggler};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::share::Scope;

pub const INFO: PluginInfo = PluginInfo {
    id: "share",
    label: "Partage",
    icon: "document-send",
    summary: "Exporter son setup (tout, ou le thème seul) ou en importer un.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Share(m)))
}

pub struct State {
    /// Tout le setup, ou le thème seul.
    pub export_scope: Scope,
    pub import_scope: Scope,
    pub export_to: String,
    pub import_from: String,
    pub screens: bool,
    /// Compte rendu du dernier export, de la dernière vérification ou du dernier import.
    pub report: Option<Result<String, String>>,
    /// L'archive à importer a été vérifiée (simulation) : l'import est permis.
    pub checked: bool,
    /// La vérification a trouvé du code Lua (tout le setup) : l'import attend
    /// l'accord explicite de l'utilisateur.
    pub has_code: bool,
    pub trust_code: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            export_scope: Scope::All,
            import_scope: Scope::All,
            export_to: format!(
                "~/hyprdesk-setup-{}.tar.gz",
                chrono::Local::now().format("%Y%m%d")
            ),
            import_from: String::new(),
            screens: false,
            report: None,
            checked: false,
            has_code: false,
            trust_code: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    ExportScope(Scope),
    ImportScope(Scope),
    ExportTo(String),
    ImportFrom(String),
    Screens(bool),
    Export,
    /// Simulation (`dry_run`), puis import réel.
    Check,
    /// Accord pour le code Lua de l'archive.
    TrustCode(bool),
    Import,
    /// Compte rendu ; `bool` : l'archive contient du code Lua.
    Done(Result<String, String>, Op),
    Checked(Result<(String, bool), String>),
}

/// Opération dont vient un compte rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Export,
    Import,
}

impl App {
    pub(in crate::ui) fn share_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        let th = &self.cfg.general.theme;
        let accent = color(th.accent, 1.0);
        let dim = self.ink().dim;
        let warn = crate::ui::tone::warning();
        let s = &p.share;
        let scope_radios = |current: Scope, msg: fn(Scope) -> Msg| {
            row![
                self.focusable(
                    radio(
                        crate::i18n::tr("Tout le setup"),
                        Scope::All,
                        Some(current),
                        move |v| wrap(msg(v))
                    )
                    .font(self.font)
                    .size(crate::ui::tokens::ts(16.0))
                ),
                self.focusable(
                    radio(
                        crate::i18n::tr("Thème seul"),
                        Scope::Theme,
                        Some(current),
                        move |v| wrap(msg(v))
                    )
                    .font(self.font)
                    .size(crate::ui::tokens::ts(16.0))
                ),
            ]
            .spacing(18)
        };
        let theme_name = &self.cfg.general.theme.name;
        let export_help = match s.export_scope {
            Scope::All => crate::i18n::tr(
                "Une archive avec toute la configuration : hyprdesk.lua, les réglages, \
                           hyprland.lua et chaque fichier qu'il charge (raccourcis, clavier, \
                           écrans, démarrage, scripts cités), les plugins tiers, les thèmes personnels, \
                           les calibrations, la place des icônes, les fonds et les images \
                           utilisés — à donner à quelqu'un, ou à garder. Les services d'IA et \
                           leurs clés n'y sont pas.",
            )
            .to_string(),
            Scope::Theme => crate::tr!(
                "Le thème « {theme_name} » seul : couleurs, fonds d'écran, effets de chaque \
                 endroit, mode du bureau, voile et image du verrouillage, transitions, \
                 police, thème d'icônes, modules du thème.",
                theme_name = theme_name
            ),
        };
        let import_help = match s.import_scope {
            Scope::All => crate::i18n::tr(
                "Vérifiez d'abord : rien n'est modifié. À l'import, la configuration \
                           actuelle est sauvegardée dans hyprdesk/backups/ ; les plugins déjà \
                           présents sont gardés. Une archive de thème ne change que le thème.",
            ),
            Scope::Theme => crate::i18n::tr(
                "Seul le thème de l'archive est appliqué (couleurs, fonds, effets…) ; \
                             barre, raccourcis, widgets et plugins ne changent pas. La \
                             configuration actuelle est sauvegardée d'abord.",
            ),
        };
        let export_box = column![
            scope_radios(s.export_scope, Msg::ExportScope),
            text(export_help)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            row![
                text_input(crate::i18n::tr("~/hyprdesk-setup.tar.gz"), &s.export_to)
                    .font(self.font)
                    .on_input(|v| wrap(Msg::ExportTo(v)))
                    .padding(8)
                    .width(Length::Fill),
                self.focusable(
                    button(
                        text("Exporter")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                    )
                    .padding([6, 14])
                    .on_press(wrap(Msg::Export))
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(12);
        let mut col = column![
            self.section("Exporter"),
            export_box,
            self.section("Importer"),
            scope_radios(s.import_scope, Msg::ImportScope),
            text(import_help).font(self.font).size(crate::ui::tokens::ts(12.0)).color(dim),
            if s.import_scope == Scope::All {
                Element::from(
                    self.focusable(toggler(s.screens)
                        .label(crate::i18n::tr(
                            "Appliquer aussi la disposition des écrans de l'archive (monitors.lua, propre à sa machine)",
                        ))
                        .on_toggle(|v| wrap(Msg::Screens(v)))),
                )
            } else {
                Space::new().into()
            },
            row![
                text_input(crate::i18n::tr("archive .tar.gz"), &s.import_from)
                    .font(self.font)
                    .on_input(|v| wrap(Msg::ImportFrom(v)))
                    .padding(8)
                    .width(Length::Fill),
                self.focusable_when(button(text("Vérifier").font(self.font).size(crate::ui::tokens::ts(13.0)))
                    .padding([6, 14])
                    .style(button::secondary)
                    .on_press_maybe((!s.import_from.trim().is_empty()).then(|| wrap(Msg::Check))), !s.import_from.trim().is_empty()),
                self.focusable_when(button(text("Importer").font(self.font).size(crate::ui::tokens::ts(13.0)))
                    .padding([6, 14])
                    .on_press_maybe(
                        (s.checked && (!s.has_code || s.trust_code)).then(|| wrap(Msg::Import))
                    ), s.checked && (!s.has_code || s.trust_code)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(14);
        if let Some(r) = &s.report {
            col = col.push(match r {
                Ok(m) => text(m.clone())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(accent),
                Err(e) => text(e.clone())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(warn),
            });
        }
        if s.checked && s.has_code {
            col = col.push(
                self.focusable(toggler(s.trust_code)
                    .label(crate::i18n::tr(
                        "J'accepte le code Lua de cette archive : il s'exécutera avec mes droits",
                    ))
                    .on_toggle(|v| wrap(Msg::TrustCode(v)))),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn share_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let path = |p: &str| crate::config::lua::expand_path(p.trim(), std::path::Path::new("/"));
        let Some(st) = self.panel.as_mut().map(|p| &mut p.share) else {
            return Task::none();
        };
        match msg {
            Msg::ExportScope(v) => {
                // Le nom proposé suit la portée, s'il n'a pas été changé à la main.
                let (from, to) = match v {
                    Scope::All => ("hyprdesk-theme-", "hyprdesk-setup-"),
                    Scope::Theme => ("hyprdesk-setup-", "hyprdesk-theme-"),
                };
                st.export_to = st.export_to.replace(from, to);
                st.export_scope = v;
                Task::none()
            }
            Msg::ImportScope(v) => {
                st.import_scope = v;
                st.checked = false;
                st.report = None;
                Task::none()
            }
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
                let (out, scope, cfg) = (
                    path(&st.export_to),
                    st.export_scope,
                    self.config_path.clone(),
                );
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::share::export(&cfg, &out, scope).map(|m| match scope {
                                Scope::Theme => crate::tr!(
                                    "Thème « {name} » exporté : {out} — {n} fond(s).",
                                    name = m.theme.map(|t| t.name).unwrap_or_default(),
                                    out = out.display(),
                                    n = m.wallpapers.len()
                                ),
                                Scope::All => crate::tr!(
                                    "Exporté : {out} — {h} fichier(s) de Hyprland, {n} fond(s), {p} plugin(s), {i} image(s).",
                                    out = out.display(),
                                    h = m.hypr.len(),
                                    n = m.wallpapers.len(),
                                    p = m.plugins.len(),
                                    i = m.assets.len()
                                ),
                            })
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |r| wrap(Msg::Done(r, Op::Export)),
                )
            }
            Msg::TrustCode(v) => {
                st.trust_code = v;
                Task::none()
            }
            m @ (Msg::Check | Msg::Import) => {
                let dry = matches!(m, Msg::Check);
                let opts = crate::share::ImportOpts {
                    scope: st.import_scope,
                    screens: st.screens,
                    dry_run: dry,
                    backup: true,
                    trust_code: !dry && st.trust_code,
                };
                let (file, cfg) = (path(&st.import_from), self.config_path.clone());
                let run = move || {
                    crate::share::import(&file, &cfg, opts).map(|r| (r.to_string(), r.has_code()))
                };
                let job = async move {
                    tokio::task::spawn_blocking(run)
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                };
                if dry {
                    Task::perform(job, |r| wrap(Msg::Checked(r)))
                } else {
                    Task::perform(job, |r| wrap(Msg::Done(r.map(|(m, _)| m), Op::Import)))
                }
            }
            Msg::Checked(r) => {
                st.has_code = r.as_ref().is_ok_and(|(_, code)| *code);
                st.trust_code = false;
                st.checked = r.is_ok();
                st.report = Some(r.map(|(m, _)| m));
                Task::none()
            }
            Msg::Done(r, op) => {
                let ok = r.is_ok();
                st.report = Some(r);
                match op {
                    // Import réel : la config a changé sur disque, on la relit.
                    Op::Import => {
                        st.checked = false;
                        st.trust_code = false;
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
