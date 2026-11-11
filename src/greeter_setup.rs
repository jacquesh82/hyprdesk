//! Mise en place de l'écran de connexion (MLG-42, MLG-43) : greetd avec
//! `hyprdesk greeter` à la place du gestionnaire de connexion en service
//! (GDM, SDDM, LightDM…).
//!
//! Lecture sans droits (`status`) pour Réglages → Écran de connexion ; les
//! gestes système passent par `pkexec hyprdesk greeter setup <action>`,
//! exécuté en root :
//!
//! - `packages` : installe greetd et cage (pacman, apt, dnf, zypper) ;
//! - `install <profil.json>` : copie hyprdesk dans `/usr/local/bin`
//!   (l'utilisateur `greeter` ne lit pas `~/.local/bin`), installe le profil
//!   de l'écran (`/etc/greetd/hyprdesk.json` : disposition du clavier,
//!   couleurs, empreinte), écrit `/etc/greetd/config.toml` (l'ancien, d'un
//!   autre outil, gardé une fois en `.bak`), pose les lignes PAM de hyprdesk
//!   dans `/etc/pam.d/greetd` (empreinte, trousseau myMFA) et le dossier
//!   d'état du greeter (dernier utilisateur) ;
//! - `enable` : désactive le gestionnaire en service, active greetd (au
//!   prochain démarrage : la session ouverte n'est pas coupée) ; refusé tant
//!   qu'il manque une pièce, pour ne jamais laisser la machine sans écran
//!   de connexion ;
//! - `restore` : revient au gestionnaire d'avant (noté par `enable`).

use std::path::{Path, PathBuf};
use std::process::Command;

pub const CONFIG: &str = "/etc/greetd/config.toml";
/// Profil de l'écran (lisible par l'utilisateur `greeter`).
pub const PROFILE: &str = "/etc/greetd/hyprdesk.json";
pub const PAM: &str = "/etc/pam.d/greetd";
/// Dossier d'état du greeter (à l'utilisateur `greeter`) : dernier
/// utilisateur et dernière session.
pub const STATE_DIR: &str = "/var/lib/hyprdesk-greeter";
/// Copie de hyprdesk lisible par l'utilisateur `greeter`.
pub const SYSTEM_BIN: &str = "/usr/local/bin/hyprdesk";
/// Binaire installé par le paquet (nfpm), lisible par tous.
const PACKAGED_BIN: &str = "/usr/bin/hyprdesk";
/// Gestionnaire de connexion d'avant la bascule (`restore` y revient).
const PREVIOUS: &str = "/etc/greetd/hyprdesk-precedent";
const DM_LINK: &str = "/etc/systemd/system/display-manager.service";
/// Repère de l'en-tête du fichier écrit par hyprdesk.
const HEADER: &str = "# Écrit par hyprdesk (Réglages → Écran de connexion).";
/// Repère des lignes PAM de hyprdesk : la ligne qui le suit est à lui.
const PAM_MARK: &str = "# hyprdesk (Réglages → Écran de connexion) :";
/// Module PAM du trousseau myMFA (client Linux, ticket MFA LIN-03).
const MYMFA_PAM: [&str; 3] = [
    "/usr/lib/security/pam_mymfa.so",
    "/lib/security/pam_mymfa.so",
    "/usr/lib64/security/pam_mymfa.so",
];

/// Profil de l'écran de connexion, repris des réglages de l'utilisateur.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Disposition xkb (« fr », « fr,us »), variante, options.
    pub layout: String,
    pub variant: String,
    pub options: String,
    /// Nom du thème et ses couleurs (RGBA).
    pub theme: String,
    pub fg: [u8; 4],
    pub bg: [u8; 4],
    pub accent: [u8; 4],
    /// Empreinte digitale à la connexion (pam_fprintd).
    pub fingerprint: bool,
}

impl Profile {
    /// Thème de l'écran (thème par défaut si le profil n'en a pas).
    pub fn theme(&self) -> crate::config::Theme {
        let mut t = crate::config::Theme::default();
        if self.bg[3] != 0 {
            let c = |v: [u8; 4]| crate::config::Rgba(v[0], v[1], v[2], v[3]);
            t.name = self.theme.clone();
            t.fg = c(self.fg);
            t.bg = c(self.bg);
            t.accent = c(self.accent);
        }
        t
    }

    /// Profil installé (absent ou illisible : aucun).
    pub fn installed() -> Option<Profile> {
        serde_json::from_str(&std::fs::read_to_string(PROFILE).ok()?).ok()
    }
}

/// Valeur xkb sûre dans une commande (lettres, chiffres, `_,:+-().`).
pub fn xkb_ok(v: &str) -> bool {
    v.len() < 200
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_,:+-().".contains(c))
}

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
    /// Profil installé dans /etc/greetd.
    pub profile: Option<Profile>,
    /// Lignes PAM de hyprdesk présentes dans /etc/pam.d/greetd.
    pub pam_fingerprint: bool,
    pub pam_mymfa: bool,
    /// Module pam_mymfa.so installé (client Linux de myMFA).
    pub mymfa_module: bool,
    pub fprintd: bool,
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
/// changements de console restent possibles). La disposition du clavier
/// passe à cage par l'environnement (XKB_DEFAULT_*), sinon le mot de passe
/// se taperait en QWERTY.
pub fn greeter_command(bin: &str, p: &Profile) -> String {
    let mut env = Vec::new();
    for (k, v) in [
        ("XKB_DEFAULT_LAYOUT", &p.layout),
        ("XKB_DEFAULT_VARIANT", &p.variant),
        ("XKB_DEFAULT_OPTIONS", &p.options),
    ] {
        if !v.is_empty() && xkb_ok(v) {
            env.push(format!("{k}={v}"));
        }
    }
    let cage = format!("cage -s -- {bin} greeter");
    if env.is_empty() {
        cage
    } else {
        format!("env {} {cage}", env.join(" "))
    }
}

/// /etc/greetd/config.toml de hyprdesk (`vt` repris de l'ancien fichier).
pub fn config_toml(bin: &str, vt: i64, p: &Profile) -> String {
    format!(
        "{HEADER}\n# Revenir en arrière : sudo hyprdesk greeter setup restore\n\n\
         [terminal]\nvt = {vt}\n\n\
         [default_session]\ncommand = \"{}\"\nuser = \"greeter\"\n",
        greeter_command(bin, p)
    )
}

/// Ligne PAM effective (commentaires et espaces retirés) : type de module.
fn pam_kind(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        return None;
    }
    t.split_whitespace()
        .next()
        .map(|k| k.trim_start_matches('-'))
}

/// /etc/pam.d/greetd avec les lignes de hyprdesk : empreinte avant la pile
/// d'authentification (`sufficient`, un essai, 10 s : le mot de passe est
/// demandé ensuite), trousseau myMFA après l'authentification et après
/// l'ouverture de session (`optional` : il ne bloque jamais la connexion).
/// Les anciennes lignes de hyprdesk (repère + ligne suivante) sont retirées.
pub fn pam_with(src: &str, fingerprint: bool, mymfa: bool) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut skip = false;
    for l in src.lines() {
        if skip {
            skip = false;
            continue;
        }
        if l.trim_start().starts_with(PAM_MARK) {
            skip = true;
            continue;
        }
        lines.push(l.to_string());
    }
    let mark = |what: &str| format!("{PAM_MARK} {what}");
    if fingerprint && let Some(i) = lines.iter().position(|l| pam_kind(l) == Some("auth")) {
        // Après pam_securetty / pam_nologin s'ils sont en tête : avant la
        // première ligne qui vérifie vraiment (include, substack, pam_unix…).
        let j = lines[i..]
            .iter()
            .position(|l| {
                pam_kind(l) == Some("auth")
                    && !l.contains("pam_securetty")
                    && !l.contains("pam_nologin")
            })
            .map(|k| i + k)
            .unwrap_or(i);
        lines.insert(
            j,
            "auth       sufficient   pam_fprintd.so max-tries=1 timeout=10".into(),
        );
        lines.insert(j, mark("empreinte digitale"));
    }
    if mymfa {
        for kind in ["auth", "session"] {
            if let Some(i) = lines.iter().rposition(|l| pam_kind(l) == Some(kind)) {
                lines.insert(i + 1, format!("{kind:<10} optional     pam_mymfa.so"));
                lines.insert(i + 1, mark("trousseau myMFA"));
            }
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Lignes de hyprdesk présentes : (empreinte, myMFA).
pub fn pam_state(src: &str) -> (bool, bool) {
    let mut it = src.lines().peekable();
    let (mut f, mut m) = (false, false);
    while let Some(l) = it.next() {
        if l.trim_start().starts_with(PAM_MARK)
            && let Some(next) = it.peek()
        {
            f |= next.contains("pam_fprintd");
            m |= next.contains("pam_mymfa");
        }
    }
    (f, m)
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

/// uid et gid d'un utilisateur (/etc/passwd).
fn user_ids(name: &str) -> Option<(u32, u32)> {
    std::fs::read_to_string("/etc/passwd")
        .ok()?
        .lines()
        .map(|l| l.split(':').collect::<Vec<_>>())
        .find(|f| f.first() == Some(&name))
        .and_then(|f| Some((f.get(2)?.parse().ok()?, f.get(3)?.parse().ok()?)))
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

fn mymfa_module() -> bool {
    MYMFA_PAM.iter().any(|p| Path::new(p).is_file())
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
    let (pam_fingerprint, pam_mymfa) = pam_state(&std::fs::read_to_string(PAM).unwrap_or_default());
    Status {
        dm_active: dm.as_deref().is_some_and(is_active),
        dm,
        greetd: crate::doctor::has("greetd"),
        cage: crate::doctor::has("cage"),
        greeter_user: user_ids("greeter").is_some(),
        binary,
        binary_stale,
        ours: command.as_deref().is_some_and(is_ours),
        command,
        previous: std::fs::read_to_string(PREVIOUS)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        profile: Profile::installed(),
        pam_fingerprint,
        pam_mymfa,
        mymfa_module: mymfa_module(),
        fprintd: crate::doctor::has("fprintd"),
        loaded: true,
    }
}

/// Arguments de `pkexec` pour une action (`hyprdesk greeter setup <action>`).
pub fn pkexec_argv(action: &str, extra: &[String]) -> Vec<String> {
    let exe = current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "hyprdesk".into());
    let mut v = vec![
        "pkexec".into(),
        exe,
        "greeter".into(),
        "setup".into(),
        action.into(),
    ];
    v.extend(extra.iter().cloned());
    v
}

/// Profil à installer, écrit pour root dans le dossier d'exécution de
/// l'utilisateur ; renvoie son chemin.
pub fn stage_profile(p: &Profile) -> Result<String, String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("hyprdesk");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("greeter-profile.json");
    let body = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// Lance l'action par pkexec (bloquant) ; `Err` : sortie d'erreur.
pub fn run_privileged(action: &str, extra: &[String]) -> Result<(), String> {
    let argv = pkexec_argv(action, extra);
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

/// Profil lu (fichier préparé par Réglages) et assaini.
fn read_profile(path: Option<&str>) -> Result<Profile, String> {
    let Some(path) = path else {
        return Ok(Profile::installed().unwrap_or_default());
    };
    let mut p: Profile =
        serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| format!("{path} : {e}"))?)
            .map_err(|e| format!("{path} : {e}"))?;
    for v in [&mut p.layout, &mut p.variant, &mut p.options] {
        if !xkb_ok(v) {
            v.clear();
        }
    }
    p.theme = p
        .theme
        .chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect();
    Ok(p)
}

/// Copie hyprdesk dans /usr/local/bin (sauf binaire du paquet), installe le
/// profil, écrit la configuration de greetd et ses lignes PAM, prépare le
/// dossier d'état du greeter.
fn install(profile: Option<&str>) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    let p = read_profile(profile)?;
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
    std::fs::write(
        PROFILE,
        serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?,
    )
    .map_err(io)?;
    let old = std::fs::read_to_string(CONFIG).unwrap_or_default();
    let bak = format!("{CONFIG}.bak");
    if !old.is_empty() && !old.starts_with(HEADER) && !Path::new(&bak).exists() {
        std::fs::write(&bak, &old).map_err(io)?;
    }
    let vt = parse_config(&old).1.unwrap_or(1);
    std::fs::write(CONFIG, config_toml(bin, vt, &p)).map_err(io)?;
    // PAM : seulement si le paquet greetd a posé son fichier (sinon greetd
    // n'est pas installé, et un fichier inventé ici serait faux).
    if let Ok(src) = std::fs::read_to_string(PAM) {
        let new = pam_with(&src, p.fingerprint && has("fprintd"), mymfa_module());
        if new != src {
            std::fs::write(PAM, new).map_err(io)?;
        }
    }
    // Dossier d'état, à l'utilisateur greeter seul.
    if let Some((uid, gid)) = user_ids("greeter") {
        std::fs::create_dir_all(STATE_DIR).map_err(io)?;
        std::os::unix::fs::chown(STATE_DIR, Some(uid), Some(gid)).map_err(io)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(STATE_DIR, std::fs::Permissions::from_mode(0o700)).map_err(io)?;
    }
    Ok(())
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

/// `hyprdesk greeter setup <action> [arg]` (en root).
pub fn cli(action: Option<&str>, arg: Option<&str>) -> Result<(), String> {
    match action {
        Some("packages" | "install" | "enable" | "restore") => {}
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
    }
    // SAFETY: geteuid n'a pas de précondition.
    if unsafe { libc::geteuid() } != 0 {
        return Err(crate::i18n::tr(
            "à lancer en root (pkexec ou sudo) : l'action touche /etc et systemd.",
        )
        .to_string());
    }
    match action {
        Some("packages") => packages(),
        Some("install") => install(arg),
        Some("enable") => enable(),
        _ => restore(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCH_PAM: &str = "#%PAM-1.0\n\nauth       required     pam_securetty.so\nauth       requisite    pam_nologin.so\nauth       include      system-local-login\naccount    include      system-local-login\nsession    include      system-local-login\n";

    fn fr() -> Profile {
        Profile {
            layout: "fr".into(),
            variant: "azerty".into(),
            ..Default::default()
        }
    }

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
        let src = config_toml(SYSTEM_BIN, 1, &Profile::default());
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
    fn clavier_passe_a_cage() {
        let (cmd, _) = parse_config(&config_toml(SYSTEM_BIN, 1, &fr()));
        assert_eq!(
            cmd.as_deref(),
            Some(
                "env XKB_DEFAULT_LAYOUT=fr XKB_DEFAULT_VARIANT=azerty \
                 cage -s -- /usr/local/bin/hyprdesk greeter"
            )
        );
        // Valeur douteuse : écartée, jamais recopiée dans la commande.
        let p = Profile {
            layout: "fr; rm -rf /".into(),
            ..Default::default()
        };
        assert!(!greeter_command(SYSTEM_BIN, &p).contains("rm"));
        assert!(xkb_ok("grp:alt_shift_toggle,caps:escape"));
    }

    #[test]
    fn lignes_pam_posees_et_retirees() {
        let both = pam_with(ARCH_PAM, true, true);
        let lines: Vec<&str> = both.lines().collect();
        let fp = lines
            .iter()
            .position(|l| l.contains("pam_fprintd"))
            .unwrap();
        let inc = lines
            .iter()
            .position(|l| l.starts_with("auth") && l.contains("system-local-login"))
            .unwrap();
        let nologin = lines
            .iter()
            .position(|l| l.contains("pam_nologin"))
            .unwrap();
        assert!(nologin < fp && fp < inc, "empreinte avant la pile : {both}");
        let auth_m = lines
            .iter()
            .position(|l| l.starts_with("auth") && l.contains("pam_mymfa"))
            .unwrap();
        assert!(auth_m > inc);
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("session") && l.contains("pam_mymfa"))
        );
        assert_eq!(pam_state(&both), (true, true));
        // Rejouable, puis retirable sans trace.
        assert_eq!(pam_with(&both, true, true), both);
        assert_eq!(pam_with(&both, false, false), ARCH_PAM);
        assert_eq!(pam_state(ARCH_PAM), (false, false));
    }

    #[test]
    fn profil_et_theme() {
        assert_eq!(Profile::default().theme(), crate::config::Theme::default());
        let p = Profile {
            theme: "Nuit".into(),
            bg: [1, 2, 3, 255],
            ..fr()
        };
        assert_eq!(p.theme().bg, crate::config::Rgba(1, 2, 3, 255));
        assert_eq!(p.theme().name, "Nuit");
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
