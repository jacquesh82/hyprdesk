//! Enregistrement des empreintes (Réglages → Verrouillage) : fprintd par
//! D-Bus, comme `fprintd-enroll`, `fprintd-list` et `fprintd-delete`.
//!
//! Lister demande le droit de vérifier (accordé à la session active) ;
//! enregistrer et supprimer demandent `net.reactivated.fprint.device.enroll`,
//! que polkit accorde contre le mot de passe de l'utilisateur : c'est l'agent
//! polkit de la session qui le demande, pas hyprdesk.
//!
//! Déroulé d'un enregistrement : Device.Claim(utilisateur),
//! EnrollStart(doigt), puis les signaux EnrollStatus(résultat, fini) —
//! une étape par passage du doigt, jusqu'à « enroll-completed » ;
//! EnrollStop et Release dans tous les cas, annulation comprise.

use futures::{SinkExt, Stream, StreamExt};

use super::fprint::{DEST, DEVICE_IFACE, device};

/// Doigts de fprintd, dans l'ordre proposé.
pub const FINGERS: [&str; 10] = [
    "right-index-finger",
    "left-index-finger",
    "right-thumb",
    "left-thumb",
    "right-middle-finger",
    "left-middle-finger",
    "right-ring-finger",
    "left-ring-finger",
    "right-little-finger",
    "left-little-finger",
];

/// Nom d'un doigt de fprintd, dans la langue de l'interface.
pub fn finger_label(name: &str) -> String {
    use crate::i18n::tr;
    match name {
        "right-index-finger" => tr("Index droit"),
        "left-index-finger" => tr("Index gauche"),
        "right-thumb" => tr("Pouce droit"),
        "left-thumb" => tr("Pouce gauche"),
        "right-middle-finger" => tr("Majeur droit"),
        "left-middle-finger" => tr("Majeur gauche"),
        "right-ring-finger" => tr("Annulaire droit"),
        "left-ring-finger" => tr("Annulaire gauche"),
        "right-little-finger" => tr("Auriculaire droit"),
        "left-little-finger" => tr("Auriculaire gauche"),
        other => other,
    }
    .to_string()
}

/// Lecteur d'empreintes et doigts enregistrés de l'utilisateur.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reader {
    pub name: String,
    pub fingers: Vec<String>,
}

/// Avancement d'un enregistrement.
#[derive(Debug, Clone, PartialEq)]
pub enum Progress {
    /// Lecteur pris, le doigt est attendu : nombre de passages à faire
    /// (0 : inconnu).
    Started(u32),
    /// Un passage de plus est réussi.
    Stage,
    /// Passage à refaire : le conseil à montrer.
    Retry(String),
    Done,
    Cancelled,
    Failed(String),
}

/// Suite à donner à un signal EnrollStatus.
pub fn classify(result: &str, done: bool) -> Progress {
    use crate::i18n::tr;
    match result {
        "enroll-completed" => Progress::Done,
        "enroll-stage-passed" => Progress::Stage,
        "enroll-retry-scan"
        | "enroll-swipe-too-short"
        | "enroll-finger-not-centered"
        | "enroll-remove-and-retry"
            if !done =>
        {
            Progress::Retry(
                match result {
                    "enroll-swipe-too-short" => tr("Passage trop court : recommencez."),
                    "enroll-finger-not-centered" => tr("Doigt mal centré : recommencez."),
                    "enroll-remove-and-retry" => tr("Retirez le doigt, puis reposez-le."),
                    _ => tr("Lecture ratée : reposez le doigt."),
                }
                .to_string(),
            )
        }
        "enroll-duplicate" => Progress::Failed(tr("Ce doigt est déjà enregistré.").to_string()),
        "enroll-data-full" => Progress::Failed(
            tr("Le lecteur est plein : supprimez une empreinte d'abord.").to_string(),
        ),
        "enroll-disconnected" => Progress::Failed(tr("lecteur d'empreintes perdu").to_string()),
        other => Progress::Failed(crate::tr!("Enregistrement échoué ({r}).", r = other)),
    }
}

/// Message d'une erreur de fprintd.
fn reason(e: &zbus::Error) -> String {
    use crate::i18n::tr;
    let s = e.to_string();
    if s.contains("PermissionDenied") {
        tr("Autorisation refusée (mot de passe demandé par polkit).").to_string()
    } else if s.contains("AlreadyInUse") {
        tr("Le lecteur d'empreintes est déjà utilisé par une autre application.").to_string()
    } else {
        s
    }
}

/// Lecteur par défaut et doigts enregistrés de `user` ; `None` sans fprintd
/// ni lecteur.
pub async fn list(user: &str) -> Option<Reader> {
    let conn = zbus::Connection::system().await.ok()?;
    let path = device(&conn).await.ok()?;
    let proxy = zbus::Proxy::new(&conn, DEST, path.as_str(), DEVICE_IFACE)
        .await
        .ok()?;
    let name = proxy
        .get_property::<String>("name")
        .await
        .unwrap_or_default();
    // Aucune empreinte : l'appel échoue (NoEnrolledPrints).
    let fingers = match proxy.call_method("ListEnrolledFingers", &(user,)).await {
        Ok(r) => r.body().deserialize::<Vec<String>>().unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    Some(Reader { name, fingers })
}

/// Supprime l'empreinte `finger` de `user`.
pub async fn delete(user: &str, finger: &str) -> Result<(), String> {
    let conn = zbus::Connection::system()
        .await
        .map_err(|e| e.to_string())?;
    let path = device(&conn).await.map_err(|e| reason(&e))?;
    let proxy = zbus::Proxy::new(&conn, DEST, path.as_str(), DEVICE_IFACE)
        .await
        .map_err(|e| reason(&e))?;
    proxy
        .call_method("Claim", &(user,))
        .await
        .map_err(|e| reason(&e))?;
    let r = proxy
        .call_method("DeleteEnrolledFinger", &(finger,))
        .await
        .map(|_| ())
        .map_err(|e| reason(&e));
    let _ = proxy.call_method("Release", &()).await;
    r
}

/// Enregistre le doigt `finger` de `user` : flux de l'avancement, clos par
/// `Done`, `Cancelled` ou `Failed`. `cancel` résolu (ou lâché, tiroir
/// refermé) : l'enregistrement s'arrête et le lecteur est rendu.
pub fn enroll(
    user: String,
    finger: String,
    cancel: tokio::sync::oneshot::Receiver<()>,
) -> impl Stream<Item = Progress> {
    // Comme `iced::stream::channel` : les envois de `run`, puis la fin.
    let (mut out, receiver) = futures::channel::mpsc::channel(8);
    let runner = futures::stream::once(async move {
        let end = run(&user, &finger, cancel, &mut out).await;
        let _ = out.send(end).await;
    })
    .filter_map(|()| async { None });
    futures::stream::select(receiver, runner)
}

async fn run(
    user: &str,
    finger: &str,
    mut cancel: tokio::sync::oneshot::Receiver<()>,
    out: &mut futures::channel::mpsc::Sender<Progress>,
) -> Progress {
    let unavailable =
        || Progress::Failed(crate::i18n::tr("Aucun lecteur d'empreintes détecté.").to_string());
    let Ok(conn) = zbus::Connection::system().await else {
        return unavailable();
    };
    let Ok(path) = device(&conn).await else {
        return unavailable();
    };
    let proxy = match zbus::Proxy::new(&conn, DEST, path.as_str(), DEVICE_IFACE).await {
        Ok(p) => p,
        Err(e) => return Progress::Failed(reason(&e)),
    };
    if let Err(e) = proxy.call_method("Claim", &(user,)).await {
        return Progress::Failed(reason(&e));
    }
    let end = async {
        let mut signals = proxy.receive_signal("EnrollStatus").await?;
        // polkit demande ici le mot de passe : l'appel attend la réponse.
        let args = (finger,);
        tokio::select! {
            r = proxy.call_method("EnrollStart", &args) => { r?; }
            _ = &mut cancel => return Ok(Progress::Cancelled),
        }
        // Lu sans cache : la valeur n'est juste qu'une fois le lecteur pris.
        let stages = conn
            .call_method(
                Some(DEST),
                path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(DEVICE_IFACE, "num-enroll-stages"),
            )
            .await
            .ok()
            .and_then(|r| {
                r.body()
                    .deserialize::<zbus::zvariant::OwnedValue>()
                    .ok()
                    .and_then(|v| i32::try_from(v).ok())
            })
            .unwrap_or(0);
        let _ = out.send(Progress::Started(stages.max(0) as u32)).await;
        loop {
            let msg = tokio::select! {
                m = signals.next() => m,
                _ = &mut cancel => return Ok(Progress::Cancelled),
            };
            let Some(msg) = msg else {
                return Ok(Progress::Failed(
                    crate::i18n::tr("lecteur d'empreintes perdu").into(),
                ));
            };
            let (result, done): (String, bool) = msg.body().deserialize()?;
            match classify(&result, done) {
                p @ (Progress::Stage | Progress::Retry(_)) => {
                    let _ = out.send(p).await;
                }
                p => return Ok::<_, zbus::Error>(p),
            }
        }
    }
    .await
    .unwrap_or_else(|e| Progress::Failed(reason(&e)));
    let _ = proxy.call_method("EnrollStop", &()).await;
    let _ = proxy.call_method("Release", &()).await;
    if end == Progress::Done {
        super::pam::journal(&format!(
            "empreinte enregistrée pour {user} ({finger}) depuis les Réglages"
        ));
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resultats_de_l_enregistrement() {
        assert_eq!(classify("enroll-completed", true), Progress::Done);
        assert_eq!(classify("enroll-stage-passed", false), Progress::Stage);
        assert!(matches!(
            classify("enroll-retry-scan", false),
            Progress::Retry(_)
        ));
        // Passage raté déclaré fini par fprintd : échec, pas d'attente sans fin.
        assert!(matches!(
            classify("enroll-retry-scan", true),
            Progress::Failed(_)
        ));
        assert!(matches!(
            classify("enroll-duplicate", true),
            Progress::Failed(_)
        ));
        assert!(matches!(
            classify("enroll-failed", true),
            Progress::Failed(_)
        ));
    }

    #[test]
    fn doigts_nommes() {
        for f in FINGERS {
            assert_ne!(finger_label(f), f, "{f}");
        }
        assert_eq!(finger_label("autre"), "autre");
    }
}
