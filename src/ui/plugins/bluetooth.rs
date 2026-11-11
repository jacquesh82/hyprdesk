//! Paquet intégré « Bluetooth » (MLG-18) : vue complète du menu Bluetooth de
//! la barre (MLG-10) — activation, appareils appairés (connexion,
//! batterie, oubli), recherche et appairage. `bluetoothctl`.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, row, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    // Pas « bluetooth » : c'est déjà le plugin de barre livré (même liste
    // de paquets coupés). `hyprdesk settings bluetooth` y mène quand même.
    id: "bt",
    label: "Bluetooth",
    icon: "bluetooth-active-symbolic",
    summary: "Activation, appareils appairés, recherche et appairage.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Bluetooth(m)))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub mac: String,
    pub name: String,
    pub connected: bool,
    pub battery: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    /// `None` : pas d'adaptateur.
    pub powered: Option<bool>,
    pub paired: Vec<Device>,
    pub nearby: Vec<Device>,
    pub scanning: bool,
    pub loaded: bool,
    pub confirm: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(State),
    Power(bool),
    Connect(String),
    Disconnect(String),
    Forget(String),
    Pair(String),
    Scan,
}

/// `bluetoothctl devices` : « Device <MAC> <nom> » ; appareils sans nom écartés.
pub fn parse_devices(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("Device ")?;
            let (mac, name) = rest.split_once(' ')?;
            let unnamed = name.replace('-', ":") == mac;
            (!unnamed).then(|| (mac.to_string(), name.trim().to_string()))
        })
        .collect()
}

/// « Battery Percentage: 0x55 (85) ».
pub fn parse_battery(info: &str) -> Option<u8> {
    let l = info.lines().find(|l| l.contains("Battery Percentage:"))?;
    l.rsplit_once('(')?
        .1
        .trim_end_matches(')')
        .trim()
        .parse()
        .ok()
}

fn btctl(args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("bluetoothctl")
        .args(args)
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    if o.status.success() {
        Ok(out)
    } else {
        Err(out.lines().last().unwrap_or_default().trim().to_string())
    }
}

pub fn read() -> State {
    let show = btctl(&["show"]).unwrap_or_default();
    let powered = (!show.trim().is_empty() && show.contains("Controller"))
        .then(|| show.contains("Powered: yes"));
    let list = |kind: &[&str]| {
        let mut a = vec!["devices"];
        a.extend_from_slice(kind);
        parse_devices(&btctl(&a).unwrap_or_default())
    };
    let connected: Vec<String> = list(&["Connected"]).into_iter().map(|(m, _)| m).collect();
    let paired: Vec<Device> = list(&["Paired"])
        .into_iter()
        .map(|(mac, name)| {
            let c = connected.contains(&mac);
            Device {
                battery: if c {
                    parse_battery(&btctl(&["info", &mac]).unwrap_or_default())
                } else {
                    None
                },
                connected: c,
                mac,
                name,
            }
        })
        .collect();
    let nearby = list(&[])
        .into_iter()
        .filter(|(m, _)| !paired.iter().any(|p| &p.mac == m))
        .map(|(mac, name)| Device {
            mac,
            name,
            connected: false,
            battery: None,
        })
        .collect();
    State {
        powered,
        paired,
        nearby,
        loaded: true,
        ..Default::default()
    }
}

pub fn load() -> Task<UiMsg> {
    Task::perform(
        async { tokio::task::spawn_blocking(read).await.unwrap_or_default() },
        |s| wrap(Msg::Loaded(s)),
    )
}

/// Suite de commandes bluetoothctl (arrêt à la première erreur), puis relecture.
fn run(steps: Vec<Vec<String>>) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let mut err = None;
                for s in &steps {
                    let a: Vec<&str> = s.iter().map(String::as_str).collect();
                    if let Err(e) = btctl(&a) {
                        err = Some(e).filter(|e| !e.is_empty());
                        break;
                    }
                }
                let mut s = read();
                s.error = err;
                s
            })
            .await
            .unwrap_or_default()
        },
        |s| wrap(Msg::Loaded(s)),
    )
}

fn step(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

impl App {
    pub(in crate::ui) fn bluetooth_page(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let Some(s) = self
            .panel
            .as_ref()
            .map(|p| &p.bluetooth)
            .filter(|s| s.loaded)
        else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let Some(on) = s.powered else {
            return text("Aucun adaptateur Bluetooth.")
                .font(self.font)
                .size(13)
                .color(dim)
                .into();
        };
        let small = |label: &'static str, m: Msg| {
            button(text(label).font(self.font).size(12))
                .padding([4, 10])
                .on_press(wrap(m))
        };
        let mut col = column![self.field(
            "Bluetooth",
            "",
            toggler(on).on_toggle(|v| wrap(Msg::Power(v)))
        )]
        .spacing(10);
        if let Some(e) = &s.error {
            col = col.push(
                text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
            );
        }
        if !on {
            return col.into();
        }
        col = col.push(self.section("Appareils appairés"));
        if s.paired.is_empty() {
            col = col.push(text("Aucun appareil.").font(self.font).size(12).color(dim));
        }
        for d in &s.paired {
            let detail = match (d.connected, d.battery) {
                (true, Some(b)) => crate::tr!("connecté · batterie {b} %", b = b),
                (true, None) => crate::i18n::tr("connecté").to_string(),
                _ => crate::i18n::tr("non connecté").to_string(),
            };
            let confirm = s.confirm.as_deref() == Some(d.mac.as_str());
            col = col.push(
                row![
                    column![
                        text(d.name.clone()).font(self.font).size(13).color(fg),
                        text(detail).font(self.font).size(11).color(dim)
                    ]
                    .width(Length::Fill),
                    if d.connected {
                        small("Déconnecter", Msg::Disconnect(d.mac.clone()))
                    } else {
                        small("Se connecter", Msg::Connect(d.mac.clone()))
                    },
                    small(
                        if confirm {
                            "Confirmer l'oubli"
                        } else {
                            "Oublier"
                        },
                        Msg::Forget(d.mac.clone())
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        col = col.push(
            row![
                self.section("À proximité"),
                iced::widget::Space::new().width(Length::Fill),
                if s.scanning {
                    button(text("Recherche (10 s)…").font(self.font).size(12)).padding([4, 10])
                } else {
                    small("Rechercher", Msg::Scan)
                },
            ]
            .align_y(Alignment::End),
        );
        if s.nearby.is_empty() {
            col = col.push(
                text("Aucun appareil trouvé ; lancez une recherche, appareil en mode appairage.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for d in &s.nearby {
            col = col.push(
                row![
                    text(d.name.clone())
                        .font(self.font)
                        .size(13)
                        .color(fg)
                        .width(Length::Fill),
                    small("Appairer", Msg::Pair(d.mac.clone())),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn bluetooth_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Loaded(s) => {
                if let Some(p) = self.panel.as_mut() {
                    p.bluetooth = s;
                }
                Task::none()
            }
            Msg::Power(v) => run(vec![step(&["power", if v { "on" } else { "off" }])]),
            Msg::Connect(mac) => run(vec![step(&["trust", &mac]), step(&["connect", &mac])]),
            Msg::Disconnect(mac) => run(vec![step(&["disconnect", &mac])]),
            Msg::Forget(mac) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                if p.bluetooth.confirm.as_deref() == Some(mac.as_str()) {
                    p.bluetooth.confirm = None;
                    run(vec![step(&["remove", &mac])])
                } else {
                    p.bluetooth.confirm = Some(mac);
                    Task::none()
                }
            }
            Msg::Pair(mac) => run(vec![
                step(&["pair", &mac]),
                step(&["trust", &mac]),
                step(&["connect", &mac]),
            ]),
            Msg::Scan => {
                if let Some(p) = self.panel.as_mut() {
                    p.bluetooth.scanning = true;
                }
                run(vec![step(&["--timeout", "10", "scan", "on"])])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appareils_lus() {
        let d = parse_devices(
            "Device AA:BB:CC:DD:EE:01 MX Vertical\nDevice AA:BB:CC:DD:EE:02 AA-BB-CC-DD-EE-02\nautre ligne\n",
        );
        assert_eq!(d, vec![("AA:BB:CC:DD:EE:01".into(), "MX Vertical".into())]);
    }

    #[test]
    fn batterie_lue() {
        assert_eq!(parse_battery("\tBattery Percentage: 0x55 (85)\n"), Some(85));
        assert_eq!(parse_battery("Name: x"), None);
    }
}
