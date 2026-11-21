//! Mise en place de l'écran de connexion (MLG-42, MLG-43) : greetd avec
//! `hyprdesk greeter` à la place du gestionnaire de connexion en service
//! (GDM, SDDM, LightDM…).
//!
//! Lecture sans droits (`status`) pour Réglages → Écran de connexion ; les
//! gestes système passent par `pkexec <hyprdesk de root> greeter setup
//! <action>`, exécuté en root. Seul un hyprdesk installé en root est lancé
//! ainsi (`/usr/bin/hyprdesk` du paquet, sinon `/usr/local/bin/hyprdesk` posé
//! en root par `scripts/install.sh --from-source`) : jamais celui de
//! `~/.local/bin`, que l'utilisateur peut remplacer (MLG-152).
//!
//! - `packages` : installe greetd et cage (pacman, apt, dnf, zypper) ;
//! - `install <profil.json>` : fait lancer ce hyprdesk de root par greetd
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
//! - `restore` : revient au gestionnaire d'avant (noté par `enable`), activé
//!   avant que greetd soit coupé, et seulement s'il est installé.
//!
//! Les fichiers de `/etc` sont réécrits d'un coup (temporaire, fsync,
//! renommage) : une coupure ne laisse jamais un PAM ou un config.toml tronqué.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const CONFIG: &str = "/etc/greetd/config.toml";
/// Profil de l'écran (lisible par l'utilisateur `greeter`).
pub const PROFILE: &str = "/etc/greetd/hyprdesk.json";
pub const PAM: &str = "/etc/pam.d/greetd";
/// Fond de l'écran, copié (et réduit) depuis la bibliothèque de l'utilisateur.
pub const WALLPAPER: &str = "/etc/greetd/hyprdesk-fond.jpg";
/// Fond vidéo (MP4, WebM…), copié tel quel : joué en boucle par le greeter.
pub const WALLPAPER_VIDEO: &str = "/etc/greetd/hyprdesk-fond.mp4";
/// Fond pour un écran de connexion vertical (écran pivoté).
pub const WALLPAPER_VERTICAL: &str = "/etc/greetd/hyprdesk-fond-vertical.jpg";
/// Fond vidéo pour un écran de connexion vertical.
pub const WALLPAPER_VIDEO_VERTICAL: &str = "/etc/greetd/hyprdesk-fond-vertical.mp4";
/// Configuration du Hyprland réduit qui porte l'écran de connexion.
pub const HYPRLAND_CONFIG: &str = "/etc/greetd/hyprdesk-hyprland.lua";
/// Dossier d'état du greeter (à l'utilisateur `greeter`) : dernier
/// utilisateur et dernière session.
pub const STATE_DIR: &str = "/var/lib/hyprdesk-greeter";
/// hyprdesk posé en root par `scripts/install.sh --from-source` (sans paquet).
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
const MYMFA_PAM: [&str; 7] = [
    "/usr/lib/security/pam_mymfa.so",
    "/lib/security/pam_mymfa.so",
    "/usr/lib64/security/pam_mymfa.so",
    // Debian, Ubuntu (multiarch).
    "/usr/lib/x86_64-linux-gnu/security/pam_mymfa.so",
    "/lib/x86_64-linux-gnu/security/pam_mymfa.so",
    "/usr/lib/aarch64-linux-gnu/security/pam_mymfa.so",
    "/lib/aarch64-linux-gnu/security/pam_mymfa.so",
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
    /// Image ou vidéo choisie par l'utilisateur (son chemin, souvent dans
    /// son dossier, que l'utilisateur greeter ne lit pas).
    pub wallpaper_source: String,
    /// Sa copie lisible par le greeter (posée par `install`), vide : aucun
    /// fond. Image (jpg) ou vidéo (mp4), d'après la source.
    pub wallpaper: String,
    /// Même chose pour un écran de connexion vertical.
    pub wallpaper_vertical_source: String,
    pub wallpaper_vertical: String,
    /// Emplacement du formulaire : 0 (haut-gauche) … 8 (bas-droite), rangée
    /// par rangée ; absent : bas-droite (comme depuis le début).
    pub anchor: Option<u8>,
    /// Langue de l'écran (LANG de la session, « fr_FR.UTF-8 »).
    pub lang: String,
    /// Disposition des écrans de la session, reprise telle quelle.
    pub monitors: Vec<MonitorRule>,
    /// Écran qui porte la connexion (les autres restent noirs) ; vide : le
    /// premier.
    pub login_monitor: String,
    /// gnome-keyring sert encore (pas remplacé par myMFA) : déverrouillé à
    /// la connexion.
    pub gnome_keyring: bool,
}

/// Un écran de la disposition (mêmes valeurs que monitors.lua).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MonitorRule {
    pub name: String,
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub refresh: f32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub transform: u8,
}

impl MonitorRule {
    pub fn from_screen(s: &crate::screens::Screen) -> Self {
        MonitorRule {
            name: s.name.clone(),
            enabled: s.enabled,
            width: s.width,
            height: s.height,
            refresh: s.refresh,
            x: s.x,
            y: s.y,
            scale: s.scale,
            transform: s.transform,
        }
    }

    pub fn vertical(&self) -> bool {
        self.transform % 2 == 1
    }

    /// Ligne `hl.monitor` (nom déjà vérifié par `name_ok`).
    fn lua(&self) -> String {
        if !self.enabled {
            return format!(
                "hl.monitor({{ output = \"{}\", disabled = true }})",
                self.name
            );
        }
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        format!(
            "hl.monitor({{ output = \"{}\", mode = \"{}x{}@{:.2}\", position = \"{}x{}\", \
             scale = {}, transform = {} }})",
            self.name,
            self.width,
            self.height,
            self.refresh,
            self.x,
            self.y,
            (scale * 1000.0).round() / 1000.0,
            self.transform % 8
        )
    }
}

/// Nom d'écran sûr (connecteur : `DP-1`, `HDMI-A-2`, `eDP-1`).
pub fn name_ok(v: &str) -> bool {
    !v.is_empty()
        && v.len() < 64
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

/// Langue sûre (`fr_FR.UTF-8`, `C.UTF-8`).
pub fn lang_ok(v: &str) -> bool {
    v.len() < 64
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.@-".contains(c))
}

/// Écran qui porte la connexion : celui choisi s'il est allumé, sinon le
/// premier allumé.
pub fn login_monitor(p: &Profile) -> Option<&MonitorRule> {
    p.monitors
        .iter()
        .find(|m| m.enabled && m.name == p.login_monitor)
        .or_else(|| p.monitors.iter().find(|m| m.enabled))
}

/// Nom de l'écran qui porte la connexion, dans le profil installé. C'est
/// l'écran visé par la surface du greeter et par les fenêtres qui l'accompagnent (le mpv
/// d'un fond vidéo) ; vide : à la charge du compositeur.
pub fn login_screen() -> Option<String> {
    Profile::installed()
        .as_ref()
        .and_then(login_monitor)
        .map(|m| m.name.clone())
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

    /// Mêmes réglages, au chemin de la copie du fond près.
    pub fn same_settings(&self, other: &Profile) -> bool {
        let strip = |p: &Profile| Profile {
            wallpaper: String::new(),
            wallpaper_vertical: String::new(),
            ..p.clone()
        };
        strip(self) == strip(other)
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
    /// gnome-keyring : ligne posée, module installé.
    pub pam_keyring: bool,
    pub keyring_module: bool,
    /// Hyprland disponible pour porter l'écran (sinon cage).
    pub hyprland: bool,
    /// ffmpeg présent : les fonds vidéo de l'écran jouent (sinon le fond
    /// livré s'affiche).
    pub ffmpeg: bool,
    /// mpv présent : les fonds vidéo sont portés par mpv en GPU (rendu
    /// vsyncé), à la place du décodage trame par trame.
    pub mpv: bool,
    /// gnome-keyring sert encore dans la session (pas remplacé par myMFA).
    pub gnome_keyring: bool,
    pub fprintd: bool,
    pub loaded: bool,
}

impl Status {
    /// Tout est en place pour basculer sur greetd sans risque.
    pub fn ready(&self) -> bool {
        self.greetd
            && (self.hyprland || self.cage)
            && self.greeter_user
            && self.binary.is_some()
            && self.ours
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
        || command.contains(crate::companion::GREETER)
        || command.contains(HYPRLAND_CONFIG)
}

/// Commande de l'écran de connexion pour le binaire root `bin` : le binaire
/// dédié posé à côté (`hyprdesk-greeter`), sinon `hyprdesk greeter`. Le
/// binaire dédié passe le même contrôle que `bin` (à root, ni lui ni son
/// dossier modifiables par d'autres) : greetd le lance avant toute connexion.
pub fn greeter_run(bin: &str) -> String {
    match crate::companion::beside(Path::new(bin), crate::companion::GREETER)
        .filter(|p| root_owned(p))
    {
        Some(p) => p.display().to_string(),
        None => format!("{bin} greeter"),
    }
}

/// Compositeur qui porte l'écran de connexion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Compositor {
    /// Hyprland réduit (configuration HYPRLAND_CONFIG), lancé par ce
    /// programme (`start-hyprland`, ou `Hyprland`).
    Hyprland(&'static str),
    /// cage, en secours sans Hyprland : pas de layer-shell (fenêtre
    /// ordinaire) et tous les écrans à la fois.
    Cage,
}

/// Commande de greetd. Environnement : `HOME` sur le dossier d'état du
/// greeter (le sien, « / », n'est pas inscriptible), la langue de la
/// session ; sous cage, la disposition du clavier (XKB_DEFAULT_*), sous
/// Hyprland elle est dans sa configuration.
/// `run` : commande de l'écran de connexion (voir [`greeter_run`]).
pub fn greeter_command(run: &str, p: &Profile, comp: Compositor) -> String {
    let mut env = vec![format!("HOME={STATE_DIR}")];
    if !p.lang.is_empty() && lang_ok(&p.lang) {
        env.push(format!("LANG={}", p.lang));
    }
    let run = match comp {
        Compositor::Hyprland(launcher) => {
            let sep = if launcher == "start-hyprland" {
                " --"
            } else {
                ""
            };
            format!("{launcher}{sep} --config {HYPRLAND_CONFIG}")
        }
        Compositor::Cage => {
            for (k, v) in [
                ("XKB_DEFAULT_LAYOUT", &p.layout),
                ("XKB_DEFAULT_VARIANT", &p.variant),
                ("XKB_DEFAULT_OPTIONS", &p.options),
            ] {
                if !v.is_empty() && xkb_ok(v) {
                    env.push(format!("{k}={v}"));
                }
            }
            // cage n'offre pas layer-shell : fenêtre ordinaire, qu'il met en plein écran.
            format!("cage -s -- {run} --window")
        }
    };
    format!("env {} {run}", env.join(" "))
}

/// Configuration du Hyprland de connexion : disposition des écrans de la
/// session, clavier, rien d'animé, fond noir (les écrans sans connexion
/// restent noirs) ; au démarrage, focus sur l'écran de connexion puis
/// l'écran lui-même ; quand il se ferme (session ouverte), Hyprland quitte
/// et greetd lance la session.
pub fn hyprland_lua(run: &str, p: &Profile) -> String {
    let q = |v: &str, ok: bool| if ok { v.to_string() } else { String::new() };
    let mut out = format!(
        "-- {}\n-- Hyprland de l'écran de connexion (greetd) ; réécrit à chaque enregistrement.\n\n",
        HEADER.trim_start_matches("# ")
    );
    for m in p.monitors.iter().filter(|m| name_ok(&m.name)) {
        out.push_str(&m.lua());
        out.push('\n');
    }
    out.push_str("hl.monitor({ output = \"\", mode = \"preferred\", position = \"auto\", scale = \"auto\" })\n\n");
    out.push_str(&format!(
        "hl.config({{\n    input = {{ kb_layout = \"{}\", kb_variant = \"{}\", kb_options = \"{}\" }},\n\
         \x20   general = {{ border_size = 0, gaps_in = 0, gaps_out = 0 }},\n\
         \x20   animations = {{ enabled = false }},\n\
         \x20   misc = {{ disable_hyprland_logo = true, disable_splash_rendering = true, background_color = \"rgb(000000)\" }},\n}})\n\n",
        q(&p.layout, xkb_ok(&p.layout)),
        q(&p.variant, xkb_ok(&p.variant)),
        q(&p.options, xkb_ok(&p.options)),
    ));
    out.push_str("hl.on(\"hyprland.start\", function()\n");
    if let Some(m) = login_monitor(p).filter(|m| name_ok(&m.name)) {
        out.push_str(&format!(
            "    hl.dispatch(hl.dsp.focus({{ monitor = \"{}\" }}))\n",
            m.name
        ));
    }
    out.push_str(&format!(
        "    hl.exec_cmd(\"sh -c '{run}; hyprctl dispatch \\\"hl.dsp.exit()\\\"'\")\n"
    ));
    out.push_str("end)\n");
    out
}

/// /etc/greetd/config.toml de hyprdesk (`vt` repris de l'ancien fichier).
pub fn config_toml(run: &str, vt: i64, p: &Profile, comp: Compositor) -> String {
    format!(
        "{HEADER}\n# Revenir en arrière : sudo hyprdesk greeter setup restore\n\n\
         [terminal]\nvt = {vt}\n\n\
         [default_session]\ncommand = \"{}\"\nuser = \"greeter\"\n",
        greeter_command(run, p, comp)
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
pub fn pam_with(src: &str, fingerprint: bool, mymfa: bool, keyring: bool) -> String {
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
    // Trousseaux ouverts par le mot de passe de connexion : gnome-keyring
    // (comme sous GDM) tant qu'il sert, myMFA s'il est installé ; les deux
    // pendant la transition.
    let modules = [
        keyring.then_some((
            "pam_gnome_keyring.so",
            " auto_start",
            "trousseau gnome-keyring",
        )),
        mymfa.then_some(("pam_mymfa.so", "", "trousseau myMFA")),
    ];
    for (module, session_arg, what) in modules.into_iter().flatten() {
        for kind in ["auth", "session"] {
            if let Some(i) = lines.iter().rposition(|l| pam_kind(l) == Some(kind)) {
                let arg = if kind == "session" { session_arg } else { "" };
                lines.insert(i + 1, format!("{kind:<10} optional     {module}{arg}"));
                lines.insert(i + 1, mark(what));
            }
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Lignes de hyprdesk présentes : (empreinte, myMFA, gnome-keyring).
pub fn pam_state(src: &str) -> (bool, bool, bool) {
    let mut it = src.lines().peekable();
    let (mut f, mut m, mut k) = (false, false, false);
    while let Some(l) = it.next() {
        if l.trim_start().starts_with(PAM_MARK)
            && let Some(next) = it.peek()
        {
            f |= next.contains("pam_fprintd");
            m |= next.contains("pam_mymfa");
            k |= next.contains("pam_gnome_keyring");
        }
    }
    (f, m, k)
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

/// Module PAM de gnome-keyring installé.
fn keyring_module() -> bool {
    MYMFA_PAM
        .iter()
        .map(|p| p.replace("pam_mymfa.so", "pam_gnome_keyring.so"))
        .any(|p| Path::new(&p).is_file())
}

/// Compositeur disponible pour l'écran de connexion (Hyprland d'abord).
pub fn compositor() -> Option<Compositor> {
    if has_program("start-hyprland") {
        Some(Compositor::Hyprland("start-hyprland"))
    } else if has_program("Hyprland") {
        Some(Compositor::Hyprland("Hyprland"))
    } else if has_program("cage") {
        Some(Compositor::Cage)
    } else {
        None
    }
}

fn has_program(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .chain(["/usr/bin".into(), "/usr/local/bin".into()])
        .any(|d| d.join(name).is_file())
}

pub fn status() -> Status {
    let dm = unit_of_link(Path::new(DM_LINK));
    let exe = current_exe();
    // Seul un hyprdesk installé en root compte (voir `root_bin`).
    let binary = root_bin().ok().map(str::to_string);
    let binary_stale = binary.as_deref() == Some(SYSTEM_BIN)
        && exe
            .as_deref()
            .is_some_and(|e| e != Path::new(SYSTEM_BIN) && stale(Path::new(SYSTEM_BIN), e));
    let (command, _) = parse_config(&std::fs::read_to_string(CONFIG).unwrap_or_default());
    let (pam_fingerprint, pam_mymfa, pam_keyring) =
        pam_state(&std::fs::read_to_string(PAM).unwrap_or_default());
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
        pam_keyring,
        mymfa_module: mymfa_module(),
        keyring_module: keyring_module(),
        hyprland: matches!(compositor(), Some(Compositor::Hyprland(_))),
        ffmpeg: crate::doctor::has("ffmpeg"),
        mpv: crate::doctor::has("mpv"),
        gnome_keyring: crate::mymfa::gnome_keyring_in_use(),
        fprintd: crate::doctor::has("fprintd"),
        loaded: true,
    }
}

/// Binaire sûr pour root : appartient à root, n'est modifiable ni par le
/// groupe ni par les autres, dans un dossier qui ne l'est pas non plus (on
/// ne lance jamais en root un fichier que l'utilisateur peut remplacer).
fn root_owned(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let safe = |m: &std::fs::Metadata| m.uid() == 0 && m.mode() & 0o022 == 0;
    let Ok(m) = std::fs::symlink_metadata(path) else {
        return false;
    };
    m.is_file()
        && safe(&m)
        && path
            .parent()
            .and_then(|d| std::fs::metadata(d).ok())
            .is_some_and(|d| safe(&d))
}

/// hyprdesk à lancer en root : celui du paquet, sinon celui posé en root par
/// `scripts/install.sh --from-source` ; jamais celui de l'utilisateur.
fn pick_root_bin(trusted: impl Fn(&Path) -> bool) -> Result<&'static str, String> {
    [PACKAGED_BIN, SYSTEM_BIN]
        .into_iter()
        .find(|p| trusted(Path::new(p)))
        .ok_or_else(|| {
            crate::tr!(
                "Aucun hyprdesk installé en root ({a} ou {b}) : installez le paquet hyprdesk ou lancez scripts/install.sh --from-source. Le hyprdesk de l'utilisateur n'est jamais lancé en root.",
                a = PACKAGED_BIN,
                b = SYSTEM_BIN
            )
        })
}

/// hyprdesk installés en root, dans l'ordre de préférence (paquet, puis
/// compilation) ; jamais celui de l'utilisateur.
pub fn root_bins() -> Vec<&'static str> {
    [PACKAGED_BIN, SYSTEM_BIN]
        .into_iter()
        .filter(|p| root_owned(Path::new(p)))
        .collect()
}

/// hyprdesk à lancer en root (pkexec, écran de connexion).
pub fn root_bin() -> Result<&'static str, String> {
    pick_root_bin(root_owned)
}

/// Arguments de `pkexec` pour une action (`hyprdesk greeter setup <action>`).
pub fn pkexec_argv(action: &str, extra: &[String]) -> Result<Vec<String>, String> {
    let mut v = vec![
        "pkexec".into(),
        root_bin()?.into(),
        "greeter".into(),
        "setup".into(),
        action.into(),
    ];
    v.extend(extra.iter().cloned());
    Ok(v)
}

/// Profil à installer, écrit pour root dans le dossier d'exécution de
/// l'utilisateur (vérifié, jamais `/tmp`) ; renvoie son chemin.
pub fn stage_profile(p: &Profile) -> Result<String, String> {
    let dir = crate::policy::runtime_dir()?.join("hyprdesk");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("greeter-profile.json");
    let body = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
    crate::policy::write_private(&path, body.as_bytes()).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// Lance l'action par pkexec (bloquant) ; `Err` : sortie d'erreur.
pub fn run_privileged(action: &str, extra: &[String]) -> Result<(), String> {
    let argv = pkexec_argv(action, extra)?;
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
    for w in [&mut p.wallpaper_source, &mut p.wallpaper_vertical_source] {
        if !Path::new(w.as_str()).is_absolute() {
            w.clear();
        }
    }
    p.wallpaper.clear();
    p.wallpaper_vertical.clear();
    if p.anchor.is_some_and(|a| a > 8) {
        p.anchor = None;
    }
    if !lang_ok(&p.lang) {
        p.lang.clear();
    }
    p.monitors.retain(|m| name_ok(&m.name));
    if !name_ok(&p.login_monitor) {
        p.login_monitor.clear();
    }
    Ok(p)
}

/// Copie du fond pour le greeter : image décodée (seuls les formats connus
/// passent), réduite à 3840 × 2160 au plus, enregistrée en JPEG.
fn install_wallpaper(src: &str, dest: &str) -> Result<(), String> {
    let img = ::image::open(src).map_err(|e| format!("{src} : {e}"))?;
    let img = if img.width() > 3840 || img.height() > 2160 {
        img.resize(3840, 2160, ::image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let tmp = format!("{dest}.nouveau");
    let file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut out = std::io::BufWriter::new(file);
    ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
        .encode_image(&img.to_rgb8())
        .map_err(|e| e.to_string())?;
    drop(out);
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// Copie de la copie posée pour le greeter, d'après la source et l'écran
/// (horizontal ou pivoté) : image en JPEG, vidéo copiée telle quelle.
fn dest_of(vertical: bool, video: bool) -> &'static str {
    match (vertical, video) {
        (false, false) => WALLPAPER,
        (true, false) => WALLPAPER_VERTICAL,
        (false, true) => WALLPAPER_VIDEO,
        (true, true) => WALLPAPER_VIDEO_VERTICAL,
    }
}

/// Vidéo lisible ? ffprobe s'il est là (sinon : le greeter fera le tri).
fn probe_video(src: &str) -> Result<(), String> {
    if !crate::launch::executable("ffprobe") {
        return Ok(());
    }
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
            src,
        ])
        .output()
        .map_err(|e| format!("{src} : {e}"))?;
    let ok = out.status.success()
        && String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|l| l.trim() == "video");
    if ok {
        Ok(())
    } else {
        Err(crate::tr!("{src} : vidéo illisible (ffprobe)", src = src))
    }
}

/// Copie de la vidéo pour le greeter : telle quelle (le décodage la réduira
/// à l'écran), lisible par tous, posée d'un coup (un greeter lancé ne voit
/// jamais une copie à moitié écrite).
fn copy_video(src: &str, dest: &str) -> Result<(), String> {
    let tmp = format!("{dest}.nouveau");
    std::fs::copy(src, &tmp).map_err(|e| format!("{src} : {e}"))?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// Pose du fond : image ou vidéo, d'après la source.
fn install_backdrop(src: &str, dest: &str) -> Result<(), String> {
    if crate::video::is_video(Path::new(src)) {
        probe_video(src)?;
        copy_video(src, dest)
    } else {
        install_wallpaper(src, dest)
    }
}

/// Fait lancer par greetd le hyprdesk installé en root (paquet, sinon
/// /usr/local/bin posé par les scripts : jamais une copie du binaire de
/// l'utilisateur), installe le profil, écrit la configuration de greetd et
/// ses lignes PAM, prépare le dossier d'état du greeter.
fn install(profile: Option<&str>) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    let mut p = read_profile(profile)?;
    let bin = root_bin()?;
    std::fs::create_dir_all("/etc/greetd").map_err(io)?;
    for (src, vertical, field) in [
        (p.wallpaper_source.clone(), false, &mut p.wallpaper),
        (
            p.wallpaper_vertical_source.clone(),
            true,
            &mut p.wallpaper_vertical,
        ),
    ] {
        if src.is_empty() {
            // Plus de fond : les copies image et vidéo s'en vont.
            for d in [dest_of(vertical, false), dest_of(vertical, true)] {
                let _ = std::fs::remove_file(d);
            }
            continue;
        }
        let video = crate::video::is_video(Path::new(&src));
        let dest = dest_of(vertical, video);
        if !Path::new(&src).is_file() && Path::new(dest).is_file() {
            // Fond d'origine retiré depuis (mise à jour, autre machine) :
            // la copie déjà installée reste.
            *field = dest.into();
        } else {
            install_backdrop(&src, dest)?;
            *field = dest.into();
            // L'ancienne copie de l'autre type (image remplacée par vidéo,
            // ou l'inverse) ne sert plus.
            let _ = std::fs::remove_file(dest_of(vertical, !video));
        }
    }
    let comp = compositor().ok_or_else(|| {
        crate::i18n::tr("Ni Hyprland ni cage : rien pour afficher l'écran de connexion.")
            .to_string()
    })?;
    let run = greeter_run(bin);
    if matches!(comp, Compositor::Hyprland(_)) {
        write_atomic(
            Path::new(HYPRLAND_CONFIG),
            hyprland_lua(&run, &p).as_bytes(),
        )
        .map_err(io)?;
    }
    write_atomic(
        Path::new(PROFILE),
        serde_json::to_string_pretty(&p)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(io)?;
    let old = std::fs::read_to_string(CONFIG).unwrap_or_default();
    let bak = format!("{CONFIG}.bak");
    if !old.is_empty() && !old.starts_with(HEADER) && !Path::new(&bak).exists() {
        std::fs::write(&bak, &old).map_err(io)?;
    }
    let vt = parse_config(&old).1.unwrap_or(1);
    write_atomic(
        Path::new(CONFIG),
        config_toml(&run, vt, &p, comp).as_bytes(),
    )
    .map_err(io)?;
    // PAM : seulement si le paquet greetd a posé son fichier (sinon greetd
    // n'est pas installé, et un fichier inventé ici serait faux).
    if let Ok(src) = std::fs::read_to_string(PAM) {
        let new = pam_with(
            &src,
            p.fingerprint && has("fprintd"),
            mymfa_module(),
            keyring_module() && p.gnome_keyring,
        );
        if new != src {
            write_atomic(Path::new(PAM), new.as_bytes()).map_err(io)?;
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

/// Écrit `data` dans `path` sans fenêtre où le fichier serait vide ou
/// tronqué : fichier temporaire du même dossier (mode et propriétaire de
/// l'ancien, 0644 root sinon), fsync, puis renommage.
fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    let old = std::fs::metadata(path).ok();
    let mode = old.as_ref().map_or(0o644, |m| m.mode() & 0o7777);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.hyprdesk-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)?;
    let res = (|| {
        f.write_all(data)?;
        if let Some(m) = &old {
            // Propriétaire d'origine (root en pratique) ; sans droit, on garde le nôtre.
            let _ = std::os::unix::fs::fchown(&f, Some(m.uid()), Some(m.gid()));
        }
        f.set_permissions(std::fs::Permissions::from_mode(mode))?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)?;
        // Le renommage lui-même, sur disque.
        if let Some(dir) = path.parent() {
            let _ = std::fs::File::open(dir).and_then(|d| d.sync_all());
        }
        Ok(())
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// Unité systemd connue (`systemctl cat`) : on ne bascule pas vers une unité
/// absente.
fn unit_exists(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["cat", "--", unit])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Gestionnaire à rétablir : celui retenu à la bascule, sinon le premier
/// installé des courants ; aucun : `None` (rien n'est changé).
fn previous_dm(saved: Option<String>, exists: impl Fn(&str) -> bool) -> Option<String> {
    saved
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "greetd.service")
        .into_iter()
        .chain(
            [
                "gdm.service",
                "sddm.service",
                "lightdm.service",
                "ly.service",
            ]
            .into_iter()
            .map(String::from),
        )
        .find(|u| exists(u))
}

/// Revient au gestionnaire d'avant : il est activé d'abord, greetd n'est
/// coupé qu'ensuite ; sans gestionnaire installé, rien ne change (le poste
/// ne doit jamais rester sans écran de connexion).
fn restore() -> Result<(), String> {
    let prev = previous_dm(std::fs::read_to_string(PREVIOUS).ok(), unit_exists).ok_or_else(|| {
        crate::i18n::tr(
            "Aucun autre gestionnaire de connexion installé (GDM, SDDM, LightDM, ly) : greetd reste en place.",
        )
        .to_string()
    })?;
    run(&["systemctl", "enable", "--force", &prev])?;
    let _ = run(&["systemctl", "disable", "greetd.service"]);
    // Le lien display-manager.service vise maintenant `prev` (enable --force).
    Ok(())
}

/// Profil de l'écran de connexion d'après la session, comme Réglages →
/// Écran de connexion : clavier (inputs.lua), thème, écrans, langue.
pub fn session_profile(config_path: &Path) -> Profile {
    let cfg = crate::theme::effective(config_path).unwrap_or_default();
    let th = &cfg.general.theme;
    let c = |v: crate::config::Rgba| [v.0, v.1, v.2, v.3];
    let i = crate::inputs::read();
    let monitors: Vec<MonitorRule> = crate::screens::detect()
        .map(|l| l.iter().map(MonitorRule::from_screen).collect())
        .unwrap_or_default();
    Profile {
        layout: i.kb_layout,
        variant: i.kb_variant,
        options: i.kb_options,
        theme: th.name.clone(),
        fg: c(th.fg),
        bg: c(th.bg),
        accent: c(th.accent),
        lang: std::env::var("LANG").unwrap_or_default(),
        monitors,
        gnome_keyring: status().gnome_keyring,
        ..Default::default()
    }
}

/// `hyprdesk greeter setup <action> [arg]` (en root, sauf `profile`).
pub fn cli(action: Option<&str>, arg: Option<&str>) -> Result<(), String> {
    match action {
        // En utilisateur : le profil de la session, écrit pour `install`.
        Some("profile") => {
            let p = session_profile(&crate::config::default_path());
            let json = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
            return match arg {
                Some(path) => std::fs::write(path, json).map_err(|e| format!("{path} : {e}")),
                None => {
                    println!("{json}");
                    Ok(())
                }
            };
        }
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
                "action inconnue : {a} (profile, packages, install, enable, restore, status)",
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

    /// En root, seul un hyprdesk installé en root est lancé, paquet d'abord ;
    /// le binaire de l'utilisateur jamais.
    #[test]
    fn binaire_lance_en_root() {
        assert_eq!(pick_root_bin(|_| true), Ok(PACKAGED_BIN));
        assert_eq!(
            pick_root_bin(|p| p == Path::new(SYSTEM_BIN)),
            Ok(SYSTEM_BIN)
        );
        let e = pick_root_bin(|_| false).unwrap_err();
        assert!(e.contains("install.sh --from-source"), "{e}");
        // Un fichier de l'utilisateur (le test ne tourne pas en root) n'est
        // pas sûr pour root.
        let f = std::env::temp_dir().join(format!("hyprdesk-rootbin-{}", std::process::id()));
        std::fs::write(&f, b"#!/bin/sh\n").unwrap();
        if unsafe { libc::getuid() } != 0 {
            assert!(!root_owned(&f));
        }
        let _ = std::fs::remove_file(&f);
        assert!(!root_owned(Path::new("/nexiste/pas/hyprdesk")));
    }

    /// Restauration : l'unité retenue si elle existe, sinon un gestionnaire
    /// installé ; aucun : rien.
    #[test]
    fn gestionnaire_a_retablir() {
        let only = |names: &'static [&'static str]| move |u: &str| names.contains(&u);
        assert_eq!(
            previous_dm(Some("sddm.service\n".into()), only(&["sddm.service"])).as_deref(),
            Some("sddm.service")
        );
        assert_eq!(
            previous_dm(Some("sddm.service".into()), only(&["lightdm.service"])).as_deref(),
            Some("lightdm.service"),
            "unité retenue absente : une autre installée"
        );
        assert_eq!(
            previous_dm(Some("greetd.service".into()), only(&["greetd.service"])),
            None
        );
        assert_eq!(previous_dm(None, only(&[])), None, "rien : greetd reste");
    }

    /// Écriture atomique : contenu remplacé, mode d'origine gardé, pas de
    /// fichier temporaire laissé.
    #[test]
    fn ecriture_atomique() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("hyprdesk-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("greetd");
        std::fs::write(&p, b"ancien").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o640)).unwrap();
        write_atomic(&p, b"nouveau").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"nouveau");
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
        write_atomic(&dir.join("neuf.toml"), b"x").unwrap();
        let mode = std::fs::metadata(dir.join("neuf.toml"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o644);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

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
        let p = Profile {
            lang: "fr_FR.UTF-8".into(),
            ..Default::default()
        };
        let src = config_toml(
            &format!("{SYSTEM_BIN} greeter"),
            1,
            &p,
            Compositor::Hyprland("start-hyprland"),
        );
        assert!(src.starts_with(HEADER));
        let (cmd, vt) = parse_config(&src);
        assert_eq!(
            cmd.as_deref(),
            Some(
                "env HOME=/var/lib/hyprdesk-greeter LANG=fr_FR.UTF-8 \
                 start-hyprland -- --config /etc/greetd/hyprdesk-hyprland.lua"
            )
        );
        assert_eq!(vt, Some(1));
        assert!(is_ours(&cmd.unwrap()));
        assert!(!is_ours("tuigreet --cmd Hyprland"));
    }

    #[test]
    fn clavier_passe_a_cage() {
        let (cmd, _) = parse_config(&config_toml(
            &format!("{SYSTEM_BIN} greeter"),
            1,
            &fr(),
            Compositor::Cage,
        ));
        assert_eq!(
            cmd.as_deref(),
            Some(
                "env HOME=/var/lib/hyprdesk-greeter XKB_DEFAULT_LAYOUT=fr XKB_DEFAULT_VARIANT=azerty \
                 cage -s -- /usr/local/bin/hyprdesk greeter --window"
            )
        );
        // Valeur douteuse : écartée, jamais recopiée dans la commande.
        let p = Profile {
            layout: "fr; rm -rf /".into(),
            ..Default::default()
        };
        assert!(
            !greeter_command(&format!("{SYSTEM_BIN} greeter"), &p, Compositor::Cage).contains("rm")
        );
        let p = Profile {
            lang: "fr; rm".into(),
            ..Default::default()
        };
        assert!(
            !greeter_command(
                &format!("{SYSTEM_BIN} greeter"),
                &p,
                Compositor::Hyprland("Hyprland")
            )
            .contains("rm")
        );
        assert!(xkb_ok("grp:alt_shift_toggle,caps:escape"));
    }

    #[test]
    fn lignes_pam_posees_et_retirees() {
        let both = pam_with(ARCH_PAM, true, true, true);
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
        // Pendant la transition, les deux trousseaux s'ouvrent.
        assert_eq!(pam_state(&both), (true, true, true));
        // Rejouable, puis retirable sans trace.
        assert_eq!(pam_with(&both, true, true, true), both);
        assert_eq!(pam_with(&both, false, false, false), ARCH_PAM);
        assert_eq!(pam_state(ARCH_PAM), (false, false, false));
    }

    #[test]
    fn gnome_keyring_sans_mymfa() {
        let k = pam_with(ARCH_PAM, false, false, true);
        assert_eq!(pam_state(&k), (false, false, true));
        let lines: Vec<&str> = k.lines().collect();
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("auth") && l.ends_with("pam_gnome_keyring.so"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("session") && l.ends_with("pam_gnome_keyring.so auto_start"))
        );
        // La ligne de session vient après la pile (pam_systemd y crée /run/user).
        let inc = lines
            .iter()
            .position(|l| l.starts_with("session") && l.contains("include"))
            .unwrap();
        let kr = lines
            .iter()
            .position(|l| l.starts_with("session") && l.contains("gnome_keyring"))
            .unwrap();
        assert!(kr > inc);
        assert_eq!(pam_with(&k, false, false, false), ARCH_PAM);
    }

    #[test]
    fn hyprland_de_connexion() {
        let ecran = |name: &str, x: i32, y: i32, transform: u8| MonitorRule {
            name: name.into(),
            enabled: true,
            width: 2560,
            height: 1440,
            refresh: 144.0,
            x,
            y,
            scale: 1.0,
            transform,
        };
        let mut p = Profile {
            monitors: vec![ecran("DP-1", 0, 0, 0), ecran("HDMI-A-2", 2560, -560, 1)],
            login_monitor: "HDMI-A-2".into(),
            ..fr()
        };
        let lua = hyprland_lua(&format!("{SYSTEM_BIN} greeter"), &p);
        assert!(lua.contains(
            "hl.monitor({ output = \"HDMI-A-2\", mode = \"2560x1440@144.00\", position = \"2560x-560\", scale = 1, transform = 1 })"
        ));
        assert!(lua.contains("kb_layout = \"fr\", kb_variant = \"azerty\""));
        assert!(lua.contains("hl.dsp.focus({ monitor = \"HDMI-A-2\" })"));
        assert!(lua.contains(
            "hl.exec_cmd(\"sh -c '/usr/local/bin/hyprdesk greeter; hyprctl dispatch \\\"hl.dsp.exit()\\\"'\")"
        ));
        assert!(login_monitor(&p).unwrap().vertical());
        // Écran choisi absent ou éteint : le premier allumé.
        p.login_monitor = "eDP-1".into();
        assert_eq!(login_monitor(&p).unwrap().name, "DP-1");
        // Nom douteux : écarté de la configuration.
        p.monitors.push(ecran("X\"); os.exit(", 0, 0, 0));
        assert!(!hyprland_lua(&format!("{SYSTEM_BIN} greeter"), &p).contains("os.exit"));
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
    fn fonds_videos_poses() {
        assert_eq!(dest_of(false, false), WALLPAPER);
        assert_eq!(dest_of(true, false), WALLPAPER_VERTICAL);
        assert_eq!(dest_of(false, true), WALLPAPER_VIDEO);
        assert_eq!(dest_of(true, true), WALLPAPER_VIDEO_VERTICAL);
        // Copie de la vidéo telle quelle, lisible par tous.
        let dir = std::env::temp_dir().join(format!("hyprdesk-greeter-v{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("fond.mp4");
        std::fs::write(&src, b"donnees").unwrap();
        let dest = dir.join("copie.mp4");
        copy_video(src.to_str().unwrap(), dest.to_str().unwrap()).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"donnees");
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&dest).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o644);
        // Source devenue vidéo : les sources image et vidéo restent absolues.
        let mut p = fr();
        p.wallpaper_source = "/murs/fond.mp4".into();
        let lu = serde_json::from_str::<Profile>(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(lu.wallpaper_source, "/murs/fond.mp4");
        std::fs::remove_dir_all(&dir).unwrap();
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
