//! Socket de commande : `hyprdesk toggle` (le client) parle au démon par
//! `$XDG_RUNTIME_DIR/hypr/<instance>/hyprdesk.sock`, une ligne par requête.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use iced::futures::{SinkExt, Stream};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Toggle,
    Show,
    Hide,
    /// `launcher-only` : fenêtres laissées en place, le lanceur (dock)
    /// accessible sous elles ; les fenêtres en mosaïque lui laissent la place.
    LauncherOnly,
    /// launcher-only, ou no-desktop s'il y est déjà (raccourci `keys.launcher`).
    ToggleLauncher,
    /// Mode suivant : no-desktop → launcher-only → full-desktop → no-desktop.
    Cycle,
    Reload,
    Status,
    Quit,
    /// Ouvre l'écran Réglages sur un paquet (`desktop`, `screens`, `fond`…),
    /// sans changer le mode du bureau.
    Settings(String),
    /// Ouvre le menu d'un module de la barre (type : volume, clock, reseau, tray…).
    Menu(String),
    /// Ouvre ou referme l'assistant (tiroir LYRA, ou sa fenêtre).
    Assistant,
    /// Ouvre les réglages sur la visite guidée.
    Tour,
    /// Ouvre ou referme le lanceur avec recherche.
    Search,
    /// Ouvre ou referme la grille des applications (plein écran).
    Apps,
    /// Vue d'ensemble des fenêtres (plugin de Hyprland, sinon le sélecteur).
    Overview,
    /// Notifications de hyprdesk : `dnd [on|off]`, `clear`.
    Notifications(String),
    /// Sélecteur de fenêtres : ouvre, ou fenêtre suivante (`true` : précédente).
    Switch(bool),
    /// Affichage à l'écran après une touche (`kind value muted`), envoyé par
    /// `hyprdesk osd` une fois l'action faite.
    Osd(String),
    /// Un problème à montrer (texte libre, sur plusieurs lignes) : soumis à
    /// l'assistant LYRA s'il est là (une conversation à lui, signalée par
    /// une notification), sinon une fenêtre de détail copiable.
    Problem(String),
    /// Le même, demandé explicitement (« Voir le problème… ») : la
    /// conversation s'affiche aussitôt.
    ProblemShow(String),
}

impl Command {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "toggle" => Command::Toggle,
            "show" | "full-desktop" => Command::Show,
            "hide" | "no-desktop" => Command::Hide,
            "launcher-only" | "controls" => Command::LauncherOnly,
            "cycle" => Command::Cycle,
            "toggle-launcher" => Command::ToggleLauncher,
            "reload" => Command::Reload,
            "status" => Command::Status,
            "quit" => Command::Quit,
            "assistant" => Command::Assistant,
            "tour" => Command::Tour,
            "search" => Command::Search,
            "apps" => Command::Apps,
            "overview" => Command::Overview,
            "switch" | "switch next" => Command::Switch(false),
            "switch prev" | "switch-prev" => Command::Switch(true),
            "settings" => Command::Settings("desktop".into()),
            m if m.starts_with("settings ") => Command::Settings(m[9..].trim().to_string()),
            m if m.starts_with("menu ") && m.len() > 5 => Command::Menu(m[5..].trim().to_string()),
            m if m.starts_with("osd-show ") => Command::Osd(m[9..].trim().to_string()),
            // Une requête tient sur une ligne : le texte voyage en chaîne JSON.
            m if m.starts_with("problem ") => Command::Problem(serde_json::from_str(&m[8..]).ok()?),
            m if m.starts_with("notifications ") => {
                Command::Notifications(m[14..].trim().to_string())
            }
            m if m.starts_with("problem-show ") => {
                Command::ProblemShow(serde_json::from_str(&m[13..]).ok()?)
            }
            _ => return None,
        })
    }

    pub fn to_line(&self) -> String {
        match self {
            Command::Menu(m) => format!("menu {m}"),
            Command::Settings(p) => format!("settings {p}"),
            Command::Osd(l) => format!("osd-show {l}"),
            Command::Problem(t) => format!(
                "problem {}",
                serde_json::to_string(t).unwrap_or_else(|_| "\"\"".into())
            ),
            Command::Notifications(a) => format!("notifications {a}"),
            Command::ProblemShow(t) => format!(
                "problem-show {}",
                serde_json::to_string(t).unwrap_or_else(|_| "\"\"".into())
            ),
            c => c.as_str().to_string(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Command::Toggle => "toggle",
            Command::Show => "show",
            Command::Hide => "hide",
            Command::LauncherOnly => "launcher-only",
            Command::Cycle => "cycle",
            Command::ToggleLauncher => "toggle-launcher",
            Command::Reload => "reload",
            Command::Status => "status",
            Command::Quit => "quit",
            Command::Settings(_) => "settings",
            Command::Menu(_) => "menu",
            Command::Assistant => "assistant",
            Command::Tour => "tour",
            Command::Search => "search",
            Command::Apps => "apps",
            Command::Overview => "overview",
            Command::Notifications(_) => "notifications",
            Command::Switch(false) => "switch",
            Command::Switch(true) => "switch prev",
            Command::Osd(_) => "osd-show",
            Command::Problem(_) => "problem",
            Command::ProblemShow(_) => "problem-show",
        }
    }
}

/// Résumé lisible de l'état du démon, tenu à jour par l'UI pour `status`.
pub static STATUS: Mutex<String> = Mutex::new(String::new());

pub fn set_status(s: String) {
    if let Ok(mut g) = STATUS.lock() {
        *g = s;
    }
}

pub fn socket_path() -> anyhow::Result<PathBuf> {
    Ok(hyprpanel_core::ipc::instance_dir()?.join("hyprdesk.sock"))
}

/// Client : envoie une commande et rend la réponse du démon.
pub fn send(cmd: Command) -> anyhow::Result<String> {
    let path = socket_path()?;
    let mut s = StdUnixStream::connect(&path).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            crate::tr!(
                "démon hyprdesk injoignable ({path}) : {e} — est-il lancé ?",
                path = path.display(),
                e = e
            )
        )
    })?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.set_write_timeout(Some(Duration::from_secs(5)))?;
    s.write_all(format!("{}\n", cmd.to_line()).as_bytes())?;
    s.shutdown(std::net::Shutdown::Write)?;
    let mut out = String::new();
    s.read_to_string(&mut out)?;
    Ok(out.trim().to_string())
}

/// Serveur : flux des commandes reçues. `status` est servi sans passer par l'UI.
pub fn server_stream() -> impl Stream<Item = Command> {
    iced::stream::channel(
        16,
        async move |out: iced::futures::channel::mpsc::Sender<Command>| {
            let path = match socket_path() {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("{}", crate::tr!("socket de commande : {e}", e = e));
                    return;
                }
            };
            // Une socket orpheline (démon précédent tué) empêche le bind.
            let _ = std::fs::remove_file(&path);
            let listener = match UnixListener::bind(&path) {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!(
                        "{}",
                        crate::tr!("écoute sur {p0} : {e}", p0 = path.display(), e = e)
                    );
                    return;
                }
            };
            tracing::info!("{}", crate::tr!("commandes sur {p0}", p0 = path.display()));
            let connections = std::sync::Arc::new(tokio::sync::Semaphore::new(32));
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                let Ok(permit) = connections.clone().try_acquire_owned() else {
                    continue;
                };
                let mut out = out.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    let (r, mut w) = stream.into_split();
                    let mut line = String::new();
                    let mut reader = BufReader::new(r.take(1025));
                    let read =
                        tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut line))
                            .await;
                    if !matches!(read, Ok(Ok(1..))) || line.len() > 1024 || !line.ends_with('\n') {
                        return;
                    }
                    let reply = match Command::parse(&line) {
                        Some(Command::Status) => {
                            STATUS.lock().map(|s| s.clone()).unwrap_or_default()
                        }
                        Some(c) => {
                            match tokio::time::timeout(Duration::from_secs(2), out.send(c)).await {
                                Ok(Ok(())) => "ok".to_string(),
                                _ => crate::i18n::tr("démon occupé").to_string(),
                            }
                        }
                        None => crate::tr!("commande inconnue : {p0}", p0 = line.trim()),
                    };
                    let _ = tokio::time::timeout(
                        Duration::from_secs(2),
                        w.write_all(format!("{reply}\n").as_bytes()),
                    )
                    .await;
                });
            }
        },
    )
}

/// SIGTERM (systemctl stop) ou SIGINT (Ctrl-C) : quitter proprement, comme
/// `hyprdesk quit` (fenêtres rendues, raccourcis retirés).
pub fn signal_stream() -> impl Stream<Item = Command> {
    iced::stream::channel(
        1,
        async move |mut out: iced::futures::channel::mpsc::Sender<Command>| {
            use tokio::signal::unix::{SignalKind, signal};
            let (Ok(mut term), Ok(mut int)) = (
                signal(SignalKind::terminate()),
                signal(SignalKind::interrupt()),
            ) else {
                tracing::warn!(
                    "{}",
                    crate::tr!("signaux indisponibles : arrêt propre sur SIGTERM désactivé")
                );
                return;
            };
            tokio::select! {
                _ = term.recv() => tracing::info!("{}", crate::tr!("SIGTERM : arrêt propre")),
                _ = int.recv() => tracing::info!("{}", crate::tr!("SIGINT : arrêt propre")),
            }
            let _ = out.send(Command::Quit).await;
            // Le démon quitte ; ne plus rien émettre.
            std::future::pending::<()>().await;
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_des_commandes() {
        for c in [
            Command::Toggle,
            Command::Show,
            Command::Hide,
            Command::Reload,
            Command::Status,
            Command::Quit,
            Command::Settings("desktop".into()),
            Command::Settings("fond".into()),
            Command::LauncherOnly,
            Command::ToggleLauncher,
            Command::Cycle,
        ] {
            assert_eq!(
                Command::parse(&format!("{}\n", c.to_line())),
                Some(c.clone())
            );
        }
        assert_eq!(Command::parse("nope"), None);
        assert_eq!(
            Command::parse("menu reseau"),
            Some(Command::Menu("reseau".into()))
        );
    }
}
