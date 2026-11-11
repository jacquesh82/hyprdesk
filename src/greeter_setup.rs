//! Mise en place de l'écran de connexion (MLG-42) : greetd avec
//! `hyprdesk greeter` à la place du gestionnaire de connexion en service
//! (GDM, SDDM, LightDM…).
//!
//! Lecture sans droits (`status`) pour Réglages → Écran de connexion ; les
//! gestes système passent par `pkexec hyprdesk greeter setup <action>`,
//! exécuté en root :
//!
//! - `packages` : installe greetd et cage (pacman, apt, dnf, zypper) ;
//! - `install` : copie hyprdesk dans `/usr/local/bin` (l'utilisateur
//!   `greeter` ne lit pas `~/.local/bin`) et écrit `/etc/greetd/config.toml`
//!   (l'ancien, d'un autre outil, gardé une fois en `.bak`) ;
//! - `enable` : désactive le gestionnaire en service, active greetd (au
//!   prochain démarrage : la session ouverte n'est pas coupée) ; refusé tant
//!   qu'il manque une pièce, pour ne jamais laisser la machine sans écran
//!   de connexion ;
//! - `restore` : revient au gestionnaire d'avant (noté par `enable`).

use std::path::{Path, PathBuf};
use std::process::Command;

pub const CONFIG: &str = "/etc/greetd/config.toml";
/// Copie de hyprdesk lisible par l'utilisateur `greeter`.
pub const SYSTEM_BIN: &str = "/usr/local/bin/hyprdesk";
/// Binaire installé par le paquet (nfpm), lisible par tous.
const PACKAGED_BIN: &str = "/usr/bin/hyprdesk";
/// Gestionnaire de connexion d'avant la bascule (`restore` y revient).
const PREVIOUS: &str = "/etc/greetd/hyprdesk-precedent";
const DM_LINK: &str = "/etc/systemd/system/display-manager.service";
/// Repère de l'en-tête du fichier écrit par hyprdesk.
const HEADER: &str = "# Écrit par hyprdesk (Réglages → Écran de connexion).";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// Unité derrière display-manager.service (« gdm.service »), s'il y en a.
    pub dm: Option<String>,
    /// Cette unité tourne en ce moment (sinon : au prochain démarrage).
    pub dm_active: bool,
    pub greetd: bool,
    pub cage: bool,
    /// Utilisateur système `greeter` (créé par le paquet greetd).
    pub greeter_user: bool,
    /// hyprdesk lisible par greetd (/usr/bin ou /usr/local/bin).
    pub binary: Option<String>,
    /// La copie de /usr/local/bin est plus ancienne que le hyprdesk lancé.
    pub binary_stale: bool,
    /// `default_session.command` de /etc/greetd/config.toml.
    pub command: Option<String>,
    /// La commande lance `hyprdesk greeter`.
    pub ours: bool,
    /// Gestionnaire d'avant la bascule.
    pub previous: Option<String>,
    pub loaded: bool,
}

impl Status {
    /// Tout est en place pour basculer sur greetd sans risque.
    pub fn ready(&self) -> bool {
        self.greetd && self.cage && self.greeter_user && self.binary.is_some() && self.ours
    }

    /// greetd tient (ou tiendra) l'écran de connexion.
    pub fn on_greetd(&self) -> bool {
        self.dm.as_deref() == Some("greetd.service")
    }
}

/// Nom lisible d'une unité de gestionnaire de connexion.
pub fn dm_label(unit: &str) -> String {
    match unit.trim_end_matches(".service") {
        "gdm" | "gdm3" => "GDM".into(),
        "sddm" => "SDDM".into(),
        "lightdm" => "LightDM".into(),
        "lxdm" => "LXDM".into(),
        "ly" => "Ly".into(),
        "greetd" => "greetd".into(),
        other => other.to_string(),
    }
}

/// `[default_session] command` et `[terminal] vt` d'un config.toml de greetd.
pub fn parse_config(src: &str) -> (Option<String>, Option<i64>) {
    let Ok(v) = src.parse::<toml::Table>() else {
        return (None, None);
    };
    let command = v
        .get("default_session")
        .and_then(|s| s.get("command"))
        .and_then(|c| c.as_str())
        .map(str::to_string);
    let vt = v
        .get("terminal")
        .and_then(|t| t.get("vt"))
        .and_then(|n| n.as_integer());
    (command, vt)
}

/// La commande lance l'écran de hyprdesk.
pub fn is_ours(command: &str) -> bool {
    command.contains("hyprdesk greeter")
}

/// Commande de greetd pour l'écran de hyprdesk, sous cage (`-s` : les
/// changements de console restent possibles).
pub fn greeter_command(bin: &str) -> String {
    format!("cage -s -- {bin} greeter")
}

/// /etc/greetd/config.toml de hyprdesk (`vt` repris de l'ancien fichier).
pub fn config_toml(bin: &str, vt: i64) -> String {
    format!(
        "{HEADER}\n# Revenir en arrière : sudo hyprdesk greeter setup restore\n\n\
         [terminal]\nvt = {vt}\n\n\
         [default_session]\ncommand = \"{}\"\nuser = \"greeter\"\n",
        greeter_command(bin)
    )
}

fn unit_of_link(link: &Path) -> Option<String> {
    std::fs::read_link(link)
        .ok()?
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}

fn is_active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", unit])
        .status()
        .is_ok_and(|s| s.success())
}

fn user_exists(name: &str) -> bool {
    std::fs::read_to_string("/etc/passwd")
        .unwrap_or_default()
        .lines()
        .any(|l| l.split(':').next() == Some(name))
}

/// hyprdesk lancé, en suivant les liens.
fn current_exe() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(p).ok())
}

/// La copie système est plus ancienne que `exe` (taille différente, ou
/// fichier lancé modifié après la copie).
fn stale(system: &Path, exe: &Path) -> bool {
    let (Ok(s), Ok(e)) = (std::fs::metadata(system), std::fs::metadata(exe)) else {
        return false;
    };
    if s.len() != e.len() {
        return true;
    }
    matches!((s.modified(), e.modified()), (Ok(a), Ok(b)) if b > a)
}

pub fn status() -> Status {
    let dm = unit_of_link(Path::new(DM_LINK));
    let exe = current_exe();
    let binary = [PACKAGED_BIN, SYSTEM_BIN]
        .into_iter()
        .find(|p| Path::new(p).is_file())
        .map(str::to_string);
    let binary_stale = binary.as_deref() == Some(SYSTEM_BIN)
        && exe
            .as_deref()
            .is_some_and(|e| e != Path::new(SYSTEM_BIN) && stale(Path::new(SYSTEM_BIN), e));
    let (command, _) = parse_config(&std::fs::read_to_string(CONFIG).unwrap_or_default());
    Status {
        dm_active: dm.as_deref().is_some_and(is_active),
        dm,
        greetd: crate::doctor::has("greetd"),
        cage: crate::doctor::has("cage"),
        greeter_user: user_exists("greeter"),
        binary,
        binary_stale,
        ours: command.as_deref().is_some_and(is_ours),
        command,
        previous: std::fs::read_to_string(PREVIOUS)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        loaded: true,
    }
}

/// Arguments de `pkexec` pour une action (`hyprdesk greeter setup <action>`).
pub fn pkexec_argv(action: &str) -> Vec<String> {
    let exe = current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "hyprdesk".into());
    vec![
        "pkexec".into(),
        exe,
        "greeter".into(),
        "setup".into(),
        action.into(),
    ]
}

/// Lance l'action par pkexec (bloquant) ; `Err` : sortie d'erreur.
pub fn run_privileged(action: &str) -> Result<(), String> {
    let argv = pkexec_argv(action);
    let out = Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(match out.status.code() {
        // pkexec : authentification refusée ou fenêtre fermée.
        Some(126) | Some(127) if err.is_empty() => {
            crate::i18n::tr("Authentification annulée.").to_string()
        }
        _ if err.is_empty() => crate::i18n::tr("Échec, sans message.").to_string(),
        _ => err,
    })
}

fn run(argv: &[&str]) -> Result<(), String> {
    let st = Command::new(argv[0])
        .args(&argv[1..])
        .status()
        .map_err(|e| format!("{} : {e}", argv[0]))?;
    if st.success() {
        Ok(())
    } else {
        Err(crate::tr!("échec de « {c} »", c = argv.join(" ")))
    }
}

fn has(p: &str) -> bool {
    crate::doctor::has(p)
}

/// Commande d'installation de greetd et cage selon la distribution.
pub fn packages_argv(has: impl Fn(&str) -> bool) -> Option<Vec<&'static str>> {
    if has("pacman") {
        Some(vec![
            "pacman",
            "-S",
            "--needed",
            "--noconfirm",
            "greetd",
            "cage",
        ])
    } else if has("apt-get") {
        Some(vec!["apt-get", "install", "-y", "greetd", "cage"])
    } else if has("dnf") {
        Some(vec!["dnf", "install", "-y", "greetd", "cage"])
    } else if has("zypper") {
        Some(vec![
            "zypper",
            "--non-interactive",
            "install",
            "greetd",
            "cage",
        ])
    } else {
        None
    }
}

fn packages() -> Result<(), String> {
    let argv = packages_argv(has).ok_or_else(|| {
        crate::i18n::tr("Gestionnaire de paquets inconnu : installez greetd et cage à la main.")
            .to_string()
    })?;
    run(&argv)
}

/// Copie hyprdesk dans /usr/local/bin (sauf binaire du paquet) et écrit la
/// configuration de greetd.
fn install() -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    let bin = if Path::new(PACKAGED_BIN).is_file() {
        PACKAGED_BIN
    } else {
        let exe =
            current_exe().ok_or_else(|| crate::i18n::tr("hyprdesk introuvable.").to_string())?;
        if exe != Path::new(SYSTEM_BIN) {
            std::fs::create_dir_all("/usr/local/bin").map_err(io)?;
            // Copie à côté puis renommage : un greeter lancé n'est pas coupé.
            let tmp = format!("{SYSTEM_BIN}.nouveau");
            std::fs::copy(&exe, &tmp).map_err(io)?;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(io)?;
            std::fs::rename(&tmp, SYSTEM_BIN).map_err(io)?;
        }
        SYSTEM_BIN
    };
    std::fs::create_dir_all("/etc/greetd").map_err(io)?;
    let old = std::fs::read_to_string(CONFIG).unwrap_or_default();
    let bak = format!("{CONFIG}.bak");
    if !old.is_empty() && !old.starts_with(HEADER) && !Path::new(&bak).exists() {
        std::fs::write(&bak, &old).map_err(io)?;
    }
    let vt = parse_config(&old).1.unwrap_or(1);
    std::fs::write(CONFIG, config_toml(bin, vt)).map_err(io)
}

/// Bascule sur greetd (au prochain démarrage).
fn enable() -> Result<(), String> {
    let s = status();
    if !s.ready() {
        return Err(crate::i18n::tr(
            "Il manque une pièce (greetd, cage, utilisateur greeter, hyprdesk dans /usr, \
             configuration) : rien n'a été changé.",
        )
        .to_string());
    }
    if let Some(dm) = s.dm.as_deref().filter(|d| *d != "greetd.service") {
        std::fs::write(PREVIOUS, format!("{dm}\n")).map_err(|e| e.to_string())?;
        run(&["systemctl", "disable", dm])?;
    }
    run(&["systemctl", "enable", "greetd.service"])
}

/// Revient au gestionnaire d'avant (GDM à défaut).
fn restore() -> Result<(), String> {
    let prev = std::fs::read_to_string(PREVIOUS)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "greetd.service")
        .unwrap_or_else(|| "gdm.service".into());
    let _ = run(&["systemctl", "disable", "greetd.service"]);
    run(&["systemctl", "enable", &prev])
}

/// `hyprdesk greeter setup <action>` (en root).
pub fn cli(action: Option<&str>) -> Result<(), String> {
    let f: fn() -> Result<(), String> = match action {
        Some("packages") => packages,
        Some("install") => install,
        Some("enable") => enable,
        Some("restore") => restore,
        Some("status") | None => {
            let s = status();
            println!(
                "{}",
                crate::tr!(
                    "gestionnaire : {dm}\ngreetd : {g}\ncage : {c}\nutilisateur greeter : {u}\n\
                     hyprdesk système : {b}\ncommande : {cmd}",
                    dm = s.dm.as_deref().unwrap_or("—"),
                    g = s.greetd,
                    c = s.cage,
                    u = s.greeter_user,
                    b = s.binary.as_deref().unwrap_or("—"),
                    cmd = s.command.as_deref().unwrap_or("—")
                )
            );
            return Ok(());
        }
        Some(other) => {
            return Err(crate::tr!(
                "action inconnue : {a} (packages, install, enable, restore, status)",
                a = other
            ));
        }
    };
    // SAFETY: geteuid n'a pas de précondition.
    if unsafe { libc::geteuid() } != 0 {
        return Err(crate::i18n::tr(
            "à lancer en root (pkexec ou sudo) : l'action touche /etc et systemd.",
        )
        .to_string());
    }
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_de_greetd_lue() {
        let src = "[terminal]\nvt = 2\n\n[default_session]\ncommand = \"agreety --cmd /bin/sh\"\nuser = \"greeter\"\n";
        assert_eq!(
            parse_config(src),
            (Some("agreety --cmd /bin/sh".into()), Some(2))
        );
        assert_eq!(parse_config("pas du toml = ["), (None, None));
    }

    #[test]
    fn config_ecrite_relue() {
        let src = config_toml(SYSTEM_BIN, 1);
        assert!(src.starts_with(HEADER));
        let (cmd, vt) = parse_config(&src);
        assert_eq!(
            cmd.as_deref(),
            Some("cage -s -- /usr/local/bin/hyprdesk greeter")
        );
        assert_eq!(vt, Some(1));
        assert!(is_ours(&cmd.unwrap()));
        assert!(!is_ours("tuigreet --cmd Hyprland"));
    }

    #[test]
    fn paquets_selon_la_distribution() {
        let arch = packages_argv(|p| p == "pacman").unwrap();
        assert_eq!(arch[0], "pacman");
        assert!(arch.contains(&"greetd") && arch.contains(&"cage"));
        assert_eq!(packages_argv(|p| p == "apt-get").unwrap()[0], "apt-get");
        assert!(packages_argv(|_| false).is_none());
    }

    #[test]
    fn noms_des_gestionnaires() {
        assert_eq!(dm_label("gdm.service"), "GDM");
        assert_eq!(dm_label("sddm.service"), "SDDM");
        assert_eq!(dm_label("autre.service"), "autre");
    }

    #[test]
    fn pret_seulement_complet() {
        let mut s = Status {
            greetd: true,
            cage: true,
            greeter_user: true,
            binary: Some(SYSTEM_BIN.into()),
            ours: true,
            ..Default::default()
        };
        assert!(s.ready());
        s.ours = false;
        assert!(!s.ready());
    }
}
