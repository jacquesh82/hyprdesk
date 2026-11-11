//! Agent d'authentification polkit : quand une application demande un
//! droit d'administration (pkexec, gestionnaire de paquets, montage de
//! disque…), polkit appelle l'agent de la session, qui demande le mot de
//! passe et le confie à `polkit-agent-helper-1`.
//!
//! Protocole relevé dans polkit 128 (`polkitagentsession.c`,
//! `polkitagenthelper-pam.c`) :
//! - socket `/run/polkit/agent-helper.socket` (activation systemd) :
//!   l'agent écrit `utilisateur\n` puis `cookie\n` ; à défaut, l'outil
//!   setuid lancé avec l'utilisateur en argument, le cookie sur stdin ;
//! - l'outil répond ligne par ligne (échappées façon `g_strescape`) :
//!   `PAM_PROMPT_ECHO_OFF …`, `PAM_PROMPT_ECHO_ON …` (une réponse attendue),
//!   `PAM_ERROR_MSG …`, `PAM_TEXT_INFO …`, puis `SUCCESS` ou `FAILURE`.
//!
//! L'interface (fenêtre du démon, ou terminal pour `hyprdesk polkit-agent`)
//! se branche par le trait `Prompter`. Un seul agent par session : si un
//! autre est déjà enregistré (polkit-kde, hyprpolkitagent…), polkit refuse
//! l'enregistrement et c'est lui qui reste en place.
//!
//! L'agent s'enregistre pour la SESSION (sujet `unix-session`, identifiant
//! de logind) : un agent enregistré pour son seul processus (`unix-process`)
//! est accepté même quand un autre tient la session, mais ne reçoit jamais
//! ses demandes. L'état (actif, occupé, erreur) est exposé par `state()`.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use zbus::zvariant::{OwnedValue, Value};

pub const SOCKET: &str = "/run/polkit/agent-helper.socket";
pub const HELPERS: &[&str] = &[
    "/usr/lib/polkit-1/polkit-agent-helper-1",
    "/usr/libexec/polkit-agent-helper-1",
    "/usr/lib/polkit-agent-helper-1",
];
pub const OBJECT_PATH: &str = "/org/hyprdesk/PolicyKit1/AuthenticationAgent";
/// Tentatives de mot de passe avant d'abandonner.
pub const ATTEMPTS: usize = 3;

/// Ligne envoyée par l'outil d'authentification.
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// Une réponse est attendue (`echo` : saisie visible).
    Prompt {
        echo: bool,
        text: String,
    },
    Error(String),
    Info(String),
    Success,
    Failure,
}

/// Inverse de `g_strescape` (comme `g_strcompress`) : `\n`, `\t`, `\\`,
/// `\"`, octal `\NNN`…
pub fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' || i + 1 == b.len() {
            out.push(b[i]);
            i += 1;
            continue;
        }
        i += 1;
        match b[i] {
            b'0'..=b'7' => {
                let mut v: u32 = 0;
                let mut n = 0;
                while n < 3 && i < b.len() && (b'0'..=b'7').contains(&b[i]) {
                    v = v * 8 + (b[i] - b'0') as u32;
                    i += 1;
                    n += 1;
                }
                out.push(v as u8);
                continue;
            }
            b'b' => out.push(8),
            b'f' => out.push(12),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'v' => out.push(11),
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Ligne lue (terminateur retiré) → événement ; `None` si inconnue.
pub fn parse_line(raw: &str) -> Option<Line> {
    let l = unescape(raw.trim_end_matches('\n'));
    let rest = |p: &str| l.strip_prefix(p).map(str::to_string);
    if let Some(t) = rest("PAM_PROMPT_ECHO_OFF ") {
        Some(Line::Prompt {
            echo: false,
            text: t,
        })
    } else if let Some(t) = rest("PAM_PROMPT_ECHO_ON ") {
        Some(Line::Prompt {
            echo: true,
            text: t,
        })
    } else if let Some(t) = rest("PAM_ERROR_MSG ") {
        Some(Line::Error(t))
    } else if let Some(t) = rest("PAM_TEXT_INFO ") {
        Some(Line::Info(t))
    } else if l.starts_with("SUCCESS") {
        Some(Line::Success)
    } else if l.starts_with("FAILURE") {
        Some(Line::Failure)
    } else {
        None
    }
}

/// Demande montrée à l'utilisateur.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// `org.freedesktop.udisks2.filesystem-mount`…
    pub action: String,
    /// Message de l'action, déjà traduit par polkit.
    pub message: String,
    /// Compte dont on demande le mot de passe.
    pub user: String,
    pub cookie: String,
    /// Tentative en cours (1…ATTEMPTS).
    pub attempt: usize,
}

pub type Reply<'a> = Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>>;

/// Ce que l'interface sait faire pour l'agent.
pub trait Prompter: Send + Sync + 'static {
    /// Demande une saisie (`echo` : visible) ; `None` : l'utilisateur annule.
    fn prompt<'a>(&'a self, req: &'a Request, text: &'a str, echo: bool) -> Reply<'a>;
    /// Message de PAM (erreur ou information).
    fn show(&self, req: &Request, text: &str, error: bool);
    /// Fin de la demande (réussie, ou abandonnée).
    fn done(&self, req: &Request, ok: bool);
}

/// Résultat d'une conversation avec l'outil.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Success,
    Failure,
    Cancelled,
}

/// Conversation avec l'outil sur un flux (socket, ou stdin/stdout de l'outil
/// setuid). `socket` : l'utilisateur est envoyé avant le cookie.
pub async fn converse<R, W>(
    reader: R,
    mut writer: W,
    req: &Request,
    socket: bool,
    prompter: &dyn Prompter,
) -> std::io::Result<Outcome>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if socket {
        writer
            .write_all(format!("{}\n", req.user).as_bytes())
            .await?;
    }
    writer
        .write_all(format!("{}\n", req.cookie).as_bytes())
        .await?;
    writer.flush().await?;
    let mut lines = BufReader::new(reader).lines();
    while let Some(l) = lines.next_line().await? {
        match parse_line(&l) {
            Some(Line::Prompt { echo, text }) => {
                let Some(mut answer) = prompter.prompt(req, &text, echo).await else {
                    return Ok(Outcome::Cancelled);
                };
                let mut line = std::mem::take(&mut answer);
                line.push('\n');
                let r = writer.write_all(line.as_bytes()).await;
                crate::lock::pam::wipe(&mut line);
                r?;
                writer.flush().await?;
            }
            Some(Line::Error(t)) => prompter.show(req, &t, true),
            Some(Line::Info(t)) => prompter.show(req, &t, false),
            Some(Line::Success) => return Ok(Outcome::Success),
            Some(Line::Failure) => return Ok(Outcome::Failure),
            None => tracing::debug!("polkit : ligne inattendue de l'outil"),
        }
    }
    Ok(Outcome::Failure)
}

/// Une tentative : socket de polkit, sinon l'outil setuid.
pub async fn attempt(req: &Request, prompter: &dyn Prompter) -> std::io::Result<Outcome> {
    if let Ok(stream) = tokio::net::UnixStream::connect(SOCKET).await {
        let (r, w) = stream.into_split();
        return converse(r, w, req, true, prompter).await;
    }
    let helper = HELPERS
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .ok_or_else(|| {
            std::io::Error::other(crate::i18n::tr("polkit-agent-helper-1 introuvable"))
        })?;
    let mut child = tokio::process::Command::new(helper)
        .arg(&req.user)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let (Some(w), Some(r)) = (child.stdin.take(), child.stdout.take()) else {
        return Err(std::io::Error::other("stdio"));
    };
    let out = converse(r, w, req, false, prompter).await;
    let _ = child.kill().await;
    out
}

/// Identité à authentifier parmi celles proposées par polkit : l'utilisateur
/// lui-même s'il est admis, sinon le premier compte proposé (root, ou un
/// administrateur). Les groupes ne s'authentifient pas directement.
pub fn pick_identity(ids: &[(String, HashMap<String, OwnedValue>)], me: u32) -> Option<u32> {
    let users: Vec<u32> = ids
        .iter()
        .filter(|(kind, _)| kind == "unix-user")
        .filter_map(|(_, d)| u32::try_from(d.get("uid")?.try_clone().ok()?).ok())
        .collect();
    users
        .iter()
        .copied()
        .find(|u| *u == me)
        .or(users.first().copied())
}

/// Nom du compte d'un uid.
pub fn user_name(uid: u32) -> Option<String> {
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buf = vec![0 as libc::c_char; 4096];
    let mut out: *mut libc::passwd = std::ptr::null_mut();
    // SAFETY : tampons possédés ici, taille passée ; résultat lu seulement si non nul.
    let rc = unsafe { libc::getpwuid_r(uid, &mut pwd, buf.as_mut_ptr(), buf.len(), &mut out) };
    if rc != 0 || out.is_null() {
        return None;
    }
    // SAFETY : pw_name pointe dans `buf`, terminé par NUL.
    let name = unsafe { std::ffi::CStr::from_ptr(pwd.pw_name) };
    Some(name.to_string_lossy().into_owned())
}

/// Instant de démarrage du processus (champ 22 de /proc/<pid>/stat), pour
/// le sujet `unix-process` de polkit.
pub fn start_time(stat: &str) -> Option<u64> {
    // Le nom du processus (entre parenthèses) peut contenir des espaces.
    let after = &stat[stat.rfind(')')? + 2..];
    after.split_whitespace().nth(19)?.parse().ok()
}

/// L'agent publié sur le bus système.
struct Agent {
    prompter: Arc<dyn Prompter>,
    /// Demandes en cours (cookie → signal d'annulation).
    active: Arc<Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>>,
}

#[zbus::interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl Agent {
    async fn begin_authentication(
        &self,
        action_id: &str,
        message: &str,
        _icon_name: &str,
        _details: HashMap<String, String>,
        cookie: &str,
        identities: Vec<(String, HashMap<String, OwnedValue>)>,
    ) -> zbus::fdo::Result<()> {
        let me = unsafe { libc::getuid() };
        let uid = pick_identity(&identities, me).ok_or_else(|| {
            zbus::fdo::Error::Failed(crate::i18n::tr("aucune identité utilisable").into())
        })?;
        let user = user_name(uid)
            .ok_or_else(|| zbus::fdo::Error::Failed(crate::i18n::tr("compte inconnu").into()))?;
        let (cancel_tx, mut cancel) = tokio::sync::watch::channel(false);
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(cookie.to_string(), cancel_tx);
        let mut req = Request {
            action: action_id.into(),
            message: message.into(),
            user,
            cookie: cookie.into(),
            attempt: 0,
        };
        let mut result = Outcome::Failure;
        for n in 1..=ATTEMPTS {
            req.attempt = n;
            result = tokio::select! {
                r = attempt(&req, self.prompter.as_ref()) => r.unwrap_or_else(|e| {
                    tracing::warn!("{}", crate::tr!("polkit : {e}", e = e));
                    Outcome::Failure
                }),
                _ = cancel.changed() => Outcome::Cancelled,
            };
            if result != Outcome::Failure {
                break;
            }
            if n < ATTEMPTS {
                self.prompter.show(
                    &req,
                    crate::i18n::tr("Mot de passe refusé, réessayez."),
                    true,
                );
            }
        }
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(cookie);
        self.prompter.done(&req, result == Outcome::Success);
        match result {
            Outcome::Success => Ok(()),
            Outcome::Cancelled => Err(zbus::fdo::Error::Failed(
                crate::i18n::tr("annulé par l'utilisateur").into(),
            )),
            Outcome::Failure => Err(zbus::fdo::Error::Failed(
                crate::i18n::tr("authentification refusée").into(),
            )),
        }
    }

    async fn cancel_authentication(&self, cookie: &str) -> zbus::fdo::Result<()> {
        if let Some(tx) = self
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(cookie)
        {
            let _ = tx.send(true);
        }
        Ok(())
    }
}

/// État de l'agent du démon, pour `hyprdesk status` et l'écran des conflits.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AgentState {
    /// Pas encore tenté (ou en cours).
    #[default]
    Starting,
    /// Enregistré pour cette session (identifiant logind).
    Active(String),
    /// Un autre agent tient la session (son programme, s'il est reconnu).
    Busy(Option<String>),
    /// Échec d'une autre nature (message D-Bus).
    Error(String),
}

impl AgentState {
    pub fn text(&self) -> String {
        match self {
            AgentState::Starting => crate::i18n::tr("démarrage").to_string(),
            AgentState::Active(s) => crate::tr!("actif (session {s})", s = s),
            AgentState::Busy(Some(h)) => crate::tr!("occupé ({h})", h = h),
            AgentState::Busy(None) => crate::i18n::tr("occupé par un autre agent").to_string(),
            AgentState::Error(e) => crate::tr!("erreur : {e}", e = e),
        }
    }
}

static STATE: Mutex<AgentState> = Mutex::new(AgentState::Starting);

pub fn state() -> AgentState {
    STATE.lock().map(|s| s.clone()).unwrap_or_default()
}

pub fn set_state(s: AgentState) {
    if let Ok(mut st) = STATE.lock() {
        *st = s;
    }
}

/// Refus parce qu'un agent tient déjà la session (sinon : autre erreur).
pub fn is_busy_error(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("already exists") || m.contains("already registered")
}

/// Délai avant le prochain essai : court tant qu'un autre agent tient la
/// session (reprise dès qu'il s'arrête), croissant pour une autre erreur.
pub fn retry_delay(attempt: u32, busy: bool) -> std::time::Duration {
    if busy {
        return std::time::Duration::from_secs(5);
    }
    std::time::Duration::from_secs((5u64 << attempt.min(4)).min(60))
}

/// Session logind de ce processus (`GetSessionByPID`, puis `Id`), sinon
/// `$XDG_SESSION_ID`.
async fn session_id(conn: &zbus::Connection) -> Option<String> {
    let from_logind = async {
        let reply = conn
            .call_method(
                Some("org.freedesktop.login1"),
                "/org/freedesktop/login1",
                Some("org.freedesktop.login1.Manager"),
                "GetSessionByPID",
                &(std::process::id()),
            )
            .await
            .ok()?;
        let path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize().ok()?;
        let reply = conn
            .call_method(
                Some("org.freedesktop.login1"),
                path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.login1.Session", "Id"),
            )
            .await
            .ok()?;
        let v: OwnedValue = reply.body().deserialize().ok()?;
        String::try_from(v).ok().filter(|s| !s.is_empty())
    };
    match from_logind.await {
        Some(id) => Some(id),
        None => std::env::var("XDG_SESSION_ID")
            .ok()
            .filter(|s| !s.is_empty()),
    }
}

/// Publie l'agent et l'enregistre auprès de polkit pour la session de ce
/// processus ; rend la connexion et l'identifiant de session. Erreur si
/// polkit refuse (un autre agent tient déjà la session : `is_busy_error`).
pub async fn register(prompter: Arc<dyn Prompter>) -> zbus::Result<(zbus::Connection, String)> {
    let conn = zbus::Connection::system().await?;
    conn.object_server()
        .at(
            OBJECT_PATH,
            Agent {
                prompter,
                active: Default::default(),
            },
        )
        .await?;
    let locale = std::env::var("LANG").unwrap_or_else(|_| "C".into());
    let mut d: HashMap<&str, Value> = HashMap::new();
    // La session (et non ce seul processus) : c'est d'elle que viennent
    // les demandes de pkexec, des gestionnaires de paquets, d'udisks…
    let (kind, label) = match session_id(&conn).await {
        Some(id) => {
            d.insert("session-id", Value::from(id.clone()));
            ("unix-session", id)
        }
        None => {
            let pid = std::process::id();
            let start = std::fs::read_to_string("/proc/self/stat")
                .ok()
                .and_then(|s| start_time(&s))
                .unwrap_or(0);
            d.insert("pid", Value::from(pid));
            d.insert("start-time", Value::from(start));
            tracing::warn!(
                "{}",
                crate::i18n::tr(
                    "agent polkit : session logind introuvable, enregistré pour ce processus seulement"
                )
            );
            ("unix-process", format!("pid {pid}"))
        }
    };
    conn.call_method(
        Some("org.freedesktop.PolicyKit1"),
        "/org/freedesktop/PolicyKit1/Authority",
        Some("org.freedesktop.PolicyKit1.Authority"),
        "RegisterAuthenticationAgent",
        &((kind, d), locale.as_str(), OBJECT_PATH),
    )
    .await?;
    Ok((conn, label))
}

/// Saisie au terminal (`hyprdesk polkit-agent`), à la manière de pkttyagent.
pub struct TtyPrompter;

pub(crate) fn read_tty(echo: bool) -> Option<String> {
    use std::io::BufRead;
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .ok()?;
    let fd = std::os::fd::AsRawFd::as_raw_fd(&tty);
    // SAFETY : termios lu puis restauré sur le terminal ouvert ici.
    let mut old: libc::termios = unsafe { std::mem::zeroed() };
    let have = unsafe { libc::tcgetattr(fd, &mut old) } == 0;
    if have && !echo {
        let mut t = old;
        t.c_lflag &= !libc::ECHO;
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &t) };
    }
    let mut line = String::new();
    let r = std::io::BufReader::new(&tty).read_line(&mut line);
    if have && !echo {
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &old) };
        eprintln!();
    }
    match r {
        Ok(0) | Err(_) => None,
        Ok(_) => {
            while line.ends_with(['\n', '\r']) {
                line.pop();
            }
            Some(line)
        }
    }
}

impl Prompter for TtyPrompter {
    fn prompt<'a>(&'a self, req: &'a Request, text: &'a str, echo: bool) -> Reply<'a> {
        Box::pin(async move {
            if req.attempt == 1 {
                eprintln!("\n{}", req.message);
                eprintln!("{}", crate::tr!("Action : {a}", a = req.action));
            }
            eprint!("{} ({}) ", text.trim_end(), req.user);
            tokio::task::spawn_blocking(move || read_tty(echo))
                .await
                .ok()
                .flatten()
        })
    }

    fn show(&self, _req: &Request, text: &str, error: bool) {
        if error {
            eprintln!("! {text}");
        } else {
            eprintln!("{text}");
        }
    }

    fn done(&self, _req: &Request, ok: bool) {
        eprintln!(
            "{}",
            if ok {
                crate::i18n::tr("Autorisé.")
            } else {
                crate::i18n::tr("Refusé.")
            }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lignes_de_l_outil() {
        assert_eq!(
            parse_line("PAM_PROMPT_ECHO_OFF Password: "),
            Some(Line::Prompt {
                echo: false,
                text: "Password: ".into()
            })
        );
        assert_eq!(
            parse_line("PAM_ERROR_MSG Compte\\nverrouill\\303\\251"),
            Some(Line::Error("Compte\nverrouillé".into()))
        );
        assert_eq!(
            parse_line("PAM_TEXT_INFO touchez le capteur"),
            Some(Line::Info("touchez le capteur".into()))
        );
        assert_eq!(parse_line("SUCCESS"), Some(Line::Success));
        assert_eq!(parse_line("FAILURE"), Some(Line::Failure));
        assert_eq!(parse_line("bruit"), None);
        assert_eq!(unescape("a\\\\b\\\"c\\t"), "a\\b\"c\t");
    }

    #[test]
    fn identite_choisie() {
        let user = |uid: u32| {
            let mut d = HashMap::new();
            d.insert("uid".to_string(), OwnedValue::from(uid));
            ("unix-user".to_string(), d)
        };
        let mut g = HashMap::new();
        g.insert("gid".to_string(), OwnedValue::from(10u32));
        let ids = vec![("unix-group".to_string(), g), user(0), user(1000)];
        assert_eq!(pick_identity(&ids, 1000), Some(1000));
        assert_eq!(pick_identity(&ids, 1001), Some(0));
        assert_eq!(pick_identity(&ids[..1], 1000), None);
        assert_eq!(user_name(0).as_deref(), Some("root"));
    }

    #[test]
    fn instant_de_demarrage() {
        let stat = "4242 (mon (drôle) nom) S 1 4242 4242 0 -1 4194560 100 0 0 0 1 2 0 0 20 0 1 0 987654 1000 10";
        assert_eq!(start_time(stat), Some(987654));
    }

    /// Faux outil : vérifie ce que l'agent écrit, joue un échange PAM.
    struct Fake {
        answer: Option<&'static str>,
        shown: Mutex<Vec<String>>,
    }

    impl Prompter for Fake {
        fn prompt<'a>(&'a self, _req: &'a Request, _text: &'a str, _echo: bool) -> Reply<'a> {
            Box::pin(async move { self.answer.map(String::from) })
        }
        fn show(&self, _req: &Request, text: &str, _error: bool) {
            self.shown.lock().unwrap().push(text.into());
        }
        fn done(&self, _req: &Request, _ok: bool) {}
    }

    fn req() -> Request {
        Request {
            action: "a".into(),
            message: "m".into(),
            user: "alice".into(),
            cookie: "c00k1e".into(),
            attempt: 1,
        }
    }

    async fn helper_side(
        stream: tokio::io::DuplexStream,
        expected_password: &'static str,
        socket: bool,
    ) -> bool {
        let (r, mut w) = tokio::io::split(stream);
        let mut lines = BufReader::new(r).lines();
        if socket {
            assert_eq!(lines.next_line().await.unwrap().as_deref(), Some("alice"));
        }
        assert_eq!(lines.next_line().await.unwrap().as_deref(), Some("c00k1e"));
        w.write_all(b"PAM_TEXT_INFO bonjour\nPAM_PROMPT_ECHO_OFF Mot de passe : \n")
            .await
            .unwrap();
        let Ok(Some(pw)) = lines.next_line().await else {
            return false;
        };
        let ok = pw == expected_password;
        w.write_all(if ok { b"SUCCESS\n" } else { b"FAILURE\n" })
            .await
            .unwrap();
        ok
    }

    #[tokio::test]
    async fn conversation_complete() {
        for (answer, socket, want) in [
            (Some("s3cret"), true, Outcome::Success),
            (Some("faux"), false, Outcome::Failure),
        ] {
            let (agent, helper) = tokio::io::duplex(1024);
            let h = tokio::spawn(helper_side(helper, "s3cret", socket));
            let fake = Fake {
                answer,
                shown: Mutex::new(Vec::new()),
            };
            let (r, w) = tokio::io::split(agent);
            let out = converse(r, w, &req(), socket, &fake).await.unwrap();
            assert_eq!(out, want);
            assert_eq!(h.await.unwrap(), want == Outcome::Success);
            assert_eq!(fake.shown.lock().unwrap().as_slice(), ["bonjour"]);
        }
    }

    #[tokio::test]
    async fn annulation_pendant_la_saisie() {
        let (agent, helper) = tokio::io::duplex(1024);
        let h = tokio::spawn(helper_side(helper, "x", true));
        let fake = Fake {
            answer: None,
            shown: Mutex::new(Vec::new()),
        };
        let (r, w) = tokio::io::split(agent);
        assert_eq!(
            converse(r, w, &req(), true, &fake).await.unwrap(),
            Outcome::Cancelled
        );
        drop(h);
    }

    #[test]
    fn etat_et_reprise() {
        assert!(is_busy_error(
            "org.freedesktop.PolicyKit1.Error.Failed: An authentication agent already exists for the given subject"
        ));
        assert!(!is_busy_error("org.freedesktop.DBus.Error.ServiceUnknown"));
        // Session occupée : un nouvel essai toutes les 5 s (reprise rapide).
        assert_eq!(retry_delay(9, true), std::time::Duration::from_secs(5));
        // Autre erreur : 5, 10, 20, 40, puis 60 s au plus.
        let d: Vec<u64> = (0..6).map(|a| retry_delay(a, false).as_secs()).collect();
        assert_eq!(d, [5, 10, 20, 40, 60, 60]);
        assert!(AgentState::Active("3".into()).text().contains('3'));
        assert!(
            AgentState::Busy(Some("polkit-kde-authentication-agent-1".into()))
                .text()
                .contains("polkit-kde")
        );
    }
}
