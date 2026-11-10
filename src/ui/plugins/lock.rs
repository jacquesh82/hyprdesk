//! Paquet intégré « Verrouillage » : réglages de `hyprdesk lock` (fond,
//! assombrissement, contenus affichés) et verrouillage immédiat.
//! Domaine : `crate::lock`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use iced::widget::{button, column, row, slider, text, toggler};
use iced::{Color, Element, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "lock",
    label: "Verrouillage",
    icon: "system-lock-screen",
    summary: "Écran de verrouillage : fond, contenus affichés, verrouiller maintenant.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Lock(m)))
}

#[derive(Debug, Clone)]
pub enum Msg {
    Blur(bool),
    /// Assombrissement pendant le glissé, puis enregistrement.
    Dim(f32),
    DimCommit,
    ShowUser(bool),
    ShowBattery(bool),
    ShowMedia(bool),
    LockNow,
}

impl App {
    pub(in crate::ui) fn lock_page(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let l = &g.lock;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        column![
            self.section("Fond"),
            self.field(
                "Fond flouté",
                "lock_blur",
                toggler(l.blur).on_toggle(|v| wrap(Msg::Blur(v))),
            ),
            self.field(
                "Assombrissement",
                "lock_dim",
                row![
                    slider(0.0..=0.9f32, l.dim, |v| wrap(Msg::Dim(v)))
                        .step(0.05f32)
                        .on_release(wrap(Msg::DimCommit))
                        .width(260),
                    text(format!("{:.0} %", l.dim * 100.0))
                        .font(self.font)
                        .size(14),
                ]
                .spacing(12),
            ),
            text(
                "Image : celle du bureau de l'écran principal, ou une image propre \\
                 (hyprdesk config set lock_image <chemin>)."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.section("Contenus"),
            column![
                toggler(l.show_user)
                    .label("Nom et avatar (AccountsService, sinon ~/.face)")
                    .on_toggle(|v| wrap(Msg::ShowUser(v))),
                toggler(l.show_battery)
                    .label("Batterie")
                    .on_toggle(|v| wrap(Msg::ShowBattery(v))),
                toggler(l.show_media)
                    .label("Morceau en cours, lecture / pause")
                    .on_toggle(|v| wrap(Msg::ShowMedia(v))),
            ]
            .spacing(8),
            self.section("Sécurité"),
            text(format!(
                "Protocole de verrouillage de Wayland : si l'écran de verrouillage s'arrête, \
                 la session reste verrouillée. Mot de passe vérifié par PAM (service « {} »). \
                 Le contenu dérive lentement et s'estompe au repos, pour ne pas marquer l'écran.",
                crate::lock::pam::service()
            ))
            .font(self.font)
            .size(12)
            .color(dim),
            button(text("Verrouiller maintenant").font(self.font).size(13))
                .padding([6, 14])
                .on_press(wrap(Msg::LockNow)),
        ]
        .spacing(14)
        .into()
    }

    pub(in crate::ui) fn lock_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Blur(v) => {
                self.settings.lock_blur = Some(v);
                self.commit()
            }
            Msg::Dim(v) => {
                self.settings.lock_dim = Some(v);
                self.apply()
            }
            Msg::DimCommit => self.commit(),
            Msg::ShowUser(v) => {
                self.settings.lock_show_user = Some(v);
                self.commit()
            }
            Msg::ShowBattery(v) => {
                self.settings.lock_show_battery = Some(v);
                self.commit()
            }
            Msg::ShowMedia(v) => {
                self.settings.lock_show_media = Some(v);
                self.commit()
            }
            Msg::LockNow => {
                let exe = std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "hyprdesk".into());
                let cmd = format!("{} lock", crate::launch::sh_quote(&exe));
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || crate::hypr::exec(&cmd)).await;
                    },
                    |_| UiMsg::Noop,
                )
            }
        }
    }
}
