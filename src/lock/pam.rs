//! Vérification du mot de passe par PAM, liaison minimale écrite à la main
//! (`pam_start`, `pam_authenticate`, `pam_setcred`, `pam_end`), comme le font
//! hyprlock et swaylock.
//!
//! Le mot de passe n'est jamais journalisé ; sa copie remise à PAM est
//! allouée par `libc` (PAM la libère), et le tampon de l'appelant est effacé
//! par [`wipe`]. Les messages de PAM (compte bloqué par faillock, lecteur
//! d'empreinte…) sont rendus pour être affichés.

use std::ffi::{CStr, CString, c_char, c_int, c_void};

const PAM_SUCCESS: c_int = 0;
const PAM_PROMPT_ECHO_OFF: c_int = 1;
const PAM_PROMPT_ECHO_ON: c_int = 2;
const PAM_ERROR_MSG: c_int = 3;
const PAM_TEXT_INFO: c_int = 4;
const PAM_BUF_ERR: c_int = 5;
const PAM_CONV_ERR: c_int = 19;
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

/// Service PAM : le nôtre s'il est installé, sinon celui de hyprlock, sinon `login`.
pub fn service() -> &'static str {
    let has = |s: &str| std::path::Path::new("/etc/pam.d").join(s).is_file();
    if has("hyprdesk-lock") {
        "hyprdesk-lock"
    } else if has("hyprlock") {
        "hyprlock"
    } else {
        "login"
    }
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

/// Vérifie le mot de passe de `user` (bloquant : à lancer hors du fil de l'interface).
pub fn authenticate(user: &str, password: &str) -> Outcome {
    let fail = |e: &str| Outcome {
        ok: false,
        error: Some(e.to_string()),
        messages: Vec::new(),
    };
    let (Ok(cuser), Ok(cpass), Ok(svc)) = (
        CString::new(user),
        CString::new(password),
        CString::new(service()),
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
            let r = pam_authenticate(handle, 0);
            let res = if r == PAM_SUCCESS {
                // Rafraîchir les jetons (Kerberos…), sans effet sur l'issue.
                let _ = pam_setcred(handle, PAM_REFRESH_CRED);
                (true, None)
            } else {
                let e = pam_strerror(handle, r);
                let msg = if e.is_null() {
                    crate::tr!("échec (code {r})", r = r)
                } else {
                    CStr::from_ptr(e).to_string_lossy().into_owned()
                };
                (false, Some(msg))
            };
            pam_end(handle, r);
            res
        }
    };
    // Effacer la copie du mot de passe gardée pour la conversation.
    let mut bytes = std::mem::take(&mut conv.password).into_bytes_with_nul();
    bytes.iter_mut().for_each(|b| *b = 0);
    Outcome {
        ok,
        error,
        messages: std::mem::take(&mut conv.messages),
    }
}

/// Efface un mot de passe en mémoire avant de le rendre.
pub fn wipe(s: &mut String) {
    // SAFETY : on n'écrit que des zéros, qui restent de l'UTF-8 valide.
    unsafe {
        s.as_bytes_mut().iter_mut().for_each(|b| *b = 0);
    }
    s.clear();
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
}
