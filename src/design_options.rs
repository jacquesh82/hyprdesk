//! Options d'apparence des composants du bureau (MLG-279) : dock, espaces
//! de travail, réglages rapides, notifications, monitoring, horloge et
//! média, verrouillage.
//!
//! Un seul registre décrit chaque option (type, bornes, défaut, libellé) ;
//! les valeurs vivent dans `general.plugin_options["ui.<composant>"]`, déjà
//! lu et écrit par `settings.lua`. Le registre sert au CLI
//! (`hyprdesk config set dock.spacing 8`), aux packs (`[dock] spacing = 8`)
//! et aux Réglages, qui en tirent leurs champs.

use crate::config::{Config, General};
use crate::tiling::OptValue;

/// Nature d'une option et ce qu'elle admet.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Int(i64, i64),
    Float(f64, f64),
    Bool,
    Choice(&'static [&'static str]),
    /// Texte libre (chemin d'image).
    Text,
}

/// Valeur par défaut d'une option.
#[derive(Debug, Clone, Copy)]
pub enum Def {
    Num(f64),
    Bool(bool),
    Str(&'static str),
}

/// Option d'apparence d'un composant.
#[derive(Debug, Clone, Copy)]
pub struct Opt {
    pub component: &'static str,
    pub key: &'static str,
    pub kind: Kind,
    pub default: Def,
    /// Libellé des Réglages.
    pub label: &'static str,
    /// Aide du CLI : valeurs admises, puis à quoi sert l'option.
    pub help: &'static str,
}

const fn opt(
    component: &'static str,
    key: &'static str,
    kind: Kind,
    default: Def,
    label: &'static str,
    help: &'static str,
) -> Opt {
    Opt {
        component,
        key,
        kind,
        default,
        label,
        help,
    }
}

/// Le registre.
pub static OPTIONS: &[Opt] = &[
    // Dock.
    opt(
        "dock",
        "spacing",
        Kind::Int(0, 24),
        Def::Num(4.0),
        "Espacement des icônes",
        "0..24 — espacement des icônes du dock (px)",
    ),
    opt(
        "dock",
        "magnify",
        Kind::Float(1.0, 2.0),
        Def::Num(1.0),
        "Grossissement au survol",
        "1..2 — grossissement de l'icône survolée (1 : aucun)",
    ),
    opt(
        "dock",
        "autohide",
        Kind::Bool,
        Def::Bool(false),
        "Masquer hors survol",
        "on | off — dock masqué hors survol, révélé au contact du bord",
    ),
    opt(
        "dock",
        "active_marker",
        Kind::Choice(&["dot", "bar", "none"]),
        Def::Str("dot"),
        "Repère de l'application active",
        "dot | bar | none — repère sous l'application au premier plan",
    ),
    // Espaces de travail.
    opt(
        "workspaces",
        "capsule_width",
        Kind::Int(16, 64),
        Def::Num(28.0),
        "Largeur des capsules",
        "16..64 — largeur d'une capsule d'espace (px, numéros et points)",
    ),
    opt(
        "workspaces",
        "wheel",
        Kind::Bool,
        Def::Bool(true),
        "Molette : espace suivant",
        "on | off — la molette sur le sélecteur change d'espace",
    ),
    opt(
        "workspaces",
        "occupied_marker",
        Kind::Bool,
        Def::Bool(true),
        "Repère des espaces occupés",
        "on | off — marque les espaces qui ont des fenêtres",
    ),
    // Réglages rapides.
    opt(
        "controls",
        "columns",
        Kind::Int(1, 4),
        Def::Num(2.0),
        "Colonnes",
        "1..4 — colonnes des tuiles des réglages rapides",
    ),
    opt(
        "controls",
        "width",
        Kind::Int(240, 560),
        Def::Num(304.0),
        "Largeur du panneau",
        "240..560 — largeur des réglages rapides (px)",
    ),
    // Notifications.
    opt(
        "notifications",
        "corner",
        Kind::Choice(&[
            "top-right",
            "top-left",
            "bottom-right",
            "bottom-left",
            "top-center",
        ]),
        Def::Str("top-right"),
        "Coin",
        "top-right | top-left | bottom-right | bottom-left | top-center — coin des notifications",
    ),
    opt(
        "notifications",
        "width",
        Kind::Int(260, 560),
        Def::Num(360.0),
        "Largeur des cartes",
        "260..560 — largeur d'une notification (px)",
    ),
    opt(
        "notifications",
        "max_visible",
        Kind::Int(1, 8),
        Def::Num(4.0),
        "Nombre visible",
        "1..8 — notifications affichées à la fois",
    ),
    opt(
        "notifications",
        "timeout",
        Kind::Int(2, 60),
        Def::Num(6.0),
        "Durée d'affichage",
        "2..60 — durée d'affichage (s), au moins : une application peut allonger une bulle, pas la raccourcir",
    ),
    opt(
        "notifications",
        "stack",
        Kind::Choice(&["down", "up"]),
        Def::Str("down"),
        "Empilement",
        "down | up — sens d'empilement",
    ),
    opt(
        "notifications",
        "hide_previews",
        Kind::Bool,
        Def::Bool(false),
        "Masquer les aperçus",
        "on | off — le corps des notifications est remplacé par un texte générique",
    ),
    // Monitoring.
    opt(
        "monitoring",
        "threshold",
        Kind::Float(0.5, 1.0),
        Def::Num(0.85),
        "Seuil d'alerte",
        "0.5..1 — jauge au-delà de laquelle la mesure passe en alerte",
    ),
    opt(
        "monitoring",
        "bars",
        Kind::Bool,
        Def::Bool(true),
        "Jauges",
        "on | off — jauges sous les mesures",
    ),
    // Horloge et média.
    opt(
        "clock",
        "date",
        Kind::Bool,
        Def::Bool(true),
        "Date sous l'heure",
        "on | off — date sous l'heure de l'horloge du bureau",
    ),
    opt(
        "media",
        "cover_size",
        Kind::Int(32, 160),
        Def::Num(64.0),
        "Taille de la pochette",
        "32..160 — taille de la pochette (px)",
    ),
    opt(
        "media",
        "cover_tint",
        Kind::Bool,
        Def::Bool(false),
        "Teinte tirée de la pochette",
        "on | off — la carte du média prend la teinte de la pochette",
    ),
    // Fond d'écran (MLG-280).
    opt(
        "wallpaper",
        "focal_x",
        Kind::Float(0.0, 1.0),
        Def::Num(0.5),
        "Point focal horizontal",
        "0..1 — part de l'image gardée au centre en largeur (remplir)",
    ),
    opt(
        "wallpaper",
        "focal_y",
        Kind::Float(0.0, 1.0),
        Def::Num(0.5),
        "Point focal vertical",
        "0..1 — part de l'image gardée au centre en hauteur (remplir)",
    ),
    opt(
        "wallpaper",
        "zoom",
        Kind::Float(1.0, 3.0),
        Def::Num(1.0),
        "Zoom",
        "1..3 — agrandissement de l'image (remplir)",
    ),
    opt(
        "wallpaper",
        "brightness",
        Kind::Float(-0.5, 0.5),
        Def::Num(0.0),
        "Luminosité",
        "-0.5..0.5 — luminosité du fond",
    ),
    opt(
        "wallpaper",
        "saturation",
        Kind::Float(0.0, 2.0),
        Def::Num(1.0),
        "Saturation",
        "0..2 — saturation du fond (0 : noir et blanc)",
    ),
    opt(
        "wallpaper",
        "contrast",
        Kind::Float(0.5, 1.5),
        Def::Num(1.0),
        "Contraste",
        "0.5..1.5 — contraste du fond",
    ),
    opt(
        "wallpaper",
        "vignette",
        Kind::Float(0.0, 1.0),
        Def::Num(0.0),
        "Vignette",
        "0..1 — assombrissement des bords",
    ),
    opt(
        "wallpaper",
        "speed",
        Kind::Float(0.25, 4.0),
        Def::Num(1.0),
        "Vitesse de lecture",
        "0.25..4 — vitesse d'un fond vidéo",
    ),
    opt(
        "wallpaper",
        "fps",
        Kind::Int(5, 60),
        Def::Num(24.0),
        "Images par seconde",
        "5..60 — cadence d'un fond vidéo",
    ),
    opt(
        "wallpaper",
        "glass_tint",
        Kind::Float(0.0, 0.5),
        Def::Num(0.0),
        "Teinte du verre par le fond",
        "0..0.5 — le verre prend la couleur moyenne du fond",
    ),
    opt(
        "wallpaper",
        "light_image",
        Kind::Text,
        Def::Str(""),
        "Fond en mode clair",
        "<chemin> — image de fond quand le thème est clair (vide : celle des règles)",
    ),
    opt(
        "wallpaper",
        "dark_image",
        Kind::Text,
        Def::Str(""),
        "Fond en mode sombre",
        "<chemin> — image de fond quand le thème est sombre (vide : celle des règles)",
    ),
    // Verrouillage et connexion.
    opt(
        "lock",
        "clock_position",
        Kind::Choice(&["top", "center", "bottom-left"]),
        Def::Str("center"),
        "Place de l'horloge",
        "top | center | bottom-left — place de l'horloge de l'écran de verrouillage",
    ),
    opt(
        "lock",
        "panel_opacity",
        Kind::Float(0.0, 1.0),
        Def::Num(0.55),
        "Densité du panneau",
        "0..1 — densité du panneau de connexion",
    ),
];

/// Réglages surchargeables par écran (MLG-281) : `screen.<écran>.<clé>`,
/// l'écran désigné par son nom (`DP-1`) ou sa définition (`2570x720`).
pub static SCREEN_KEYS: &[Opt] = &[
    opt(
        "screen",
        "bar_height",
        Kind::Int(24, 64),
        Def::Num(0.0),
        "Hauteur de la barre",
        "24..64 — hauteur de la barre sur cet écran",
    ),
    opt(
        "screen",
        "text_scale",
        Kind::Float(0.5, 2.0),
        Def::Num(1.0),
        "Échelle du texte",
        "0.5..2 — échelle du texte des surfaces de cet écran",
    ),
];

/// Option par écran : (écran, option).
pub fn screen_find(name: &str) -> Option<(&str, &'static Opt)> {
    let rest = name.strip_prefix("screen.")?;
    let (sel, key) = rest.rsplit_once('.')?;
    if sel.is_empty() || sel.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    SCREEN_KEYS.iter().find(|o| o.key == key).map(|o| (sel, o))
}

/// Valeur d'une option par écran pour l'écran `monitor` (définition
/// logique `size`), la plus précise d'abord : nom, puis définition.
pub fn screen_num(g: &General, monitor: &str, size: (u32, u32), key: &str) -> Option<f64> {
    let get = |sel: &str| {
        g.plugin_options
            .get(&format!("ui.screen.{sel}"))
            .and_then(|m| m.get(key))
            .and_then(|v| match v {
                OptValue::Num(n) => Some(*n),
                _ => None,
            })
    };
    get(monitor).or_else(|| {
        g.plugin_options
            .keys()
            .filter_map(|k| k.strip_prefix("ui.screen."))
            .find(|sel| crate::theme_engine::design::Screen::matches(sel, monitor, Some(size)))
            .and_then(get)
    })
}

/// Clé de `plugin_options` qui porte un composant.
pub fn store(component: &str) -> String {
    format!("ui.{component}")
}

/// Option `<composant>.<clé>`.
pub fn find(name: &str) -> Option<&'static Opt> {
    let (c, k) = name.split_once('.')?;
    OPTIONS.iter().find(|o| o.component == c && o.key == k)
}

impl Opt {
    /// Nom complet `<composant>.<clé>`.
    pub fn name(&self) -> String {
        format!("{}.{}", self.component, self.key)
    }

    /// Valeur lue d'un texte (CLI, pack), bornée ou refusée.
    pub fn parse(&self, text: &str) -> Result<OptValue, String> {
        let t = text.trim();
        match self.kind {
            Kind::Int(lo, hi) => {
                let v: f64 = t
                    .replace(',', ".")
                    .parse()
                    .map_err(|_| crate::tr!("« {v} » : nombre attendu", v = t))?;
                if v < lo as f64 || v > hi as f64 {
                    return Err(crate::tr!(
                        "« {v} » : entre {lo} et {hi}",
                        v = t,
                        lo = lo,
                        hi = hi
                    ));
                }
                Ok(OptValue::Num(v.round()))
            }
            Kind::Float(lo, hi) => {
                let v: f64 = t
                    .replace(',', ".")
                    .parse()
                    .map_err(|_| crate::tr!("« {v} » : nombre attendu", v = t))?;
                if !(lo..=hi).contains(&v) {
                    return Err(crate::tr!(
                        "« {v} » : entre {lo} et {hi}",
                        v = t,
                        lo = lo,
                        hi = hi
                    ));
                }
                Ok(OptValue::Num(v))
            }
            Kind::Bool => match t {
                "on" | "true" | "yes" | "oui" | "1" => Ok(OptValue::Bool(true)),
                "off" | "false" | "no" | "non" | "0" => Ok(OptValue::Bool(false)),
                _ => Err(crate::tr!("« {v} » : on ou off", v = t)),
            },
            Kind::Text => {
                if t.chars().any(char::is_control) || t.len() > 1024 {
                    return Err(crate::tr!("« {v} » : texte d'une ligne attendu", v = t));
                }
                // `~/…` : dossier personnel.
                let path = match (t.strip_prefix("~/"), std::env::var_os("HOME")) {
                    (Some(rest), Some(home)) => {
                        std::path::Path::new(&home).join(rest).display().to_string()
                    }
                    _ => t.to_string(),
                };
                Ok(OptValue::Str(path))
            }
            Kind::Choice(list) => {
                if list.contains(&t) {
                    Ok(OptValue::Str(t.to_string()))
                } else {
                    Err(crate::tr!(
                        "« {v} » : valeurs admises {allowed}",
                        v = t,
                        allowed = list.join(", ")
                    ))
                }
            }
        }
    }

    fn default_value(&self) -> OptValue {
        match self.default {
            Def::Num(n) => OptValue::Num(n),
            Def::Bool(b) => OptValue::Bool(b),
            Def::Str(s) => OptValue::Str(s.to_string()),
        }
    }

    /// Valeur en place (réglée, sinon défaut).
    pub fn value(&self, g: &General) -> OptValue {
        g.plugin_options
            .get(&store(self.component))
            .and_then(|m| m.get(self.key))
            .cloned()
            .unwrap_or_else(|| self.default_value())
    }

    /// Valeur affichée par `hyprdesk config get`.
    pub fn shown(&self, c: &Config) -> String {
        match self.value(&c.general) {
            OptValue::Bool(b) => if b { "on" } else { "off" }.to_string(),
            OptValue::Num(n) => {
                if matches!(self.kind, Kind::Int(..)) {
                    format!("{}", n.round() as i64)
                } else {
                    format!("{n}")
                }
            }
            OptValue::Str(s) => s,
            other => other.display(),
        }
    }
}

/// Nombre d'une option (défaut si absente ou d'un autre type).
pub fn num(g: &General, name: &str) -> f64 {
    let Some(o) = find(name) else { return 0.0 };
    match (o.value(g), o.default) {
        (OptValue::Num(n), _) => n,
        (_, Def::Num(n)) => n,
        _ => 0.0,
    }
}

/// Booléen d'une option.
pub fn flag(g: &General, name: &str) -> bool {
    let Some(o) = find(name) else { return false };
    match (o.value(g), o.default) {
        (OptValue::Bool(b), _) => b,
        (_, Def::Bool(b)) => b,
        _ => false,
    }
}

/// Choix d'une option.
pub fn choice(g: &General, name: &str) -> String {
    let Some(o) = find(name) else {
        return String::new();
    };
    match (o.value(g), o.default) {
        (OptValue::Str(s), _) => s,
        (_, Def::Str(s)) => s.to_string(),
        _ => String::new(),
    }
}

/// Pose une option dans des réglages.
pub fn set(
    s: &mut crate::settings::Settings,
    cfg: &Config,
    name: &str,
    text: &str,
) -> Result<(), String> {
    if let Some((sel, o)) = screen_find(name) {
        let v = o.parse(text)?;
        let all = s
            .plugin_options
            .get_or_insert_with(|| cfg.general.plugin_options.clone());
        all.entry(format!("ui.screen.{sel}"))
            .or_default()
            .insert(o.key.to_string(), v);
        return Ok(());
    }
    let o = find(name).ok_or_else(|| crate::tr!("option inconnue : {k}", k = name))?;
    let v = o.parse(text)?;
    let all = s
        .plugin_options
        .get_or_insert_with(|| cfg.general.plugin_options.clone());
    all.entry(store(o.component))
        .or_default()
        .insert(o.key.to_string(), v);
    Ok(())
}

/// Retire une option (retour au défaut).
pub fn unset(s: &mut crate::settings::Settings, cfg: &Config, name: &str) {
    if let Some((sel, o)) = screen_find(name) {
        let all = s
            .plugin_options
            .get_or_insert_with(|| cfg.general.plugin_options.clone());
        let store = format!("ui.screen.{sel}");
        if let Some(m) = all.get_mut(&store) {
            m.remove(o.key);
            if m.is_empty() {
                all.remove(&store);
            }
        }
        return;
    }
    let Some(o) = find(name) else { return };
    let all = s
        .plugin_options
        .get_or_insert_with(|| cfg.general.plugin_options.clone());
    if let Some(m) = all.get_mut(&store(o.component)) {
        m.remove(o.key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registre_cohérent() {
        let mut seen = std::collections::BTreeSet::new();
        for o in OPTIONS {
            assert!(seen.insert(o.name()), "en double : {}", o.name());
            // Le défaut passe sa propre validation.
            let text = match o.default {
                Def::Num(n) => n.to_string(),
                Def::Bool(b) => if b { "on" } else { "off" }.to_string(),
                Def::Str(s) => s.to_string(),
            };
            o.parse(&text)
                .unwrap_or_else(|e| panic!("{} : {e}", o.name()));
        }
        assert!(find("dock.spacing").is_some());
        assert!(find("dock.inconnu").is_none());
        assert!(find("dock.magnify").unwrap().parse("3").is_err());
        assert!(
            find("notifications.corner")
                .unwrap()
                .parse("milieu")
                .is_err()
        );
    }

    #[test]
    fn surcharges_par_écran() {
        let cfg = Config::default();
        let mut s = crate::settings::Settings::default();
        set(&mut s, &cfg, "screen.2570x720.bar_height", "36").unwrap();
        set(&mut s, &cfg, "screen.DP-1.text_scale", "1.2").unwrap();
        assert!(set(&mut s, &cfg, "screen.DP-1.inconnu", "1").is_err());
        let mut g = cfg.general.clone();
        g.plugin_options = s.plugin_options.clone().unwrap();
        assert_eq!(
            screen_num(&g, "HDMI-A-1", (2570, 720), "bar_height"),
            Some(36.0)
        );
        assert_eq!(screen_num(&g, "HDMI-A-1", (1920, 1080), "bar_height"), None);
        assert_eq!(
            screen_num(&g, "DP-1", (2560, 1440), "text_scale"),
            Some(1.2)
        );
        unset(&mut s, &cfg, "screen.2570x720.bar_height");
        assert!(!s.plugin_options.unwrap().contains_key("ui.screen.2570x720"));
    }

    #[test]
    fn réglées_puis_lues() {
        let cfg = Config::default();
        let mut s = crate::settings::Settings::default();
        set(&mut s, &cfg, "dock.spacing", "10").unwrap();
        set(&mut s, &cfg, "notifications.corner", "bottom-left").unwrap();
        let mut g = cfg.general.clone();
        g.plugin_options = s.plugin_options.clone().unwrap();
        assert_eq!(num(&g, "dock.spacing"), 10.0);
        assert_eq!(choice(&g, "notifications.corner"), "bottom-left");
        assert!(flag(&g, "workspaces.wheel"));
        unset(&mut s, &cfg, "dock.spacing");
        g.plugin_options = s.plugin_options.clone().unwrap();
        assert_eq!(num(&g, "dock.spacing"), 4.0);
    }
}
