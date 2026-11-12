//! Paquet intégré « À propos » : version de hyprdesk, système, Hyprland,
//! LYRA, fichiers de configuration, dépendances et thème SmartUI, avec de
//! quoi copier ces informations (rapport de bogue) ou ouvrir le dépôt.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{Space, button, column, container, row};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "about",
    label: "À propos",
    icon: "help-about",
    summary: "Version de hyprdesk, système, Hyprland et LYRA, fichiers de configuration.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::About(m)))
}

/// Ce qui se lit hors du fil de l'interface.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub os: String,
    pub kernel: String,
    pub hyprland: String,
    pub lyra: Option<String>,
    pub deps: (usize, usize),
    pub smartui_icons: usize,
}

#[derive(Default)]
pub struct State {
    pub facts: Option<Facts>,
    pub copied: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Facts(Facts),
    Copy,
    Copied,
    OpenRepo,
}

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// Chemin affiché, le dossier personnel abrégé en `~` (rien de personnel
/// dans une capture ou un rapport de bogue).
fn tilde(p: impl AsRef<std::path::Path>) -> String {
    let p = p.as_ref();
    match std::env::var_os("HOME").map(std::path::PathBuf::from) {
        Some(h) if !h.as_os_str().is_empty() => match p.strip_prefix(&h) {
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => p.display().to_string(),
        },
        _ => p.display().to_string(),
    }
}

pub fn load() -> Task<UiMsg> {
    Task::perform(
        async { tokio::task::spawn_blocking(facts).await.unwrap_or_default() },
        |f| wrap(Msg::Facts(f)),
    )
}

fn facts() -> Facts {
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                .map(|v| v.trim_matches('"').to_string())
        })
        .unwrap_or_default();
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let hyprland = hyprpanel_core::ipc::query_json("version")
        .ok()
        .and_then(|v| {
            let tag = v["tag"].as_str().or(v["version"].as_str())?.to_string();
            let commit: String = v["commit"].as_str().unwrap_or("").chars().take(7).collect();
            Some(if commit.is_empty() {
                tag
            } else {
                format!("{tag} ({commit})")
            })
        })
        .unwrap_or_default();
    let list = crate::doctor::cached();
    let deps = (
        list.iter().filter(|s| s.found.is_some()).count(),
        list.len(),
    );
    let smartui_icons = std::fs::read_dir(crate::smartui::theme_dir().join("scalable/apps"))
        .map(|d| d.flatten().count())
        .unwrap_or(0);
    Facts {
        os,
        kernel,
        hyprland,
        lyra: crate::assistant::lyra().map(tilde),
        deps,
        smartui_icons,
    }
}

impl App {
    /// Les informations, en texte (copiées pour un rapport de bogue).
    fn about_text(&self, f: &Facts) -> String {
        let unknown = crate::i18n::tr("inconnu");
        let or = |s: &str| {
            if s.is_empty() {
                unknown.to_string()
            } else {
                s.to_string()
            }
        };
        let settings = self
            .config_path
            .parent()
            .map(|d| d.join("hyprdesk/settings.lua"))
            .unwrap_or_default();
        [
            format!(
                "hyprdesk {} ({})",
                env!("CARGO_PKG_VERSION"),
                if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                }
            ),
            format!("{} : {}", crate::i18n::tr("Système"), or(&f.os)),
            format!("{} : {}", crate::i18n::tr("Noyau"), or(&f.kernel)),
            format!("Hyprland : {}", or(&f.hyprland)),
            format!(
                "LYRA : {}",
                f.lyra
                    .clone()
                    .unwrap_or_else(|| crate::i18n::tr("absente").to_string())
            ),
            format!(
                "{} : {}",
                crate::i18n::tr("Configuration"),
                tilde(&self.config_path)
            ),
            format!("{} : {}", crate::i18n::tr("Réglages"), tilde(&settings)),
            crate::tr!(
                "Dépendances : {n}/{m} présentes",
                n = f.deps.0.to_string(),
                m = f.deps.1.to_string()
            ),
            crate::tr!(
                "Thème SmartUI : {n} icône(s), {dir}",
                n = f.smartui_icons.to_string(),
                dir = tilde(crate::smartui::theme_dir())
            ),
        ]
        .join("\n")
    }

    pub(in crate::ui) fn about_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.6, ..fg };
        let head = column![
            text("hyprdesk").font(self.title_font()).size(28).color(fg),
            text(format!(
                "{} {}",
                crate::i18n::tr("Version"),
                env!("CARGO_PKG_VERSION")
            ))
            .font(self.font)
            .size(14)
            .color(accent),
            text(crate::i18n::tr(env!("CARGO_PKG_DESCRIPTION")))
                .font(self.font)
                .size(13)
                .color(dim),
            text(crate::tr!(
                "Licence {l} · {repo}",
                l = env!("CARGO_PKG_LICENSE").to_string(),
                repo = REPOSITORY.to_string()
            ))
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(6);
        let details: Element<'_, UiMsg> = match &p.about.facts {
            None => text("Lecture des informations…")
                .font(self.font)
                .size(13)
                .color(dim)
                .into(),
            Some(f) => {
                let lines = self.about_text(f);
                container(lines.lines().skip(1).fold(column![].spacing(6), |c, l| {
                    let (k, v) = l.split_once(" : ").unwrap_or((l, ""));
                    c.push(
                        row![
                            text(k.to_string())
                                .font(self.font)
                                .size(13)
                                .color(dim)
                                .width(150),
                            text(v.to_string()).font(self.font).size(13).color(fg),
                        ]
                        .spacing(10),
                    )
                }))
                .padding(12)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    border: iced::Border::default()
                        .rounded(10)
                        .width(1)
                        .color(Color { a: 0.18, ..fg }),
                    ..Default::default()
                })
                .into()
            }
        };
        column![
            head,
            self.section("Ce bureau"),
            details,
            row![
                button(
                    text(if p.about.copied {
                        crate::i18n::tr("Copié ✓")
                    } else {
                        crate::i18n::tr("Copier les informations")
                    })
                    .font(self.font)
                    .size(13)
                )
                .padding([6, 14])
                .on_press_maybe(p.about.facts.is_some().then(|| wrap(Msg::Copy))),
                button(text("Ouvrir le dépôt").font(self.font).size(13))
                    .padding([6, 14])
                    .style(button::secondary)
                    .on_press(wrap(Msg::OpenRepo)),
                Space::new().width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text("Pour un rapport de bogue : copiez ces informations, et joignez « hyprdesk doctor ».")
                .font(self.font)
                .size(12)
                .color(dim),
        ]
        .spacing(14)
        .into()
    }

    pub(in crate::ui) fn about_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Facts(f) => {
                if let Some(p) = self.panel.as_mut() {
                    p.about.facts = Some(f);
                    p.about.copied = false;
                }
                Task::none()
            }
            Msg::Copy => {
                let Some(f) = self.panel.as_ref().and_then(|p| p.about.facts.clone()) else {
                    return Task::none();
                };
                let t = self.about_text(&f);
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            use std::io::Write as _;
                            let mut c = std::process::Command::new("wl-copy")
                                .stdin(std::process::Stdio::piped())
                                .spawn()?;
                            if let Some(mut i) = c.stdin.take() {
                                i.write_all(t.as_bytes())?;
                            }
                            c.wait()
                        })
                        .await;
                    },
                    |_| wrap(Msg::Copied),
                )
            }
            Msg::Copied => {
                if let Some(p) = self.panel.as_mut() {
                    p.about.copied = true;
                }
                Task::none()
            }
            Msg::OpenRepo => {
                let _ = std::process::Command::new("xdg-open")
                    .arg(REPOSITORY)
                    .stdin(std::process::Stdio::null())
                    .spawn();
                Task::none()
            }
        }
    }
}
