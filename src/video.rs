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
    /// Trame entière, aux proportions de la source et au grand côté borné
    /// par `w.max(h)` : une seule lecture sert alors tous les écrans qui
    /// montrent ce fond, chacun en recadrant sa part au rendu
    /// (`Tiles::view`). Sans effet sur une scène étalée.
    pub whole: bool,
}

/// Taille de trame ramenée aux pixels de source qu'elle montre vraiment :
/// ffmpeg couvre la trame puis la recadre, une trame plus grande que cette
/// région n'est qu'un agrandissement (proportions gardées, dimensions
/// paires).
fn capped(w: u32, h: u32, source: Option<(u32, u32)>) -> (u32, u32) {
    let Some((sw, sh)) = source else {
        return (w, h);
    };
    let f = (sw as f32 / w.max(1) as f32).min(sh as f32 / h.max(1) as f32);
    if f >= 1.0 {
        return (w, h);
    }
    let even = |v: f32| ((v.round() as u32).max(2)) & !1;
    (even(w as f32 * f), even(h as f32 * f))
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
    // Vitesse réglée du fond vidéo (MLG-280).
    let speed = crate::wallpaper_fx::current().speed;
    if (speed - 1.0).abs() > 1e-3 {
        f.push_str(&format!(",setpts=PTS/{speed}"));
    }
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

/// Un nœud de rendu DRM existe : le décodage matériel (VA-API) vaut d'être
/// tenté.
fn has_render_node() -> bool {
    std::fs::read_dir("/dev/dri").is_ok_and(|d| {
        d.flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("renderD"))
    })
}

/// Commande ffmpeg : la vidéo en boucle, en trames RGBA brutes sur stdout.
/// Le rythme n'est pas laissé à ffmpeg (`-re` attend un saut d'horloge à
/// chaque jointure de boucle : l'affichage se fige un demi-seconde) : le
/// lecteur (`frames`) lit une trame par créneau, et le contrôle de flux du
/// tube règle ffmpeg tout seul.
///
/// `hw` : décodage par VA-API (le processeur ne fait plus que la conversion
/// en RGBA : deux fois moins de calcul pour une 720p). Les fils sont
/// comptés : un fond à 24 i/s n'a pas besoin de tous les cœurs, et leur
/// synchronisation coûtait un quart du temps en logiciel.
pub fn argv(r: &Recipe, hw: bool) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "ffmpeg".into(),
        "-nostdin".into(),
        "-stream_loop".into(),
        "-1".into(),
        "-loglevel".into(),
        "error".into(),
    ];
    if hw {
        a.extend(["-hwaccel".into(), "vaapi".into()]);
    }
    a.extend([
        "-threads".into(),
        if hw { "1" } else { "2" }.into(),
        "-filter_threads".into(),
        "1".into(),
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
    ]);
    a
}

/// Taille sous laquelle iced_wgpu 0.14 téléverse une image dans le dessin
/// même (`image/cache.rs`, `upload_raster`) ; au-delà, il la téléverse en
/// tâche de fond et ne la dessine qu'une fois sur le GPU. Une trame vidéo
/// (3,7 Mo en 720p, 8,3 en 1080p) étant une image neuve à chaque fois, le
/// fond disparaissait une ou deux images : il clignotait, surtout quand la
/// machine peinait (jointure de boucle de ffmpeg). Découpée en bandes plus
/// petites, chaque trame est affichée dès le dessin qui la reçoit.
const SYNC_UPLOAD: usize = 2 * 1024 * 1024;

/// Trame vidéo en bandes horizontales de moins de [`SYNC_UPLOAD`] octets
/// (les lignes RGBA se suivent : les bandes sont des tranches du même
/// tampon, sans recopie de pixels).
#[derive(Debug, Clone)]
pub struct Tiles {
    w: u32,
    h: u32,
    /// Bandes de haut en bas, avec leur nombre de lignes.
    strips: Vec<(Handle, u32)>,
}

impl Tiles {
    #[cfg(test)]
    pub fn new(w: u32, h: u32, data: Vec<u8>) -> Self {
        Self::from_bytes(w, h, iced::advanced::graphics::core::Bytes::from(data))
    }

    fn from_bytes(w: u32, h: u32, data: iced::advanced::graphics::core::Bytes) -> Self {
        let row = (w.max(1) as usize) * 4;
        let band = ((SYNC_UPLOAD - 1) / row).max(1) * row;
        let strips = (0..data.len())
            .step_by(band)
            .map(|start| {
                let end = (start + band).min(data.len());
                let rows = ((end - start) / row) as u32;
                (Handle::from_rgba(w, rows, data.slice(start..end)), rows)
            })
            .collect();
        Self { w, h, strips }
    }

    /// Bandes visibles pour un rendu « couvrir » dans `size` : la région
    /// centrale de la trame (en pixels de trame), coupée bande par bande —
    /// (image, région rognée, lignes affichées).
    fn cover(&self, size: iced::Size) -> Vec<(Handle, iced::Rectangle<u32>)> {
        let (w, h) = (self.w.max(1) as f32, self.h.max(1) as f32);
        let s = (size.width / w).max(size.height / h);
        if !s.is_finite() || s <= 0.0 {
            return Vec::new();
        }
        let cw = ((size.width / s).round() as u32).clamp(1, self.w.max(1));
        let ch = ((size.height / s).round() as u32).clamp(1, self.h.max(1));
        let x0 = (self.w.max(1) - cw) / 2;
        let y0 = (self.h.max(1) - ch) / 2;
        let y1 = y0 + ch;
        let mut out = Vec::new();
        let mut top = 0;
        for (handle, rows) in &self.strips {
            let (a, b) = (top.max(y0), (top + rows).min(y1));
            if a < b {
                out.push((
                    handle.clone(),
                    iced::Rectangle {
                        x: x0,
                        y: a - top,
                        width: cw,
                        height: b - a,
                    },
                ));
            }
            top += rows;
        }
        out
    }

    /// La trame à l'ajustement « couvrir », à l'opacité donnée.
    pub fn view<'a, M: 'a>(&self, alpha: f32) -> iced::Element<'a, M> {
        use iced::Length;
        use iced::widget::{column, image, responsive};
        let tiles = self.clone();
        responsive(move |size| {
            column(tiles.cover(size).into_iter().map(|(handle, crop)| {
                image(handle)
                    .crop(crop)
                    .content_fit(iced::ContentFit::Fill)
                    .filter_method(image::FilterMethod::Linear)
                    .width(Length::Fill)
                    .height(Length::FillPortion(crop.height.min(u16::MAX as u32) as u16))
                    .opacity(alpha)
                    .into()
            }))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        })
        .into()
    }
}

/// Tampons de trame recyclés : une trame de 3,7 Mo allouée à neuf, mise à
/// zéro puis rendue au système vingt-quatre fois par seconde coûtait plus
/// de cent mille défauts de page par seconde au bureau. Le tampon revient
/// ici quand la dernière bande qui le partage est lâchée (iced l'a
/// téléversée et oubliée).
#[derive(Clone, Default)]
struct Pool(std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>>);

/// Garde au plus ce nombre de tampons libres (les trames en vol, plus une).
const POOL_KEEP: usize = 6;

impl Pool {
    fn take(&self, size: usize) -> Vec<u8> {
        let spare = self.0.lock().ok().and_then(|mut v| v.pop());
        match spare {
            Some(b) if b.len() == size => b,
            _ => vec![0u8; size],
        }
    }

    fn bytes(&self, buf: Vec<u8>) -> iced::advanced::graphics::core::Bytes {
        iced::advanced::graphics::core::Bytes::from_owner(Pooled {
            buf,
            pool: self.clone(),
        })
    }
}

/// Tampon prêté aux bandes d'une trame, rendu au [`Pool`] à la fin.
struct Pooled {
    buf: Vec<u8>,
    pool: Pool,
}

impl AsRef<[u8]> for Pooled {
    fn as_ref(&self) -> &[u8] {
        &self.buf
    }
}

impl Drop for Pooled {
    fn drop(&mut self) {
        if let Ok(mut v) = self.pool.0.lock()
            && v.len() < POOL_KEEP
        {
            v.push(std::mem::take(&mut self.buf));
        }
    }
}

/// ffmpeg à la priorité la plus basse : il ne prend que le temps que
/// personne d'autre ne veut (SCHED_IDLE, gentillesse 19, entrées-sorties
/// en classe « idle »). Le fond peut sauter des trames sous charge, une
/// application au premier plan jamais.
fn lowest_priority(cmd: &mut tokio::process::Command) {
    // SAFETY : entre fork et exec, seuls des appels système sans
    // allocation, sur le processus lui-même.
    unsafe {
        cmd.pre_exec(|| {
            libc::setpriority(libc::PRIO_PROCESS, 0, 19);
            let param = libc::sched_param { sched_priority: 0 };
            libc::sched_setscheduler(0, libc::SCHED_IDLE, &param);
            // ioprio_set(IOPRIO_WHO_PROCESS, soi, IOPRIO_CLASS_IDLE << 13).
            libc::syscall(libc::SYS_ioprio_set, 1, 0, 3 << 13);
            Ok(())
        });
    }
}

/// Tube agrandi (1 Mio, le plafond sans privilège par défaut) : une trame
/// se lit en quatre appels au lieu de soixante.
fn widen_pipe(pipe: &tokio::process::ChildStdout) {
    use std::os::fd::AsRawFd;
    // SAFETY : fcntl sur un descripteur ouvert que l'on possède.
    unsafe {
        libc::fcntl(pipe.as_raw_fd(), libc::F_SETPIPE_SZ, 1 << 20);
    }
}

/// Une trame prête à afficher : ses bandes, et l'analyse du lanceur (bureau).
#[derive(Debug, Clone)]
pub struct Frame {
    pub tiles: Tiles,
    pub backdrop: Option<crate::ui::plugins::dock::contrast::Backdrop>,
}

/// Boucle de lecture : trames tant que quelqu'un écoute, une par créneau de
/// 1/fps (l'horloge est locale : aucune pause à la jointure de boucle ; le
/// tube bloque ffmpeg quand la lecture s'endort). ffmpeg mort est relancé :
/// 250 ms après une vraie lecture (reprise de la boucle), délai doublé sinon
/// et plafonné à 10 s (fichier cassé : pas de boucle chaude).
pub fn frames(r: Recipe) -> impl Stream<Item = Frame> {
    stream::channel(2, async move |mut tx| {
        // Pas plus de pixels que la source : agrandie par ffmpeg, une vidéo
        // 720p coûtait 2,25 fois plus à copier et téléverser, pour rien (le
        // rendu l'étire de toute façon).
        let path = r.path.clone();
        let source = tokio::task::spawn_blocking(move || dimensions(&path))
            .await
            .ok()
            .flatten();
        let mut r = r;
        if r.whole && r.span.is_none() {
            // Trame entière : les proportions de la source (16:9 si elles
            // sont inconnues), jamais agrandie.
            let (sw, sh) = source.unwrap_or((1920, 1080));
            let bound = r.w.max(r.h).min(sw.max(sh)).max(2);
            (r.w, r.h) = target_size(sw, sh, bound);
        } else {
            (r.w, r.h) = capped(r.w, r.h, source.filter(|_| r.span.is_none()));
        }
        let (w, h) = (r.w.max(1), r.h.max(1));
        let size = w as usize * h as usize * 4;
        let step = Duration::from_millis((1000 / r.fps.max(1)).max(20) as u64);
        let mut delay = Duration::from_millis(250);
        // VA-API tant qu'il tient : un ffmpeg mort sans livrer une trame
        // (pilote absent, codec refusé) repasse au décodage logiciel.
        let mut hw = has_render_node();
        let pool = Pool::default();
        loop {
            let a = argv(&r, hw);
            let mut cmd = tokio::process::Command::new(&a[0]);
            cmd.args(&a[1..])
                .stdin(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .kill_on_drop(true);
            lowest_priority(&mut cmd);
            let spawned = cmd.spawn();
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
            widen_pipe(&pipe);
            use tokio::io::AsyncReadExt;
            let mut next = Instant::now();
            let mut delivered = false;
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
                let mut data = pool.take(size);
                if pipe.read_exact(&mut data).await.is_err() {
                    break;
                }
                let Some(img) = image::RgbaImage::from_raw(w, h, data) else {
                    break;
                };
                let backdrop = crate::ui::plugins::dock::contrast::Backdrop::from_rgba(&img);
                let tiles = Tiles::from_bytes(w, h, pool.bytes(img.into_raw()));
                if tx
                    .send(Frame {
                        tiles,
                        backdrop: Some(backdrop),
                    })
                    .await
                    .is_err()
                {
                    return; // Plus personne n'écoute : le fond a changé ou l'app sort.
                }
                delivered = true;
            }
            let _ = child.wait().await;
            if hw && !delivered {
                tracing::info!("fond vidéo : VA-API indisponible, décodage logiciel");
                hw = false;
                continue;
            }
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

    #[test]
    fn frames_are_capped_to_the_source() {
        // 720p sur un écran 1080p : la trame reste en 720p.
        assert_eq!(capped(1920, 1080, Some((1280, 720))), (1280, 720));
        // Écran portrait : la colonne centrale de la source, 405 × 720.
        assert_eq!(capped(1080, 1920, Some((1280, 720))), (404, 720));
        // Source plus grande, ou inconnue : rien ne change.
        assert_eq!(capped(1920, 1080, Some((3840, 2160))), (1920, 1080));
        assert_eq!(capped(1920, 1080, None), (1920, 1080));
    }

    #[test]
    fn frame_buffers_come_back_to_the_pool() {
        let pool = Pool::default();
        let mut b = pool.take(16);
        b[0] = 7;
        let t = Tiles::from_bytes(2, 2, pool.bytes(b));
        let copy = t.clone();
        drop(t);
        assert!(pool.0.lock().unwrap().is_empty(), "encore partagé");
        drop(copy);
        let back = pool.take(16);
        assert_eq!(back[0], 7, "le même tampon, sans remise à zéro");
        // Taille changée : un tampon neuf.
        drop(pool.bytes(back));
        assert_eq!(pool.take(32).len(), 32);
    }

    #[test]
    fn tiles_stay_under_the_sync_upload_size() {
        let (w, h) = (1920, 1080);
        let t = Tiles::new(w, h, vec![0u8; (w * h * 4) as usize]);
        assert!(t.strips.len() > 1);
        assert_eq!(t.strips.iter().map(|(_, r)| r).sum::<u32>(), h);
        assert!(
            t.strips
                .iter()
                .all(|(_, r)| (w * r * 4) < SYNC_UPLOAD as u32)
        );
    }

    #[test]
    fn cover_crops_the_center_band_by_band() {
        let (w, h) = (1920, 1080);
        let t = Tiles::new(w, h, vec![0u8; (w * h * 4) as usize]);
        // Même proportion : toute la trame, bandes entières.
        let full = t.cover(iced::Size::new(2560.0, 1440.0));
        assert_eq!(full.iter().map(|(_, c)| c.height).sum::<u32>(), h);
        assert!(full.iter().all(|(_, c)| c.x == 0 && c.width == w));
        // Écran portrait : une colonne centrale, toute la hauteur.
        let tall = t.cover(iced::Size::new(1440.0, 2560.0));
        assert_eq!(tall.iter().map(|(_, c)| c.height).sum::<u32>(), h);
        let c = tall[0].1;
        assert_eq!(c.width, 608); // 1080 × 1440 / 2560 ≈ 607,5
        assert_eq!(c.x, (w - c.width) / 2);
        // Bandeau large : les lignes du milieu seulement.
        let wide = t.cover(iced::Size::new(3840.0, 1080.0));
        assert_eq!(wide.iter().map(|(_, c)| c.height).sum::<u32>(), 540);
    }

    fn recipe(w: u32, h: u32) -> Recipe {
        Recipe {
            path: "/v/fond.mp4".into(),
            w,
            h,
            fps: 30,
            blur: None,
            span: None,
            whole: false,
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
        let a = argv(&recipe(1920, 1080), false);
        let s = a.join(" ");
        assert!(!s.contains("-hwaccel"), "logiciel");
        let hw = argv(&recipe(1920, 1080), true).join(" ");
        assert!(
            hw.contains("-hwaccel vaapi -threads 1"),
            "VA-API avant l'entrée"
        );
        assert!(hw.find("-hwaccel").unwrap() < hw.find("-i ").unwrap());
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
        assert!(argv(&r, false).join(" ").contains("gblur=sigma=2.5"));
        // Part d'une scène étalée.
        r.blur = None;
        r.span = Some(((3840, 2160), (1920, 0, 1920, 1080)));
        let s = argv(&r, false).join(" ");
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
