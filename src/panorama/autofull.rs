//! Vidéo seule automatique du lecteur du Panorama (MLG-172) : quand la
//! source se met à jouer, hyprdesk trouve la vidéo dans la page et la passe
//! en plein écran (qui reste dans la zone du lecteur, `assets/zones.lua`).
//!
//! Aucune page n'est lue (le navigateur garde le profil de la personne) :
//! - la vidéo est la région qui bouge entre trois captures rapprochées de la
//!   fenêtre (`grim`) ;
//! - on essaie dans l'ordre le double-clic sur la vidéo, la touche F, puis
//!   les places habituelles du bouton plein écran (en bas à droite de la
//!   vidéo, commandes révélées par le survol) ;
//! - chaque essai est vérifié : la vidéo doit alors remplir la fenêtre ;
//! - la méthode qui a marché est mémorisée pour la source (`fs`), et rejouée
//!   directement les fois suivantes.
//!
//! Le curseur et la fenêtre active sont rendus à la fin.

use std::time::Duration;

/// Rectangle global (px logiques).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Méthode de mise en plein écran, mémorisée par source.
#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    DoubleClick,
    Key,
    /// Clic à cette place, en part de la vidéo (0 à 1).
    Click(f32, f32),
}

impl Method {
    pub fn id(&self) -> String {
        match self {
            Method::DoubleClick => "dblclick".into(),
            Method::Key => "key".into(),
            Method::Click(x, y) => format!("click:{x:.3},{y:.3}"),
        }
    }
    pub fn parse(s: &str) -> Option<Method> {
        Some(match s.trim() {
            "dblclick" => Method::DoubleClick,
            "key" => Method::Key,
            s => {
                let (x, y) = s.strip_prefix("click:")?.split_once(',')?;
                let (x, y) = (x.trim().parse().ok()?, y.trim().parse().ok()?);
                if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
                    return None;
                }
                Method::Click(x, y)
            }
        })
    }
}

/// Places habituelles du bouton plein écran, en part de la vidéo.
const SPOTS: &[(f32, f32)] = &[(0.965, 0.935), (0.94, 0.935), (0.965, 0.9)];

/// Fenêtre : rectangle et plein écran côté application.
fn window(address: &str) -> Option<(Rect, bool)> {
    let v = hyprpanel_core::ipc::query_json("clients").ok()?;
    v.as_array()?.iter().find_map(|c| {
        (c["address"].as_str() == Some(address)).then(|| {
            (
                Rect {
                    x: c["at"][0].as_f64().unwrap_or(0.0) as f32,
                    y: c["at"][1].as_f64().unwrap_or(0.0) as f32,
                    w: c["size"][0].as_f64().unwrap_or(0.0) as f32,
                    h: c["size"][1].as_f64().unwrap_or(0.0) as f32,
                },
                c["fullscreenClient"].as_i64().unwrap_or(0) != 0,
            )
        })
    })
}

/// Capture d'une région (grim, PNG sur la sortie standard).
fn capture(r: Rect) -> Option<image::RgbaImage> {
    let out = std::process::Command::new("grim")
        .args([
            "-g",
            &format!(
                "{},{} {}x{}",
                r.x.round() as i32,
                r.y.round() as i32,
                r.w.round().max(1.0) as i32,
                r.h.round().max(1.0) as i32
            ),
            "-t",
            "png",
            "-l",
            "0",
            "-",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(image::load_from_memory(&out.stdout).ok()?.to_rgba8())
}

const GW: usize = 48;
const GH: usize = 27;

/// Cases de la grille qui changent entre deux images de même taille.
fn changed(a: &image::RgbaImage, b: &image::RgbaImage, grid: &mut [bool]) {
    if a.dimensions() != b.dimensions() {
        return;
    }
    let (w, h) = a.dimensions();
    for gy in 0..GH {
        for gx in 0..GW {
            let (x0, x1) = (gx as u32 * w / GW as u32, (gx as u32 + 1) * w / GW as u32);
            let (y0, y1) = (gy as u32 * h / GH as u32, (gy as u32 + 1) * h / GH as u32);
            let (mut sum, mut n) = (0u64, 0u64);
            for y in (y0..y1).step_by(3) {
                for x in (x0..x1).step_by(3) {
                    let (p, q) = (a.get_pixel(x, y).0, b.get_pixel(x, y).0);
                    sum += (0..3).map(|i| p[i].abs_diff(q[i]) as u64).sum::<u64>();
                    n += 3;
                }
            }
            if n > 0 && sum / n > 6 {
                grid[gy * GW + gx] = true;
            }
        }
    }
}

/// Plus grande région de cases qui changent : (x, y, l, h) en part de
/// l'image ; `None` sous 4 % de l'image.
fn largest(grid: &[bool]) -> Option<(f32, f32, f32, f32)> {
    let mut seen = vec![false; grid.len()];
    let mut best: Option<(usize, (usize, usize, usize, usize))> = None;
    for start in 0..grid.len() {
        if !grid[start] || seen[start] {
            continue;
        }
        let (mut n, mut bb) = (0, (GW, GH, 0, 0));
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(i) = stack.pop() {
            let (x, y) = (i % GW, i / GW);
            n += 1;
            bb = (bb.0.min(x), bb.1.min(y), bb.2.max(x), bb.3.max(y));
            let mut push = |j: usize| {
                if grid[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                push(i - 1);
            }
            if x + 1 < GW {
                push(i + 1);
            }
            if y > 0 {
                push(i - GW);
            }
            if y + 1 < GH {
                push(i + GW);
            }
        }
        if best.is_none_or(|(m, _)| n > m) {
            best = Some((n, bb));
        }
    }
    let (n, (x0, y0, x1, y1)) = best?;
    if (n as f32) < 0.04 * (GW * GH) as f32 {
        return None;
    }
    Some((
        x0 as f32 / GW as f32,
        y0 as f32 / GH as f32,
        (x1 + 1 - x0) as f32 / GW as f32,
        (y1 + 1 - y0) as f32 / GH as f32,
    ))
}

/// La vidéo dans la fenêtre `win` : la région qui bouge (rectangle global).
pub fn video_box(win: Rect) -> Option<Rect> {
    let a = capture(win)?;
    std::thread::sleep(Duration::from_millis(250));
    let b = capture(win)?;
    std::thread::sleep(Duration::from_millis(250));
    let c = capture(win)?;
    let mut grid = vec![false; GW * GH];
    changed(&a, &b, &mut grid);
    changed(&b, &c, &mut grid);
    let (fx, fy, fw, fh) = largest(&grid)?;
    Some(Rect {
        x: win.x + fx * win.w,
        y: win.y + fy * win.h,
        w: fw * win.w,
        h: fh * win.h,
    })
}

/// Région qui a la forme d'une vidéo : proportions entre 1,2:1 et 2,5:1,
/// au moins 12 % de la fenêtre (une page de musique, une animation ou une
/// barre de progression n'en sont pas).
fn video_like(v: Rect, win: Rect) -> bool {
    let ratio = v.w / v.h.max(1.0);
    (1.2..=2.5).contains(&ratio) && v.w * v.h >= 0.12 * win.w * win.h
}

/// La vidéo remplit la fenêtre : couverture d'au moins 85 % et proportions
/// de la fenêtre, mesurées deux fois (une animation de la page passe).
fn covers(v: Rect, win: Rect) -> bool {
    let (rv, rw) = (v.w / v.h.max(1.0), win.w / win.h.max(1.0));
    v.w * v.h >= 0.85 * win.w * win.h && (rv / rw - 1.0).abs() < 0.15
}

/// La vidéo remplit la fenêtre (vidéo seule).
fn filled(address: &str) -> bool {
    let Some((win, full)) = window(address) else {
        return false;
    };
    if full {
        return true;
    }
    let once = || video_box(win).is_some_and(|v| covers(v, win));
    once() && {
        std::thread::sleep(Duration::from_millis(400));
        once()
    }
}

fn move_cursor(x: f32, y: f32) {
    let _ = hyprpanel_core::ipc::eval(&format!(
        "hl.dispatch(hl.dsp.cursor.move({{ x = {}, y = {} }}))",
        x.round() as i32,
        y.round() as i32
    ));
}

fn cursor() -> Option<(f32, f32)> {
    let v = hyprpanel_core::ipc::query_json("cursorpos").ok()?;
    Some((v["x"].as_f64()? as f32, v["y"].as_f64()? as f32))
}

fn active() -> Option<String> {
    let v = hyprpanel_core::ipc::query_json("activewindow").ok()?;
    v["address"].as_str().map(str::to_string)
}

/// Clic gauche (`n` fois) là où est le curseur : pointeur virtuel
/// (zwlr_virtual_pointer_v1, que Hyprland fournit).
fn click(n: u32) -> Result<(), String> {
    use wayland_client::protocol::{wl_pointer::ButtonState, wl_registry};
    use wayland_client::{Connection, Dispatch, QueueHandle};
    use wayland_protocols_wlr::virtual_pointer::v1::client::{
        zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
    };
    struct St(Option<ZwlrVirtualPointerManagerV1>);
    impl Dispatch<wl_registry::WlRegistry, ()> for St {
        fn event(
            st: &mut Self,
            reg: &wl_registry::WlRegistry,
            ev: wl_registry::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global {
                name,
                interface,
                version,
            } = ev
                && interface == "zwlr_virtual_pointer_manager_v1"
            {
                st.0 = Some(reg.bind(name, version.min(2), qh, ()));
            }
        }
    }
    impl Dispatch<ZwlrVirtualPointerManagerV1, ()> for St {
        fn event(
            _: &mut Self,
            _: &ZwlrVirtualPointerManagerV1,
            _: <ZwlrVirtualPointerManagerV1 as wayland_client::Proxy>::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }
    impl Dispatch<ZwlrVirtualPointerV1, ()> for St {
        fn event(
            _: &mut Self,
            _: &ZwlrVirtualPointerV1,
            _: <ZwlrVirtualPointerV1 as wayland_client::Proxy>::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }
    const BTN_LEFT: u32 = 0x110;
    let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let mut q = conn.new_event_queue();
    let qh = q.handle();
    conn.display().get_registry(&qh, ());
    let mut st = St(None);
    q.roundtrip(&mut st).map_err(|e| e.to_string())?;
    let mgr =
        st.0.take()
            .ok_or_else(|| "pointeur virtuel indisponible".to_string())?;
    let p = mgr.create_virtual_pointer(None, &qh, ());
    let t0 = std::time::Instant::now();
    let ms = || t0.elapsed().as_millis() as u32;
    for _ in 0..n {
        p.button(ms(), BTN_LEFT, ButtonState::Pressed);
        p.frame();
        conn.flush().map_err(|e| e.to_string())?;
        std::thread::sleep(Duration::from_millis(40));
        p.button(ms(), BTN_LEFT, ButtonState::Released);
        p.frame();
        conn.flush().map_err(|e| e.to_string())?;
        std::thread::sleep(Duration::from_millis(70));
    }
    p.destroy();
    mgr.destroy();
    q.roundtrip(&mut st).map_err(|e| e.to_string())?;
    Ok(())
}

fn apply(address: &str, video: Rect, m: &Method) {
    match m {
        Method::DoubleClick => {
            move_cursor(video.x + video.w / 2.0, video.y + video.h / 2.0);
            std::thread::sleep(Duration::from_millis(80));
            let _ = click(2);
        }
        Method::Key => {
            let _ = crate::hypr::focus_window(address);
            std::thread::sleep(Duration::from_millis(80));
            let _ = super::video::send_key(address, "f");
        }
        Method::Click(fx, fy) => {
            // Survol d'abord : les commandes du lecteur apparaissent.
            move_cursor(video.x + video.w / 2.0, video.y + video.h / 2.0);
            std::thread::sleep(Duration::from_millis(350));
            move_cursor(video.x + fx * video.w, video.y + fy * video.h);
            std::thread::sleep(Duration::from_millis(250));
            let _ = click(1);
        }
    }
}

/// Passe la vidéo de la fenêtre en plein écran ; `known` : méthode
/// mémorisée pour la source. Rend la méthode qui a marché (`None` : déjà
/// en vidéo seule). Bloquant (quelques secondes).
pub fn run(address: &str, known: Option<Method>) -> Result<Option<Method>, String> {
    let (win, full) = window(address).ok_or("fenêtre du lecteur introuvable")?;
    if full {
        return Ok(None);
    }
    let video = video_box(win)
        .filter(|v| video_like(*v, win) || covers(*v, win))
        .ok_or("aucune vidéo en mouvement dans la page")?;
    if covers(video, win) {
        return Ok(None);
    }
    let back_cursor = cursor();
    let back_focus = active();
    let mut tries: Vec<Method> = known.into_iter().collect();
    for m in [Method::DoubleClick, Method::Key]
        .into_iter()
        .chain(SPOTS.iter().map(|(x, y)| Method::Click(*x, *y)))
    {
        if !tries.contains(&m) {
            tries.push(m);
        }
    }
    let mut found = None;
    for m in tries {
        apply(address, video, &m);
        std::thread::sleep(Duration::from_millis(1500));
        if filled(address) {
            found = Some(m);
            break;
        }
    }
    if let Some((x, y)) = back_cursor {
        move_cursor(x, y);
    }
    if let Some(a) = back_focus.filter(|a| a != address) {
        let _ = crate::hypr::focus_window(&a);
    }
    found
        .map(Some)
        .ok_or_else(|| "bouton plein écran introuvable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methode_lue_et_ecrite() {
        for m in [
            Method::DoubleClick,
            Method::Key,
            Method::Click(0.965, 0.935),
        ] {
            assert_eq!(Method::parse(&m.id()), Some(m));
        }
        assert_eq!(Method::parse("click:2,0"), None);
        assert_eq!(Method::parse("x"), None);
    }

    #[test]
    fn forme_de_video() {
        let win = Rect {
            x: 0.0,
            y: 0.0,
            w: 708.0,
            h: 398.0,
        };
        let r = |w, h| Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
        };
        assert!(video_like(r(480.0, 270.0), win));
        // Barre de progression, pochette carrée, petite animation : non.
        assert!(!video_like(r(700.0, 40.0), win));
        assert!(!video_like(r(280.0, 280.0), win));
        assert!(!video_like(r(120.0, 68.0), win));
        assert!(covers(r(700.0, 396.0), win));
        assert!(!covers(r(708.0, 300.0), win));
    }

    #[test]
    fn plus_grande_region_qui_bouge() {
        let mut g = vec![false; GW * GH];
        // Vidéo : 24×12 cases en haut à gauche ; bruit isolé ailleurs.
        for y in 2..14 {
            for x in 4..28 {
                g[y * GW + x] = true;
            }
        }
        g[20 * GW + 40] = true;
        let (x, y, w, h) = largest(&g).unwrap();
        assert!((x - 4.0 / GW as f32).abs() < 1e-6 && (y - 2.0 / GH as f32).abs() < 1e-6);
        assert!((w - 24.0 / GW as f32).abs() < 1e-6 && (h - 12.0 / GH as f32).abs() < 1e-6);
        assert!(largest(&vec![false; GW * GH]).is_none());
    }
}
