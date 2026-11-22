//! Palette depuis le fond d'écran (§17.2) : dominantes en OKLab par
//! k-moyennes déterministe, accent choisi selon le schéma, palette construite
//! avec le même moteur d'intention et son contraste garanti — le superset de
//! matugen, sans outil externe.

use std::path::Path;

use crate::config::Rgba;

use super::color::{self, Oklch};
use super::intent::{self, Contrast, Intent, Mood};
use super::resolve;
use super::{Error, Result};

/// Schéma de dérivation de l'accent depuis les dominantes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// Dominante la plus vivante (chroma × couverture).
    Vibrant,
    /// Dominante la plus couvrante (tonal, façon material).
    Material,
    /// Dominante douce : chroma plafonné.
    Pastel,
}

impl Scheme {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "vibrant" => Self::Vibrant,
            "material" => Self::Material,
            "pastel" => Self::Pastel,
            _ => return None,
        })
    }

    fn mood(self) -> Mood {
        match self {
            Self::Vibrant | Self::Material => Mood::Vif,
            Self::Pastel => Mood::Calme,
        }
    }
}

/// Limite de taille du fichier d'image (§14 : décodage borné).
const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024;

/// Nombre de clusters k-moyennes et d'itérations (déterministe).
const K: usize = 6;
const ITERATIONS: usize = 14;

/// Un cluster de dominantes : moyenne en OKLab, couverture (0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cluster {
    l: f64,
    a: f64,
    b: f64,
    coverage: f64,
}

impl Cluster {
    fn chroma(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    fn hue(&self) -> f64 {
        self.b.atan2(self.a).to_degrees().rem_euclid(360.0)
    }
}

/// Palette résolue depuis une image : dominantes, accent selon le schéma,
/// fond sombre ou clair, verrou de teinte optionnel (degrés).
pub fn from_image(
    image: &Path,
    variant: &str,
    scheme: &Scheme,
    hue_lock: Option<f64>,
) -> Result<resolve::Resolved> {
    let meta = std::fs::metadata(image)
        .map_err(|e| Error::Manifest("image".to_string(), e.to_string()))?;
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(Error::Manifest(
            "image".to_string(),
            format!("trop grande : {} > {MAX_IMAGE_BYTES} octets", meta.len()),
        ));
    }
    let reader = image::ImageReader::open(image)
        .and_then(|r| r.with_guessed_format())
        .map_err(|e| Error::Manifest("image".to_string(), e.to_string()))?;
    let img = reader
        .decode()
        .map_err(|e| Error::Manifest("image".to_string(), format!("décode : {e}")))?;
    // Échantillon réduit : assez pour les dominantes, mémoire bornée.
    let small = img.thumbnail(96, 96).to_rgb8();
    let pixels: Vec<[f64; 3]> = small
        .pixels()
        .map(|p| {
            let c = color::to_oklch(Rgba(p[0], p[1], p[2], 255));
            let a = c.c * c.h.to_radians().cos();
            let b = c.c * c.h.to_radians().sin();
            [c.l, a, b]
        })
        .collect();
    if pixels.is_empty() {
        return Err(Error::Manifest(
            "image".to_string(),
            "image vide".to_string(),
        ));
    }
    let clusters = kmeans(&pixels);
    // Accent : score selon le schéma.
    let accent_cluster = clusters
        .iter()
        .max_by(|x, y| {
            score(x, scheme)
                .partial_cmp(&score(y, scheme))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap_or(Cluster {
            l: 0.5,
            a: 0.0,
            b: 0.0,
            coverage: 1.0,
        });
    let hue = hue_lock.unwrap_or(accent_cluster.hue());
    // Fond : le cluster extrême (sombre ou clair), chroma contenu.
    let fond = clusters
        .iter()
        .max_by(|x, y| {
            let (xl, yl) = (x.l, y.l);
            if variant == "light" {
                xl.partial_cmp(&yl).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                yl.partial_cmp(&xl).unwrap_or(std::cmp::Ordering::Equal)
            }
        })
        .copied()
        .unwrap_or(accent_cluster);
    let bg_l = if variant == "light" {
        fond.l.clamp(0.86, 0.97)
    } else {
        fond.l.clamp(0.08, 0.20)
    };
    let bg = color::from_oklch(Oklch {
        l: bg_l,
        c: fond.chroma().clamp(0.0, 0.05),
        h: hue,
    });
    // L'intention fait le reste (mood du schéma, contraste aa, accent dérivée
    // du fond si verrou absent).
    let accent = accent_color(&accent_cluster, &bg, hue, *scheme);
    let i = Intent {
        name: image
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "image".to_string()),
        variant: variant.to_string(),
        hue_degrees: hue,
        graphite: accent_cluster.chroma() < 0.02,
        mood: scheme.mood(),
        contrast: Contrast::Aa,
        accent: Some(accent),
        inherit: Some("hyprdesk-base".to_string()),
    };
    let mut r = i.build();
    // Le fond de l'image parle plus fort que les défauts d'ambiance.
    r.palette.insert("bg".to_string(), bg);
    r.palette.insert(
        "background".to_string(),
        color::darken(bg, if variant == "light" { -4.0 } else { 4.0 }),
    );
    // Re-garantie de contraste sur le fond retenu.
    let min = Contrast::Aa.min();
    r.palette.insert(
        "fg".to_string(),
        color::ensure_contrast(r.palette["fg"], bg, min),
    );
    Ok(r)
}

fn score(c: &Cluster, scheme: &Scheme) -> f64 {
    let chroma = c.chroma();
    match scheme {
        Scheme::Vibrant => chroma * c.coverage.sqrt(),
        Scheme::Material => c.coverage,
        Scheme::Pastel => c.coverage / (1.0 + chroma * 8.0),
    }
}

/// Accent dérivé du cluster : plus clair que le fond sombre, plus sombre que
/// le fond clair, chroma selon le schéma.
fn accent_color(cluster: &Cluster, bg: &Rgba, hue: f64, scheme: Scheme) -> Rgba {
    let bg_l = color::to_oklch(*bg).l;
    let dark_bg = bg_l < 0.5;
    let mut l = if dark_bg { 0.72 } else { 0.52 };
    let mut chroma = match scheme {
        Scheme::Vibrant => cluster.chroma().clamp(0.09, 0.17),
        Scheme::Material => cluster.chroma().clamp(0.07, 0.13),
        Scheme::Pastel => cluster.chroma().clamp(0.03, 0.09),
    };
    if dark_bg && l <= bg_l + 0.1 {
        l = (bg_l + 0.35).clamp(0.0, 0.9);
    }
    if !dark_bg && l >= bg_l - 0.1 {
        l = (bg_l - 0.3).clamp(0.1, 1.0);
    }
    if chroma < 0.02 {
        // Image presque grise : accent teinté discret à la teinte retenue.
        chroma = 0.04;
    }
    color::from_oklch(Oklch {
        l,
        c: chroma,
        h: hue,
    })
}

/// K-moyennes déterministes : graines réparties par luminosité triée,
/// convergence par itérations bornées, sans aléatoire.
fn kmeans(pixels: &[[f64; 3]]) -> Vec<Cluster> {
    let mut sorted: Vec<&[f64; 3]> = pixels.iter().collect();
    sorted.sort_by(|x, y| x[0].partial_cmp(&y[0]).unwrap_or(std::cmp::Ordering::Equal));
    let mut centers: Vec<[f64; 3]> = (0..K)
        .filter_map(|i| {
            let idx = (i + 1) * sorted.len() / (K + 1);
            sorted.get(idx).map(|p| **p)
        })
        .collect();
    if centers.is_empty() {
        return vec![Cluster {
            l: 0.5,
            a: 0.0,
            b: 0.0,
            coverage: 1.0,
        }];
    }
    for _ in 0..ITERATIONS {
        let mut sums = vec![[0.0f64; 3]; centers.len()];
        let mut counts = vec![0usize; centers.len()];
        for p in pixels {
            let (best, _) = centers
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let d = (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2);
                    (i, d)
                })
                .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or((0, 0.0));
            for dim in 0..3 {
                sums[best][dim] += p[dim];
            }
            counts[best] += 1;
        }
        for (i, c) in centers.iter_mut().enumerate() {
            if counts[i] > 0 {
                for dim in 0..3 {
                    c[dim] = sums[i][dim] / counts[i] as f64;
                }
            }
        }
    }
    let n = pixels.len() as f64;
    centers
        .iter()
        .map(|c| Cluster {
            l: c[0],
            a: c[1],
            b: c[2],
            coverage: 0.0,
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|mut cl| {
            let count = pixels
                .iter()
                .filter(|p| {
                    let d = (p[0] - cl.l).powi(2) + (p[1] - cl.a).powi(2) + (p[2] - cl.b).powi(2);
                    d < 0.02
                })
                .count();
            cl.coverage = (count as f64 / n).max(1.0 / n);
            cl
        })
        .collect()
}

/// Identifiant du pack image, partagé avec la CLI.
pub fn pack_id_for(image: &Path) -> String {
    intent::pack_id(
        &image
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "image".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PNG généré à la volée (image crate, encodeur sans écriture disque) :
    /// deux moitiés de couleurs franches + une bande grise.
    fn image_pixels() -> image::RgbImage {
        let mut img = image::RgbImage::new(96, 96);
        for (x, y, p) in img.enumerate_pixels_mut() {
            if y < 48 {
                if x < 48 {
                    *p = image::Rgb([20, 30, 80]);
                } else {
                    *p = image::Rgb([200, 40, 40]);
                }
            } else if x < 48 {
                *p = image::Rgb([10, 12, 16]);
            } else {
                *p = image::Rgb([128, 128, 128]);
            }
        }
        img
    }

    fn write_fixture(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        // Petit dossier d'essai par étiquette : les tests tournent en
        // parallèle dans le même processus (même pid).
        let dir = std::env::temp_dir().join(format!("hd-recolor-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fond.png");
        image_pixels().save(&path).expect("écriture du PNG de test");
        (dir, path)
    }

    #[test]
    fn dominantes_en_palette_contrastée() {
        let (_dir, path) = write_fixture("sombre");
        let r = from_image(&path, "dark", &Scheme::Vibrant, None).unwrap();
        let ratio = color::contrast(r.palette["fg"], r.palette["bg"]);
        assert!(ratio >= 4.5, "fg/bg : {ratio}");
        // L'accent est vivant (image franche) : chroma notable.
        let o = color::to_oklch(r.palette["accent"]);
        assert!(o.c > 0.05, "accent : {}", o.c);
        assert!(r.warnings.is_empty());
        let _ = std::fs::remove_dir_all(_dir);
    }

    #[test]
    fn verrou_de_teinte_respecté() {
        let (_dir, path) = write_fixture("verrou");
        let r = from_image(&path, "dark", &Scheme::Pastel, Some(130.0)).unwrap();
        let o = color::to_oklch(r.palette["accent"]);
        assert!((o.h - 130.0).abs() < 5.0 || o.c < 0.05, "teinte : {}", o.h);
        let _ = std::fs::remove_dir_all(_dir);
    }

    #[test]
    fn variante_claire_et_schémas() {
        let (_dir, path) = write_fixture("clair");
        for scheme in [Scheme::Vibrant, Scheme::Material, Scheme::Pastel] {
            let r = from_image(&path, "light", &scheme, None).unwrap();
            let l = color::to_oklch(r.palette["bg"]).l;
            assert!(l > 0.7, "fond clair attendu : {l}");
            assert!(color::contrast(r.palette["fg"], r.palette["bg"]) >= 4.5);
        }
        let _ = std::fs::remove_dir_all(_dir);
    }

    #[test]
    fn image_absente_et_illisible() {
        assert!(from_image(Path::new("/rien/nette.png"), "dark", &Scheme::Vibrant, None).is_err());
        let dir = std::env::temp_dir().join(format!("hd-recolor-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("faux.png");
        std::fs::write(&path, b"ceci n'est pas un png").unwrap();
        assert!(from_image(&path, "dark", &Scheme::Vibrant, None).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
