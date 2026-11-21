//! Composition et apparence du bandeau. Même validation en Lua, CLI et réglages.
use mlua::{Table, Value};

pub const PRESETS: &[&str] = &["cinematic", "forest", "telemetry", "studio", "applications"];
pub const CLOCKS: &[&str] = &["digital", "flip", "analog"];
pub const SYSTEMS: &[&str] = &["cards", "gauges", "graphs"];
pub const MEDIA: &[&str] = &["standard", "vinyl", "compact"];
/// Mesures système, une par tuile `metric:<id>[:<style>]`.
pub const METRICS: &[&str] = &[
    "cpu", "ram", "gpu", "vram", "disk", "load", "uptime", "cpu_temp", "gpu_temp",
];

/// Libellé d'une mesure.
pub fn metric_label(id: &str) -> &'static str {
    match id {
        "cpu" => "Processeur",
        "ram" => "Mémoire",
        "gpu" => "Carte graphique",
        "vram" => "Mémoire vidéo",
        "disk" => "Disque",
        "load" => "Charge",
        "uptime" => "Allumé depuis",
        "cpu_temp" => "Température CPU",
        "gpu_temp" => "Température GPU",
        _ => "Mesure",
    }
}

/// Tuile d'une mesure : (mesure, présentation ; graphs par défaut).
pub fn metric_of(widget: &str) -> Option<(&str, &str)> {
    let rest = widget.strip_prefix("metric:")?;
    let (id, style) = rest.split_once(':').unwrap_or((rest, "graphs"));
    (METRICS.contains(&id) && SYSTEMS.contains(&style)).then_some((id, style))
}

/// Page du carrousel du Panorama, après la vue par défaut : un nom et sa
/// composition (zones en part de l'écran, comme la composition libre).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub name: String,
    pub tiles: Vec<Tile>,
}

impl Page {
    /// Page d'un seul widget, sur tout l'écran (anciennes pages).
    pub fn single(widget: &str) -> Page {
        Page {
            name: super::page_label(widget).to_string(),
            tiles: vec![Tile::new(widget, 0.0, 0.0, 1.0, 1.0)],
        }
    }

    /// Page Système : le plus de mesures possible, en graphiques.
    pub fn system() -> Page {
        let ids = [
            "cpu", "ram", "gpu", "vram", "cpu_temp", "gpu_temp", "disk", "load",
        ];
        let tiles = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let (c, r) = ((i % 4) as f32, (i / 4) as f32);
                Tile::new(&format!("metric:{id}"), c * 0.25, r * 0.5, 0.25, 0.5)
            })
            .collect();
        Page {
            name: "Système".into(),
            tiles,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Panorama : chaque page a un nom.".into());
        }
        if self.tiles.is_empty() || self.tiles.len() > 16 {
            return Err("Panorama : 1 à 16 zones par page.".into());
        }
        self.tiles.iter().try_for_each(Tile::validate)
    }
}

/// Pages par défaut : la page Système.
pub fn default_pages() -> Vec<Page> {
    vec![Page::system()]
}
/// Profils du bloc de raccourcis : média vidéo, média audio, raccourcis de
/// la personne (la liste personnalisable `shortcuts`, identifiant `system`
/// gardé pour les configurations écrites), puis Système (`power` :
/// alimentation, veille des écrans, mises à jour), en dernier.
pub const PROFILES: &[&str] = &["video", "audio", "system", "power"];

/// Libellé et icône d'un profil, pour le sélecteur du bloc.
pub fn profile_label(id: &str) -> (&'static str, &'static str) {
    match id {
        "video" => ("Vidéo", "video-x-generic-symbolic"),
        "audio" => ("Musique", "audio-x-generic-symbolic"),
        "power" => ("Système", "system-shutdown-symbolic"),
        _ => ("Raccourcis", "preferences-system-symbolic"),
    }
}

/// Commandes lourdes du profil Système : un premier appui arme, le second
/// lance (le Panorama est tactile, un frôlement ne doit pas éteindre).
pub fn needs_confirm(command: &str) -> bool {
    matches!(command.trim(), "systemctl poweroff" | "systemctl reboot")
}

/// Colonne des touches, commune à tous les profils (même place, même taille
/// des boutons du sélecteur d'un profil à l'autre) : (touches, lecteur), en
/// part de la longueur de la surface.
fn menu_split(w: f32, h: f32, pad: f32, gap: f32) -> (f32, f32) {
    let (long, short) = (w.max(h), w.min(h));
    let inner_l = (long - 2.0 * pad).max(1.0);
    // Paysage : le lecteur prend 16:9 de toute la hauteur de l'écran (il
    // touche les bords : 1280×720 sur un bandeau 2560×720). Portrait (bande
    // verticale) : 16:9 de toute la largeur, la playlist profite de la hauteur.
    if h > w {
        // Playlist : ce qu'il faut pour ses lignes (~28 %) ; les touches
        // prennent le reste.
        let p = ((short * 9.0 / 16.0 + gap) / inner_l).clamp(0.15, 0.5);
        let list = 0.28_f32.min(1.0 - p - 0.2);
        (1.0 - p - list, p)
    } else {
        let p = ((short * 16.0 / 9.0 + gap) / inner_l).clamp(0.3, 0.6);
        (0.26_f32.min((1.0 - p) * 0.55), p)
    }
}

/// Composition des profils Vidéo et Musique : le menu à gauche, le lecteur
/// en 16:9 sur toute la hauteur (720p sur un bandeau 2560×720), puis la
/// colonne « playlist » des services. Fractions de la surface (paysage ;
/// un écran portrait les échange, comme toute composition).
pub fn player_tiles(w: f32, h: f32, pad: f32, gap: f32) -> Vec<Tile> {
    let (menu, player) = menu_split(w, h, pad, gap);
    let list = 1.0 - menu - player;
    vec![
        Tile::new("shortcuts", 0.0, 0.0, menu, 1.0),
        Tile::new("video", menu, 0.0, player, 1.0),
        Tile::new("playlist", menu + player, 0.0, list, 1.0),
    ]
}

/// Composition du profil Système : la colonne des touches, puis l'état des
/// mises à jour (paquets par source, avancement) sur le reste.
pub fn power_tiles(w: f32, h: f32, pad: f32, gap: f32) -> Vec<Tile> {
    let (menu, _) = menu_split(w, h, pad, gap);
    vec![
        Tile::new("shortcuts", 0.0, 0.0, menu, 1.0),
        Tile::new("updates", menu, 0.0, 1.0 - menu, 1.0),
    ]
}

/// Composition du profil Raccourcis : la colonne des touches à la même
/// place que dans les autres profils, puis la bande horizontale des modules
/// (widget `modules`, qui défile) sur le reste.
pub fn modules_tiles(w: f32, h: f32, pad: f32, gap: f32) -> Vec<Tile> {
    let (menu, _) = menu_split(w, h, pad, gap);
    vec![
        Tile::new("shortcuts", 0.0, 0.0, menu, 1.0),
        Tile::new("modules", menu, 0.0, 1.0 - menu, 1.0),
    ]
}

/// Touches intégrées d'un profil (`system` : la liste de la personne). Les
/// touches de la télécommande vont à la fenêtre active (`hyprdesk key`).
pub fn profile_shortcuts(id: &str) -> Option<Vec<Shortcut>> {
    let list: &[(&str, &str, &str)] = match id {
        "video" => &[
            (
                "−10 s",
                "media-seek-backward-symbolic",
                "hyprdesk media seek -10",
            ),
            (
                "Lecture",
                "media-playback-start-symbolic",
                "hyprdesk media play-pause",
            ),
            (
                "+10 s",
                "media-seek-forward-symbolic",
                "hyprdesk media seek +10",
            ),
            ("Plein écran", "view-fullscreen-symbolic", "hyprdesk key f"),
            (
                "Volume −",
                "audio-volume-low-symbolic",
                "hyprdesk osd volume -5",
            ),
            (
                "Muet",
                "audio-volume-muted-symbolic",
                "hyprdesk osd volume toggle",
            ),
            (
                "Volume +",
                "audio-volume-high-symbolic",
                "hyprdesk osd volume +5",
            ),
            (
                "Suivant",
                "media-skip-forward-symbolic",
                "hyprdesk media next",
            ),
        ],
        "audio" => &[
            (
                "Précédent",
                "media-skip-backward-symbolic",
                "hyprdesk media previous",
            ),
            (
                "Lecture",
                "media-playback-start-symbolic",
                "hyprdesk media play-pause",
            ),
            (
                "Suivant",
                "media-skip-forward-symbolic",
                "hyprdesk media next",
            ),
            ("Sortie", "audio-speakers-symbolic", "hyprdesk menu volume"),
            (
                "Volume −",
                "audio-volume-low-symbolic",
                "hyprdesk osd volume -5",
            ),
            (
                "Muet",
                "audio-volume-muted-symbolic",
                "hyprdesk osd volume toggle",
            ),
            (
                "Volume +",
                "audio-volume-high-symbolic",
                "hyprdesk osd volume +5",
            ),
            (
                "Micro",
                "microphone-sensitivity-muted-symbolic",
                "hyprdesk osd mic toggle",
            ),
        ],
        "power" => &[
            (
                "Verrouiller",
                "system-lock-screen-symbolic",
                "hyprdesk lock",
            ),
            (
                "Veille des écrans",
                "video-display-symbolic",
                "hyprdesk screens-off",
            ),
            (
                "Mettre en veille",
                "weather-clear-night-symbolic",
                "systemctl suspend",
            ),
            (
                "Mettre à jour",
                "software-update-available-symbolic",
                "hyprdesk updates --upgrade",
            ),
            ("Redémarrer", "system-reboot-symbolic", "systemctl reboot"),
            ("Éteindre", "system-shutdown-symbolic", "systemctl poweroff"),
        ],
        _ => return None,
    };
    Some(
        list.iter()
            .map(|(n, i, c)| Shortcut {
                name: crate::i18n::tr(n).into(),
                icon: (*i).into(),
                command: (*c).into(),
            })
            .collect(),
    )
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tile {
    pub widget: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Tile {
    pub fn new(widget: &str, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            widget: widget.into(),
            x,
            y,
            w,
            h,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        // `playlist` : colonne des services du lecteur (profils Vidéo,
        // Musique) ; `updates` : mises à jour du profil Système.
        if !super::PAGES.iter().any(|(id, _)| *id == self.widget)
            && self.widget != "playlist"
            && self.widget != "updates"
            && self.widget != "modules"
            && metric_of(&self.widget).is_none()
            && !self
                .widget
                .strip_prefix("widget:")
                .is_some_and(|id| !id.trim().is_empty())
        {
            return Err(format!("Widget inconnu : {}", self.widget));
        }
        if ![self.x, self.y, self.w, self.h]
            .iter()
            .all(|n| n.is_finite())
            || self.x < 0.0
            || self.y < 0.0
            || self.w < 0.05
            || self.h < 0.05
            || self.x + self.w > 1.00001
            || self.y + self.h > 1.00001
        {
            return Err("La zone doit rester dans le bandeau (coordonnées de 0 à 1).".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shortcut {
    pub name: String,
    pub icon: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Design {
    pub preset: String,
    pub wallpaper: String,
    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub font_family: String,
    pub dimming: f32,
    pub card_opacity: f32,
    pub border_opacity: f32,
    pub radius: f32,
    pub gap: f32,
    pub padding: f32,
    pub text_scale: f32,
    pub effects: bool,
    pub controls: bool,
    pub clock_style: String,
    pub seconds: bool,
    pub date: bool,
    pub system_style: String,
    pub metrics: String,
    pub history_seconds: f32,
    pub media_style: String,
    pub shortcut_columns: f32,
    /// Profil du bloc de raccourcis (`PROFILES`).
    pub shortcut_profile: String,
    pub tiles: Vec<Tile>,
    pub shortcuts: Vec<Shortcut>,
}

impl Default for Design {
    fn default() -> Self {
        Self {
            preset: "cinematic".into(),
            wallpaper: String::new(),
            background: String::new(),
            foreground: String::new(),
            accent: String::new(),
            font_family: String::new(),
            dimming: 0.35,
            card_opacity: 0.15,
            border_opacity: 0.15,
            radius: 18.0,
            gap: 12.0,
            padding: 20.0,
            text_scale: 1.0,
            effects: true,
            controls: false,
            clock_style: "digital".into(),
            seconds: false,
            date: true,
            system_style: "cards".into(),
            history_seconds: 60.0,
            media_style: "standard".into(),
            shortcut_columns: 4.0,
            shortcut_profile: "system".into(),
            metrics: "cpu,ram,disk,load,uptime".into(),
            tiles: vec![
                Tile::new("clock", 0.0, 0.0, 0.25, 1.0),
                Tile::new("media", 0.27, 0.0, 0.46, 1.0),
                Tile::new("shortcuts", 0.75, 0.0, 0.25, 1.0),
            ],
            shortcuts: [
                ("Applications", "view-app-grid-symbolic", "hyprdesk apps"),
                ("Recherche", "system-search-symbolic", "hyprdesk search"),
                ("Fichiers", "system-file-manager-symbolic", "hyprdesk files"),
                (
                    "Réglages",
                    "preferences-system-symbolic",
                    "hyprdesk settings panorama",
                ),
                (
                    "Lecture",
                    "media-playback-start-symbolic",
                    "hyprdesk media play-pause",
                ),
                (
                    "Volume",
                    "audio-volume-high-symbolic",
                    "hyprdesk menu volume",
                ),
                (
                    "Calendrier",
                    "x-office-calendar-symbolic",
                    "hyprdesk menu clock",
                ),
                ("Bureau", "user-desktop-symbolic", "hyprdesk toggle"),
                (
                    "Réseau",
                    "network-wireless-symbolic",
                    "hyprdesk settings network",
                ),
                ("Bluetooth", "bluetooth-symbolic", "hyprdesk settings bt"),
                (
                    "Luminosité",
                    "display-brightness-symbolic",
                    "hyprdesk settings brightness",
                ),
                (
                    "Notifications",
                    "preferences-system-notifications-symbolic",
                    "hyprdesk settings notifications",
                ),
            ]
            .into_iter()
            .map(|(n, i, c)| Shortcut {
                name: n.into(),
                icon: i.into(),
                command: c.into(),
            })
            .collect(),
        }
    }
}

impl Design {
    /// Touches du profil choisi : intégrées, ou la liste de la personne
    /// (Système).
    pub fn active_shortcuts(&self) -> std::borrow::Cow<'_, [Shortcut]> {
        match profile_shortcuts(&self.shortcut_profile) {
            Some(l) => l.into(),
            None => self.shortcuts.as_slice().into(),
        }
    }

    /// Composition affichée : celle du profil Vidéo ou Musique (menu, lecteur
    /// 16:9 sur toute la hauteur, colonne des services), sinon la sienne.
    /// `w`, `h` : taille de la surface, pour un lecteur au format 16:9.
    pub fn active_tiles(&self, w: f32, h: f32) -> std::borrow::Cow<'_, [Tile]> {
        match self.shortcut_profile.as_str() {
            "video" | "audio" => player_tiles(w, h, self.padding, self.gap).into(),
            "power" => power_tiles(w, h, self.padding, self.gap).into(),
            _ => modules_tiles(w, h, self.padding, self.gap).into(),
        }
    }

    /// Modules de la bande du profil Raccourcis, dans l'ordre (la
    /// composition de la personne lue de gauche à droite).
    pub fn modules(&self) -> Vec<String> {
        let mut t: Vec<&Tile> = self
            .tiles
            .iter()
            .filter(|t| {
                super::modules::CATALOGUE
                    .iter()
                    .any(|(id, _, _)| *id == t.widget)
            })
            .collect();
        t.sort_by(|a, b| {
            (a.x, a.y)
                .partial_cmp(&(b.x, b.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut out: Vec<String> = Vec::new();
        for x in t {
            if !out.contains(&x.widget) {
                out.push(x.widget.clone());
            }
        }
        out
    }

    /// Ajoute (au bout) ou retire un module de la bande ; la composition
    /// devient une rangée, dans l'ordre.
    pub fn toggle_module(&mut self, id: &str) {
        let mut list = self.modules();
        if let Some(i) = list.iter().position(|m| m == id) {
            list.remove(i);
        } else if super::modules::CATALOGUE.iter().any(|(m, _, _)| *m == id) {
            list.push(id.to_string());
        }
        let n = list.len().max(1) as f32;
        self.tiles = list
            .iter()
            .enumerate()
            .map(|(i, m)| Tile::new(m, i as f32 / n, 0.0, 1.0 / n, 1.0))
            .collect();
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.text(key)
            .map(str::to_string)
            .or_else(|| self.number(key).map(|n| n.to_string()))
            .or_else(|| self.flag(key).map(|v| v.to_string()))
            .or_else(|| match key {
                "tiles" => serde_json::to_string(&self.tiles).ok(),
                "shortcuts" => serde_json::to_string(&self.shortcuts).ok(),
                _ => None,
            })
    }
    pub fn text(&self, key: &str) -> Option<&str> {
        Some(match key {
            "preset" => &self.preset,
            "wallpaper" => &self.wallpaper,
            "background" => &self.background,
            "foreground" => &self.foreground,
            "accent" => &self.accent,
            "font_family" => &self.font_family,
            "clock_style" => &self.clock_style,
            "system_style" => &self.system_style,
            "media_style" => &self.media_style,
            "metrics" => &self.metrics,
            "shortcut_profile" => &self.shortcut_profile,
            _ => return None,
        })
    }
    pub fn number(&self, key: &str) -> Option<f32> {
        Some(match key {
            "dimming" => self.dimming,
            "card_opacity" => self.card_opacity,
            "border_opacity" => self.border_opacity,
            "radius" => self.radius,
            "gap" => self.gap,
            "padding" => self.padding,
            "text_scale" => self.text_scale,
            "history_seconds" => self.history_seconds,
            "shortcut_columns" => self.shortcut_columns,
            _ => return None,
        })
    }
    pub fn flag(&self, key: &str) -> Option<bool> {
        Some(match key {
            "effects" => self.effects,
            "controls" => self.controls,
            "seconds" => self.seconds,
            "date" => self.date,
            _ => return None,
        })
    }
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let choice = |options: &[&str]| {
            if options.contains(&value) {
                Ok(value.to_string())
            } else {
                Err(format!("{key} : {}", options.join(", ")))
            }
        };
        match key {
            "tiles" => {
                let tiles: Vec<Tile> = serde_json::from_str(value).map_err(|e| e.to_string())?;
                if tiles.is_empty() || tiles.len() > 16 {
                    return Err("Panorama : 1 à 16 zones attendues.".into());
                }
                for t in &tiles {
                    t.validate()?;
                }
                self.tiles = tiles;
            }
            "shortcuts" => {
                let shortcuts: Vec<Shortcut> =
                    serde_json::from_str(value).map_err(|e| e.to_string())?;
                if shortcuts.len() > 64
                    || shortcuts
                        .iter()
                        .any(|s| s.command.trim().is_empty() || s.name.trim().is_empty())
                {
                    return Err(
                        "Panorama : noms et commandes requis, 64 raccourcis au maximum.".into(),
                    );
                }
                self.shortcuts = shortcuts;
            }
            "preset" => self.preset = choice(PRESETS)?,
            "clock_style" => self.clock_style = choice(CLOCKS)?,
            "system_style" => self.system_style = choice(SYSTEMS)?,
            "media_style" => self.media_style = choice(MEDIA)?,
            // Ancien profil Télécommande (retiré) : le profil Vidéo.
            "shortcut_profile" if value == "remote" => self.shortcut_profile = "video".into(),
            "shortcut_profile" => self.shortcut_profile = choice(PROFILES)?,
            "metrics" => {
                let ids: Vec<_> = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if ids.is_empty() || ids.len() > 8 || ids.iter().any(|id| !METRICS.contains(id)) {
                    return Err(
                        "Mesures : cpu, ram, gpu, vram, disk, load, uptime, cpu_temp, gpu_temp (1 à 8)."
                            .into(),
                    );
                }
                self.metrics = ids.join(",");
            }
            "wallpaper" => self.wallpaper = value.trim().into(),
            "font_family" => self.font_family = value.trim().into(),
            "background" | "foreground" | "accent" => {
                let v = value.trim();
                if !v.is_empty() && crate::config::Rgba::parse(v).is_none() {
                    return Err("Couleur attendue : #rrggbb ou #rrggbbaa (vide : thème).".into());
                }
                match key {
                    "background" => self.background = v.into(),
                    "foreground" => self.foreground = v.into(),
                    _ => self.accent = v.into(),
                }
            }
            "effects" | "controls" | "seconds" | "date" => {
                let v = match value {
                    "true" | "on" | "1" => true,
                    "false" | "off" | "0" => false,
                    _ => return Err("Valeur attendue : on ou off.".into()),
                };
                match key {
                    "effects" => self.effects = v,
                    "controls" => self.controls = v,
                    "seconds" => self.seconds = v,
                    _ => self.date = v,
                }
            }
            "dimming" | "card_opacity" | "border_opacity" | "radius" | "gap" | "padding"
            | "text_scale" | "history_seconds" | "shortcut_columns" => {
                let v: f32 = value.parse().map_err(|_| "Nombre attendu.".to_string())?;
                let (min, max) = match key {
                    "dimming" | "card_opacity" | "border_opacity" => (0.0, 1.0),
                    "text_scale" => (0.5, 2.0),
                    "history_seconds" => (15.0, 600.0),
                    "shortcut_columns" => (2.0, 8.0),
                    _ => (0.0, 64.0),
                };
                if !v.is_finite() || !(min..=max).contains(&v) {
                    return Err(format!("{key} : {min} à {max}."));
                }
                match key {
                    "dimming" => self.dimming = v,
                    "card_opacity" => self.card_opacity = v,
                    "border_opacity" => self.border_opacity = v,
                    "radius" => self.radius = v,
                    "gap" => self.gap = v,
                    "padding" => self.padding = v,
                    "text_scale" => self.text_scale = v,
                    "history_seconds" => self.history_seconds = v.round(),
                    _ => self.shortcut_columns = v.round(),
                }
            }
            _ => return Err(format!("Réglage Panorama inconnu : {key}")),
        }
        Ok(())
    }
    pub fn apply_preset(&mut self, id: &str) -> Result<(), String> {
        if !PRESETS.contains(&id) {
            return Err("Préréglage Panorama inconnu.".into());
        }
        // Les images et les raccourcis de la personne sont conservés.
        let wallpaper = self.wallpaper.clone();
        let shortcuts = self.shortcuts.clone();
        let profile = self.shortcut_profile.clone();
        *self = Self::default();
        self.wallpaper = wallpaper;
        self.shortcuts = shortcuts;
        self.shortcut_profile = profile;
        self.preset = id.into();
        if id == "cinematic" {
            return Ok(());
        }
        self.background = "#080e1b".into();
        self.foreground = "#f1f5ff".into();
        self.accent = "#70ceff".into();
        self.effects = false;
        self.controls = false;
        self.card_opacity = 0.05;
        self.border_opacity = 0.0;
        self.font_family = "DejaVu Sans".into();
        match id {
            "forest" => {
                self.background = "#0c2118".into();
                self.accent = "#a4e2d7".into();
                self.clock_style = "analog".into();
                self.system_style = "graphs".into();
                self.metrics = "cpu_temp,gpu_temp".into();
                self.tiles = vec![
                    Tile::new("shortcuts", 0.0, 0.0, 0.28, 1.0),
                    Tile::new("system", 0.30, 0.0, 0.34, 1.0),
                    Tile::new("clock", 0.66, 0.0, 0.34, 1.0),
                ];
            }
            "telemetry" => {
                self.accent = "#ffd84b".into();
                self.clock_style = "flip".into();
                self.seconds = true;
                self.system_style = "gauges".into();
                self.metrics = "cpu_temp,cpu,ram,gpu_temp".into();
                self.tiles = vec![
                    Tile::new("clock", 0.0, 0.0, 0.28, 0.45),
                    Tile::new("system", 0.30, 0.0, 0.42, 1.0),
                    Tile::new("calendar", 0.74, 0.0, 0.26, 1.0),
                    Tile::new("media", 0.0, 0.5, 0.28, 0.5),
                ];
                self.media_style = "compact".into();
            }
            "studio" => {
                self.background = "#111032".into();
                self.accent = "#a4aeff".into();
                self.media_style = "vinyl".into();
                self.seconds = true;
            }
            "applications" => {
                self.tiles = vec![
                    Tile::new("video", 0.0, 0.0, 0.60, 1.0),
                    Tile::new("media", 0.62, 0.0, 0.38, 1.0),
                ];
                self.media_style = "compact".into();
            }
            _ => {}
        }
        Ok(())
    }
}

const TEXTS: &[&str] = &[
    "preset",
    "wallpaper",
    "background",
    "foreground",
    "accent",
    "font_family",
    "clock_style",
    "system_style",
    "media_style",
    "metrics",
    "shortcut_profile",
];
const NUMBERS: &[&str] = &[
    "dimming",
    "card_opacity",
    "border_opacity",
    "radius",
    "gap",
    "padding",
    "text_scale",
    "history_seconds",
    "shortcut_columns",
];
const FLAGS: &[&str] = &["effects", "controls", "seconds", "date"];
fn err(s: String) -> mlua::Error {
    mlua::Error::RuntimeError(s)
}
pub fn from_lua(t: &Table) -> mlua::Result<Design> {
    let mut d = Design::default();
    if let Some(id) = t.get::<Option<String>>("preset")? {
        d.apply_preset(&id).map_err(err)?;
    }
    for pair in t.pairs::<String, Value>() {
        let (key, v) = pair?;
        if key == "tiles" || key == "shortcuts" {
            continue;
        }
        let value = if TEXTS.contains(&key.as_str()) {
            match v {
                Value::String(s) => s.to_str()?.to_string(),
                _ => return Err(err(format!("panorama.design.{key} : chaîne attendue."))),
            }
        } else if NUMBERS.contains(&key.as_str()) {
            match v {
                Value::Integer(n) => n.to_string(),
                Value::Number(n) => n.to_string(),
                _ => return Err(err(format!("panorama.design.{key} : nombre attendu."))),
            }
        } else if FLAGS.contains(&key.as_str()) {
            match v {
                Value::Boolean(b) => b.to_string(),
                _ => return Err(err(format!("panorama.design.{key} : booléen attendu."))),
            }
        } else {
            return Err(err(format!("panorama.design : clé inconnue {key}")));
        };
        d.set(&key, &value).map_err(err)?;
    }
    if let Some(tiles) = t.get::<Option<Table>>("tiles")? {
        d.tiles = tiles_from_lua(&tiles)?;
        if d.tiles.is_empty() || d.tiles.len() > 16 {
            return Err(err("Panorama : 1 à 16 zones attendues.".into()));
        }
    }
    if let Some(list) = t.get::<Option<Table>>("shortcuts")? {
        d.shortcuts.clear();
        for row in list.sequence_values::<Table>() {
            let row = row?;
            let command: String = row.get("command")?;
            if command.trim().is_empty() {
                return Err(err("Panorama : commande de raccourci vide.".into()));
            }
            d.shortcuts.push(Shortcut {
                name: row.get("name")?,
                icon: row
                    .get::<Option<String>>("icon")?
                    .unwrap_or_else(|| "application-x-executable".into()),
                command,
            });
        }
        if d.shortcuts.len() > 64 {
            return Err(err("Panorama : 64 raccourcis au maximum.".into()));
        }
    }
    Ok(d)
}
/// Zones d'une composition, lues en Lua et validées.
pub fn tiles_from_lua(tiles: &Table) -> mlua::Result<Vec<Tile>> {
    let mut out = Vec::new();
    for row in tiles.sequence_values::<Table>() {
        let row = row?;
        let tile = Tile::new(
            &row.get::<String>("widget")?,
            row.get("x")?,
            row.get("y")?,
            row.get("w")?,
            row.get("h")?,
        );
        tile.validate().map_err(err)?;
        out.push(tile);
    }
    Ok(out)
}

/// Zones d'une composition en Lua.
pub fn tiles_lua(tiles: &[Tile]) -> String {
    format!(
        "{{ {} }}",
        tiles
            .iter()
            .map(|t| format!(
                "{{ widget = {}, x = {}, y = {}, w = {}, h = {} }}",
                q(&t.widget),
                t.x,
                t.y,
                t.w,
                t.h
            ))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Pages en Lua.
pub fn pages_lua(pages: &[Page]) -> String {
    format!(
        "{{ {} }}",
        pages
            .iter()
            .map(|p| format!(
                "{{ name = {}, tiles = {} }}",
                q(&p.name),
                tiles_lua(&p.tiles)
            ))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Pages lues en Lua : une table { name, tiles } par page, ou le nom d'un
/// widget (page d'un seul widget, ancienne forme).
pub fn pages_from_lua(l: &Table) -> mlua::Result<Vec<Page>> {
    let mut out = Vec::new();
    for v in l.sequence_values::<Value>() {
        match v? {
            Value::String(s) => {
                let id = s.to_str()?.trim().to_string();
                if super::PAGES.iter().any(|(p, _)| *p == id) {
                    out.push(Page::single(&id));
                }
            }
            Value::Table(p) => {
                let page = Page {
                    name: p
                        .get::<Option<String>>("name")?
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                    tiles: match p.get::<Option<Table>>("tiles")? {
                        Some(t) => tiles_from_lua(&t)?,
                        None => Vec::new(),
                    },
                };
                page.validate().map_err(err)?;
                out.push(page);
            }
            _ => return Err(err("panorama.pages : page { name, tiles } attendue.".into())),
        }
    }
    Ok(out)
}

fn q(s: &str) -> String {
    format!("\"{}\"", hyprpanel_core::ipc::lua_escape(s))
}
impl Design {
    pub fn to_lua(&self) -> String {
        let mut f = Vec::new();
        for key in TEXTS {
            f.push(format!("{key} = {}", q(self.text(key).unwrap())));
        }
        for key in NUMBERS {
            f.push(format!("{key} = {}", self.number(key).unwrap()));
        }
        for key in FLAGS {
            f.push(format!("{key} = {}", self.flag(key).unwrap()));
        }
        f.push(format!("tiles = {}", tiles_lua(&self.tiles)));
        f.push(format!(
            "shortcuts = {{ {} }}",
            self.shortcuts
                .iter()
                .map(|s| format!(
                    "{{ name = {}, icon = {}, command = {} }}",
                    q(&s.name),
                    q(&s.icon),
                    q(&s.command)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        format!("{{ {} }}", f.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(src: &str) -> mlua::Result<Design> {
        from_lua(&mlua::Lua::new().load(src).eval::<Table>()?)
    }
    #[test]
    fn presets_round_trip_and_keep_personal_content() {
        for id in PRESETS {
            let mut d = Design {
                wallpaper: "/tmp/a \"b\".jpg".into(),
                ..Default::default()
            };
            d.shortcuts.clear();
            d.shortcuts.push(Shortcut {
                name: "Éditeur".into(),
                icon: "code".into(),
                command: "code --new-window".into(),
            });
            d.apply_preset(id).unwrap();
            for t in &d.tiles {
                t.validate().unwrap();
            }
            assert_eq!(read(&d.to_lua()).unwrap(), d);
            assert_eq!(d.shortcuts.len(), 1);
            assert_eq!(d.wallpaper, "/tmp/a \"b\".jpg");
        }
    }
    #[test]
    fn profils_de_raccourcis() {
        let mut d = Design::default();
        assert_eq!(d.shortcut_profile, "system");
        assert_eq!(d.active_shortcuts().len(), d.shortcuts.len());
        for id in PROFILES {
            d.set("shortcut_profile", id).unwrap();
            assert!(!d.active_shortcuts().is_empty(), "{id}");
            assert_eq!(read(&d.to_lua()).unwrap(), d);
        }
        assert!(profile_shortcuts("remote").is_none());
        // Bandeau 1600×450 : lecteur 16:9 sur toute la hauteur de l'écran.
        let t = player_tiles(1600.0, 450.0, 20.0, 12.0);
        let (pw, ph) = (t[1].w * 1560.0 - 12.0, 450.0);
        assert!((pw / ph - 16.0 / 9.0).abs() < 0.01, "{pw}×{ph}");
        assert!((t.iter().map(|t| t.w).sum::<f32>() - 1.0).abs() < 1e-4);
        assert!(t.iter().all(|t| t.validate().is_ok()));
        // Bande verticale 640×1392 : lecteur 16:9 sur la largeur.
        let t = player_tiles(640.0, 1392.0, 20.0, 12.0);
        let (pw, ph) = (640.0, t[1].w * 1352.0 - 12.0);
        assert!((pw / ph - 16.0 / 9.0).abs() < 0.02, "{pw}×{ph}");
        assert!(
            (0.2..=0.3).contains(&t[2].w),
            "playlist mesurée : {}",
            t[2].w
        );
        assert!(t[0].w > t[2].w, "les touches prennent le reste");
        d.set("shortcut_profile", "video").unwrap();
        assert_eq!(d.active_tiles(1600.0, 450.0).len(), 3);
        d.set("shortcut_profile", "remote").unwrap();
        assert_eq!(d.shortcut_profile, "video", "Télécommande retirée : Vidéo");
        // Même colonne de touches dans tous les profils : le sélecteur ne
        // bouge pas d'un profil à l'autre.
        let menu = d.active_tiles(1600.0, 450.0)[0].clone();
        for id in PROFILES {
            d.set("shortcut_profile", id).unwrap();
            let t = d.active_tiles(1600.0, 450.0);
            assert_eq!(t[0], menu, "{id}");
            assert!(t.iter().all(|t| t.validate().is_ok()), "{id}");
            assert!(
                t.iter()
                    .all(|t| t.x + t.w <= 1.0 + 1e-4 && t.y + t.h <= 1.0 + 1e-4),
                "{id}"
            );
        }
        d.set("shortcut_profile", "system").unwrap();
        let t = d.active_tiles(1600.0, 450.0);
        // Colonne des touches, puis la bande des modules jusqu'au bord.
        assert_eq!(t.len(), 2);
        assert_eq!(t[1].widget, "modules");
        assert!((t[1].x + t[1].w - 1.0).abs() < 1e-4);
        assert!(needs_confirm("systemctl poweroff") && !needs_confirm("hyprdesk lock"));
        d.set("shortcut_profile", "system").unwrap();
        // Un préréglage garde le profil choisi.
        d.apply_preset("forest").unwrap();
        assert_eq!(d.shortcut_profile, "system");
        d.set("shortcut_profile", "audio").unwrap();
        d.apply_preset("studio").unwrap();
        assert_eq!(d.shortcut_profile, "audio");
    }

    #[test]
    fn modules_de_la_bande() {
        let mut d = Design {
            tiles: vec![
                Tile::new("hue", 0.36, 0.0, 0.64, 1.0),
                Tile::new("notes", 0.0, 0.0, 0.34, 1.0),
                Tile::new("shortcuts", 0.0, 0.0, 0.2, 1.0),
            ],
            ..Default::default()
        };
        assert_eq!(d.modules(), ["notes", "hue"]);
        d.toggle_module("voice");
        assert_eq!(d.modules(), ["notes", "hue", "voice"]);
        d.toggle_module("notes");
        assert_eq!(d.modules(), ["hue", "voice"]);
        d.toggle_module("inconnu");
        assert_eq!(d.modules(), ["hue", "voice"]);
        assert!(d.tiles.iter().all(|t| t.validate().is_ok()));
    }

    #[test]
    fn bad_values_are_rejected() {
        for src in [
            "{ padding = -1 }",
            "{ text_scale = 0 }",
            "{ accent = 'purple' }",
            "{ clock_style = 'x' }",
            "{ effects = 'yes' }",
            "{ tiles = {} }",
            "{ typo = 1 }",
            "{ shortcut_profile = 'tv' }",
            "{ shortcuts = {{name='x',command=''}} }",
        ] {
            assert!(read(src).is_err(), "{src}");
        }
        assert!(Tile::new("clock", 0.9, 0.0, 0.2, 1.0).validate().is_err());
    }
}
