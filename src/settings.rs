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
    Anchor, AppMatch, AssignRule, BarItem, BarPosition, Config, DesktopRule, Effect, Fit, IconDef,
    IconKind, Mode, Origin, Place, Placement, Rgba, RunningApps, Selector, ShowOn, SysItem,
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
        BarEntry {
            type_id: type_id.to_string(),
            fields: Vec::new(),
        }
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
    /// Zones des écrans verticaux, si elles diffèrent.
    pub portrait: Option<[Vec<BarEntry>; 3]>,
    pub compact: crate::config::BarCompact,
    pub colors: crate::config::BarColors,
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
                vec![BarEntry {
                    type_id: "clock".into(),
                    fields: vec![format!("format = {}", q("%a %d %b  %H:%M"))],
                }],
                vec![
                    BarEntry::new("tray"),
                    BarEntry::new("media"),
                    BarEntry::new("reseau"),
                    BarEntry::new("bluetooth"),
                    BarEntry::new("batterie"),
                    BarEntry::new("volume"),
                    BarEntry::new("brightness"),
                    // Cachés d'eux-mêmes quand ils n'ont rien à montrer.
                    BarEntry::new("clavier"),
                    BarEntry::new("nuit"),
                    BarEntry::new("supports"),
                    BarEntry::new("capture"),
                    BarEntry::new("majs"),
                    BarEntry::new("notifications"),
                    // Accès au bureau vide, toujours en bout de barre.
                    BarEntry::new("desktop"),
                    BarEntry::new("session"),
                ],
            ],
            portrait: None,
            compact: Default::default(),
            colors: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    /// Barre native, si réglée depuis l'écran ⚙.
    pub bar: Option<BarSettings>,
    pub key_toggle: Option<String>,
    pub key_launcher: Option<String>,
    pub key_hide: Option<String>,
    pub key_settings: Option<String>,
    pub key_lock: Option<String>,
    pub key_switch: Option<String>,
    pub key_search: Option<String>,
    pub key_overview: Option<String>,
    pub overview_command: Option<String>,
    pub key_leave: Option<String>,
    pub key_media: Option<bool>,
    pub suspend_binds: Option<bool>,
    pub clipboard_history: Option<bool>,
    pub autostart: Option<bool>,
    pub desktop_files: Option<bool>,
    pub float_launched: Option<bool>,
    pub show_on: Option<ShowOn>,
    pub mode: Option<Mode>,
    pub glass_opacity: Option<f32>,
    pub close_on_launch: Option<bool>,
    pub advanced_settings: Option<bool>,
    pub tour_seen: Option<bool>,
    pub app_themes: Option<std::collections::BTreeSet<String>>,
    pub fade_ms: Option<u64>,
    pub transition: Option<Transition>,
    pub crossfade_ms: Option<u64>,
    pub running_apps: Option<RunningApps>,
    pub launcher_group: Option<crate::config::LauncherGroup>,
    pub launcher_position: Option<crate::config::LauncherPosition>,
    pub launcher_size: Option<u16>,
    pub launcher_pinned: Option<Vec<String>>,
    pub launcher_show_settings: Option<bool>,
    pub launcher_show_assistant: Option<bool>,
    pub apps_style: Option<crate::config::AppsStyle>,
    pub apps_suggestions: Option<u8>,
    pub apps_categories: Option<bool>,
    pub lock_blur: Option<bool>,
    pub lock_dim: Option<f32>,
    pub lock_image: Option<PathBuf>,
    pub lock_show_user: Option<bool>,
    pub lock_show_battery: Option<bool>,
    pub lock_show_media: Option<bool>,
    pub lock_fingerprint: Option<bool>,
    pub idle_manage: Option<bool>,
    pub idle_dim: Option<u32>,
    pub idle_lock: Option<u32>,
    pub idle_dpms: Option<u32>,
    pub idle_suspend: Option<u32>,
    pub icon_theme: Option<String>,
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub icon_size: Option<u16>,
    pub cell: Option<u16>,
    pub accent: Option<Rgba>,
    /// Thème : nom, couleurs du texte et des surfaces, fond sans image.
    pub theme_name: Option<String>,
    pub fg: Option<Rgba>,
    pub bg: Option<Rgba>,
    pub background: Option<Rgba>,
    /// Effets de surface réglés ici, par endroit.
    pub effects: BTreeMap<Place, Effect>,
    /// Workspaces permanents (`hd.config({ workspaces = N })`).
    pub workspaces: Option<u8>,
    /// Workspaces de chaque écran.
    pub workspace_monitors: Option<BTreeMap<String, Vec<String>>>,
    pub forced_modes: Option<BTreeMap<String, String>>,
    pub plugin_options: Option<BTreeMap<String, BTreeMap<String, crate::tiling::OptValue>>>,
    pub update_commands: Option<BTreeMap<String, (String, String)>>,
    /// Écrans dédiés.
    pub workspace_dedicated: Option<std::collections::BTreeSet<String>>,
    /// Écran principal (`hd.config({ primary = "DP-1" })`).
    pub primary: Option<String>,
    pub assistant_mode: Option<crate::assistant::AssistantMode>,
    pub language: Option<String>,
    /// Layouts (`layout`, `layout_workspaces`, `dwindle`, `master`, `scrolling`).
    pub tiling: Option<crate::tiling::Tiling>,
    /// Animations et effets de Hyprland (paquet « Animations »).
    pub fx: Option<crate::fx::Fx>,
    /// Paquets intégrés désactivés.
    pub disabled_plugins: Option<std::collections::BTreeSet<String>>,
    /// Applications par défaut réglées ici, par rôle.
    pub apps: BTreeMap<String, String>,
    /// Affectations d'applications réglées ici (`hd.assign`).
    pub assigns: Vec<AssignRule>,
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

impl BarSettings {
    pub fn from_config(c: &Config) -> Self {
        // Modules d'une config chargée → entrées des réglages (type et options).
        let entries = |zones: &[Vec<BarItem>; 3]| -> [Vec<BarEntry>; 3] {
            std::array::from_fn(|z| {
                zones[z]
                    .iter()
                    .map(|it| match it {
                        BarItem::Workspaces => BarEntry::new("workspaces"),
                        BarItem::Window => BarEntry::new("window"),
                        BarItem::Desktop => BarEntry::new("desktop"),
                        BarItem::Volume => BarEntry::new("volume"),
                        BarItem::Tray => BarEntry::new("tray"),
                        BarItem::Brightness => BarEntry::new("brightness"),
                        BarItem::Widget(id) => match c.widgets.iter().find(|w| &w.id == id) {
                            Some(w) => BarEntry {
                                type_id: w.type_id.clone(),
                                fields: widget_fields(&w.kind),
                            },
                            None => BarEntry::new("clock"),
                        },
                    })
                    .collect()
            })
        };
        match &c.bar {
            None => BarSettings {
                enabled: false,
                ..BarSettings::default()
            },
            Some(b) => BarSettings {
                enabled: true,
                monitors: b.monitors.clone(),
                position: b.position,
                height: b.height,
                zones: entries(&b.zones),
                portrait: b.portrait.as_ref().map(entries),
                compact: b.compact,
                colors: b.colors.clone(),
            },
        }
    }

    /// Zone éditée : 0..=2 la barre ordinaire, 3..=5 celle des écrans
    /// verticaux (créée à partir de l'ordinaire au premier changement).
    pub fn zone_mut(&mut self, z: usize) -> &mut Vec<BarEntry> {
        if z < 3 {
            return &mut self.zones[z];
        }
        let base = self.zones.clone();
        &mut self.portrait.get_or_insert(base)[(z - 3).min(2)]
    }

    /// Déplace le module `from` (zone, rang) pour qu'il occupe la place
    /// `to` (zone, rang d'insertion, compté avant le retrait). Faux si rien
    /// ne bouge.
    pub fn move_entry(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        let (fz, fi) = from;
        let (tz, mut ti) = to;
        if fz > 2 || tz > 2 || fi >= self.zones[fz].len() {
            return false;
        }
        if fz == tz && (ti == fi || ti == fi + 1) {
            return false;
        }
        let e = self.zones[fz].remove(fi);
        if fz == tz && ti > fi {
            ti -= 1;
        }
        let ti = ti.min(self.zones[tz].len());
        self.zones[tz].insert(ti, e);
        true
    }
}

impl Settings {
    /// Reconstruit les réglages depuis la config chargée à partir du SEUL settings.lua.
    pub fn from_config(c: &Config) -> Self {
        let g = &c.general;
        let has = |k: &str| c.keys_settings.contains(k);
        let bar = c
            .keys_settings
            .contains("bar")
            .then(|| BarSettings::from_config(c));
        Settings {
            bar,
            key_toggle: has("keys.toggle").then(|| g.keys.toggle.clone()),
            key_launcher: has("keys.launcher").then(|| g.keys.launcher.clone()),
            key_hide: has("keys.hide").then(|| g.keys.hide.clone()),
            key_settings: has("keys.settings").then(|| g.keys.settings.clone()),
            key_lock: has("keys.lock").then(|| g.keys.lock.clone()),
            key_switch: has("keys.switch").then(|| g.keys.switch.clone()),
            key_search: has("keys.search").then(|| g.keys.search.clone()),
            key_overview: has("keys.overview").then(|| g.keys.overview.clone()),
            overview_command: has("overview_command").then(|| g.overview_command.clone()),
            key_leave: has("keys.leave").then(|| g.keys.leave.clone()),
            key_media: has("keys.media").then_some(g.keys.media),
            suspend_binds: has("suspend_binds").then_some(g.suspend_binds),
            clipboard_history: has("clipboard_history").then_some(g.clipboard_history),
            autostart: has("autostart").then_some(g.autostart),
            desktop_files: has("desktop_files").then_some(g.desktop_files),
            float_launched: has("float_launched").then_some(g.float_launched),
            show_on: has("show_on").then_some(g.show_on),
            mode: has("mode").then_some(g.mode),
            glass_opacity: has("glass_opacity").then_some(g.glass_opacity),
            close_on_launch: has("close_on_launch").then_some(g.close_on_launch),
            advanced_settings: has("advanced_settings").then_some(g.advanced_settings),
            tour_seen: has("tour_seen").then_some(g.tour_seen),
            app_themes: has("app_themes").then(|| g.app_themes.clone()),
            fade_ms: has("fade_ms").then_some(g.fade_ms),
            transition: has("transition").then_some(g.transition),
            crossfade_ms: has("crossfade_ms").then_some(g.crossfade_ms),
            running_apps: has("running_apps").then_some(g.running_apps),
            launcher_group: has("launcher_group").then_some(g.launcher_group),
            launcher_position: has("launcher_position").then_some(g.launcher.position),
            launcher_size: has("launcher_size").then_some(g.launcher.size),
            launcher_pinned: has("launcher_pinned").then(|| g.launcher.pinned.clone()),
            launcher_show_settings: has("launcher_show_settings")
                .then_some(g.launcher.show_settings),
            lock_blur: has("lock_blur").then_some(g.lock.blur),
            lock_dim: has("lock_dim").then_some(g.lock.dim),
            lock_image: if has("lock_image") {
                g.lock.image.clone()
            } else {
                None
            },
            lock_show_user: has("lock_show_user").then_some(g.lock.show_user),
            lock_show_battery: has("lock_show_battery").then_some(g.lock.show_battery),
            lock_show_media: has("lock_show_media").then_some(g.lock.show_media),
            lock_fingerprint: has("lock_fingerprint").then_some(g.lock.fingerprint),
            idle_manage: has("idle_manage").then_some(g.idle.manage),
            idle_dim: has("idle_dim").then_some(g.idle.dim),
            idle_lock: has("idle_lock").then_some(g.idle.lock),
            idle_dpms: has("idle_dpms").then_some(g.idle.dpms),
            idle_suspend: has("idle_suspend").then_some(g.idle.suspend),
            launcher_show_assistant: has("launcher_show_assistant")
                .then_some(g.launcher.show_assistant),
            apps_style: has("apps_style").then_some(g.apps_launcher.style),
            apps_suggestions: has("apps_suggestions").then_some(g.apps_launcher.suggestions),
            apps_categories: has("apps_categories").then_some(g.apps_launcher.categories),
            icon_theme: if has("icon_theme") {
                g.icon_theme.clone()
            } else {
                None
            },
            font: if has("font") { g.font.clone() } else { None },
            font_size: has("font_size").then_some(g.font_size),
            icon_size: has("grid.icon_size").then_some(g.grid.icon_size),
            cell: has("grid.cell").then_some(g.grid.cell),
            accent: has("theme.accent").then_some(g.theme.accent),
            theme_name: has("theme.name").then(|| g.theme.name.clone()),
            fg: has("theme.fg").then_some(g.theme.fg),
            bg: has("theme.bg").then_some(g.theme.bg),
            background: has("background").then_some(g.background),
            effects: Place::ALL
                .iter()
                .filter(|p| has(&format!("theme.effects.{}", p.as_str())))
                .map(|p| (*p, g.theme.effect(*p)))
                .collect(),
            workspace_monitors: has("workspace_monitors")
                .then(|| g.workspace_monitors_declared.clone()),
            forced_modes: has("forced_modes").then(|| g.forced_modes.clone()),
            plugin_options: has("plugin_options").then(|| g.plugin_options.clone()),
            update_commands: has("update_commands").then(|| g.update_commands.clone()),
            workspace_dedicated: has("workspace_dedicated").then(|| g.workspace_dedicated.clone()),
            workspaces: if has("workspaces") {
                g.workspaces
            } else {
                None
            },
            assistant_mode: has("assistant_mode").then_some(g.assistant_mode),
            language: has("language").then(|| g.language.clone()),
            tiling: [
                "layout",
                "layout_workspaces",
                "dwindle",
                "master",
                "scrolling",
            ]
            .iter()
            .any(|k| has(k))
            .then(|| g.tiling.clone()),
            disabled_plugins: has("disabled_plugins").then(|| g.disabled_plugins.clone()),
            fx: has("fx").then(|| g.fx.clone()),
            apps: g
                .apps
                .iter()
                .filter(|(r, _)| has(&format!("apps.{r}")))
                .map(|(r, v)| (r.clone(), v.clone()))
                .collect(),
            primary: if has("primary") {
                g.primary.clone()
            } else {
                None
            },
            assigns: c
                .assigns
                .iter()
                .filter(|a| a.origin == Origin::Settings)
                .cloned()
                .collect(),
            wallpapers: c
                .wallpapers
                .iter()
                .filter(|w| w.origin == Origin::Settings)
                .cloned()
                .collect(),
            desktops: c
                .desktops
                .iter()
                .filter(|d| d.origin == Origin::Settings)
                .cloned()
                .collect(),
            icons: c
                .icons
                .iter()
                .filter(|i| i.origin == Origin::Settings)
                .cloned()
                .collect(),
            widgets: c
                .widgets
                .iter()
                .filter(|w| w.origin == Origin::Settings && !w.in_bar && !w.id.starts_with("bar:"))
                .cloned()
                .collect(),
            placements: c
                .placements
                .iter()
                .filter(|(id, _)| {
                    !c.widgets
                        .iter()
                        .any(|w| &w.id == *id && w.origin == Origin::Settings)
                })
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            disabled: c.disabled.clone(),
        }
    }

    /// Active ou désactive un type de widget, quelle que soit l'origine de ses
    /// instances. `user_ids` : les widgets de ce type déclarés dans hyprdesk.lua.
    pub fn set_widget_type(
        &mut self,
        type_id: &str,
        on: bool,
        anchor: Anchor,
        user_ids: &[String],
        taken: &[String],
    ) -> Option<String> {
        if on {
            let was_disabled: Vec<String> = self
                .disabled
                .iter()
                .filter(|(_, t)| *t == type_id)
                .map(|(id, _)| id.clone())
                .collect();
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

    /// Retire un widget déclaré dans les réglages.
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
        set!(key_launcher, "keys.launcher", g.keys.launcher);
        set!(key_hide, "keys.hide", g.keys.hide);
        set!(key_settings, "keys.settings", g.keys.settings);
        set!(key_lock, "keys.lock", g.keys.lock);
        set!(key_switch, "keys.switch", g.keys.switch);
        set!(key_search, "keys.search", g.keys.search);
        set!(key_overview, "keys.overview", g.keys.overview);
        set!(overview_command, "overview_command", g.overview_command);
        set!(key_leave, "keys.leave", g.keys.leave);
        set!(key_media, "keys.media", g.keys.media);
        set!(suspend_binds, "suspend_binds", g.suspend_binds);
        set!(clipboard_history, "clipboard_history", g.clipboard_history);
        set!(autostart, "autostart", g.autostart);
        set!(desktop_files, "desktop_files", g.desktop_files);
        set!(float_launched, "float_launched", g.float_launched);
        set!(show_on, "show_on", g.show_on);
        set!(mode, "mode", g.mode);
        set!(glass_opacity, "glass_opacity", g.glass_opacity);
        set!(close_on_launch, "close_on_launch", g.close_on_launch);
        set!(advanced_settings, "advanced_settings", g.advanced_settings);
        set!(tour_seen, "tour_seen", g.tour_seen);
        set!(app_themes, "app_themes", g.app_themes);
        set!(fade_ms, "fade_ms", g.fade_ms);
        set!(transition, "transition", g.transition);
        set!(crossfade_ms, "crossfade_ms", g.crossfade_ms);
        set!(running_apps, "running_apps", g.running_apps);
        set!(launcher_group, "launcher_group", g.launcher_group);
        set!(launcher_position, "launcher_position", g.launcher.position);
        set!(launcher_size, "launcher_size", g.launcher.size);
        set!(apps_style, "apps_style", g.apps_launcher.style);
        set!(
            apps_suggestions,
            "apps_suggestions",
            g.apps_launcher.suggestions
        );
        set!(
            apps_categories,
            "apps_categories",
            g.apps_launcher.categories
        );
        set!(launcher_pinned, "launcher_pinned", g.launcher.pinned);
        set!(
            launcher_show_settings,
            "launcher_show_settings",
            g.launcher.show_settings
        );
        set!(
            launcher_show_assistant,
            "launcher_show_assistant",
            g.launcher.show_assistant
        );
        set!(assistant_mode, "assistant_mode", g.assistant_mode);
        set!(language, "language", g.language);
        set!(fx, "fx", g.fx);
        if let Some(t) = &self.tiling {
            if free("layout") {
                g.tiling.layout = t.layout.clone();
            }
            if free("layout_workspaces") {
                g.tiling.workspaces = t.workspaces.clone();
            }
            for section in crate::tiling::SECTIONS {
                if free(section) {
                    let prefix = format!("{section}.");
                    g.tiling.options.retain(|k, _| !k.starts_with(&prefix));
                    g.tiling.options.extend(
                        t.options
                            .iter()
                            .filter(|(k, _)| k.starts_with(&prefix))
                            .map(|(k, v)| (k.clone(), v.clone())),
                    );
                }
            }
        }
        set!(lock_blur, "lock_blur", g.lock.blur);
        set!(lock_dim, "lock_dim", g.lock.dim);
        set!(lock_show_user, "lock_show_user", g.lock.show_user);
        set!(lock_show_battery, "lock_show_battery", g.lock.show_battery);
        set!(lock_show_media, "lock_show_media", g.lock.show_media);
        set!(lock_fingerprint, "lock_fingerprint", g.lock.fingerprint);
        set!(idle_manage, "idle_manage", g.idle.manage);
        set!(idle_dim, "idle_dim", g.idle.dim);
        set!(idle_lock, "idle_lock", g.idle.lock);
        set!(idle_dpms, "idle_dpms", g.idle.dpms);
        set!(idle_suspend, "idle_suspend", g.idle.suspend);
        if free("lock_image") {
            g.lock.image = self.lock_image.clone();
        }
        set!(icon_size, "grid.icon_size", g.grid.icon_size);
        set!(cell, "grid.cell", g.grid.cell);
        set!(accent, "theme.accent", g.theme.accent);
        set!(theme_name, "theme.name", g.theme.name);
        set!(fg, "theme.fg", g.theme.fg);
        set!(bg, "theme.bg", g.theme.bg);
        set!(background, "background", g.background);
        for (p, e) in &self.effects {
            if free(&format!("theme.effects.{}", p.as_str())) {
                g.theme.effects.insert(*p, *e);
            }
        }
        set!(font_size, "font_size", g.font_size);
        if let Some(font) = &self.font
            && free("font")
        {
            g.font = Some(font.clone());
        }
        if let Some(t) = &self.icon_theme
            && free("icon_theme")
        {
            g.icon_theme = Some(t.clone());
        }
        // Retirer le réglage rend la main à Hyprland : pas de valeur par défaut à garder.
        if free("workspaces") {
            g.workspaces = self.workspaces;
        }
        if free("workspace_monitors") {
            g.workspace_monitors_declared = self.workspace_monitors.clone().unwrap_or_default();
        }
        if free("forced_modes") {
            g.forced_modes = self.forced_modes.clone().unwrap_or_default();
        }
        if free("plugin_options") {
            g.plugin_options = self.plugin_options.clone().unwrap_or_default();
        }
        if free("update_commands") {
            g.update_commands = self.update_commands.clone().unwrap_or_default();
        }
        if free("workspace_dedicated") {
            g.workspace_dedicated = self.workspace_dedicated.clone().unwrap_or_default();
        }
        g.expand_workspace_monitors();
        if free("primary") {
            g.primary = self.primary.clone();
        }
        if free("disabled_plugins") {
            g.disabled_plugins = self.disabled_plugins.clone().unwrap_or_default();
        }
        // Rôles : ceux de hyprdesk.lua, puis ceux des réglages (un rôle retiré
        // des réglages disparaît de la config effective).
        g.apps
            .retain(|r, _| cfg.keys_user.contains(&format!("apps.{r}")));
        for (r, v) in &self.apps {
            if free(&format!("apps.{r}")) {
                if r == "terminal" {
                    g.terminal = crate::apps::terminal_command(v);
                }
                g.apps.insert(r.clone(), v.clone());
            }
        }
        let user = |o: Origin| o == Origin::User;
        c.wallpapers = self
            .wallpapers
            .iter()
            .cloned()
            .chain(cfg.wallpapers.iter().filter(|w| user(w.origin)).cloned())
            .collect();
        c.desktops = self
            .desktops
            .iter()
            .cloned()
            .chain(cfg.desktops.iter().filter(|d| user(d.origin)).cloned())
            .collect();
        c.icons = self
            .icons
            .iter()
            .cloned()
            .chain(cfg.icons.iter().filter(|i| user(i.origin)).cloned())
            .collect();
        // Comme au chargement : une affectation de hyprdesk.lua remplace celle
        // des réglages pour la même application.
        c.assigns = self
            .assigns
            .iter()
            .filter(|a| !cfg.assigns.iter().any(|u| user(u.origin) && u.app == a.app))
            .cloned()
            .chain(cfg.assigns.iter().filter(|a| user(a.origin)).cloned())
            .collect();
        // Les widgets des réglages gardent leur contenu chargé (fonctions Lua des
        // plugins) : seul le placement vient des réglages. Les activations et
        // retraits passent par un rechargement.
        for w in c.widgets.iter_mut() {
            let p = match w.origin {
                Origin::Settings => self
                    .widgets
                    .iter()
                    .find(|s| s.id == w.id)
                    .map(|s| s.placement()),
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

    /// Fond d'une portée. Une scène étalée le reste si la source est une image
    /// ou un diaporama.
    pub fn set_wallpaper(&mut self, sel: Selector, source: WallSource, fit: Fit) {
        let color = matches!(source, WallSource::Color(_));
        let span = self.wallpaper(&sel).is_some_and(|w| w.span) && !color;
        let span_lock = self.wallpaper(&sel).is_some_and(|w| w.span_lock) && !color;
        self.wallpapers.retain(|w| w.sel != sel);
        self.wallpapers.push(WallpaperRule {
            origin: Origin::Settings,
            sel,
            source,
            fit,
            span,
            span_lock,
        });
    }

    /// Étale (ou non) le fond d'une portée sur tous les écrans : au bureau
    /// (`span`) et au verrouillage (`span_lock`).
    pub fn set_wallpaper_span(&mut self, sel: &Selector, span: bool, span_lock: bool) {
        if let Some(w) = self.wallpapers.iter_mut().find(|w| &w.sel == sel)
            && !matches!(w.source, WallSource::Color(_))
        {
            w.span = span;
            w.span_lock = span_lock;
        }
    }

    pub fn remove_wallpaper(&mut self, sel: &Selector) {
        self.wallpapers.retain(|w| &w.sel != sel);
    }

    pub fn wallpaper(&self, sel: &Selector) -> Option<&WallpaperRule> {
        self.wallpapers.iter().find(|w| &w.sel == sel)
    }

    /// Mode d'un workspace ; `None` le rend au mode par défaut.
    pub fn set_workspace_mode(&mut self, workspace: &str, mode: Option<Mode>) {
        let sel = Selector {
            workspaces: Some(vec![workspace.to_string()]),
            monitors: None,
        };
        self.desktops.retain(|d| d.sel != sel);
        if let Some(m) = mode {
            self.desktops.push(DesktopRule {
                origin: Origin::Settings,
                sel,
                mode: Some(m),
                inherit: true,
            });
        }
    }

    pub fn workspace_mode(&self, workspace: &str) -> Option<Mode> {
        let sel = Selector {
            workspaces: Some(vec![workspace.to_string()]),
            monitors: None,
        };
        self.desktops
            .iter()
            .find(|d| d.sel == sel)
            .and_then(|d| d.mode)
    }

    /// Affecte une application à un workspace (remplace son affectation précédente).
    pub fn set_assign(&mut self, app: AppMatch, workspace: &str, silent: bool) {
        if let Some(a) = self.assigns.iter_mut().find(|a| a.app == app) {
            a.workspace = workspace.to_string();
            a.silent = silent;
            return;
        }
        self.assigns.push(AssignRule {
            origin: Origin::Settings,
            app,
            workspace: workspace.to_string(),
            silent,
        });
    }

    pub fn remove_assign(&mut self, idx: usize) {
        if idx < self.assigns.len() {
            self.assigns.remove(idx);
        }
    }

    pub fn add_app(&mut self, app: &str, workspace: Option<&str>) {
        let sel = Selector {
            workspaces: workspace.map(|w| vec![w.to_string()]),
            monitors: None,
        };
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

    /// Icône d'un fichier ou d'un dossier déposé sur le bureau ; une
    /// application (`.desktop` des dossiers d'applications) devient une icône
    /// d'application. Rend son identifiant (pour la placer), ou `None` si elle
    /// existe déjà.
    pub fn add_dropped(&mut self, path: &std::path::Path) -> Option<String> {
        let app = path
            .extension()
            .is_some_and(|e| e == "desktop")
            .then(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
            .flatten()
            .filter(|_| {
                path.parent().is_some_and(|d| {
                    d.ends_with("applications") || d.to_string_lossy().contains("/applications")
                })
            });
        let (id, kind, icon, label) = match app {
            Some(a) => (format!("app:{a}"), IconKind::App(a), None, None),
            None => {
                let label = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let icon = if path.is_dir() {
                    "folder"
                } else {
                    match path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .as_deref()
                    {
                        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "svg") => "image-x-generic",
                        Some("pdf") => "application-pdf",
                        Some("mp4" | "mkv" | "webm" | "avi" | "mov") => "video-x-generic",
                        Some("mp3" | "flac" | "ogg" | "wav" | "opus") => "audio-x-generic",
                        Some("zip" | "tar" | "gz" | "xz" | "7z" | "rar") => "package-x-generic",
                        Some("sh" | "py" | "rs" | "lua" | "js" | "ts") => "text-x-script",
                        _ => "text-x-generic",
                    }
                };
                (
                    // Même règle que hd.icon : `path:` suivi du chemin tel qu'écrit.
                    format!("path:{}", tilde(path)),
                    IconKind::Path(path.to_path_buf()),
                    Some(icon.to_string()),
                    Some(label),
                )
            }
        };
        if self.icons.iter().any(|i| i.id == id) {
            return None;
        }
        self.icons.push(IconDef {
            origin: Origin::Settings,
            sel: Selector::default(),
            id: id.clone(),
            label,
            kind,
            icon,
            pos: None,
        });
        Some(id)
    }

    pub fn remove_icon(&mut self, idx: usize) {
        if idx < self.icons.len() {
            self.icons.remove(idx);
        }
    }

    pub fn to_lua(&self) -> String {
        let mut o = String::new();
        let _ = writeln!(
            o,
            "-- Généré par hyprdesk (écran Réglages ⚙) — NE PAS ÉDITER : réécrit à chaque enregistrement."
        );
        let _ = writeln!(
            o,
            "-- Chargé AVANT ~/.config/hypr/hyprdesk.lua, qui garde le dernier mot."
        );
        let _ = writeln!(o);

        let mut cfg: Vec<String> = Vec::new();
        let mut grid: Vec<String> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        for (name, v) in [
            ("toggle", &self.key_toggle),
            ("launcher", &self.key_launcher),
            ("hide", &self.key_hide),
            ("settings", &self.key_settings),
            ("lock", &self.key_lock),
            ("switch", &self.key_switch),
            ("search", &self.key_search),
            ("overview", &self.key_overview),
            ("leave", &self.key_leave),
        ] {
            if let Some(k) = v {
                keys.push(if k.is_empty() {
                    format!("{name} = false")
                } else {
                    format!("{name} = {}", q(k))
                });
            }
        }
        if let Some(v) = self.key_media {
            keys.push(format!("media = {v}"));
        }
        if !keys.is_empty() {
            cfg.push(format!("keys = {{ {} }}", keys.join(", ")));
        }
        if let Some(v) = self.suspend_binds {
            cfg.push(format!("suspend_binds = {v}"));
        }
        if let Some(v) = self.clipboard_history {
            cfg.push(format!("clipboard_history = {v}"));
        }
        if let Some(v) = self.autostart {
            cfg.push(format!("autostart = {v}"));
        }
        if let Some(v) = self.desktop_files {
            cfg.push(format!("desktop_files = {v}"));
        }
        if let Some(v) = self.float_launched {
            cfg.push(format!("float_launched = {v}"));
        }
        if let Some(v) = self.show_on {
            cfg.push(format!("show_on = {}", q(v.as_str())));
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
        if let Some(v) = self.advanced_settings {
            cfg.push(format!("advanced_settings = {v}"));
        }
        if let Some(v) = self.tour_seen {
            cfg.push(format!("tour_seen = {v}"));
        }
        if let Some(v) = &self.app_themes {
            let l: Vec<String> = v.iter().map(|x| q(x)).collect();
            cfg.push(format!("app_themes = {{ {} }}", l.join(", ")));
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
        if let Some(v) = self.assistant_mode {
            cfg.push(format!("assistant_mode = {}", q(v.as_str())));
        }
        if let Some(v) = &self.language {
            cfg.push(format!("language = {}", q(v)));
        }
        if let Some(fx) = self.fx.as_ref().filter(|f| !f.is_empty()) {
            cfg.push(format!("fx = {}", fx.to_lua()));
        }
        if let Some(t) = &self.tiling {
            if let Some(l) = &t.layout {
                cfg.push(format!("layout = {}", q(l)));
            }
            if !t.workspaces.is_empty() {
                let f: Vec<String> = t
                    .workspaces
                    .iter()
                    .map(|(w, l)| format!("[{}] = {}", q(w), q(l)))
                    .collect();
                cfg.push(format!("layout_workspaces = {{ {} }}", f.join(", ")));
            }
            for section in crate::tiling::SECTIONS {
                let prefix = format!("{section}.");
                let f: Vec<String> = t
                    .options
                    .iter()
                    .filter_map(|(k, v)| {
                        Some(format!("{} = {}", k.strip_prefix(&prefix)?, v.lua()))
                    })
                    .collect();
                if !f.is_empty() {
                    cfg.push(format!("{section} = {{ {} }}", f.join(", ")));
                }
            }
        }
        if let Some(v) = self.launcher_position {
            cfg.push(format!("launcher_position = {}", q(v.as_str())));
        }
        if let Some(v) = self.launcher_size {
            cfg.push(format!("launcher_size = {v}"));
        }
        if let Some(l) = &self.launcher_pinned {
            let l: Vec<String> = l.iter().map(|a| q(a)).collect();
            cfg.push(format!("launcher_pinned = {{ {} }}", l.join(", ")));
        }
        if let Some(v) = self.launcher_show_settings {
            cfg.push(format!("launcher_show_settings = {v}"));
        }
        if let Some(v) = self.launcher_show_assistant {
            cfg.push(format!("launcher_show_assistant = {v}"));
        }
        if let Some(v) = self.apps_style {
            cfg.push(format!("apps_style = {}", q(v.as_str())));
        }
        if let Some(v) = self.apps_suggestions {
            cfg.push(format!("apps_suggestions = {v}"));
        }
        if let Some(v) = self.apps_categories {
            cfg.push(format!("apps_categories = {v}"));
        }
        if let Some(v) = self.lock_blur {
            cfg.push(format!("lock_blur = {v}"));
        }
        if let Some(v) = self.lock_dim {
            cfg.push(format!("lock_dim = {}", num(v)));
        }
        if let Some(p) = &self.lock_image {
            cfg.push(format!("lock_image = {}", q(&tilde(p))));
        }
        for (k, v) in [
            ("lock_show_user", self.lock_show_user),
            ("lock_show_battery", self.lock_show_battery),
            ("lock_show_media", self.lock_show_media),
            ("lock_fingerprint", self.lock_fingerprint),
            ("idle_manage", self.idle_manage),
        ] {
            if let Some(v) = v {
                cfg.push(format!("{k} = {v}"));
            }
        }
        for (k, v) in [
            ("idle_dim", self.idle_dim),
            ("idle_lock", self.idle_lock),
            ("idle_dpms", self.idle_dpms),
            ("idle_suspend", self.idle_suspend),
        ] {
            if let Some(v) = v {
                cfg.push(format!("{k} = {v}"));
            }
        }
        if let Some(v) = self.launcher_group {
            cfg.push(format!("launcher_group = {}", q(v.as_str())));
        }
        if let Some(v) = &self.icon_theme {
            cfg.push(format!("icon_theme = {}", q(v)));
        }
        if let Some(v) = &self.font {
            cfg.push(format!("font = {}", q(v)));
        }
        if let Some(v) = self.font_size {
            cfg.push(format!("font_size = {}", num(v)));
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
        let mut theme: Vec<String> = Vec::new();
        if let Some(n) = &self.theme_name {
            theme.push(format!("name = {}", q(n)));
        }
        for (k, c) in [("fg", self.fg), ("bg", self.bg), ("accent", self.accent)] {
            if let Some(c) = c {
                theme.push(format!("{k} = {}", q(&hex(c))));
            }
        }
        if !self.effects.is_empty() {
            let fx: Vec<String> = self
                .effects
                .iter()
                .map(|(p, e)| {
                    format!(
                        "{} = {{ opacity = {}, blur = {} }}",
                        p.as_str(),
                        num(e.opacity),
                        e.blur
                    )
                })
                .collect();
            theme.push(format!("effects = {{ {} }}", fx.join(", ")));
        }
        if !theme.is_empty() {
            cfg.push(format!("theme = {{ {} }}", theme.join(", ")));
        }
        if let Some(c) = self.background {
            cfg.push(format!("background = {}", q(&hex(c))));
        }
        if let Some(n) = self.workspaces {
            cfg.push(format!("workspaces = {n}"));
        }
        if let Some(m) = self.plugin_options.as_ref().filter(|m| !m.is_empty()) {
            let f: Vec<String> = m
                .iter()
                .filter(|(_, o)| !o.is_empty())
                .map(|(p, o)| {
                    let o: Vec<String> = o
                        .iter()
                        .map(|(k, v)| format!("[{}] = {}", q(k), v.lua()))
                        .collect();
                    format!("[{}] = {{ {} }}", q(p), o.join(", "))
                })
                .collect();
            if !f.is_empty() {
                cfg.push(format!("plugin_options = {{ {} }}", f.join(", ")));
            }
        }
        if let Some(m) = self.update_commands.as_ref().filter(|m| !m.is_empty()) {
            let f: Vec<String> = m
                .iter()
                .map(|(src, (c, u))| {
                    format!("[{}] = {{ check = {}, upgrade = {} }}", q(src), q(c), q(u))
                })
                .collect();
            cfg.push(format!("update_commands = {{ {} }}", f.join(", ")));
        }
        if let Some(m) = self.forced_modes.as_ref().filter(|m| !m.is_empty()) {
            let f: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("[{}] = {}", q(k), q(v)))
                .collect();
            cfg.push(format!("forced_modes = {{ {} }}", f.join(", ")));
        }
        if let Some(m) = self.workspace_monitors.as_ref().filter(|m| !m.is_empty()) {
            let f: Vec<String> = m
                .iter()
                .map(|(mon, l)| {
                    let l: Vec<String> = l.iter().map(|w| q(w)).collect();
                    format!("[{}] = {{ {} }}", q(mon), l.join(", "))
                })
                .collect();
            cfg.push(format!("workspace_monitors = {{ {} }}", f.join(", ")));
        }
        if let Some(d) = self.workspace_dedicated.as_ref().filter(|d| !d.is_empty()) {
            let l: Vec<String> = d.iter().map(|m| q(m)).collect();
            cfg.push(format!("workspace_dedicated = {{ {} }}", l.join(", ")));
        }
        if let Some(p) = &self.primary {
            cfg.push(format!("primary = {}", q(p)));
        }
        if let Some(d) = self.disabled_plugins.as_ref().filter(|d| !d.is_empty()) {
            let l: Vec<String> = d.iter().map(|p| q(p)).collect();
            cfg.push(format!("disabled_plugins = {{ {} }}", l.join(", ")));
        }
        if !self.apps.is_empty() {
            let f: Vec<String> = self
                .apps
                .iter()
                .map(|(r, v)| format!("{r} = {}", q(v)))
                .collect();
            cfg.push(format!("apps = {{ {} }}", f.join(", ")));
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
                WallSource::Rotate {
                    dir,
                    interval,
                    shuffle,
                } => f.push(format!(
                    "rotate = {{ dir = {}, interval = {}, shuffle = {shuffle} }}",
                    q(&tilde(dir)),
                    interval.as_secs()
                )),
            }
            match (w.span, w.span_lock) {
                (true, true) => f.push("span = true".into()),
                (false, true) => f.push("span = \"lock\"".into()),
                (true, false) => f.push("span = \"desktop\"".into()),
                (false, false) => {}
            }
            // Le bureau garde son ajustement s'il n'est pas étalé.
            if !w.span && w.fit != Fit::Cover && !matches!(w.source, WallSource::Color(_)) {
                f.push(format!("fit = {}", q(fit_str(w.fit))));
            }
            let _ = writeln!(o, "hd.wallpaper({{ {} }})", f.join(", "));
        }
        for a in &self.assigns {
            let mut f = vec![match &a.app {
                AppMatch::App(id) => format!("app = {}", q(id)),
                AppMatch::Class(re) => format!("class = {}", q(re)),
            }];
            f.push(format!("workspace = {}", q(&a.workspace)));
            if a.silent {
                f.push("silent = true".into());
            }
            let _ = writeln!(o, "hd.assign({{ {} }})", f.join(", "));
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
                f.push(format!(
                    "position = {}",
                    q(if b.position == BarPosition::Top {
                        "top"
                    } else {
                        "bottom"
                    })
                ));
                f.push(format!("height = {}", b.height));
                let zone = |l: &[BarEntry]| -> String {
                    l.iter()
                        .map(|e| {
                            if e.fields.is_empty() {
                                q(&e.type_id)
                            } else {
                                format!("{{ {}, {} }}", q(&e.type_id), e.fields.join(", "))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                for (zi, name) in ["left", "center", "right"].iter().enumerate() {
                    f.push(format!("{name} = {{ {} }}", zone(&b.zones[zi])));
                }
                if let Some(p) = &b.portrait {
                    f.push(format!(
                        "portrait = {{ left = {{ {} }}, center = {{ {} }}, right = {{ {} }} }}",
                        zone(&p[0]),
                        zone(&p[1]),
                        zone(&p[2])
                    ));
                }
                if b.compact != crate::config::BarCompact::Auto {
                    f.push(format!("compact = {}", q(b.compact.as_str())));
                }
                let slots: Vec<String> = crate::config::BAR_COLOR_SLOTS
                    .iter()
                    .filter_map(|(k, _)| Some(format!("{k} = {}", q(&hex(b.colors.slot(k)?)))))
                    .collect();
                if !slots.is_empty() {
                    f.push(format!("colors = {{ {} }}", slots.join(", ")));
                }
                if let Some(o) = b.colors.opacity {
                    f.push(format!("opacity = {:.2}", o.clamp(0.0, 1.0)));
                }
                if !b.colors.modules.is_empty() {
                    let m: Vec<String> = b
                        .colors
                        .modules
                        .iter()
                        .map(|(k, c)| format!("[{}] = {}", q(k), q(&hex(*c))))
                        .collect();
                    f.push(format!("module_colors = {{ {} }}", m.join(", ")));
                }
                let _ = writeln!(o, "hd.bar({{\n    {},\n}})\n", f.join(",\n    "));
            } else {
                let _ = writeln!(o, "hd.bar(false)\n");
            }
        }
        for w in self
            .widgets
            .iter()
            .filter(|w| !w.in_bar && !w.id.starts_with("bar:"))
        {
            let mut f = selector_fields(&w.sel);
            f.insert(0, format!("id = {}", q(&w.id)));
            f.extend(widget_fields(&w.kind));
            f.extend(placement_fields(&w.placement()));
            let _ = writeln!(o, "hd.widget({}, {{ {} }})", q(&w.type_id), f.join(", "));
        }
        for (id, p) in &self.placements {
            let _ = writeln!(
                o,
                "hd.place({}, {{ {} }})",
                q(id),
                placement_fields(p).join(", ")
            );
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
            if let Some(ic) = &i.icon {
                f.push(format!("icon = {}", q(ic)));
            }
            if let Some((c, r)) = i.pos {
                f.push(format!("pos = {{ {c}, {r} }}"));
            }
            let _ = writeln!(o, "hd.icon({{ {} }})", f.join(", "));
        }
        o
    }

    /// Écriture atomique (fichier temporaire + renommage) : la surveillance de
    /// la config ne voit jamais un fichier à moitié écrit.
    /// Enregistrement simple ; l'écran Réglages passe par `save_versioned`.
    #[cfg_attr(not(test), allow(dead_code))]
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
        WidgetKind::Clock { format } if format != "%H:%M" => {
            vec![format!("format = {}", q(format))]
        }
        WidgetKind::Media { player } if player != "*" => vec![format!("player = {}", q(player))],
        WidgetKind::System { show } if !is_default_show(show) => {
            vec![format!(
                "show = {{ {} }}",
                show.iter()
                    .map(|i| q(&sys_item_str(i)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )]
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
    if c.3 == 255 {
        format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c.0, c.1, c.2, c.3)
    }
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
        return if rest.as_os_str().is_empty() {
            "~".into()
        } else {
            format!("~/{}", rest.display())
        };
    }
    p.display().to_string()
}

fn list(v: &[String]) -> String {
    if v.len() == 1 {
        q(&v[0])
    } else {
        format!(
            "{{ {} }}",
            v.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")
        )
    }
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
    fn standalone_settings_keep_user_widget_placements() {
        let source = "hd.place('calendar#0',{anchor='right',x=16,y=-416,size=0,card=true})";
        let loaded = load_chunks(
            &[(source.into(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        let settings = Settings::from_config(&loaded);
        assert_eq!(settings.placements["calendar#0"].y, -416.0);
        let again = load_chunks(
            &[(settings.to_lua(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        assert_eq!(again.placements, loaded.placements);
    }

    #[test]
    fn bar_modules_are_never_reserialized_as_desktop_widgets() {
        let legacy = r#"hd.bar({ center={ 'clock' }, right={ 'media','volume' } })
            hd.widget('clock',{ id='bar:center:1' })
            hd.widget('media',{ id='bar:right:1' })
            hd.widget('calendar',{ id='calendar' })"#;
        let config = load_chunks(
            &[(legacy.into(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        assert_eq!(config.widgets.iter().filter(|w| w.in_bar).count(), 2);
        let mut settings = Settings::from_config(&config);
        assert_eq!(
            settings
                .widgets
                .iter()
                .map(|w| w.id.as_str())
                .collect::<Vec<_>>(),
            vec!["calendar"]
        );
        // Même un état mémoire pollué par l'ancienne version est assaini à l'écriture.
        settings
            .widgets
            .extend(config.widgets.iter().filter(|w| w.in_bar).cloned());
        let generated = settings.to_lua();
        assert!(!generated.contains("id = \"bar:"));
        let reloaded = load_chunks(
            &[(generated, "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        assert_eq!(reloaded.widgets.len(), 3);
        assert_eq!(reloaded.widgets.iter().filter(|w| w.in_bar).count(), 2);
    }

    #[test]
    fn font_selection_survives_save_and_respects_user_overrides() {
        let settings = Settings {
            font: Some("Adwaita Sans".into()),
            font_size: Some(15.5),
            ..Settings::default()
        };
        let loaded = load_chunks(
            &[(settings.to_lua(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        assert_eq!(Settings::from_config(&loaded), settings);
        assert_eq!(
            settings.apply_to(&Config::default()).general.font_size,
            15.5
        );
        let user = load_chunks(
            &[(
                "hd.config({ font='Noto Sans', font_size=18 })".into(),
                "config.lua".into(),
                Origin::User,
            )],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        assert_eq!(
            settings.apply_to(&user).general.font.as_deref(),
            Some("Noto Sans")
        );
        assert_eq!(settings.apply_to(&user).general.font_size, 18.0);
    }

    #[test]
    fn aller_retour_lua() {
        let mut s = Settings {
            key_toggle: Some("SUPER + SHIFT + D".into()),
            key_launcher: Some("SUPER + ALT + L".into()),
            key_hide: Some(String::new()),
            key_settings: Some(String::new()),
            suspend_binds: Some(false),
            float_launched: Some(false),
            show_on: Some(ShowOn::Focused),
            mode: Some(Mode::Glass),
            glass_opacity: Some(0.5),
            close_on_launch: Some(false),
            advanced_settings: Some(true),
            tour_seen: Some(true),
            app_themes: Some(["kitty".to_string()].into()),
            fade_ms: Some(300),
            transition: Some(Transition::Slide),
            crossfade_ms: Some(1200),
            running_apps: Some(RunningApps::Workspace),
            launcher_group: Some(crate::config::LauncherGroup::App),
            launcher_position: Some(crate::config::LauncherPosition::Left),
            launcher_size: Some(56),
            launcher_pinned: Some(vec!["firefox".into(), "kitty".into()]),
            launcher_show_settings: Some(false),
            lock_blur: Some(false),
            lock_dim: Some(0.5),
            lock_image: Some("/img/verrou.png".into()),
            lock_show_media: Some(false),
            idle_manage: Some(true),
            idle_suspend: Some(1800),
            icon_theme: Some("Papirus-Dark".into()),
            icon_size: Some(64),
            cell: Some(112),
            accent: Some(Rgba(0xff, 0x80, 0xbf, 0xff)),
            ..Default::default()
        };
        s.set_wallpaper(
            Selector::default(),
            WallSource::Path("/img/l'été \"a\".jpg".into()),
            Fit::Contain,
        );
        s.set_wallpaper(
            Selector {
                workspaces: Some(vec!["3".into()]),
                monitors: None,
            },
            WallSource::Rotate {
                dir: "/img".into(),
                interval: Duration::from_secs(600),
                shuffle: true,
            },
            Fit::Cover,
        );
        s.set_wallpaper(
            Selector {
                workspaces: None,
                monitors: Some(vec!["DP-1".into()]),
            },
            WallSource::Color(Rgba(1, 2, 3, 255)),
            Fit::Cover,
        );
        s.set_workspace_mode("4", Some(Mode::Opaque));
        s.add_app("firefox", None);
        s.add_app("code", Some("2"));
        s.add_app("firefox", None); // doublon ignoré
        s.workspaces = Some(7);
        s.workspace_monitors = Some(BTreeMap::from([
            ("HDMI-A-2".to_string(), vec!["1".into(), "2".into()]),
            ("primary".to_string(), vec!["6".into()]),
        ]));
        s.workspace_dedicated = Some(["HDMI-A-2".to_string()].into());
        let mut t = crate::tiling::Tiling {
            layout: Some("centered".into()),
            ..Default::default()
        };
        t.workspaces.insert("3".into(), "monocle".into());
        t.workspaces.insert("name:web".into(), "lua:mien".into());
        t.options.insert(
            "dwindle.preserve_split".into(),
            crate::tiling::OptValue::Bool(true),
        );
        t.options.insert(
            "master.orientation".into(),
            crate::tiling::OptValue::Str("center".into()),
        );
        t.options
            .insert("master.mfact".into(), crate::tiling::OptValue::Num(0.6));
        t.options.insert(
            "dwindle.force_split".into(),
            crate::tiling::OptValue::Num(2.0),
        );
        s.tiling = Some(t);
        s.primary = Some("DP-1".into());
        s.apps.insert("ai".into(), "claude".into());
        s.apps.insert("browser".into(), "brave-browser".into());
        s.show_on = Some(ShowOn::Primary);
        s.set_assign(AppMatch::App("firefox".into()), "2", false);
        s.set_assign(AppMatch::Class(r"^(org\.telegram\..*)$".into()), "5", true);
        s.set_assign(AppMatch::App("firefox".into()), "3", true); // remplace
        assert_eq!(s.assigns.len(), 2);

        let lua = s.to_lua();
        let cfg = load_chunks(
            &[(lua.clone(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap_or_else(|e| panic!("{e}\n{lua}"))
        .config;
        assert_eq!(Settings::from_config(&cfg), s, "\n{lua}");
    }

    /// Couleurs et transparence de la barre : aller-retour par settings.lua.
    #[test]
    fn couleurs_de_la_barre() {
        let mut s = Settings::default();
        let b = s.bar.get_or_insert_with(BarSettings::default);
        b.colors.text = Some(Rgba(0x80, 0xff, 0xea, 0xff));
        b.colors.critical = Some(Rgba(0xff, 0, 0, 0xff));
        b.colors.opacity = Some(0.35);
        b.colors
            .modules
            .insert("clock".into(), Rgba(0xff, 0x80, 0xbf, 0xff));
        let lua = s.to_lua();
        assert!(lua.contains("opacity = 0.35"), "{lua}");
        let cfg = load_chunks(
            &[(lua.clone(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap_or_else(|e| panic!("{e}\n{lua}"))
        .config;
        let back = Settings::from_config(&cfg).bar.unwrap().colors;
        assert_eq!(back, s.bar.unwrap().colors, "\n{lua}");
        // Clé inconnue refusée.
        let bad = "hd.bar({ colors = { texte = '#ffffff' } })";
        assert!(
            load_chunks(
                &[(bad.into(), "x.lua".into(), Origin::User)],
                Path::new("/cfg")
            )
            .is_err()
        );
    }

    /// Fichier, dossier ou application déposés : l'icône qui convient, pas de doublon.
    #[test]
    fn icones_deposees() {
        let mut s = Settings::default();
        let dir = std::env::temp_dir();
        assert_eq!(
            s.add_dropped(&dir).as_deref(),
            Some(format!("path:{}", tilde(&dir)).as_str())
        );
        assert_eq!(s.icons[0].icon.as_deref(), Some("folder"));
        assert_eq!(s.add_dropped(&dir), None, "pas de doublon");
        let id = s
            .add_dropped(std::path::Path::new("/home/moi/photo.JPG"))
            .unwrap();
        let i = s.icons.iter().find(|i| i.id == id).unwrap();
        assert_eq!(i.icon.as_deref(), Some("image-x-generic"));
        assert_eq!(i.label.as_deref(), Some("photo.JPG"));
        let id = s
            .add_dropped(std::path::Path::new(
                "/usr/share/applications/firefox.desktop",
            ))
            .unwrap();
        assert_eq!(id, "app:firefox");
        assert!(matches!(&s.icons.last().unwrap().kind, IconKind::App(a) if a == "firefox"));
        // Un .desktop hors des dossiers d'applications reste un fichier.
        let id = s
            .add_dropped(std::path::Path::new("/tmp/lanceur.desktop"))
            .unwrap();
        assert!(id.starts_with("path:"));
        // Aller-retour par settings.lua.
        let lua = s.to_lua();
        let cfg = load_chunks(
            &[(lua.clone(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap_or_else(|e| panic!("{e}\n{lua}"))
        .config;
        assert_eq!(Settings::from_config(&cfg).icons, s.icons, "\n{lua}");
    }

    #[test]
    fn widgets_et_placements_aller_retour() {
        let user = r#"hd.widget("clock", { format = "%H:%M:%S" })"#;
        let mut s = Settings::default();
        let id = s.add_widget("calendar", Anchor::Right, &[]);
        assert_eq!(id, "calendar");
        assert_eq!(s.add_widget("calendar", Anchor::Right, &[]), "calendar-2");
        s.remove_widget("calendar-2");
        s.place_widget(
            "calendar",
            Placement {
                anchor: Anchor::Center,
                x: 12.0,
                y: -40.0,
                size: 18.0,
                card: false,
            },
        );
        s.place_widget(
            "clock#0",
            Placement {
                anchor: Anchor::Top,
                x: 0.0,
                y: 8.0,
                size: 0.0,
                card: true,
            },
        );
        let lua = s.to_lua();
        let chunks = |l: String| {
            vec![
                (l, "settings.lua".to_string(), Origin::Settings),
                (user.to_string(), "u.lua".to_string(), Origin::User),
            ]
        };
        let cfg = load_chunks(&chunks(lua.clone()), Path::new("/cfg"))
            .unwrap_or_else(|e| panic!("{e}\n{lua}"))
            .config;
        let cal = cfg.widgets.iter().find(|w| w.id == "calendar").unwrap();
        assert_eq!(
            (cal.anchor, cal.x, cal.y, cal.card),
            (Anchor::Center, 12.0, -40.0, false)
        );
        let clock = cfg.widgets.iter().find(|w| w.id == "clock#0").unwrap();
        assert_eq!((clock.anchor, clock.y), (Anchor::Top, 8.0));
        let back = Settings::from_config(&cfg);
        assert_eq!(back.widgets.len(), 1);
        assert_eq!(back.placements, s.placements);
        // Déplacement en direct, identique au rechargement.
        let mut moved = back.clone();
        moved.place_widget(
            "clock#0",
            Placement {
                anchor: Anchor::BottomLeft,
                x: 4.0,
                y: 4.0,
                size: 30.0,
                card: false,
            },
        );
        let live = moved.apply_to(&cfg);
        let reloaded = load_chunks(&chunks(moved.to_lua()), Path::new("/cfg"))
            .unwrap()
            .config;
        assert_eq!(live.widgets, reloaded.widgets);
    }

    #[test]
    fn desactiver_un_widget_de_l_utilisateur() {
        let user = r#"hd.widget("clock") hd.widget("calendar")"#;
        let chunks = |l: String| {
            vec![
                (l, "settings.lua".to_string(), Origin::Settings),
                (user.to_string(), "u.lua".to_string(), Origin::User),
            ]
        };
        let base = load_chunks(&chunks(String::new()), Path::new("/cfg"))
            .unwrap()
            .config;
        let mut s = Settings::from_config(&base);
        // Éteindre l'horloge (déclarée dans hyprdesk.lua) : elle disparaît, en direct comme au rechargement.
        s.set_widget_type(
            "clock",
            false,
            Anchor::TopRight,
            &["clock#0".to_string()],
            &[],
        );
        let live = s.apply_to(&base);
        let reloaded = load_chunks(&chunks(s.to_lua()), Path::new("/cfg"))
            .unwrap()
            .config;
        assert!(!live.widgets.iter().any(|w| w.type_id == "clock"));
        assert_eq!(live.widgets, reloaded.widgets);
        assert_eq!(
            reloaded.disabled.get("clock#0").map(String::as_str),
            Some("clock")
        );
        // La rallumer réactive celle de l'utilisateur, sans en créer une seconde.
        let mut s2 = Settings::from_config(
            &load_chunks(&chunks(s.to_lua()), Path::new("/cfg"))
                .unwrap()
                .config,
        );
        assert_eq!(
            s2.set_widget_type("clock", true, Anchor::TopRight, &[], &[]),
            None
        );
        let back = load_chunks(&chunks(s2.to_lua()), Path::new("/cfg"))
            .unwrap()
            .config;
        assert_eq!(
            back.widgets.iter().filter(|w| w.type_id == "clock").count(),
            1
        );
        assert!(back.disabled.is_empty());
    }

    /// Bug relevé : éteindre « Lecteur média » rallumait « Système » (et
    /// inversement). settings.lua est relu SEUL pour l'état de l'écran ; la
    /// désactivation d'un widget de hyprdesk.lua s'y perdait.
    #[test]
    fn eteindre_deux_widgets_de_l_utilisateur_l_un_apres_l_autre() {
        let user = r#"hd.widget("system", { anchor = "bottom-right" }) hd.widget("media", { anchor = "bottom-left" })"#;
        let full = |l: &str| {
            load_chunks(
                &[
                    (l.to_string(), "settings.lua".to_string(), Origin::Settings),
                    (user.to_string(), "u.lua".to_string(), Origin::User),
                ],
                Path::new("/cfg"),
            )
            .unwrap()
            .config
        };
        // État de l'écran Réglages : settings.lua relu seul.
        let alone = |l: &str| {
            Settings::from_config(
                &load_chunks(
                    &[(l.to_string(), "settings.lua".to_string(), Origin::Settings)],
                    Path::new("/cfg"),
                )
                .unwrap()
                .config,
            )
        };
        let mut s = alone("");
        s.set_widget_type(
            "system",
            false,
            Anchor::BottomRight,
            &["system#0".into()],
            &[],
        );
        let lua = s.to_lua();
        // Puis on éteint le lecteur média, à partir de l'état relu.
        let mut s = alone(&lua);
        assert_eq!(
            s.disabled.get("system#0").map(String::as_str),
            Some("system")
        );
        s.set_widget_type("media", false, Anchor::BottomLeft, &["media#0".into()], &[]);
        let cfg = full(&s.to_lua());
        assert!(
            !cfg.widgets
                .iter()
                .any(|w| w.type_id == "system" || w.type_id == "media"),
            "les deux restent éteints : {:?}",
            cfg.widgets.iter().map(|w| &w.id).collect::<Vec<_>>()
        );
        // Rallumer le système ne touche pas au lecteur média.
        let mut s = alone(&s.to_lua());
        assert_eq!(
            s.set_widget_type("system", true, Anchor::BottomRight, &[], &[]),
            None
        );
        let cfg = full(&s.to_lua());
        assert!(cfg.widgets.iter().any(|w| w.type_id == "system"));
        assert!(!cfg.widgets.iter().any(|w| w.type_id == "media"));
    }

    #[test]
    fn barre_aller_retour() {
        let mut s = Settings {
            bar: Some(BarSettings::default()),
            ..Default::default()
        };
        s.bar.as_mut().unwrap().position = BarPosition::Bottom;
        s.bar.as_mut().unwrap().compact = crate::config::BarCompact::Always;
        s.bar
            .as_mut()
            .unwrap()
            .zone_mut(5)
            .retain(|e| e.type_id != "media");
        s.bar.as_mut().unwrap().zone_mut(4).clear();
        s.bar.as_mut().unwrap().zones[2].push(BarEntry {
            type_id: "system".into(),
            fields: vec![r#"show = { "cpu" }"#.into()],
        });
        let lua = s.to_lua();
        let cfg = load_chunks(
            &[(lua.clone(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap_or_else(|e| panic!("{e}\n{lua}"))
        .config;
        assert_eq!(Settings::from_config(&cfg).bar, s.bar, "\n{lua}");
        let off = Settings {
            bar: Some(BarSettings {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let cfg = load_chunks(
            &[(off.to_lua(), "s.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        assert!(cfg.bar.is_none());
        assert_eq!(
            Settings::from_config(&cfg).bar.map(|b| b.enabled),
            Some(false)
        );
    }

    #[test]
    fn remplacer_et_retirer() {
        let mut s = Settings::default();
        let all = Selector::default();
        s.set_wallpaper(
            all.clone(),
            WallSource::Color(Rgba(0, 0, 0, 255)),
            Fit::Cover,
        );
        s.set_wallpaper(
            all.clone(),
            WallSource::Color(Rgba(9, 9, 9, 255)),
            Fit::Cover,
        );
        assert_eq!(s.wallpapers.len(), 1);
        assert_eq!(
            s.wallpaper(&all).unwrap().source,
            WallSource::Color(Rgba(9, 9, 9, 255))
        );
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
        std::fs::write(
            &cfg_path,
            r#"hd.config({ fade_ms = 50 }) hd.icon({ app = "kitty" })"#,
        )
        .unwrap();
        let s = Settings {
            fade_ms: Some(400),
            running_apps: Some(RunningApps::All),
            ..Default::default()
        };
        let written = s.save(&cfg_path).unwrap();
        assert_eq!(written, dir.join("hyprdesk/settings.lua"));
        assert!(!dir.join("hyprdesk/settings.lua.tmp").exists());
        let c = crate::config::lua::load_file(&cfg_path).unwrap().config;
        // Le Lua de l'utilisateur garde le dernier mot ; le reste vient des réglages.
        assert_eq!(c.general.fade_ms, 50);
        assert_eq!(c.general.running_apps, RunningApps::All);
        assert_eq!(c.icons.len(), 1);
        assert_eq!(
            Settings::from_config(&crate::config::lua::load_settings_only(&cfg_path).unwrap()),
            s
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn application_en_direct_identique_au_rechargement() {
        let user_src = r##"hd.config({ fade_ms = 50 }) hd.wallpaper({ color = "#222222" }) hd.icon({ app = "kitty" })"##;
        let old = Settings {
            show_on: Some(ShowOn::Focused),
            ..Default::default()
        };
        let base = load_chunks(
            &[
                (old.to_lua(), "settings.lua".into(), Origin::Settings),
                (user_src.into(), "user.lua".into(), Origin::User),
            ],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        let mut new = old.clone();
        new.show_on = Some(ShowOn::All);
        new.fade_ms = Some(900); // surchargé par l'utilisateur : sans effet
        new.running_apps = Some(RunningApps::Workspace);
        new.set_wallpaper(
            Selector::default(),
            WallSource::Color(Rgba(1, 1, 1, 255)),
            Fit::Cover,
        );
        new.add_app("firefox", None);
        let live = new.apply_to(&base);
        let reloaded = load_chunks(
            &[
                (new.to_lua(), "settings.lua".into(), Origin::Settings),
                (user_src.into(), "user.lua".into(), Origin::User),
            ],
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
    fn affectations_de_l_utilisateur_l_emportent() {
        let user = r#"hd.config({ workspaces = 4 }) hd.assign({ app = "firefox", workspace = 9 })"#;
        let chunks = |l: String| {
            vec![
                (l, "settings.lua".to_string(), Origin::Settings),
                (user.to_string(), "u.lua".to_string(), Origin::User),
            ]
        };
        let mut s = Settings {
            workspaces: Some(6),
            ..Default::default()
        };
        s.set_assign(AppMatch::App("firefox".into()), "2", false);
        s.set_assign(AppMatch::App("kitty".into()), "1", true);
        let base = load_chunks(&chunks(String::new()), Path::new("/cfg"))
            .unwrap()
            .config;
        let live = s.apply_to(&base);
        let reloaded = load_chunks(&chunks(s.to_lua()), Path::new("/cfg"))
            .unwrap()
            .config;
        assert_eq!(live.assigns, reloaded.assigns);
        assert_eq!(live.general.workspaces, Some(4));
        assert_eq!(reloaded.general.workspaces, Some(4));
        let firefox: Vec<_> = live
            .assigns
            .iter()
            .filter(|a| a.app == AppMatch::App("firefox".into()))
            .collect();
        assert_eq!(firefox.len(), 1);
        assert_eq!(firefox[0].workspace, "9");
        // Retirer le réglage rend la main à Hyprland.
        let only = load_chunks(
            &[(s.to_lua(), "settings.lua".into(), Origin::Settings)],
            Path::new("/cfg"),
        )
        .unwrap()
        .config;
        let off = Settings {
            workspaces: None,
            ..s.clone()
        };
        assert_eq!(off.apply_to(&only).general.workspaces, None);
    }

    #[test]
    fn vide_ne_produit_que_l_entete() {
        let lua = Settings::default().to_lua();
        assert!(
            lua.lines().all(|l| l.starts_with("--") || l.is_empty()),
            "{lua}"
        );
    }

    #[test]
    fn deplacer_un_module_de_barre() {
        let ids = |b: &BarSettings, z: usize| -> Vec<String> {
            b.zones[z].iter().map(|e| e.type_id.clone()).collect()
        };
        let mut b = BarSettings {
            zones: [
                vec![BarEntry::new("a"), BarEntry::new("b"), BarEntry::new("c")],
                vec![],
                vec![BarEntry::new("x")],
            ],
            ..Default::default()
        };
        // Sur place : rien.
        assert!(!b.move_entry((0, 1), (0, 1)) && !b.move_entry((0, 1), (0, 2)));
        // Vers la droite dans la même zone : avant « c » … puis en bout.
        assert!(b.move_entry((0, 0), (0, 2)));
        assert_eq!(ids(&b, 0), ["b", "a", "c"]);
        assert!(b.move_entry((0, 0), (0, 3)));
        assert_eq!(ids(&b, 0), ["a", "c", "b"]);
        // Vers une zone vide, puis avant « x ».
        assert!(b.move_entry((0, 2), (1, 0)));
        assert!(b.move_entry((0, 0), (2, 0)));
        assert_eq!(
            (ids(&b, 0), ids(&b, 1), ids(&b, 2)),
            (
                vec!["c".to_string()],
                vec!["b".to_string()],
                vec!["a".to_string(), "x".to_string()]
            )
        );
        assert!(!b.move_entry((0, 5), (1, 0)));
    }
}
