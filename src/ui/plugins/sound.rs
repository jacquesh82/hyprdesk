//! Paquet intégré « Son » (MLG-18) : sortie et entrée par défaut, leur
//! volume, et le volume de chaque application qui joue du son (flux de
//! PipeWire, section « Streams » de `wpctl status`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, radio, row, slider};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "sound",
    label: "Son",
    icon: "audio-volume-high",
    summary: "Sortie et entrée audio, leur volume, et le volume de chaque application.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Sound(m)))
}

/// Un nœud audio (appareil ou application) et son volume.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: u32,
    pub name: String,
    pub default: bool,
    /// 0.0 – 1.5 (wpctl).
    pub volume: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub outputs: Vec<Node>,
    pub inputs: Vec<Node>,
    pub streams: Vec<Node>,
    /// Lecture faite (sinon « lecture… »).
    pub loaded: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(State),
    Refresh,
    Default(u32),
    /// Curseur glissé : appliqué tout de suite (id, volume).
    Volume(u32, f32),
    Mute(u32),
}

/// Applications de la section « Streams » de `wpctl status` (id, nom) ;
/// leurs ports (lignes « … > … » ou « … < … ») sont ignorés.
pub fn parse_streams(out: &str) -> Vec<(u32, String)> {
    let mut v = Vec::new();
    let (mut in_audio, mut in_streams) = (false, false);
    for line in out.lines() {
        if !line.starts_with(' ') && !line.trim().is_empty() {
            in_audio = line.trim() == "Audio";
            in_streams = false;
            continue;
        }
        if !in_audio {
            continue;
        }
        let body = line.trim_start_matches(|c: char| c.is_whitespace() || "│├└─".contains(c));
        if let Some(name) = body.strip_suffix(':')
            && (line.contains('├') || line.contains('└'))
        {
            in_streams = name == "Streams";
            continue;
        }
        if !in_streams {
            continue;
        }
        let body = body.trim_start_matches('*').trim_start();
        let Some((id, name)) = body.split_once(". ") else {
            continue;
        };
        let Ok(id) = id.trim().parse::<u32>() else {
            continue;
        };
        if name.contains(" > ") || name.contains(" < ") {
            continue;
        }
        v.push((id, name.trim().to_string()));
    }
    v
}

/// `wpctl get-volume` : « Volume: 0.45 [MUTED] ».
pub fn parse_volume(out: &str) -> Option<(f32, bool)> {
    let rest = out.trim().strip_prefix("Volume:")?.trim();
    let num = rest.split_whitespace().next()?.parse::<f32>().ok()?;
    Some((num, rest.contains("[MUTED]")))
}

fn wpctl(args: &[&str]) -> String {
    std::process::Command::new("wpctl")
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

fn node(id: u32, name: String, default: bool) -> Node {
    let (volume, muted) =
        parse_volume(&wpctl(&["get-volume", &id.to_string()])).unwrap_or((0.0, false));
    Node {
        id,
        name,
        default,
        volume,
        muted,
    }
}

/// Relevé complet (bloquant).
pub fn read() -> State {
    let status = wpctl(&["status"]);
    let (outs, ins) = crate::ui::bar::parse_wpctl_status(&status);
    State {
        outputs: outs
            .into_iter()
            .map(|d| node(d.id, d.name, d.default))
            .collect(),
        inputs: ins
            .into_iter()
            .map(|d| node(d.id, d.name, d.default))
            .collect(),
        streams: parse_streams(&status)
            .into_iter()
            .map(|(id, name)| node(id, name, false))
            .collect(),
        loaded: true,
    }
}

pub fn load() -> Task<UiMsg> {
    Task::perform(
        async { tokio::task::spawn_blocking(read).await.unwrap_or_default() },
        |s| wrap(Msg::Loaded(s)),
    )
}

/// Commande wpctl hors du fil de l'interface, puis relecture.
fn run(args: Vec<String>) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let a: Vec<&str> = args.iter().map(String::as_str).collect();
                wpctl(&a);
                read()
            })
            .await
            .unwrap_or_default()
        },
        |s| wrap(Msg::Loaded(s)),
    )
}

impl App {
    fn sound_state(&self) -> Option<&State> {
        self.panel.as_ref().map(|p| &p.sound)
    }

    fn sound_node<'a>(&self, n: &'a Node, fg: Color) -> Element<'a, UiMsg> {
        let id = n.id;
        let pct = (n.volume * 100.0).round() as i32;
        row![
            slider(0.0f32..=1.5, n.volume, move |v| wrap(Msg::Volume(id, v)))
                .step(0.01_f32)
                .width(Length::Fill),
            text(format!("{pct} %"))
                .font(self.font)
                .size(12)
                .color(fg)
                .width(48),
            button(
                text(if n.muted { "Rétablir" } else { "Couper" })
                    .font(self.font)
                    .size(12)
            )
            .padding([4, 10])
            .on_press(wrap(Msg::Mute(id))),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }

    fn sound_devices<'a>(&'a self, list: &'a [Node], fg: Color, dim: Color) -> Element<'a, UiMsg> {
        let mut col = column![].spacing(8);
        if list.is_empty() {
            return text("Aucun appareil.")
                .font(self.font)
                .size(12)
                .color(dim)
                .into();
        }
        let current = list.iter().find(|n| n.default).map(|n| n.id);
        for n in list {
            col = col.push(
                radio(n.name.clone(), n.id, current, |id| wrap(Msg::Default(id)))
                    .font(self.font)
                    .size(16)
                    .text_size(13),
            );
        }
        if let Some(d) = list.iter().find(|n| n.default) {
            col = col.push(self.sound_node(d, fg));
        }
        col.into()
    }

    pub(in crate::ui) fn sound_page(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = self.ink().dim;
        let Some(s) = self.sound_state().filter(|s| s.loaded) else {
            return text("Lecture des appareils…")
                .font(self.font)
                .size(13)
                .color(dim)
                .into();
        };
        let mut col = column![
            row![
                text("Réglages de PipeWire (wpctl).")
                    .font(self.font)
                    .size(12)
                    .color(dim)
                    .width(Length::Fill),
                button(text("Actualiser").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press(wrap(Msg::Refresh)),
            ]
            .align_y(Alignment::Center),
            self.section("Sortie"),
            self.sound_devices(&s.outputs, fg, dim),
            self.section("Entrée"),
            self.sound_devices(&s.inputs, fg, dim),
            self.section("Applications"),
        ]
        .spacing(10);
        if s.streams.is_empty() {
            col = col.push(
                text("Aucune application ne joue de son.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for n in &s.streams {
            col = col.push(
                column![
                    text(n.name.clone()).font(self.font).size(13).color(fg),
                    self.sound_node(n, fg)
                ]
                .spacing(4),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn sound_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Loaded(s) => {
                if let Some(p) = self.panel.as_mut() {
                    p.sound = s;
                }
                Task::none()
            }
            Msg::Refresh => load(),
            Msg::Default(id) => run(vec!["set-default".into(), id.to_string()]),
            Msg::Volume(id, v) => {
                let v = (v * 100.0).round() / 100.0;
                // Le curseur suit aussitôt ; wpctl applique en arrière-plan.
                if let Some(p) = self.panel.as_mut() {
                    for n in p
                        .sound
                        .outputs
                        .iter_mut()
                        .chain(p.sound.inputs.iter_mut())
                        .chain(p.sound.streams.iter_mut())
                        .filter(|n| n.id == id)
                    {
                        n.volume = v;
                    }
                }
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            wpctl(&["set-volume", &id.to_string(), &format!("{v:.2}")])
                        })
                        .await;
                    },
                    |_| UiMsg::Noop,
                )
            }
            Msg::Mute(id) => run(vec!["set-mute".into(), id.to_string(), "toggle".into()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = "PipeWire 'pipewire-0' [1.4.0]
Audio
 ├─ Devices:
 │      48. Arctis Nova Pro Wireless            [alsa]
 │
 ├─ Sinks:
 │  *   55. Arctis Nova Pro Wireless Analog Stereo [vol: 0.45]
 │
 ├─ Sources:
 │  *   56. Arctis Nova Pro Wireless Mono       [vol: 0.90]
 │
 ├─ Filters:
 │
 └─ Streams:
        78. Firefox
             79. output_FL       > Arctis Nova Pro Wireless:playback_FL	[active]
             80. output_FR       > Arctis Nova Pro Wireless:playback_FR	[active]
        91. spotify
             92. output_FL       > Arctis Nova Pro Wireless:playback_FL	[active]

Video
 └─ Streams:
        99. OBS
";

    #[test]
    fn flux_des_applications() {
        assert_eq!(
            parse_streams(STATUS),
            vec![(78, "Firefox".to_string()), (91, "spotify".to_string())]
        );
        assert!(parse_streams("Audio\n └─ Streams:\n").is_empty());
    }

    #[test]
    fn volume_lu() {
        assert_eq!(parse_volume("Volume: 0.45"), Some((0.45, false)));
        assert_eq!(parse_volume("Volume: 1.00 [MUTED]\n"), Some((1.0, true)));
        assert_eq!(parse_volume("erreur"), None);
    }
}
