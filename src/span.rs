//! Scène étalée sur plusieurs écrans : découpe automatique d'une image selon
//! la disposition réelle des écrans.
//!
//! Le **canevas** est le rectangle qui englobe tous les écrans, dans les
//! coordonnées logiques de Hyprland (position, taille après échelle et
//! rotation). L'image le couvre, centrée, sans déformation (comme `cover`) ;
//! chaque écran en montre la part qui se trouve sous lui. Une disposition en
//! L, empilée ou décalée donne donc une scène continue, et les vides du
//! canevas (sous un écran portrait plus court, par exemple) sont simplement
//! des parts de l'image que personne ne voit.
//!
//! Le même calcul sert au bureau (`ui::decode_wall`) et à `hyprdesk
//! wallpaper split`, qui écrit une image par écran (pour le verrouillage,
//! un autre outil de fond, ou un pack).

use std::path::{Path, PathBuf};

/// Rectangle en coordonnées logiques.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Place d'un écran dans le canevas : décalage dans le canevas, taille du canevas.
/// En entiers, pour servir de clé de cache.
pub type Placement = (i32, i32, u32, u32);

/// Rectangle englobant des écrans (`None` sans écran).
pub fn canvas(screens: &[Rect]) -> Option<Rect> {
    let first = screens.first()?;
    let (mut x0, mut y0) = (first.x, first.y);
    let (mut x1, mut y1) = (first.x + first.w, first.y + first.h);
    for s in &screens[1..] {
        x0 = x0.min(s.x);
        y0 = y0.min(s.y);
        x1 = x1.max(s.x + s.w);
        y1 = y1.max(s.y + s.h);
    }
    Some(Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    })
}

/// Place de l'écran `screen` dans le canevas `c`.
pub fn placement(c: Rect, screen: Rect) -> Placement {
    (
        (screen.x - c.x).round() as i32,
        (screen.y - c.y).round() as i32,
        c.w.round().max(1.0) as u32,
        c.h.round().max(1.0) as u32,
    )
}

/// Part d'une image `iw × ih` que montre un écran `sw × sh` placé en `p` :
/// (x, y, largeur, hauteur) en pixels de l'image, bornés à l'image.
pub fn crop(iw: u32, ih: u32, sw: u32, sh: u32, p: Placement) -> (u32, u32, u32, u32) {
    let (ox, oy, cw, ch) = p;
    let (iw_f, ih_f) = (iw as f32, ih as f32);
    // L'image couvre le canevas : échelle image → canevas, débord centré.
    let s = (cw as f32 / iw_f).max(ch as f32 / ih_f);
    let (dx, dy) = ((iw_f * s - cw as f32) / 2.0, (ih_f * s - ch as f32) / 2.0);
    let x = ((ox as f32 + dx) / s).clamp(0.0, iw_f - 1.0);
    let y = ((oy as f32 + dy) / s).clamp(0.0, ih_f - 1.0);
    let w = (sw as f32 / s).clamp(1.0, iw_f - x);
    let h = (sh as f32 / s).clamp(1.0, ih_f - y);
    (
        x.round() as u32,
        y.round() as u32,
        w.round().max(1.0) as u32,
        h.round().max(1.0) as u32,
    )
}

/// Découpe `img` pour un écran `sw × sh` placé en `p`, à sa taille.
pub fn cut(img: &image::DynamicImage, sw: u32, sh: u32, p: Placement) -> image::DynamicImage {
    let (x, y, w, h) = crop(img.width(), img.height(), sw, sh, p);
    img.crop_imm(x, y, w, h).resize_exact(
        sw.max(1),
        sh.max(1),
        image::imageops::FilterType::Triangle,
    )
}

/// Agrandissement qu'impose une image `iw × ih` : pour couvrir chaque écran
/// (le pire des écrans), ou toute la disposition si elle est étalée. Au-delà
/// de 1, l'image est agrandie et perd en netteté.
pub fn upscale(iw: u32, ih: u32, screens: &[Rect], span: bool) -> f32 {
    let (iw, ih) = (iw.max(1) as f32, ih.max(1) as f32);
    let cover = |r: &Rect| (r.w / iw).max(r.h / ih);
    if span {
        canvas(screens).map(|c| cover(&c)).unwrap_or(0.0)
    } else {
        screens.iter().map(cover).fold(0.0, f32::max)
    }
}

/// Seuil d'avertissement : une image agrandie de plus de 10 % se voit.
pub const UPSCALE_WARN: f32 = 1.1;

/// Avertissement (non bloquant) si l'image est trop petite pour ces écrans.
pub fn resolution_warning(iw: u32, ih: u32, screens: &[Rect], span: bool) -> Option<String> {
    let f = upscale(iw, ih, screens, span);
    if f <= UPSCALE_WARN {
        return None;
    }
    let (nw, nh) = ((iw as f32 * f).ceil() as u32, (ih as f32 * f).ceil() as u32);
    let target = if span {
        crate::i18n::tr("l'ensemble des écrans")
    } else if screens.len() > 1 {
        crate::i18n::tr("le plus grand écran")
    } else {
        crate::i18n::tr("l'écran")
    };
    Some(crate::tr!(
        "Image {iw}×{ih} : agrandie ×{f} pour couvrir {target} — elle paraîtra floue. \
         Au moins {nw}×{nh} pour rester nette.",
        iw = iw,
        ih = ih,
        f = format!("{f:.1}"),
        target = target,
        nw = nw,
        nh = nh
    ))
}

/// Un écran pour la découpe : son nom, son rectangle logique, son échelle.
#[derive(Debug, Clone)]
pub struct Screen {
    pub name: String,
    pub rect: Rect,
    pub scale: f32,
}

/// Rectangle en pixels physiques (pour juger de la résolution d'une image).
pub fn physical(r: Rect, scale: f32) -> Rect {
    Rect {
        x: r.x * scale,
        y: r.y * scale,
        w: r.w * scale,
        h: r.h * scale,
    }
}

/// Écrans actifs, d'après Hyprland.
pub fn screens() -> Result<Vec<Screen>, String> {
    Ok(crate::hypr::monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| Screen {
            name: m.name,
            rect: Rect {
                x: m.x,
                y: m.y,
                w: m.width as f32,
                h: m.height as f32,
            },
            scale: m.scale,
        })
        .collect())
}

/// Écrit une image par écran dans `out` (`<écran>.png`) ; `scale` multiplie la
/// taille logique (2 : pour un écran HiDPI à l'échelle 2).
pub fn split(
    image: &Path,
    screens: &[Screen],
    out: &Path,
    scale: f32,
) -> Result<Vec<PathBuf>, String> {
    let img = image::open(image).map_err(|e| format!("{} : {e}", image.display()))?;
    let rects: Vec<Rect> = screens.iter().map(|s| s.rect).collect();
    let c = canvas(&rects).ok_or(crate::i18n::tr("aucun écran"))?;
    std::fs::create_dir_all(out).map_err(|e| format!("{} : {e}", out.display()))?;
    let scaled = |r: Rect| Rect {
        x: r.x * scale,
        y: r.y * scale,
        w: r.w * scale,
        h: r.h * scale,
    };
    let c = scaled(c);
    let mut written = Vec::new();
    for s in screens {
        let r = scaled(s.rect);
        let part = cut(
            &img,
            r.w.round() as u32,
            r.h.round() as u32,
            placement(c, r),
        );
        let file = out.join(format!("{}.png", s.name));
        part.save(&file)
            .map_err(|e| format!("{} : {e}", file.display()))?;
        written.push(file);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn deux_ecrans_cote_a_cote() {
        // Deux 1920×1080 côte à côte : canevas 3840×1080 ; image 3840×1080.
        let (a, b) = (r(0.0, 0.0, 1920.0, 1080.0), r(1920.0, 0.0, 1920.0, 1080.0));
        let c = canvas(&[a, b]).unwrap();
        assert_eq!(c, r(0.0, 0.0, 3840.0, 1080.0));
        assert_eq!(
            crop(3840, 1080, 1920, 1080, placement(c, a)),
            (0, 0, 1920, 1080)
        );
        assert_eq!(
            crop(3840, 1080, 1920, 1080, placement(c, b)),
            (1920, 0, 1920, 1080)
        );
    }

    #[test]
    fn paysage_et_portrait_decales() {
        // Paysage 1920×1080 en (0, 420), portrait 1080×1920 à droite en (1920, 0) :
        // canevas 3000×1920. Image 4200×2600 (scène maîtresse) : échelle 0,738.
        let (land, port) = (
            r(0.0, 420.0, 1920.0, 1080.0),
            r(1920.0, 0.0, 1080.0, 1920.0),
        );
        let c = canvas(&[land, port]).unwrap();
        assert_eq!(c, r(0.0, 0.0, 3000.0, 1920.0));
        let (x1, y1, w1, h1) = crop(4200, 2600, 1920, 1080, placement(c, land));
        let (x2, y2, w2, h2) = crop(4200, 2600, 1080, 1920, placement(c, port));
        // Les deux parts sont contiguës dans l'image, et à la même échelle.
        assert_eq!(x1 + w1, x2);
        assert!((w1 as f32 / 1920.0 - h2 as f32 / 1920.0).abs() < 0.01);
        // Le paysage est plus bas que le haut du portrait, d'autant que dans le canevas.
        assert!(y1 > y2 && h1 < h2);
        assert!((x2 + w2) as i32 <= 4200);
    }

    #[test]
    fn avertir_si_trop_petite() {
        let screens = [r(0.0, 0.0, 1920.0, 1080.0), r(1920.0, 0.0, 1920.0, 1080.0)];
        // Assez grande pour un écran, pas pour la scène étalée.
        assert!(resolution_warning(1920, 1080, &screens, false).is_none());
        let w = resolution_warning(1920, 1080, &screens, true).unwrap();
        assert!(w.contains("×2.0") && w.contains("3840×2160"), "{w}");
        assert!(resolution_warning(1672, 941, &screens[..1], false).is_some());
        assert!(resolution_warning(4200, 2600, &screens, true).is_none());
    }

    #[test]
    fn decoupe_a_la_taille_de_l_ecran() {
        let img = image::DynamicImage::new_rgb8(400, 100);
        let c = r(0.0, 0.0, 400.0, 100.0);
        let part = cut(&img, 200, 100, placement(c, r(200.0, 0.0, 200.0, 100.0)));
        assert_eq!((part.width(), part.height()), (200, 100));
    }
}
