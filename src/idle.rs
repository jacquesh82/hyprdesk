//! Inactivité et veille (paquet Verrouillage → Veille) : hyprdesk orchestre
//! hypridle plutôt que de le refaire.
//!
//! Réglage activé (`idle_manage`), hyprdesk écrit la configuration de
//! hypridle d'après les délais choisis, puis le lance comme composant de
//! session supervisé (`hypridle -c <fichier>`), relancé quand les délais
//! changent. Le verrouillage passe par logind (`loginctl lock-session`) vers
//! `hyprdesk lock`, y compris avant une mise en veille. Réglage coupé : rien
//! n'est lancé, un hypridle configuré à la main reste maître.
//!
//! Activé par défaut (verrouillage après 5 min). Un hypridle lancé hors de
//! hyprdesk reste maître : celui de hyprdesk ne démarre pas à côté. La
//! politique du poste (`policy::idle_lock_max`) impose la gestion et borne
//! le délai de verrouillage (« jamais » compris).

use std::path::PathBuf;

use crate::config::{ComponentDef, IdleCfg};

/// Nom du composant de session.
pub const COMPONENT: &str = "hypridle";

/// Fichier de configuration généré (dossier de l'instance Hyprland : il
/// disparaît avec la session, il est réécrit à chaque démarrage). Sans
/// dossier d'instance : le dossier d'exécution vérifié, jamais `/tmp`.
pub fn conf_path() -> Result<PathBuf, String> {
    hyprpanel_core::ipc::instance_dir()
        .map_err(|e| e.to_string())
        .or_else(|_| crate::policy::runtime_dir())
        .map(|d| d.join("hyprdesk-hypridle.conf"))
}

/// Réglages effectifs : ceux de l'utilisateur, bornés par la politique du
/// poste (gestion imposée, délai de verrouillage au plus `idle_lock_max`).
pub fn effective(c: &IdleCfg) -> IdleCfg {
    effective_with(c, crate::policy::get())
}

fn effective_with(c: &IdleCfg, p: &crate::policy::Policy) -> IdleCfg {
    let mut e = c.clone();
    if p.idle_lock_max.is_some() {
        e.manage = true;
        e.lock = p.clamp_idle_lock(c.lock);
    }
    e
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
         inhibit_sleep = 3\n    \
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

/// Composant hypridle à superviser, si hyprdesk gère la veille. Un hypridle
/// étranger déjà lancé garde la main (deux verrouilleraient deux fois), sauf
/// si la politique du poste impose la gestion par hyprdesk.
pub fn component(c: &IdleCfg) -> Option<ComponentDef> {
    let imposed = crate::policy::get().idle_lock_max.is_some();
    let c = effective(c);
    if !c.manage || (!imposed && foreign_hypridle().is_some()) {
        return None;
    }
    component_at(&conf_path().map_err(|e| tracing::warn!("{e}")).ok()?)
}

fn component_at(path: &std::path::Path) -> Option<ComponentDef> {
    Some(ComponentDef {
        name: COMPONENT.into(),
        exec: format!(
            "hypridle -c {}",
            crate::launch::sh_quote(&path.display().to_string())
        ),
        restart: true,
    })
}

/// Écrit la configuration (si hyprdesk gère la veille). Le composant est
/// relancé par `session::sync` quand sa définition change ; un changement de
/// délais seul change le fichier, d'où le `restart` explicite de l'appelant.
pub fn write(c: &IdleCfg) -> std::io::Result<()> {
    let c = effective(c);
    if !c.manage {
        return Ok(());
    }
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "hyprdesk".into());
    let path = conf_path().map_err(std::io::Error::other)?;
    std::fs::write(path, conf(&c, &exe))
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
                // Celui de hyprdesk se reconnaît aussi à sa configuration.
                && !std::fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|c| {
                    String::from_utf8_lossy(&c).contains("hyprdesk-hypridle")
                })
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
            ..Default::default()
        };
        let s = conf(&c, "/opt/hyprdesk bin/hyprdesk");
        assert!(s.contains("lock_cmd = '/opt/hyprdesk bin/hyprdesk' lock"));
        assert!(s.contains("before_sleep_cmd = loginctl lock-session"));
        // La veille attend que le verrou soit affiché sur chaque écran.
        assert!(s.contains("inhibit_sleep = 3"));
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
        let all = conf(
            &IdleCfg {
                suspend: 1800,
                ..c.clone()
            },
            "hyprdesk",
        );
        assert!(all.contains("timeout = 1800\n    on-timeout = systemctl suspend"));
    }

    #[test]
    fn composant_seulement_si_gere() {
        let off = IdleCfg {
            manage: false,
            ..Default::default()
        };
        assert!(component(&off).is_none() || crate::policy::get().idle_lock_max.is_some());
        let d = component_at(std::path::Path::new(
            "/run/user/1000/hyprdesk-hypridle.conf",
        ))
        .unwrap();
        assert_eq!(d.name, COMPONENT);
        assert!(d.exec.starts_with("hypridle -c '"));
        assert!(d.restart);
    }

    /// Verrouillage actif par défaut ; la politique impose la gestion et
    /// borne le délai, « jamais » compris.
    #[test]
    fn defaut_et_politique() {
        let d = IdleCfg::default();
        assert!(d.manage && d.lock == 300);
        let p = crate::policy::Policy {
            idle_lock_max: Some(600),
            ..Default::default()
        };
        let off = IdleCfg {
            manage: false,
            lock: 0,
            ..Default::default()
        };
        let e = effective_with(&off, &p);
        assert!(e.manage);
        assert_eq!(e.lock, 600);
        let long = IdleCfg {
            lock: 3600,
            ..Default::default()
        };
        assert_eq!(effective_with(&long, &p).lock, 600);
        assert_eq!(
            effective_with(&off, &crate::policy::Policy::default()),
            off,
            "sans politique : réglages de l'utilisateur"
        );
    }
}

/// Avertissement quand l'écran s'éteint avant le verrouillage (MLG-140) :
/// la session reste ouverte, écran noir, jusqu'au verrouillage (ou toujours
/// s'il est coupé). Les délais sont ceux de l'utilisateur : hyprdesk avertit,
/// il ne les change pas.
pub fn exposure(c: &crate::config::IdleCfg) -> Option<String> {
    if !c.manage || c.dpms == 0 {
        return None;
    }
    let min = |s: u64| s.div_ceil(60);
    if c.lock == 0 {
        return Some(crate::tr!(
            "Les écrans s'éteignent après {d} min mais la session ne se verrouille jamais : un simple mouvement de souris la rouvre.",
            d = min(c.dpms as u64)
        ));
    }
    (c.lock > c.dpms).then(|| {
        crate::tr!(
            "Les écrans s'éteignent après {d} min, la session ne se verrouille qu'après {l} min : {n} min déverrouillée, écran noir.",
            d = min(c.dpms as u64),
            l = min(c.lock as u64),
            n = min((c.lock - c.dpms) as u64)
        )
    })
}

#[cfg(test)]
mod exposure_tests {
    use super::*;

    #[test]
    fn verrouillage_apres_extinction() {
        let mut c = crate::config::IdleCfg {
            manage: true,
            ..Default::default()
        };
        c.dpms = 1200;
        c.lock = 3600;
        assert!(exposure(&c).unwrap().contains("40 min"));
        c.lock = 600;
        assert!(exposure(&c).is_none());
        c.lock = 0;
        assert!(exposure(&c).is_some());
        c.manage = false;
        assert!(exposure(&c).is_none());
    }
}
