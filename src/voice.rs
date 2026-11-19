//! LYRA vocale : écoute du mot « Lyra » et réponse parlée, hors ligne.
//!
//! - Écoute : Vosk (petits modèles FR et EN), dans un environnement Python
//!   à part ; script `assets/voice/lyra_voice.py`, micro par `pw-record`.
//! - Voix : Piper (binaire autonome, voix FR siwis et EN lessac), sortie par
//!   `pw-play`.
//!
//! Tout s'installe dans `~/.local/share/hyprdesk/voice` (`hyprdesk voice
//! setup`), sans droits ni paquet système.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SCRIPT: &str = include_str!("../assets/voice/lyra_voice.py");

const PIPER_URL: &str =
    "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz";
const VOICES: &[(&str, &str)] = &[
    ("fr", "fr/fr_FR/siwis/medium/fr_FR-siwis-medium"),
    ("en", "en/en_US/lessac/medium/en_US-lessac-medium"),
];
const VOICE_BASE: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0";
const VOSK: &[(&str, &str)] = &[
    ("fr", "vosk-model-small-fr-0.22"),
    ("en", "vosk-model-small-en-us-0.15"),
];
const VOSK_BASE: &str = "https://alphacephei.com/vosk/models";

/// Dossier de la voix.
pub fn dir() -> PathBuf {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        });
    data.join("hyprdesk/voice")
}

fn python() -> PathBuf {
    dir().join("venv/bin/python")
}
fn piper() -> PathBuf {
    dir().join("piper/piper")
}
fn voice_model(lang: &str) -> PathBuf {
    let name = VOICES
        .iter()
        .find(|(l, _)| *l == lang)
        .or(VOICES.first())
        .map(|(_, p)| p.rsplit('/').next().unwrap_or(p))
        .unwrap_or_default();
    dir().join("voices").join(format!("{name}.onnx"))
}
fn vosk_model(lang: &str) -> PathBuf {
    let name = VOSK
        .iter()
        .find(|(l, _)| *l == lang)
        .map(|(_, n)| *n)
        .unwrap_or_default();
    dir().join("vosk").join(name)
}
fn script() -> PathBuf {
    dir().join("lyra_voice.py")
}

/// Tout est installé (sinon `hyprdesk voice setup`).
pub fn ready() -> bool {
    python().is_file()
        && piper().is_file()
        && VOICES.iter().all(|(l, _)| voice_model(l).is_file())
        && VOSK.iter().all(|(l, _)| vosk_model(l).is_dir())
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let out = cmd
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{:?} : {e}", cmd.get_program()))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{:?} : {}",
            cmd.get_program(),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn download(url: &str, to: &Path) -> Result<(), String> {
    let part = to.with_extension("part");
    run(Command::new("curl")
        .args(["-fsSL", "--retry", "2", "-o"])
        .arg(&part)
        .arg(url))?;
    std::fs::rename(&part, to).map_err(|e| e.to_string())
}

/// Installe ce qui manque (bloquant : ~250 Mo à télécharger la première
/// fois). `progress` reçoit une ligne par étape.
pub fn setup(progress: &mut dyn FnMut(&str)) -> Result<(), String> {
    let d = dir();
    std::fs::create_dir_all(d.join("voices")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(d.join("vosk")).map_err(|e| e.to_string())?;
    std::fs::write(script(), SCRIPT).map_err(|e| e.to_string())?;
    if !piper().is_file() {
        progress("Piper (synthèse vocale)…");
        let tgz = d.join("piper.tar.gz");
        download(PIPER_URL, &tgz)?;
        run(Command::new("tar").arg("-xzf").arg(&tgz).arg("-C").arg(&d))?;
        let _ = std::fs::remove_file(&tgz);
    }
    for (lang, path) in VOICES {
        let model = voice_model(lang);
        if !model.is_file() {
            progress(&format!("Voix {lang}…"));
            download(&format!("{VOICE_BASE}/{path}.onnx.json"), &model.with_extension("onnx.json"))?;
            download(&format!("{VOICE_BASE}/{path}.onnx"), &model)?;
        }
    }
    if !python().is_file() {
        progress("Environnement Python…");
        run(Command::new("python3").args(["-m", "venv"]).arg(d.join("venv")))?;
    }
    let has_vosk = Command::new(python())
        .args(["-c", "import vosk"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !has_vosk {
        progress("Vosk (reconnaissance vocale)…");
        run(Command::new(python()).args(["-m", "pip", "install", "--quiet", "vosk"]))?;
    }
    for (lang, name) in VOSK {
        if !vosk_model(lang).is_dir() {
            progress(&format!("Modèle d'écoute {lang}…"));
            let zip = d.join("vosk").join(format!("{name}.zip"));
            download(&format!("{VOSK_BASE}/{name}.zip"), &zip)?;
            run(Command::new(python())
                .args(["-m", "zipfile", "-e"])
                .arg(&zip)
                .arg(d.join("vosk")))?;
            let _ = std::fs::remove_file(&zip);
        }
    }
    Ok(())
}

/// Langue d'une réponse : celle de la demande, sinon le français.
pub fn lang_of(lang: &str) -> &'static str {
    if lang == "en" { "en" } else { "fr" }
}

/// Texte à dire : sans la mise en forme Markdown ni les blocs de code.
pub fn speakable(md: &str) -> String {
    let mut out = String::new();
    let mut code = false;
    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            code = !code;
            continue;
        }
        if code {
            continue;
        }
        let l = line
            .trim()
            .trim_start_matches(['#', '>', '-', '*', ' '])
            .replace(['*', '`', '_'], "");
        if !l.is_empty() {
            out.push_str(&l);
            out.push(' ');
        }
    }
    out.trim().to_string()
}

/// Dit `text` (bloquant jusqu'à la fin) : Piper, puis `pw-play`.
pub fn speak(text: &str, lang: &str) -> Result<(), String> {
    let text = speakable(text);
    if text.is_empty() {
        return Ok(());
    }
    let model = voice_model(lang_of(lang));
    let rate = std::fs::read_to_string(model.with_extension("onnx.json"))
        .ok()
        .and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
        .and_then(|v| v["audio"]["sample_rate"].as_u64())
        .unwrap_or(22050);
    let mut synth = Command::new(piper())
        .arg("--model")
        .arg(&model)
        .arg("--output-raw")
        .current_dir(dir().join("piper"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("piper : {e}"))?;
    let raw = synth.stdout.take().ok_or("piper : pas de sortie")?;
    let mut play = Command::new("pw-play")
        .args([
            "--raw",
            "--rate",
            &rate.to_string(),
            "--channels",
            "1",
            "--format",
            "s16",
            "-",
        ])
        .stdin(raw)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("pw-play : {e}"))?;
    {
        use std::io::Write;
        let mut input = synth.stdin.take().ok_or("piper : pas d'entrée")?;
        input
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let _ = synth.wait();
    let _ = play.wait();
    Ok(())
}

/// Événement de l'écoute.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Ready,
    /// « Lyra » entendu : la demande suit.
    Wake,
    /// Demande, et sa langue.
    Heard(String, String),
    /// Phrase entendue (diagnostic, affichée au survol du micro).
    Partial(String),
    Error(String),
}

/// Ligne JSON du script d'écoute.
pub fn parse(line: &str) -> Option<Event> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    Some(match v["event"].as_str()? {
        "ready" => Event::Ready,
        "wake" => Event::Wake,
        "text" => Event::Heard(
            v["lang"].as_str().unwrap_or("fr").to_string(),
            v["text"].as_str()?.to_string(),
        ),
        "heard" => Event::Partial(v["text"].as_str().unwrap_or("").to_string()),
        "error" => Event::Error(v["message"].as_str().unwrap_or("").to_string()),
        _ => return None,
    })
}

/// Commande d'écoute (programme, arguments) ; `mic` : nœud PipeWire du
/// micro (vide : celui par défaut).
pub fn listen_command(mic: &str) -> (PathBuf, Vec<PathBuf>) {
    let mut args = vec![script(), vosk_model("fr"), vosk_model("en")];
    if !mic.is_empty() {
        args.push(PathBuf::from(mic));
    }
    (python(), args)
}

/// Micros de PipeWire : (nom du nœud, libellé, par défaut). Bloquant.
pub fn microphones() -> Vec<(String, String, bool)> {
    let wpctl = |args: &[&str]| {
        Command::new("wpctl")
            .args(args)
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default()
    };
    let (_, inputs) = crate::ui::bar::parse_wpctl_status(&wpctl(&["status"]));
    inputs
        .into_iter()
        .filter_map(|d| {
            let info = wpctl(&["inspect", &d.id.to_string()]);
            let name = info.lines().find_map(|l| {
                let v = l.trim().trim_start_matches('*').trim();
                v.strip_prefix("node.name = ")
                    .map(|n| n.trim_matches('"').to_string())
            })?;
            Some((name, d.name, d.default))
        })
        .collect()
}

/// Entrée du script d'écoute en cours (pause, reprise).
static INPUT: std::sync::Mutex<Option<std::process::ChildStdin>> = std::sync::Mutex::new(None);

/// Suspend (`pause`) ou reprend (`resume`) l'écoute : LYRA ne s'entend pas
/// parler.
pub fn control(cmd: &str) {
    use std::io::Write;
    if let Ok(mut i) = INPUT.lock()
        && let Some(input) = i.as_mut()
    {
        let _ = writeln!(input, "{cmd}");
        let _ = input.flush();
    }
}

/// Processus d'écoute arrêté avec le flux (LYRA vocale coupée).
struct Guard(std::process::Child);

impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(mut i) = INPUT.lock() {
            *i = None;
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Écoute en continu : événements du script (souscription de l'interface).
pub fn stream(mic: String) -> impl iced::futures::Stream<Item = Event> {
    use iced::futures::SinkExt;
    iced::stream::channel(8, async move |mut tx| {
        install_script();
        let (prog, args) = listen_command(&mic);
        let child = Command::new(prog)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Event::Error(format!("écoute : {e}"))).await;
                return std::future::pending().await;
            }
        };
        if let Ok(mut i) = INPUT.lock() {
            *i = child.stdin.take();
        }
        let out = child.stdout.take();
        let guard = Guard(child);
        let (ltx, mut lrx) = tokio::sync::mpsc::unbounded_channel::<String>();
        if let Some(out) = out {
            std::thread::spawn(move || {
                use std::io::BufRead;
                for line in std::io::BufReader::new(out).lines() {
                    let Ok(line) = line else { break };
                    if ltx.send(line).is_err() {
                        break;
                    }
                }
            });
        }
        while let Some(line) = lrx.recv().await {
            if let Some(e) = parse(&line) {
                let _ = tx.send(e).await;
            }
        }
        drop(guard);
        let _ = tx
            .send(Event::Error(crate::i18n::tr("l'écoute s'est arrêtée").into()))
            .await;
        std::future::pending().await
    })
}

/// Écrit le script d'écoute (mis à jour avec hyprdesk).
pub fn install_script() {
    let _ = std::fs::create_dir_all(dir());
    let _ = std::fs::write(script(), SCRIPT);
}

/// `hyprdesk voice setup | say <texte> | status`.
pub fn cli(args: &[String]) -> std::process::ExitCode {
    use std::process::ExitCode;
    match args.first().map(String::as_str) {
        Some("setup") => match setup(&mut |s| println!("{s}")) {
            Ok(()) => {
                println!("{}", crate::i18n::tr("LYRA vocale prête."));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        Some("say") => {
            let (lang, text) = match args.get(1).map(String::as_str) {
                Some("--en") => ("en", args[2..].join(" ")),
                _ => ("fr", args[1..].join(" ")),
            };
            match speak(&text, lang) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            println!(
                "{}",
                if ready() {
                    crate::i18n::tr("LYRA vocale : installée")
                } else {
                    crate::i18n::tr("LYRA vocale : à installer (hyprdesk voice setup)")
                }
            );
            ExitCode::SUCCESS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evenements_et_texte_parle() {
        assert_eq!(parse(r#"{"event":"wake"}"#), Some(Event::Wake));
        assert_eq!(
            parse(r#"{"event":"text","lang":"en","text":"play jazz"}"#),
            Some(Event::Heard("en".into(), "play jazz".into()))
        );
        assert_eq!(parse("x"), None);
        assert_eq!(
            speakable("## Titre\n**FIP** est lancée.\n```sh\nhyprdesk x\n```\n- ok"),
            "Titre FIP est lancée. ok"
        );
    }
}
