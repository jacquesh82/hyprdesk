//! Vérification du mot de passe par PAM, liaison minimale écrite à la main
//! (`pam_start`, `pam_authenticate`, `pam_acct_mgmt`, `pam_setcred`,
//! `pam_end`), comme le font hyprlock et swaylock.
//!
//! Service du verrou ([`service`]) : `/etc/pam.d/hyprdesk-lock` (livré par
//! les paquets et par `install.sh --from-source` : pile d'authentification
//! du système et phase account), sinon celui de hyprlock s'il déclare une
//! phase account, sinon `login`. Après l'authentification, la phase
//! account est toujours évaluée : un compte expiré, désactivé ou refusé par
//! pam_access ne déverrouille pas.
//!
//! Empreinte ([`fingerprint_service`]) : si l'administrateur a posé
//! `/etc/pam.d/hyprdesk-fingerprint` (pam_fprintd, au choix), l'empreinte
//! passe entièrement par PAM ([`fingerprint`]) ; sinon fprintd est lu par
//! D-Bus et la phase account du service du verrou ([`account`]) décide.
//! Chaque déverrouillage par empreinte est journalisé (authpriv).
//!
//! Le mot de passe n'est jamais journalisé ; sa copie remise à PAM est
//! allouée par `libc` (PAM la libère), la copie de la conversation et le
//! tampon de l'appelant sont effacés par `zeroize`, que l'optimiseur ne
//! supprime pas. Les messages de PAM (compte bloqué par faillock, lecteur
//! d'empreinte…) sont rendus pour être affichés.

use std::ffi::{CStr, CString, c_char, c_int, c_void};

use zeroize::Zeroize;

const PAM_SUCCESS: c_int = 0;
const PAM_PROMPT_ECHO_OFF: c_int = 1;
const PAM_PROMPT_ECHO_ON: c_int = 2;
const PAM_ERROR_MSG: c_int = 3;
const PAM_TEXT_INFO: c_int = 4;
const PAM_BUF_ERR: c_int = 5;
const PAM_CONV_ERR: c_int = 19;
const PAM_NEW_AUTHTOK_REQD: c_int = 12;
const PAM_REFRESH_CRED: c_int = 0x0010;

#[repr(C)]
struct PamMessage {
    msg_style: c_int,
    msg: *const c_char,
}

#[repr(C)]
struct PamResponse {
    resp: *mut c_char,
    resp_retcode: c_int,
}

#[repr(C)]
struct PamConv {
    conv: extern "C" fn(c_int, *mut *const PamMessage, *mut *mut PamResponse, *mut c_void) -> c_int,
    appdata_ptr: *mut c_void,
}

#[link(name = "pam")]
unsafe extern "C" {
    fn pam_start(
        service: *const c_char,
        user: *const c_char,
        conv: *const PamConv,
        handle: *mut *mut c_void,
    ) -> c_int;
    fn pam_authenticate(handle: *mut c_void, flags: c_int) -> c_int;
    fn pam_acct_mgmt(handle: *mut c_void, flags: c_int) -> c_int;
    fn pam_setcred(handle: *mut c_void, flags: c_int) -> c_int;
    fn pam_end(handle: *mut c_void, status: c_int) -> c_int;
    fn pam_strerror(handle: *mut c_void, errnum: c_int) -> *const c_char;
}

/// Données de la conversation : ce qu'on répond, ce que PAM dit.
struct Conversation {
    user: CString,
    password: CString,
    messages: Vec<String>,
}

/// Réponses aux questions de PAM : le mot de passe (saisie masquée), le nom
/// d'utilisateur (saisie visible) ; les messages sont recueillis.
extern "C" fn converse(
    num: c_int,
    msgs: *mut *const PamMessage,
    resp: *mut *mut PamResponse,
    appdata: *mut c_void,
) -> c_int {
    if num <= 0 || msgs.is_null() || resp.is_null() || appdata.is_null() {
        return PAM_CONV_ERR;
    }
    // SAFETY : PAM fournit `num` messages valides le temps de l'appel, et
    // `appdata` est la `Conversation` passée à `pam_start`, vivante jusqu'à `pam_end`.
    unsafe {
        let conv = &mut *(appdata as *mut Conversation);
        let out =
            libc::calloc(num as usize, std::mem::size_of::<PamResponse>()) as *mut PamResponse;
        if out.is_null() {
            return PAM_BUF_ERR;
        }
        for i in 0..num as usize {
            // Linux-PAM : tableau de pointeurs vers des messages.
            let m = &**msgs.add(i);
            let text = if m.msg.is_null() {
                String::new()
            } else {
                CStr::from_ptr(m.msg).to_string_lossy().trim().to_string()
            };
            let r = &mut *out.add(i);
            match m.msg_style {
                PAM_PROMPT_ECHO_OFF => r.resp = libc::strdup(conv.password.as_ptr()),
                PAM_PROMPT_ECHO_ON => r.resp = libc::strdup(conv.user.as_ptr()),
                PAM_ERROR_MSG | PAM_TEXT_INFO => {
                    if !text.is_empty() {
                        conv.messages.push(text);
                    }
                }
                _ => {
                    // Style inconnu : on refuse proprement, en libérant ce qui est déjà alloué.
                    for j in 0..num as usize {
                        let p = (*out.add(j)).resp;
                        if !p.is_null() {
                            libc::free(p as *mut c_void);
                        }
                    }
                    libc::free(out as *mut c_void);
                    return PAM_CONV_ERR;
                }
            }
        }
        *resp = out;
    }
    PAM_SUCCESS
}

/// Résultat d'une tentative : réussite, ou raison de l'échec ; avec les
/// messages de PAM à afficher.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub ok: bool,
    pub error: Option<String>,
    pub messages: Vec<String>,
}

/// Service PAM : le nôtre s'il est installé, sinon celui de hyprlock s'il
/// déclare une phase account, sinon `login`.
///
/// hyprlock n'appelle que `pam_authenticate` : le fichier de son paquet
/// (Arch : `auth include login`, rien d'autre) n'a pas de phase account, que
/// Linux-PAM prend alors dans `other` (`pam_deny`). Le choisir refuserait
/// tout déverrouillage, mot de passe juste compris.
pub fn service() -> &'static str {
    let read = |s: &str| std::fs::read_to_string(std::path::Path::new("/etc/pam.d").join(s));
    if std::path::Path::new("/etc/pam.d/hyprdesk-lock").is_file() {
        "hyprdesk-lock"
    } else if read("hyprlock").is_ok_and(|t| declares_account(&t)) {
        "hyprlock"
    } else {
        "login"
    }
}

/// Le fichier de service déclare-t-il une phase account (ligne `account`,
/// `-account`, ou `@include` à la Debian, qui l'apporte) ?
fn declares_account(text: &str) -> bool {
    text.lines().any(|l| {
        let first = l.split_whitespace().next().unwrap_or("");
        matches!(first, "account" | "-account" | "@include")
    })
}

/// Utilisateur de la session.
pub fn current_user() -> String {
    // SAFETY : getpwuid rend un pointeur vers une structure statique, lue aussitôt.
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if !pw.is_null() && !(*pw).pw_name.is_null() {
            return CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned();
        }
    }
    std::env::var("USER").unwrap_or_default()
}

/// Service PAM dédié à l'empreinte, si l'administrateur l'a posé.
pub fn fingerprint_service() -> Option<&'static str> {
    std::path::Path::new("/etc/pam.d/hyprdesk-fingerprint")
        .is_file()
        .then_some("hyprdesk-fingerprint")
}

/// Ce que la tentative demande à PAM.
#[derive(Clone, Copy, PartialEq)]
enum Steps {
    /// Authentification puis phase account.
    Authenticate,
    /// Phase account seule (empreinte déjà vérifiée par fprintd).
    AccountOnly,
}

/// Vérifie le mot de passe de `user` (bloquant : à lancer hors du fil de l'interface).
pub fn authenticate(user: &str, password: &str) -> Outcome {
    converse_with(service(), user, password, Steps::Authenticate)
}

/// Empreinte par le service PAM dédié (pam_fprintd attend le doigt :
/// bloquant). Journalisé en cas de succès.
pub fn fingerprint(user: &str) -> Outcome {
    let Some(svc) = fingerprint_service() else {
        return Outcome {
            ok: false,
            error: None,
            messages: Vec::new(),
        };
    };
    let o = converse_with(svc, user, "", Steps::Authenticate);
    if o.ok {
        journal(&format!(
            "session de {user} déverrouillée par empreinte (PAM, service {svc})"
        ));
    }
    o
}

/// Empreinte reconnue par fprintd (D-Bus) : la phase account du service du
/// verrou décide encore (compte expiré, désactivé…). Journalisé.
pub fn account(user: &str) -> Outcome {
    let o = converse_with(service(), user, "", Steps::AccountOnly);
    journal(&if o.ok {
        format!(
            "session de {user} déverrouillée par empreinte (fprintd, account {})",
            service()
        )
    } else {
        format!("empreinte de {user} reconnue, compte refusé par PAM")
    });
    o
}

/// Message de refus de la phase account.
fn account_error(r: c_int, strerror: Option<String>) -> String {
    if r == PAM_NEW_AUTHTOK_REQD {
        crate::i18n::tr(
            "Mot de passe expiré : changez-le (passwd, sur une console Ctrl+Alt+F3), puis déverrouillez.",
        )
        .to_string()
    } else {
        crate::tr!(
            "Compte refusé : {e}",
            e = strerror.unwrap_or_else(|| crate::tr!("échec (code {r})", r = r))
        )
    }
}

/// Journal d'authentification du système (authpriv) : déverrouillages par
/// empreinte, qui ne laissent sinon pas toujours de trace PAM.
pub fn journal(msg: &str) {
    let Ok(m) = CString::new(msg.replace('\0', "")) else {
        return;
    };
    // SAFETY : chaînes C valides le temps de l'appel ; format fixe « %s ».
    unsafe {
        libc::openlog(c"hyprdesk-lock".as_ptr(), libc::LOG_PID, libc::LOG_AUTHPRIV);
        libc::syslog(libc::LOG_NOTICE, c"%s".as_ptr(), m.as_ptr());
        libc::closelog();
    }
}

fn converse_with(svc: &str, user: &str, password: &str, steps: Steps) -> Outcome {
    let fail = |e: &str| Outcome {
        ok: false,
        error: Some(e.to_string()),
        messages: Vec::new(),
    };
    let (Ok(cuser), Ok(cpass), Ok(svc)) = (
        CString::new(user),
        CString::new(password),
        CString::new(svc),
    ) else {
        return fail(crate::i18n::tr("caractère nul dans la saisie"));
    };
    let mut conv = Box::new(Conversation {
        user: cuser.clone(),
        password: cpass,
        messages: Vec::new(),
    });
    let pc = PamConv {
        conv: converse,
        appdata_ptr: &mut *conv as *mut Conversation as *mut c_void,
    };
    let mut handle: *mut c_void = std::ptr::null_mut();
    // SAFETY : chaînes et conversation valides jusqu'à pam_end ; le handle n'est
    // utilisé qu'entre pam_start réussi et pam_end.
    let (ok, error) = unsafe {
        let r = pam_start(svc.as_ptr(), cuser.as_ptr(), &pc, &mut handle);
        if r != PAM_SUCCESS || handle.is_null() {
            (false, Some(format!("PAM indisponible (code {r})")))
        } else {
            let strerror = |r: c_int| {
                let e = pam_strerror(handle, r);
                (!e.is_null()).then(|| CStr::from_ptr(e).to_string_lossy().into_owned())
            };
            let mut r = if steps == Steps::Authenticate {
                pam_authenticate(handle, 0)
            } else {
                PAM_SUCCESS
            };
            let res = if r != PAM_SUCCESS {
                let msg = strerror(r).unwrap_or_else(|| crate::tr!("échec (code {r})", r = r));
                (false, Some(msg))
            } else {
                // Phase account : compte expiré, désactivé, pam_access…
                r = pam_acct_mgmt(handle, 0);
                if r == PAM_SUCCESS {
                    // Rafraîchir les jetons (Kerberos…), sans effet sur l'issue.
                    let _ = pam_setcred(handle, PAM_REFRESH_CRED);
                    (true, None)
                } else {
                    (false, Some(account_error(r, strerror(r))))
                }
            };
            pam_end(handle, r);
            res
        }
    };
    // Effacer la copie du mot de passe gardée pour la conversation.
    let mut bytes = std::mem::take(&mut conv.password).into_bytes_with_nul();
    bytes.zeroize();
    Outcome {
        ok,
        error,
        messages: std::mem::take(&mut conv.messages),
    }
}

/// Efface un mot de passe en mémoire avant de le rendre (écritures que
/// l'optimiseur ne supprime pas).
pub fn wipe(s: &mut String) {
    s.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La conversation répond au mot de passe et à l'utilisateur, recueille
    /// les messages, et refuse un style inconnu.
    #[test]
    fn conversation() {
        let mut conv = Conversation {
            user: CString::new("alice").unwrap(),
            password: CString::new("secret").unwrap(),
            messages: Vec::new(),
        };
        let texts = [
            CString::new("Password: ").unwrap(),
            CString::new("login:").unwrap(),
            CString::new("The account is locked due to 3 failed logins.").unwrap(),
        ];
        let ms = [
            PamMessage {
                msg_style: PAM_PROMPT_ECHO_OFF,
                msg: texts[0].as_ptr(),
            },
            PamMessage {
                msg_style: PAM_PROMPT_ECHO_ON,
                msg: texts[1].as_ptr(),
            },
            PamMessage {
                msg_style: PAM_ERROR_MSG,
                msg: texts[2].as_ptr(),
            },
        ];
        let mut ptrs: Vec<*const PamMessage> = ms.iter().map(|m| m as *const _).collect();
        let mut resp: *mut PamResponse = std::ptr::null_mut();
        let r = converse(
            3,
            ptrs.as_mut_ptr(),
            &mut resp,
            &mut conv as *mut _ as *mut c_void,
        );
        assert_eq!(r, PAM_SUCCESS);
        // SAFETY : réponses allouées par converse, libérées ici comme le ferait PAM.
        unsafe {
            let a = CStr::from_ptr((*resp).resp).to_str().unwrap().to_string();
            let b = CStr::from_ptr((*resp.add(1)).resp)
                .to_str()
                .unwrap()
                .to_string();
            assert_eq!((a.as_str(), b.as_str()), ("secret", "alice"));
            assert!((*resp.add(2)).resp.is_null());
            for i in 0..3 {
                libc::free((*resp.add(i)).resp as *mut c_void);
            }
            libc::free(resp as *mut c_void);
        }
        assert_eq!(
            conv.messages,
            ["The account is locked due to 3 failed logins."]
        );
        let bad = [PamMessage {
            msg_style: 99,
            msg: texts[0].as_ptr(),
        }];
        let mut p: Vec<*const PamMessage> = bad.iter().map(|m| m as *const _).collect();
        let r = converse(
            1,
            p.as_mut_ptr(),
            &mut resp,
            &mut conv as *mut _ as *mut c_void,
        );
        assert_eq!(r, PAM_CONV_ERR);
        let mut s = String::from("motdepasse");
        wipe(&mut s);
        assert!(s.is_empty());
        assert!(!current_user().is_empty());
    }

    /// Le fichier de hyprlock (Arch) n'a pas de phase account : écarté.
    #[test]
    fn phase_account_declaree() {
        let hyprlock = "# PAM configuration file for hyprlock\n\
                        # the 'login' configuration file (see /etc/pam.d/login)\n\n\
                        auth        include     login\n";
        assert!(!declares_account(hyprlock));
        assert!(!declares_account("# account include login\n"));
        assert!(declares_account(
            "auth include login\naccount include login\n"
        ));
        assert!(declares_account("-account optional pam_foo.so\n"));
        assert!(declares_account("@include common-auth\n"));
    }

    #[test]
    fn refus_de_la_phase_account() {
        assert!(account_error(PAM_NEW_AUTHTOK_REQD, None).contains("expiré"));
        assert!(
            account_error(13, Some("User account has expired".into()))
                .contains("User account has expired")
        );
    }
}
