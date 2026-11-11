//! Paquet intégré « Réseau » (MLG-18) : vue complète du menu réseau de la
//! barre (MLG-9) — Wi-Fi et mode avion, connexions enregistrées (filaire,
//! Wi-Fi, VPN, WireGuard…), réseaux Wi-Fi à proximité. NetworkManager
//! (`nmcli`). Un réseau protégé jamais utilisé se rejoint depuis le menu de
//! la barre (champ de mot de passe) ou l'éditeur de connexions.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, row, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "network",
    label: "Réseau",
    icon: "network-wired-symbolic",
    summary: "Wi-Fi, mode avion, connexions enregistrées et VPN (NetworkManager).",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Network(m)))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conn {
    pub name: String,
    /// Type nmcli (`802-11-wireless`, `vpn`…).
    pub kind: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Net {
    pub ssid: String,
    pub signal: u8,
    pub secure: bool,
    pub in_use: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub wifi: bool,
    pub airplane: bool,
    pub conns: Vec<Conn>,
    pub nets: Vec<Net>,
    pub loaded: bool,
    /// Connexion dont l'oubli attend confirmation.
    pub confirm: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(State),
    Refresh,
    Wifi(bool),
    Airplane(bool),
    Up(String),
    Down(String),
    Forget(String),
    Join(String),
    Editor,
}

/// Champs d'une ligne `nmcli -t` : séparés par « : », « \: » échappé.
pub fn fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.last_mut().unwrap().push(n);
                }
            }
            ':' => out.push(String::new()),
            c => out.last_mut().unwrap().push(c),
        }
    }
    out
}

/// Types de connexion montrés (pas les ponts, boucles, interfaces virtuelles).
const KINDS: &[&str] = &[
    "802-11-wireless",
    "802-3-ethernet",
    "vpn",
    "wireguard",
    "gsm",
    "bluetooth",
];

pub fn parse_conns(out: &str) -> Vec<Conn> {
    let mut v: Vec<Conn> = out
        .lines()
        .map(fields)
        .filter(|f| f.len() >= 3 && KINDS.contains(&f[1].as_str()))
        .map(|f| Conn {
            name: f[0].clone(),
            kind: f[1].clone(),
            active: f[2] == "yes",
        })
        .collect();
    v.sort_by(|a, b| b.active.cmp(&a.active).then(a.name.cmp(&b.name)));
    v
}

/// `nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY dev wifi list` ; un SSID gardé
/// une fois (le plus fort), réseaux cachés écartés.
pub fn parse_nets(out: &str) -> Vec<Net> {
    let mut v: Vec<Net> = Vec::new();
    for f in out.lines().map(fields).filter(|f| f.len() >= 4) {
        if f[1].is_empty() {
            continue;
        }
        let n = Net {
            ssid: f[1].clone(),
            signal: f[2].parse().unwrap_or(0),
            secure: !f[3].is_empty() && f[3] != "--",
            in_use: f[0] == "*",
        };
        match v.iter_mut().find(|x| x.ssid == n.ssid) {
            Some(x) => {
                x.in_use |= n.in_use;
                x.signal = x.signal.max(n.signal);
            }
            None => v.push(n),
        }
    }
    v.sort_by(|a, b| b.in_use.cmp(&a.in_use).then(b.signal.cmp(&a.signal)));
    v
}

fn kind_label(k: &str) -> &str {
    match k {
        "802-11-wireless" => crate::i18n::tr("Wi-Fi"),
        "802-3-ethernet" => crate::i18n::tr("Filaire"),
        "vpn" => crate::i18n::tr("VPN"),
        "wireguard" => "WireGuard",
        "gsm" => crate::i18n::tr("Mobile"),
        "bluetooth" => crate::i18n::tr("Bluetooth"),
        other => other,
    }
}

fn sh(prog: &str, args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new(prog)
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
    let nm = |a: &[&str]| sh("nmcli", a).unwrap_or_default();
    let wifi = nm(&["radio", "wifi"]).trim() == "enabled";
    let airplane = sh("rfkill", &["-n", "-o", "SOFT"])
        .map(|o| !o.trim().is_empty() && !o.contains("unblocked"))
        .unwrap_or(false);
    State {
        wifi,
        airplane,
        conns: parse_conns(&nm(&["-t", "-f", "NAME,TYPE,ACTIVE", "connection", "show"])),
        nets: if wifi {
            parse_nets(&nm(&[
                "-t",
                "-f",
                "IN-USE,SSID,SIGNAL,SECURITY",
                "dev",
                "wifi",
                "list",
            ]))
        } else {
            Vec::new()
        },
        loaded: true,
        confirm: None,
        error: None,
    }
}

pub fn load() -> Task<UiMsg> {
    Task::perform(
        async { tokio::task::spawn_blocking(read).await.unwrap_or_default() },
        |s| wrap(Msg::Loaded(s)),
    )
}

/// Commande hors du fil de l'interface, puis relecture (erreur gardée).
fn run(prog: &'static str, args: Vec<String>) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let a: Vec<&str> = args.iter().map(String::as_str).collect();
                let r = sh(prog, &a);
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
    pub(in crate::ui) fn network_page(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let Some(s) = self.panel.as_ref().map(|p| &p.network).filter(|s| s.loaded) else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let small = |label: &'static str, m: Msg| {
            button(text(label).font(self.font).size(12))
                .padding([4, 10])
                .on_press(wrap(m))
        };
        let mut col = column![
            row![
                text("NetworkManager (nmcli).")
                    .font(self.font)
                    .size(12)
                    .color(dim)
                    .width(Length::Fill),
                small("Éditeur de connexions…", Msg::Editor),
                small("Actualiser", Msg::Refresh),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            self.field(
                "Wi-Fi",
                "",
                toggler(s.wifi).on_toggle(|v| wrap(Msg::Wifi(v)))
            ),
            self.field(
                "Mode avion",
                "",
                toggler(s.airplane).on_toggle(|v| wrap(Msg::Airplane(v)))
            ),
        ]
        .spacing(10);
        if let Some(e) = &s.error {
            col = col.push(
                text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
            );
        }
        col = col.push(self.section("Connexions enregistrées"));
        if s.conns.is_empty() {
            col = col.push(
                text("Aucune connexion.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for c in &s.conns {
            let mut line = row![
                column![
                    text(c.name.clone()).font(self.font).size(13).color(fg),
                    text(format!(
                        "{}{}",
                        kind_label(&c.kind),
                        if c.active {
                            crate::i18n::tr(" · active")
                        } else {
                            ""
                        }
                    ))
                    .font(self.font)
                    .size(11)
                    .color(dim)
                ]
                .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            line = if c.active {
                line.push(small("Déconnecter", Msg::Down(c.name.clone())))
            } else {
                line.push(small("Se connecter", Msg::Up(c.name.clone())))
            };
            if !c.active {
                let confirm = s.confirm.as_deref() == Some(c.name.as_str());
                line = line.push(small(
                    if confirm {
                        "Confirmer l'oubli"
                    } else {
                        "Oublier"
                    },
                    Msg::Forget(c.name.clone()),
                ));
            }
            col = col.push(line);
        }
        if s.wifi {
            col = col.push(self.section("Réseaux Wi-Fi à proximité"));
            if s.nets.is_empty() {
                col = col.push(
                    text("Aucun réseau visible.")
                        .font(self.font)
                        .size(12)
                        .color(dim),
                );
            }
            let known = |ssid: &str| s.conns.iter().any(|c| c.name == ssid);
            for n in s.nets.iter().take(12) {
                let mut line = row![
                    column![
                        text(n.ssid.clone()).font(self.font).size(13).color(fg),
                        text(format!(
                            "{} %{}{}",
                            n.signal,
                            if n.secure {
                                crate::i18n::tr(" · sécurisé")
                            } else {
                                ""
                            },
                            if n.in_use {
                                crate::i18n::tr(" · connecté")
                            } else {
                                ""
                            }
                        ))
                        .font(self.font)
                        .size(11)
                        .color(dim)
                    ]
                    .width(Length::Fill),
                ]
                .spacing(8)
                .align_y(Alignment::Center);
                if !n.in_use {
                    line = if !n.secure || known(&n.ssid) {
                        line.push(small("Se connecter", Msg::Join(n.ssid.clone())))
                    } else {
                        line.push(
                            text("mot de passe : menu Réseau de la barre")
                                .font(self.font)
                                .size(11)
                                .color(dim),
                        )
                    };
                }
                col = col.push(line);
            }
        }
        col.into()
    }

    pub(in crate::ui) fn network_update(&mut self, m: Msg) -> Task<UiMsg> {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        match m {
            Msg::Loaded(st) => {
                if let Some(p) = self.panel.as_mut() {
                    p.network = st;
                }
                Task::none()
            }
            Msg::Refresh => load(),
            Msg::Wifi(v) => run("nmcli", s(&["radio", "wifi", if v { "on" } else { "off" }])),
            Msg::Airplane(v) => run("rfkill", s(&[if v { "block" } else { "unblock" }, "all"])),
            Msg::Up(name) => run(
                "nmcli",
                vec!["connection".into(), "up".into(), "id".into(), name],
            ),
            Msg::Down(name) => run(
                "nmcli",
                vec!["connection".into(), "down".into(), "id".into(), name],
            ),
            Msg::Forget(name) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                if p.network.confirm.as_deref() == Some(name.as_str()) {
                    p.network.confirm = None;
                    run(
                        "nmcli",
                        vec!["connection".into(), "delete".into(), "id".into(), name],
                    )
                } else {
                    p.network.confirm = Some(name);
                    Task::none()
                }
            }
            Msg::Join(ssid) => run(
                "nmcli",
                vec!["dev".into(), "wifi".into(), "connect".into(), ssid],
            ),
            Msg::Editor => {
                let _ = std::process::Command::new("nm-connection-editor")
                    .stdin(std::process::Stdio::null())
                    .spawn();
                Task::none()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn champs_echappes() {
        assert_eq!(fields(r"a\:b:c:"), vec!["a:b", "c", ""]);
    }

    #[test]
    fn connexions_filtrees() {
        let c = parse_conns(
            "Wired connection 1:802-3-ethernet:yes\nbr0:bridge:yes\nMaison:802-11-wireless:no\nwg0:wireguard:yes\nlo:loopback:yes\n",
        );
        let names: Vec<&str> = c.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Wired connection 1", "wg0", "Maison"]);
        assert!(c[0].active && !c[2].active);
    }

    #[test]
    fn reseaux_dedoublonnes() {
        let n = parse_nets(
            "*:Maison:70:WPA2\n :Maison:40:WPA2\n :Gare:80:--\n ::90:WPA2\n :Voisin\\:5G:60:WPA3\n",
        );
        let ssids: Vec<&str> = n.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(ssids, ["Maison", "Gare", "Voisin:5G"]);
        assert!(n[0].in_use && n[0].secure && n[0].signal == 70);
        assert!(!n[1].secure);
    }
}
