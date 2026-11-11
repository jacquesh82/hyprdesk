//! Écran de connexion pour greetd (MLG-23) : protocole IPC de greetd et
//! liste des sessions. greetd lance le greeter avec `$GREETD_SOCK` ; chaque
//! message est un objet JSON précédé de sa longueur (u32, ordre natif).
//!
//! Conversation : `create_session { username }`, puis tant que greetd pose
//! des questions (`auth_message` : visible, secret, info, error), une
//! réponse `post_auth_message_response` ; une fois `success`,
//! `start_session { cmd, env }` ; à la moindre erreur, `cancel_session`.
//!
//! `hyprdesk greeter --tty` mène cette conversation au terminal (essai, ou
//! greeter de secours). L'écran graphique (reprise de `src/lock/`) s'appuie
//! sur la même fonction `login`.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    CreateSession { username: String },
    PostAuthMessageResponse { response: Option<String> },
    StartSession { cmd: Vec<String>, env: Vec<String> },
    CancelSession,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    Visible,
    Secret,
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Success,
    Error {
        error_type: String,
        description: String,
    },
    AuthMessage {
        auth_message_type: AuthKind,
        auth_message: String,
    },
}

pub fn write_msg<T: serde::Serialize>(s: &mut impl Write, msg: &T) -> std::io::Result<()> {
    let body = serde_json::to_vec(msg).map_err(std::io::Error::other)?;
    s.write_all(&(body.len() as u32).to_ne_bytes())?;
    s.write_all(&body)?;
    s.flush()
}

pub fn read_msg<T: serde::de::DeserializeOwned>(s: &mut impl Read) -> std::io::Result<T> {
    let mut len = [0u8; 4];
    s.read_exact(&mut len)?;
    let len = u32::from_ne_bytes(len) as usize;
    if len > 1 << 20 {
        return Err(std::io::Error::other("message greetd trop long"));
    }
    let mut body = vec![0u8; len];
    s.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(std::io::Error::other)
}

/// Session proposée (fichier .desktop de wayland-sessions ou xsessions).
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub id: String,
    pub name: String,
    /// Commande, codes de champ retirés.
    pub exec: String,
    pub wayland: bool,
}

fn session_dirs() -> Vec<(PathBuf, bool)> {
    let sys =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    let mut v = Vec::new();
    for d in sys.split(':').filter(|d| !d.is_empty()) {
        v.push((Path::new(d).join("wayland-sessions"), true));
        v.push((Path::new(d).join("xsessions"), false));
    }
    v
}

/// Sessions d'un dossier (masquées et sans Exec écartées).
pub fn sessions_in(dir: &Path, wayland: bool) -> Vec<Session> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let lang = std::env::var("LANG").unwrap_or_default();
    let mut v: Vec<Session> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "desktop"))
        .filter_map(|p| {
            let src = std::fs::read_to_string(&p).ok()?;
            let e = crate::launch::parse_desktop_entry(&src, &lang)?;
            if e.hidden {
                return None;
            }
            Some(Session {
                id: p.file_stem()?.to_string_lossy().into_owned(),
                name: if e.name.is_empty() {
                    p.file_stem()?.to_string_lossy().into_owned()
                } else {
                    e.name.clone()
                },
                exec: crate::launch::strip_field_codes(&e.exec),
                wayland,
            })
        })
        .collect();
    v.sort_by_key(|s| s.name.to_lowercase());
    v
}

/// Toutes les sessions, sans doublon d'identifiant ; Hyprland en tête.
pub fn sessions() -> Vec<Session> {
    let mut v: Vec<Session> = Vec::new();
    for (d, w) in session_dirs() {
        for s in sessions_in(&d, w) {
            if !v.iter().any(|x| x.id == s.id) {
                v.push(s);
            }
        }
    }
    v.sort_by_key(|s| !s.id.to_lowercase().starts_with("hyprland"));
    v
}

/// Shell de connexion d'un utilisateur (/etc/passwd), `/bin/sh` à défaut.
pub fn login_shell(user: &str) -> String {
    shell_in(
        &std::fs::read_to_string("/etc/passwd").unwrap_or_default(),
        user,
    )
}

fn shell_in(passwd: &str, user: &str) -> String {
    passwd
        .lines()
        .map(|l| l.split(':').collect::<Vec<_>>())
        .find(|f| f.first() == Some(&user))
        .and_then(|f| f.get(6).map(|s| s.trim().to_string()))
        .filter(|s| s.starts_with('/') && !s.ends_with("nologin") && !s.ends_with("false"))
        .unwrap_or_else(|| "/bin/sh".into())
}

/// Commande de la session : par le shell de connexion de l'utilisateur
/// (`-l`), qui lit son profil (~/.bash_profile, ~/.profile : PATH,
/// variables), comme GDM.
///
/// Une seule ligne de shell, déjà citée : greetd ne lance pas `cmd` comme
/// une liste d'arguments, il en recolle les éléments par des espaces dans
/// `/bin/sh -c "… ; exec <cmd>"`. Envoyer `["bash", "-l", "-c", "exec X"]`
/// y devenait `bash -l -c exec X` : bash lançait `exec` seul et la session
/// se fermait aussitôt (retour en boucle à l'écran de connexion).
pub fn session_cmd(shell: &str, exec: &str) -> Vec<String> {
    vec![format!(
        "{} -l -c {}",
        crate::launch::sh_quote(shell),
        crate::launch::sh_quote(&format!("exec {exec}"))
    )]
}

/// Environnement de la session lancée.
pub fn session_env(s: &Session) -> Vec<String> {
    let mut env = vec![format!(
        "XDG_SESSION_TYPE={}",
        if s.wayland { "wayland" } else { "x11" }
    )];
    env.push(format!("XDG_SESSION_DESKTOP={}", s.id));
    env
}

/// Une requête, sa réponse (bloquant).
pub fn exchange(stream: &mut UnixStream, req: &Request) -> std::io::Result<Response> {
    write_msg(stream, req)?;
    read_msg(stream)
}

/// Suite à donner à une réponse de greetd (écran graphique, pas à pas).
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Question à poser (visible ou secrète).
    Ask(AuthKind, String),
    /// Message à montrer ; répondre sans texte pour continuer.
    Show(AuthKind, String),
    /// Authentifié : lancer la session.
    Start,
    /// Session lancée : le greeter quitte.
    Done,
    /// Échec (à annuler côté greetd) : le message.
    Fail(String),
}

/// `started` : start_session déjà envoyé.
pub fn step(started: bool, r: Response) -> Step {
    match (started, r) {
        (false, Response::Success) => Step::Start,
        (true, Response::Success) => Step::Done,
        (_, Response::Error { description, .. }) => Step::Fail(description),
        (true, Response::AuthMessage { auth_message, .. }) => Step::Fail(auth_message),
        (
            false,
            Response::AuthMessage {
                auth_message_type: k,
                auth_message: t,
            },
        ) => match k {
            AuthKind::Visible | AuthKind::Secret => Step::Ask(k, t),
            AuthKind::Info | AuthKind::Error => Step::Show(k, t),
        },
    }
}

/// Ce qui répond aux questions de greetd (écran, ou terminal).
pub trait Conversation {
    /// Question : réponse, ou `None` pour abandonner.
    fn ask(&mut self, kind: AuthKind, text: &str) -> Option<String>;
    /// Message à montrer (info, erreur).
    fn show(&mut self, kind: AuthKind, text: &str);
}

/// Connexion complète : authentification de `user`, puis lancement de
/// `cmd`. `Ok` : greetd lance la session (le greeter n'a plus qu'à quitter).
pub fn login(
    stream: &mut UnixStream,
    user: &str,
    cmd: Vec<String>,
    env: Vec<String>,
    conv: &mut dyn Conversation,
) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    let fail = |stream: &mut UnixStream, msg: String| -> Result<(), String> {
        let _ = write_msg(stream, &Request::CancelSession);
        let _ = read_msg::<Response>(stream);
        Err(msg)
    };
    write_msg(
        stream,
        &Request::CreateSession {
            username: user.to_string(),
        },
    )
    .map_err(io)?;
    loop {
        match read_msg::<Response>(stream).map_err(io)? {
            Response::Success => break,
            Response::Error { description, .. } => return fail(stream, description),
            Response::AuthMessage {
                auth_message_type: kind,
                auth_message: text,
            } => {
                let response = match kind {
                    AuthKind::Visible | AuthKind::Secret => match conv.ask(kind, &text) {
                        Some(r) => Some(r),
                        None => return fail(stream, crate::i18n::tr("Connexion annulée.").into()),
                    },
                    AuthKind::Info | AuthKind::Error => {
                        conv.show(kind, &text);
                        None
                    }
                };
                let mut req = Request::PostAuthMessageResponse { response };
                let r = write_msg(stream, &req);
                // Le secret ne reste pas en mémoire plus que nécessaire.
                if let Request::PostAuthMessageResponse { response: Some(s) } = &mut req {
                    crate::lock::pam::wipe(s);
                }
                r.map_err(io)?;
            }
        }
    }
    write_msg(stream, &Request::StartSession { cmd, env }).map_err(io)?;
    match read_msg::<Response>(stream).map_err(io)? {
        Response::Success => Ok(()),
        Response::Error { description, .. } => fail(stream, description),
        Response::AuthMessage { auth_message, .. } => fail(stream, auth_message),
    }
}

/// Conversation au terminal.
struct Tty;

impl Conversation for Tty {
    fn ask(&mut self, kind: AuthKind, text: &str) -> Option<String> {
        eprint!("{} ", text.trim_end());
        crate::polkit::read_tty(kind == AuthKind::Visible)
    }
    fn show(&mut self, _kind: AuthKind, text: &str) {
        eprintln!("{text}");
    }
}

/// `hyprdesk greeter --tty` : identifiant, session, conversation.
pub fn run_tty() -> Result<(), String> {
    let sock = std::env::var_os("GREETD_SOCK")
        .ok_or_else(|| crate::i18n::tr("GREETD_SOCK absent : lancer depuis greetd.").to_string())?;
    let list = sessions();
    if list.is_empty() {
        return Err(crate::i18n::tr("Aucune session (wayland-sessions, xsessions).").into());
    }
    for (i, s) in list.iter().enumerate() {
        eprintln!("  {}. {}", i + 1, s.name);
    }
    loop {
        eprint!("{} ", crate::i18n::tr("Session [1] :"));
        let pick = crate::polkit::read_tty(true).unwrap_or_default();
        let s = pick
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|n| list.get(n.wrapping_sub(1)))
            .unwrap_or(&list[0])
            .clone();
        eprint!("{} ", crate::i18n::tr("Identifiant :"));
        let Some(user) = crate::polkit::read_tty(true).filter(|u| !u.trim().is_empty()) else {
            continue;
        };
        let mut stream = UnixStream::connect(&sock).map_err(|e| e.to_string())?;
        let cmd = vec!["sh".into(), "-c".into(), format!("exec {}", s.exec)];
        match login(&mut stream, user.trim(), cmd, session_env(&s), &mut Tty) {
            Ok(()) => return Ok(()),
            Err(e) => eprintln!("{e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_par_le_shell_de_connexion() {
        let pw = "root:x:0:0::/root:/usr/bin/bash\njacques:x:1000:1000::/home/jacques:/usr/bin/zsh\nsvc:x:5:5::/:/usr/bin/nologin\n";
        assert_eq!(shell_in(pw, "jacques"), "/usr/bin/zsh");
        assert_eq!(shell_in(pw, "svc"), "/bin/sh");
        assert_eq!(shell_in(pw, "inconnu"), "/bin/sh");
        assert_eq!(
            session_cmd("/usr/bin/bash", "start-hyprland"),
            vec!["'/usr/bin/bash' -l -c 'exec start-hyprland'"]
        );
        // Recollée par greetd (« exec » + éléments séparés d'espaces), la
        // ligne reste une seule commande : le programme reçoit tout le -c.
        let cmd = session_cmd("/bin/echo", "uwsm start -e -D Hyprland hyprland.desktop");
        let out = std::process::Command::new("sh")
            .args(["-c", &format!("exec {}", cmd.join(" "))])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "-l -c exec uwsm start -e -D Hyprland hyprland.desktop\n"
        );
    }
    use std::os::unix::net::UnixListener;

    #[test]
    fn messages_au_format_de_greetd() {
        let v = serde_json::to_value(Request::CreateSession {
            username: "u".into(),
        })
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({"type": "create_session", "username": "u"})
        );
        let r: Response = serde_json::from_str(
            r#"{"type":"auth_message","auth_message_type":"secret","auth_message":"Password: "}"#,
        )
        .unwrap();
        assert_eq!(
            r,
            Response::AuthMessage {
                auth_message_type: AuthKind::Secret,
                auth_message: "Password: ".into()
            }
        );
        let mut buf = Vec::new();
        write_msg(&mut buf, &Request::CancelSession).unwrap();
        assert_eq!(&buf[..4], &(buf.len() as u32 - 4).to_ne_bytes());
        let back: serde_json::Value = read_msg(&mut buf.as_slice()).unwrap();
        assert_eq!(back, serde_json::json!({"type": "cancel_session"}));
    }

    struct Script(Vec<&'static str>, Vec<String>);
    impl Conversation for Script {
        fn ask(&mut self, _k: AuthKind, text: &str) -> Option<String> {
            self.1.push(format!("? {text}"));
            (!self.0.is_empty()).then(|| self.0.remove(0).to_string())
        }
        fn show(&mut self, _k: AuthKind, text: &str) {
            self.1.push(format!("! {text}"));
        }
    }

    /// Faux greetd : vérifie les requêtes, rejoue des réponses.
    fn fake(steps: Vec<(serde_json::Value, Response)>) -> (PathBuf, std::thread::JoinHandle<()>) {
        let dir = std::env::temp_dir().join(format!(
            "hyprdesk-greetd-{}-{}",
            std::process::id(),
            steps.len()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("greetd.sock");
        let l = UnixListener::bind(&sock).unwrap();
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            for (want, reply) in steps {
                let got: serde_json::Value = read_msg(&mut s).unwrap();
                assert_eq!(got, want);
                write_msg(&mut s, &reply).unwrap();
            }
        });
        (sock, h)
    }

    #[test]
    fn connexion_reussie() {
        use serde_json::json;
        let (sock, h) = fake(vec![
            (
                json!({"type":"create_session","username":"jacques"}),
                Response::AuthMessage {
                    auth_message_type: AuthKind::Info,
                    auth_message: "Bienvenue".into(),
                },
            ),
            (
                json!({"type":"post_auth_message_response","response":null}),
                Response::AuthMessage {
                    auth_message_type: AuthKind::Secret,
                    auth_message: "Mot de passe :".into(),
                },
            ),
            (
                json!({"type":"post_auth_message_response","response":"s3cret"}),
                Response::Success,
            ),
            (
                json!({"type":"start_session","cmd":["Hyprland"],"env":["XDG_SESSION_TYPE=wayland"]}),
                Response::Success,
            ),
        ]);
        let mut s = UnixStream::connect(&sock).unwrap();
        let mut conv = Script(vec!["s3cret"], Vec::new());
        login(
            &mut s,
            "jacques",
            vec!["Hyprland".into()],
            vec!["XDG_SESSION_TYPE=wayland".into()],
            &mut conv,
        )
        .unwrap();
        h.join().unwrap();
        assert_eq!(conv.1, ["! Bienvenue", "? Mot de passe :"]);
    }

    #[test]
    fn mauvais_mot_de_passe_annule() {
        use serde_json::json;
        let (sock, h) = fake(vec![
            (
                json!({"type":"create_session","username":"x"}),
                Response::AuthMessage {
                    auth_message_type: AuthKind::Secret,
                    auth_message: "Password:".into(),
                },
            ),
            (
                json!({"type":"post_auth_message_response","response":"faux"}),
                Response::Error {
                    error_type: "auth_error".into(),
                    description: "pam_authenticate: AUTH_ERR".into(),
                },
            ),
            (json!({"type":"cancel_session"}), Response::Success),
        ]);
        let mut s = UnixStream::connect(&sock).unwrap();
        let mut conv = Script(vec!["faux"], Vec::new());
        let r = login(&mut s, "x", vec!["sh".into()], vec![], &mut conv);
        h.join().unwrap();
        assert_eq!(r, Err("pam_authenticate: AUTH_ERR".into()));
    }

    #[test]
    fn etapes() {
        let ask = Response::AuthMessage {
            auth_message_type: AuthKind::Secret,
            auth_message: "Password:".into(),
        };
        assert_eq!(
            step(false, ask.clone()),
            Step::Ask(AuthKind::Secret, "Password:".into())
        );
        assert_eq!(step(true, ask), Step::Fail("Password:".into()));
        assert_eq!(step(false, Response::Success), Step::Start);
        assert_eq!(step(true, Response::Success), Step::Done);
        let info = Response::AuthMessage {
            auth_message_type: AuthKind::Info,
            auth_message: "Touchez le lecteur".into(),
        };
        assert_eq!(
            step(false, info),
            Step::Show(AuthKind::Info, "Touchez le lecteur".into())
        );
        let err = Response::Error {
            error_type: "auth_error".into(),
            description: "refusé".into(),
        };
        assert_eq!(step(false, err), Step::Fail("refusé".into()));
    }

    #[test]
    fn sessions_lues() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-sessions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("hyprland.desktop"),
            "[Desktop Entry]\nName=Hyprland\nExec=Hyprland\nType=Application\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("cachee.desktop"),
            "[Desktop Entry]\nName=Cachée\nExec=x\nHidden=true\n",
        )
        .unwrap();
        let s = sessions_in(&dir, true);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].exec, "Hyprland");
        assert_eq!(
            session_env(&s[0]),
            ["XDG_SESSION_TYPE=wayland", "XDG_SESSION_DESKTOP=hyprland"]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
