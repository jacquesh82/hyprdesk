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
    /// Fichiers Lua de ~/.config/hypr : (nom, généré par hyprdesk).
    pub files: Vec<(String, bool)>,
    /// Lignes de hyprland.lua reprises par hyprdesk (`-- [hyprdesk]`).
    pub adopted: usize,
    /// Historique : nombre d'instantanés, et le plus récent (date, motif).
    pub history: (usize, Option<String>),
}

#[derive(Default)]
pub struct State {
    pub facts: Option<Facts>,
    pub copied: bool,
    /// « Configuration par défaut » cliqué une fois : attend la confirmation.
    pub confirm_reset: bool,
    /// Compte rendu de la remise par défaut.
    pub reset: Option<Result<String, String>>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Facts(Facts),
    Copy,
    Copied,
    OpenRepo,
    /// Ouvre ~/.config/hypr dans le gestionnaire de fichiers.
    OpenConfigDir,
    /// Va à Configuration → Historique.
    History,
    /// Remise à la configuration par défaut (deux clics).
    Reset,
    ResetCancel,
    ResetDone(Result<String, String>),
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

pub fn load(config_path: std::path::PathBuf) -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(move || facts(&config_path))
                .await
                .unwrap_or_default()
        },
        |f| wrap(Msg::Facts(f)),
    )
}

fn facts(config_path: &std::path::Path) -> Facts {
    let dir = config_path.parent().unwrap_or(std::path::Path::new("."));
    let mut files: Vec<(String, bool)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "lua"))
        .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.lua"))
        .map(|e| {
            let generated =
                std::fs::read_to_string(e.path()).is_ok_and(|s| crate::hyprsrc::generated(&s));
            (e.file_name().to_string_lossy().to_string(), generated)
        })
        .collect();
    files.sort_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)));
    let adopted = std::fs::read_to_string(dir.join("hyprland.lua"))
        .map(|s| {
            s.lines()
                .filter(|l| l.trim_start().starts_with(crate::adopt::MARK.trim_end()))
                .count()
        })
        .unwrap_or(0);
    let snaps = crate::history::list();
    let history = (
        snaps.len(),
        snaps.first().map(|s| {
            format!(
                "{} · {}",
                s.time().format("%d/%m %H:%M"),
                crate::history::reason_label(&s.reason)
            )
        }),
    );
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
        files,
        adopted,
        history,
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
            crate::tr!(
                "Rendu : {r}",
                r = crate::render::running_summary()
                    .unwrap_or_else(|| crate::i18n::tr("pas encore relevé").to_string())
            ),
            crate::tr!(
                "hyprland.lua : {n} ligne(s) reprise(s) par hyprdesk",
                n = f.adopted.to_string()
            ),
            crate::tr!(
                "Historique : {n} instantané(s){last}",
                n = f.history.0.to_string(),
                last = f
                    .history
                    .1
                    .as_ref()
                    .map(|l| crate::tr!(", le dernier : {l}", l = l))
                    .unwrap_or_default()
            ),
        ]
        .join("\n")
    }

    pub(in crate::ui) fn about_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        let th = &self.cfg.general.theme;
        // Texte accentué : l'encre garantie à 4,5:1 (HIG FON-05).
        let (fg, accent) = (color(th.fg, 1.0), self.ink().accent);
        let dim = self.ink().dim;
        let head = column![
            text("hyprdesk")
                .font(self.title_font())
                .size(crate::ui::tokens::ts(28.0))
                .color(fg),
            text(format!(
                "{} {}",
                crate::i18n::tr("Version"),
                env!("CARGO_PKG_VERSION")
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(14.0))
            .color(accent),
            text(crate::i18n::tr(env!("CARGO_PKG_DESCRIPTION")))
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .color(dim),
            text(crate::tr!(
                "Licence {l} · {repo}",
                l = env!("CARGO_PKG_LICENSE").to_string(),
                repo = REPOSITORY.to_string()
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(6);
        let details: Element<'_, UiMsg> = match &p.about.facts {
            None => text("Lecture des informations…")
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
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
                                .size(crate::ui::tokens::ts(13.0))
                                .color(dim)
                                .width(150),
                            text(v.to_string())
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                                .color(fg),
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
                    .size(crate::ui::tokens::ts(13.0))
                )
                .padding([6, 14])
                .on_press_maybe(p.about.facts.is_some().then(|| wrap(Msg::Copy))),
                button(text("Ouvrir le dépôt").font(self.font).size(crate::ui::tokens::ts(13.0)))
                    .padding([6, 14])
                    .style(button::secondary)
                    .on_press(wrap(Msg::OpenRepo)),
                Space::new().width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text("Pour un rapport de bogue : copiez ces informations, et joignez « hyprdesk doctor ».")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            self.section("Fichiers de configuration"),
            text("Dans ~/.config/hypr : les fichiers générés par hyprdesk sont réécrits par les Réglages (ne pas les éditer) ; hyprland.lua et les autres restent à vous. Chaque changement est gardé 30 jours dans l'historique.")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            self.about_files(p),
            row![
                button(text("Ouvrir le dossier").font(self.font).size(crate::ui::tokens::ts(13.0)))
                    .padding([6, 14])
                    .style(button::secondary)
                    .on_press(wrap(Msg::OpenConfigDir)),
                button(text("Historique…").font(self.font).size(crate::ui::tokens::ts(13.0)))
                    .padding([6, 14])
                    .style(button::secondary)
                    .on_press(wrap(Msg::History)),
            ]
            .spacing(10),
            self.section("Nouveautés"),
            self.about_news(),
            self.section("Configuration par défaut"),
            text("Remet chaque composant à sa configuration par défaut : les réglages faits ici (bureau de départ), les fichiers de Hyprland que tient hyprdesk (raccourcis réactivés, clavier et souris aux valeurs de Hyprland en gardant la disposition, écrans en mode préféré, démarrage vide), l'état mémorisé (positions des icônes, places des fenêtres, lancements, notifications). Vos thèmes et widgets perso sont mis de côté. hyprdesk.lua n'est pas touché. Une copie est gardée dans ~/.config/hypr/hyprdesk/sauvegardes. Chaque page a aussi son bouton « Par défaut… ».")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            if p.about.confirm_reset {
                Element::from(
                    row![
                        button(text("Confirmer : tout remettre par défaut").font(self.font).size(crate::ui::tokens::ts(13.0)))
                            .padding([6, 14])
                            .style(button::danger)
                            .on_press(wrap(Msg::Reset)),
                        button(text("Annuler").font(self.font).size(crate::ui::tokens::ts(13.0)))
                            .padding([6, 14])
                            .style(button::secondary)
                            .on_press(wrap(Msg::ResetCancel)),
                    ]
                    .spacing(10),
                )
            } else {
                button(text("Revenir à la configuration par défaut…").font(self.font).size(crate::ui::tokens::ts(13.0)))
                    .padding([6, 14])
                    .style(button::secondary)
                    .on_press(wrap(Msg::Reset))
                    .into()
            },
            match &p.about.reset {
                Some(Ok(m)) => Element::from(text(m.clone()).font(self.font).size(crate::ui::tokens::ts(12.0)).color(accent)),
                Some(Err(e)) => text(e.clone())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(crate::ui::tone::warning())
                    .into(),
                None => Space::new().into(),
            },
        ]
        .spacing(14)
        .into()
    }

    /// Fichiers Lua de ~/.config/hypr, généré par hyprdesk ou à vous.
    fn about_files(&self, p: &super::super::panel::Panel) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = self.ink().dim;
        let accent = self.ink().accent;
        let Some(f) = &p.about.facts else {
            return Space::new().into();
        };
        let col = f
            .files
            .iter()
            .fold(column![].spacing(4), |c, (name, generated)| {
                c.push(
                    row![
                        text(name.clone())
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                            .color(fg)
                            .width(170),
                        text(if *generated {
                            crate::i18n::tr("généré par hyprdesk")
                        } else {
                            crate::i18n::tr("à vous")
                        })
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(if *generated { accent } else { dim }),
                    ]
                    .spacing(10),
                )
            });
        container(col)
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

    /// Ce qu'apporte cette version (en bref ; le détail : CHANGELOG.md).
    fn about_news(&self) -> Element<'_, UiMsg> {
        const NEWS: &[&str] = &[
            "Réglages de Hyprland écrits dans des fichiers générés (appearance.lua, animations.lua, layouts.lua, rules.lua, env.lua, system.lua), et hyprland.lua repris : il ne dit plus autre chose que ce qui tourne.",
            "Historique de la configuration sur 30 jours, avec retour arrière (Configuration → Historique).",
            "Fenêtres : espacement, bordures (aux couleurs du thème si vous voulez), règles par application, groupes à onglets.",
            "Clavier et souris : curseur, gestes ; Animations : vos courbes, dessinées et éditables.",
            "Écrans : glisser-déposer sur le plan avec aimant, réglage au pixel, repères d'alignement avec Identifier.",
            "Performance : moteur de rendu matériel ou logiciel, carte graphique, fils ; moins de processus et de réveils au repos.",
            "Réglages réorganisés par sujet ; les réglages avancés sont marqués en cyan.",
        ];
        let dim = self.ink().dim;
        NEWS.iter()
            .fold(column![].spacing(6), |c, n| {
                c.push(
                    row![
                        text("•")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                            .color(dim),
                        text(crate::i18n::tr(n))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0)),
                    ]
                    .spacing(8),
                )
            })
            .into()
    }

    pub(in crate::ui) fn about_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::OpenConfigDir => {
                let dir = self
                    .config_path
                    .parent()
                    .map(std::path::Path::to_path_buf)
                    .unwrap_or_default();
                let _ = crate::launch::open_path(&dir);
                Task::none()
            }
            Msg::History => self.panel_update(super::super::panel::PanelMsg::Tab("history")),
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
            Msg::Reset => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                if !p.about.confirm_reset {
                    p.about.confirm_reset = true;
                    p.about.reset = None;
                    return Task::none();
                }
                p.about.confirm_reset = false;
                Task::perform(
                    crate::ui::panel::defaults_job(self.config_path.clone(), None),
                    |r| wrap(Msg::ResetDone(r)),
                )
            }
            Msg::ResetCancel => {
                if let Some(p) = self.panel.as_mut() {
                    p.about.confirm_reset = false;
                }
                Task::none()
            }
            Msg::ResetDone(r) => {
                let ok = r.is_ok();
                if let Some(p) = self.panel.as_mut() {
                    p.about.reset = Some(r);
                }
                // État et configuration relus : l'interface repart des défauts.
                if ok {
                    self.after_defaults("all")
                } else {
                    Task::done(UiMsg::ConfigChanged)
                }
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
