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
    /// Fond d’écran net ; les fenêtres sont mises de côté.
    #[default]
    Opaque,
    /// Fond d’écran flouté et teinté ; les fenêtres sont mises de côté.
    Glass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShowOn {
    /// Le bureau apparaît sur tous les écrans.
    #[default]
    All,
    /// Seulement sur l'écran qui a le focus.
    Focused,
    /// Seulement sur l'écran principal.
    Primary,
}

impl ShowOn {
    pub fn as_str(self) -> &'static str {
        match self {
            ShowOn::All => "all",
            ShowOn::Focused => "focused",
            ShowOn::Primary => "primary",
        }
    }
}

/// Animation d'apparition du bureau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Transition {
    #[default]
    Fade,
    /// Fondu, et le contenu remonte de quelques pixels.
    Slide,
    /// Apparition immédiate.
    None,
}

impl Transition {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "fade" => Transition::Fade,
            "slide" => Transition::Slide,
            "none" => Transition::None,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Transition::Fade => "fade",
            Transition::Slide => "slide",
            Transition::None => "none",
        }
    }
}

/// Applications ouvertes affichées dans le dock du bureau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunningApps {
    #[default]
    Off,
    /// Les fenêtres du workspace affiché.
    Workspace,
    /// Toutes les fenêtres.
    All,
}

impl RunningApps {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "off" => RunningApps::Off,
            "workspace" => RunningApps::Workspace,
            "all" => RunningApps::All,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            RunningApps::Off => "off",
            RunningApps::Workspace => "workspace",
            RunningApps::All => "all",
        }
    }
}

/// Regroupement des fenêtres dans le lanceur (dock).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LauncherGroup {
    /// Une entrée par fenêtre.
    #[default]
    None,
    /// Une entrée par application (classe de fenêtre, donc par binaire),
    /// avec le nombre de fenêtres ; un clic passe de l'une à l'autre.
    App,
    /// Fenêtres rangées par workspace, chaque groupe sous son numéro.
    Workspace,
}

impl LauncherGroup {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "none" => LauncherGroup::None,
            "app" => LauncherGroup::App,
            "workspace" => LauncherGroup::Workspace,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            LauncherGroup::None => "none",
            LauncherGroup::App => "app",
            LauncherGroup::Workspace => "workspace",
        }
    }
}

/// Écran de verrouillage (`hyprdesk lock`).
#[derive(Debug, Clone, PartialEq)]
pub struct LockCfg {
    /// Fond flouté.
    pub blur: bool,
    /// Assombrissement du fond (0..1).
    pub dim: f32,
    /// Image propre au verrouillage ; sinon le fond du bureau de l'écran principal.
    pub image: Option<std::path::PathBuf>,
    pub show_user: bool,
    pub show_battery: bool,
    pub show_media: bool,
    /// Empreinte (fprintd) en plus du mot de passe, si un lecteur est là.
    pub fingerprint: bool,
}

impl Default for LockCfg {
    fn default() -> Self {
        LockCfg {
            blur: true,
            dim: 0.35,
            image: None,
            show_user: true,
            show_battery: true,
            show_media: true,
            fingerprint: true,
        }
    }
}

/// Inactivité et veille, orchestrées par hyprdesk via hypridle (`crate::idle`).
/// Délais en secondes depuis la dernière activité ; 0 = étape coupée.
#[derive(Debug, Clone, PartialEq)]
pub struct IdleCfg {
    /// hyprdesk écrit la config de hypridle et le lance (sinon : rien).
    pub manage: bool,
    /// Assombrir l'écran.
    pub dim: u32,
    /// Verrouiller (`hyprdesk lock`).
    pub lock: u32,
    /// Éteindre les écrans.
    pub dpms: u32,
    /// Mettre en veille.
    pub suspend: u32,
}

impl Default for IdleCfg {
    fn default() -> Self {
        IdleCfg {
            manage: false,
            dim: 240,
            lock: 300,
            dpms: 360,
            suspend: 0,
        }
    }
}

/// Bord du bureau où se tient le lanceur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LauncherPosition {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}

impl LauncherPosition {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "bottom" => LauncherPosition::Bottom,
            "top" => LauncherPosition::Top,
            "left" => LauncherPosition::Left,
            "right" => LauncherPosition::Right,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            LauncherPosition::Bottom => "bottom",
            LauncherPosition::Top => "top",
            LauncherPosition::Left => "left",
            LauncherPosition::Right => "right",
        }
    }
    pub fn vertical(self) -> bool {
        matches!(self, LauncherPosition::Left | LauncherPosition::Right)
    }
}

/// Apparence et contenu du lanceur.
#[derive(Debug, Clone, PartialEq)]
pub struct LauncherCfg {
    pub position: LauncherPosition,
    /// Taille des icônes, en pixels.
    pub size: u16,
    /// Applications épinglées (identifiants `.desktop`), toujours présentes.
    pub pinned: Vec<String>,
    pub show_settings: bool,
    pub show_assistant: bool,
}

/// Présentation du lanceur d'applications (`hyprdesk apps`, `search`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppsStyle {
    /// Grande feuille en plein bureau, carte centrée dans les autres modes.
    #[default]
    Auto,
    /// Toujours la grande feuille (façon Launchpad, tiroir One UI).
    Sheet,
    /// Toujours la carte centrée (façon Spotlight).
    Spotlight,
}

impl AppsStyle {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "auto" => AppsStyle::Auto,
            "plein" | "sheet" | "full" => AppsStyle::Sheet,
            "central" | "spotlight" | "center" => AppsStyle::Spotlight,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AppsStyle::Auto => "auto",
            AppsStyle::Sheet => "plein",
            AppsStyle::Spotlight => "central",
        }
    }
}

/// Lanceur d'applications : présentation, suggestions, catégories.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppsCfg {
    /// Le même lanceur dans tous les modes : `keys.search` ouvre celui de
    /// hyprdesk partout, pris à hyprland.lua s'il y est lié (rofi…). Sinon,
    /// hors plein bureau, la combinaison reste à hyprland.lua.
    pub unified: bool,
    pub style: AppsStyle,
    /// Applications mises en avant (les plus lancées), 0 : aucune.
    pub suggestions: u8,
    pub categories: bool,
}

impl Default for AppsCfg {
    fn default() -> Self {
        AppsCfg {
            unified: true,
            style: AppsStyle::Auto,
            suggestions: 8,
            categories: true,
        }
    }
}

impl Default for LauncherCfg {
    fn default() -> Self {
        LauncherCfg {
            position: LauncherPosition::Bottom,
            size: 42,
            pinned: Vec::new(),
            show_settings: true,
            show_assistant: true,
        }
    }
}

/// D'où vient une règle : l'écran Réglages (`hyprdesk/settings.lua`) ou le Lua de l'utilisateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Origin {
    Settings,
    #[default]
    User,
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
    /// Nom du thème (prédéfini, importé, ou choisi par l'utilisateur).
    pub name: String,
    pub fg: Rgba,
    pub bg: Rgba,
    pub accent: Rgba,
    /// Effet de surface de chaque endroit de l'interface.
    pub effects: std::collections::BTreeMap<Place, Effect>,
}

impl Default for Theme {
    fn default() -> Self {
        // Palette ardoise et iris, partagée par les surfaces de l’interface.
        Theme {
            name: "Ardoise".into(),
            fg: Rgba(0xf2, 0xf4, 0xfc, 0xff),
            bg: Rgba(0x17, 0x1b, 0x27, 0xff),
            accent: Rgba(0xa6, 0x9c, 0xff, 0xff),
            effects: Place::ALL
                .iter()
                .map(|p| (*p, p.default_effect()))
                .collect(),
        }
    }
}

impl Theme {
    /// Effet d'un endroit (celui par défaut s'il n'est pas réglé).
    pub fn effect(&self, p: Place) -> Effect {
        self.effects
            .get(&p)
            .copied()
            .unwrap_or_else(|| p.default_effect())
    }

    /// Espaces de noms des couches que Hyprland floute.
    pub fn blurred_namespaces(&self) -> Vec<&'static str> {
        Place::ALL
            .iter()
            .filter(|p| p.blurrable() && self.effect(**p).blur)
            .flat_map(|p| p.namespaces().iter().copied())
            .collect()
    }
}

/// Endroit de l'interface qui porte un effet de surface (opacité, flou).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    Bar,
    Menus,
    Launcher,
    Settings,
    Widgets,
}

impl Place {
    pub const ALL: [Place; 5] = [
        Place::Bar,
        Place::Menus,
        Place::Launcher,
        Place::Settings,
        Place::Widgets,
    ];

    pub fn parse(s: &str) -> Option<Self> {
        Place::ALL.into_iter().find(|p| p.as_str() == s)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Place::Bar => "bar",
            Place::Menus => "menus",
            Place::Launcher => "launcher",
            Place::Settings => "settings",
            Place::Widgets => "widgets",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Place::Bar => "Barre",
            Place::Menus => crate::i18n::tr("Menus déroulants"),
            Place::Launcher => "Lanceur",
            Place::Settings => crate::i18n::tr("Réglages"),
            Place::Widgets => crate::i18n::tr("Widgets du bureau"),
        }
    }

    /// Effet livré : l'apparence d'origine de hyprdesk.
    pub fn default_effect(self) -> Effect {
        let opacity = match self {
            Place::Bar => 0.72,
            Place::Menus => 0.90,
            Place::Launcher => 0.88,
            Place::Settings => 0.97,
            Place::Widgets => 0.84,
        };
        Effect {
            opacity,
            blur: self.blurrable(),
        }
    }

    /// Les widgets sont dessinés sur la couche du bureau, avec le fond
    /// d'écran : pas de flou propre.
    pub fn blurrable(self) -> bool {
        !matches!(self, Place::Widgets)
    }

    /// Couches Hyprland de cet endroit.
    pub fn namespaces(self) -> &'static [&'static str] {
        match self {
            Place::Bar => &["hyprdesk-bar"],
            Place::Menus => &["hyprdesk-popover"],
            Place::Launcher => &["hyprdesk-launcher", "hyprdesk-launcher-popup"],
            Place::Settings => &["hyprdesk-settings"],
            Place::Widgets => &[],
        }
    }
}

/// Effet de surface : opacité du fond (0..1) et flou de ce qu'il y a derrière.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    pub opacity: f32,
    pub blur: bool,
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
        Grid {
            cell: 96,
            gap: 16,
            margin: 32,
            icon_size: 48,
        }
    }
}

/// État d'un widget rendu : teinte (avertissement, critique), mise en avant, ou masqué.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderState {
    #[default]
    Normal,
    /// Mis en avant (ex. Wi-Fi connecté, Ne pas déranger actif).
    Active,
    Warning,
    Critical,
    /// Rien à afficher (ex. pas de batterie) : le module disparaît.
    Hidden,
}

impl RenderState {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "normal" => RenderState::Normal,
            "active" => RenderState::Active,
            "warning" => RenderState::Warning,
            "critical" => RenderState::Critical,
            "hidden" => RenderState::Hidden,
            _ => return None,
        })
    }
}

/// Ce qu'un widget Lua rend : du texte, ou une table { text, icon, tooltip, state }.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rendered {
    pub text: String,
    /// Nom d'icône du thème (de préférence `…-symbolic`, teintée) ou chemin.
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub state: RenderState,
}

/// Entrée d'un menu déroulant (plugin, zone de notification).
#[derive(Debug, Clone, PartialEq)]
pub enum MenuEntry {
    Item {
        label: String,
        detail: Option<String>,
        icon: Option<String>,
        /// Case à cocher : Some(coché).
        checked: Option<bool>,
        enabled: bool,
        /// Index de l'action (fonction Lua, ou id dbusmenu).
        action: Option<i64>,
        /// Demander confirmation avant l'action (texte de la question).
        confirm: Option<String>,
    },
    Separator,
    Header(String),
    /// Image (pochette d'album…) : un fichier local, en tête de menu.
    Image(std::path::PathBuf),
    /// Champ de saisie : Entrée appelle l'action avec le texte (mot de passe
    /// Wi-Fi, nom…), puis le menu se ferme.
    Input {
        label: Option<String>,
        placeholder: String,
        /// Saisie masquée (mot de passe).
        secret: bool,
        action: Option<i64>,
    },
}

/// Position de la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarPosition {
    #[default]
    Top,
    Bottom,
}

/// Module de la barre : propre à la barre, ou widget (par son id).
#[derive(Debug, Clone, PartialEq)]
pub enum BarItem {
    Workspaces,
    /// Titre de la fenêtre active.
    Window,
    /// Bouton afficher / masquer le bureau.
    Desktop,
    /// Volume de la sortie audio (wpctl).
    Volume,
    /// Zone de notification (StatusNotifierItem).
    Tray,
    /// Luminosité : rétroéclairage (portable) ou écrans externes (DDC/CI).
    Brightness,
    /// Widget (intégré ou plugin) affiché en version compacte.
    Widget(String),
}

/// Modules propres à la barre (les autres noms sont des types de widgets).
pub const BAR_ITEMS: &[(&str, &str)] = &[
    ("workspaces", "Workspaces"),
    ("window", "Fenêtre active"),
    ("volume", "Volume"),
    ("brightness", "Luminosité"),
    ("tray", "Zone de notification"),
    ("desktop", "Bouton bureau"),
];

impl BarItem {
    pub fn builtin(name: &str) -> Option<Self> {
        Some(match name {
            "workspaces" => BarItem::Workspaces,
            "window" => BarItem::Window,
            "desktop" => BarItem::Desktop,
            "volume" => BarItem::Volume,
            "tray" => BarItem::Tray,
            "brightness" | "luminosite" => BarItem::Brightness,
            _ => return None,
        })
    }
}

/// Barre native de hyprdesk (`hd.bar`) : au-dessus des fenêtres, zone réservée.
#[derive(Debug, Clone, PartialEq)]
pub struct BarDef {
    pub monitors: Option<Vec<String>>,
    pub position: BarPosition,
    pub height: u16,
    /// Gauche, centre, droite.
    pub zones: [Vec<BarItem>; 3],
    /// Modules des écrans verticaux (portrait), s'ils diffèrent.
    pub portrait: Option<[Vec<BarItem>; 3]>,
    pub compact: BarCompact,
    /// Couleurs propres à la barre (sinon celles du thème).
    pub colors: BarColors,
}

/// Couleurs de la barre : `None` = celle du thème ; et une couleur par
/// module (`clock`, `workspaces`, `volume`…), pour son texte et ses icônes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarColors {
    pub text: Option<Rgba>,
    pub background: Option<Rgba>,
    pub accent: Option<Rgba>,
    pub warning: Option<Rgba>,
    pub critical: Option<Rgba>,
    /// Opacité du fond (0 transparent … 1 opaque) ; sinon l'effet « Barre » du thème.
    pub opacity: Option<f32>,
    pub modules: std::collections::BTreeMap<String, Rgba>,
}

/// Couleurs réglables de la barre : (clé Lua, libellé).
pub const BAR_COLOR_SLOTS: &[(&str, &str)] = &[
    ("text", "Texte et icônes"),
    ("background", "Fond"),
    ("accent", "Accent (workspace actif, module actif)"),
    ("warning", "Avertissement"),
    ("critical", "Critique"),
];

impl BarColors {
    /// Avertissement et critique par défaut (le thème n'en déclare pas).
    pub const WARNING: Rgba = Rgba(0xff, 0xb8, 0x6c, 0xff);
    pub const CRITICAL: Rgba = Rgba(0xff, 0x55, 0x55, 0xff);

    pub fn slot(&self, key: &str) -> Option<Rgba> {
        match key {
            "text" => self.text,
            "background" => self.background,
            "accent" => self.accent,
            "warning" => self.warning,
            "critical" => self.critical,
            _ => None,
        }
    }

    pub fn set_slot(&mut self, key: &str, c: Option<Rgba>) {
        match key {
            "text" => self.text = c,
            "background" => self.background = c,
            "accent" => self.accent = c,
            "warning" => self.warning = c,
            "critical" => self.critical = c,
            _ => {}
        }
    }
}

/// Barre compacte : titres raccourcis, heure seule, icônes sans libellé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarCompact {
    /// Sur les écrans étroits (moins de 1600 px) ou verticaux.
    #[default]
    Auto,
    Always,
    Never,
}

impl BarCompact {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "auto" => BarCompact::Auto,
            "always" => BarCompact::Always,
            "never" => BarCompact::Never,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            BarCompact::Auto => "auto",
            BarCompact::Always => "always",
            BarCompact::Never => "never",
        }
    }
}

/// Composant de session lancé et supervisé par hyprdesk (`hd.component`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDef {
    pub name: String,
    pub exec: String,
    /// Relancer après un arrêt inattendu.
    pub restart: bool,
}

/// Raccourcis de hyprdesk, installés dans Hyprland par hyprdesk lui-même.
/// Chaîne vide = pas de raccourci.
#[derive(Debug, Clone, PartialEq)]
pub struct Keys {
    /// full-desktop, ou retour à no-desktop s'il y est déjà.
    pub toggle: String,
    /// launcher-only, ou retour à no-desktop s'il y est déjà.
    pub launcher: String,
    /// no-desktop.
    pub hide: String,
    /// Ouvrir les réglages.
    pub settings: String,
    /// Verrouiller la session (`hyprdesk lock`), bureau affiché compris.
    pub lock: String,
    /// Sortie de secours : masque le bureau et rend tous les raccourcis.
    pub leave: String,
    /// Sélecteur de fenêtres (`hyprdesk switch`) ; avec Maj : en arrière.
    pub switch: String,
    /// Lanceur d'applications (`hyprdesk search`), dans tous les modes.
    pub search: String,
    /// Plein bureau : un autre raccourci pour le lanceur (vide : `search`).
    pub search_desktop: String,
    /// Vue d'ensemble des fenêtres (`hyprdesk overview`).
    pub overview: String,
    /// Raccourcis que hyprdesk prend à hyprland.lua : la combinaison y est
    /// déliée tant qu'elle figure ici, rendue (Hyprland relit sa config) sinon.
    pub take: Vec<String>,
    /// Touches multimédia (volume, luminosité, lecture) par hyprdesk : là où
    /// hyprland.lua ne les lie pas déjà, et toujours bureau affiché.
    pub media: bool,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            toggle: "SUPER + D".into(),
            launcher: "SUPER + ALT + D".into(),
            switch: "ALT + Tab".into(),
            // Le lanceur de hyprdesk, dans tous les modes (apps_unified).
            search: "SUPER + R".into(),
            search_desktop: String::new(),
            overview: "SUPER + Tab".into(),
            take: Vec::new(),
            hide: "SUPER + CTRL + D".into(),
            settings: "SUPER + SHIFT + D".into(),
            // Vide : le verrouillage de hyprland.lua (SUPER + L, posé par
            // scripts/install.sh). Réglé, hyprdesk le remplace.
            lock: String::new(),
            leave: "SUPER + Escape".into(),
            media: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct General {
    pub keys: Keys,
    /// Bureau affiché : suspendre les raccourcis de Hyprland (submap
    /// « hyprdesk »), sauf ceux marqués `submap_universal`.
    /// Historique du presse-papiers : hyprdesk lance cliphist (`crate::clipboard`).
    pub clipboard_history: bool,
    /// Démarrage automatique XDG (`crate::autostart`), une fois par session.
    pub autostart: bool,
    /// Bureau type gestionnaire de fichiers : le dossier Bureau XDG en icônes (`crate::desktop_files`).
    pub desktop_files: bool,
    pub suspend_binds: bool,
    /// Fenêtres ouvertes depuis le bureau resté affiché : flottantes et centrées,
    /// le bureau autour d'elles.
    pub float_launched: bool,
    pub mode: Mode,
    pub fade_ms: u64,
    pub transition: Transition,
    /// Durée du fondu enchaîné entre deux fonds d'écran.
    pub crossfade_ms: u64,
    pub running_apps: RunningApps,
    /// Regroupement des fenêtres du lanceur.
    pub launcher_group: LauncherGroup,
    /// Position, taille, applications épinglées, boutons du lanceur.
    pub launcher: LauncherCfg,
    /// Lanceur d'applications (grille ou carte centrée).
    pub apps_launcher: AppsCfg,
    /// Écran de verrouillage.
    pub lock: LockCfg,
    /// Inactivité et veille (hypridle orchestré).
    pub idle: IdleCfg,
    pub show_on: ShowOn,
    pub close_on_launch: bool,
    /// Écran Réglages : tous les détails (sinon l'essentiel seulement).
    pub advanced_settings: bool,
    /// Visite guidée faite ou écartée : plus proposée à l'ouverture des Réglages.
    pub tour_seen: bool,
    /// Applications qui prennent le thème de hyprdesk (`kitty`…).
    pub app_themes: std::collections::BTreeSet<String>,
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
    /// Workspaces permanents 1 à N (`None` : Hyprland les crée à la demande).
    pub workspaces: Option<u8>,
    /// Workspaces de chaque écran : écran (connecteur, ou `primary`) → workspaces,
    /// le premier étant celui que l'écran affiche par défaut.
    pub workspace_monitors: std::collections::BTreeMap<String, Vec<String>>,
    /// Placement tel que déclaré (`hd.config`), avant l'effet des écrans dédiés ;
    /// `workspace_monitors` est le placement effectif qui en découle.
    pub workspace_monitors_declared: std::collections::BTreeMap<String, Vec<String>>,
    /// Écrans dédiés : ils n'accueillent que leurs workspaces ; les autres vont
    /// sur l'écran principal.
    pub workspace_dedicated: std::collections::BTreeSet<String>,
    /// Options des plugins réglées dans Réglages → Plugins (« Configurer ») :
    /// plugin → option → valeur ; elles valent pour chaque instance, sauf
    /// les options écrites sur l'instance elle-même.
    pub plugin_options: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, crate::tiling::OptValue>,
    >,
    /// Commandes des mises à jour remplacées : source (`pacman`, `aur`…) →
    /// (relevé, mise à jour), en shell ; vide : celle de hyprdesk.
    pub update_commands: std::collections::BTreeMap<String, (String, String)>,
    /// Vue d'ensemble : commande d'un plugin de Hyprland (hyprexpo,
    /// Hyprspace…) ; vide : le sélecteur de fenêtres de hyprdesk.
    pub overview_command: String,
    /// Modes imposés : écran (`DP-1`) ou workspace (`ws:5`) → `no-desktop`,
    /// `launcher-only` ou `full-desktop`.
    pub forced_modes: std::collections::BTreeMap<String, String>,
    /// Ouverture de l'assistant IA (LYRA) : fenêtre ou terminal.
    pub assistant_mode: crate::assistant::AssistantMode,
    /// Langue de l'interface : `auto` (celle du système), `fr` ou `en`.
    pub language: String,
    /// Layouts de Hyprland réglés dans hyprdesk (paquet « Layouts »).
    pub tiling: crate::tiling::Tiling,
    /// Animations et effets de Hyprland réglés dans hyprdesk (paquet « Animations »).
    pub fx: crate::fx::Fx,
    /// Paquets intégrés désactivés (`hd.config({ disabled_plugins = { "bar" } })`) :
    /// retirés des réglages, et leur fonction est coupée.
    pub disabled_plugins: std::collections::BTreeSet<String>,
    /// Applications par défaut, par rôle (`browser`, `terminal`, `ai`…) :
    /// identifiant `.desktop` ou commande. Voir `crate::apps`.
    pub apps: std::collections::BTreeMap<String, String>,
    /// Écran principal (connecteur) : ancre de la disposition en 0×0, et
    /// cible de `show_on = "primary"`, de `hd.bar({ monitor = "primary" })`
    /// et de l'écran Réglages.
    pub primary: Option<String>,
}

impl Default for General {
    fn default() -> Self {
        General {
            keys: Keys::default(),
            suspend_binds: true,
            clipboard_history: true,
            autostart: false,
            desktop_files: false,
            float_launched: true,
            mode: Mode::Opaque,
            fade_ms: 150,
            transition: Transition::Fade,
            crossfade_ms: 700,
            running_apps: RunningApps::Off,
            launcher_group: LauncherGroup::None,
            launcher: LauncherCfg::default(),
            apps_launcher: AppsCfg::default(),
            lock: LockCfg::default(),
            idle: IdleCfg::default(),
            show_on: ShowOn::All,
            close_on_launch: true,
            advanced_settings: false,
            tour_seen: false,
            app_themes: Default::default(),
            icon_theme: None,
            font: None,
            font_size: 14.0,
            terminal: "kitty".into(),
            grid: Grid::default(),
            theme: Theme::default(),
            glass_opacity: 0.35,
            background: Rgba(0x13, 0x17, 0x24, 0xff),
            workspaces: None,
            workspace_monitors: Default::default(),
            workspace_monitors_declared: Default::default(),
            forced_modes: Default::default(),
            overview_command: String::new(),
            plugin_options: Default::default(),
            update_commands: Default::default(),
            workspace_dedicated: Default::default(),
            primary: None,
            apps: Default::default(),
            disabled_plugins: Default::default(),
            assistant_mode: Default::default(),
            language: "auto".into(),
            tiling: Default::default(),
            fx: Default::default(),
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
        let ok = |list: &Option<Vec<String>>, v: &str| {
            list.as_ref().is_none_or(|l| l.iter().any(|x| x == v))
        };
        ok(&self.workspaces, workspace) && ok(&self.monitors, monitor)
    }

    /// Spécificité : un workspace nommé pèse plus qu'un écran nommé.
    pub fn specificity(&self) -> u8 {
        (if self.workspaces.is_some() { 2 } else { 0 })
            + (if self.monitors.is_some() { 1 } else { 0 })
    }

    pub fn is_workspace_wildcard(&self) -> bool {
        self.workspaces.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WallSource {
    Path(PathBuf),
    Rotate {
        dir: PathBuf,
        interval: Duration,
        shuffle: bool,
    },
    /// Aplat de couleur, sans image.
    Color(Rgba),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WallpaperRule {
    pub origin: Origin,
    pub sel: Selector,
    pub source: WallSource,
    pub fit: Fit,
    /// Scène étalée sur tous les écrans : l'image couvre la disposition entière
    /// (positions et tailles logiques de Hyprland) et chaque écran en montre sa
    /// part, découpée automatiquement. `fit` ne s'applique pas.
    pub span: bool,
    /// Scène étalée sur l'écran de verrouillage (avec `span` : partout ;
    /// seule : au verrouillage seulement, le bureau gardant `fit`).
    pub span_lock: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DesktopRule {
    pub origin: Origin,
    pub sel: Selector,
    pub mode: Option<Mode>,
    /// `false` : les icônes et widgets déclarés pour tous les workspaces ne
    /// s'affichent pas sur ce bureau.
    pub inherit: bool,
}

/// Plus grand nombre de workspaces permanents réglable.
pub const MAX_WORKSPACES: u8 = 30;

/// Fenêtres visées par une affectation de workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMatch {
    /// Application installée (id `.desktop`) : sa classe de fenêtre est déduite.
    App(String),
    /// Expression régulière de classe, telle que Hyprland l'attend.
    Class(String),
}

/// `hd.assign` : les fenêtres d'une application s'ouvrent sur un workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignRule {
    pub origin: Origin,
    pub app: AppMatch,
    pub workspace: String,
    /// S'ouvrir là-bas sans y emmener l'utilisateur.
    pub silent: bool,
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
    pub origin: Origin,
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
    Clock {
        format: String,
    },
    Calendar,
    System {
        show: Vec<SysItem>,
    },
    /// `player` : sous-chaîne du nom MPRIS, `*` = le premier qui joue.
    Media {
        player: String,
    },
    Command {
        exec: String,
        timeout: Duration,
    },
    /// Fonctions Lua (index dans le registre) : `render`, et les facultatives
    /// `on_click`, `on_scroll` et `menu`.
    Lua {
        func: usize,
        click: Option<usize>,
        scroll: Option<usize>,
        menu: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetDef {
    pub origin: Origin,
    pub sel: Selector,
    pub id: String,
    /// Type au catalogue (`clock`, `system`, ou nom d'un plugin).
    pub type_id: String,
    pub kind: WidgetKind,
    pub anchor: Anchor,
    pub x: f32,
    pub y: f32,
    /// Taille du texte principal.
    pub size: f32,
    pub interval: Duration,
    /// Fond en carte (true) ou texte nu sur le fond d'écran (false).
    pub card: bool,
    /// Module de la barre : jamais affiché sur le bureau.
    pub in_bar: bool,
}

impl WidgetDef {
    pub fn placement(&self) -> Placement {
        Placement {
            anchor: self.anchor,
            x: self.x,
            y: self.y,
            size: self.size,
            card: self.card,
        }
    }

    pub fn set_placement(&mut self, p: &Placement) {
        self.anchor = p.anchor;
        self.x = p.x;
        self.y = p.y;
        self.size = p.size;
        self.card = p.card;
    }

    /// Même widget, en ignorant sa position : un déplacement ne doit pas
    /// relancer ses relevés.
    pub fn same_content(&self, o: &WidgetDef) -> bool {
        self.id == o.id
            && self.type_id == o.type_id
            && self.kind == o.kind
            && self.interval == o.interval
            && self.sel == o.sel
    }
}

/// Position d'un widget à l'écran (`hd.place`, ou glissé dans les réglages).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub anchor: Anchor,
    pub x: f32,
    pub y: f32,
    /// 0 = taille par défaut du widget.
    pub size: f32,
    pub card: bool,
}

/// D'où vient un type de widget.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetSource {
    Builtin,
    /// Fichier Lua détecté dans un dossier de plugins.
    Plugin(PathBuf),
    /// Plugin livré avec hyprdesk (réseau, Bluetooth…), remplaçable par un fichier du même nom.
    Bundled,
}

/// Entrée du catalogue des widgets disponibles.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetType {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: WidgetSource,
    pub anchor: Anchor,
    /// Activable d'un clic (sans option obligatoire).
    pub toggleable: bool,
    /// Plugin illisible : le message d'erreur, affiché dans les réglages.
    pub error: Option<String>,
    /// Options du plugin et leurs valeurs par défaut (« Configurer »).
    pub options: Vec<(String, crate::tiling::OptValue)>,
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
/// Paquets du cœur : toujours actifs.
pub const CORE_PLUGINS: &[&str] = &["desktop", "plugins", "effects", "theme"];

impl General {
    /// Placement effectif : celui déclaré ; si des écrans sont dédiés, tous les
    /// autres workspaces (1 à max(N, 10)) sont placés sur l'écran principal,
    /// pour qu'aucun n'arrive sur un écran dédié.
    /// Mode imposé à un écran affichant ce workspace : la règle du
    /// workspace d'abord, puis celle de l'écran.
    pub fn forced_mode(&self, monitor: &str, workspace: &str) -> Option<&str> {
        self.forced_modes
            .get(&format!("ws:{workspace}"))
            .or_else(|| self.forced_modes.get(monitor))
            .map(String::as_str)
    }

    pub fn expand_workspace_monitors(&mut self) {
        let mut map = self.workspace_monitors_declared.clone();
        let target = "primary";
        let dedicated_primary = self.workspace_dedicated.contains(target)
            || self
                .primary
                .as_ref()
                .is_some_and(|p| self.workspace_dedicated.contains(p));
        if !self.workspace_dedicated.is_empty() && !dedicated_primary {
            let upto = self.workspaces.unwrap_or(10).max(10) as usize;
            let placed: Vec<String> = map.values().flatten().cloned().collect();
            let rest: Vec<String> = (1..=upto)
                .map(|i| i.to_string())
                .filter(|w| !placed.contains(w))
                .collect();
            if !rest.is_empty() {
                let l = map.entry(target.to_string()).or_default();
                l.extend(rest);
                l.sort_by_key(|w| w.parse::<i64>().unwrap_or(i64::MAX));
                l.dedup();
            }
        }
        self.workspace_monitors = map;
    }
}

impl Config {
    /// Le paquet intégré `id` est-il actif ?
    pub fn plugin_on(&self, id: &str) -> bool {
        CORE_PLUGINS.contains(&id) || !self.general.disabled_plugins.contains(id)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub general: General,
    pub wallpapers: Vec<WallpaperRule>,
    pub desktops: Vec<DesktopRule>,
    pub icons: Vec<IconDef>,
    pub widgets: Vec<WidgetDef>,
    pub hooks: Vec<(HookEvent, usize)>,
    /// Clés de `hd.config` posées par l'écran Réglages, puis par le Lua de
    /// l'utilisateur (`show_on`, `grid.icon_size`, `theme.accent`…).
    pub keys_settings: std::collections::BTreeSet<String>,
    pub keys_user: std::collections::BTreeSet<String>,
    /// Types de widgets disponibles : intégrés, puis plugins détectés.
    pub catalog: Vec<WidgetType>,
    /// Positions imposées par l'écran Réglages (`hd.place`), par id de widget.
    pub placements: std::collections::BTreeMap<String, Placement>,
    /// Barre native, si déclarée.
    pub bar: Option<BarDef>,
    /// Composants de session (`hd.component`).
    pub components: Vec<ComponentDef>,
    /// Widgets désactivés depuis les réglages (`hd.disable`) : id → type.
    pub disabled: std::collections::BTreeMap<String, String>,
    /// Affectations d'applications à un workspace (`hd.assign`).
    pub assigns: Vec<AssignRule>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couleurs_hex_et_hyprland() {
        assert_eq!(Rgba::parse("#9580ff"), Some(Rgba(0x95, 0x80, 0xff, 0xff)));
        assert_eq!(
            Rgba::parse("rgba(454158aa)"),
            Some(Rgba(0x45, 0x41, 0x58, 0xaa))
        );
        assert_eq!(
            Rgba::parse("rgb(22212c)"),
            Some(Rgba(0x22, 0x21, 0x2c, 0xff))
        );
        assert_eq!(Rgba::parse("bleu"), None);
        assert_eq!(Rgba::parse("#12"), None);
    }

    #[test]
    fn selecteur_et_specificite() {
        let any = Selector::default();
        let ws = Selector {
            workspaces: Some(vec!["3".into()]),
            monitors: None,
        };
        let both = Selector {
            workspaces: Some(vec!["3".into()]),
            monitors: Some(vec!["DP-1".into()]),
        };
        assert!(any.matches("HDMI-A-2", "1"));
        assert!(ws.matches("HDMI-A-2", "3"));
        assert!(!ws.matches("HDMI-A-2", "4"));
        assert!(!both.matches("HDMI-A-2", "3"));
        assert!(any.specificity() < ws.specificity() && ws.specificity() < both.specificity());
    }

    #[test]
    fn elements_systeme() {
        assert_eq!(
            SysItem::parse("disk:/home"),
            Some(SysItem::Disk("/home".into()))
        );
        assert_eq!(SysItem::parse("cpu"), Some(SysItem::Cpu));
        assert_eq!(SysItem::parse("gpu"), None);
    }
}
