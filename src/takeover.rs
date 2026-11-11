//! Services que la configuration locale empêche hyprdesk de prendre en
//! charge : un autre programme les tient déjà (démon de notifications,
//! agent polkit, éclairage nocturne, veille, presse-papiers). Chacun vient
//! avec une demande toute prête pour LYRA, qui trouve d'où il est lancé et
//! propose de laisser la main à hyprdesk (en demandant avant d'agir).

/// Un service tenu par un autre programme.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    /// Fonction concernée (clé de traduction).
    pub feature: &'static str,
    /// Programme en place.
    pub holder: String,
    pub pid: u32,
}

impl Conflict {
    /// Demande à LYRA, dans la langue de l'interface.
    pub fn prompt(&self) -> String {
        ask_prompt(
            crate::i18n::tr(self.feature),
            &format!("{} (pid {})", self.holder, self.pid),
        )
    }
}

/// Demande type : laisser hyprdesk prendre en charge `feature`, tenue par `holder`.
pub fn ask_prompt(feature: &str, holder: &str) -> String {
    crate::tr!(
        "Dans ma session Hyprland, hyprdesk ne peut pas prendre en charge « {feature} » : {holder} le fait déjà. Trouve d'où il est lancé (hyprland.lua, exec-once, services systemd --user, ~/.config/autostart…), explique-moi, puis propose la modification qui laisse hyprdesk s'en charger sans rien casser. Demande-moi avant de modifier quoi que ce soit, et ne touche jamais à inputs.lua ni à monitors.lua.",
        feature = feature,
        holder = holder
    )
}

/// Programmes qui tiennent chaque service (noms de processus).
const HOLDERS: &[(&str, &[&str])] = &[
    (
        "Notifications",
        &[
            "wayle",
            "mako",
            "dunst",
            "swaync",
            "fnott",
            "notification-daemon",
            "xfce4-notifyd",
        ],
    ),
    (
        "Authentification administrateur (agent polkit)",
        &[
            "polkit-kde-authentication-agent-1",
            "polkit-gnome-authentication-agent-1",
            "hyprpolkitagent",
            "lxpolkit",
            "polkit-mate-authentication-agent-1",
            "polkit-agent-helper",
        ],
    ),
    ("Éclairage nocturne", &["wlsunset", "gammastep", "redshift"]),
];

/// Processus `name` → pid, parmi `/proc` (nom court du processus).
fn processes() -> Vec<(String, u32)> {
    std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let pid = e.file_name().to_str()?.parse::<u32>().ok()?;
            let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
            Some((comm.trim().to_string(), pid))
        })
        .collect()
}

/// Conflits d'après la liste des processus. `notifications_ours` : hyprdesk
/// tient déjà le service de notifications.
pub fn detect(procs: &[(String, u32)], notifications_ours: bool) -> Vec<Conflict> {
    let mut out = Vec::new();
    for (feature, names) in HOLDERS {
        if *feature == "Notifications" && notifications_ours {
            continue;
        }
        // Les noms courts de /proc sont tronqués à 15 caractères.
        if let Some((name, pid)) = procs.iter().find(|(c, _)| {
            names
                .iter()
                .any(|n| c == n || (c.len() == 15 && n.starts_with(c.as_str())))
        }) {
            out.push(Conflict {
                feature,
                holder: name.clone(),
                pid: *pid,
            });
        }
    }
    out
}

/// Relevé complet (processus, veille, presse-papiers), récent : l'écran
/// Réglages se redessine souvent.
pub fn cached(notifications_ours: bool, clipboard_on: bool) -> Vec<Conflict> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    type Cached = (Instant, bool, bool, Vec<Conflict>);
    static CACHE: Mutex<Option<Cached>> = Mutex::new(None);
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, n, cl, list)) = &*c
        && at.elapsed() < Duration::from_secs(3)
        && *n == notifications_ours
        && *cl == clipboard_on
    {
        return list.clone();
    }
    let mut list = detect(&processes(), notifications_ours);
    if let Some(pid) = crate::idle::foreign_cached() {
        list.push(Conflict {
            feature: "Veille et verrouillage automatique",
            holder: "hypridle".into(),
            pid,
        });
    }
    if clipboard_on && let Some(pid) = crate::clipboard::foreign_watcher() {
        list.push(Conflict {
            feature: "Historique du presse-papiers",
            holder: "wl-paste --watch cliphist".into(),
            pid,
        });
    }
    *c = Some((
        Instant::now(),
        notifications_ours,
        clipboard_on,
        list.clone(),
    ));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn services_tenus() {
        let procs = vec![
            ("wayle".to_string(), 10),
            ("polkit-kde-auth".to_string(), 11),
            ("wlsunset".to_string(), 12),
            ("kitty".to_string(), 13),
        ];
        let c = detect(&procs, false);
        let f: Vec<&str> = c.iter().map(|c| c.feature).collect();
        assert_eq!(
            f,
            [
                "Notifications",
                "Authentification administrateur (agent polkit)",
                "Éclairage nocturne"
            ]
        );
        assert_eq!(c[1].pid, 11);
        // Notifications tenues par hyprdesk : pas de conflit.
        assert_eq!(detect(&procs, true).len(), 2);
        assert!(c[2].prompt().contains("wlsunset (pid 12)"));
    }
}
