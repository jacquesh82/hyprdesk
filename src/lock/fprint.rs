//! Empreinte au verrouillage (MLG-25) : vérification fprintd par D-Bus, en
//! parallèle du mot de passe, comme hyprlock. Aucune configuration système
//! (pile PAM) n'est nécessaire : fprintd demande seulement que l'utilisateur
//! de la session active puisse vérifier son empreinte (règle polkit livrée
//! avec fprintd). Sans fprintd ni lecteur : rien ne s'affiche.
//!
//! Déroulé : Manager.GetDefaultDevice, Device.Claim(utilisateur),
//! VerifyStart("any"), puis les signaux VerifyStatus(résultat, fini) ;
//! « verify-match » déverrouille ; un échec relance la vérification
//! (trois essais) ; VerifyStop et Release dans tous les cas.

use iced::futures::StreamExt;

const DEST: &str = "net.reactivated.Fprint";
const MANAGER: &str = "/net/reactivated/Fprint/Manager";
const MANAGER_IFACE: &str = "net.reactivated.Fprint.Manager";
const DEVICE_IFACE: &str = "net.reactivated.Fprint.Device";
/// Essais avant de laisser la place au mot de passe seul.
const TRIES: u32 = 3;

/// Issue de la vérification.
#[derive(Debug, Clone, PartialEq)]
pub enum Finger {
    /// Pas de fprintd, de lecteur, ou d'empreinte enregistrée.
    Unavailable,
    Matched,
    /// Essais épuisés (ou erreur) : le message à montrer.
    Failed(String),
}

/// Suite à donner à un signal VerifyStatus.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Match,
    /// Mauvais doigt : relancer une vérification.
    NoMatch,
    /// Passage à refaire (doigt mal posé…) : la vérification continue
    /// d'elle-même, sauf si fprintd la déclare finie.
    Retry,
    /// Erreur du lecteur : abandon.
    Error(String),
}

pub fn classify(result: &str, done: bool) -> Verdict {
    match result {
        "verify-match" => Verdict::Match,
        "verify-no-match" => Verdict::NoMatch,
        "verify-retry-scan"
        | "verify-swipe-too-short"
        | "verify-finger-not-centered"
        | "verify-remove-and-retry"
            if !done =>
        {
            Verdict::Retry
        }
        r if r.starts_with("verify-retry") || r.starts_with("verify-swipe") => Verdict::NoMatch,
        "verify-finger-not-centered" | "verify-remove-and-retry" => Verdict::NoMatch,
        other => Verdict::Error(other.to_string()),
    }
}

async fn device(conn: &zbus::Connection) -> zbus::Result<zbus::zvariant::OwnedObjectPath> {
    let r = conn
        .call_method(
            Some(DEST),
            MANAGER,
            Some(MANAGER_IFACE),
            "GetDefaultDevice",
            &(),
        )
        .await?;
    r.body().deserialize()
}

/// Vérifie l'empreinte de `user` (attend le doigt, sans limite de temps
/// autre que celle de fprintd).
pub async fn verify(user: &str) -> Finger {
    let Ok(conn) = zbus::Connection::system().await else {
        return Finger::Unavailable;
    };
    let Ok(path) = device(&conn).await else {
        return Finger::Unavailable;
    };
    let call = |m: &'static str| {
        let (conn, path) = (conn.clone(), path.clone());
        async move {
            conn.call_method(Some(DEST), path.as_str(), Some(DEVICE_IFACE), m, &())
                .await
                .map(|_| ())
        }
    };
    if conn
        .call_method(
            Some(DEST),
            path.as_str(),
            Some(DEVICE_IFACE),
            "Claim",
            &(user,),
        )
        .await
        .is_err()
    {
        return Finger::Unavailable;
    }
    let outcome = async {
        let proxy = zbus::Proxy::new(&conn, DEST, path.as_str(), DEVICE_IFACE).await?;
        let mut signals = proxy.receive_signal("VerifyStatus").await?;
        let mut tries = 0;
        loop {
            // Pas d'empreinte enregistrée : VerifyStart échoue (NoEnrolledPrints).
            if let Err(e) = conn
                .call_method(
                    Some(DEST),
                    path.as_str(),
                    Some(DEVICE_IFACE),
                    "VerifyStart",
                    &("any",),
                )
                .await
            {
                return Ok::<_, zbus::Error>(if e.to_string().contains("NoEnrolledPrints") {
                    Finger::Unavailable
                } else {
                    Finger::Failed(e.to_string())
                });
            }
            loop {
                let Some(msg) = signals.next().await else {
                    return Ok(Finger::Failed(
                        crate::i18n::tr("lecteur d'empreintes perdu").into(),
                    ));
                };
                let (result, done): (String, bool) = msg.body().deserialize()?;
                match classify(&result, done) {
                    Verdict::Match => return Ok(Finger::Matched),
                    Verdict::Retry => continue,
                    Verdict::NoMatch => break,
                    Verdict::Error(e) => return Ok(Finger::Failed(e)),
                }
            }
            let _ = call("VerifyStop").await;
            tries += 1;
            if tries >= TRIES {
                return Ok(Finger::Failed(
                    crate::i18n::tr("Empreinte non reconnue.").into(),
                ));
            }
        }
    }
    .await
    .unwrap_or_else(|e| Finger::Failed(e.to_string()));
    let _ = call("VerifyStop").await;
    let _ = call("Release").await;
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resultats_de_fprintd() {
        assert_eq!(classify("verify-match", true), Verdict::Match);
        assert_eq!(classify("verify-no-match", true), Verdict::NoMatch);
        assert_eq!(classify("verify-retry-scan", false), Verdict::Retry);
        assert_eq!(classify("verify-swipe-too-short", true), Verdict::NoMatch);
        assert_eq!(
            classify("verify-finger-not-centered", false),
            Verdict::Retry
        );
        assert_eq!(
            classify("verify-disconnected", true),
            Verdict::Error("verify-disconnected".into())
        );
    }
}
