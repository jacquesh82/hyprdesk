//! Paquet intégré « Date et heure » (MLG-18) : fuseau horaire et
//! synchronisation réseau de l'horloge, par `timedatectl` (systemd-timedated ;
//! l'autorisation éventuelle passe par l'agent polkit de hyprdesk).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, pick_list, row, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "datetime",
    label: "Date et heure",
    icon: "preferences-system-time",
    summary: "Fuseau horaire et mise à l'heure automatique par le réseau.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::DateTime(m)))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub timezone: String,
    pub ntp: bool,
    pub can_ntp: bool,
    pub synced: bool,
    pub local_rtc: bool,
    pub zones: Vec<String>,
    pub loaded: bool,
    /// Dernière erreur de timedatectl (refus, annulation).
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(State),
    Zone(String),
    Ntp(bool),
}

/// `timedatectl show` : clés « Nom=valeur ».
pub fn parse_show(out: &str, s: &mut State) {
    for line in out.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let yes = v.trim() == "yes";
        match k.trim() {
            "Timezone" => s.timezone = v.trim().to_string(),
            "NTP" => s.ntp = yes,
            "CanNTP" => s.can_ntp = yes,
            "NTPSynchronized" => s.synced = yes,
            "LocalRTC" => s.local_rtc = yes,
            _ => {}
        }
    }
}

fn timedatectl(args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("timedatectl")
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

pub fn read() -> State {
    let mut s = State {
        loaded: true,
        ..Default::default()
    };
    if let Ok(out) = timedatectl(&["show"]) {
        parse_show(&out, &mut s);
    }
    s.zones = timedatectl(&["list-timezones"])
        .map(|o| o.lines().map(String::from).collect())
        .unwrap_or_default();
    s
}

pub fn load() -> Task<UiMsg> {
    Task::perform(
        async { tokio::task::spawn_blocking(read).await.unwrap_or_default() },
        |s| wrap(Msg::Loaded(s)),
    )
}

/// timedatectl hors du fil de l'interface (il attend l'agent polkit), puis relecture.
fn run(args: Vec<String>) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let a: Vec<&str> = args.iter().map(String::as_str).collect();
                let r = timedatectl(&a);
                let mut s = read();
                s.error = r.err().filter(|e| !e.is_empty());
                s
            })
            .await
            .unwrap_or_default()
        },
        |s| wrap(Msg::Loaded(s)),
    )
}

impl App {
    pub(in crate::ui) fn datetime_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let Some(s) = self
            .panel
            .as_ref()
            .map(|p| &p.datetime)
            .filter(|s| s.loaded)
        else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let mut col = column![
            self.section("Fuseau horaire"),
            self.field(
                "Fuseau",
                "",
                pick_list(s.zones.clone(), Some(s.timezone.clone()), |z| wrap(
                    Msg::Zone(z)
                ))
                .font(self.font)
                .width(Length::Fixed(320.0)),
            ),
            self.section("Horloge"),
        ]
        .spacing(10);
        if s.can_ntp {
            col = col.push(self.field(
                "Mise à l'heure automatique (réseau)",
                "",
                toggler(s.ntp).on_toggle(|v| wrap(Msg::Ntp(v))),
            ));
            col = col.push(
                text(if s.synced {
                    "Horloge synchronisée."
                } else {
                    "Horloge pas encore synchronisée."
                })
                .font(self.font)
                .size(12)
                .color(dim),
            );
        } else {
            col = col.push(
                text("Aucun service de synchronisation (systemd-timesyncd) disponible.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        if s.local_rtc {
            col = col.push(
                text(
                    "L'horloge matérielle est à l'heure locale (double démarrage avec Windows ?).",
                )
                .font(self.font)
                .size(12)
                .color(dim),
            );
        }
        if let Some(e) = &s.error {
            col = col.push(
                row![
                    text(e.clone())
                        .font(self.font)
                        .size(12)
                        .color(Color::from_rgb8(0xff, 0xb8, 0x6c))
                        .width(Length::Fill),
                    button(text("Relire").font(self.font).size(12))
                        .padding([4, 10])
                        .on_press(UiMsg::Panel(PanelMsg::Tab(INFO.id))),
                ]
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn datetime_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Loaded(s) => {
                if let Some(p) = self.panel.as_mut() {
                    p.datetime = s;
                }
                Task::none()
            }
            Msg::Zone(z) => run(vec!["set-timezone".into(), z]),
            Msg::Ntp(v) => run(vec!["set-ntp".into(), v.to_string()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_de_timedatectl() {
        let mut s = State::default();
        parse_show(
            "Timezone=Europe/Paris\nLocalRTC=no\nCanNTP=yes\nNTP=yes\nNTPSynchronized=no\nTimeUSec=Wed 2026-11-11\n",
            &mut s,
        );
        assert_eq!(s.timezone, "Europe/Paris");
        assert!(s.ntp && s.can_ntp && !s.synced && !s.local_rtc);
    }
}
