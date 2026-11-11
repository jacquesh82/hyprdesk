//! Dépendances externes de hyprdesk (`hyprdesk doctor`, Réglages → Bureau →
//! Dépendances).
//!
//! hyprdesk dessine lui-même ce qui se voit, mais confie à d'autres outils ce
//! qui ne se voit pas (veille, presse-papiers, captures…) : chacun est relevé
//! ici, avec ce qui en dépend et le paquet qui le fournit. Un outil absent
//! n'empêche rien de démarrer ; la fonction qui en dépend reste simplement
//! inopérante.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Comment savoir qu'une dépendance est là : un des programmes dans le PATH,
/// ou un des fichiers présent.
#[derive(Debug)]
pub enum Probe {
    Program(&'static [&'static str]),
    File(&'static [&'static str]),
}

#[derive(Debug)]
pub struct Dep {
    /// Nom affiché (programme ou composant).
    pub name: &'static str,
    pub probe: Probe,
    /// Paquet qui la fournit (Arch Linux).
    pub package: &'static str,
    /// Ce qui en dépend, en français (clé de traduction).
    pub feature: &'static str,
    /// Sans elle, hyprdesk ne fonctionne pas du tout.
    pub essential: bool,
}

pub const DEPS: &[Dep] = &[
    Dep {
        name: "hyprctl",
        probe: Probe::Program(&["hyprctl"]),
        package: "hyprland",
        feature: "Hyprland : fenêtres, workspaces, raccourcis",
        essential: true,
    },
    Dep {
        name: "wpctl",
        probe: Probe::Program(&["wpctl"]),
        package: "wireplumber",
        feature: "Son : volume, sorties et entrées",
        essential: false,
    },
    Dep {
        name: "nmcli",
        probe: Probe::Program(&["nmcli"]),
        package: "networkmanager",
        feature: "Réseau : Wi-Fi, Ethernet, VPN",
        essential: false,
    },
    Dep {
        name: "bluetoothctl",
        probe: Probe::Program(&["bluetoothctl"]),
        package: "bluez-utils",
        feature: "Bluetooth",
        essential: false,
    },
    Dep {
        name: "powerprofilesctl",
        probe: Probe::Program(&["powerprofilesctl"]),
        package: "power-profiles-daemon",
        feature: "Profils d'énergie",
        essential: false,
    },
    Dep {
        name: "hypridle",
        probe: Probe::Program(&["hypridle"]),
        package: "hypridle",
        feature: "Veille et verrouillage automatique",
        essential: false,
    },
    Dep {
        name: "checkupdates",
        probe: Probe::Program(&["checkupdates"]),
        package: "pacman-contrib",
        feature: "Mises à jour en attente (Arch)",
        essential: false,
    },
    Dep {
        name: "brightnessctl",
        probe: Probe::Program(&["brightnessctl"]),
        package: "brightnessctl",
        feature: "Assombrir l'écran à l'inactivité",
        essential: false,
    },
    Dep {
        name: "polkit-agent-helper-1",
        probe: Probe::File(&[
            "/usr/lib/polkit-1/polkit-agent-helper-1",
            "/usr/libexec/polkit-agent-helper-1",
            "/usr/lib/polkit-agent-helper-1",
        ]),
        package: "polkit",
        feature: "Authentification administrateur (agent polkit)",
        essential: false,
    },
    Dep {
        name: "xdg-mime",
        probe: Probe::Program(&["xdg-mime"]),
        package: "xdg-utils",
        feature: "Applications par défaut, ouverture des fichiers",
        essential: false,
    },
    Dep {
        name: "xdg-desktop-portal-hyprland",
        probe: Probe::File(&["/usr/share/xdg-desktop-portal/portals/hyprland.portal"]),
        package: "xdg-desktop-portal-hyprland",
        feature: "Partage d'écran par les applications",
        essential: false,
    },
    Dep {
        name: "xdg-desktop-portal-gtk",
        probe: Probe::File(&["/usr/share/xdg-desktop-portal/portals/gtk.portal"]),
        package: "xdg-desktop-portal-gtk",
        feature: "Sélecteur de fichiers des applications",
        essential: false,
    },
    Dep {
        name: "hyprshot",
        probe: Probe::Program(&["hyprshot"]),
        package: "hyprshot",
        feature: "Captures d'écran",
        essential: false,
    },
    Dep {
        name: "wf-recorder",
        probe: Probe::Program(&["wf-recorder"]),
        package: "wf-recorder",
        feature: "Enregistrement de l'écran",
        essential: false,
    },
    Dep {
        name: "slurp",
        probe: Probe::Program(&["slurp"]),
        package: "slurp",
        feature: "Enregistrement d'une zone de l'écran",
        essential: false,
    },
    Dep {
        name: "wl-paste",
        probe: Probe::Program(&["wl-paste"]),
        package: "wl-clipboard",
        feature: "Presse-papiers",
        essential: false,
    },
    Dep {
        name: "cliphist",
        probe: Probe::Program(&["cliphist"]),
        package: "cliphist",
        feature: "Historique du presse-papiers",
        essential: false,
    },
    Dep {
        name: "hyprsunset",
        probe: Probe::Program(&["hyprsunset"]),
        package: "hyprsunset",
        feature: "Éclairage nocturne",
        essential: false,
    },
    Dep {
        name: "wayle",
        probe: Probe::Program(&["wayle"]),
        package: "wayle",
        feature: "Notifications (en attendant le démon intégré)",
        essential: false,
    },
];

/// Une dépendance relevée : ce qui l'a trouvée (programme ou chemin), sinon rien.
#[derive(Debug, Clone)]
pub struct Status {
    pub dep: &'static Dep,
    pub found: Option<String>,
}

fn probe_with(
    p: &Probe,
    has_program: impl Fn(&str) -> bool,
    has_file: impl Fn(&str) -> bool,
) -> Option<String> {
    match p {
        Probe::Program(names) => names.iter().find(|n| has_program(n)).map(|n| n.to_string()),
        Probe::File(paths) => paths.iter().find(|f| has_file(f)).map(|f| f.to_string()),
    }
}

/// Relève toutes les dépendances, maintenant.
pub fn check() -> Vec<Status> {
    DEPS.iter()
        .map(|dep| Status {
            dep,
            found: probe_with(&dep.probe, crate::launch::executable, |f| {
                Path::new(f).exists()
            }),
        })
        .collect()
}

/// Relevé récent (quelques secondes) : l'écran Réglages le redessine souvent.
pub fn cached() -> Vec<Status> {
    static CACHE: Mutex<Option<(Instant, Vec<Status>)>> = Mutex::new(None);
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    match &*c {
        Some((at, list)) if at.elapsed() < Duration::from_secs(5) => list.clone(),
        _ => {
            let list = check();
            *c = Some((Instant::now(), list.clone()));
            list
        }
    }
}

/// Rapport texte pour `hyprdesk doctor`.
pub fn report(list: &[Status]) -> String {
    let width = list.iter().map(|s| s.dep.name.len()).max().unwrap_or(0);
    let mut out = format!(
        "{}\n\n",
        crate::i18n::tr("hyprdesk doctor — dépendances externes")
    );
    for s in list {
        let feature = crate::i18n::tr(s.dep.feature);
        match &s.found {
            Some(_) => out.push_str(&format!("  ✓ {:<width$}  {feature}\n", s.dep.name)),
            None => {
                let pkg = crate::tr!("paquet : {p}", p = s.dep.package);
                let tag = if s.dep.essential {
                    format!(" — {}", crate::i18n::tr("indispensable"))
                } else {
                    String::new()
                };
                out.push_str(&format!(
                    "  ✗ {:<width$}  {feature} ({pkg}){tag}\n",
                    s.dep.name
                ));
            }
        }
    }
    let present = list.iter().filter(|s| s.found.is_some()).count();
    out.push('\n');
    out.push_str(&crate::tr!(
        "{n}/{m} présentes ; une dépendance absente rend seulement sa fonction inopérante.",
        n = present,
        m = list.len()
    ));
    out
}

/// Une dépendance indispensable manque.
pub fn essential_missing(list: &[Status]) -> bool {
    list.iter().any(|s| s.dep.essential && s.found.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sondes_programme_et_fichier() {
        let p = Probe::Program(&["absent", "present"]);
        assert_eq!(
            probe_with(&p, |n| n == "present", |_| false).as_deref(),
            Some("present")
        );
        assert_eq!(probe_with(&p, |_| false, |_| true), None);
        let f = Probe::File(&["/a", "/b"]);
        assert_eq!(
            probe_with(&f, |_| true, |p| p == "/b").as_deref(),
            Some("/b")
        );
    }

    #[test]
    fn rapport_et_indispensables() {
        let list: Vec<Status> = DEPS
            .iter()
            .map(|dep| Status {
                dep,
                found: (dep.name != "hypridle").then(|| dep.name.to_string()),
            })
            .collect();
        let r = report(&list);
        assert!(r.contains("✗ hypridle"));
        assert!(r.contains("hypridle)"), "le paquet est indiqué : {r}");
        assert!(r.contains(&format!("{}/{}", DEPS.len() - 1, DEPS.len())));
        assert!(!essential_missing(&list));
        let none: Vec<Status> = DEPS.iter().map(|dep| Status { dep, found: None }).collect();
        assert!(essential_missing(&none));
    }

    #[test]
    fn noms_uniques_et_fonctions_traduites() {
        let mut names: Vec<&str> = DEPS.iter().map(|d| d.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), DEPS.len());
        let missing = crate::i18n::untranslated("doctor", DEPS.iter().map(|d| d.feature));
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }
}
