//! Modèle de configuration, indépendant de Lua : ce que `hd.*` produit.

use std::path::PathBuf;
use std::time::Duration;

/// Couleur RGBA 8 bits, lue depuis `#rrggbb`, `#rrggbbaa`, `rgb(…)` ou `rgba(…)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix('#') {
            return Self::from_hex(hex);
        }
        // Forme Hyprland : rgb(9580ff) / rgba(454158aa)
        if let Some(inner) = s.strip_prefix("rgba(").and_then(|r| r.strip_suffix(')')) {
            return Self::from_hex(inner);
        }
        if let Some(inner) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
            return Self::from_hex(inner);
        }
        None
    }

    fn from_hex(hex: &str) -> Option<Self> {
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        match hex.len() {
            6 => Some(Rgba(byte(0)?, byte(2)?, byte(4)?, 255)),
            8 => Some(Rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Le fond d'écran du workspace couvre les fenêtres.
    #[default]
    Opaque,
    /// Fond translucide : les fenêtres restent visibles, floutées par Hyprland.
    Glass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShowOn {
    /// Le bureau apparaît sur tous les écrans.
    #[default]
    All,
    /// Seulement sur l'écran qui a le focus.
    Focused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Fit {
    #[default]
    Cover,
    Contain,
    Fill,
    Center,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub fg: Rgba,
    pub bg: Rgba,
    pub accent: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        // Dracula PRO, cohérent avec la config Hyprland de l'auteur.
        Theme {
            fg: Rgba(0xf8, 0xf8, 0xf2, 0xff),
            bg: Rgba(0x22, 0x21, 0x2c, 0xff),
            accent: Rgba(0x95, 0x80, 0xff, 0xff),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// Côté d'une cellule d'icône, en pixels logiques.
    pub cell: u16,
    pub gap: u16,
    /// Marge entre le bord de l'écran et la grille.
    pub margin: u16,
    /// Taille de l'image d'icône dans la cellule.
    pub icon_size: u16,
}

impl Default for Grid {
    fn default() -> Self {
        Grid { cell: 96, gap: 16, margin: 32, icon_size: 48 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct General {
    pub mode: Mode,
    pub fade_ms: u64,
    pub show_on: ShowOn,
    pub close_on_launch: bool,
    pub icon_theme: Option<String>,
    pub font: Option<String>,
    pub font_size: f32,
    pub terminal: String,
    pub grid: Grid,
    pub theme: Theme,
    /// Opacité du voile en mode verre (0 = transparent, 1 = opaque).
    pub glass_opacity: f32,
    /// Couleur de fond quand aucun fond d'écran ne s'applique.
    pub background: Rgba,
}

impl Default for General {
    fn default() -> Self {
        General {
            mode: Mode::Opaque,
            fade_ms: 150,
            show_on: ShowOn::All,
            close_on_launch: true,
            icon_theme: None,
            font: None,
            font_size: 14.0,
            terminal: "kitty".into(),
            grid: Grid::default(),
            theme: Theme::default(),
            glass_opacity: 0.35,
            background: Rgba(0x22, 0x21, 0x2c, 0xff),
        }
    }
}

/// Sélecteur commun à toutes les règles : `None` (ou `"*"`) = n'importe lequel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Selector {
    /// Noms ou identifiants de workspace (`"1"`, `"special:magic"`, `"web"`).
    pub workspaces: Option<Vec<String>>,
    /// Noms d'écran (`"DP-1"`).
    pub monitors: Option<Vec<String>>,
}

impl Selector {
    pub fn matches(&self, monitor: &str, workspace: &str) -> bool {
        let ok = |list: &Option<Vec<String>>, v: &str| list.as_ref().is_none_or(|l| l.iter().any(|x| x == v));
        ok(&self.workspaces, workspace) && ok(&self.monitors, monitor)
    }

    /// Spécificité : un workspace nommé pèse plus qu'un écran nommé.
    pub fn specificity(&self) -> u8 {
        (if self.workspaces.is_some() { 2 } else { 0 }) + (if self.monitors.is_some() { 1 } else { 0 })
    }

    pub fn is_workspace_wildcard(&self) -> bool {
        self.workspaces.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WallSource {
    Path(PathBuf),
    Rotate { dir: PathBuf, interval: Duration, shuffle: bool },
    /// Aplat de couleur, sans image.
    Color(Rgba),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WallpaperRule {
    pub sel: Selector,
    pub source: WallSource,
    pub fit: Fit,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DesktopRule {
    pub sel: Selector,
    pub mode: Option<Mode>,
    /// `false` : les icônes et widgets déclarés pour tous les workspaces ne
    /// s'affichent pas sur ce bureau.
    pub inherit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IconKind {
    /// Identifiant d'application `.desktop` (`firefox`, `org.gnome.Nautilus`).
    App(String),
    /// Fichier ou dossier, ouvert avec `xdg-open`.
    Path(PathBuf),
    /// Commande shell.
    Exec { cmd: String, terminal: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconDef {
    pub sel: Selector,
    /// Clé stable (pour la disposition mémorisée).
    pub id: String,
    pub label: Option<String>,
    pub kind: IconKind,
    /// Nom d'icône du thème, ou chemin vers une image.
    pub icon: Option<String>,
    /// Cellule (colonne, ligne) imposée.
    pub pos: Option<(u16, u16)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    TopLeft,
    Top,
    #[default]
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "top-left" => Anchor::TopLeft,
            "top" => Anchor::Top,
            "top-right" => Anchor::TopRight,
            "left" => Anchor::Left,
            "center" => Anchor::Center,
            "right" => Anchor::Right,
            "bottom-left" => Anchor::BottomLeft,
            "bottom" => Anchor::Bottom,
            "bottom-right" => Anchor::BottomRight,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SysItem {
    Cpu,
    Ram,
    Disk(String),
    Load,
    Uptime,
}

impl SysItem {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "cpu" => SysItem::Cpu,
            "ram" | "mem" => SysItem::Ram,
            "load" => SysItem::Load,
            "uptime" => SysItem::Uptime,
            "disk" => SysItem::Disk("/".into()),
            _ => SysItem::Disk(s.strip_prefix("disk:")?.to_string()),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Clock { format: String },
    Calendar,
    System { show: Vec<SysItem> },
    /// `player` : sous-chaîne du nom MPRIS, `*` = le premier qui joue.
    Media { player: String },
    Command { exec: String, timeout: Duration },
    /// Index de la fonction `render` dans le registre Lua.
    Lua { func: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetDef {
    pub sel: Selector,
    pub id: String,
    pub kind: WidgetKind,
    pub anchor: Anchor,
    pub x: f32,
    pub y: f32,
    /// Taille du texte principal.
    pub size: f32,
    pub interval: Duration,
    /// Fond en carte (true) ou texte nu sur le fond d'écran (false).
    pub card: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    Show,
    Hide,
    Workspace,
}

impl HookEvent {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "show" => HookEvent::Show,
            "hide" => HookEvent::Hide,
            "workspace" => HookEvent::Workspace,
            _ => return None,
        })
    }
}

/// Configuration complète, sans référence à l'état Lua (sauf indices de fonctions).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub general: General,
    pub wallpapers: Vec<WallpaperRule>,
    pub desktops: Vec<DesktopRule>,
    pub icons: Vec<IconDef>,
    pub widgets: Vec<WidgetDef>,
    pub hooks: Vec<(HookEvent, usize)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couleurs_hex_et_hyprland() {
        assert_eq!(Rgba::parse("#9580ff"), Some(Rgba(0x95, 0x80, 0xff, 0xff)));
        assert_eq!(Rgba::parse("rgba(454158aa)"), Some(Rgba(0x45, 0x41, 0x58, 0xaa)));
        assert_eq!(Rgba::parse("rgb(22212c)"), Some(Rgba(0x22, 0x21, 0x2c, 0xff)));
        assert_eq!(Rgba::parse("bleu"), None);
        assert_eq!(Rgba::parse("#12"), None);
    }

    #[test]
    fn selecteur_et_specificite() {
        let any = Selector::default();
        let ws = Selector { workspaces: Some(vec!["3".into()]), monitors: None };
        let both = Selector { workspaces: Some(vec!["3".into()]), monitors: Some(vec!["DP-1".into()]) };
        assert!(any.matches("HDMI-A-2", "1"));
        assert!(ws.matches("HDMI-A-2", "3"));
        assert!(!ws.matches("HDMI-A-2", "4"));
        assert!(!both.matches("HDMI-A-2", "3"));
        assert!(any.specificity() < ws.specificity() && ws.specificity() < both.specificity());
    }

    #[test]
    fn elements_systeme() {
        assert_eq!(SysItem::parse("disk:/home"), Some(SysItem::Disk("/home".into())));
        assert_eq!(SysItem::parse("cpu"), Some(SysItem::Cpu));
        assert_eq!(SysItem::parse("gpu"), None);
    }
}
