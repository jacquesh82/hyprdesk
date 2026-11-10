//! Inactivité et veille (paquet Verrouillage → Veille) : hyprdesk orchestre
//! hypridle plutôt que de le refaire.
//!
//! Réglage activé (`idle_manage`), hyprdesk écrit la configuration de
//! hypridle d'après les délais choisis, puis le lance comme composant de
//! session supervisé (`hypridle -c <fichier>`), relancé quand les délais
//! changent. Le verrouillage passe par logind (`loginctl lock-session`) vers
//! `hyprdesk lock`, y compris avant une mise en veille. Réglage coupé : rien
//! n'est lancé, un hypridle configuré à la main reste maître.

use std::path::PathBuf;

use crate::config::{ComponentDef, IdleCfg};

/// Nom du composant de session.
pub const COMPONENT: &str = "hypridle";

/// Fichier de configuration généré (dossier de l'instance Hyprland : il
/// disparaît avec la session, il est réécrit à chaque démarrage).
pub fn conf_path() -> PathBuf {
    hyprpanel_core::ipc::instance_dir()
        .or_else(|_| hyprpanel_core::ipc::runtime_dir())
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("hyprdesk-hypridle.conf")
}

fn dpms(action: &str) -> String {
    format!("hyprctl dispatch 'hl.dsp.dpms({{ action = \"{action}\" }})'")
}

/// Configuration hypridle pour ces délais (secondes, 0 = étape coupée).
pub fn conf(c: &IdleCfg, exe: &str) -> String {
    let mut out = format!(
        "# Généré par hyprdesk (Réglages → Verrouillage → Veille) : ne pas éditer.\n\
         general {{\n    \
         lock_cmd = {exe} lock\n    \
         before_sleep_cmd = loginctl lock-session\n    \
         after_sleep_cmd = {on}\n    \
         ignore_dbus_inhibit = false\n    \
         ignore_systemd_inhibit = false\n\
         }}\n",
        exe = crate::launch::sh_quote(exe),
        on = dpms("enable"),
    );
    let mut listener = |timeout: u32, on_timeout: String, on_resume: Option<String>| {
        if timeout == 0 {
            return;
        }
        out.push_str(&format!(
            "\nlistener {{\n    timeout = {timeout}\n    on-timeout = {on_timeout}\n"
        ));
        if let Some(r) = on_resume {
            out.push_str(&format!("    on-resume = {r}\n"));
        }
        out.push_str("}\n");
    };
    // Assombrir : luminosité de l'écran réduite (brightnessctl), puis rendue.
    listener(
        c.dim,
        "brightnessctl -s set 10%".into(),
        Some("brightnessctl -r".into()),
    );
    listener(c.lock, "loginctl lock-session".into(), None);
    listener(c.dpms, dpms("disable"), Some(dpms("enable")));
    listener(c.suspend, "systemctl suspend".into(), None);
    out
}

/// Composant hypridle à superviser, si hyprdesk gère la veille.
pub fn component(c: &IdleCfg) -> Option<ComponentDef> {
    c.manage.then(|| ComponentDef {
        name: COMPONENT.into(),
        exec: format!(
            "hypridle -c {}",
            crate::launch::sh_quote(&conf_path().display().to_string())
        ),
        restart: true,
    })
}

/// Écrit la configuration (si hyprdesk gère la veille). Le composant est
/// relancé par `session::sync` quand sa définition change ; un changement de
/// délais seul change le fichier, d'où le `restart` explicite de l'appelant.
pub fn write(c: &IdleCfg) -> std::io::Result<()> {
    if !c.manage {
        return Ok(());
    }
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "hyprdesk".into());
    std::fs::write(conf_path(), conf(c, &exe))
}

/// `foreign_hypridle`, relevé récent (l'écran Réglages se redessine souvent).
pub fn foreign_cached() -> Option<u32> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: Mutex<Option<(Instant, Option<u32>)>> = Mutex::new(None);
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    match *c {
        Some((at, pid)) if at.elapsed() < Duration::from_secs(3) => pid,
        _ => {
            let pid = foreign_hypridle();
            *c = Some((Instant::now(), pid));
            pid
        }
    }
}

/// PID d'un hypridle qui n'est pas celui de hyprdesk (lancé par
/// hyprland.lua, par exemple) : deux hypridle verrouilleraient deux fois.
pub fn foreign_hypridle() -> Option<u32> {
    let ours = crate::session::snapshot()
        .into_iter()
        .find(|(d, _)| d.name == COMPONENT)
        .and_then(|(_, s)| s.pid);
    std::fs::read_dir("/proc")
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| Some(*pid) != ours)
        .find(|pid| {
            std::fs::read_to_string(format!("/proc/{pid}/comm"))
                .is_ok_and(|c| c.trim() == "hypridle")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_generee() {
        let c = IdleCfg {
            manage: true,
            dim: 240,
            lock: 300,
            dpms: 360,
            suspend: 0,
        };
        let s = conf(&c, "/opt/hyprdesk bin/hyprdesk");
        assert!(s.contains("lock_cmd = '/opt/hyprdesk bin/hyprdesk' lock"));
        assert!(s.contains("before_sleep_cmd = loginctl lock-session"));
        assert!(s.contains(
            r#"after_sleep_cmd = hyprctl dispatch 'hl.dsp.dpms({ action = "enable" })'"#
        ));
        assert_eq!(s.matches("listener {").count(), 3, "veille coupée : {s}");
        assert!(s.contains("timeout = 240\n    on-timeout = brightnessctl -s set 10%"));
        assert!(s.contains("timeout = 300\n    on-timeout = loginctl lock-session"));
        assert!(
            s.contains(r#"on-timeout = hyprctl dispatch 'hl.dsp.dpms({ action = "disable" })'"#)
        );
        assert!(!s.contains("systemctl suspend"));
        let all = conf(&IdleCfg { suspend: 1800, ..c }, "hyprdesk");
        assert!(all.contains("timeout = 1800\n    on-timeout = systemctl suspend"));
    }

    #[test]
    fn composant_seulement_si_gere() {
        assert!(component(&IdleCfg::default()).is_none());
        let d = component(&IdleCfg {
            manage: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(d.name, COMPONENT);
        assert!(d.exec.starts_with("hypridle -c '"));
        assert!(d.restart);
    }
}
