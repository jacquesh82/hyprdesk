//! Configuration du Panorama : `hd.config({ panorama = { … } })`.
//!
//! ```lua
//! hd.config({ panorama = {
//!   screens  = { "DP-3" },            -- connecteur ou identifiant EDID
//!   pages    = { "media", "system", "clock", "video" },
//!   layout   = "1x2",                 -- single | 1x2 | 1x3 | 2x2 | 3x3 | dashboard
//!   zones    = {                      -- réglages de chaque découpage
//!     ["1x2"] = { panes = { "media", "video" }, first = "1/3" },
//!     ["2x2"] = { panes = { "clock", "media", "video", "system" } },
//!   },
//!   sources  = { { name = "Free TV", url = "https://tv.free.fr", icon = "…" } },
//!   browser  = "brave",               -- vide : le premier navigateur Chromium trouvé
//!   declined = { "Acme M 1" },        -- écrans refusés (identifiant EDID)
//!   sleep_minutes = 5,                -- 0 : jamais d'horloge de veille
//!   video_dim = {                     -- voile autour de la vidéo qui joue
//!     on = true, level = 55,          -- intensité en % (10..90)
//!     scope = "all",                  -- all : fond et zones | background : fond
//!     paused = false,                 -- garder le voile en pause
//!     fade = 600,                     -- durée du fondu, ms (0..3000)
//!     hover = 35,                     -- % du voile gardé sous la souris
//!   },
//! } })
//! ```

use std::collections::BTreeMap;

use mlua::{Table, Value};

/// Disposition de l'écran : une page à la fois, l'écran découpé en zones
/// (1×2, 1×3, 2×2, 3×3, voir `crate::zones`), ou la composition libre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    Single,
    Split2,
    Split3,
    Grid4,
    Grid9,
    Dashboard,
}

impl Layout {
    pub const ALL: [Layout; 6] = [
        Layout::Single,
        Layout::Split2,
        Layout::Split3,
        Layout::Grid4,
        Layout::Grid9,
        Layout::Dashboard,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Layout::Single => "single",
            Layout::Split2 => "1x2",
            Layout::Split3 => "1x3",
            Layout::Grid4 => "2x2",
            Layout::Grid9 => "3x3",
            Layout::Dashboard => "dashboard",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "single" | "1" => return Some(Layout::Single),
            "dashboard" => return Some(Layout::Dashboard),
            _ => {}
        }
        Some(match crate::zones::Grid::parse(s)? {
            crate::zones::Grid::One => Layout::Single,
            crate::zones::Grid::Cols2 => Layout::Split2,
            crate::zones::Grid::Cols3 => Layout::Split3,
            crate::zones::Grid::Grid2 => Layout::Grid4,
            crate::zones::Grid::Grid3 => Layout::Grid9,
        })
    }

    /// Découpage en zones (`None` : une page à la fois, composition libre).
    pub fn grid(self) -> Option<crate::zones::Grid> {
        use crate::zones::Grid;
        match self {
            Layout::Split2 => Some(Grid::Cols2),
            Layout::Split3 => Some(Grid::Cols3),
            Layout::Grid4 => Some(Grid::Grid2),
            Layout::Grid9 => Some(Grid::Grid3),
            Layout::Single | Layout::Dashboard => None,
        }
    }

    /// Libellé court des boutons de l'écran : 1, 1×2… ; ✦ pour la composition.
    pub fn short(self) -> &'static str {
        match self {
            Layout::Single => "1",
            Layout::Dashboard => "✦",
            l => l.grid().map(crate::zones::Grid::short).unwrap_or("?"),
        }
    }

    /// Disposition suivante (bouton unique de l'écran compact).
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|l| *l == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Nombre de zones à widget choisi (découpages seulement).
    pub fn panes(self) -> usize {
        self.grid().map(crate::zones::Grid::count).unwrap_or(1)
    }
}

/// Source vidéo : un service ouvert en fenêtre d'application du navigateur.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub name: String,
    pub url: String,
    /// Icône de la tuile : nom d'icône ou chemin (vide : initiale ; SmartUI
    /// la dessine, `hyprdesk panorama icons`).
    pub icon: String,
    /// Mémorisé par source : comment passer sa vidéo en plein écran
    /// (`autofull::Method` : dblclick, key, click:x,y), trouvé à la
    /// première lecture ; vide : à chercher.
    pub fs: String,
    /// Cadrage mémorisé par source : la fenêtre est agrandie (`zoom` ≥ 1) et
    /// décalée (`dx`, `dy`, en part de la zone) derrière la zone du lecteur,
    /// qui n'en laisse voir que la vidéo.
    pub zoom: f32,
    pub dx: f32,
    pub dy: f32,
    /// Affichage dans le Panorama : auto (sans compte, ou connecté), on, off.
    pub show: String,
}

impl Default for Source {
    fn default() -> Self {
        Source {
            name: String::new(),
            url: String::new(),
            icon: String::new(),
            fs: String::new(),
            zoom: 1.0,
            dx: 0.0,
            dy: 0.0,
            show: "auto".into(),
        }
    }
}

impl Source {
    /// Rectangle de la fenêtre pour la zone `r` (x, y, l, h), cadrage appliqué.
    pub fn framed(&self, r: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
        let z = self.zoom.clamp(1.0, 4.0);
        let (w, h) = (r.2 as f32 * z, r.3 as f32 * z);
        let x = r.0 as f32 - (w - r.2 as f32) / 2.0 + self.dx * r.2 as f32;
        let y = r.1 as f32 - (h - r.3 as f32) / 2.0 + self.dy * r.3 as f32;
        (
            x.round() as i32,
            y.round() as i32,
            w.round() as i32,
            h.round() as i32,
        )
    }

    /// Cadrage par défaut (fenêtre = zone).
    pub fn unframed(&self) -> bool {
        self.zoom == 1.0 && self.dx == 0.0 && self.dy == 0.0
    }
}

/// Sources par défaut (modifiables dans Réglages → Panorama).
pub fn default_sources() -> Vec<Source> {
    [
        ("Free TV", "https://tv.free.fr"),
        ("Netflix", "https://www.netflix.com"),
        ("Disney+", "https://www.disneyplus.com"),
        ("HBO Max", "https://play.hbomax.com"),
        ("Canal+", "https://www.canalplus.com"),
        ("Prime Video", "https://www.primevideo.com"),
        ("YouTube", "https://www.youtube.com"),
    ]
    .iter()
    .map(|(n, u)| Source {
        name: n.to_string(),
        url: u.to_string(),
        icon: String::new(),
        ..Default::default()
    })
    .collect()
}

/// Services de musique par défaut (profil Musique du Panorama).
pub fn default_music_sources() -> Vec<Source> {
    [
        ("Spotify", "https://open.spotify.com"),
        ("Deezer", "https://www.deezer.com"),
        ("YouTube Music", "https://music.youtube.com"),
        ("Qobuz", "https://play.qobuz.com"),
        ("Radio France", "https://www.radiofrance.fr"),
    ]
    .iter()
    .map(|(n, u)| Source {
        name: n.to_string(),
        url: u.to_string(),
        icon: String::new(),
        ..Default::default()
    })
    .collect()
}

/// Zone Panorama sur un écran ordinaire : une bande collée à un bord, que
/// les fenêtres laissent libre (zone exclusive), comme un second écran.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub screen: String,
    /// Bord : bottom, top (bande horizontale), left, right (verticale).
    pub side: String,
    /// Part de l'écran (0,1 à 0,5).
    pub size: f32,
}

impl Zone {
    pub const SIDES: &'static [&'static str] = &["bottom", "top", "left", "right"];

    pub fn vertical(&self) -> bool {
        matches!(self.side.as_str(), "left" | "right")
    }

    /// Épaisseur de la bande (px logiques) sur un écran `w`×`h`.
    pub fn thickness(&self, w: f32, h: f32) -> f32 {
        let base = if self.vertical() { w } else { h };
        (base * self.size.clamp(0.1, 0.5)).round()
    }

    /// Position de la bande dans l'écran (px logiques) : (x, y, l, h).
    pub fn rect(&self, w: f32, h: f32) -> (f32, f32, f32, f32) {
        let t = self.thickness(w, h);
        match self.side.as_str() {
            "top" => (0.0, 0.0, w, t),
            "left" => (0.0, 0.0, t, h),
            "right" => (w - t, 0.0, t, h),
            _ => (0.0, h - t, w, t),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Écrans au rôle Panorama : connecteur (`DP-3`) ou identifiant EDID.
    pub screens: Vec<String>,
    /// Pages du carrousel après la vue par défaut (une page à la fois en
    /// disposition `single`), chacune avec sa composition.
    pub pages: Vec<super::design::Page>,
    pub layout: Layout,
    /// Réglages propres à chaque découpage (1x2, 1x3, 2x2, 3x3) : chacun
    /// garde les siens quand on passe de l'un à l'autre.
    pub zones: BTreeMap<String, ZoneSet>,
    /// Sources du widget Vidéo.
    pub sources: Vec<Source>,
    /// Services de musique (profil Musique : même lecteur que la vidéo).
    pub music_sources: Vec<Source>,
    /// Navigateur des sources vidéo (vide : détection automatique).
    pub browser: String,
    /// Restituer automatiquement la fenêtre capturée à la sortie du plein écran.
    pub restore_on_fullscreen_exit: bool,
    /// Voile sombre autour de la vidéo pendant la lecture.
    pub video_dim: VideoDim,
    /// Service de webradio de l'onglet Webradio (`radio::SERVICES`).
    pub webradio: String,
    /// Radios favorites de l'onglet Webradio (adresse du flux).
    pub radios: Vec<Source>,
    /// Écrans dont la proposition a été refusée (identifiant EDID).
    pub declined: Vec<String>,
    /// Minutes sans lecture ni toucher avant l'horloge de veille (0 : jamais).
    pub sleep_minutes: u32,
    /// LYRA vocale : écoute du mot « Lyra » (bouton micro du Panorama).
    pub voice: bool,
    /// Zone Panorama sur un écran ordinaire (aucune : écrans entiers).
    pub zone: Option<Zone>,
    /// Micro de LYRA vocale : nœud PipeWire (vide : celui par défaut).
    pub voice_mic: String,
    /// Halo de conversation : screens (tous les écrans), panorama, off.
    pub voice_halo: String,
    /// Couleur du halo : lyra (violet-bleu) ou accent (thème).
    pub voice_halo_color: String,
    /// Ambiance de la barre « flux vocal » (webradio) : aurora, sunset, ocean.
    pub flux_ambiance: String,
    pub design: super::design::Design,
}

/// Voile sombre posé autour de la zone Vidéo pendant la lecture (MLG-191) :
/// la vidéo se détache, l'écran fatigue moins. Jamais sur la fenêtre vidéo.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoDim {
    pub on: bool,
    /// Intensité, en % d'opacité du voile (10..=90).
    pub level: u8,
    /// `all` : fond et zones autour de la vidéo ; `background` : fond seul.
    pub scope: String,
    /// Garder le voile quand la lecture est en pause.
    pub paused: bool,
    /// Durée du fondu d'entrée et de sortie, en ms (0..=3000).
    pub fade: u32,
    /// Part du voile gardée quand la souris est sur le Panorama, en %
    /// (0 : le voile s'efface, 100 : il ne change pas).
    pub hover: u8,
}

impl VideoDim {
    pub const SCOPES: [&'static str; 2] = ["all", "background"];
    pub const LEVELS: std::ops::RangeInclusive<u8> = 10..=90;
    pub const FADE_MAX: u32 = 3000;

    /// Opacité du voile (0..1) : lecture en cours, ou en pause si réglé.
    pub fn alpha(&self, playing: bool) -> f32 {
        if self.on && (playing || self.paused) {
            f32::from(self.level) / 100.0
        } else {
            0.0
        }
    }

    /// Part du voile gardée sous la souris (0..1).
    pub fn hover_factor(&self) -> f32 {
        f32::from(self.hover.min(100)) / 100.0
    }

    fn from_lua(v: Value) -> mlua::Result<Self> {
        let mut d = VideoDim::default();
        match v {
            Value::Boolean(b) => d.on = b,
            Value::Table(t) => {
                if let Some(b) = t.get::<Option<bool>>("on")? {
                    d.on = b;
                }
                if let Some(n) = t.get::<Option<f64>>("level")? {
                    d.level = (n.round().clamp(0.0, 100.0) as u8)
                        .clamp(*Self::LEVELS.start(), *Self::LEVELS.end());
                }
                if let Some(sc) = t.get::<Option<String>>("scope")? {
                    if !Self::SCOPES.contains(&sc.as_str()) {
                        return Err(mlua::Error::RuntimeError(format!(
                            "{CTX} : video_dim.scope = all | background"
                        )));
                    }
                    d.scope = sc;
                }
                if let Some(b) = t.get::<Option<bool>>("paused")? {
                    d.paused = b;
                }
                if let Some(n) = t.get::<Option<f64>>("fade")? {
                    d.fade = n.round().clamp(0.0, f64::from(Self::FADE_MAX)) as u32;
                }
                if let Some(n) = t.get::<Option<f64>>("hover")? {
                    d.hover = n.round().clamp(0.0, 100.0) as u8;
                }
            }
            Value::Nil => {}
            _ => {
                return Err(mlua::Error::RuntimeError(format!(
                    "{CTX} : video_dim = {{ on, level, scope, paused, fade, hover }}"
                )));
            }
        }
        Ok(d)
    }

    /// Champs différents du défaut (vide : rien à écrire).
    fn to_lua(&self) -> Option<String> {
        let d = VideoDim::default();
        let mut f = Vec::new();
        if self.on != d.on {
            f.push(format!("on = {}", self.on));
        }
        if self.level != d.level {
            f.push(format!("level = {}", self.level));
        }
        if self.scope != d.scope {
            f.push(format!("scope = {}", q(&self.scope)));
        }
        if self.paused != d.paused {
            f.push(format!("paused = {}", self.paused));
        }
        if self.fade != d.fade {
            f.push(format!("fade = {}", self.fade));
        }
        if self.hover != d.hover {
            f.push(format!("hover = {}", self.hover));
        }
        (!f.is_empty()).then(|| format!("{{ {} }}", f.join(", ")))
    }
}

impl Default for VideoDim {
    fn default() -> Self {
        VideoDim {
            on: true,
            level: 55,
            scope: "all".into(),
            paused: false,
            fade: 600,
            hover: 35,
        }
    }
}

/// Réglages d'un découpage : widget de chaque zone, part de la zone 1.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ZoneSet {
    /// Widget de chaque zone (9 au plus) ; zones absentes : leur défaut.
    pub panes: Vec<String>,
    /// Part de la zone 1 en 1×2 et 1×3 (`None` : 1/2 en 1×2, 1/3 en 1×3).
    pub first: Option<crate::zones::First>,
}

/// Zones au plus (3×3).
pub const MAX_PANES: usize = 9;

/// Widget par défaut de chaque zone.
const DEFAULT_PANES: [&str; MAX_PANES] = [
    "media",
    "video",
    "system",
    "clock",
    "calendar",
    "shortcuts",
    "media",
    "system",
    "clock",
];

impl Default for Config {
    fn default() -> Self {
        Config {
            screens: Vec::new(),
            pages: super::design::default_pages(),
            layout: Layout::Dashboard,
            zones: BTreeMap::new(),
            sources: default_sources(),
            music_sources: default_music_sources(),
            browser: String::new(),
            restore_on_fullscreen_exit: true,
            video_dim: VideoDim::default(),
            webradio: super::radio::DEFAULT_SERVICE.into(),
            radios: super::radio::default_radios(),
            declined: Vec::new(),
            sleep_minutes: 5,
            voice: false,
            zone: None,
            voice_mic: String::new(),
            voice_halo: "screens".into(),
            voice_halo_color: "lyra".into(),
            flux_ambiance: "aurora".into(),
            design: Default::default(),
        }
    }
}

const CTX: &str = "hd.config.panorama";
const KEYS: &[&str] = &[
    "screens",
    "pages",
    "layout",
    "first",
    "panes",
    "zones",
    "sources",
    "music_sources",
    "browser",
    "restore_on_fullscreen_exit",
    "video_dim",
    "webradio",
    "radios",
    "declined",
    "sleep_minutes",
    "voice",
    "zone",
    "voice_mic",
    "voice_halo",
    "voice_halo_color",
    "flux_ambiance",
    "design",
];

fn list(t: &Table, key: &str) -> mlua::Result<Option<Vec<String>>> {
    match t.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Table(l) => {
            let mut out = Vec::new();
            for v in l.sequence_values::<String>() {
                let v = v?;
                if !v.trim().is_empty() {
                    out.push(v.trim().to_string());
                }
            }
            Ok(Some(out))
        }
        Value::String(s) => Ok(Some(vec![s.to_str()?.trim().to_string()])),
        v => Err(mlua::Error::RuntimeError(crate::tr!(
            "{CTX} : « {key} » attend une liste de chaînes, pas {p0}",
            CTX = CTX,
            key = key.to_string(),
            p0 = v.type_name()
        ))),
    }
}

fn known_widget(id: &str) -> bool {
    super::PAGES.iter().any(|(p, _)| *p == id)
}

/// Widgets des zones : inconnu remplacé par celui de la zone par défaut, neuf au plus.
fn known_panes(l: Vec<String>) -> Vec<String> {
    l.into_iter()
        .take(MAX_PANES)
        .enumerate()
        .map(|(i, p)| {
            if known_widget(&p) {
                p
            } else {
                DEFAULT_PANES[i].to_string()
            }
        })
        .collect()
}

fn parse_first(s: &str) -> mlua::Result<crate::zones::First> {
    crate::zones::First::parse(s).ok_or_else(|| {
        mlua::Error::RuntimeError(crate::tr!("{CTX} : « first » attend 1/2 ou 1/3", CTX = CTX))
    })
}

/// Lit la table `panorama` ; les clés absentes gardent leur défaut.
pub fn from_lua(t: &Table) -> mlua::Result<Config> {
    let mut c = Config::default();
    for pair in t.pairs::<Value, Value>() {
        let (k, _) = pair?;
        let Value::String(k) = k else { continue };
        let k = k.to_str()?.to_string();
        if !KEYS.contains(&k.as_str()) {
            return Err(mlua::Error::RuntimeError(crate::tr!(
                "{CTX} : clé inconnue « {k} » ({p0})",
                CTX = CTX,
                k = k,
                p0 = KEYS.join(", ")
            )));
        }
    }
    if let Some(l) = list(t, "screens")? {
        c.screens = l;
    }
    if let Some(l) = t.get::<Option<Table>>("pages")? {
        let pages = super::design::pages_from_lua(&l)?;
        // Que des widgets inconnus : les pages par défaut.
        if !pages.is_empty() || l.raw_len() == 0 {
            c.pages = pages;
        }
    }
    if let Some(s) = t.get::<Option<String>>("layout")? {
        c.layout = Layout::parse(&s).ok_or_else(|| {
            mlua::Error::RuntimeError(crate::tr!(
                "{CTX} : « layout » attend single, 1x2, 1x3, 2x2, 3x3 ou dashboard",
                CTX = CTX
            ))
        })?;
    }
    // Anciennes clés : la même part et les mêmes widgets pour chaque découpage.
    if let Some(s) = t.get::<Option<String>>("first")? {
        let f = parse_first(&s)?;
        for l in [Layout::Split2, Layout::Split3] {
            c.zones.entry(l.id().into()).or_default().first = Some(f);
        }
    }
    if let Some(l) = list(t, "panes")? {
        let panes = known_panes(l);
        for l in Layout::ALL.into_iter().filter(|l| l.grid().is_some()) {
            c.zones.entry(l.id().into()).or_default().panes = panes.clone();
        }
    }
    if let Some(map) = t.get::<Option<Table>>("zones")? {
        for pair in map.pairs::<String, Table>() {
            let (k, v) = pair?;
            let l = Layout::parse(&k)
                .filter(|l| l.grid().is_some())
                .ok_or_else(|| {
                    mlua::Error::RuntimeError(crate::tr!(
                        "{CTX} : « zones » : découpage inconnu « {k} » (1x2, 1x3, 2x2, 3x3)",
                        CTX = CTX,
                        k = k
                    ))
                })?;
            let z = c.zones.entry(l.id().into()).or_default();
            if let Some(p) = list(&v, "panes")? {
                z.panes = known_panes(p);
            }
            if let Some(f) = v.get::<Option<String>>("first")? {
                z.first = Some(parse_first(&f)?);
            }
        }
    }
    for key in ["sources", "music_sources", "radios"] {
        let Some(l) = t.get::<Option<Table>>(key)? else {
            continue;
        };
        let mut out = Vec::new();
        for v in l.sequence_values::<Table>() {
            let v = v?;
            let name: String = v.get::<Option<String>>("name")?.unwrap_or_default();
            let url: String = v.get::<Option<String>>("url")?.unwrap_or_default();
            let icon: String = v.get::<Option<String>>("icon")?.unwrap_or_default();
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(mlua::Error::RuntimeError(crate::tr!(
                    "{CTX} : source « {name} » : l'adresse doit commencer par https://",
                    CTX = CTX,
                    name = name
                )));
            }
            out.push(Source {
                name: if name.trim().is_empty() {
                    url.clone()
                } else {
                    name.trim().to_string()
                },
                url: url.trim().to_string(),
                icon: icon.trim().to_string(),
                fs: v
                    .get::<Option<String>>("fs")?
                    .filter(|m| super::autofull::Method::parse(m).is_some())
                    .unwrap_or_default(),
                zoom: v.get::<Option<f32>>("zoom")?.unwrap_or(1.0).clamp(1.0, 4.0),
                dx: v.get::<Option<f32>>("dx")?.unwrap_or(0.0).clamp(-2.0, 2.0),
                dy: v.get::<Option<f32>>("dy")?.unwrap_or(0.0).clamp(-2.0, 2.0),
                show: v
                    .get::<Option<String>>("show")?
                    .filter(|s| matches!(s.as_str(), "on" | "off"))
                    .unwrap_or_else(|| "auto".into()),
            });
        }
        match key {
            "sources" => c.sources = out,
            "music_sources" => c.music_sources = out,
            _ => c.radios = out,
        }
    }
    if let Some(b) = t.get::<Option<String>>("browser")? {
        c.browser = b.trim().to_string();
    }
    if let Some(b) = t.get::<Option<bool>>("restore_on_fullscreen_exit")? {
        c.restore_on_fullscreen_exit = b;
    }
    c.video_dim = VideoDim::from_lua(t.get::<Value>("video_dim")?)?;
    if let Some(w) = t.get::<Option<String>>("webradio")? {
        if super::radio::service(&w).is_none() {
            return Err(mlua::Error::RuntimeError(format!(
                "{CTX} : webradio = {}",
                super::radio::SERVICES
                    .iter()
                    .map(|s| s.0)
                    .collect::<Vec<_>>()
                    .join(" | ")
            )));
        }
        c.webradio = w;
    }
    if let Some(l) = list(t, "declined")? {
        c.declined = l;
    }
    if let Some(v) = t.get::<Option<bool>>("voice")? {
        c.voice = v;
    }
    if let Some(z) = t.get::<Option<Table>>("zone")? {
        let screen: String = z.get::<Option<String>>("screen")?.unwrap_or_default();
        let side: String = z
            .get::<Option<String>>("side")?
            .unwrap_or_else(|| "bottom".into());
        let size: f32 = z.get::<Option<f32>>("size")?.unwrap_or(0.25);
        if screen.trim().is_empty() || !Zone::SIDES.contains(&side.as_str()) {
            return Err(mlua::Error::RuntimeError(format!(
                "{CTX} : zone = {{ screen = \"DP-2\", side = bottom | top | left | right, size = 0.1..0.5 }}"
            )));
        }
        c.zone = Some(Zone {
            screen: screen.trim().to_string(),
            side,
            size: size.clamp(0.1, 0.5),
        });
    }
    if let Some(m) = t.get::<Option<String>>("voice_mic")? {
        c.voice_mic = m.trim().to_string();
    }
    if let Some(h) = t.get::<Option<String>>("voice_halo")?
        && matches!(h.as_str(), "screens" | "panorama" | "off")
    {
        c.voice_halo = h;
    }
    if let Some(a) = t.get::<Option<String>>("flux_ambiance")?
        && matches!(a.as_str(), "aurora" | "sunset" | "ocean")
    {
        c.flux_ambiance = a;
    }
    if let Some(h) = t.get::<Option<String>>("voice_halo_color")?
        && matches!(h.as_str(), "lyra" | "accent")
    {
        c.voice_halo_color = h;
    }
    match t.get::<Value>("sleep_minutes")? {
        Value::Nil => {}
        Value::Integer(n) if n >= 0 => c.sleep_minutes = n as u32,
        Value::Number(n) if n >= 0.0 => c.sleep_minutes = n as u32,
        _ => {
            return Err(mlua::Error::RuntimeError(crate::tr!(
                "{CTX} : « sleep_minutes » attend un nombre de minutes (0 : jamais)",
                CTX = CTX
            )));
        }
    }
    if let Some(t) = t.get::<Option<Table>>("design")? {
        c.design = super::design::from_lua(&t)?;
    }
    if t.get::<Value>("layout")?.is_nil() && c.design.preset != "cinematic" {
        c.layout = Layout::Dashboard;
    }
    Ok(c)
}

fn q(s: &str) -> String {
    format!("\"{}\"", hyprpanel_core::ipc::lua_escape(s))
}

fn lua_list(l: &[String]) -> String {
    format!(
        "{{ {} }}",
        l.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")
    )
}

impl Config {
    /// Sources du lecteur selon le profil du bloc de raccourcis : services
    /// de musique en profil Musique, sources vidéo sinon.
    pub fn active_sources(&self) -> &[Source] {
        if self.design.shortcut_profile == "audio" {
            &self.music_sources
        } else {
            &self.sources
        }
    }

    /// Table Lua (relue par [`from_lua`]) ; les valeurs par défaut sont omises.
    pub fn to_lua(&self) -> String {
        let d = Config::default();
        let mut f = vec![format!("screens = {}", lua_list(&self.screens))];
        if self.pages != d.pages {
            f.push(format!("pages = {}", super::design::pages_lua(&self.pages)));
        }
        // Toujours écrite hors défaut, et avec un préréglage autre que
        // « cinematic » (sans elle, la relecture choisit la composition libre).
        if self.layout != d.layout || self.design.preset != "cinematic" {
            f.push(format!("layout = {}", q(self.layout.id())));
        }
        let zones: Vec<String> = self
            .zones
            .iter()
            .filter(|(_, z)| **z != ZoneSet::default())
            .map(|(k, z)| {
                let mut e = Vec::new();
                if !z.panes.is_empty() {
                    e.push(format!("panes = {}", lua_list(&z.panes)));
                }
                if let Some(first) = z.first {
                    e.push(format!("first = {}", q(first.id())));
                }
                format!("[{}] = {{ {} }}", q(k), e.join(", "))
            })
            .collect();
        if !zones.is_empty() {
            f.push(format!("zones = {{ {} }}", zones.join(", ")));
        }
        for (key, mine, def) in [
            ("sources", &self.sources, &d.sources),
            ("music_sources", &self.music_sources, &d.music_sources),
            ("radios", &self.radios, &d.radios),
        ] {
            if mine == def {
                continue;
            }
            let s: Vec<String> = mine
                .iter()
                .map(|s| {
                    let mut e = vec![
                        format!("name = {}", q(&s.name)),
                        format!("url = {}", q(&s.url)),
                    ];
                    if !s.icon.is_empty() {
                        e.push(format!("icon = {}", q(&s.icon)));
                    }
                    if !s.fs.is_empty() {
                        e.push(format!("fs = {}", q(&s.fs)));
                    }
                    if !s.unframed() {
                        e.push(format!("zoom = {}, dx = {}, dy = {}", s.zoom, s.dx, s.dy));
                    }
                    if s.show != "auto" {
                        e.push(format!("show = {}", q(&s.show)));
                    }
                    format!("{{ {} }}", e.join(", "))
                })
                .collect();
            f.push(format!("{key} = {{ {} }}", s.join(", ")));
        }
        if self.webradio != d.webradio {
            f.push(format!("webradio = {}", q(&self.webradio)));
        }
        if !self.browser.is_empty() {
            f.push(format!("browser = {}", q(&self.browser)));
        }
        if !self.restore_on_fullscreen_exit {
            f.push("restore_on_fullscreen_exit = false".into());
        }
        if let Some(v) = self.video_dim.to_lua() {
            f.push(format!("video_dim = {v}"));
        }
        if !self.declined.is_empty() {
            f.push(format!("declined = {}", lua_list(&self.declined)));
        }
        f.push(format!("sleep_minutes = {}", self.sleep_minutes));
        if self.voice {
            f.push("voice = true".into());
        }
        if let Some(z) = &self.zone {
            f.push(format!(
                "zone = {{ screen = {}, side = {}, size = {} }}",
                q(&z.screen),
                q(&z.side),
                z.size
            ));
        }
        if !self.voice_mic.is_empty() {
            f.push(format!("voice_mic = {}", q(&self.voice_mic)));
        }
        if self.voice_halo != d.voice_halo {
            f.push(format!("voice_halo = {}", q(&self.voice_halo)));
        }
        if self.flux_ambiance != d.flux_ambiance {
            f.push(format!("flux_ambiance = {}", q(&self.flux_ambiance)));
        }
        if self.voice_halo_color != d.voice_halo_color {
            f.push(format!("voice_halo_color = {}", q(&self.voice_halo_color)));
        }
        if self.design != d.design {
            f.push(format!("design = {}", self.design.to_lua()));
        }
        format!("{{ {} }}", f.join(", "))
    }

    /// L'écran `s` a le rôle Panorama.
    pub fn has(&self, s: &super::detect::Screen) -> bool {
        self.screens.iter().any(|e| s.matches(e))
    }

    /// Découpage de la disposition active (`None` : une page à la fois,
    /// composition libre).
    pub fn split(&self) -> Option<crate::zones::Split> {
        self.split_of(self.layout)
    }

    /// Découpage d'une disposition, avec sa part de zone 1.
    pub fn split_of(&self, l: Layout) -> Option<crate::zones::Split> {
        l.grid().map(|grid| crate::zones::Split {
            grid,
            first: self.first_of(l),
        })
    }

    /// Part de la zone 1 réglée pour une disposition.
    pub fn first_of(&self, l: Layout) -> Option<crate::zones::First> {
        self.zones.get(l.id()).and_then(|z| z.first)
    }

    pub fn set_first(&mut self, l: Layout, f: Option<crate::zones::First>) {
        self.zones.entry(l.id().into()).or_default().first = f;
    }

    /// Widget de la zone `i` de la disposition active.
    pub fn pane(&self, i: usize) -> &str {
        self.pane_of(self.layout, i)
    }

    /// Widget de la zone `i` d'une disposition (celui de la zone par défaut
    /// s'il n'est pas donné).
    pub fn pane_of(&self, l: Layout, i: usize) -> &str {
        self.zones
            .get(l.id())
            .and_then(|z| z.panes.get(i))
            .map(String::as_str)
            .or_else(|| DEFAULT_PANES.get(i).copied())
            .unwrap_or(super::PAGES[0].0)
    }

    /// Widgets de toutes les zones d'une disposition.
    pub fn panes_of(&self, l: Layout) -> Vec<String> {
        (0..l.panes())
            .map(|i| self.pane_of(l, i).to_string())
            .collect()
    }

    /// Change le widget de la zone `i` (les zones d'avant prennent leur défaut).
    pub fn set_pane(&mut self, l: Layout, i: usize, w: &str) {
        if i >= MAX_PANES {
            return;
        }
        let mut panes = self
            .zones
            .get(l.id())
            .map(|z| z.panes.clone())
            .unwrap_or_default();
        while panes.len() <= i {
            let n = panes.len();
            panes.push(self.pane_of(l, n).to_string());
        }
        panes[i] = w.to_string();
        self.zones.entry(l.id().into()).or_default().panes = panes;
    }

    /// Widgets des zones d'une disposition, en entier.
    pub fn set_panes(&mut self, l: Layout, panes: Vec<String>) {
        self.zones.entry(l.id().into()).or_default().panes = known_panes(panes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str) -> mlua::Result<Config> {
        let lua = mlua::Lua::new();
        let t: Table = lua.load(src).eval()?;
        from_lua(&t)
    }

    #[test]
    fn cadrage_par_source() {
        let mut s = Source::default();
        assert!(s.unframed());
        assert_eq!(s.framed((100, 50, 800, 450)), (100, 50, 800, 450));
        // Zoom 1,5 centré, puis décalé d'un quart de zone vers le haut.
        s.zoom = 1.5;
        assert_eq!(s.framed((100, 50, 800, 450)), (-100, -63, 1200, 675));
        s.dy = -0.25;
        assert_eq!(s.framed((100, 50, 800, 450)).1, -175);
    }

    #[test]
    fn zone_panorama() {
        let z = Zone {
            screen: "DP-2".into(),
            side: "bottom".into(),
            size: 0.25,
        };
        assert_eq!(z.rect(2560.0, 1440.0), (0.0, 1080.0, 2560.0, 360.0));
        let z = Zone {
            side: "right".into(),
            ..z
        };
        assert!(z.vertical());
        assert_eq!(z.rect(2560.0, 1440.0), (1920.0, 0.0, 640.0, 1440.0));
        assert!(eval("{ zone = { screen = 'DP-2', side = 'middle' } }").is_err());
    }

    #[test]
    fn aller_retour() {
        let c = Config {
            screens: vec!["DP-3".into(), "Corsair XENEON EDGE S1".into()],
            pages: vec![
                crate::panorama::design::Page::single("clock"),
                crate::panorama::design::Page::system(),
            ],
            layout: Layout::Split3,
            zones: BTreeMap::from([
                (
                    "1x3".to_string(),
                    ZoneSet {
                        panes: vec!["video".into(), "clock".into(), "media".into()],
                        first: Some(crate::zones::First::Half),
                    },
                ),
                (
                    "2x2".to_string(),
                    ZoneSet {
                        panes: vec!["clock".into()],
                        first: None,
                    },
                ),
            ]),
            sources: vec![Source {
                name: "Free \"TV\"".into(),
                url: "https://tv.free.fr".into(),
                icon: "/tmp/free.svg".into(),
                fs: "click:0.965,0.935".into(),
                zoom: 1.25,
                dx: 0.0,
                dy: -0.5,
                show: "off".into(),
            }],
            music_sources: vec![Source {
                name: "Radio".into(),
                url: "https://www.radiofrance.fr".into(),
                icon: String::new(),
                ..Default::default()
            }],
            browser: "chromium".into(),
            restore_on_fullscreen_exit: false,
            video_dim: VideoDim {
                on: false,
                level: 70,
                scope: "background".into(),
                paused: true,
                fade: 0,
                hover: 100,
            },
            webradio: "radio-garden".into(),
            radios: vec![Source {
                name: "FIP".into(),
                url: "https://icecast.radiofrance.fr/fip-hifi.aac".into(),
                ..Default::default()
            }],
            declined: vec!["Acme \"M\" 1".into()],
            sleep_minutes: 0,
            voice: true,
            zone: Some(Zone {
                screen: "DP-2".into(),
                side: "left".into(),
                size: 0.3,
            }),
            voice_mic: "alsa_input.usb-x".into(),
            voice_halo: "panorama".into(),
            voice_halo_color: "accent".into(),
            flux_ambiance: "ocean".into(),
            design: Default::default(),
        };
        assert_eq!(eval(&c.to_lua()).unwrap(), c);
        assert_eq!(
            eval(&Config::default().to_lua()).unwrap(),
            Config::default()
        );
        // Les défauts ne sont pas écrits.
        assert!(!Config::default().to_lua().contains("sources"));
    }

    #[test]
    fn erreurs_et_pages_inconnues() {
        assert!(eval("{ screen = {} }").is_err());
        assert!(eval("{ sleep_minutes = 'x' }").is_err());
        assert!(eval("{ video_dim = { scope = 'tout' } }").is_err());
        assert!(eval("{ video_dim = 3 }").is_err());
        let v = eval("{ video_dim = { level = 99, fade = 9000, hover = 150 } }")
            .unwrap()
            .video_dim;
        assert_eq!((v.level, v.fade, v.hover), (90, 3000, 100));
        assert!(!eval("{ video_dim = false }").unwrap().video_dim.on);
        assert!(eval("{ screens = 3 }").is_err());
        assert!(eval("{ layout = 'split9' }").is_err());
        assert!(eval("{ sources = { { name = 'x', url = 'file:///etc' } } }").is_err());
        let c = eval("{ pages = { 'radio', 'clock' }, screens = 'DP-1' }").unwrap();
        assert_eq!(
            c.pages,
            vec![crate::panorama::design::Page::single("clock")]
        );
        assert_eq!(c.screens, vec!["DP-1".to_string()]);
        assert_eq!(
            eval("{ pages = { 'x' } }").unwrap().pages,
            Config::default().pages
        );
        // Zones : inconnues remplacées par le défaut, neuf au plus.
        let c = eval("{ layout = '1x2', panes = { 'clock', 'radio' } }").unwrap();
        assert_eq!(c.zones["1x2"].panes, vec!["clock", "video"]);
        assert_eq!(
            (c.pane(0), c.pane(2), c.pane(8)),
            ("clock", "system", "clock")
        );
        assert!(eval("{ zones = { ['4x4'] = {} } }").is_err());
        assert!(eval("{ first = '1/4' }").is_err());
    }

    #[test]
    fn dispositions() {
        // Anciennes valeurs relues.
        assert_eq!(
            eval("{ layout = 'split2' }").unwrap().layout,
            Layout::Split2
        );
        assert_eq!(eval("{ layout = '3x3' }").unwrap().layout, Layout::Grid9);
        let c = eval("{ layout = '1x3', first = '1/2' }").unwrap();
        assert_eq!(c.split().unwrap().id(), "1x3:1/2");
        assert_eq!(c.layout.panes(), 3);
        assert!(Config::default().split().is_none());
        assert_eq!(Layout::Dashboard.next(), Layout::Single);
        // Chaque découpage garde ses réglages.
        let mut c = eval("{ layout = '2x2', zones = { ['1x2'] = { panes = { 'video', 'clock' }, first = '1/3' } } }").unwrap();
        assert_eq!(c.pane_of(Layout::Split2, 0), "video");
        assert_eq!(c.split_of(Layout::Split2).unwrap().id(), "1x2:1/3");
        assert_eq!(c.pane(0), "media", "2x2 : défaut");
        c.set_pane(Layout::Grid4, 3, "calendar");
        assert_eq!(
            c.panes_of(Layout::Grid4),
            vec!["media", "video", "system", "calendar"]
        );
        assert_eq!(c.pane_of(Layout::Split2, 0), "video");
        assert_eq!(eval(&c.to_lua()).unwrap(), c);
    }
}
