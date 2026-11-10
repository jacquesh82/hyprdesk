//! Socket de commande : `hyprdesk toggle` (le client) parle au démon par
//! `$XDG_RUNTIME_DIR/hypr/<instance>/hyprdesk.sock`, une ligne par requête.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use iced::futures::{SinkExt, Stream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Toggle,
    Show,
    Hide,
    Reload,
    Status,
    Quit,
    /// Affiche le bureau et ouvre l'écran Réglages sur un onglet
    /// (0 bureau, 1 icônes, 2 fond d'écran, 3 effets, 4 widgets).
    Settings(u8),
    /// Ouvre le menu d'un module de la barre (type : volume, clock, reseau, tray…).
    Menu(String),
}

impl Command {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "toggle" => Command::Toggle,
            "show" => Command::Show,
            "hide" => Command::Hide,
            "reload" => Command::Reload,
            "status" => Command::Status,
            "quit" => Command::Quit,
            "settings" | "settings bureau" => Command::Settings(0),
            "settings icones" | "settings icônes" => Command::Settings(1),
            "settings fond" => Command::Settings(2),
            "settings effets" => Command::Settings(3),
            "settings widgets" => Command::Settings(4),
            m if m.starts_with("menu ") && m.len() > 5 => Command::Menu(m[5..].trim().to_string()),
            _ => return None,
        })
    }

    pub fn to_line(&self) -> String {
        match self {
            Command::Menu(m) => format!("menu {m}"),
            c => c.as_str().to_string(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Command::Toggle => "toggle",
            Command::Show => "show",
            Command::Hide => "hide",
            Command::Reload => "reload",
            Command::Status => "status",
            Command::Quit => "quit",
            Command::Settings(1) => "settings icones",
            Command::Settings(2) => "settings fond",
            Command::Settings(3) => "settings effets",
            Command::Settings(4) => "settings widgets",
            Command::Settings(_) => "settings",
            Command::Menu(_) => "menu",
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
    let mut s = StdUnixStream::connect(&path)
        .map_err(|e| anyhow::anyhow!("démon hyprdesk injoignable ({}) : {e} — est-il lancé ?", path.display()))?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.write_all(format!("{}\n", cmd.to_line()).as_bytes())?;
    s.shutdown(std::net::Shutdown::Write)?;
    let mut out = String::new();
    s.read_to_string(&mut out)?;
    Ok(out.trim().to_string())
}

/// Serveur : flux des commandes reçues. `status` est servi sans passer par l'UI.
pub fn server_stream() -> impl Stream<Item = Command> {
    iced::stream::channel(16, async move |mut out: iced::futures::channel::mpsc::Sender<Command>| {
        let path = match socket_path() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("socket de commande : {e}");
                return;
            }
        };
        // Une socket orpheline (démon précédent tué) empêche le bind.
        let _ = std::fs::remove_file(&path);
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("écoute sur {} : {e}", path.display());
                return;
            }
        };
        tracing::info!("commandes sur {}", path.display());
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            let (r, mut w) = stream.into_split();
            let mut line = String::new();
            if BufReader::new(r).read_line(&mut line).await.is_err() {
                continue;
            }
            let reply = match Command::parse(&line) {
                Some(Command::Status) => STATUS.lock().map(|s| s.clone()).unwrap_or_default(),
                Some(c) => {
                    let _ = out.send(c).await;
                    "ok".to_string()
                }
                None => format!("commande inconnue : {}", line.trim()),
            };
            let _ = w.write_all(format!("{reply}\n").as_bytes()).await;
        }
    })
}

/// SIGTERM (systemctl stop) ou SIGINT (Ctrl-C) : quitter proprement, comme
/// `hyprdesk quit` (fenêtres rendues, raccourcis retirés).
pub fn signal_stream() -> impl Stream<Item = Command> {
    iced::stream::channel(1, async move |mut out: iced::futures::channel::mpsc::Sender<Command>| {
        use tokio::signal::unix::{SignalKind, signal};
        let (Ok(mut term), Ok(mut int)) = (signal(SignalKind::terminate()), signal(SignalKind::interrupt())) else {
            tracing::warn!("signaux indisponibles : arrêt propre sur SIGTERM désactivé");
            return;
        };
        tokio::select! {
            _ = term.recv() => tracing::info!("SIGTERM : arrêt propre"),
            _ = int.recv() => tracing::info!("SIGINT : arrêt propre"),
        }
        let _ = out.send(Command::Quit).await;
        // Le démon quitte ; ne plus rien émettre.
        std::future::pending::<()>().await;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_des_commandes() {
        for c in [Command::Toggle, Command::Show, Command::Hide, Command::Reload, Command::Status, Command::Quit, Command::Settings(0), Command::Settings(2)] {
            assert_eq!(Command::parse(&format!("{}\n", c.to_line())), Some(c.clone()));
        }
        assert_eq!(Command::parse("nope"), None);
        assert_eq!(Command::parse("menu reseau"), Some(Command::Menu("reseau".into())));
    }
}
