//! Couleurs du moteur : le type [`Rgba`] est partagé avec la configuration,
//! les maths esthétiques passent par OKLCH (OkLab, Björn Ottosson) et les
//! contrôles d'accessibilité par le contraste WCAG.
//!
//! Petit doublet local de `luminance`/`contrast` : le binaire dédié ne doit
//! pas tirer `crate::theme`, qui entraîne les réglages du démon (et Lua avec).

use crate::config::Rgba;

/// Couleur en OKLCH : luminosité `l` (0..1), chroma `c` (0..≈0,4), teinte `h`
/// en degrés (0..360). L'alpha vit dans le `Rgba` d'origine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

/// Formats de sortie d'une couleur, cible par cible dans les gabarits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fmt {
    /// `#rrggbb`, `#rrggbbaa` si non opaque.
    #[default]
    Hex,
    /// `rgb(r, g, b)` (CSS moderne, GTK).
    Rgb,
    /// `hsl(h, s%, l%)`.
    Hsl,
    /// `rgba(rrggbbaa)` (hyprlang).
    Hypr,
    /// `#aarrggbb` (QPalette de Qt, schémas qt5/6ct).
    Argb,
}

impl Fmt {
    /// Nom du filtre `fmt` dans un gabarit.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "hex" => Self::Hex,
            "rgb" => Self::Rgb,
            "hsl" => Self::Hsl,
            "hypr" => Self::Hypr,
            "argb" => Self::Argb,
            _ => return None,
        })
    }
}

/// Couleur acceptée dans un manifeste : `#rrggbb`, `#rrggbbaa`, `rgb()`,
/// `rgba()` (formes Hyprland comprises) et `hsl()` / `hsla()`.
pub fn parse(text: &str) -> Option<Rgba> {
    let s = text.trim();
    if Rgba::parse(s).is_some() {
        return Rgba::parse(s);
    }
    let inner = s
        .strip_prefix("hsla(")
        .or_else(|| s.strip_prefix("hsl("))
        .and_then(|r| r.strip_suffix(')'))?;
    let mut parts = inner.split([',', ' ']).filter(|p| !p.is_empty());
    let h: f64 = parts
        .next()?
        .trim_end_matches(['d', 'e', 'g'])
        .parse()
        .ok()?;
    let s_pct: f64 = parts.next()?.trim_end_matches('%').parse().ok()?;
    let l_pct: f64 = parts.next()?.trim_end_matches('%').parse().ok()?;
    let a: f64 = match parts.next() {
        Some(p) => p.trim().parse().unwrap_or(1.0),
        None => 1.0,
    };
    Some(hsla_to_rgba(
        h,
        s_pct / 100.0,
        l_pct / 100.0,
        (a * 255.0).round() as u8,
    ))
}

/// Conversion HSL → RVB (teinte en degrés, saturation et lumière 0..1).
pub fn hsla_to_rgba(h: f64, s: f64, l: f64, a: u8) -> Rgba {
    if s <= f64::EPSILON {
        let v = (l * 255.0).round().clamp(0.0, 255.0) as u8;
        return Rgba(v, v, v, a);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hue = |t: f64| {
        let mut t = t;
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let h = ((h.rem_euclid(360.0)) / 360.0) + 1.0 / 3.0;
    let ch = |t: f64| (hue(t) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgba(ch(h), ch(h - 1.0 / 3.0), ch(h - 2.0 / 3.0), a)
}

/// Luminance relative WCAG (0..1).
pub fn luminance(c: Rgba) -> f64 {
    let lin = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.039_28 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.0) + 0.7152 * lin(c.1) + 0.0722 * lin(c.2)
}

/// Rapport de contraste WCAG entre deux couleurs (1 à 21).
pub fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// `c` passée en OKLCH (alpha perdu, il ne se transforme pas en OKLCH).
pub fn to_oklch(c: Rgba) -> Oklch {
    let (l, a, b) = to_oklab(c);
    let ch = (a * a + b * b).sqrt();
    Oklch {
        l,
        c: ch,
        h: b.atan2(a).to_degrees().rem_euclid(360.0),
    }
}

/// OKLCH → `Rgba` opaque (borné à la gamme sRGB : composantes serrées).
pub fn from_oklch(o: Oklch) -> Rgba {
    let a = o.c * o.h.to_radians().cos();
    let b = o.c * o.h.to_radians().sin();
    from_oklab(o.l, a, b)
}

/// Éclaircit de `pct` (pourcentage de luminosité OKLCH, borné).
pub fn lighten(c: Rgba, pct: f64) -> Rgba {
    shift_l(c, pct / 100.0)
}

/// Assombrit de `pct` (borné).
pub fn darken(c: Rgba, pct: f64) -> Rgba {
    shift_l(c, -pct / 100.0)
}

/// Sature de `pct` (signé : négatif = désature).
pub fn saturate(c: Rgba, pct: f64) -> Rgba {
    let o = to_oklch(c);
    from_oklch(Oklch {
        c: (o.c * (1.0 + pct / 100.0)).max(0.0),
        ..o
    })
}

/// Tourne la teinte de `deg` (signé, modulo 360).
pub fn rotate_hue(c: Rgba, deg: f64) -> Rgba {
    let o = to_oklch(c);
    from_oklch(Oklch {
        h: (o.h + deg).rem_euclid(360.0),
        ..o
    })
}

/// Mélange deux couleurs en OKLab (`t` 0 = `a`, 1 = `b`), alpha interpolé.
pub fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let (la, aa, ba) = to_oklab(a);
    let (lb, ab, bb) = to_oklab(b);
    let out = from_oklab(la + (lb - la) * t, aa + (ab - aa) * t, ba + (bb - ba) * t);
    Rgba(
        out.0,
        out.1,
        out.2,
        (a.3 as f64 + (b.3 as f64 - a.3 as f64) * t).round() as u8,
    )
}

/// `c` avec un alpha réglé (0..1).
pub fn with_alpha(c: Rgba, a: f64) -> Rgba {
    Rgba(c.0, c.1, c.2, (a * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// `c` assez contrastée sur `against` (au moins `min`, WCAG) : luminosité
/// OKLCH poussée loin de celle du fond par recherche dichotomique ; en cas
/// d'impossibilité (fond extrême), noir ou blanc, celui qui contraste le plus.
pub fn ensure_contrast(c: Rgba, against: Rgba, min: f64) -> Rgba {
    if contrast(c, against) >= min {
        return c;
    }
    let o = to_oklch(c);
    let against_l = to_oklch(against).l;
    let mut best = if against_l > o.l {
        search_l(o, o.l, 1.0, against, min)
    } else {
        search_l(o, o.l, 0.0, against, min)
    };
    // Le fond bloque la direction : l'autre extrême reste possible.
    if contrast(best, against) < min {
        let alt = if against_l > o.l {
            search_l(o, o.l, 0.0, against, min)
        } else {
            search_l(o, o.l, 1.0, against, min)
        };
        if contrast(alt, against) > contrast(best, against) {
            best = alt;
        }
    }
    best
}

/// Dichotomie de la luminosité entre `from` et `to` pour atteindre `min`.
fn search_l(o: Oklch, from: f64, to: f64, against: Rgba, min: f64) -> Rgba {
    let try_l = |l: f64| from_oklch(Oklch { l, ..o });
    let mut lo = from.min(to);
    let mut hi = from.max(to);
    // Si l'extrémité n'atteint pas le plancher, prendre l'extrémité même.
    let end = try_l(to);
    if contrast(end, against) < min {
        return end;
    }
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if contrast(try_l(mid), against) >= min {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    try_l(hi)
}

/// Rendu d'une couleur dans un format de gabarit.
pub fn fmt(c: Rgba, f: Fmt) -> String {
    match f {
        Fmt::Hex => {
            if c.3 == 255 {
                format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
            } else {
                format!("#{:02x}{:02x}{:02x}{:02x}", c.0, c.1, c.2, c.3)
            }
        }
        Fmt::Rgb => format!("rgb({}, {}, {})", c.0, c.1, c.2),
        Fmt::Hsl => {
            let (h, s, l) = hsl_from_rgba(c);
            format!(
                "hsl({}, {}%, {}%)",
                h.round() as i64,
                (s * 100.0).round() as i64,
                (l * 100.0).round() as i64
            )
        }
        Fmt::Hypr => format!("rgba({:02x}{:02x}{:02x}{:02x})", c.0, c.1, c.2, c.3),
        Fmt::Argb => format!("#{:02x}{:02x}{:02x}{:02x}", c.3, c.0, c.1, c.2),
    }
}

/// HSL calculé depuis le RVB (pour le filtre `fmt(hsl)`).
pub fn hsl_from_rgba(c: Rgba) -> (f64, f64, f64) {
    let (r, g, b) = (c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < f64::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn shift_l(c: Rgba, delta: f64) -> Rgba {
    let o = to_oklch(c);
    from_oklch(Oklch {
        l: (o.l + delta).clamp(0.0, 1.0),
        ..o
    })
}

fn to_oklab(c: Rgba) -> (f64, f64, f64) {
    let lin = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.040_45 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(c.0), lin(c.1), lin(c.2));
    let l_ = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m_ = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s_ = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let (l, m, s) = (l_.cbrt(), m_.cbrt(), s_.cbrt());
    (
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    )
}

fn from_oklab(l: f64, a: f64, b: f64) -> Rgba {
    let l_ = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m_ = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s_ = (l - 0.089_484_177_5 * a - 1.291_485_548_0 * b).powi(3);
    let r = 4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_;
    let g = -1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_;
    let b = -0.004_196_086_5 * l_ - 0.703_418_614_7 * m_ + 1.707_614_701_0 * s_;
    let srgb = |v: f64| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.003_130_8 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    };
    Rgba(
        (srgb(r) * 255.0).round() as u8,
        (srgb(g) * 255.0).round() as u8,
        (srgb(b) * 255.0).round() as u8,
        255,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proche(a: Rgba, b: Rgba) -> bool {
        (a.0 as i16 - b.0 as i16).abs() <= 2
            && (a.1 as i16 - b.1 as i16).abs() <= 2
            && (a.2 as i16 - b.2 as i16).abs() <= 2
            && a.3 == b.3
    }

    #[test]
    fn aller_retour_oklch_fidèle() {
        for c in [
            Rgba(0x17, 0x1b, 0x27, 0xff),
            Rgba(0xa6, 0x9c, 0xff, 0xff),
            Rgba(0xff, 0xd4, 0x00, 0xff),
            Rgba(0x00, 0x00, 0x00, 0xff),
            Rgba(0xff, 0xff, 0xff, 0xff),
            Rgba(0x8a, 0xff, 0x80, 0xff),
        ] {
            assert!(
                proche(from_oklch(to_oklch(c)), c),
                "aller-retour faux : {c:?}"
            );
        }
    }

    #[test]
    fn contraste_noir_blanc_21() {
        let blanc = Rgba(255, 255, 255, 255);
        let noir = Rgba(0, 0, 0, 255);
        assert!((contrast(blanc, noir) - 21.0).abs() < 0.1);
        assert!((contrast(blanc, blanc) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn luminance_conforme_aux_valeurs_connues() {
        // Le jaune pur est plus lumineux que le bleu pur (WCAG).
        assert!(luminance(Rgba(255, 255, 0, 255)) > luminance(Rgba(0, 0, 255, 255)));
    }

    #[test]
    fn éclaircir_et_assombrir_dans_le_bon_sens() {
        let c = Rgba(0x40, 0x50, 0x60, 0xff);
        let plus = lighten(c, 20.0);
        let moins = darken(c, 20.0);
        assert!(luminance(plus) > luminance(c));
        assert!(luminance(moins) < luminance(c));
        // Borné : éclaircir du blanc reste du blanc.
        assert_eq!(
            lighten(Rgba(255, 255, 255, 255), 50.0),
            Rgba(255, 255, 255, 255)
        );
    }

    #[test]
    fn teinte_et_saturation() {
        let rouge = Rgba(255, 0, 0, 0xff);
        let vert = rotate_hue(rouge, 120.0);
        // La teinte tournée ne reste pas rouge : sa teinte est proche de 120°.
        let o = to_oklch(vert);
        assert!(
            (o.h - 120.0).abs() < 3.0 || (o.h - 140.0).abs() < 15.0,
            "teinte : {}",
            o.h
        );
        assert!(to_oklch(saturate(rouge, -100.0)).c < 0.01);
    }

    #[test]
    fn mélange_aux_extrémités() {
        let noir = Rgba(0, 0, 0, 255);
        let blanc = Rgba(255, 255, 255, 255);
        assert_eq!(mix(noir, blanc, 0.0), noir);
        assert_eq!(mix(noir, blanc, 1.0), blanc);
        let moitié = mix(noir, blanc, 0.5);
        assert!(
            moitié.0 > 60 && moitié.0 < 140,
            "gris perceptuel : {}",
            moitié.0
        );
    }

    #[test]
    fn alpha_appliqué() {
        let c = with_alpha(Rgba(10, 20, 30, 255), 0.5);
        assert!((c.3 as i16 - 128).abs() <= 1);
        assert!(c.0 == 10 && c.1 == 20 && c.2 == 30);
    }

    #[test]
    fn contraste_garanti_par_ajustement() {
        let fond = Rgba(0x17, 0x1b, 0x27, 0xff);
        let peu_lisible = Rgba(0x2a, 0x30, 0x40, 0xff);
        assert!(contrast(peu_lisible, fond) < 2.0);
        let ajusté = ensure_contrast(peu_lisible, fond, 4.5);
        assert!(
            contrast(ajusté, fond) >= 4.5,
            "contraste obtenu : {}",
            contrast(ajusté, fond)
        );
        // Une couleur déjà suffisante n'est pas touchée.
        assert_eq!(
            ensure_contrast(Rgba(255, 255, 255, 255), fond, 4.5),
            Rgba(255, 255, 255, 255)
        );
    }

    #[test]
    fn analyse_des_couleurs_de_manifeste() {
        assert_eq!(parse("#a69cff"), Some(Rgba(0xa6, 0x9c, 0xff, 255)));
        assert_eq!(parse("rgba(a69cffcc)"), Some(Rgba(0xa6, 0x9c, 0xff, 0xcc)));
        assert!(parse("hsl(0, 100%, 50%)").is_some());
        assert!(parse("hsl(120 50% 30%)").is_some());
        assert_eq!(parse("rouge"), None);
        // hsl rouge pur = #ff0000
        let hsl = parse("hsl(0, 100%, 50%)").unwrap();
        assert!(proche(hsl, Rgba(255, 0, 0, 255)), "hsl rouge : {hsl:?}");
    }

    #[test]
    fn formats_de_sortie() {
        let c = Rgba(0xa6, 0x9c, 0xff, 0xcc);
        assert_eq!(fmt(c, Fmt::Hex), "#a69cffcc");
        assert_eq!(fmt(Rgba(1, 2, 3, 255), Fmt::Hex), "#010203");
        assert_eq!(fmt(Rgba(1, 2, 3, 255), Fmt::Rgb), "rgb(1, 2, 3)");
        assert_eq!(
            fmt(Rgba(0xa6, 0x9c, 0xff, 0xcc), Fmt::Hypr),
            "rgba(a69cffcc)"
        );
        let hsl = fmt(Rgba(255, 0, 0, 255), Fmt::Hsl);
        assert!(hsl.starts_with("hsl(0, 100%"), "hsl : {hsl}");
    }

    #[test]
    fn nom_de_format_reconnu() {
        assert_eq!(Fmt::parse("hypr"), Some(Fmt::Hypr));
        assert_eq!(Fmt::parse("autre"), None);
    }
}
