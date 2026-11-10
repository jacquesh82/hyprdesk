//! Paquet intégré « Assistant IA » : LYRA (mindlog.hyprland_tools), avec le
//! fournisseur choisi (Claude Code, Codex, Zcode) et le contexte de hyprdesk.
//! Domaine : `crate::assistant`. Ouvrir l'assistant ne change pas le mode.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use iced::widget::{button, column, container, radio, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::assistant::{self, AssistantMode};

pub const INFO: PluginInfo = PluginInfo {
    id: "assistant",
    label: "Assistant IA",
    icon: "ai-assistant",
    summary: "LYRA avec Claude Code, Codex ou Zcode, et tout l'environnement en contexte.",
    builtin: true,
};

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
}

#[derive(Debug, Clone)]
pub enum Msg {
    Status(Status),
    Mode(AssistantMode),
    /// Fournisseur : valeur du rôle `ai` (`claude`, `codex`, `zcode`).
    Provider(String),
    Open,
    ToggleContext,
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
        // Tiroir déjà ouvert : le même geste le referme.
        if self.chat.is_some() {
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

    fn open_assistant_window(&mut self) -> Task<UiMsg> {
        // Déjà ouverte : on la ramène plutôt que d'en lancer une seconde.
        if let Some(c) = self
            .clients
            .iter()
            .find(|c| c.class == assistant::LYRA_APP_ID)
        {
            let address = c.address.clone();
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
                    "hyprdesk : aucun assistant IA (Réglages → Applications → Assistant IA)",
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
                        tracing::warn!("règle de LYRA : {e}");
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
                "LYRA installée (hyprpanel-chat)",
                "LYRA absente : github.com/jacquesh82/mindlog.hyprland_tools, ./scripts/install.sh"
            ),
            line(
                st.service,
                "Service d'intégration Hyprland actif (hyprpanel.service)",
                "Service hyprpanel.service arrêté : LYRA ne pourra pas agir sur Hyprland"
            ),
        ]
        .spacing(6);
        if st.installed {
            status = status
                .push(line(
                    st.provider_flag,
                    "Le fournisseur choisi ici est transmis à LYRA",
                    "LYRA ne gère pas encore --provider : elle garde son propre choix",
                ))
                .push(line(
                    st.context_dir,
                    "LYRA lit le contexte de hyprdesk",
                    "LYRA ne lit pas encore context.d : contexte écrit, en attente",
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
            radio(label, v, Some(g.assistant_mode), |v| wrap(Msg::Mode(v))).font(self.font)
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
            self.field(
                "Ouvrir l'assistant",
                "assistant_mode",
                column![
                    mode("Dans sa fenêtre", AssistantMode::Window),
                    mode("Dans le terminal (hyprpanel-chat cli)", AssistantMode::Terminal),
                ]
                .spacing(8),
            ),
            row![
                button(text("Ouvrir l'assistant").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press(wrap(Msg::Open)),
                button(
                    text(if p.assistant.show_context {
                        "Masquer le contexte"
                    } else {
                        "Voir le contexte transmis"
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
        }
    }
}
