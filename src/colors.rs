//! Couleurs par écran (Réglages → Couleurs des écrans) : luminosité,
//! contraste, gamma, gain de chaque canal, et fichier de calibration.
//!
//! - Les réglages deviennent une table gamma (LUT) par écran, posée par le
//!   protocole `wlr-gamma-control-unstable-v1` que Hyprland implémente : un
//!   fil du démon tient une connexion Wayland à lui, et garde la table tant
//!   qu'il vit (Hyprland rend la table d'origine quand il la lâche, ou quand
//!   l'écran revient aux réglages neutres).
//! - Un fichier `.cal` (ArgyllCMS, `dispcal`) est lu et ses courbes viennent
//!   après les réglages dans la table.
//! - Un profil `.icc` / `.icm` passe par l'option `icc` de Hyprland
//!   (monitors.lua, `crate::screens`), qui en applique aussi les courbes.
//!
//! Un seul programme peut tenir la table gamma d'un écran : si un autre la
//! tient déjà (wlsunset, gammastep…), l'écran est signalé en échec.

use std::collections::{BTreeMap, HashMap};
use std::os::fd::AsFd;
use std::sync::{Arc, Mutex, mpsc};

use wayland_client::protocol::{wl_output, wl_registry};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::gamma_control::v1::client::{
    zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1,
    zwlr_gamma_control_v1::{self, ZwlrGammaControlV1},
};

/// Réglages de couleur d'un écran.
#[derive(Debug, Clone, PartialEq)]
pub struct Adjust {
    /// Décalage ajouté (−0.5 … 0.5 ; 0 : neutre).
    pub brightness: f32,
    /// Pente autour du gris moyen (0.5 … 1.5 ; 1 : neutre).
    pub contrast: f32,
    /// Gamma (0.4 … 2.5 ; 1 : neutre ; plus grand : tons moyens plus clairs).
    pub gamma: f32,
    /// Gain de chaque canal (0.5 … 1.5 ; 1 : neutre).
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    /// Gamma de chaque canal (courbe de couleurs ; 1 : neutre).
    pub gamma_red: f32,
    pub gamma_green: f32,
    pub gamma_blue: f32,
    /// Fichier de calibration : `.cal` (courbes, ici) ou `.icc` / `.icm`
    /// (profil, par Hyprland) ; vide : aucun.
    pub calibration: String,
}

impl Default for Adjust {
    fn default() -> Self {
        Adjust {
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            gamma_red: 1.0,
            gamma_green: 1.0,
            gamma_blue: 1.0,
            calibration: String::new(),
        }
    }
}

/// Champs réglables : (clé, libellé, minimum, maximum, neutre).
pub const FIELDS: &[(&str, &str, f32, f32, f32)] = &[
    ("brightness", "Luminosité", -0.5, 0.5, 0.0),
    ("contrast", "Contraste", 0.5, 1.5, 1.0),
    ("gamma", "Gamma", 0.4, 2.5, 1.0),
    ("red", "Rouge", 0.5, 1.5, 1.0),
    ("green", "Vert", 0.5, 1.5, 1.0),
    ("blue", "Bleu", 0.5, 1.5, 1.0),
    ("gamma_red", "Gamma rouge", 0.4, 2.5, 1.0),
    ("gamma_green", "Gamma vert", 0.4, 2.5, 1.0),
    ("gamma_blue", "Gamma bleu", 0.4, 2.5, 1.0),
];

impl Adjust {
    pub fn get(&self, key: &str) -> Option<f32> {
        Some(match key {
            "brightness" => self.brightness,
            "contrast" => self.contrast,
            "gamma" => self.gamma,
            "red" => self.red,
            "green" => self.green,
            "blue" => self.blue,
            "gamma_red" => self.gamma_red,
            "gamma_green" => self.gamma_green,
            "gamma_blue" => self.gamma_blue,
            _ => return None,
        })
    }

    /// Pose un champ (borné) ; faux si la clé est inconnue.
    pub fn set(&mut self, key: &str, v: f32) -> bool {
        let Some(&(_, _, lo, hi, _)) = FIELDS.iter().find(|f| f.0 == key) else {
            return false;
        };
        let v = if v.is_finite() { v.clamp(lo, hi) } else { return false };
        match key {
            "brightness" => self.brightness = v,
            "contrast" => self.contrast = v,
            "gamma" => self.gamma = v,
            "red" => self.red = v,
            "green" => self.green = v,
            "blue" => self.blue = v,
            "gamma_red" => self.gamma_red = v,
            "gamma_green" => self.gamma_green = v,
            "gamma_blue" => self.gamma_blue = v,
            _ => return false,
        }
        true
    }

    /// Courbes à poser : les réglages ne sont pas neutres, ou un `.cal`.
    pub fn needs_table(&self) -> bool {
        !self.neutral_values() || is_cal(&self.calibration)
    }

    /// Tous les curseurs à leur valeur neutre.
    pub fn neutral_values(&self) -> bool {
        FIELDS
            .iter()
            .all(|(k, _, _, _, n)| self.get(k).is_some_and(|v| (v - n).abs() < 1e-3))
    }

    /// Rien de réglé (ni curseur, ni fichier) : l'écran est laissé tel quel.
    pub fn is_default(&self) -> bool {
        self.neutral_values() && self.calibration.trim().is_empty()
    }

    /// Profil ICC à donner à Hyprland, s'il y en a un.
    pub fn icc(&self) -> Option<&str> {
        let c = self.calibration.trim();
        let l = c.to_lowercase();
        (l.ends_with(".icc") || l.ends_with(".icm")).then_some(c)
    }

    /// Valeur de sortie (0 … 1) du canal `ch` (0 rouge, 1 vert, 2 bleu) pour
    /// l'entrée `x` (0 … 1), courbes de calibration comprises.
    pub fn curve(&self, ch: usize, x: f32, cal: Option<&Cal>) -> f32 {
        let mut y = (x - 0.5) * self.contrast + 0.5 + self.brightness;
        y = y.clamp(0.0, 1.0).powf(1.0 / self.gamma.max(0.05));
        y = y.powf(1.0 / [self.gamma_red, self.gamma_green, self.gamma_blue][ch].max(0.05));
        y *= [self.red, self.green, self.blue][ch];
        y = y.clamp(0.0, 1.0);
        match cal {
            Some(c) => c.eval(ch, y),
            None => y,
        }
    }

    /// Table gamma de `size` entrées par canal (rouge, vert, bleu), en 16 bits.
    pub fn ramps(&self, size: usize, cal: Option<&Cal>) -> [Vec<u16>; 3] {
        let size = size.max(2);
        let ch = |c: usize| {
            (0..size)
                .map(|i| {
                    let x = i as f32 / (size - 1) as f32;
                    (self.curve(c, x, cal) * 65535.0).round() as u16
                })
                .collect::<Vec<u16>>()
        };
        [ch(0), ch(1), ch(2)]
    }
}

// ---------------------------------------------------------------------------
// Profils ICC dans monitors.lua
// ---------------------------------------------------------------------------

static ICC: std::sync::RwLock<BTreeMap<String, String>> = std::sync::RwLock::new(BTreeMap::new());

/// Profils ICC voulus (écran → fichier), repris par chaque ligne
/// `hl.monitor` que hyprdesk écrit ou applique (`crate::screens`).
pub fn set_icc_map(colors: &BTreeMap<String, Adjust>) -> bool {
    let next: BTreeMap<String, String> = colors
        .iter()
        .filter_map(|(m, a)| a.icc().map(|p| (m.clone(), crate::apptheme::expand_home(p))))
        .collect();
    let Ok(mut cur) = ICC.write() else { return false };
    let changed = *cur != next;
    *cur = next;
    changed
}

/// Profil ICC de l'écran, s'il y en a un.
pub fn icc_for(monitor: &str) -> Option<String> {
    ICC.read().ok()?.get(monitor).cloned()
}

/// Profils ICC installés (`~/.local/share/icc`, `/usr/share/color/icc`…).
pub fn installed_profiles() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut out = Vec::new();
    for dir in [
        format!("{home}/.local/share/icc"),
        format!("{home}/.color/icc"),
        "/usr/share/color/icc".to_string(),
        "/usr/local/share/color/icc".to_string(),
        "/var/lib/colord/icc".to_string(),
    ] {
        let mut stack = vec![std::path::PathBuf::from(dir)];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| matches!(x.to_lowercase().as_str(), "icc" | "icm" | "cal"))
                {
                    out.push(p.to_string_lossy().replace(&home, "~"));
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Copie un fichier de calibration dans `~/.config/hypr/hyprdesk/calibration`
/// (le réglage ne dépend plus du dossier de téléchargement) ; rend le chemin gardé.
pub fn import(config_path: &std::path::Path, file: &str) -> Result<String, String> {
    let src = std::path::PathBuf::from(crate::apptheme::expand_home(file.trim()));
    let ext = src
        .extension()
        .and_then(|x| x.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();
    if !matches!(ext.as_str(), "icc" | "icm" | "cal") {
        return Err(crate::i18n::tr("fichier de calibration attendu : .icc, .icm ou .cal").to_string());
    }
    if ext == "cal" {
        let t = std::fs::read_to_string(&src).map_err(|e| format!("{} : {e}", src.display()))?;
        parse_cal(&t)?;
    } else {
        let b = std::fs::read(&src).map_err(|e| format!("{} : {e}", src.display()))?;
        // En-tête ICC : signature « acsp » à l'octet 36.
        if b.len() < 128 || &b[36..40] != b"acsp" {
            return Err(crate::tr!("{f} : ce n'est pas un profil ICC", f = src.display()));
        }
    }
    let dir = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("hyprdesk")
        .join("calibration");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(src.file_name().unwrap_or_default());
    if dest != src {
        std::fs::copy(&src, &dest).map_err(|e| format!("{} : {e}", dest.display()))?;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    Ok(dest.to_string_lossy().replace(&home, "~"))
}

// ---------------------------------------------------------------------------
// Images de test
// ---------------------------------------------------------------------------

/// Image RGBA : largeur, hauteur, octets.
pub type Rgba = (u32, u32, Vec<u8>);

fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = (h.rem_euclid(1.0)) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r + m, g + m, b + m].map(|t| (t * 255.0).round() as u8)
}

/// Mire de référence : dégradés gris et couleur, paliers des noirs et des
/// blancs (chaque case doit se distinguer), damier de gamma, couleurs
/// primaires et secondaires, tons chair.
pub fn reference_image(w: u32, h: u32) -> Rgba {
    let mut px = vec![0u8; (w * h * 4) as usize];
    let rows = 8.0;
    for y in 0..h {
        let band = (y as f32 / h as f32 * rows) as u32;
        for x in 0..w {
            let t = x as f32 / (w - 1).max(1) as f32;
            let g = |v: f32| (v * 255.0).round() as u8;
            let c: [u8; 3] = match band {
                // Dégradé de gris continu, puis en 16 paliers.
                0 => [g(t); 3],
                1 => [g((t * 16.0).floor().min(15.0) / 15.0); 3],
                // Dégradés rouge, vert, bleu.
                2 => [g(t), 0, 0],
                3 => [0, g(t), 0],
                4 => [0, 0, g(t)],
                // Noirs (0 à 15 %) à gauche, blancs (85 à 100 %) à droite : 10 cases chacun.
                5 => {
                    let k = (t * 20.0).floor().min(19.0);
                    if k < 10.0 {
                        [g(k * 0.015); 3]
                    } else {
                        [g(0.85 + (k - 10.0) * 0.015 + 0.015); 3]
                    }
                }
                // Damier de gamma : lignes noires et blanches contre le gris
                // 50 % de luminance (186 en sRGB) ; à bon gamma, ils se confondent de loin.
                6 => {
                    let k = (t * 6.0) as u32;
                    if k % 2 == 0 {
                        if y % 2 == 0 { [255; 3] } else { [0; 3] }
                    } else {
                        [186; 3]
                    }
                }
                // Primaires, secondaires et tons chair.
                _ => {
                    const P: [[u8; 3]; 12] = [
                        [255, 255, 255],
                        [255, 255, 0],
                        [0, 255, 255],
                        [0, 255, 0],
                        [255, 0, 255],
                        [255, 0, 0],
                        [0, 0, 255],
                        [0, 0, 0],
                        [255, 224, 196],
                        [234, 192, 160],
                        [198, 134, 98],
                        [141, 85, 60],
                    ];
                    P[((t * 12.0) as usize).min(11)]
                }
            };
            let i = ((y * w + x) * 4) as usize;
            px[i..i + 3].copy_from_slice(&c);
            px[i + 3] = 255;
        }
    }
    (w, h, px)
}

/// Courbes des trois canaux (entrée en abscisse, sortie en ordonnée), sur
/// fond sombre avec la diagonale neutre.
pub fn curves_image(a: &Adjust, cal: Option<&Cal>, w: u32, h: u32) -> Rgba {
    let mut px = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: i64, y: i64, c: [u8; 3]| {
        if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
            return;
        }
        let i = ((y as u32 * w + x as u32) * 4) as usize;
        for k in 0..3 {
            px[i + k] = px[i + k].max(c[k]);
        }
        px[i + 3] = 255;
    };
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let grid = x % (w as i64 / 4).max(1) == 0 || y % (h as i64 / 4).max(1) == 0;
            put(x, y, if grid { [44, 46, 58] } else { [22, 23, 30] });
        }
    }
    for x in 0..w as i64 {
        let y = h as i64 - 1 - x * (h as i64 - 1) / (w as i64 - 1).max(1);
        put(x, y, [90, 90, 100]);
    }
    const C: [[u8; 3]; 3] = [[255, 85, 85], [80, 250, 123], [98, 160, 255]];
    for ch in 0..3 {
        let mut prev: Option<i64> = None;
        for x in 0..w as i64 {
            let v = a.curve(ch, x as f32 / (w - 1).max(1) as f32, cal);
            let y = h as i64 - 1 - (v * (h - 1) as f32).round() as i64;
            let (lo, hi) = match prev {
                Some(p) => (p.min(y), p.max(y)),
                None => (y, y),
            };
            for yy in lo..=hi {
                put(x, yy, C[ch]);
                put(x, yy + 1, C[ch]);
            }
            prev = Some(y);
        }
    }
    (w, h, px)
}

/// Part de l'écran `mon` (x, y, largeur, hauteur en coordonnées globales) dans
/// une mire qui s'étale sur `all` (le rectangle de tous les écrans) : dégradé
/// de gris, arc-en-ciel et barres de couleur continus d'un écran à l'autre,
/// en quatre bandes sur la hauteur de chaque écran.
/// Image de `w`×`h` (affichée étirée à la taille de l'écran).
pub fn span_image(all: (f32, f32, f32, f32), mon: (f32, f32, f32, f32), w: u32, h: u32) -> Rgba {
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        // Bandes sur la hauteur de chaque écran (toutes visibles partout) ;
        // dégradés continus d'un écran à l'autre dans la largeur.
        let v = (y as f32 + 0.5) / h as f32;
        for x in 0..w {
            let gx = mon.0 + mon.2 * (x as f32 + 0.5) / w as f32;
            let t = ((gx - all.0) / all.2.max(1.0)).clamp(0.0, 1.0);
            let g = (t * 255.0).round() as u8;
            let c: [u8; 3] = if v < 0.25 {
                [g; 3]
            } else if v < 0.5 {
                hsv(t, 1.0, 1.0)
            } else if v < 0.75 {
                const B: [[u8; 3]; 8] = [
                    [255, 255, 255],
                    [255, 255, 0],
                    [0, 255, 255],
                    [0, 255, 0],
                    [255, 0, 255],
                    [255, 0, 0],
                    [0, 0, 255],
                    [128, 128, 128],
                ];
                B[((t * 8.0) as usize).min(7)]
            } else {
                // Tons chair et gris moyens en paliers.
                const S: [[u8; 3]; 8] = [
                    [255, 224, 196],
                    [234, 192, 160],
                    [198, 134, 98],
                    [141, 85, 60],
                    [64, 64, 64],
                    [118, 118, 118],
                    [186, 186, 186],
                    [230, 230, 230],
                ];
                S[((t * 8.0) as usize).min(7)]
            };
            let i = ((y * w + x) * 4) as usize;
            px[i..i + 3].copy_from_slice(&c);
            px[i + 3] = 255;
        }
    }
    (w, h, px)
}

fn is_cal(path: &str) -> bool {
    path.trim().to_lowercase().ends_with(".cal")
}

/// Courbes d'un fichier `.cal` d'ArgyllCMS : pour chaque entrée (0 … 1), la
/// sortie de chaque canal.
#[derive(Debug, Clone, PartialEq)]
pub struct Cal {
    input: Vec<f32>,
    out: [Vec<f32>; 3],
}

impl Cal {
    /// Sortie du canal `ch` pour l'entrée `x`, par interpolation linéaire.
    pub fn eval(&self, ch: usize, x: f32) -> f32 {
        let (i, o) = (&self.input, &self.out[ch]);
        if i.is_empty() {
            return x;
        }
        if x <= i[0] {
            return o[0];
        }
        for k in 1..i.len() {
            if x <= i[k] {
                let t = (x - i[k - 1]) / (i[k] - i[k - 1]).max(1e-9);
                return o[k - 1] + t * (o[k] - o[k - 1]);
            }
        }
        *o.last().unwrap_or(&x)
    }
}

/// Lit un `.cal` (format CGATS d'ArgyllCMS : champs RGB_I RGB_R RGB_G RGB_B).
pub fn parse_cal(src: &str) -> Result<Cal, String> {
    let mut fields: Vec<String> = Vec::new();
    let mut in_format = false;
    let mut in_data = false;
    let (mut input, mut r, mut g, mut b) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for line in src.lines() {
        let t = line.trim();
        match t {
            "BEGIN_DATA_FORMAT" => in_format = true,
            "END_DATA_FORMAT" => in_format = false,
            "BEGIN_DATA" => in_data = true,
            "END_DATA" => in_data = false,
            _ if in_format => fields.extend(t.split_whitespace().map(str::to_string)),
            _ if in_data => {
                let v: Vec<f32> = t.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                let at = |name: &str| fields.iter().position(|f| f == name).and_then(|i| v.get(i).copied());
                if let (Some(i), Some(rr), Some(gg), Some(bb)) =
                    (at("RGB_I"), at("RGB_R"), at("RGB_G"), at("RGB_B"))
                {
                    input.push(i);
                    r.push(rr);
                    g.push(gg);
                    b.push(bb);
                }
            }
            _ => {}
        }
    }
    if input.len() < 2 {
        return Err(crate::i18n::tr("fichier .cal sans courbes RGB_I / RGB_R / RGB_G / RGB_B").to_string());
    }
    Ok(Cal { input, out: [r, g, b] })
}

/// Lit le `.cal` d'un réglage (rien si ce n'en est pas un).
pub fn load_cal(a: &Adjust) -> Result<Option<Cal>, String> {
    if !is_cal(&a.calibration) {
        return Ok(None);
    }
    let path = crate::apptheme::expand_home(a.calibration.trim());
    let src = std::fs::read_to_string(&path).map_err(|e| format!("{path} : {e}"))?;
    parse_cal(&src).map(Some)
}

// ---------------------------------------------------------------------------
// Tenue des tables gamma (fil du démon)
// ---------------------------------------------------------------------------

/// Écran → réglages voulus.
pub type Wanted = BTreeMap<String, Adjust>;

/// Le fil qui tient les tables gamma ; `status` : écran → problème.
pub struct Gamma {
    tx: Option<mpsc::Sender<Wanted>>,
    status: Arc<Mutex<HashMap<String, String>>>,
}

impl Gamma {
    /// Sans fil (aperçu, essais).
    pub fn disabled() -> Self {
        Gamma {
            tx: None,
            status: Arc::default(),
        }
    }

    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Wanted>();
        let status: Arc<Mutex<HashMap<String, String>>> = Arc::default();
        let st = status.clone();
        let spawned = std::thread::Builder::new()
            .name("hyprdesk-couleurs".into())
            .spawn(move || {
                if let Err(e) = run(rx, st.clone()) {
                    tracing::warn!("{}", crate::tr!("couleurs des écrans : {e}", e = e));
                    if let Ok(mut s) = st.lock() {
                        s.insert(String::new(), e);
                    }
                }
            });
        Gamma {
            tx: spawned.is_ok().then_some(tx),
            status,
        }
    }

    /// Réglages voulus (tous les écrans) : appliqués aussitôt.
    pub fn set(&self, wanted: Wanted) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(wanted);
        }
    }

    /// Problèmes en cours : écran → message (clé vide : le service lui-même).
    pub fn status(&self) -> HashMap<String, String> {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

struct Out {
    id: u32,
    output: wl_output::WlOutput,
    name: Option<String>,
    control: Option<ZwlrGammaControlV1>,
    size: Option<u32>,
    failed: bool,
    /// Table posée en dernier (pour ne pas la reposer à l'identique).
    applied: Option<Adjust>,
}

struct State {
    manager: Option<ZwlrGammaControlManagerV1>,
    outs: Vec<Out>,
    wanted: Wanted,
    status: Arc<Mutex<HashMap<String, String>>>,
}

impl State {
    fn report(&self, monitor: &str, problem: Option<String>) {
        if let Ok(mut s) = self.status.lock() {
            match problem {
                Some(p) => {
                    s.insert(monitor.to_string(), p);
                }
                None => {
                    s.remove(monitor);
                }
            }
        }
    }

    /// Pose, change ou rend la table de chaque écran nommé.
    fn apply(&mut self, qh: &QueueHandle<State>) {
        let manager = self.manager.clone();
        let mut reports = Vec::new();
        for o in &mut self.outs {
            let Some(name) = o.name.clone() else { continue };
            let want = self.wanted.get(&name).filter(|a| a.needs_table()).cloned();
            match want {
                None => {
                    // Neutre : la table d'origine revient en lâchant la nôtre.
                    if let Some(c) = o.control.take() {
                        c.destroy();
                        tracing::info!("{}", crate::tr!("couleurs de {m} : table d'origine rendue", m = name));
                    }
                    o.size = None;
                    o.applied = None;
                    o.failed = false;
                    reports.push((name, None));
                }
                Some(a) => {
                    if o.failed {
                        continue;
                    }
                    if o.control.is_none()
                        && let Some(m) = &manager
                    {
                        o.control = Some(m.get_gamma_control(&o.output, qh, o.id));
                    }
                    if let (Some(c), Some(size)) = (&o.control, o.size)
                        && o.applied.as_ref() != Some(&a)
                    {
                        let r = load_cal(&a).and_then(|cal| set_table(c, size, &a, cal.as_ref()));
                        match &r {
                            Ok(()) => {
                                tracing::info!("{}", crate::tr!("couleurs de {m} : table gamma posée ({n} entrées)", m = name, n = size));
                                o.applied = Some(a);
                            }
                            Err(e) => tracing::warn!("{}", crate::tr!("couleurs de {m} : {e}", m = name, e = e)),
                        }
                        reports.push((name, r.err()));
                    }
                }
            }
        }
        for (n, p) in reports {
            self.report(&n, p);
        }
    }
}

/// Écrit la table dans un fichier et la donne à Hyprland.
fn set_table(c: &ZwlrGammaControlV1, size: u32, a: &Adjust, cal: Option<&Cal>) -> Result<(), String> {
    use std::io::{Seek, Write};
    let [r, g, b] = a.ramps(size as usize, cal);
    let mut bytes = Vec::with_capacity(size as usize * 6);
    for v in r.iter().chain(&g).chain(&b) {
        bytes.extend_from_slice(&v.to_ne_bytes());
    }
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = dir.join(format!("hyprdesk-gamma-{}", std::process::id()));
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&path);
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    f.seek(std::io::SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    c.set_gamma(f.as_fd());
    Ok(())
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == "wl_output" {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(name, version.min(4), qh, name);
                    state.outs.push(Out {
                        id: name,
                        output,
                        name: None,
                        control: None,
                        size: None,
                        failed: false,
                        applied: None,
                    });
                } else if interface == "zwlr_gamma_control_manager_v1" {
                    state.manager = Some(registry.bind(name, 1, qh, ()));
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(i) = state.outs.iter().position(|o| o.id == name) {
                    let o = state.outs.remove(i);
                    if let Some(c) = o.control {
                        c.destroy();
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for State {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        id: &u32,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            if let Some(o) = state.outs.iter_mut().find(|o| o.id == *id) {
                o.name = Some(name);
            }
            state.apply(qh);
        }
    }
}

impl Dispatch<ZwlrGammaControlManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ZwlrGammaControlManagerV1,
        _: <ZwlrGammaControlManagerV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrGammaControlV1, u32> for State {
    fn event(
        state: &mut Self,
        _: &ZwlrGammaControlV1,
        event: zwlr_gamma_control_v1::Event,
        id: &u32,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let Some(o) = state.outs.iter_mut().find(|o| o.id == *id) else {
            return;
        };
        match event {
            zwlr_gamma_control_v1::Event::GammaSize { size } => {
                o.size = Some(size);
                o.applied = None;
                state.apply(qh);
            }
            zwlr_gamma_control_v1::Event::Failed => {
                if let Some(c) = o.control.take() {
                    c.destroy();
                }
                o.failed = true;
                o.size = None;
                let name = o.name.clone().unwrap_or_default();
                tracing::warn!("{}", crate::tr!("couleurs de {m} : table gamma refusée (déjà tenue par un autre programme)", m = name));
                state.report(
                    &name,
                    Some(crate::i18n::tr("un autre programme tient déjà les couleurs de cet écran (wlsunset, gammastep…)").to_string()),
                );
            }
            _ => {}
        }
    }
}

/// Boucle du fil : connexion Wayland à part, réglages reçus par `rx`.
fn run(rx: mpsc::Receiver<Wanted>, status: Arc<Mutex<HashMap<String, String>>>) -> Result<(), String> {
    use std::os::fd::AsRawFd;
    let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let mut queue = conn.new_event_queue::<State>();
    let qh = queue.handle();
    conn.display().get_registry(&qh, ());
    let mut state = State {
        manager: None,
        outs: Vec::new(),
        wanted: Wanted::new(),
        status,
    };
    queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
    queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
    if state.manager.is_none() {
        return Err(crate::i18n::tr("Hyprland ne propose pas wlr-gamma-control").to_string());
    }
    loop {
        // Derniers réglages reçus (les précédents sont dépassés) ; fil arrêté
        // avec le démon.
        let mut latest = None;
        loop {
            match rx.try_recv() {
                Ok(w) => latest = Some(w),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            }
        }
        if let Some(w) = latest {
            state.wanted = w;
            // Un écran en échec réessaie quand on change ses réglages.
            for o in &mut state.outs {
                o.failed = false;
            }
            state.apply(&qh);
        }
        let _ = conn.flush();
        if let Some(guard) = queue.prepare_read() {
            let mut pfd = libc::pollfd {
                fd: guard.connection_fd().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY : un seul descripteur, valide tant que `guard` vit.
            let ready = unsafe { libc::poll(&mut pfd, 1, 120) };
            if ready > 0 {
                guard.read().map_err(|e| e.to_string())?;
            }
        }
        queue.dispatch_pending(&mut state).map_err(|e| e.to_string())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutre_et_reglages() {
        let a = Adjust::default();
        assert!(!a.needs_table());
        let [r, _, b] = a.ramps(256, None);
        assert_eq!((r[0], r[255], r[128]), (0, 65535, (128.0 / 255.0 * 65535.0f32).round() as u16));
        let mut c = a.clone();
        assert!(c.set("blue", 0.8) && c.needs_table());
        assert!(!c.set("inconnu", 1.0));
        assert!(c.set("gamma", 9.0) && c.gamma == 2.5, "borné");
        let [_, _, b2] = c.ramps(256, None);
        assert!(b2[255] < b[255], "bleu atténué");
        let mut d = Adjust::default();
        d.set("brightness", 0.2);
        assert!(d.ramps(16, None)[0][0] > 0, "noir relevé");
    }

    #[test]
    fn images_de_test() {
        let (w, h, px) = reference_image(320, 160);
        assert_eq!(px.len(), (w * h * 4) as usize);
        assert_eq!(&px[0..4], &[0, 0, 0, 255], "dégradé de gris : noir à gauche");
        let (_, _, c) = curves_image(&Adjust::default(), None, 64, 40);
        assert_eq!(c.len(), 64 * 40 * 4);
        // Deux écrans côte à côte : le second commence où finit le premier.
        let all = (0.0, 0.0, 200.0, 100.0);
        let (_, _, a) = span_image(all, (0.0, 0.0, 100.0, 100.0), 10, 4);
        let (_, _, b) = span_image(all, (100.0, 0.0, 100.0, 100.0), 10, 4);
        assert!(a[(9 * 4) as usize] < b[0], "gris continu d'un écran à l'autre");
        let mut g = Adjust::default();
        assert!(g.is_default());
        g.set("gamma_red", 1.4);
        assert!(g.curve(0, 0.5, None) > g.curve(1, 0.5, None), "courbe du rouge relevée");
    }

    #[test]
    fn fichier_cal_d_argyll() {
        let src = "CAL\nNUMBER_OF_FIELDS 4\nBEGIN_DATA_FORMAT\nRGB_I RGB_R RGB_G RGB_B\nEND_DATA_FORMAT\nNUMBER_OF_SETS 3\nBEGIN_DATA\n0.0 0.0 0.0 0.0\n0.5 0.4 0.5 0.6\n1.0 0.9 1.0 1.0\nEND_DATA\n";
        let cal = parse_cal(src).unwrap();
        assert!((cal.eval(0, 0.5) - 0.4).abs() < 1e-6);
        assert!((cal.eval(0, 0.25) - 0.2).abs() < 1e-6);
        assert!((cal.eval(2, 1.0) - 1.0).abs() < 1e-6);
        let mut a = Adjust::default();
        a.calibration = "/x/ecran.cal".into();
        assert!(a.needs_table() && a.icc().is_none());
        a.calibration = "/x/ecran.ICC".into();
        assert_eq!(a.icc(), Some("/x/ecran.ICC"));
        assert!(parse_cal("rien").is_err());
    }
}
