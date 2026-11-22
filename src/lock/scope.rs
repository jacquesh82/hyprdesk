//! Le verrou hors du service de hyprdesk.
//!
//! hypridle, lancé par le démon, lance `hyprdesk lock` : sans précaution, le
//! verrou vit dans le cgroup de `hyprdesk.service`. Un redémarrage du service
//! (mise à jour, `--replace`, relance après une erreur) le tuait alors avec
//! le reste : Hyprland gardait la session verrouillée sans écran pour la
//! déverrouiller (« lockscreen died »), et le verrou suivant était refusé.
//! Au réveil, l'écran de verrouillage semblait avoir planté.
//!
//! Le verrou passe donc dans sa propre unité (`hyprdesk-lock-<pid>.scope`,
//! comme `systemd-run --user --scope`), avant toute chose. Sans systemd
//! utilisateur, il reste où il est : verrouiller passe avant.
//!
//! La scope ne suffit pas : le démon lance hypridle dans son propre groupe de
//! processus et l'arrête par `kill -TERM/-KILL -- -<pgid>`. Le verrou, enfant
//! de hypridle, hérite de ce groupe et recevait ces signaux quel que soit son
//! cgroup. Il quitte donc aussi le groupe (`setsid`, voir
//! [`leave_process_group`]).

use std::time::{Duration, Instant};

/// Quitte le groupe de processus hérité du parent (hypridle) en ouvrant une
/// session à soi : les signaux envoyés au groupe du parent ne l'atteignent
/// plus. Rien à faire si le processus mène déjà son groupe (lancé depuis un
/// shell à contrôle de tâches, ou par `systemd-run`).
pub fn leave_process_group() -> Result<(), String> {
    // SAFETY: getpid/getpgrp/setsid n'ont pas de préconditions.
    unsafe {
        if libc::getpgrp() == libc::getpid() {
            return Ok(());
        }
        if libc::setsid() == -1 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}

/// Le processus courant vit dans un service systemd (et non une scope ou une
/// tranche) : un arrêt de ce service le tuerait.
fn in_service(cgroup: &str) -> bool {
    cgroup
        .lines()
        .find_map(|l| l.strip_prefix("0::"))
        .and_then(|p| p.trim_end().rsplit('/').next())
        .is_some_and(|unit| unit.ends_with(".service"))
}

fn own_cgroup() -> String {
    std::fs::read_to_string("/proc/self/cgroup").unwrap_or_default()
}

/// Déplace le processus dans une scope à lui s'il est dans un service.
/// Rend une explication si c'est impossible (le verrou continue quand même).
pub fn detach_from_service() -> Result<(), String> {
    if !in_service(&own_cgroup()) {
        return Ok(());
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(start_scope())?;
    // Le déplacement suit le travail systemd, presque aussitôt.
    let until = Instant::now() + Duration::from_secs(1);
    while in_service(&own_cgroup()) {
        if Instant::now() >= until {
            return Err("scope créée, processus pas encore déplacé".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

async fn start_scope() -> Result<(), String> {
    use zbus::zvariant::Value;
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| e.to_string())?;
    let pid = std::process::id();
    let name = format!("hyprdesk-lock-{pid}.scope");
    let props: Vec<(&str, Value)> = vec![
        ("PIDs", Value::from(vec![pid])),
        ("Description", Value::from("hyprdesk lock")),
        // Rien ne reste derrière, même après un échec.
        ("CollectMode", Value::from("inactive-or-failed")),
    ];
    let aux: Vec<(&str, Vec<(&str, Value)>)> = Vec::new();
    conn.call_method(
        Some("org.freedesktop.systemd1"),
        "/org/freedesktop/systemd1",
        Some("org.freedesktop.systemd1.Manager"),
        "StartTransientUnit",
        &(name.as_str(), "fail", props, aux),
    )
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_ou_scope() {
        assert!(in_service(
            "0::/user.slice/user-1000.slice/user@1000.service/session.slice/hyprdesk.service\n"
        ));
        assert!(!in_service(
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/hyprdesk-lock-42.scope\n"
        ));
        // Lancé depuis Hyprland (sa session) : rien à faire.
        assert!(!in_service(
            "0::/user.slice/user-1000.slice/session-16.scope\n"
        ));
        // cgroup v1 seul, ou illisible : on ne touche à rien.
        assert!(!in_service("1:name=systemd:/foo.service\n"));
        assert!(!in_service(""));
    }
}
