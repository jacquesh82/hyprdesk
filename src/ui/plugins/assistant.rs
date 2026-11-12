//! Paquet intégré « IA » : l'onglet Assistant (LYRA, mindlog.hyprland_tools,
//! avec le fournisseur choisi — Claude Code, Codex, Zcode — et le contexte
//! de hyprdesk) et l'onglet Autres services IA (`services.rs` : endpoint et
//! clé API, SmartUI design). Domaine : `crate::assistant`,
//! `crate::ai_services`, `crate::smartui`. Ouvrir l'assistant ne change pas le mode.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

pub(in crate::ui) mod providers;
pub(in crate::ui) mod services;

use crate::i18n::text;
use iced::widget::{button, column, container, radio, row, scrollable};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::assistant::{self, AssistantMode};

pub const INFO: PluginInfo = PluginInfo {
    id: "assistant",
    label: "IA",
    icon: "ai-assistant",
    summary: "LYRA, l'assistante du bureau, et les autres services IA comme SmartUI design.",
    builtin: true,
};

pub const TABS: &[super::SubTab] = &[
    ("assistant", "Assistant"),
    ("services", "Autres services IA"),
    ("smartui", "SmartUI design"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Assistant(m)))
}

/// Ce qu'on sait de LYRA sur cette machine.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    pub installed: bool,
    pub service: bool,
    pub provider_flag: bool,
    pub context_dir: bool,
}

#[derive(Default)]
pub struct State {
    pub status: Option<Status>,
    pub show_context: bool,
    pub services: services::State,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Status(Status),
    Mode(AssistantMode),
    /// Fournisseur : valeur du rôle `ai` (`claude`, `codex`, `zcode`).
    Provider(String),
    Open,
    ToggleContext,
    // Fournisseurs IA (`providers.rs`).
    AiAdd(crate::ai_services::Provider),
    AiEdit(String),
    AiDraft(providers::DraftField, String),
    AiSave,
    AiCancel,
    /// Suppression en deux temps (armer, confirmer).
    AiDelete(String),
    AiKeyClear(String),
    /// Clé enregistrée (`true`) ou effacée, ou l'erreur du trousseau.
    AiKeyStored(Result<bool, String>),
    AiReloadKeys,
    AiKeys(Vec<(String, crate::ai_services::KeySource)>),
    /// Fonction → fournisseur.
    AiUse(&'static str, String),
    // SmartUI design.
    SmartToggle(bool),
    SmartStyleEdit(iced::widget::text_editor::Action),
    SmartStyleSave,
    /// Retour au style de hyprdesk.
    SmartStyleReset,
    SmartRows(Vec<services::RowData>),
    SmartGenerate(String),
    SmartGenerateMissing,
    SmartGenerated(String, Result<Vec<u8>, String>),
    SmartValidate(String),
    SmartReject(String),
    SmartForget(String),
    SmartClear,
    /// Génération automatique : fin d'une icône (clé ajoutée au thème, rien à faire, erreur).
    SmartAutoDone(Result<Option<String>, String>),
    SmartAutoToggle(bool),
    /// Réglages → IA → SmartUI design (depuis le Lanceur).
    OpenServices,
}

/// Arrivée sur le paquet : LYRA, la clé du service, les icônes SmartUI.
pub(in crate::ui) fn on_show(app: &mut App) -> Task<UiMsg> {
    app.services_init();
    Task::batch([
        load_status(),
        providers::load_keys(app),
        app.smartui_load(),
    ])
}

pub fn load_status() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| Status {
                installed: assistant::lyra().is_some(),
                service: assistant::service_active(),
                provider_flag: assistant::supports_provider(),
                context_dir: assistant::supports_context_dir(),
            })
            .await
            .unwrap_or_default()
        },
        |s| wrap(Msg::Status(s)),
    )
}

impl App {
    /// Ouvre l'assistant : LYRA si elle est là, sinon l'outil IA choisi, dans
    /// le terminal. Comme le lanceur, ne change pas le mode du bureau.
    pub(in crate::ui) fn open_assistant(&mut self) -> Task<UiMsg> {
        // Tiroir affiché : le même geste le cache (la session continue).
        if self.chat_shown() {
            return self.close_chat();
        }
        // LYRA sait se faire dessiner par hyprdesk : son tiroir, au format des Réglages.
        if let Some(engine) = assistant::serve_command(&self.cfg) {
            return self.open_chat(engine);
        }
        // LYRA prend la place du tiroir des réglages : un seul tiroir à la fois.
        let close = if self.panel.is_some() {
            self.close_panel()
        } else {
            Task::none()
        };
        self.open_assistant_window().chain(close)
    }

    /// L'assistant est affiché : son tiroir, ou la fenêtre de LYRA sur un
    /// workspace visible (pastille du bouton du lanceur).
    pub(in crate::ui) fn assistant_shown(&self) -> bool {
        self.chat_shown() || self.lyra_window_shown()
    }

    fn lyra_window_shown(&self) -> bool {
        self.clients.iter().any(|c| {
            c.class == assistant::LYRA_APP_ID
                && self.monitors.iter().any(|m| m.workspace == c.workspace)
        })
    }

    fn open_assistant_window(&mut self) -> Task<UiMsg> {
        // Déjà ouverte : affichée, le même bouton la ferme ; ailleurs, on la
        // ramène plutôt que d'en lancer une seconde.
        if let Some(c) = self
            .clients
            .iter()
            .find(|c| c.class == assistant::LYRA_APP_ID)
        {
            let address = c.address.clone();
            if self.lyra_window_shown() {
                if let Err(e) = crate::hypr::close_window(&address) {
                    tracing::warn!("{}", crate::tr!("LYRA : {e}", e = e.to_string()));
                }
                return Task::none();
            }
            return self.update_inner(UiMsg::FocusWindow(address));
        }
        let g = &self.cfg.general;
        let cmd = assistant::command(&self.cfg).or_else(|| {
            crate::apps::role("ai").and_then(|r| crate::apps::command(r, &g.apps, &g.terminal))
        });
        match cmd {
            Some(c) => self.run_from_desktop(c, true),
            None => {
                crate::hypr::notify(
                    3,
                    5000,
                    crate::i18n::tr(
                        "hyprdesk : aucun assistant IA (Réglages → Applications → Assistant IA)",
                    ),
                );
                Task::none()
            }
        }
    }

    /// LYRA présentée comme le tiroir des réglages (règle Hyprland sur sa
    /// classe `lyra`), recalculée si l'écran principal ou sa zone réservée change.
    pub(in crate::ui) fn sync_lyra_drawer(&mut self) -> Task<UiMsg> {
        let on = self.cfg.plugin_on("assistant")
            && self.cfg.general.assistant_mode == AssistantMode::Window
            && assistant::lyra_installed();
        let primary = self.primary_monitor();
        let rect = on
            .then(|| {
                self.monitors
                    .iter()
                    .find(|m| Some(&m.name) == primary.as_ref())
            })
            .flatten()
            .map(|m| {
                const GAP: f32 = 12.0;
                let (_, top, right, bottom) = m.reserved;
                let w = (super::super::panel::DRAWER_W - 2.0 * GAP).min(m.width as f32 - 2.0 * GAP);
                let h = m.height as f32 - top - bottom - 2.0 * GAP;
                let x = m.x + m.width as f32 - right - GAP - w;
                let y = m.y + top + GAP;
                (x as i32, y as i32, w as i32, h as i32)
            });
        if self.lyra_rule == rect {
            return Task::none();
        }
        self.lyra_rule = rect;
        let lua = assistant::drawer_rule_lua(rect);
        Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || {
                    if let Err(e) = hyprpanel_core::ipc::eval(&lua) {
                        tracing::warn!("{}", crate::tr!("règle de LYRA : {e}", e = e));
                    }
                })
                .await;
            },
            |_| UiMsg::Noop,
        )
    }

    pub(in crate::ui) fn assistant_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        if self.on_subtab(INFO.id, "services") {
            return self.services_tab(&p.assistant.services);
        }
        if self.on_subtab(INFO.id, "smartui") {
            return self.smartui_section(&p.assistant.services);
        }
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let g = &self.cfg.general;
        let line = |ok: bool, yes: &str, no: &str| {
            text(format!(
                "{} {}",
                if ok { "✓" } else { "•" },
                if ok { yes } else { no }
            ))
            .font(self.font)
            .size(13)
            .color(if ok { fg } else { warn })
        };
        let st = p.assistant.status.clone().unwrap_or_default();
        let mut status = column![
            line(
                st.installed,
                crate::i18n::tr("LYRA installée (hyprpanel-chat)"),
                "LYRA absente : github.com/jacquesh82/mindlog.hyprland_tools, ./scripts/install.sh"
            ),
            line(
                st.service,
                crate::i18n::tr("Service d'intégration Hyprland actif (hyprpanel.service)"),
                crate::i18n::tr(
                    "Service hyprpanel.service arrêté : LYRA ne pourra pas agir sur Hyprland"
                )
            ),
        ]
        .spacing(6);
        if st.installed {
            status = status
                .push(line(
                    st.provider_flag,
                    crate::i18n::tr("Le fournisseur choisi ici est transmis à LYRA"),
                    crate::i18n::tr(
                        "LYRA ne gère pas encore --provider : elle garde son propre choix",
                    ),
                ))
                .push(line(
                    st.context_dir,
                    crate::i18n::tr("LYRA lit le contexte de hyprdesk"),
                    crate::i18n::tr(
                        "LYRA ne lit pas encore context.d : contexte écrit, en attente",
                    ),
                ));
        }
        let ai = g.apps.get("ai").cloned().unwrap_or_default();
        let provider = |label: &'static str, v: &'static str| {
            radio(
                label,
                v,
                Some(ai.as_str()).filter(|a| *a == v).map(|_| v),
                |v| wrap(Msg::Provider(v.to_string())),
            )
            .font(self.font)
        };
        let mode = |label: &'static str, v: AssistantMode| {
            radio(crate::i18n::tr(label), v, Some(g.assistant_mode), |v| {
                wrap(Msg::Mode(v))
            })
            .font(self.font)
        };
        let mut col = column![
            self.section("LYRA"),
            text(
                "L'assistante du bureau Linux : elle explore l'environnement, dépanne et \
                 personnalise, avec confirmation avant chaque commande."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            status,
            self.section("Fournisseur"),
            self.field(
                "Abonnement utilisé",
                "apps.ai",
                column![
                    provider("Claude Code", "claude"),
                    provider("Codex", "codex"),
                    provider("Zcode", "zcode"),
                ]
                .spacing(8),
            ),
            self.adv(self.field(
                "Ouvrir l'assistant",
                "assistant_mode",
                column![
                    mode(crate::i18n::tr("Dans sa fenêtre"), AssistantMode::Window),
                    mode(crate::i18n::tr("Dans le terminal (hyprpanel-chat cli)"), AssistantMode::Terminal),
                ]
                .spacing(8),
            )),
            row![
                button(text("Ouvrir l'assistant").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press(wrap(Msg::Open)),
                button(
                    text(if p.assistant.show_context {
                        crate::i18n::tr("Masquer le contexte")
                    } else {
                        crate::i18n::tr("Voir le contexte transmis")
                    })
                    .font(self.font)
                    .size(13)
                )
                .padding([6, 14])
                .style(button::secondary)
                .on_press(wrap(Msg::ToggleContext)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text(
                "Aussi : « hyprdesk open ai » (raccourci), et le bouton de l'assistant dans le lanceur."
            )
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(14);
        if p.assistant.show_context {
            col = col.push(
                container(
                    scrollable(
                        text(assistant::context(&self.cfg, &self.config_path))
                            .font(self.font)
                            .size(12)
                            .color(fg),
                    )
                    .height(260),
                )
                .padding(10)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    border: iced::Border::default()
                        .rounded(8)
                        .width(1)
                        .color(Color { a: 0.4, ..accent }),
                    ..Default::default()
                }),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn assistant_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Status(s) => {
                if let Some(p) = self.panel.as_mut() {
                    p.assistant.status = Some(s);
                }
                Task::none()
            }
            Msg::ToggleContext => {
                if let Some(p) = self.panel.as_mut() {
                    p.assistant.show_context = !p.assistant.show_context;
                }
                Task::none()
            }
            Msg::Mode(m) => {
                self.settings.assistant_mode = Some(m);
                self.commit()
            }
            // Même réglage que Applications → Assistant IA.
            Msg::Provider(v) => {
                self.update_inner(super::apps::wrap(super::apps::Msg::Pick("ai".into(), v)))
            }
            Msg::Open => self.open_assistant(),
            m @ (Msg::AiAdd(_)
            | Msg::AiEdit(_)
            | Msg::AiDraft(..)
            | Msg::AiSave
            | Msg::AiCancel
            | Msg::AiDelete(_)
            | Msg::AiKeyClear(_)
            | Msg::AiKeyStored(_)
            | Msg::AiReloadKeys
            | Msg::AiKeys(_)
            | Msg::AiUse(..)) => self.providers_update(m),
            other => self.services_update(other),
        }
    }
}
