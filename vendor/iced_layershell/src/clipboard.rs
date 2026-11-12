use std::sync::atomic::{AtomicBool, Ordering};

use iced_core::Clipboard;
use iced_core::clipboard::Kind;
use layershellev::WindowWrapper;

static DISABLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_disabled() {
    DISABLED.store(true, Ordering::Relaxed);
}

pub(crate) fn is_disabled() -> bool {
    DISABLED.load(Ordering::Relaxed)
}

pub struct LayerShellClipboard {
    state: State,
}

enum State {
    Connected(window_clipboard::Clipboard),
    Unavailable,
}

impl LayerShellClipboard {
    /// Creates a new [`Clipboard`] for the given window.
    pub fn connect(window: &WindowWrapper) -> Self {
        #[allow(unsafe_code)]
        let state = unsafe { window_clipboard::Clipboard::connect(window) }
            .ok()
            .map(State::Connected)
            .unwrap_or(State::Unavailable);

        Self { state }
    }

    /// Creates a new [`Clipboard`] that isn't associated with a window.
    /// This clipboard will never contain a copied value.
    #[allow(unused)]
    pub fn unconnected() -> Self {
        Self {
            state: State::Unavailable,
        }
    }

    /// Reads the current content of the [`Clipboard`] as text.
    ///
    /// hyprdesk : `wl-paste` d'abord. Le « data device » ajouté à layershellev
    /// pour le glisser-déposer reçoit la sélection à la place de celui de
    /// smithay-clipboard (Hyprland n'en sert qu'un par client) : sans ce
    /// détour, Ctrl+V ne colle rien dans les champs des surfaces.
    pub fn read(&self, kind: Kind) -> Option<String> {
        if is_disabled() {
            return None;
        }
        if let Some(text) = wl_paste(kind) {
            return Some(text);
        }
        match &self.state {
            State::Connected(clipboard) => match kind {
                Kind::Standard => clipboard.read().ok(),
                Kind::Primary => clipboard.read_primary().and_then(Result::ok),
            },
            State::Unavailable => None,
        }
    }

    /// Writes the given text contents to the [`Clipboard`].
    pub fn write(&mut self, kind: Kind, contents: String) {
        match &mut self.state {
            State::Connected(clipboard) => {
                let result = match kind {
                    Kind::Standard => clipboard.write(contents),
                    Kind::Primary => clipboard.write_primary(contents).unwrap_or(Ok(())),
                };

                match result {
                    Ok(()) => {}
                    Err(error) => {
                        log::warn!("error writing to clipboard: {error}");
                    }
                }
            }
            State::Unavailable => {}
        }
    }
}

/// Texte de la sélection lu par `wl-paste` (wl-clipboard), en une seconde au
/// plus : la boucle d'événements attend la réponse.
fn wl_paste(kind: Kind) -> Option<String> {
    use std::io::Read as _;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let mut cmd = Command::new("wl-paste");
    cmd.args(["--no-newline", "--type", "text"]);
    if kind == Kind::Primary {
        cmd.arg("--primary");
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Lecture à part : une grande sélection remplirait le tuyau.
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < Duration::from_secs(1) => {
                std::thread::sleep(Duration::from_millis(5));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let bytes = reader.join().ok()?;
    if !status.success() || bytes.is_empty() {
        return None;
    }
    String::from_utf8(bytes).ok()
}

impl Clipboard for LayerShellClipboard {
    fn read(&self, kind: Kind) -> Option<String> {
        self.read(kind)
    }

    fn write(&mut self, kind: Kind, contents: String) {
        self.write(kind, contents);
    }
}
