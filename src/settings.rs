//! Réglages faits à la souris (écran ⚙), enregistrés en Lua GÉNÉRÉ dans
//! `~/.config/hypr/hyprdesk/settings.lua` — comme hyprdmc avec monitors.lua.
//!
//! Ce module est chargé AVANT `hyprdesk.lua` : ce que l'utilisateur écrit à la
//! main garde le dernier mot, et son fichier n'est jamais réécrit. Le Lua
//! généré est la seule source de vérité : l'écran le relit en l'exécutant.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use hyprpanel_core::ipc::lua_escape;

use std::collections::BTreeMap;

use crate::config::{
    Anchor, BarItem, BarPosition, Config, DesktopRule, Fit, IconDef, IconKind, Mode, Origin, Placement, Rgba, RunningApps, Selector, ShowOn, SysItem,
    Transition, WallSource, WallpaperRule, WidgetDef, WidgetKind,
};

/// Module de la barre tel qu'écrit dans settings.lua : son type et ses options.
#[derive(Debug, Clone, PartialEq)]
pub struct BarEntry {
    pub type_id: String,
    /// Options en Lua (`format = "%H:%M"`).
    pub fields: Vec<String>,
}

impl BarEntry {
    pub fn new(type_id: &str) -> Self {
        BarEntry { type_id: type_id.to_string(), fields: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BarSettings {
    pub enabled: bool,
    pub monitors: Option<Vec<String>>,
    pub position: BarPosition,
    pub height: u16,
    /// Gauche, centre, droite.
    pub zones: [Vec<BarEntry>; 3],
}

impl Default for BarSettings {
    /// Barre proposée à la première activation.
    fn default() -> Self {
        BarSettings {
            enabled: true,
            monitors: None,
            position: BarPosition::Top,
            height: 36,
            zones: [
                vec![BarEntry::new("workspaces"), BarEntry::new("window")],
                vec![BarEntry { type_id: "clock".into(), fields: vec![format!("format = {}", q("%a %d %b  %H:%M"))] }],
                vec![
                    BarEntry::new("tray"),
                    BarEntry::new("media"),
                    BarEntry::new("reseau"),
                    BarEntry::new("bluetooth"),
                    BarEntry::new("batterie"),
                    BarEntry::new("volume"),
                    BarEntry::new("notifications"),
                    // Accès au bureau vide, toujours en bout de barre.
                    BarEntry::new("desktop"),
                    BarEntry::new("session"),
                ],
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    /// Barre native, si réglée depuis l'écran ⚙.
    pub bar: Option<BarSettings>,
    pub key_toggle: Option<String>,
    pub key_settings: Option<String>,
    pub key_leave: Option<String>,
    pub suspend_binds: Option<bool>,
    pub float_launched: Option<bool>,
    pub show_on: Option<ShowOn>,
    pub mode: Option<Mode>,
    pub glass_opacity: Option<f32>,
    pub close_on_launch: Option<bool>,
    pub fade_ms: Option<u64>,
    pub transition: Option<Transition>,
    pub crossfade_ms: Option<u64>,
    pub running_apps: Option<RunningApps>,
    pub icon_theme: Option<String>,
    pub icon_size: Option<u16>,
    pub cell: Option<u16>,
    pub accent: Option<Rgba>,
    pub wallpapers: Vec<WallpaperRule>,
    pub desktops: Vec<DesktopRule>,
    pub icons: Vec<IconDef>,
    /// Widgets activés depuis les réglages.
    pub widgets: Vec<WidgetDef>,
    /// Positions imposées aux widgets déclarés dans hyprdesk.lua (`hd.place`).
    pub placements: BTreeMap<String, Placement>,
    /// Widgets de hyprdesk.lua désactivés depuis les réglages : id → type.
    pub disabled: BTreeMap<String, String>,
}

impl Settings {
    /// Reconstruit les réglages depuis la config chargée à partir du SEUL settings.lua.
    pub fn from_config(c: &Config) -> Self {
        let g = &c.general;
        let has = |k: &str| c.keys_settings.contains(k);
        let bar = c.keys_settings.contains("bar").then(|| match &c.bar {
            None => BarSettings { enabled: false, ..BarSettings::default() },
            Some(b) => BarSettings {
                enabled: true,
                monitors: b.monitors.clone(),
                position: b.position,
                height: b.height,
                zones: std::array::from_fn(|z| {
                    b.zones[z]
                        .iter()
                        .map(|it| match it {
                            BarItem::Workspaces => BarEntry::new("workspaces"),
                            BarItem::Window => BarEntry::new("window"),
                            BarItem::Desktop => BarEntry::new("desktop"),
                            BarItem::Volume => BarEntry::new("volume"),
                            BarItem::Tray => BarEntry::new("tray"),
                            BarItem::Widget(id) => match c.widgets.iter().find(|w| &w.id == id) {
                                Some(w) => BarEntry { type_id: w.type_id.clone(), fields: widget_fields(&w.kind) },
                                None => BarEntry::new("clock"),
                            },
                        })
                        .collect()
                }),
            },
        });
        Settings {
            bar,
            key_toggle: has("keys.toggle").then(|| g.keys.toggle.clone()),
            key_settings: has("keys.settings").then(|| g.keys.settings.clone()),
            key_leave: has("keys.leave").then(|| g.keys.leave.clone()),
            suspend_binds: has("suspend_binds").then_some(g.suspend_binds),
            float_launched: has("float_launched").then_some(g.float_launched),
            show_on: has("show_on").then_some(g.show_on),
            mode: has("mode").then_some(g.mode),
            glass_opacity: has("glass_opacity").then_some(g.glass_opacity),
            close_on_launch: has("close_on_launch").then_some(g.close_on_launch),
            fade_ms: has("fade_ms").then_some(g.fade_ms),
            transition: has("transition").then_some(g.transition),
            crossfade_ms: has("crossfade_ms").then_some(g.crossfade_ms),
            running_apps: has("running_apps").then_some(g.running_apps),
            icon_theme: if has("icon_theme") { g.icon_theme.clone() } else { None },
            icon_size: has("grid.icon_size").then_some(g.grid.icon_size),
            cell: has("grid.cell").then_some(g.grid.cell),
            accent: has("theme.accent").then_some(g.theme.accent),
            wallpapers: c.wallpapers.iter().filter(|w| w.origin == Origin::Settings).cloned().collect(),
            desktops: c.desktops.iter().filter(|d| d.origin == Origin::Settings).cloned().collect(),
            icons: c.icons.iter().filter(|i| i.origin == Origin::Settings).cloned().collect(),
            widgets: c.widgets.iter().filter(|w| w.origin == Origin::Settings).cloned().collect(),
            placements: c
                .placements
                .iter()
                .filter(|(id, _)| !c.widgets.iter().any(|w| &w.id == *id && w.origin == Origin::Settings))
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            disabled: c.disabled.clone(),
        }
    }

    /// Active ou désactive un type de widget, quelle que soit l'origine de ses
    /// instances. `user_ids` : les widgets de ce type déclarés dans hyprdesk.lua.
    pub fn set_widget_type(&mut self, type_id: &str, on: bool, anchor: Anchor, user_ids: &[String], taken: &[String]) -> Option<String> {
        if on {
            let was_disabled: Vec<String> = self.disabled.iter().filter(|(_, t)| *t == type_id).map(|(id, _)| id.clone()).collect();
            if !was_disabled.is_empty() {
                for id in was_disabled {
                    self.disabled.remove(&id);
                }
                return None;
            }
            Some(self.add_widget(type_id, anchor, taken))
        } else {
            self.widgets.retain(|w| w.type_id != type_id);
            for id in user_ids {
                self.disabled.insert(id.clone(), type_id.to_string());
            }
            None
        }
    }

    /// Active un type de widget (une instance, sur tous les bureaux). Rend son id.
    pub fn add_widget(&mut self, type_id: &str, anchor: Anchor, taken: &[String]) -> String {
        let mut id = type_id.to_string();
        let mut n = 2;
        while taken.contains(&id) || self.widgets.iter().any(|w| w.id == id) {
            id = format!("{type_id}-{n}");
            n += 1;
        }
        self.widgets.push(WidgetDef {
            origin: Origin::Settings,
            sel: Selector::default(),
            id: id.clone(),
            type_id: type_id.to_string(),
            // Le contenu réel vient du rechargement : seul le type est écrit.
            kind: WidgetKind::Calendar,
            anchor,
            x: 0.0,
            y: 0.0,
            size: 0.0,
            interval: std::time::Duration::from_secs(60),
            card: true,
            in_bar: false,
        });
        id
    }

    #[cfg(test)]
    pub fn remove_widget(&mut self, id: &str) {
        self.widgets.retain(|w| w.id != id);
    }

    /// Nouveau placement d'un widget : dans sa déclaration s'il vient des
    /// réglages, sinon en `hd.place` (le fichier de l'utilisateur n'est pas touché).
    pub fn place_widget(&mut self, id: &str, p: Placement) {
        if let Some(w) = self.widgets.iter_mut().find(|w| w.id == id) {
            w.set_placement(&p);
        } else {
            self.placements.insert(id.to_string(), p);
        }
    }

    /// Config effective avec ces réglages, sans repasser par Lua : les règles
    /// des réglages remplacent celles de l'ancien settings.lua (toujours en
    /// tête, comme au chargement), et une clé posée par l'utilisateur dans
    /// hyprdesk.lua garde sa valeur.
    pub fn apply_to(&self, cfg: &Config) -> Config {
        let mut c = cfg.clone();
        let free = |k: &str| !cfg.keys_user.contains(k);
        let g = &mut c.general;
        macro_rules! set {
            ($field:ident, $key:literal, $target:expr) => {
                if let Some(v) = self.$field.clone()
                    && free($key)
                {
                    $target = v;
                }
            };
        }
        set!(key_toggle, "keys.toggle", g.keys.toggle);
        set!(key_settings, "keys.settings", g.keys.settings);
        set!(key_leave, "keys.leave", g.keys.leave);
        set!(suspend_binds, "suspend_binds", g.suspend_binds);
        set!(float_launched, "float_launched", g.float_launched);
        set!(show_on, "show_on", g.show_on);
        set!(mode, "mode", g.mode);
        set!(glass_opacity, "glass_opacity", g.glass_opacity);
        set!(close_on_launch, "close_on_launch", g.close_on_launch);
        set!(fade_ms, "fade_ms", g.fade_ms);
        set!(transition, "transition", g.transition);
        set!(crossfade_ms, "crossfade_ms", g.crossfade_ms);
        set!(running_apps, "running_apps", g.running_apps);
        set!(icon_size, "grid.icon_size", g.grid.icon_size);
        set!(cell, "grid.cell", g.grid.cell);
        set!(accent, "theme.accent", g.theme.accent);
        if let Some(t) = &self.icon_theme
            && free("icon_theme")
        {
            g.icon_theme = Some(t.clone());
        }
        let user = |o: Origin| o == Origin::User;
        c.wallpapers = self.wallpapers.iter().cloned().chain(cfg.wallpapers.iter().filter(|w| user(w.origin)).cloned()).collect();
        c.desktops = self.desktops.iter().cloned().chain(cfg.desktops.iter().filter(|d| user(d.origin)).cloned()).collect();
        c.icons = self.icons.iter().cloned().chain(cfg.icons.iter().filter(|i| user(i.origin)).cloned()).collect();
        // Les widgets des réglages gardent leur contenu chargé (fonctions Lua des
        // plugins) : seul le placement vient des réglages. Les activations et
        // retraits passent par un rechargement.
        for w in c.widgets.iter_mut() {
            let p = match w.origin {
                Origin::Settings => self.widgets.iter().find(|s| s.id == w.id).map(|s| s.placement()),
                Origin::User => self.placements.get(&w.id).copied(),
            };
            if let Some(p) = p {
                w.set_placement(&p);
            }
        }
        c.placements = self.placements.clone();
        c.widgets.retain(|w| !self.disabled.contains_key(&w.id));
        c.disabled = self.disabled.clone();
        c
    }

    pub fn set_wallpaper(&mut self, sel: Selector, source: WallSource, fit: Fit) {
        self.wallpapers.retain(|w| w.sel != sel);
        self.wallpapers.push(WallpaperRule { origin: Origin::Settings, sel, source, fit });
    }

    pub fn remove_wallpaper(&mut self, sel: &Selector) {
        self.wallpapers.retain(|w| &w.sel != sel);
    }

    pub fn wallpaper(&self, sel: &Selector) -> Option<&WallpaperRule> {
        self.wallpapers.iter().find(|w| &w.sel == sel)
    }

    /// Mode d'un workspace ; `None` le rend au mode par défaut.
    pub fn set_workspace_mode(&mut self, workspace: &str, mode: Option<Mode>) {
        let sel = Selector { workspaces: Some(vec![workspace.to_string()]), monitors: None };
        self.desktops.retain(|d| d.sel != sel);
        if let Some(m) = mode {
            self.desktops.push(DesktopRule { origin: Origin::Settings, sel, mode: Some(m), inherit: true });
        }
    }

    pub fn workspace_mode(&self, workspace: &str) -> Option<Mode> {
        let sel = Selector { workspaces: Some(vec![workspace.to_string()]), monitors: None };
        self.desktops.iter().find(|d| d.sel == sel).and_then(|d| d.mode)
    }

    pub fn add_app(&mut self, app: &str, workspace: Option<&str>) {
        let sel = Selector { workspaces: workspace.map(|w| vec![w.to_string()]), monitors: None };
        let id = format!("app:{app}");
        if self.icons.iter().any(|i| i.id == id && i.sel == sel) {
            return;
        }
        self.icons.push(IconDef {
            origin: Origin::Settings,
            sel,
            id,
            label: None,
            kind: IconKind::App(app.to_string()),
            icon: None,
            pos: None,
        });
    }

    pub fn remove_icon(&mut self, idx: usize) {
        if idx < self.icons.len() {
            self.icons.remove(idx);
        }
    }

    pub fn to_lua(&self) -> String {
        let mut o = String::new();
        let _ = writeln!(o, "-- Généré par hyprdesk (écran Réglages ⚙) — NE PAS ÉDITER : réécrit à chaque enregistrement.");
        let _ = writeln!(o, "-- Chargé AVANT ~/.config/hypr/hyprdesk.lua, qui garde le dernier mot.");
        let _ = writeln!(o);

        let mut cfg: Vec<String> = Vec::new();
        let mut grid: Vec<String> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        for (name, v) in [("toggle", &self.key_toggle), ("settings", &self.key_settings), ("leave", &self.key_leave)] {
            if let Some(k) = v {
                keys.push(if k.is_empty() { format!("{name} = false") } else { format!("{name} = {}", q(k)) });
            }
        }
        if !keys.is_empty() {
            cfg.push(format!("keys = {{ {} }}", keys.join(", ")));
        }
        if let Some(v) = self.suspend_binds {
            cfg.push(format!("suspend_binds = {v}"));
        }
        if let Some(v) = self.float_launched {
            cfg.push(format!("float_launched = {v}"));
        }
        if let Some(v) = self.show_on {
            cfg.push(format!("show_on = {}", q(if v == ShowOn::All { "all" } else { "focused" })));
        }
        if let Some(v) = self.mode {
            cfg.push(format!("mode = {}", q(mode_str(v))));
        }
        if let Some(v) = self.glass_opacity {
            cfg.push(format!("glass_opacity = {}", num(v)));
        }
        if let Some(v) = self.close_on_launch {
            cfg.push(format!("close_on_launch = {v}"));
        }
        if let Some(v) = self.transition {
            cfg.push(format!("transition = {}", q(v.as_str())));
        }
        if let Some(v) = self.fade_ms {
            cfg.push(format!("fade_ms = {v}"));
        }
        if let Some(v) = self.crossfade_ms {
            cfg.push(format!("crossfade_ms = {v}"));
        }
        if let Some(v) = self.running_apps {
            cfg.push(format!("running_apps = {}", q(v.as_str())));
        }
        if let Some(v) = &self.icon_theme {
            cfg.push(format!("icon_theme = {}", q(v)));
        }
        if let Some(v) = self.icon_size {
            grid.push(format!("icon_size = {v}"));
        }
        if let Some(v) = self.cell {
            grid.push(format!("cell = {v}"));
        }
        if !grid.is_empty() {
            cfg.push(format!("grid = {{ {} }}", grid.join(", ")));
        }
        if let Some(c) = self.accent {
            cfg.push(format!("theme = {{ accent = {} }}", q(&hex(c))));
        }
        if !cfg.is_empty() {
            let _ = writeln!(o, "hd.config({{");
            for line in cfg {
                let _ = writeln!(o, "    {line},");
            }
            let _ = writeln!(o, "}})\n");
        }

        for w in &self.wallpapers {
            let mut f = selector_fields(&w.sel);
            match &w.source {
                WallSource::Path(p) => f.push(format!("path = {}", q(&tilde(p)))),
                WallSource::Color(c) => f.push(format!("color = {}", q(&hex(*c)))),
                WallSource::Rotate { dir, interval, shuffle } => f.push(format!(
                    "rotate = {{ dir = {}, interval = {}, shuffle = {shuffle} }}",
                    q(&tilde(dir)),
                    interval.as_secs()
                )),
            }
            if w.fit != Fit::Cover && !matches!(w.source, WallSource::Color(_)) {
                f.push(format!("fit = {}", q(fit_str(w.fit))));
            }
            let _ = writeln!(o, "hd.wallpaper({{ {} }})", f.join(", "));
        }
        for d in &self.desktops {
            let mut f = selector_fields(&d.sel);
            if let Some(m) = d.mode {
                f.push(format!("mode = {}", q(mode_str(m))));
            }
            if !d.inherit {
                f.push("inherit = false".into());
            }
            let _ = writeln!(o, "hd.desktop({{ {} }})", f.join(", "));
        }
        if let Some(b) = &self.bar {
            if b.enabled {
                let mut f = Vec::new();
                if let Some(m) = &b.monitors {
                    f.push(format!("monitor = {}", list(m)));
                }
                f.push(format!("position = {}", q(if b.position == BarPosition::Top { "top" } else { "bottom" })));
                f.push(format!("height = {}", b.height));
                for (zi, name) in ["left", "center", "right"].iter().enumerate() {
                    let items: Vec<String> = b.zones[zi]
                        .iter()
                        .map(|e| if e.fields.is_empty() { q(&e.type_id) } else { format!("{{ {}, {} }}", q(&e.type_id), e.fields.join(", ")) })
                        .collect();
                    f.push(format!("{name} = {{ {} }}", items.join(", ")));
                }
                let _ = writeln!(o, "hd.bar({{\n    {},\n}})\n", f.join(",\n    "));
            } else {
                let _ = writeln!(o, "hd.bar(false)\n");
            }
        }
        for w in &self.widgets {
            let mut f = selector_fields(&w.sel);
            f.insert(0, format!("id = {}", q(&w.id)));
            f.extend(widget_fields(&w.kind));
            f.extend(placement_fields(&w.placement()));
            let _ = writeln!(o, "hd.widget({}, {{ {} }})", q(&w.type_id), f.join(", "));
        }
        for (id, p) in &self.placements {
            let _ = writeln!(o, "hd.place({}, {{ {} }})", q(id), placement_fields(p).join(", "));
        }
        for id in self.disabled.keys() {
            let _ = writeln!(o, "hd.disable({})", q(id));
        }
        for i in &self.icons {
            let mut f = selector_fields(&i.sel);
            match &i.kind {
                IconKind::App(a) => f.push(format!("app = {}", q(a))),
                IconKind::Path(p) => f.push(format!("path = {}", q(&tilde(p)))),
                IconKind::Exec { cmd, terminal } => {
                    f.push(format!("exec = {}", q(cmd)));
                    if *terminal {
                        f.push("terminal = true".into());
                    }
                }
            }
            if let Some(l) = &i.label {
                f.push(format!("label = {}", q(l)));
            }
            let _ = writeln!(o, "hd.icon({{ {} }})", f.join(", "));
        }
        o
    }

    /// Écriture atomique (fichier temporaire + renommage) : la surveillance de
    /// la config ne voit jamais un fichier à moitié écrit.
    pub fn save(&self, config_path: &Path) -> std::io::Result<PathBuf> {
        self.save_versioned(config_path, crate::storage::revision())
    }

    pub fn save_versioned(&self, config_path: &Path, revision: u64) -> std::io::Result<PathBuf> {
        let p = crate::config::lua::settings_path(config_path);
        crate::storage::write(&p, self.to_lua().as_bytes(), revision)?;
        Ok(p)
    }
}

/// Options propres d'un widget intégré, quand elles diffèrent des valeurs par défaut.
fn widget_fields(k: &WidgetKind) -> Vec<String> {
    match k {
        WidgetKind::Clock { format } if format != "%H:%M" => vec![format!("format = {}", q(format))],
        WidgetKind::Media { player } if player != "*" => vec![format!("player = {}", q(player))],
        WidgetKind::System { show } if !is_default_show(show) => {
            vec![format!("show = {{ {} }}", show.iter().map(|i| q(&sys_item_str(i))).collect::<Vec<_>>().join(", "))]
        }
        _ => Vec::new(),
    }
}

fn placement_fields(p: &Placement) -> Vec<String> {
    vec![
        format!("anchor = {}", q(anchor_str(p.anchor))),
        format!("x = {}", num(p.x)),
        format!("y = {}", num(p.y)),
        format!("size = {}", num(p.size)),
        format!("card = {}", p.card),
    ]
}

pub fn anchor_str(a: Anchor) -> &'static str {
    match a {
        Anchor::TopLeft => "top-left",
        Anchor::Top => "top",
        Anchor::TopRight => "top-right",
        Anchor::Left => "left",
        Anchor::Center => "center",
        Anchor::Right => "right",
        Anchor::BottomLeft => "bottom-left",
        Anchor::Bottom => "bottom",
        Anchor::BottomRight => "bottom-right",
    }
}

fn sys_item_str(i: &SysItem) -> String {
    match i {
        SysItem::Cpu => "cpu".into(),
        SysItem::Ram => "ram".into(),
        SysItem::Load => "load".into(),
        SysItem::Uptime => "uptime".into(),
        SysItem::Disk(m) => format!("disk:{m}"),
    }
}

fn is_default_show(s: &[SysItem]) -> bool {
    s == [SysItem::Cpu, SysItem::Ram, SysItem::Disk("/".into())]
}

fn q(s: &str) -> String {
    format!("\"{}\"", lua_escape(s))
}

fn num(v: f32) -> String {
    format!("{}", (v * 100.0).round() / 100.0)
}

pub fn hex(c: Rgba) -> String {
    if c.3 == 255 { format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2) } else { format!("#{:02x}{:02x}{:02x}{:02x}", c.0, c.1, c.2, c.3) }
}

fn mode_str(m: Mode) -> &'static str {
    match m {
        Mode::Opaque => "opaque",
        Mode::Glass => "glass",
    }
}

pub fn fit_str(f: Fit) -> &'static str {
    match f {
        Fit::Cover => "cover",
        Fit::Contain => "contain",
        Fit::Fill => "fill",
        Fit::Center => "center",
    }
}

/// `$HOME/x` → `~/x`, plus lisible dans le fichier généré.
fn tilde(p: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
        && let Ok(rest) = p.strip_prefix(&home)
    {
        return if rest.as_os_str().is_empty() { "~".into() } else { format!("~/{}", rest.display()) };
    }
    p.display().to_string()
}

fn list(v: &[String]) -> String {
    if v.len() == 1 { q(&v[0]) } else { format!("{{ {} }}", v.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")) }
}

fn selector_fields(s: &Selector) -> Vec<String> {
    let mut f = Vec::new();
    if let Some(w) = &s.workspaces {
        f.push(format!("workspace = {}", list(w)));
    }
    if let Some(m) = &s.monitors {
        f.push(format!("monitor = {}", list(m)));
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::lua::load_chunks;
    use std::time::Duration;

    #[test]
    fn aller_retour_lua() {
        let mut s = Settings {
            key_toggle: Some("SUPER + SHIFT + D".into()),
            key_settings: Some(String::new()),
            suspend_binds: Some(false),
            float_launched: Some(false),
            show_on: Some(ShowOn::Focused),
            mode: Some(Mode::Glass),
            glass_opacity: Some(0.5),
            close_on_launch: Some(false),
            fade_ms: Some(300),
            transition: Some(Transition::Slide),
            crossfade_ms: Some(1200),
            running_apps: Some(RunningApps::Workspace),
            icon_theme: Some("Papirus-Dark".into()),
            icon_size: Some(64),
            cell: Some(112),
            accent: Some(Rgba(0xff, 0x80, 0xbf, 0xff)),
            ..Default::default()
        };
        s.set_wallpaper(Selector::default(), WallSource::Path("/img/l'été \"a\".jpg".into()), Fit::Contain);
        s.set_wallpaper(
            Selector { workspaces: Some(vec!["3".into()]), monitors: None },
            WallSource::Rotate { dir: "/img".into(), interval: Duration::from_secs(600), shuffle: true },
            Fit::Cover,
        );
        s.set_wallpaper(Selector { workspaces: None, monitors: Some(vec!["DP-1".into()]) }, WallSource::Color(Rgba(1, 2, 3, 255)), Fit::Cover);
        s.set_workspace_mode("4", Some(Mode::Opaque));
        s.add_app("firefox", None);
        s.add_app("code", Some("2"));
        s.add_app("firefox", None); // doublon ignoré

        let lua = s.to_lua();
        let cfg = load_chunks(&[(lua.clone(), "settings.lua".into(), Origin::Settings)], Path::new("/cfg")).unwrap_or_else(|e| panic!("{e}\n{lua}")).config;
        assert_eq!(Settings::from_config(&cfg), s, "\n{lua}");
    }

    #[test]
    fn widgets_et_placements_aller_retour() {
        let user = r#"hd.widget("clock", { format = "%H:%M:%S" })"#;
        let mut s = Settings::default();
        let id = s.add_widget("calendar", Anchor::Right, &[]);
        assert_eq!(id, "calendar");
        assert_eq!(s.add_widget("calendar", Anchor::Right, &[]), "calendar-2");
        s.remove_widget("calendar-2");
        s.place_widget("calendar", Placement { anchor: Anchor::Center, x: 12.0, y: -40.0, size: 18.0, card: false });
        s.place_widget("clock#0", Placement { anchor: Anchor::Top, x: 0.0, y: 8.0, size: 0.0, card: true });
        let lua = s.to_lua();
        let chunks = |l: String| vec![(l, "settings.lua".to_string(), Origin::Settings), (user.to_string(), "u.lua".to_string(), Origin::User)];
        let cfg = load_chunks(&chunks(lua.clone()), Path::new("/cfg")).unwrap_or_else(|e| panic!("{e}\n{lua}")).config;
        let cal = cfg.widgets.iter().find(|w| w.id == "calendar").unwrap();
        assert_eq!((cal.anchor, cal.x, cal.y, cal.card), (Anchor::Center, 12.0, -40.0, false));
        let clock = cfg.widgets.iter().find(|w| w.id == "clock#0").unwrap();
        assert_eq!((clock.anchor, clock.y), (Anchor::Top, 8.0));
        let back = Settings::from_config(&cfg);
        assert_eq!(back.widgets.len(), 1);
        assert_eq!(back.placements, s.placements);
        // Déplacement en direct, identique au rechargement.
        let mut moved = back.clone();
        moved.place_widget("clock#0", Placement { anchor: Anchor::BottomLeft, x: 4.0, y: 4.0, size: 30.0, card: false });
        let live = moved.apply_to(&cfg);
        let reloaded = load_chunks(&chunks(moved.to_lua()), Path::new("/cfg")).unwrap().config;
        assert_eq!(live.widgets, reloaded.widgets);
    }

    #[test]
    fn desactiver_un_widget_de_l_utilisateur() {
        let user = r#"hd.widget("clock") hd.widget("calendar")"#;
        let chunks = |l: String| vec![(l, "settings.lua".to_string(), Origin::Settings), (user.to_string(), "u.lua".to_string(), Origin::User)];
        let base = load_chunks(&chunks(String::new()), Path::new("/cfg")).unwrap().config;
        let mut s = Settings::from_config(&base);
        // Éteindre l'horloge (déclarée dans hyprdesk.lua) : elle disparaît, en direct comme au rechargement.
        s.set_widget_type("clock", false, Anchor::TopRight, &["clock#0".to_string()], &[]);
        let live = s.apply_to(&base);
        let reloaded = load_chunks(&chunks(s.to_lua()), Path::new("/cfg")).unwrap().config;
        assert!(!live.widgets.iter().any(|w| w.type_id == "clock"));
        assert_eq!(live.widgets, reloaded.widgets);
        assert_eq!(reloaded.disabled.get("clock#0").map(String::as_str), Some("clock"));
        // La rallumer réactive celle de l'utilisateur, sans en créer une seconde.
        let mut s2 = Settings::from_config(&load_chunks(&chunks(s.to_lua()), Path::new("/cfg")).unwrap().config);
        assert_eq!(s2.set_widget_type("clock", true, Anchor::TopRight, &[], &[]), None);
        let back = load_chunks(&chunks(s2.to_lua()), Path::new("/cfg")).unwrap().config;
        assert_eq!(back.widgets.iter().filter(|w| w.type_id == "clock").count(), 1);
        assert!(back.disabled.is_empty());
    }

    #[test]
    fn barre_aller_retour() {
        let mut s = Settings { bar: Some(BarSettings::default()), ..Default::default() };
        s.bar.as_mut().unwrap().position = BarPosition::Bottom;
        s.bar.as_mut().unwrap().zones[2].push(BarEntry { type_id: "system".into(), fields: vec![r#"show = { "cpu" }"#.into()] });
        let lua = s.to_lua();
        let cfg = load_chunks(&[(lua.clone(), "settings.lua".into(), Origin::Settings)], Path::new("/cfg")).unwrap_or_else(|e| panic!("{e}\n{lua}")).config;
        assert_eq!(Settings::from_config(&cfg).bar, s.bar, "\n{lua}");
        let off = Settings { bar: Some(BarSettings { enabled: false, ..Default::default() }), ..Default::default() };
        let cfg = load_chunks(&[(off.to_lua(), "s.lua".into(), Origin::Settings)], Path::new("/cfg")).unwrap().config;
        assert!(cfg.bar.is_none());
        assert_eq!(Settings::from_config(&cfg).bar.map(|b| b.enabled), Some(false));
    }

    #[test]
    fn remplacer_et_retirer() {
        let mut s = Settings::default();
        let all = Selector::default();
        s.set_wallpaper(all.clone(), WallSource::Color(Rgba(0, 0, 0, 255)), Fit::Cover);
        s.set_wallpaper(all.clone(), WallSource::Color(Rgba(9, 9, 9, 255)), Fit::Cover);
        assert_eq!(s.wallpapers.len(), 1);
        assert_eq!(s.wallpaper(&all).unwrap().source, WallSource::Color(Rgba(9, 9, 9, 255)));
        s.remove_wallpaper(&all);
        assert!(s.wallpapers.is_empty());
        s.set_workspace_mode("4", Some(Mode::Glass));
        assert_eq!(s.workspace_mode("4"), Some(Mode::Glass));
        s.set_workspace_mode("4", None);
        assert_eq!(s.workspace_mode("4"), None);
    }

    #[test]
    fn enregistrer_puis_charger_avec_la_config_utilisateur() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = dir.join("hyprdesk.lua");
        std::fs::write(&cfg_path, r#"hd.config({ fade_ms = 50 }) hd.icon({ app = "kitty" })"#).unwrap();
        let s = Settings { fade_ms: Some(400), running_apps: Some(RunningApps::All), ..Default::default() };
        let written = s.save(&cfg_path).unwrap();
        assert_eq!(written, dir.join("hyprdesk/settings.lua"));
        assert!(!dir.join("hyprdesk/settings.lua.tmp").exists());
        let c = crate::config::lua::load_file(&cfg_path).unwrap().config;
        // Le Lua de l'utilisateur garde le dernier mot ; le reste vient des réglages.
        assert_eq!(c.general.fade_ms, 50);
        assert_eq!(c.general.running_apps, RunningApps::All);
        assert_eq!(c.icons.len(), 1);
        assert_eq!(Settings::from_config(&crate::config::lua::load_settings_only(&cfg_path).unwrap()), s);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn application_en_direct_identique_au_rechargement() {
        let user_src = r##"hd.config({ fade_ms = 50 }) hd.wallpaper({ color = "#222222" }) hd.icon({ app = "kitty" })"##;
        let old = Settings { show_on: Some(ShowOn::Focused), ..Default::default() };
        let base = load_chunks(
            &[(old.to_lua(), "settings.lua".into(), Origin::Settings), (user_src.into(), "user.lua".into(), Origin::User)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        let mut new = old.clone();
        new.show_on = Some(ShowOn::All);
        new.fade_ms = Some(900); // surchargé par l'utilisateur : sans effet
        new.running_apps = Some(RunningApps::Workspace);
        new.set_wallpaper(Selector::default(), WallSource::Color(Rgba(1, 1, 1, 255)), Fit::Cover);
        new.add_app("firefox", None);
        let live = new.apply_to(&base);
        let reloaded = load_chunks(
            &[(new.to_lua(), "settings.lua".into(), Origin::Settings), (user_src.into(), "user.lua".into(), Origin::User)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        assert_eq!(live.general, reloaded.general);
        assert_eq!(live.wallpapers, reloaded.wallpapers);
        assert_eq!(live.icons, reloaded.icons);
        assert_eq!(live.general.fade_ms, 50);
        assert_eq!(live.general.show_on, ShowOn::All);
    }

    #[test]
    fn vide_ne_produit_que_l_entete() {
        let lua = Settings::default().to_lua();
        assert!(lua.lines().all(|l| l.starts_with("--") || l.is_empty()), "{lua}");
    }
}
