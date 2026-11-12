//! Paquet intégré « Énergie » (MLG-18) : profil d'énergie
//! (power-profiles-daemon), batteries, et résumé de la veille (réglée dans
//! Verrouillage → Veille, `crate::idle`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, radio};
use iced::{Element, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "energy",
    label: "Énergie",
    icon: "battery-full",
    summary: "Profil d'énergie, batteries et délais de mise en veille.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Energy(m)))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    /// Profils disponibles, l'actuel marqué.
    pub profiles: Vec<(String, bool)>,
    /// Batteries : nom, charge, état.
    pub batteries: Vec<(String, u8, String)>,
    pub loaded: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(State),
    Profile(String),
    /// Ouvre Verrouillage → Veille.
    Idle,
}

/// `powerprofilesctl list` : « * performance: » (actuel) ou « balanced: ».
pub fn parse_profiles(out: &str) -> Vec<(String, bool)> {
    out.lines()
        // Profils au plus à deux espaces du bord ; leurs propriétés, plus loin.
        .filter(|l| l.len() - l.trim_start().len() <= 2)
        .filter_map(|l| {
            let t = l.trim();
            let (cur, name) = match t.strip_prefix('*') {
                Some(r) => (true, r.trim()),
                None => (false, t),
            };
            let name = name.strip_suffix(':')?;
            (!name.is_empty() && !name.contains(char::is_whitespace))
                .then(|| (name.to_string(), cur))
        })
        .collect()
}

fn batteries() -> Vec<(String, u8, String)> {
    let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") else {
        return Vec::new();
    };
    let mut v: Vec<_> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let read = |f: &str| {
                std::fs::read_to_string(p.join(f))
                    .ok()
                    .map(|s| s.trim().to_string())
            };
            (read("type")? == "Battery" && read("scope").as_deref() != Some("Device")).then(|| {
                (
                    e.file_name().to_string_lossy().into_owned(),
                    read("capacity").and_then(|c| c.parse().ok()).unwrap_or(0),
                    read("status").unwrap_or_default(),
                )
            })
        })
        .collect();
    v.sort();
    v
}

pub fn read() -> State {
    let profiles = std::process::Command::new("powerprofilesctl")
        .arg("list")
        .output()
        .map(|o| parse_profiles(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();
    State {
        profiles,
        batteries: batteries(),
        loaded: true,
    }
}

pub fn load() -> Task<UiMsg> {
    Task::perform(
        async { tokio::task::spawn_blocking(read).await.unwrap_or_default() },
        |s| wrap(Msg::Loaded(s)),
    )
}

fn profile_label(id: &str) -> &str {
    match id {
        "performance" => crate::i18n::tr("Performance"),
        "balanced" => crate::i18n::tr("Équilibré"),
        "power-saver" => crate::i18n::tr("Économie d'énergie"),
        other => other,
    }
}

fn status_label(s: &str) -> &str {
    match s {
        "Charging" => crate::i18n::tr("en charge"),
        "Discharging" => crate::i18n::tr("sur batterie"),
        "Full" => crate::i18n::tr("chargée"),
        "Not charging" => crate::i18n::tr("branchée, sans charger"),
        other => other,
    }
}

/// Délai lisible (« 5 min », « jamais »).
fn delay(s: u32) -> String {
    match s {
        0 => crate::i18n::tr("jamais").to_string(),
        s if s < 60 => format!("{s} s"),
        s => format!("{} min", s / 60),
    }
}

impl App {
    pub(in crate::ui) fn energy_page(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = self.ink().dim;
        let Some(s) = self.panel.as_ref().map(|p| &p.energy).filter(|s| s.loaded) else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let mut col = column![self.section("Profil d'énergie")].spacing(10);
        if s.profiles.is_empty() {
            col = col.push(
                text("power-profiles-daemon n'est pas installé ou pas démarré.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        let current = s.profiles.iter().position(|(_, c)| *c);
        for (k, (p, _)) in s.profiles.iter().enumerate() {
            let id = p.clone();
            col = col.push(
                radio(profile_label(p), k, current, move |_| {
                    wrap(Msg::Profile(id.clone()))
                })
                .font(self.font)
                .size(16)
                .text_size(13),
            );
        }
        if !s.batteries.is_empty() {
            col = col.push(self.section("Batteries"));
            for (name, cap, st) in &s.batteries {
                col = col.push(
                    text(format!("{name} — {cap} % · {}", status_label(st)))
                        .font(self.font)
                        .size(13)
                        .color(fg),
                );
            }
        }
        let i = &self.cfg.general.idle;
        col = col.push(self.section("Veille"));
        col = col.push(
            text(if i.manage {
                crate::tr!(
                    "Assombrir : {dim} · verrouiller : {lock} · éteindre les écrans : {dpms} · mise en veille : {sus}",
                    dim = delay(i.dim),
                    lock = delay(i.lock),
                    dpms = delay(i.dpms),
                    sus = delay(i.suspend)
                )
            } else {
                crate::i18n::tr("hyprdesk ne gère pas la veille (hypridle réglé à la main, ou rien).")
                    .to_string()
            })
            .font(self.font)
            .size(12)
            .color(dim),
        );
        col = col.push(
            button(text("Régler la veille…").font(self.font).size(12))
                .padding([4, 10])
                .on_press(wrap(Msg::Idle)),
        );
        col.into()
    }

    pub(in crate::ui) fn energy_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Loaded(s) => {
                if let Some(p) = self.panel.as_mut() {
                    p.energy = s;
                }
                Task::none()
            }
            Msg::Profile(id) => Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        let _ = std::process::Command::new("powerprofilesctl")
                            .args(["set", &id])
                            .status();
                        read()
                    })
                    .await
                    .unwrap_or_default()
                },
                |s| wrap(Msg::Loaded(s)),
            ),
            Msg::Idle => {
                self.subtabs.insert("lock", "veille");
                self.panel_update(PanelMsg::Tab("lock"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profils_lus() {
        let out = "* performance:\n    CpuDriver:\tintel_pstate\n    Degraded:   no\n\n  balanced:\n    CpuDriver:\tintel_pstate\n\n  power-saver:\n    PlatformDriver:\tplaceholder\n";
        assert_eq!(
            parse_profiles(out),
            vec![
                ("performance".to_string(), true),
                ("balanced".to_string(), false),
                ("power-saver".to_string(), false)
            ]
        );
    }

    #[test]
    fn delais_lisibles() {
        assert_eq!(delay(300), "5 min");
        assert_eq!(delay(45), "45 s");
    }
}
