//! Composition et apparence du bandeau. Même validation en Lua, CLI et réglages.
use mlua::{Table, Value};

pub const PRESETS: &[&str] = &["cinematic", "forest", "telemetry", "studio", "applications"];
pub const CLOCKS: &[&str] = &["digital", "flip", "analog"];
pub const SYSTEMS: &[&str] = &["cards", "gauges", "graphs"];
pub const MEDIA: &[&str] = &["standard", "vinyl", "compact"];

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
        if !super::PAGES.iter().any(|(id, _)| *id == self.widget)
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
            controls: true,
            clock_style: "digital".into(),
            seconds: false,
            date: true,
            system_style: "cards".into(),
            history_seconds: 60.0,
            media_style: "standard".into(),
            shortcut_columns: 4.0,
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
            "metrics" => {
                let ids: Vec<_> = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if ids.is_empty()
                    || ids.len() > 8
                    || ids.iter().any(|id| {
                        ![
                            "cpu", "ram", "disk", "load", "uptime", "cpu_temp", "gpu_temp",
                        ]
                        .contains(id)
                    })
                {
                    return Err(
                        "Mesures : cpu, ram, disk, load, uptime, cpu_temp, gpu_temp (1 à 8)."
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
        *self = Self::default();
        self.wallpaper = wallpaper;
        self.shortcuts = shortcuts;
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
        d.tiles.clear();
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
            d.tiles.push(tile);
        }
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
        f.push(format!(
            "tiles = {{ {} }}",
            self.tiles
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
        ));
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
            let mut d = Design::default();
            d.wallpaper = "/tmp/a \"b\".jpg".into();
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
    fn bad_values_are_rejected() {
        for src in [
            "{ padding = -1 }",
            "{ text_scale = 0 }",
            "{ accent = 'purple' }",
            "{ clock_style = 'x' }",
            "{ effects = 'yes' }",
            "{ tiles = {} }",
            "{ typo = 1 }",
            "{ shortcuts = {{name='x',command=''}} }",
        ] {
            assert!(read(src).is_err(), "{src}");
        }
        assert!(Tile::new("clock", 0.9, 0.0, 0.2, 1.0).validate().is_err());
    }
}
