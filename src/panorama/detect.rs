//! Détection générique des écrans Panorama : aucun modèle codé en dur.
//!
//! Un écran est **candidat** si son rapport d'aspect logique (rotation
//! comprise) atteint [`MIN_RATIO`] : 2560×720 (3,56), 1920×480 (4), 1920×515,
//! 1280×400, 3840×1100, 1920×360… Les 16:9 (1,78) et 21:9 (2,37) ne le sont
//! jamais d'office ; on peut leur donner le rôle à la main.
//!
//! Le **tactile** est rapproché de l'écran, du plus sûr au moins sûr : le
//! fabricant de l'écran (EDID) dans le nom du périphérique, puis l'unicité
//! (un seul candidat, un seul tactile). Sinon la proposition s'affiche sur
//! chaque candidat : l'écran où l'on répond « Oui » est retenu.

use serde_json::Value;

/// Rapport d'aspect minimal d'un écran bandeau (grand côté / petit côté).
pub const MIN_RATIO: f32 = 2.5;

/// Écran tel que Hyprland le décrit (`hyprctl monitors -j`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Screen {
    /// Connecteur (`DP-3`, `HDMI-A-1`).
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    /// Taille logique (rotation et échelle appliquées).
    pub width: u32,
    pub height: u32,
    /// Taille physique en mm (rotation appliquée), 0 si inconnue.
    pub mm_width: u32,
    pub mm_height: u32,
}

impl Screen {
    /// Identifiant stable de l'écran (survit au changement de port) :
    /// fabricant, modèle et numéro de série de l'EDID.
    pub fn edid_id(&self) -> String {
        [self.make.trim(), self.model.trim(), self.serial.trim()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Grand côté / petit côté.
    pub fn ratio(&self) -> f32 {
        let (a, b) = (self.width.max(self.height), self.width.min(self.height));
        if b == 0 { 0.0 } else { a as f32 / b as f32 }
    }

    pub fn is_candidate(&self) -> bool {
        self.ratio() >= MIN_RATIO
    }

    /// Pixels logiques par millimètre (largeur), si la taille physique est connue.
    pub fn px_per_mm(&self) -> Option<f32> {
        (self.mm_width > 0).then(|| self.width as f32 / self.mm_width as f32)
    }

    /// L'entrée de configuration `entry` désigne cet écran : connecteur ou
    /// identifiant EDID.
    pub fn matches(&self, entry: &str) -> bool {
        let e = entry.trim();
        !e.is_empty() && (e == self.name || e == self.edid_id())
    }
}

/// Écrans actifs, depuis `hyprctl monitors -j`.
pub fn parse_screens(v: &Value) -> Vec<Screen> {
    let Some(list) = v.as_array() else {
        return Vec::new();
    };
    list.iter()
        .filter(|m| !m["disabled"].as_bool().unwrap_or(false))
        .filter_map(|m| {
            let s = |k: &str| m[k].as_str().unwrap_or("").trim().to_string();
            let n = |k: &str| m[k].as_u64().unwrap_or(0) as u32;
            let scale = m["scale"].as_f64().filter(|s| *s > 0.0).unwrap_or(1.0);
            let rotated = m["transform"].as_u64().unwrap_or(0) % 2 == 1;
            let (w, h) = (n("width"), n("height"));
            let (pw, ph) = (n("physicalWidth"), n("physicalHeight"));
            let ((w, h), (pw, ph)) = if rotated {
                ((h, w), (ph, pw))
            } else {
                ((w, h), (pw, ph))
            };
            Some(Screen {
                name: m["name"].as_str()?.to_string(),
                make: s("make"),
                model: s("model"),
                serial: s("serial"),
                width: (w as f64 / scale).round() as u32,
                height: (h as f64 / scale).round() as u32,
                mm_width: pw,
                mm_height: ph,
            })
        })
        .collect()
}

/// Noms des périphériques tactiles (`hyprctl devices -j`, section `touch`).
pub fn parse_touch(v: &Value) -> Vec<String> {
    v["touch"]
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|d| d["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Mots significatifs d'un nom de fabricant (« Corsair Memory Inc » → corsair, memory).
fn make_words(make: &str) -> Vec<String> {
    const NOISE: &[&str] = &[
        "inc",
        "ltd",
        "co",
        "corp",
        "corporation",
        "gmbh",
        "llc",
        "sa",
        "ag",
        "the",
        "company",
        "technology",
        "technologies",
        "electronics",
        "display",
        "displays",
        "communications",
    ];
    make.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.len() >= 3 && !NOISE.contains(&w.as_str()))
        .collect()
}

/// Le périphérique tactile porte le nom du fabricant (ou du modèle) de l'écran.
pub fn same_maker(screen: &Screen, device: &str) -> bool {
    let d = device.to_lowercase();
    make_words(&screen.make)
        .iter()
        .chain(make_words(&screen.model).iter().take(1))
        .any(|w| d.contains(w.as_str()))
}

/// Ce qu'on propose pour un écran candidat.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    /// Connecteur de l'écran.
    pub screen: String,
    /// Identifiant EDID (pour mémoriser un refus).
    pub edid: String,
    /// Périphérique tactile rattaché, s'il est sûr.
    pub touch: Option<String>,
    /// Un tactile est présent mais son écran n'est pas sûr : la réponse
    /// « Oui » sur cet écran le désigne.
    pub ambiguous: bool,
}

/// Propositions à faire : écrans candidats sans rôle ni refus.
/// `assigned` : entrées `panorama.screens` ; `declined` : identifiants refusés.
pub fn offers(
    screens: &[Screen],
    touch: &[String],
    assigned: &[String],
    declined: &[String],
) -> Vec<Offer> {
    let taken = |s: &Screen| assigned.iter().any(|a| s.matches(a));
    let refused = |s: &Screen| declined.iter().any(|d| s.matches(d));
    let candidates: Vec<&Screen> = screens.iter().filter(|s| s.is_candidate()).collect();
    candidates
        .iter()
        .filter(|s| !taken(s) && !refused(s))
        .map(|s| {
            let by_maker = touch.iter().find(|d| same_maker(s, d)).cloned();
            let unique = (candidates.len() == 1 && touch.len() == 1).then(|| touch[0].clone());
            let t = by_maker.or(unique);
            Offer {
                screen: s.name.clone(),
                edid: s.edid_id(),
                ambiguous: t.is_none() && !touch.is_empty(),
                touch: t,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scr(name: &str, make: &str, w: u32, h: u32) -> Screen {
        Screen {
            name: name.into(),
            make: make.into(),
            model: "M".into(),
            serial: "1".into(),
            width: w,
            height: h,
            ..Default::default()
        }
    }

    #[test]
    fn rapports_d_aspect() {
        for (w, h) in [
            (2560, 720),
            (1920, 480),
            (1920, 515),
            (1280, 400),
            (3840, 1100),
            (1920, 360),
        ] {
            assert!(scr("A", "", w, h).is_candidate(), "{w}×{h}");
            assert!(scr("A", "", h, w).is_candidate(), "portrait {h}×{w}");
        }
        for (w, h) in [(2560, 1440), (3440, 1440), (1920, 1080), (1280, 1024)] {
            assert!(!scr("A", "", w, h).is_candidate(), "{w}×{h}");
        }
    }

    #[test]
    fn lecture_hyprctl_rotation_et_echelle() {
        let v: Value = serde_json::from_str(
            r#"[{"name":"DP-3","make":"Corsair","model":"XENEON EDGE","serial":"S1",
                "width":2560,"height":720,"physicalWidth":344,"physicalHeight":97,
                "scale":1.25,"transform":1},
               {"name":"DP-9","disabled":true,"width":1,"height":1}]"#,
        )
        .unwrap();
        let l = parse_screens(&v);
        assert_eq!(l.len(), 1);
        let s = &l[0];
        assert_eq!((s.width, s.height), (576, 2048));
        assert_eq!((s.mm_width, s.mm_height), (97, 344));
        assert_eq!(s.edid_id(), "Corsair XENEON EDGE S1");
        assert!(s.matches("DP-3") && s.matches("Corsair XENEON EDGE S1") && !s.matches(""));
    }

    #[test]
    fn appariement_par_fabricant() {
        let screens = [
            scr("DP-2", "Ancor Communications Inc", 2560, 1440),
            scr("DP-3", "Corsair Memory Inc", 2560, 720),
            scr("HDMI-A-1", "Wisecoco", 1920, 480),
        ];
        let touch = vec![
            "ILITEK ILITEK-TP".to_string(),
            "Corsair XENEON EDGE Touch".to_string(),
        ];
        let o = offers(&screens, &touch, &[], &[]);
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].touch.as_deref(), Some("Corsair XENEON EDGE Touch"));
        assert!(!o[0].ambiguous);
        // Le second : tactile présent, mais rien ne le relie sûrement.
        assert_eq!(o[1].touch, None);
        assert!(o[1].ambiguous);
    }

    #[test]
    fn appariement_par_unicite_et_sans_tactile() {
        let screens = [
            scr("DP-2", "Dell", 2560, 1440),
            scr("DP-3", "Acme", 1920, 480),
        ];
        let o = offers(&screens, &["ILITEK-TP".into()], &[], &[]);
        assert_eq!(o[0].touch.as_deref(), Some("ILITEK-TP"));
        let o = offers(&screens, &[], &[], &[]);
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].touch, None);
        assert!(!o[0].ambiguous);
    }

    #[test]
    fn deja_attribue_ou_refuse() {
        let screens = [scr("DP-3", "Acme", 1920, 480)];
        assert!(offers(&screens, &[], &["DP-3".into()], &[]).is_empty());
        assert!(offers(&screens, &[], &["Acme M 1".into()], &[]).is_empty());
        assert!(offers(&screens, &[], &[], &["Acme M 1".into()]).is_empty());
        // Le refus suit l'écran, pas le port.
        let moved = [scr("HDMI-A-2", "Acme", 1920, 480)];
        assert!(offers(&moved, &[], &[], &["Acme M 1".into()]).is_empty());
    }

    #[test]
    fn fabricant_sans_mots_parasites() {
        let s = scr("A", "Ancor Communications Inc", 1, 1);
        assert!(!same_maker(&s, "Logitech Inc Touch"));
        assert!(same_maker(&s, "ANCOR touch"));
    }
}
