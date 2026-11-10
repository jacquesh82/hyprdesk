//! Réglages faits à la souris (écran ⚙), enregistrés en Lua GÉNÉRÉ dans
//! `~/.config/hypr/hyprdesk/settings.lua` — comme hyprdmc avec monitors.lua.
//!
//! Ce module est chargé AVANT `hyprdesk.lua` : ce que l'utilisateur écrit à la
//! main garde le dernier mot, et son fichier n'est jamais réécrit. Le Lua
//! généré est la seule source de vérité : l'écran le relit en l'exécutant.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use hyprpanel_core::ipc::lua_escape;

use crate::config::{
    Config, DesktopRule, Fit, IconDef, IconKind, Mode, Origin, Rgba, RunningApps, Selector, ShowOn, Transition, WallSource,
    WallpaperRule,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
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
}

impl Settings {
    /// Reconstruit les réglages depuis la config chargée à partir du SEUL settings.lua.
    pub fn from_config(c: &Config) -> Self {
        let g = &c.general;
        let has = |k: &str| c.keys_settings.contains(k);
        Settings {
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
        }
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
        let p = crate::config::lua::settings_path(config_path);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = p.with_extension("lua.tmp");
        std::fs::write(&tmp, self.to_lua())?;
        std::fs::rename(&tmp, &p)?;
        Ok(p)
    }
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
    fn vide_ne_produit_que_l_entete() {
        let lua = Settings::default().to_lua();
        assert!(lua.lines().all(|l| l.starts_with("--") || l.is_empty()), "{lua}");
    }
}
