//! Paquet intégré « Verrouillage » : réglages de `hyprdesk lock` (fond,
//! assombrissement, contenus affichés) et verrouillage immédiat.
//! Domaine : `crate::lock`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, row, slider, toggler};
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

/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[("ecran", "Écran"), ("veille", "Veille")];

/// Étapes de l'inactivité, dans l'ordre où elles arrivent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleStep {
    Dim,
    Lock,
    Dpms,
    Suspend,
}

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
    Fingerprint(bool),
    LockNow,
    /// hyprdesk gère la veille (hypridle).
    IdleManage(bool),
    /// Délai d'une étape en minutes pendant le glissé (0 = jamais), puis enregistrement.
    IdleDelay(IdleStep, u32),
    IdleCommit,
}

impl App {
    pub(in crate::ui) fn lock_page(&self) -> Element<'_, UiMsg> {
        if self.on_subtab(INFO.id, "veille") {
            return self.idle_section();
        }
        let g = &self.cfg.general;
        let l = &g.lock;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        // D'où vient le fond du verrouillage : image propre, ou celle du bureau
        // (donc du thème), scène étalée comprise.
        let origin = match &l.image {
            Some(p) => format!(
                "Image propre au verrouillage : {} (hyprdesk config unset lock_image pour \
                 reprendre celle du bureau).",
                p.display()
            ),
            None => {
                let primary = self.primary_monitor().unwrap_or_default();
                let ws = self
                    .monitors
                    .iter()
                    .find(|m| m.name == primary)
                    .map(|m| m.workspace.clone())
                    .unwrap_or_else(|| "1".into());
                let d = crate::config::resolve::resolve(&self.cfg, &primary, &ws);
                match d.wallpaper {
                    Some(w) if w.span_lock => crate::tr!(
                        "Le fond du bureau, thème « {p0} » : la scène étalée sur les écrans, \
                         chacun en montre sa part. Une image propre : hyprdesk config set \
                         lock_image <chemin>.",
                        p0 = g.theme.name
                    ),
                    Some(_) => crate::tr!(
                        "Le fond du bureau de l'écran principal (thème « {p0} »). Une image \
                         propre : hyprdesk config set lock_image <chemin>.",
                        p0 = g.theme.name
                    ),
                    None => crate::i18n::tr(
                        "Aucun fond d'écran : la couleur « fond sans image » du thème. \
                             Une image propre : hyprdesk config set lock_image <chemin>.",
                    )
                    .to_string(),
                }
            }
        };
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
            text(origin).font(self.font).size(12).color(dim),
            self.section("Contenus"),
            column![
                toggler(l.show_user)
                    .label(crate::i18n::tr(
                        "Nom et avatar (AccountsService, sinon ~/.face)"
                    ))
                    .on_toggle(|v| wrap(Msg::ShowUser(v))),
                toggler(l.show_battery)
                    .label(crate::i18n::tr("Batterie"))
                    .on_toggle(|v| wrap(Msg::ShowBattery(v))),
                toggler(l.show_media)
                    .label(crate::i18n::tr("Morceau en cours, lecture / pause"))
                    .on_toggle(|v| wrap(Msg::ShowMedia(v))),
                toggler(l.fingerprint)
                    .label(crate::i18n::tr(
                        "Empreinte digitale en plus du mot de passe (fprintd)"
                    ))
                    .on_toggle(|v| wrap(Msg::Fingerprint(v))),
            ]
            .spacing(8),
            self.section("Sécurité"),
            text(crate::tr!(
                "Protocole de verrouillage de Wayland : si l'écran de verrouillage s'arrête, \
                 la session reste verrouillée. Mot de passe vérifié par PAM (service « {p0} »). \
                 Le contenu dérive lentement et s'estompe au repos, pour ne pas marquer l'écran.",
                p0 = crate::lock::pam::service()
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

    /// Inactivité et veille : hypridle orchestré par hyprdesk.
    fn idle_section(&self) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let i = &g.idle;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let warn = color(g.theme.accent, 1.0);
        let mut col = column![
            self.section("Veille"),
            text("À chaque étape, après ce délai sans activité. hyprdesk écrit la configuration de hypridle et le lance ; une application qui demande à rester éveillée (vidéo, appel) l'en empêche.")
                .font(self.font)
                .size(12)
                .color(dim),
            self.field(
                "hyprdesk gère la veille",
                "idle_manage",
                toggler(i.manage).on_toggle(|v| wrap(Msg::IdleManage(v))),
            ),
        ]
        .spacing(14);
        if !crate::launch::executable("hypridle") {
            col = col.push(
                text("hypridle est absent (paquet hypridle) : rien ne se passera à l'inactivité.")
                    .font(self.font)
                    .size(12)
                    .color(warn),
            );
        }
        if let Some(pid) = crate::idle::foreign_cached() {
            col = col.push(
                text(crate::tr!(
                    "Un hypridle lancé hors de hyprdesk tourne déjà (pid {pid}). Pour que hyprdesk gère la veille, retirez son lancement de hyprland.lua : deux hypridle verrouilleraient deux fois.",
                    pid = pid
                ))
                .font(self.font)
                .size(12)
                .color(if i.manage { warn } else { dim }),
            );
        }
        if !i.manage {
            return col.into();
        }
        for (step, label, key, secs) in [
            (IdleStep::Dim, "Assombrir l'écran", "idle_dim", i.dim),
            (IdleStep::Lock, "Verrouiller", "idle_lock", i.lock),
            (IdleStep::Dpms, "Éteindre les écrans", "idle_dpms", i.dpms),
            (
                IdleStep::Suspend,
                "Mettre en veille",
                "idle_suspend",
                i.suspend,
            ),
        ] {
            let minutes = secs.div_ceil(60);
            let value = if minutes == 0 {
                crate::i18n::tr("jamais").to_string()
            } else {
                crate::tr!("{n} min", n = minutes)
            };
            col = col.push(
                self.field(
                    label,
                    key,
                    row![
                        slider(0..=120u32, minutes, move |m| wrap(Msg::IdleDelay(step, m)))
                            .on_release(wrap(Msg::IdleCommit))
                            .width(260),
                        text(value).font(self.font).size(14),
                    ]
                    .spacing(12),
                ),
            );
        }
        col = col.push(
            text("Avant une mise en veille, la session est toujours verrouillée ; au réveil, les écrans se rallument. Assombrir passe par brightnessctl.")
                .font(self.font)
                .size(12)
                .color(dim),
        );
        col.into()
    }

    pub(in crate::ui) fn lock_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::IdleManage(v) => {
                self.settings.idle_manage = Some(v);
                self.commit()
            }
            Msg::IdleDelay(step, minutes) => {
                let secs = Some(minutes * 60);
                match step {
                    IdleStep::Dim => self.settings.idle_dim = secs,
                    IdleStep::Lock => self.settings.idle_lock = secs,
                    IdleStep::Dpms => self.settings.idle_dpms = secs,
                    IdleStep::Suspend => self.settings.idle_suspend = secs,
                }
                self.apply()
            }
            Msg::IdleCommit => self.commit(),
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
            Msg::Fingerprint(v) => {
                self.settings.lock_fingerprint = Some(v);
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
