//! Paquet intégré « Bureau » : Où et comment le bureau s'affiche, typographie, session.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{Space, button, column, container, pick_list, radio, row, slider, toggler};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::{Mode, ShowOn};

pub const INFO: PluginInfo = PluginInfo {
    id: "desktop",
    label: "Bureau",
    icon: "user-desktop",
    summary: "Où et comment le bureau s'affiche, typographie, session.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
/// Onglets du paquet (police et taille : Thème → Texte).
pub const TABS: &[super::SubTab] = &[
    ("affichage", "Affichage"),
    ("modes", "Modes imposés"),
    ("session", "Session"),
    ("dependances", "Dépendances"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Desktop(m)))
}

/// État propre à ce paquet, porté par le tiroir des réglages.
#[derive(Default)]
pub struct State {
    pub fonts: Vec<String>,
    /// « Prendre la main » cliqué une fois : le pid à arrêter, à confirmer.
    pub takeover: Option<u32>,
    /// Places de fenêtres choisies par l'utilisateur en plein bureau.
    pub window_places: usize,
    /// Hyprbars : chemin trouvé (relevé à l'arrivée sur la page).
    pub hyprbars: Option<Option<std::path::PathBuf>>,
}

/// Hyprbars installé ? (relevé hors du fil de l'interface)
pub fn load_hyprbars(custom: String) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || crate::hyprbars::find(&custom))
                .await
                .unwrap_or(None)
        },
        |p| wrap(Msg::HyprbarsFound(p)),
    )
}

/// Nombre de places de fenêtres choisies (lu hors du fil de l'interface).
pub fn load_places() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(crate::windows::places_count)
                .await
                .unwrap_or(0)
        },
        |n| wrap(Msg::WindowPlaces(n)),
    )
}

#[derive(Debug, Clone)]
pub enum Msg {
    WindowPlaces(usize),
    Hyprbars(bool),
    HyprbarsFound(Option<std::path::PathBuf>),
    /// Oublier les places de fenêtres choisies.
    ForgetWindowPlaces,
    WindowPlacesForgotten(Result<(), String>),
    /// Langue de l'interface : `auto`, `fr`, `en`.
    Language(&'static str),
    ShowOn(ShowOn),
    Mode(Mode),
    GlassOpacity(f32),
    WorkspaceMode(WsMode),
    Font(String),
    FontSize(f32),
    Fonts(Vec<String>),
    ClipboardHistory(bool),
    Autostart(bool),
    AutostartEntry(String, bool),
    DesktopFiles(bool),
    ComponentRestart(String),
    FloatLaunched(bool),
    /// Agent polkit concurrent : « Prendre la main » (pid), puis confirmation.
    TakeOver(u32),
    /// Mode imposé : écran (`DP-1`) ou workspace (`ws:5`) → mode, ou libre.
    Forced(String, Option<&'static str>),
}

/// Choix d'un mode imposé (`None` : libre).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForcedChoice(pub Option<&'static str>);

impl std::fmt::Display for ForcedChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            None => crate::i18n::tr("Libre (mode global)"),
            Some(m) => m,
        })
    }
}

const FORCED_CHOICES: [ForcedChoice; 4] = [
    ForcedChoice(None),
    ForcedChoice(Some("no-desktop")),
    ForcedChoice(Some("launcher-only")),
    ForcedChoice(Some("full-desktop")),
];

/// Mode d'un workspace : par défaut, ou imposé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsMode {
    Default,
    Opaque,
    Glass,
}

impl App {
    pub(in crate::ui) fn tab_desktop(&self, ws: &str) -> Element<'_, UiMsg> {
        let p_places = self.panel.as_ref().map_or(0, |p| p.desktop.window_places);
        let hyprbars_found = self.panel.as_ref().and_then(|p| p.desktop.hyprbars.clone());
        let g = &self.cfg.general;
        let ws_mode = match self.settings.workspace_mode(ws) {
            None => WsMode::Default,
            Some(Mode::Opaque) => WsMode::Opaque,
            Some(Mode::Glass) => WsMode::Glass,
        };
        if self.on_subtab(INFO.id, "session") {
            return self.session_section();
        }
        if self.on_subtab(INFO.id, "modes") {
            return self.forced_section();
        }
        if self.on_subtab(INFO.id, "dependances") {
            return self.deps_section();
        }
        let lang: &'static str = match g.language.as_str() {
            "fr" => "fr",
            "en" => "en",
            _ => "auto",
        };
        column![
            self.section("Langue"),
            self.field(
                "Langue de l'interface",
                "language",
                row![
                    radio(
                        crate::i18n::tr("Celle du système"),
                        "auto",
                        Some(lang),
                        |v| wrap(Msg::Language(v))
                    )
                    .font(self.font),
                    radio("Français", "fr", Some(lang), |v| wrap(
                        Msg::Language(v)
                    ))
                    .font(self.font),
                    radio("English", "en", Some(lang), |v| wrap(
                        Msg::Language(v)
                    ))
                    .font(self.font),
                ]
                .spacing(16),
            ),
            self.section("Affichage"),
            self.field(
                "Afficher le bureau sur",
                "show_on",
                column![
                    radio(crate::i18n::tr("Tous les écrans"), ShowOn::All, Some(g.show_on), |v| wrap(Msg::ShowOn(v))).font(self.font),
                    radio(crate::i18n::tr("L'écran actif seulement"), ShowOn::Focused, Some(g.show_on), |v| wrap(Msg::ShowOn(v))).font(self.font),
                    radio(crate::i18n::tr("L'écran principal seulement"), ShowOn::Primary, Some(g.show_on), |v| wrap(Msg::ShowOn(v))).font(self.font),
                ]
                .spacing(8),
            ),
            self.field(
                "Fichiers du dossier Bureau et Corbeille",
                "desktop_files",
                toggler(g.desktop_files).on_toggle(|v| wrap(Msg::DesktopFiles(v))),
            ),
            self.field(
                "Bureau resté affiché : fenêtres ouvertes flottantes et centrées",
                "float_launched",
                toggler(g.float_launched).on_toggle(|v| wrap(Msg::FloatLaunched(v))),
            ),
            text("Bureau affiché, les fenêtres du workspace flottent, organisées sur l'écran ; une fenêtre que vous déplacez garde sa place (par workspace et par application) et n'est plus réorganisée. Elles reprennent leur disposition quand on masque le bureau.").font(self.font).size(12),
            row![
                text(match p_places {
                    0 => crate::i18n::tr("Aucune place de fenêtre choisie pour l'instant.").to_string(),
                    n => crate::tr!("{n} place(s) de fenêtre choisie(s) par vous.", n = n),
                })
                .font(self.font)
                .size(12)
                .width(Length::Fill),
                button(text("Oublier ces places").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press_maybe((p_places > 0).then(|| wrap(Msg::ForgetWindowPlaces))),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            self.field(
                "Barres de titre Hyprbars en plein bureau (plugin Hyprland)",
                "hyprbars",
                toggler(g.hyprbars).on_toggle(|v| wrap(Msg::Hyprbars(v))),
            ),
            text(match hyprbars_found {
                None => crate::i18n::tr("Recherche du plugin…").to_string(),
                Some(Some(p)) => crate::tr!("Chargé en entrant en plein bureau, déchargé en sortant ({p}).", p = p.display()),
                Some(None) => crate::i18n::tr("Hyprbars n'est pas installé : ./scripts/install.sh l'installe (étape Hyprbars), ou à la main hyprpm update puis hyprpm add https://github.com/hyprwm/hyprland-plugins (ou réglez hyprbars_path).").to_string(),
            })
            .font(self.font)
            .size(12),
            self.section("Mode"),
            self.field(
                "Mode par défaut",
                "mode",
                column![
                    radio(crate::i18n::tr("Opaque — le fond d'écran net"), Mode::Opaque, Some(g.mode), |v| wrap(Msg::Mode(v))).font(self.font),
                    radio(crate::i18n::tr("Verre — fond d'écran flouté et teinté"), Mode::Glass, Some(g.mode), |v| wrap(Msg::Mode(v))).font(self.font),
                ]
                .spacing(8),
            ),
            self.field(
                "Mode de ce workspace",
                "",
                row![
                    radio(crate::i18n::tr("Par défaut"), WsMode::Default, Some(ws_mode), |v| wrap(Msg::WorkspaceMode(v))).font(self.font),
                    radio(crate::i18n::tr("Opaque"), WsMode::Opaque, Some(ws_mode), |v| wrap(Msg::WorkspaceMode(v))).font(self.font),
                    radio(crate::i18n::tr("Verre"), WsMode::Glass, Some(ws_mode), |v| wrap(Msg::WorkspaceMode(v))).font(self.font),
                ]
                .spacing(16),
            ),
            self.adv(self.field(
                "Opacité du voile (verre)",
                "glass_opacity",
                row![
                    slider(0.0..=1.0, g.glass_opacity, |v| wrap(Msg::GlassOpacity(v)))
                        .step(0.05f32)
                        .on_release(UiMsg::Panel(PanelMsg::Commit))
                        .width(260),
                    text(format!("{:.0} %", g.glass_opacity * 100.0)).font(self.font).size(14),
                ]
                .spacing(12),
            )),
        ]
        .spacing(14)
        .into()
    }
    /// Police et taille du texte de l'interface (affichées dans Thème → Texte).
    pub(in crate::ui) fn typography(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        column![
            self.section("Typographie"),
            self.field(
                "Police de l’interface",
                "font",
                pick_list(
                    self.panel
                        .as_ref()
                        .map(|p| p.desktop.fonts.clone())
                        .unwrap_or_default(),
                    g.font.clone().or_else(|| Some(crate::fonts::FAMILY.into())),
                    |f| wrap(Msg::Font(f)),
                )
                .font(self.font)
                .width(Length::Fill)
            ),
            container(
                column![
                    text("Le bureau, à votre rythme.")
                        .font(self.font)
                        .size(24)
                        .font(self.title_font()),
                    text("Aa Bb Cc · 0123456789 · Éàçœ")
                        .font(self.font)
                        .size(g.font_size)
                        .font(self.font),
                ]
                .spacing(8)
            )
            .padding(16)
            .width(Length::Fill)
            .style(|theme| super::super::design::surface(
                theme,
                super::super::design::Surface::Inset
            )),
            self.field(
                "Taille du texte",
                "font_size",
                row![
                    slider(11.0..=22.0, g.font_size, |v| wrap(Msg::FontSize(v)))
                        .step(0.5_f32)
                        .on_release(UiMsg::Panel(PanelMsg::Commit))
                        .width(260),
                    text(format!("{} px", g.font_size)).font(self.font).size(13),
                ]
                .spacing(12)
            ),
        ]
        .spacing(14)
        .into()
    }

    /// Composants de session supervisés par hyprdesk.
    /// Modes imposés : par écran, puis par workspace.
    pub(in crate::ui) fn forced_section(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let dim = self.ink().dim;
        let current = |key: &str| {
            let v = g.forced_modes.get(key).map(String::as_str);
            FORCED_CHOICES
                .iter()
                .copied()
                .find(|c| c.0 == v)
                .unwrap_or(ForcedChoice(None))
        };
        let pick = |key: String| {
            let k2 = key.clone();
            pick_list(
                FORCED_CHOICES,
                Some(current(&key)),
                move |c: ForcedChoice| wrap(Msg::Forced(k2.clone(), c.0)),
            )
            .font(self.font)
            .text_size(13)
            .width(220)
        };
        let mut col = column![
            self.section("Modes imposés"),
            text(
                "Un écran ou un workspace peut garder son propre mode. no-desktop : le bureau \
                 n'y apparaît jamais. launcher-only ou full-desktop : en y arrivant, le bureau \
                 passe dans ce mode, et reprend le précédent en le quittant. La règle d'un \
                 workspace l'emporte sur celle de l'écran."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.section("Écrans"),
        ]
        .spacing(10);
        let mut screens: Vec<String> = self.monitors.iter().map(|m| m.name.clone()).collect();
        screens.extend(
            g.forced_modes
                .keys()
                .filter(|k| !k.starts_with("ws:") && !screens.contains(k))
                .cloned()
                .collect::<Vec<_>>(),
        );
        for name in screens {
            col = col.push(
                row![
                    text(name.clone())
                        .font(self.font)
                        .size(14)
                        .width(Length::Fill),
                    pick(name)
                ]
                .align_y(Alignment::Center),
            );
        }
        col = col.push(self.section("Workspaces"));
        let n = g.workspaces.unwrap_or(10).max(1);
        let mut ws: Vec<String> = (1..=n).map(|w| w.to_string()).collect();
        for k in g.forced_modes.keys().filter_map(|k| k.strip_prefix("ws:")) {
            if !ws.iter().any(|w| w == k) {
                ws.push(k.to_string());
            }
        }
        for w in ws {
            col = col.push(
                row![
                    text(crate::tr!("Workspace {w}", w = w))
                        .font(self.font)
                        .size(14)
                        .width(Length::Fill),
                    pick(format!("ws:{w}"))
                ]
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    /// Démarrage automatique XDG (Réglages → Démarrage automatique → Applications).
    pub(in crate::ui) fn xdg_autostart_section(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = self.ink().fg;
        let dim = self.ink().dim;
        let mut col = column![].spacing(10);
        col = col.push(self.field(
            "Démarrage automatique des applications (XDG)",
            "autostart",
            toggler(g.autostart).on_toggle(|v| wrap(Msg::Autostart(v))),
        ));
        col = col.push(
            text("Lance une fois par session les applications de ~/.config/autostart et /etc/xdg/autostart, sauf celles qui tournent déjà. À laisser coupé si hyprland.lua les lance déjà.")
                .font(self.font)
                .size(12)
                .color(dim),
        );
        if self.advanced() || g.autostart {
            for e in crate::autostart::entries_cached() {
                use crate::autostart::Status;
                let (state, toggle) = match e.status {
                    Status::Run => (crate::i18n::tr("lancée au démarrage"), Some(false)),
                    Status::Running => (crate::i18n::tr("déjà en cours"), Some(false)),
                    Status::Disabled => (crate::i18n::tr("désactivée"), Some(true)),
                    Status::OtherDesktop => (crate::i18n::tr("réservée à un autre bureau"), None),
                    Status::Missing => (crate::i18n::tr("programme absent"), None),
                };
                let mut line = row![
                    column![
                        text(e.name.clone()).font(self.font).size(13).color(fg),
                        text(format!("{state} — {}", e.exec))
                            .font(self.font)
                            .size(12)
                            .color(dim)
                    ]
                    .width(Length::Fill),
                ]
                .spacing(8)
                .align_y(Alignment::Center);
                if let Some(enable) = toggle {
                    line = line.push(
                        button(
                            text(if enable { "Activer" } else { "Désactiver" })
                                .font(self.font)
                                .size(12),
                        )
                        .padding([4, 10])
                        .on_press(wrap(Msg::AutostartEntry(e.id.clone(), enable))),
                    );
                }
                col = col.push(line);
            }
        }
        col.into()
    }

    pub(in crate::ui) fn session_section(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let mut col = column![self.section("Session")].spacing(10);
        col = col.push(
            text("Composants lancés et supervisés par hyprdesk (relancés s'ils s'arrêtent, arrêtés avec lui). À déclarer dans hyprdesk.lua : hd.component(\"nom\", { exec = \"commande\" }).").font(self.font)
                .size(12)
                .color(dim),
        );
        col = col.push(self.field(
            "Historique du presse-papiers (cliphist)",
            "clipboard_history",
            toggler(g.clipboard_history).on_toggle(|v| wrap(Msg::ClipboardHistory(v))),
        ));
        col = col.push(
            text(if crate::clipboard::available() {
                "Chaque copie est gardée par cliphist. Dans la recherche, tapez « :clip » puis quelques lettres, et Entrée remet l'élément dans le presse-papiers."
            } else {
                "cliphist et wl-clipboard ne sont pas installés (voir Dépendances)."
            })
            .font(self.font)
            .size(12)
            .color(dim),
        );
        col = col.push(
            text("Démarrage automatique (hyprland.lua, applications XDG) : Réglages → Démarrage automatique.")
                .font(self.font)
                .size(12)
                .color(dim),
        );
        let comps = crate::session::snapshot();
        if comps.is_empty() {
            col = col.push(text("Aucun composant.").font(self.font).size(12).color(dim));
        }
        for (d, st) in comps {
            let state = match (st.pid, st.given_up) {
                (Some(pid), _) => crate::tr!("actif (pid {pid})", pid = pid),
                (None, true) => crate::i18n::tr("abandonné (plantages)").to_string(),
                (None, false) => crate::i18n::tr("arrêté").to_string(),
            };
            let extra = if st.restarts > 0 {
                crate::tr!(" · {n} relance(s)", n = st.restarts)
            } else {
                String::new()
            };
            col = col.push(
                row![
                    column![
                        text(d.name.clone()).font(self.font).size(14).color(fg),
                        text(format!("{} — {state}{extra}", d.exec))
                            .font(self.font)
                            .size(12)
                            .color(dim)
                    ]
                    .width(Length::Fill),
                    button(text("Relancer").font(self.font).size(12))
                        .padding([4, 10])
                        .on_press(wrap(Msg::ComponentRestart(d.name.clone()))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    /// Dépendances externes (même relevé que `hyprdesk doctor`).
    pub(in crate::ui) fn deps_section(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let list = crate::doctor::cached();
        let present = list.iter().filter(|s| s.found.is_some()).count();
        let mut col = column![].spacing(10);
        // Services qu'un autre programme tient déjà : LYRA peut aider à
        // laisser la main à hyprdesk.
        let conflicts = crate::takeover::cached(self.toasts.active, g.clipboard_history);
        if !conflicts.is_empty() {
            col = col.push(self.section("Tenu par un autre programme"));
            col = col.push(
                text("Ces fonctions restent assurées par un programme lancé par votre configuration ; hyprdesk ne prend pas sa place sans vous. Le bouton IA demande à LYRA de trouver d'où il est lancé et de proposer la modification.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
            for c in conflicts {
                let origin = crate::i18n::tr(c.feature).to_string();
                // Agent polkit : il peut être arrêté d'ici ; hyprdesk
                // s'enregistre alors dans les 5 s (nouvel essai périodique).
                let take: Element<'_, UiMsg> =
                    if c.feature.starts_with("Authentification administrateur") {
                        let confirm =
                            self.panel.as_ref().and_then(|p| p.desktop.takeover) == Some(c.pid);
                        button(
                            text(if confirm {
                                crate::tr!("Confirmer : arrêter {h}", h = c.holder)
                            } else {
                                crate::i18n::tr("Prendre la main").to_string()
                            })
                            .font(self.font)
                            .size(12),
                        )
                        .padding([4, 10])
                        .style(if confirm {
                            button::danger
                        } else {
                            button::secondary
                        })
                        .on_press(wrap(Msg::TakeOver(c.pid)))
                        .into()
                    } else {
                        Space::new().into()
                    };
                col = col.push(
                    row![
                        column![
                            text(origin.clone()).font(self.font).size(14).color(fg),
                            text(crate::tr!(
                                "tenu par {h} (pid {pid})",
                                h = c.holder,
                                pid = c.pid
                            ))
                            .font(self.font)
                            .size(12)
                            .color(dim),
                        ]
                        .width(Length::Fill),
                        take,
                        self.lyra_button(&origin, c.prompt()),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
            }
        }
        col = col.push(self.section("Dépendances"));
        col = col.push(
            text("Outils auxquels hyprdesk confie ce qui ne se voit pas. Un outil absent rend seulement sa fonction inopérante ; hyprdesk doctor donne le même relevé.")
                .font(self.font)
                .size(12)
                .color(dim),
        );
        col = col.push(
            text(crate::tr!("{n}/{m} présentes", n = present, m = list.len()))
                .font(self.font)
                .size(13)
                .color(fg),
        );
        for s in list {
            let (mark, mark_color) = match (&s.found, s.dep.essential) {
                (Some(_), _) => ("✓", fg),
                (None, true) => ("✗", color(g.theme.accent, 1.0)),
                (None, false) => ("✗", dim),
            };
            let detail = match &s.found {
                Some(found) => found.clone(),
                None => crate::tr!("absent — paquet : {p}", p = s.dep.package),
            };
            col = col.push(
                row![
                    text(mark).font(self.font).size(14).color(mark_color),
                    column![
                        text(s.dep.name).font(self.font).size(14).color(fg),
                        text(format!("{} · {detail}", crate::i18n::tr(s.dep.feature)))
                            .font(self.font)
                            .size(12)
                            .color(dim),
                    ]
                    .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn desktop_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::ShowOn(v) => {
                self.settings.show_on = Some(v);
                self.commit()
            }
            Msg::Mode(v) => {
                self.settings.mode = Some(v);
                self.commit()
            }
            Msg::GlassOpacity(v) => {
                self.settings.glass_opacity = Some(v);
                self.apply()
            }
            Msg::Hyprbars(v) => {
                self.settings.hyprbars = Some(v);
                self.commit()
            }
            Msg::HyprbarsFound(p) => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.desktop.hyprbars = Some(p);
                }
                Task::none()
            }
            Msg::WindowPlaces(n) => {
                if let Some(p) = self.panel.as_mut() {
                    p.desktop.window_places = n;
                }
                Task::none()
            }
            Msg::ForgetWindowPlaces => {
                let reply = self.windows.forget();
                Task::perform(
                    async move {
                        reply.await.unwrap_or_else(|_| {
                            Err(crate::i18n::tr("gestionnaire de fenêtres arrêté").into())
                        })
                    },
                    |r| wrap(Msg::WindowPlacesForgotten(r)),
                )
            }
            Msg::WindowPlacesForgotten(r) => {
                if let Err(e) = r {
                    tracing::warn!("{}", crate::tr!("fenêtres : {e}", e = e));
                }
                load_places()
            }
            Msg::Language(l) => {
                self.settings.language = Some(l.to_string());
                self.commit()
            }
            Msg::WorkspaceMode(v) => {
                let (_, ws) = self.panel_ctx().unwrap_or_default();
                let mode = match v {
                    WsMode::Default => None,
                    WsMode::Opaque => Some(Mode::Opaque),
                    WsMode::Glass => Some(Mode::Glass),
                };
                self.settings.set_workspace_mode(&ws, mode);
                self.commit()
            }
            Msg::Font(f) => {
                self.settings.font = Some(f);
                self.commit()
            }
            Msg::FontSize(size) => {
                self.settings.font_size = Some(size);
                self.apply()
            }
            Msg::Fonts(f) => {
                panel.desktop.fonts = f;
                Task::none()
            }
            Msg::ClipboardHistory(v) => {
                self.settings.clipboard_history = Some(v);
                self.commit()
            }
            Msg::Autostart(v) => {
                self.settings.autostart = Some(v);
                self.commit()
            }
            Msg::DesktopFiles(v) => {
                // Les icônes du dossier sont ajoutées au chargement : on recharge.
                self.settings.desktop_files = Some(v);
                self.commit_reload()
            }
            Msg::AutostartEntry(id, enabled) => {
                if let Some(e) = crate::autostart::entries().into_iter().find(|e| e.id == id)
                    && let Err(err) = crate::autostart::set_enabled(&e, enabled)
                {
                    tracing::warn!("{}", crate::tr!("démarrage automatique : {e}", e = err));
                }
                Task::none()
            }
            Msg::ComponentRestart(name) => {
                Task::perform(async move { crate::session::restart(&name).await }, |_| {
                    UiMsg::Noop
                })
            }
            Msg::FloatLaunched(v) => {
                self.settings.float_launched = Some(v);
                self.commit()
            }
            Msg::TakeOver(pid) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                if p.desktop.takeover != Some(pid) {
                    p.desktop.takeover = Some(pid);
                    return Task::none();
                }
                p.desktop.takeover = None;
                tracing::warn!(
                    "{}",
                    crate::tr!(
                        "agent polkit concurrent arrêté à la demande (pid {pid})",
                        pid = pid
                    )
                );
                // SAFETY : simple signal à un processus de l'utilisateur.
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                Task::none()
            }
            Msg::Forced(key, mode) => {
                let mut m = self
                    .settings
                    .forced_modes
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.forced_modes.clone());
                match mode {
                    Some(v) => {
                        m.insert(key, v.to_string());
                    }
                    None => {
                        m.remove(&key);
                    }
                }
                self.settings.forced_modes = Some(m);
                self.commit()
            }
        }
    }
}
