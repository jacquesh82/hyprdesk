//! Fond d'écran (MLG-280) : cadrage (point focal, zoom) et traitement de
//! l'image (luminosité, saturation, contraste, vignette), appliqués au
//! décodage, hors du fil de l'interface. Les réglages viennent du registre
//! `design_options` (`wallpaper.*`), posés à chaque config installée.

use std::sync::RwLock;

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

/// Réglages du traitement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Treatment {
    pub focal: (f32, f32),
    pub zoom: f32,
    pub brightness: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub vignette: f32,
    /// Fond vidéo : vitesse et cadence.
    pub speed: f32,
    pub fps: u32,
}

impl Default for Treatment {
    fn default() -> Self {
        Self {
            focal: (0.5, 0.5),
            zoom: 1.0,
            brightness: 0.0,
            saturation: 1.0,
            contrast: 1.0,
            vignette: 0.0,
            speed: 1.0,
            fps: 24,
        }
    }
}

impl Treatment {
    /// Réglages lus dans la config.
    pub fn of(g: &crate::config::General) -> Self {
        use crate::design_options::num;
        Self {
            focal: (
                num(g, "wallpaper.focal_x") as f32,
                num(g, "wallpaper.focal_y") as f32,
            ),
            zoom: num(g, "wallpaper.zoom") as f32,
            brightness: num(g, "wallpaper.brightness") as f32,
            saturation: num(g, "wallpaper.saturation") as f32,
            contrast: num(g, "wallpaper.contrast") as f32,
            vignette: num(g, "wallpaper.vignette") as f32,
            speed: num(g, "wallpaper.speed") as f32,
            fps: num(g, "wallpaper.fps") as u32,
        }
    }

    /// Rien à faire sur les couleurs.
    fn neutral_colors(&self) -> bool {
        self.brightness.abs() < 1e-3
            && (self.saturation - 1.0).abs() < 1e-3
            && (self.contrast - 1.0).abs() < 1e-3
            && self.vignette < 1e-3
    }
}

static CURRENT: RwLock<Option<Treatment>> = RwLock::new(None);

/// Pose les réglages ; `true` s'ils ont changé (fonds à redécoder).
pub fn set(t: Treatment) -> bool {
    let Ok(mut w) = CURRENT.write() else {
        return false;
    };
    let changed = w.is_some_and(|old| old != t);
    *w = Some(t);
    changed
}

/// Réglages en place.
pub fn current() -> Treatment {
    CURRENT.read().ok().and_then(|t| *t).unwrap_or_default()
}

/// Échelle de remplissage avec le zoom réglé (ajustement « remplir »).
pub fn cover_scale(base: f32) -> f32 {
    base * current().zoom.max(1.0)
}

/// Recadre une image agrandie à `w`×`h` autour du point focal.
pub fn crop_focal(img: DynamicImage, w: u32, h: u32) -> DynamicImage {
    let (iw, ih) = img.dimensions();
    if iw <= w && ih <= h {
        return img;
    }
    let (fx, fy) = current().focal;
    let x = ((iw.saturating_sub(w)) as f32 * fx.clamp(0.0, 1.0)).round() as u32;
    let y = ((ih.saturating_sub(h)) as f32 * fy.clamp(0.0, 1.0)).round() as u32;
    img.crop_imm(x, y, w.min(iw), h.min(ih))
}

/// Luminosité, saturation, contraste et vignette.
pub fn grade(img: RgbaImage) -> RgbaImage {
    let t = current();
    if t.neutral_colors() {
        return img;
    }
    let (w, h) = img.dimensions();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let rmax = (cx * cx + cy * cy).sqrt().max(1.0);
    let mut out = img;
    for (x, y, p) in out.enumerate_pixels_mut() {
        let Rgba([r, g, b, a]) = *p;
        let mut c = [
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
        ];
        let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        for v in &mut c {
            *v = l + (*v - l) * t.saturation;
            *v = (*v - 0.5) * t.contrast + 0.5 + t.brightness;
        }
        if t.vignette > 0.0 {
            let d =
                (((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / rmax).clamp(0.0, 1.0);
            let k = 1.0 - t.vignette * d * d;
            for v in &mut c {
                *v *= k;
            }
        }
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        *p = Rgba([q(c[0]), q(c[1]), q(c[2]), a]);
    }
    out
}

/// Couleur moyenne d'une image (teinte du verre par le fond).
pub fn average(img: &RgbaImage) -> crate::config::Rgba {
    let small = image::imageops::thumbnail(img, 4, 4);
    let n = (small.width() * small.height()).max(1);
    let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
    for p in small.pixels() {
        r += u32::from(p[0]);
        g += u32::from(p[1]);
        b += u32::from(p[2]);
    }
    crate::config::Rgba((r / n) as u8, (g / n) as u8, (b / n) as u8, 255)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traitement_et_cadrage() {
        let img = RgbaImage::from_pixel(10, 10, Rgba([100, 150, 200, 255]));
        assert_eq!(grade(img.clone()), img, "neutre : intacte");
        set(Treatment {
            brightness: 0.2,
            saturation: 0.0,
            ..Default::default()
        });
        let g = grade(img);
        let p = g.get_pixel(5, 5);
        assert!(p[0] == p[1] && p[1] == p[2], "désaturée : {p:?}");
        let big = DynamicImage::ImageRgba8(RgbaImage::new(200, 100));
        set(Treatment {
            focal: (1.0, 0.0),
            ..Default::default()
        });
        let c = crop_focal(big, 100, 100);
        assert_eq!(c.dimensions(), (100, 100));
        set(Treatment::default());
    }
}
