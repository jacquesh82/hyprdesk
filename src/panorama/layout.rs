//! Mise en page du Panorama : aucune dimension figée, tout part de la taille
//! logique de la surface (et de sa taille physique quand l'EDID la donne).

/// Rectangle en pixels logiques de la surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Landscape,
    Portrait,
}

/// Zones d'une page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zones {
    pub orientation: Orientation,
    /// Carré de la pochette (ou de l'illustration de la page).
    pub art: Rect,
    /// Zone principale (titre, progression, commandes).
    pub main: Rect,
    /// Colonne latérale (volume, lecteurs) ; absente si la place manque.
    pub side: Option<Rect>,
    /// Côté minimal d'une cible tactile, en pixels logiques.
    pub target: f32,
    /// Marge intérieure.
    pub pad: f32,
}

/// Cible tactile minimale : 9 mm.
pub const TARGET_MM: f32 = 9.0;
/// Repli sans taille physique.
pub const TARGET_PX: f32 = 64.0;
/// En dessous de cette longueur (grand côté) pour la zone principale, la
/// colonne latérale disparaît.
const SIDE_MIN_MAIN: f32 = 900.0;

/// Côté d'une cible tactile : 9 mm si la densité est connue, 64 px sinon ;
/// jamais moins de 44 px ni plus de 120 px.
pub fn target(px_per_mm: Option<f32>) -> f32 {
    px_per_mm
        .filter(|d| *d > 0.5)
        .map(|d| d * TARGET_MM)
        .unwrap_or(TARGET_PX)
        .clamp(44.0, 120.0)
}

/// Zones pour une surface `w`×`h`.
pub fn zones(w: f32, h: f32, px_per_mm: Option<f32>) -> Zones {
    let orientation = if w >= h {
        Orientation::Landscape
    } else {
        Orientation::Portrait
    };
    let short = w.min(h);
    let long = w.max(h);
    let pad = (short * 0.05).clamp(8.0, 32.0);
    let art_side = short - 2.0 * pad;
    // Colonne latérale : un tiers de la longueur restante, si la zone
    // principale garde assez de place.
    let rest = long - short;
    let side_len = (rest * 0.32).max(0.0);
    let with_side = rest - side_len >= SIDE_MIN_MAIN;
    let main_len = if with_side { rest - side_len } else { rest };
    let (art, main, side) = match orientation {
        Orientation::Landscape => (
            Rect {
                x: pad,
                y: pad,
                w: art_side,
                h: art_side,
            },
            Rect {
                x: short,
                y: pad,
                w: main_len - pad,
                h: art_side,
            },
            with_side.then_some(Rect {
                x: short + main_len,
                y: pad,
                w: side_len - pad,
                h: art_side,
            }),
        ),
        Orientation::Portrait => (
            Rect {
                x: pad,
                y: pad,
                w: art_side,
                h: art_side,
            },
            Rect {
                x: pad,
                y: short,
                w: art_side,
                h: main_len - pad,
            },
            with_side.then_some(Rect {
                x: pad,
                y: short + main_len,
                w: art_side,
                h: side_len - pad,
            }),
        ),
    };
    Zones {
        orientation,
        art,
        main,
        side,
        target: target(px_per_mm),
        pad,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(r: Rect, w: f32, h: f32) -> bool {
        r.x >= 0.0
            && r.y >= 0.0
            && r.x + r.w <= w + 0.5
            && r.y + r.h <= h + 0.5
            && r.w > 0.0
            && r.h > 0.0
    }

    #[test]
    fn resolutions_types() {
        for (w, h, side) in [
            (2560.0, 720.0, true),
            (1920.0, 480.0, true),
            (1280.0, 400.0, false),
            (720.0, 2560.0, true),
            (480.0, 1920.0, true),
            (400.0, 1280.0, false),
        ] {
            let z = zones(w, h, None);
            assert_eq!(z.side.is_some(), side, "{w}×{h}");
            assert!(
                inside(z.art, w, h) && inside(z.main, w, h),
                "{w}×{h} : {z:?}"
            );
            if let Some(s) = z.side {
                assert!(inside(s, w, h), "{w}×{h} : {s:?}");
            }
            assert_eq!(
                z.orientation,
                if w >= h {
                    Orientation::Landscape
                } else {
                    Orientation::Portrait
                }
            );
            // Pochette carrée.
            assert_eq!(z.art.w, z.art.h);
        }
    }

    #[test]
    fn cible_tactile() {
        // Xeneon Edge : 2560 px sur 344 mm ≈ 7,4 px/mm → ~67 px.
        assert!((target(Some(2560.0 / 344.0)) - 67.0).abs() < 1.0);
        assert_eq!(target(None), TARGET_PX);
        assert_eq!(target(Some(30.0)), 120.0);
    }
}
