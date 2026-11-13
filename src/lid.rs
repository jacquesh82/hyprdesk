//! Capot d'un portable (Réglages → Verrouillage → Veille) : ce qui se passe
//! quand il se referme, sur batterie, sur secteur ou avec un écran externe.
//!
//! Tant qu'un des trois cas n'est pas « comme le système », hyprdesk tient le
//! capot :
//! - un inhibiteur logind `handle-lid-switch` (composant de session
//!   `systemd-inhibit … sleep infinity`) empêche logind d'agir de son côté ;
//!   s'il s'arrête avec hyprdesk, logind reprend la main ;
//! - Hyprland appelle `hyprdesk lid close` / `hyprdesk lid open` sur les
//!   interrupteurs du capot (`switch:on:Lid Switch`, actifs même verrouillé).
//!
//! Un cas laissé « comme le système » alors que hyprdesk tient le capot
//! reprend le réglage de logind (HandleLidSwitch, …ExternalPower, …Docked).

use std::path::PathBuf;

use hyprpanel_core::ipc;

use crate::config::{ComponentDef, IdleCfg, LidAction};

/// Nom du composant de session (inhibiteur logind).
pub const COMPONENT: &str = "lid-inhibit";

/// Inhibiteur logind, si hyprdesk tient le capot d'un poste qui en a un.
pub fn component(c: &IdleCfg) -> Option<ComponentDef> {
    (c.lid_managed() && has_lid()).then(|| ComponentDef {
        name: COMPONENT.into(),
        exec: "systemd-inhibit --what=handle-lid-switch --who=hyprdesk \
               --why='Capot géré par hyprdesk' --mode=block sleep infinity"
            .into(),
        restart: true,
    })
}

/// Le poste a-t-il un capot (ACPI, ou interrupteur vu par Hyprland) ?
pub fn has_lid() -> bool {
    lid_state_files().next().is_some() || !switch_names().is_empty()
}

fn lid_state_files() -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir("/proc/acpi/button/lid")
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path().join("state"))
        .filter(|p| p.exists())
}

/// Capot fermé (ACPI) ; `None` si l'état est illisible.
pub fn lid_closed() -> Option<bool> {
    let s = std::fs::read_to_string(lid_state_files().next()?).ok()?;
    Some(s.contains("closed"))
}

/// Interrupteurs de capot vus par Hyprland (« Lid Switch » le plus souvent).
pub fn switch_names() -> Vec<String> {
    ipc::query_json("devices")
        .ok()
        .and_then(|v| {
            v["switches"].as_array().map(|a| {
                a.iter()
                    .filter_map(|s| s["name"].as_str())
                    .filter(|n| n.to_lowercase().contains("lid"))
                    .map(str::to_string)
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Sur secteur : une alimentation « Mains » (ou USB) branchée ; un poste
/// sans batterie compte comme sur secteur.
pub fn on_ac() -> bool {
    let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") else {
        return true;
    };
    let mut battery = false;
    for e in rd.flatten() {
        let p = e.path();
        let kind = std::fs::read_to_string(p.join("type")).unwrap_or_default();
        match kind.trim() {
            "Battery" => battery = true,
            "Mains" | "USB" | "USB_C" | "USB_PD"
                if std::fs::read_to_string(p.join("online")).is_ok_and(|s| s.trim() == "1") =>
            {
                return true;
            }
            _ => {}
        }
    }
    !battery
}

/// Écran du portable (eDP, LVDS, DSI).
pub fn is_internal(name: &str) -> bool {
    ["eDP", "LVDS", "DSI"].iter().any(|p| name.starts_with(p))
}

/// Écrans allumés : (nom, interne).
fn active_monitors() -> Vec<String> {
    ipc::query_json("monitors")
        .ok()
        .and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|m| m["name"].as_str().map(str::to_string))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Situation au moment où le capot se ferme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Situation {
    Battery,
    Ac,
    Docked,
}

pub fn situation(monitors: &[String], ac: bool) -> Situation {
    if monitors.iter().any(|m| !is_internal(m)) {
        Situation::Docked
    } else if ac {
        Situation::Ac
    } else {
        Situation::Battery
    }
}

/// Réglage de hyprdesk pour cette situation.
pub fn action_for(c: &IdleCfg, s: Situation) -> LidAction {
    match s {
        Situation::Battery => c.lid_battery,
        Situation::Ac => c.lid_ac,
        Situation::Docked => c.lid_docked,
    }
}

/// Réglage de logind pour cette situation (`ignore`, `suspend`, `lock`…).
fn logind_action(s: Situation) -> String {
    let prop = match s {
        Situation::Battery => "HandleLidSwitch",
        Situation::Ac => "HandleLidSwitchExternalPower",
        Situation::Docked => "HandleLidSwitchDocked",
    };
    std::process::Command::new("busctl")
        .args([
            "get-property",
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            prop,
        ])
        .output()
        .ok()
        .and_then(|o| {
            // Sortie : s "suspend"
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            s.split('"').nth(1).map(str::to_string)
        })
        .unwrap_or_else(|| {
            if s == Situation::Docked {
                "ignore"
            } else {
                "suspend"
            }
            .into()
        })
}

fn exe() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "hyprdesk".into())
}

/// Verrouille (écran de verrouillage de hyprdesk, une seule instance).
fn lock() {
    let _ = crate::hypr::exec(&format!("{} lock", crate::launch::sh_quote(&exe())));
}

fn systemctl(verb: &str) -> Result<(), String> {
    let st = std::process::Command::new("systemctl")
        .arg(verb)
        .status()
        .map_err(|e| e.to_string())?;
    st.success()
        .then_some(())
        .ok_or_else(|| crate::tr!("systemctl {v} a échoué", v = verb))
}

/// Ligne `hl.monitor` gardée pour rallumer l'écran du portable à l'ouverture.
fn saved_path(name: &str) -> PathBuf {
    ipc::runtime_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join(format!("hyprdesk-lid-{name}.lua"))
}

/// `hyprdesk lid close` : le capot vient de se fermer.
pub fn close(c: &IdleCfg) -> Result<String, String> {
    let monitors = active_monitors();
    let sit = situation(&monitors, on_ac());
    let action = action_for(c, sit);
    let internal: Vec<&String> = monitors.iter().filter(|m| is_internal(m)).collect();
    let verb = match action {
        LidAction::System => logind_action(sit),
        LidAction::Suspend => "suspend".into(),
        LidAction::Hibernate => "hibernate".into(),
        LidAction::Lock => "lock".into(),
        LidAction::ScreenOff => "screen-off".into(),
        LidAction::Nothing => "ignore".into(),
    };
    tracing::info!(
        "{}",
        crate::tr!("capot fermé ({s}) : {v}", s = format!("{sit:?}"), v = verb)
    );
    match verb.as_str() {
        "ignore" => {}
        "lock" => lock(),
        "screen-off" => {
            for m in internal {
                if sit == Situation::Docked {
                    // Écran externe : l'écran du portable est désactivé (ses
                    // workspaces passent sur l'externe), sa ligne gardée.
                    if let Some(s) = crate::screens::detect()
                        .ok()
                        .and_then(|l| l.into_iter().find(|s| &s.name == m))
                    {
                        let _ = std::fs::write(saved_path(m), s.lua());
                    }
                    ipc::eval(&format!(
                        "hl.monitor({{ output = \"{}\", disabled = true }})",
                        ipc::lua_escape(m)
                    ))
                    .map_err(|e| e.to_string())?;
                } else {
                    ipc::dispatch(&format!(
                        "hl.dsp.dpms({{ action = \"disable\", monitor = \"{}\" }})",
                        ipc::lua_escape(m)
                    ))
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        "suspend" | "hibernate" | "hybrid-sleep" | "suspend-then-hibernate" | "sleep" => {
            // Verrouillé avant de dormir : la surface a le temps de s'afficher.
            lock();
            std::thread::sleep(std::time::Duration::from_millis(700));
            systemctl(if verb == "sleep" { "suspend" } else { &verb })?;
        }
        "poweroff" | "reboot" | "halt" => systemctl(&verb)?,
        other => tracing::warn!(
            "{}",
            crate::tr!("capot : action logind « {a} » ignorée", a = other)
        ),
    }
    Ok(verb)
}

/// `hyprdesk lid open` : le capot s'ouvre ; l'écran du portable revient.
pub fn open() -> Result<String, String> {
    let mut back = Vec::new();
    let all = crate::screens::detect().unwrap_or_default();
    for s in all.iter().filter(|s| is_internal(&s.name)) {
        let path = saved_path(&s.name);
        if let Ok(line) = std::fs::read_to_string(&path) {
            ipc::eval(&line).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&path);
            back.push(s.name.clone());
        } else if !s.enabled {
            // Désactivé sans ligne gardée : mode préféré, placé automatiquement.
            ipc::eval(&format!(
                "hl.monitor({{ output = \"{}\", mode = \"preferred\", position = \"auto\", scale = \"auto\" }})",
                ipc::lua_escape(&s.name)
            ))
            .map_err(|e| e.to_string())?;
            back.push(s.name.clone());
        }
        let _ = ipc::dispatch(&format!(
            "hl.dsp.dpms({{ action = \"enable\", monitor = \"{}\" }})",
            ipc::lua_escape(&s.name)
        ));
    }
    Ok(back.join(", "))
}

/// Lua des raccourcis du capot ; vide (`active` faux) : seulement le retrait
/// des anciens.
pub fn build_binds_lua(active: bool, switches: &[String], exe: &str) -> String {
    let mut lua = String::from(
        "if __hyprdesk_lid then for _, b in ipairs(__hyprdesk_lid) do pcall(function() b:remove() end) end end\n\
         __hyprdesk_lid = {}\n",
    );
    if !active {
        return lua;
    }
    let names: Vec<String> = if switches.is_empty() {
        vec!["Lid Switch".into()]
    } else {
        switches.to_vec()
    };
    for n in names {
        for (state, arg, desc) in [
            ("on", "close", "capot fermé"),
            ("off", "open", "capot ouvert"),
        ] {
            let cmd = format!("{} lid {arg}", crate::launch::sh_quote(exe));
            lua += &format!(
                "pcall(function() table.insert(__hyprdesk_lid, hl.bind(\"switch:{state}:{}\", hl.dsp.exec_cmd(\"{}\"), {{ locked = true, description = \"hyprdesk : {desc}\" }})) end)\n",
                ipc::lua_escape(&n),
                ipc::lua_escape(&cmd),
            );
        }
    }
    lua
}

/// Pose (ou retire) les raccourcis du capot dans Hyprland.
pub fn install_binds(c: &IdleCfg) {
    let active = c.lid_managed() && has_lid();
    let lua = build_binds_lua(active, &switch_names(), &exe());
    if let Err(e) = ipc::eval(&lua) {
        tracing::warn!("{}", crate::tr!("raccourcis du capot : {e}", e = e));
    }
}

/// `hyprdesk lid [status|close|open]`.
pub fn run(c: &IdleCfg, args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str).unwrap_or("status") {
        "close" => close(c).map(|v| crate::tr!("capot fermé : {v}", v = v)),
        "open" => open().map(|l| {
            if l.is_empty() {
                crate::i18n::tr("capot ouvert").to_string()
            } else {
                crate::tr!("capot ouvert : {l} rallumé", l = l)
            }
        }),
        "status" => {
            let monitors = active_monitors();
            let sit = situation(&monitors, on_ac());
            let state = match lid_closed() {
                Some(true) => crate::i18n::tr("fermé"),
                Some(false) => crate::i18n::tr("ouvert"),
                None => crate::i18n::tr("inconnu"),
            };
            let mut out = if has_lid() {
                crate::tr!("capot : {s}", s = state)
            } else {
                crate::i18n::tr("aucun capot détecté sur ce poste").to_string()
            };
            out += &format!(
                "\n{}\n",
                crate::tr!(
                    "situation : {s} → {a}",
                    s = match sit {
                        Situation::Battery => crate::i18n::tr("sur batterie"),
                        Situation::Ac => crate::i18n::tr("sur secteur"),
                        Situation::Docked => crate::i18n::tr("avec un écran externe"),
                    },
                    a = match action_for(c, sit) {
                        LidAction::System =>
                            crate::tr!("comme le système ({v})", v = logind_action(sit)),
                        a => crate::i18n::tr(a.label()).to_string(),
                    }
                )
            );
            out += if c.lid_managed() {
                crate::i18n::tr("hyprdesk tient le capot (inhibiteur logind et raccourcis)")
            } else {
                crate::i18n::tr("capot laissé à logind")
            };
            Ok(out)
        }
        other => Err(crate::tr!("lid {o} : status | close | open", o = other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn situation_du_capot() {
        let edp = vec!["eDP-1".to_string()];
        assert_eq!(situation(&edp, false), Situation::Battery);
        assert_eq!(situation(&edp, true), Situation::Ac);
        let docked = vec!["eDP-1".to_string(), "HDMI-A-1".to_string()];
        assert_eq!(situation(&docked, false), Situation::Docked);
        let c = IdleCfg {
            lid_docked: LidAction::ScreenOff,
            ..Default::default()
        };
        assert!(c.lid_managed());
        assert_eq!(action_for(&c, Situation::Docked), LidAction::ScreenOff);
        assert_eq!(action_for(&c, Situation::Battery), LidAction::System);
        assert!(!IdleCfg::default().lid_managed());
    }

    #[test]
    fn raccourcis_du_capot() {
        let off = build_binds_lua(false, &[], "/bin/hyprdesk");
        assert!(!off.contains("hl.bind"));
        assert!(off.contains("b:remove()"));
        let on = build_binds_lua(true, &[], "/bin/hyprdesk");
        assert!(on.contains(r#"hl.bind("switch:on:Lid Switch", hl.dsp.exec_cmd("'/bin/hyprdesk' lid close"), { locked = true"#));
        assert!(on.contains(
            r#"hl.bind("switch:off:Lid Switch", hl.dsp.exec_cmd("'/bin/hyprdesk' lid open")"#
        ));
        for a in LidAction::ALL {
            assert_eq!(LidAction::parse(a.as_str()), Some(a));
        }
    }
}
