//! Lecture vidéo par ffmpeg, pour les fonds animés (bureau, écran de
//! verrouillage, écran de connexion) : un processus ffmpeg par fond, qui
//! décode en trames RGBA brutes sur un tube, en boucle infinie
//! (`-stream_loop -1`) ; s'il meurt, il est relancé.
//!
//! La chaîne de filtres cadre au centre à la taille demandée (le rendu
//! étire ensuite la trame : l'ajustement « couvrir » ne recadre plus rien),
//! réduit le grand côté à 1920 au plus, et applique le flou demandé (mode
//! verre, verrouillage flouté). Sans ffmpeg, aucun fond vidéo : les
//! surfaces gardent leur couleur de fond.

use std::path::Path;
use std::time::{Duration, Instant};

use iced::futures::{SinkExt, Stream};
use iced::stream;
use iced::widget::image::Handle;

/// Extensions de vidéo acceptées comme fond.
const EXTENSIONS: [&str; 6] = ["mp4", "m4v", "mkv", "webm", "mov", "avi"];

/// Le fichier est une vidéo (d'après son extension).
pub fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_lowercase().as_str()))
}

/// ffmpeg est là pour décoder (sinon : pas de fond vidéo).
pub fn has_decoder() -> bool {
    crate::doctor::has("ffmpeg")
}

/// Recette d'un fond vidéo : fichier, taille de trame, flou, part étalée.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub path: std::path::PathBuf,
    /// Taille de trame demandée à ffmpeg (paire) : le rendu l'étire à l'écran.
    pub w: u32,
    pub h: u32,
    /// Trames par seconde (le bureau redessine à chaque trame : moins vite).
    pub fps: u32,
    /// Flou gaussien dans la chaîne (sigma).
    pub blur: Option<f32>,
    /// Scène étalée : taille du canevas, puis part de l'écran dans le canevas
    /// (x, y, largeur, hauteur — les mêmes valeurs que `span::Placement`).
    pub span: Option<((u32, u32), crate::span::Placement)>,
}

/// Taille de trame : grand côté borné à `bound`, dimensions paires.
pub fn target_size(w: u32, h: u32, bound: u32) -> (u32, u32) {
    let even = |v: u32| (v.max(2)) & !1;
    let (w, h) = (w.max(1), h.max(1));
    let m = w.max(h);
    if m <= bound {
        (even(w), even(h))
    } else {
        let f = bound as f32 / m as f32;
        (
            even((w as f32 * f).round() as u32),
            even((h as f32 * f).round() as u32),
        )
    }
}

/// Chaîne de filtres : cadence, cadrage (ou part étalée), flou.
fn filter(r: &Recipe) -> String {
    let mut f = format!("fps={}", r.fps.max(1));
    match r.span {
        Some(((cw, ch), (x, y, pw, ph))) => {
            // La vidéo couvre le canevas de la disposition, l'écran y prend
            // sa part, remise à la taille de trame demandée.
            f.push_str(&format!(
                ",scale={cw}:{ch}:force_original_aspect_ratio=increase,crop={cw}:{ch},\
                 crop={pw}:{ph}:{x}:{y},scale={}:{}",
                r.w, r.h
            ));
        }
        None => {
            f.push_str(&format!(
                ",scale={}:{}:force_original_aspect_ratio=increase,crop={}:{}",
                r.w, r.h, r.w, r.h
            ));
        }
    }
    if let Some(sigma) = r.blur {
        f.push_str(&format!(",gblur=sigma={sigma}"));
    }
    f
}

/// Commande ffmpeg : la vidéo en boucle, en trames RGBA brutes sur stdout.
/// Le rythme n'est pas laissé à ffmpeg (`-re` attend un saut d'horloge à
/// chaque jointure de boucle : l'affichage se fige un demi-seconde) : le
/// lecteur (`frames`) lit une trame par créneau, et le contrôle de flux du
/// tube règle ffmpeg tout seul.
pub fn argv(r: &Recipe) -> Vec<String> {
    vec![
        "ffmpeg".into(),
        "-nostdin".into(),
        "-stream_loop".into(),
        "-1".into(),
        "-loglevel".into(),
        "error".into(),
        "-an".into(),
        "-sn".into(),
        "-dn".into(),
        "-i".into(),
        r.path.display().to_string(),
        "-vf".into(),
        filter(r),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "rgba".into(),
        "pipe:1".into(),
    ]
}

/// Une trame prête à afficher : son image, et l'analyse du lanceur (bureau).
#[derive(Debug, Clone)]
pub struct Frame {
    pub handle: Handle,
    pub backdrop: Option<crate::ui::plugins::launcher::contrast::Backdrop>,
}

/// Boucle de lecture : trames tant que quelqu'un écoute, une par créneau de
/// 1/fps (l'horloge est locale : aucune pause à la jointure de boucle ; le
/// tube bloque ffmpeg quand la lecture s'endort). ffmpeg mort est relancé :
/// 250 ms après une vraie lecture (reprise de la boucle), délai doublé sinon
/// et plafonné à 10 s (fichier cassé : pas de boucle chaude).
pub fn frames(r: Recipe) -> impl Stream<Item = Frame> {
    stream::channel(2, async move |mut tx| {
        let (w, h) = (r.w.max(1), r.h.max(1));
        let size = w as usize * h as usize * 4;
        let step = Duration::from_millis((1000 / r.fps.max(1)).max(20) as u64);
        let mut delay = Duration::from_millis(250);
        loop {
            let a = argv(&r);
            let spawned = tokio::process::Command::new(&a[0])
                .args(&a[1..])
                .stdin(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .kill_on_drop(true)
                .spawn();
            let mut child = match spawned {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        "{}",
                        crate::tr!("fond vidéo : ffmpeg ne démarre pas — {e}", e = e)
                    );
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(10));
                    continue;
                }
            };
            let started = Instant::now();
            let Some(mut pipe) = child.stdout.take() else {
                let _ = child.wait().await;
                tokio::time::sleep(delay).await;
                continue;
            };
            use tokio::io::AsyncReadExt;
            let mut next = Instant::now();
            loop {
                // Créneau suivant : la lecture qui suit attend son tour, et le
                // tube endort ffmpeg pendant ce temps.
                next = if next > Instant::now() {
                    next
                } else {
                    Instant::now()
                };
                tokio::time::sleep_until(next.into()).await;
                next += step;
                let mut data = vec![0u8; size];
                if pipe.read_exact(&mut data).await.is_err() {
                    break;
                }
                let Some(img) = image::RgbaImage::from_raw(w, h, data) else {
                    break;
                };
                let backdrop = crate::ui::plugins::launcher::contrast::Backdrop::from_rgba(&img);
                let handle = Handle::from_rgba(w, h, img.into_raw());
                if tx
                    .send(Frame {
                        handle,
                        backdrop: Some(backdrop),
                    })
                    .await
                    .is_err()
                {
                    return; // Plus personne n'écoute : le fond a changé ou l'app sort.
                }
            }
            let _ = child.wait().await;
            delay = if started.elapsed() >= Duration::from_secs(2) {
                Duration::from_millis(250)
            } else {
                (delay * 2).min(Duration::from_secs(10))
            };
            tokio::time::sleep(delay).await;
        }
    })
}

/// Première trame, réduite et cadrée au centre (vignette des réglages).
/// Bloquant : à lancer hors du fil de l'interface.
pub fn first_frame(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    let a = [
        "ffmpeg".to_string(),
        "-nostdin".into(),
        "-loglevel".into(),
        "error".into(),
        "-an".into(),
        "-sn".into(),
        "-dn".into(),
        "-i".into(),
        path.display().to_string(),
        "-frames:v".into(),
        "1".into(),
        "-vf".into(),
        format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "rgba".into(),
        "pipe:1".into(),
    ];
    let mut child = std::process::Command::new(&a[0])
        .args(&a[1..])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let mut pipe = child.stdout.take()?;
    use std::io::Read;
    let mut data = vec![0u8; w as usize * h as usize * 4];
    pipe.read_exact(&mut data).ok()?;
    let _ = child.wait();
    Some(data)
}

/// Dimensions de la première piste vidéo (ffprobe, s'il est là) : pour
/// l'avis de résolution des réglages. Bloquant : hors du fil de l'interface.
pub fn dimensions(path: &Path) -> Option<(u32, u32)> {
    if !crate::launch::executable("ffprobe") {
        return None;
    }
    let out = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
            &path.display().to_string(),
        ])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout.lines().next()?;
    let mut it = line.split(',');
    let w: u32 = it.next()?.trim().parse().ok()?;
    let h: u32 = it.next()?.trim().parse().ok()?;
    (w > 0 && h > 0).then_some((w, h))
}

/// mpv est là pour porter le fond vidéo en GPU (rendu vsyncé, boucle sans
/// accroc) — la route de choix quand le compositeur l'autorise.
pub fn has_player() -> bool {
    crate::launch::executable("mpv")
}

/// Commande mpv : la vidéo en boucle plein écran, sans son ni interface, au
/// rythme d'affichage rééchantillonné (mouvement fluide sur tout
/// rafraîchissement). `--no-config` : l'environnement du greeter ne doit rien
/// hériter de la config de l'utilisateur. Le titre distinctif sert à ne
/// nettoyer que ceux de hyprdesk.
pub fn mpv_args(path: &Path) -> Vec<String> {
    vec![
        "mpv".into(),
        "--no-config".into(),
        "--no-audio".into(),
        "--loop-file=inf".into(),
        "--fullscreen".into(),
        "--no-border".into(),
        "--panscan=1.0".into(),
        "--video-sync=display-resample".into(),
        "--interpolation=yes".into(),
        "--osc=no".into(),
        "--osd-level=0".into(),
        "--cursor-autohide=always".into(),
        "--input-default-bindings=no".into(),
        "--input-vo-keyboard=no".into(),
        "--title=hyprdesk-greeter-video".into(),
        path.display().to_string(),
    ]
}

/// Le mpv d'un fond précédent (greeter relancé) ne doit pas rester au-dessus
/// du nouveau : précisément ceux de hyprdesk (titre), rien d'autre — les
/// mpv de l'utilisateur restent.
pub fn stop_players() {
    let _ = std::process::Command::new("pkill")
        .args(["-f", "title=hyprdesk-greeter-video"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(w: u32, h: u32) -> Recipe {
        Recipe {
            path: "/v/fond.mp4".into(),
            w,
            h,
            fps: 30,
            blur: None,
            span: None,
        }
    }

    #[test]
    fn extensions_reconnues() {
        assert!(is_video(Path::new("/a/Fond.MP4")));
        assert!(is_video(Path::new("/a/f.webm")));
        assert!(!is_video(Path::new("/a/fond.jpg")));
        assert!(!is_video(Path::new("/a/fond")));
        assert!(!is_video(Path::new("/a/mp4")));
    }

    #[test]
    fn taille_bornee_et_paire() {
        assert_eq!(target_size(1920, 1080, 1920), (1920, 1080));
        assert_eq!(target_size(3840, 2160, 1920), (1920, 1080));
        assert_eq!(target_size(2560, 1440, 1920), (1920, 1080));
        assert_eq!(target_size(1080, 1920, 1920), (1080, 1920));
        assert_eq!(target_size(3440, 1440, 1920), (1920, 804), "ultralarge");
        assert_eq!(target_size(1366, 769, 1920), (1366, 768), "paire");
        assert_eq!(target_size(1, 1, 1920), (2, 2), "jamais nul");
        assert_eq!(target_size(0, 0, 1920), (2, 2));
    }

    #[test]
    fn commande_ffmpeg() {
        let a = argv(&recipe(1920, 1080));
        let s = a.join(" ");
        assert!(s.contains("-stream_loop -1"), "boucle infinie");
        assert!(!s.contains("-re"), "le rythme est tenu par le lecteur");
        assert!(s.contains("-f rawvideo -pix_fmt rgba pipe:1"));
        assert!(s.contains("fps=30"));
        assert!(s.contains("scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080"));
        assert!(s.contains("-an -sn -dn"), "aucune piste sonore");
        assert!(!s.contains("gblur"));
        // Flou (mode verre).
        let mut r = recipe(480, 270);
        r.blur = Some(2.5);
        assert!(argv(&r).join(" ").contains("gblur=sigma=2.5"));
        // Part d'une scène étalée.
        r.blur = None;
        r.span = Some(((3840, 2160), (1920, 0, 1920, 1080)));
        let s = argv(&r).join(" ");
        assert!(s.contains("scale=3840:2160:force_original_aspect_ratio=increase,crop=3840:2160"));
        assert!(s.contains("crop=1920:1080:1920:0"));
    }

    #[test]
    fn commande_mpv() {
        let a = mpv_args(Path::new("/etc/greetd/hyprdesk-fond.mp4"));
        let s = a.join(" ");
        assert!(s.contains("--loop-file=inf"), "boucle infinie");
        assert!(s.contains("--no-config"), "aucune config utilisateur");
        assert!(s.contains("--no-audio"));
        assert!(s.contains("--video-sync=display-resample --interpolation=yes"));
        assert!(s.contains("--fullscreen --no-border --panscan=1.0"));
        assert!(s.ends_with("/etc/greetd/hyprdesk-fond.mp4"));
        assert!(
            s.contains("--input-default-bindings=no"),
            "clavier au greeter"
        );
        assert!(s.contains("--title=hyprdesk-greeter-video"), "nettoyable");
    }

    #[test]
    fn vignette_cadree() {
        // Le fichier n'existe pas : rien.
        assert!(first_frame(Path::new("/pas-là/f.mp4"), 160, 90).is_none());
    }

    /// Chaîne ffmpeg réelle : un clip de test décodé à la taille demandée.
    /// Sans ffmpeg (CI légère) : rien à prouver ici.
    #[test]
    fn premiere_trame_lue() {
        if !has_decoder() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("hyprdesk-video-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let clip = dir.join("clip.mp4");
        let ok = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=1280x720:rate=30:duration=1",
                "-pix_fmt",
                "yuv420p",
                "-y",
            ])
            .arg(&clip)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "clip de test généré");
        assert!(is_video(&clip));
        assert_eq!(dimensions(&clip), Some((1280, 720)));
        let d = first_frame(&clip, 160, 90).expect("première trame");
        assert_eq!(d.len(), 160 * 90 * 4);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
