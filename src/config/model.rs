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

/// Ce qui donne le focus en plein bureau (`focus.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusMode {
    /// Celui de Hyprland (Réglages → Clavier et souris) : hyprdesk n'y
    /// touche pas.
    #[default]
    Hyprland,
    /// Le clic : la fenêtre cliquée prend le clavier et passe au premier
    /// plan ; le survol ne change rien.
    Click,
    /// La souris : la fenêtre survolée prend le clavier et passe au premier
    /// plan.
    Mouse,
}

impl FocusMode {
    pub fn as_str(self) -> &'static str {
        match self {
            FocusMode::Hyprland => "hyprland",
            FocusMode::Click => "click",
            FocusMode::Mouse => "mouse",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "hyprland" => FocusMode::Hyprland,
            "click" => FocusMode::Click,
            "mouse" => FocusMode::Mouse,
            _ => return None,
        })
    }
}

/// Où va le focus quand la fenêtre qui l'a se ferme (`focus.on_close`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusOnClose {
    /// La fenêtre suivante (Hyprland par défaut).
    #[default]
    Next,
    /// Celle sous le pointeur.
    Cursor,
    /// La plus récemment utilisée.
    Recent,
}

impl FocusOnClose {
    pub fn as_str(self) -> &'static str {
        match self {
            FocusOnClose::Next => "next",
            FocusOnClose::Cursor => "cursor",
            FocusOnClose::Recent => "recent",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "next" => FocusOnClose::Next,
            "cursor" => FocusOnClose::Cursor,
            "recent" => FocusOnClose::Recent,
            _ => return None,
        })
    }
}

/// Focus en plein bureau (`hd.config({ focus = { … } })`), posé tant qu'un
/// écran est en plein bureau (`desktop_focus`). Les autres réglages ne
/// comptent qu'avec un mode autre que `hyprland`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DesktopFocus {
    pub mode: FocusMode,
    /// Suivre la souris : distance parcourue (pixels logiques) avant de
    /// changer de fenêtre.
    pub threshold: u16,
    /// Au clic : le clic qui met une fenêtre au premier plan ne lui est pas
    /// transmis.
    pub swallow_click: bool,
    /// Une application peut prendre le focus, et passer devant, d'elle-même.
    pub on_activate: bool,
    pub on_close: FocusOnClose,
}

/// Portée des icônes ajoutées depuis Réglages ou déposées sur le bureau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconScope {
    /// Les mêmes icônes partout.
    #[default]
    Global,
    /// Chaque écran a les siennes.
    Screen,
    /// Chaque workspace a les siennes.
    Workspace,
}

impl IconScope {
    pub fn as_str(self) -> &'static str {
        match self {
            IconScope::Global => "global",
            IconScope::Screen => "screen",
            IconScope::Workspace => "workspace",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "global" => IconScope::Global,
            "screen" => IconScope::Screen,
            "workspace" => IconScope::Workspace,
            _ => return None,
        })
    }

    /// Filtre d'une icône des Réglages : la portée décide de la part du
    /// sélecteur qui compte (aucune, l'écran, le workspace).
    pub fn keeps(self, sel: &Selector, monitor: &str, workspace: &str) -> bool {
        let ok = |list: &Option<Vec<String>>, v: &str| {
            list.as_ref().is_none_or(|l| l.iter().any(|x| x == v))
        };
        match self {
            IconScope::Global => true,
            IconScope::Screen => ok(&sel.monitors, monitor),
            IconScope::Workspace => ok(&sel.workspaces, workspace),
        }
    }

    /// Sélecteur d'une icône ajoutée maintenant sur cet écran et ce workspace.
    pub fn selector(self, monitor: &str, workspace: &str) -> Selector {
        match self {
            IconScope::Global => Selector::default(),
            IconScope::Screen => Selector {
                monitors: Some(vec![monitor.to_string()]),
                workspaces: None,
            },
            IconScope::Workspace => Selector {
                workspaces: Some(vec![workspace.to_string()]),
                monitors: None,
            },
        }
    }
}

/// Style du sélecteur de workspaces dans la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WsSelector {
    /// Touches en relief : une touche par workspace, l'active enfoncée.
    Relief,
    /// Rail continu segmenté, pilule glissante sur l'actif.
    #[default]
    Rail,
    /// Minimap des fenêtres de chaque workspace.
    Miniatures,
    /// Icône de l'application au premier plan de chaque workspace.
    Icones,
    /// Numéros dans des capsules (MLG-279).
    Numeros,
    /// Points, l'actif allongé (MLG-279).
    Points,
}

impl WsSelector {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "relief" => WsSelector::Relief,
            "rail" => WsSelector::Rail,
            // Anciens noms (français) toujours lus.
            "thumbnails" | "miniatures" => WsSelector::Miniatures,
            "icons" | "icones" => WsSelector::Icones,
            "numbers" | "numeros" => WsSelector::Numeros,
            "dots" | "points" => WsSelector::Points,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            WsSelector::Relief => "relief",
            WsSelector::Rail => "rail",
            WsSelector::Miniatures => "thumbnails",
            WsSelector::Icones => "icons",
            WsSelector::Numeros => "numbers",
            WsSelector::Points => "dots",
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
    /// Plancher d'assombrissement du fond (0..1) : le voile monte seul jusqu'au
    /// contraste du texte (HIG ACC-02).
    pub dim: f32,
    /// Image propre au verrouillage ; sinon le fond du bureau de l'écran principal.
    pub image: Option<std::path::PathBuf>,
    pub show_user: bool,
    pub show_battery: bool,
    pub show_media: bool,
    /// Empreinte (fprintd) acceptée à la place du mot de passe, si un lecteur
    /// est là. Désactivée par défaut ; la politique du poste peut l'interdire.
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
            fingerprint: false,
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
    /// Écrans éteints à la main (`hyprdesk screens-off`) : un changement de
    /// volume les rallume.
    pub wake_volume: bool,
    /// Capot d'un portable refermé : sur batterie, sur secteur, et avec un
    /// écran externe allumé (`crate::lid`).
    pub lid_battery: LidAction,
    pub lid_ac: LidAction,
    pub lid_docked: LidAction,
}

impl Default for IdleCfg {
    fn default() -> Self {
        IdleCfg {
            // Verrouillage sur inactivité actif par défaut (5 min) ; un
            // hypridle lancé hors de hyprdesk reste maître (`idle::component`).
            manage: true,
            dim: 240,
            lock: 300,
            dpms: 360,
            suspend: 0,
            wake_volume: true,
            lid_battery: LidAction::System,
            lid_ac: LidAction::System,
            lid_docked: LidAction::System,
        }
    }
}

impl IdleCfg {
    /// hyprdesk tient le capot (au moins un cas n'est pas laissé au système).
    pub fn lid_managed(&self) -> bool {
        [self.lid_battery, self.lid_ac, self.lid_docked]
            .iter()
            .any(|a| *a != LidAction::System)
    }
}

/// Ce que fait le capot refermé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LidAction {
    /// Laissé à logind (HandleLidSwitch…), hyprdesk n'intervient pas.
    #[default]
    System,
    /// Verrouiller puis mettre en veille.
    Suspend,
    /// Verrouiller puis hiberner.
    Hibernate,
    /// Verrouiller seulement.
    Lock,
    /// Éteindre l'écran du portable (désactivé s'il y a un écran externe).
    ScreenOff,
    /// Rien.
    Nothing,
}

impl LidAction {
    pub const ALL: [LidAction; 6] = [
        LidAction::System,
        LidAction::Suspend,
        LidAction::Hibernate,
        LidAction::Lock,
        LidAction::ScreenOff,
        LidAction::Nothing,
    ];
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "system" => LidAction::System,
            "suspend" => LidAction::Suspend,
            "hibernate" => LidAction::Hibernate,
            "lock" => LidAction::Lock,
            "screen-off" => LidAction::ScreenOff,
            "nothing" => LidAction::Nothing,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            LidAction::System => "system",
            LidAction::Suspend => "suspend",
            LidAction::Hibernate => "hibernate",
            LidAction::Lock => "lock",
            LidAction::ScreenOff => "screen-off",
            LidAction::Nothing => "nothing",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LidAction::System => "Comme le système (logind)",
            LidAction::Suspend => "Mettre en veille",
            LidAction::Hibernate => "Hiberner",
            LidAction::Lock => "Verrouiller seulement",
            LidAction::ScreenOff => "Éteindre l'écran du portable",
            LidAction::Nothing => "Ne rien faire",
        }
    }
}

impl std::fmt::Display for LidAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(crate::i18n::tr(self.label()))
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
    /// SmartUI design : les icônes redessinées par l'IA et validées
    /// remplacent celles du thème (cache `smartui`).
    pub smartui: bool,
    /// Style demandé à l'IA ; vide : celui de hyprdesk ([`crate::smartui::DEFAULT_STYLE`]).
    pub smartui_style: String,
    /// Génération automatique (icônes validées d'office) ; `None` : coupée
    /// ([`crate::smartui::auto_on`]).
    pub smartui_auto: Option<bool>,
}

/// Présentation du lanceur d'applications (`hyprdesk apps`, `search`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppsStyle {
    /// Grande feuille en plein bureau, carte centrée dans les autres modes.
    Auto,
    /// Toujours la grande feuille (façon Launchpad, tiroir One UI).
    Sheet,
    /// Toujours la carte centrée (façon Spotlight) : le défaut (MLG-322),
    /// le même lanceur dans tous les modes.
    #[default]
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
            AppsStyle::Sheet => "sheet",
            AppsStyle::Spotlight => "spotlight",
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
    /// Forme du lanceur (MLG-278).
    pub shape: LauncherShape,
}

impl Default for AppsCfg {
    fn default() -> Self {
        AppsCfg {
            unified: true,
            style: AppsStyle::default(),
            suggestions: 8,
            categories: true,
            shape: LauncherShape::default(),
        }
    }
}

/// Ce que montre le lanceur avant la saisie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmptyView {
    /// Les applications les plus lancées.
    #[default]
    Suggestions,
    /// Les applications épinglées au dock.
    Favorites,
    /// Le champ seul.
    Field,
}

/// Effet d'ouverture du lanceur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpenEffect {
    /// Fondu avec léger déplacement (mouvement du thème).
    #[default]
    Fade,
    /// Agrandissement depuis le centre.
    Scale,
    /// Glissement depuis le bas.
    Slide,
    /// Ouverture immédiate.
    None,
}

/// Qualité des effets du verre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EffectsQuality {
    /// Flou et teinte, sans lentille ni reflet.
    Low,
    /// Lentille sans aberration.
    Medium,
    #[default]
    High,
}

/// Forme du lanceur (MLG-278) : dimensions, résultats, état initial,
/// arrière-plan, ouverture, sélection, fermeture, effets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LauncherShape {
    pub width: u16,
    /// Hauteur maximale, part de l'écran (0 : celle du lanceur).
    pub max_height: f32,
    pub radius: u16,
    pub padding: u16,
    pub columns: u8,
    pub row_height: u16,
    pub max_results: u8,
    pub empty_view: EmptyView,
    /// Voile sur le bureau derrière la carte (0..1), une fois et demie
    /// sous la feuille : sous 0,2, Hyprland ne floute plus ce qu'il recouvre
    /// (règle de calque `ignore_alpha = 0.2`, `crate::hypr`).
    pub backdrop_dim: f32,
    pub open_effect: OpenEffect,
    /// Durée d'ouverture (ms, 0 : celle du thème).
    pub open_duration: u16,
    /// Carte en haut de l'écran plutôt qu'au centre, à `offset` px.
    pub top: bool,
    pub offset: i16,
    pub selection_radius: u16,
    /// Opacités du filet et de l'ombre de la carte.
    pub border: f32,
    pub shadow: f32,
    pub close_on_outside_click: bool,
    pub quality: EffectsQuality,
}

impl Default for LauncherShape {
    fn default() -> Self {
        Self {
            width: 680,
            max_height: 0.0,
            radius: 28,
            padding: 22,
            columns: 7,
            row_height: 48,
            max_results: 8,
            empty_view: EmptyView::Suggestions,
            backdrop_dim: 0.30,
            open_effect: OpenEffect::Fade,
            open_duration: 0,
            top: false,
            offset: 0,
            selection_radius: 12,
            border: 1.0,
            shadow: 0.5,
            close_on_outside_click: true,
            quality: EffectsQuality::High,
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
            smartui: false,
            smartui_style: String::new(),
            smartui_auto: None,
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
    /// Surface de lecture stable sous le contenu (résultats, formulaires,
    /// graphiques) d'une surface de verre (MLG-282).
    Content,
    /// Le dock et ses bulles (MLG-336) ; sans réglage propre, celui du
    /// lanceur (moteur de thème).
    Dock,
    /// Le tiroir de LYRA (MLG-336) ; sans réglage propre, celui des
    /// Réglages (moteur de thème).
    Assistant,
}

impl Place {
    pub const ALL: [Place; 8] = [
        Place::Bar,
        Place::Menus,
        Place::Launcher,
        Place::Settings,
        Place::Widgets,
        Place::Content,
        Place::Dock,
        Place::Assistant,
    ];

    /// Surface dont celle-ci hérite quand un thème ne la règle pas : le
    /// dock a longtemps suivi le lanceur, LYRA s'ouvre à côté des Réglages.
    pub fn parent(self) -> Option<Place> {
        match self {
            Place::Dock => Some(Place::Launcher),
            Place::Assistant => Some(Place::Settings),
            _ => None,
        }
    }

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
            Place::Content => "content",
            Place::Dock => "dock",
            Place::Assistant => "assistant",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Place::Bar => "Barre",
            Place::Menus => crate::i18n::tr("Menus déroulants"),
            Place::Launcher => "Lanceur",
            Place::Settings => crate::i18n::tr("Réglages"),
            Place::Widgets => crate::i18n::tr("Widgets du bureau"),
            Place::Content => crate::i18n::tr("Contenu (résultats, formulaires)"),
            Place::Dock => "Dock",
            Place::Assistant => "LYRA",
        }
    }

    /// Effet livré : l'apparence d'origine de hyprdesk.
    /// Effets par défaut : ceux du pack Ardoise, le thème par défaut — le
    /// thème par défaut du démon est exactement ce pack du moteur (MLG-231).
    pub fn default_effect(self) -> Effect {
        let opacity = match self {
            Place::Bar => 0.72,
            Place::Menus => 0.90,
            Place::Launcher | Place::Dock => 0.88,
            Place::Settings | Place::Assistant => 0.97,
            Place::Widgets => 0.84,
            // Pas de surface de contenu sans thème : l'apparence d'origine.
            Place::Content => 0.0,
        };
        Effect {
            opacity,
            blur: self.blurrable(),
            // Barre et widgets sont posés sur le fond d'écran : ils se
            // densifient jusqu'à ce plafond quand le fond rendrait leur texte
            // illisible (HIG FON-01) ; le lanceur l'est toujours.
            adaptive: match self {
                Place::Bar | Place::Widgets => Some(0.95),
                // Le dock se densifie jusqu'à l'opaque sur un fond qui
                // gênerait la lecture (son plancher d'origine), réglable.
                Place::Dock => Some(1.0),
                _ => None,
            },
            material: None,
        }
    }

    /// Surface posée sur le fond d'écran, qui peut se densifier quand le
    /// fond gênerait la lecture (`Effect::adaptive`).
    pub fn adaptable(self) -> bool {
        matches!(self, Place::Bar | Place::Widgets | Place::Dock)
    }

    /// Les widgets sont dessinés sur la couche du bureau, avec le fond
    /// d'écran : pas de flou propre.
    pub fn blurrable(self) -> bool {
        !matches!(self, Place::Widgets | Place::Content)
    }

    /// Couches Hyprland de cet endroit.
    pub fn namespaces(self) -> &'static [&'static str] {
        match self {
            Place::Bar => &["hyprdesk-bar", "hyprdesk-convex-frame"],
            Place::Menus => &["hyprdesk-popover"],
            // Le dock est la couche « launcher » (son nom d'origine).
            Place::Launcher => &["hyprdesk-search"],
            Place::Dock => &["hyprdesk-launcher", "hyprdesk-launcher-popup"],
            Place::Settings => &["hyprdesk-settings", "hyprdesk-wizard"],
            Place::Assistant => &["hyprdesk-assistant"],
            Place::Widgets | Place::Content => &[],
        }
    }
}

/// Effet de surface : opacité du fond (0..1) et flou de ce qu'il y a derrière.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    pub opacity: f32,
    pub blur: bool,
    /// Transparence adaptative (MLG-261) : plafond d'opacité jusqu'où le
    /// verre se densifie quand le fond d'écran derrière rendrait le texte
    /// illisible (4,5:1) ; `None` : opacité fixe.
    pub adaptive: Option<f32>,
    /// Matériau choisi pour la surface (MLG-336) : son verre (reflet,
    /// réfraction, teinte) ; `None` : celui du thème.
    pub material: Option<Material>,
}

/// Matériau d'une surface (MLG-282, MLG-336), d'après les Human Interface
/// Guidelines : les préréglages du moteur de thème du même nom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Material {
    /// Le bureau reste visible : ni fond, ni flou, ni verre.
    Transparent,
    /// Verre très transparent, bord réfractif.
    GlassClear,
    /// Verre des commandes : flou, reflet, densifié sur un fond gênant.
    GlassRegular,
    /// Surface de lecture stable, presque opaque.
    ContentSurface,
}

impl Material {
    pub const ALL: [Material; 4] = [
        Material::Transparent,
        Material::GlassClear,
        Material::GlassRegular,
        Material::ContentSurface,
    ];

    /// Nom du préréglage du moteur de thème.
    pub fn as_str(self) -> &'static str {
        match self {
            Material::Transparent => "transparent",
            Material::GlassClear => "glass_clear",
            Material::GlassRegular => "glass_regular",
            Material::ContentSurface => "content_surface",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Material::ALL.into_iter().find(|m| m.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Material::Transparent => "Transparent",
            Material::GlassClear => crate::i18n::tr("Verre clair"),
            Material::GlassRegular => crate::i18n::tr("Verre"),
            Material::ContentSurface => crate::i18n::tr("Surface opaque"),
        }
    }

    /// Opacité, flou et densification du matériau, tels que le moteur les
    /// pose (`theme_engine::design::builtin_presets`) : la surface les prend
    /// quand on le choisit, l'utilisateur règle ensuite son opacité.
    pub fn effect(self) -> Effect {
        let (opacity, blur, adaptive) = match self {
            Material::Transparent => (0.0, false, None),
            Material::GlassClear => (0.35, true, None),
            Material::GlassRegular => (0.72, true, Some(0.94)),
            Material::ContentSurface => (0.94, true, None),
        };
        Effect {
            opacity,
            blur,
            adaptive,
            material: Some(self),
        }
    }

    /// Effet du matériau posé sur `place` : sans flou là où la surface n'en
    /// a pas, sans densification là où elle ne se densifie pas.
    pub fn effect_for(self, place: Place) -> Effect {
        let mut e = self.effect();
        e.blur &= place.blurrable();
        if !place.adaptable() {
            e.adaptive = None;
        }
        e
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

/// Rendu Lua : texte, icône, info-bulle, état et interrupteur confirmé facultatif.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rendered {
    pub text: String,
    /// Nom d'icône du thème (de préférence `…-symbolic`, teintée) ou chemin.
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub state: RenderState,
    /// État confirmé d'un interrupteur proposé dans le centre de contrôle.
    pub toggled: Option<bool>,
    /// Avancement d'une tâche (0 à 1) : une barre de progression dans le
    /// module (MLG-161).
    pub progress: Option<f32>,
    /// Compte posé en ovale sur le module, comme les non-lues des
    /// notifications (HIG SYS-02) : paquets à mettre à jour… `None` ou 0 :
    /// pas d'ovale.
    pub badge: Option<u32>,
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
    /// Sous-menu d'un niveau (HIG MENU-03) : chevron après le libellé, ouvert
    /// au survol. Ses entrées sont des `Item`, `Separator`, `Header`,
    /// `Image` ; un sous-menu imbriqué n'y est jamais lu.
    Submenu {
        label: String,
        icon: Option<String>,
        /// Faux : ligne grisée, sa colonne ne s'ouvre pas.
        enabled: bool,
        children: Vec<MenuEntry>,
    },
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
    /// Barre verticale au bord gauche (MLG-277) : `height` en est la
    /// largeur.
    Left,
    /// Barre verticale au bord droit.
    Right,
}

impl BarPosition {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "top" => BarPosition::Top,
            "bottom" => BarPosition::Bottom,
            "left" => BarPosition::Left,
            "right" => BarPosition::Right,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BarPosition::Top => "top",
            BarPosition::Bottom => "bottom",
            BarPosition::Left => "left",
            BarPosition::Right => "right",
        }
    }

    /// Barre verticale (bord gauche ou droit).
    pub fn vertical(self) -> bool {
        matches!(self, BarPosition::Left | BarPosition::Right)
    }
}

/// Module de la barre : propre à la barre, ou widget (par son id).
#[derive(Debug, Clone, PartialEq)]
pub enum BarItem {
    /// Menus du shell (HIG MENU-01) : « hyprdesk », « Fenêtres », « Espaces »,
    /// « Aide », en une rangée de titres. Un module comme les autres : il se
    /// place, se déplace ou se retire depuis Réglages → Barre.
    Menus,
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
    ("menus", "Menus"),
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
            "menus" => BarItem::Menus,
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
    /// Distance au bord de l'écran (0 = collée) ; la zone réservée aux
    /// fenêtres compte la hauteur et cette marge.
    pub margin: u16,
    /// Gauche, centre, droite.
    pub zones: [Vec<BarItem>; 3],
    /// Modules des écrans verticaux (portrait), s'ils diffèrent.
    pub portrait: Option<[Vec<BarItem>; 3]>,
    pub compact: BarCompact,
    /// Style visuel de la barre (`ui::plugins::bar::style`).
    pub style: BarStyle,
    /// Couleurs propres à la barre (sinon celles du thème).
    pub colors: BarColors,
    pub convex: ConvexDef,
    /// Forme, dimensions internes, visibilité, relations aux fenêtres
    /// (MLG-277).
    pub shape: BarShape,
}

/// Longueur de la barre le long de son bord.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BarLength {
    /// Tout le bord.
    #[default]
    Full,
    /// Pixels logiques.
    Fixed(u16),
    /// Part du bord (0..1).
    Percent(f32),
    /// Selon le contenu (mesuré au rendu).
    Content,
}

impl BarLength {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        Some(match s {
            "full" | "fill" => BarLength::Full,
            "content" | "auto" | "shrink" => BarLength::Content,
            _ if s.ends_with('%') => {
                let v: f32 = s
                    .trim_end_matches('%')
                    .trim()
                    .replace(',', ".")
                    .parse()
                    .ok()?;
                if !(5.0..=100.0).contains(&v) {
                    return None;
                }
                BarLength::Percent(v / 100.0)
            }
            _ => {
                let v: u16 = s.trim_end_matches("px").trim().parse().ok()?;
                if !(48..=8192).contains(&v) {
                    return None;
                }
                BarLength::Fixed(v)
            }
        })
    }

    pub fn as_text(self) -> String {
        match self {
            BarLength::Full => "full".into(),
            BarLength::Content => "content".into(),
            BarLength::Percent(p) => format!("{}%", (p * 100.0).round()),
            BarLength::Fixed(v) => v.to_string(),
        }
    }
}

/// Alignement d'une barre plus courte que son bord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarAlign {
    Start,
    #[default]
    Center,
    End,
}

/// Visibilité de la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarVisibility {
    #[default]
    Always,
    /// Masquée hors survol : une fine zone au bord la fait revenir.
    AutoHide,
}

/// Couche de la barre par rapport aux fenêtres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarLayer {
    /// Au-dessus des fenêtres, sous une fenêtre en plein écran.
    #[default]
    Top,
    /// Au-dessus de tout, plein écran compris.
    Overlay,
    /// Sous les fenêtres.
    Bottom,
}

/// Forme de la barre (MLG-277). `None` : valeur du style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarShape {
    pub length: BarLength,
    pub min_length: u16,
    pub max_length: u16,
    pub alignment: BarAlign,
    /// Marges sur les côtés (barre flottante).
    pub side_margin: u16,
    /// Groupes détachés plutôt qu'une lame continue (Continuité).
    pub groups: bool,
    pub radius: Option<u16>,
    pub padding: Option<u16>,
    pub item_gap: Option<u16>,
    pub icon_size: Option<u16>,
    pub font_size: Option<f32>,
    pub visibility: BarVisibility,
    pub show_delay: u16,
    pub hide_delay: u16,
    pub reveal_zone: u16,
    /// Les fenêtres laissent la place de la barre.
    pub reserve_space: bool,
    pub layer: BarLayer,
}

impl Default for BarShape {
    fn default() -> Self {
        Self {
            length: BarLength::Full,
            min_length: 0,
            max_length: 0,
            alignment: BarAlign::Center,
            side_margin: 0,
            groups: false,
            radius: None,
            padding: None,
            item_gap: None,
            icon_size: None,
            font_size: None,
            visibility: BarVisibility::Always,
            show_delay: 120,
            hide_delay: 600,
            reveal_zone: 4,
            reserve_space: true,
            layer: BarLayer::Top,
        }
    }
}

impl BarShape {
    /// Longueur en pixels sur un bord de `edge` pixels (`content` : largeur
    /// mesurée du contenu, si connue).
    pub fn length_on(&self, edge: f32, content: Option<f32>) -> f32 {
        let raw = match self.length {
            BarLength::Full => edge,
            BarLength::Fixed(v) => f32::from(v),
            BarLength::Percent(p) => edge * p,
            BarLength::Content => content.unwrap_or(edge),
        };
        let mut l = raw;
        if self.min_length > 0 {
            l = l.max(f32::from(self.min_length));
        }
        if self.max_length > 0 {
            l = l.min(f32::from(self.max_length));
        }
        l.min(edge - 2.0 * f32::from(self.side_margin)).max(48.0)
    }
}

/// Géométrie du cadre Convex, en pixels logiques ; mouvement en millisecondes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvexDef {
    pub frame_width: u16,
    pub radius: u16,
    pub duration: u16,
    pub attached_panels: bool,
    pub auto_hide: bool,
}

impl Default for ConvexDef {
    fn default() -> Self {
        Self {
            frame_width: 6,
            radius: 44,
            duration: 180,
            attached_panels: true,
            auto_hide: true,
        }
    }
}

/// Couleur réglée qui peut suivre le mode clair/sombre (MLG-263) : fixe,
/// slot de la palette du pack actif (`active`, qui change avec la variante),
/// ou paire de couleurs fixes, sombre puis claire (`#2dd4bf/#006b5f`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorRef {
    Fixed(Rgba),
    /// Slot de la palette résolue du pack (`accent`, `active`, `focus`…).
    Slot(String),
    Pair {
        dark: Rgba,
        light: Rgba,
    },
}

impl ColorRef {
    /// `#rrggbb` (ou `rgb(…)`), `<sombre>/<clair>`, ou nom de slot
    /// (`[a-z][a-z0-9_]*`).
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(c) = Rgba::parse(s) {
            return Some(Self::Fixed(c));
        }
        if let Some((d, l)) = s.split_once('/') {
            return Some(Self::Pair {
                dark: Rgba::parse(d)?,
                light: Rgba::parse(l)?,
            });
        }
        let mut chars = s.chars();
        let slot = chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        slot.then(|| Self::Slot(s.to_string()))
    }

    /// Forme texte, relisible par [`ColorRef::parse`].
    pub fn to_text(&self) -> String {
        match self {
            Self::Fixed(c) => c.to_hex(),
            Self::Slot(s) => s.clone(),
            Self::Pair { dark, light } => format!("{}/{}", dark.to_hex(), light.to_hex()),
        }
    }

    /// Couleur à afficher dans le mode donné ; un slot absent de la palette
    /// ne donne rien (la couleur du thème reprend la main).
    pub fn resolve(&self, light: bool, slot: &dyn Fn(&str) -> Option<Rgba>) -> Option<Rgba> {
        match self {
            Self::Fixed(c) => Some(*c),
            Self::Slot(s) => slot(s),
            Self::Pair { dark, light: l } => Some(if light { *l } else { *dark }),
        }
    }

    /// La couleur change-t-elle avec le mode ?
    pub fn follows_mode(&self) -> bool {
        !matches!(self, Self::Fixed(_))
    }
}

impl Rgba {
    /// `#rrggbb`, ou `#rrggbbaa` si la couleur n'est pas opaque.
    pub fn to_hex(self) -> String {
        if self.3 == 255 {
            format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.0, self.1, self.2, self.3)
        }
    }
}

/// Couleurs de la barre : `None` = celle du thème ; et une couleur par
/// module (`clock`, `workspaces`, `volume`…), pour son texte et ses icônes.
/// Chacune peut suivre le mode clair/sombre ([`ColorRef`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BarColors {
    pub text: Option<ColorRef>,
    pub background: Option<ColorRef>,
    pub accent: Option<ColorRef>,
    pub warning: Option<ColorRef>,
    pub critical: Option<ColorRef>,
    /// Opacité du fond (0 transparent … 1 opaque) ; sinon l'effet « Barre » du thème.
    pub opacity: Option<f32>,
    pub modules: std::collections::BTreeMap<String, ColorRef>,
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
    pub fn slot(&self, key: &str) -> Option<&ColorRef> {
        match key {
            "text" => self.text.as_ref(),
            "background" => self.background.as_ref(),
            "accent" => self.accent.as_ref(),
            "warning" => self.warning.as_ref(),
            "critical" => self.critical.as_ref(),
            _ => None,
        }
    }

    pub fn set_slot(&mut self, key: &str, c: Option<ColorRef>) {
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

/// Styles visuels de la barre : chaque style est un rendu complet (surface,
/// modules, workspaces) porté par `ui::plugins::bar::style`, avec ses jetons de mouvement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarStyle {
    /// Le style d'origine : verre fumé, capsules au survol, capsule mobile
    /// sur le workspace actif. Voir `design-plans/barre-continuite.md`.
    #[default]
    Continuite,
    /// Barre des tâches : lanceur centré et widgets à droite, bord à bord.
    Graphite,
    /// Capsule noire centrée, vide au repos, révélée au survol et occupée
    /// temporairement par les notifications.
    Island,
    /// Lame pleine largeur sculptée de trois niches reliées par des ponts
    /// fins. Voir `design-plans/barre-convexe.md`.
    Convexe,
}

/// Styles de barre réglables : (id Lua, libellé).
pub const BAR_STYLES: &[(BarStyle, &str, &str)] = &[
    (BarStyle::Continuite, "continuity", "Continuité"),
    (BarStyle::Graphite, "graphite", "Graphite"),
    (BarStyle::Island, "island", "Island"),
    (BarStyle::Convexe, "convex", "Convex"),
];

impl BarStyle {
    /// Style encore en cours de développement : signalé comme tel dans
    /// Réglages → Barre. Seul Continuité est terminé.
    pub fn in_development(self) -> bool {
        self != BarStyle::Continuite
    }

    pub fn parse(s: &str) -> Option<Self> {
        // Anciens noms (français) toujours lus.
        let s = match s {
            "continuite" => "continuity",
            "convexe" => "convex",
            s => s,
        };
        BAR_STYLES
            .iter()
            .find(|(_, id, _)| *id == s)
            .map(|(style, _, _)| *style)
    }
    pub fn as_str(self) -> &'static str {
        BAR_STYLES
            .iter()
            .find(|(style, _, _)| *style == self)
            .map_or("continuity", |&(_, id, _)| id)
    }
    /// Hauteur proposée quand `hd.bar` ne la fixe pas.
    pub fn default_height(self) -> u16 {
        match self {
            BarStyle::Continuite => 36,
            BarStyle::Graphite => 48,
            // Pilule flottante : plus fine que les lames.
            BarStyle::Island => 36,
            // 44 de silhouette + 4 d'air sous la lame.
            BarStyle::Convexe => 48,
        }
    }
    /// Bord proposé quand `hd.bar` ne le fixe pas.
    pub fn default_position(self) -> BarPosition {
        match self {
            BarStyle::Continuite => BarPosition::Top,
            BarStyle::Graphite => BarPosition::Bottom,
            BarStyle::Island => BarPosition::Top,
            BarStyle::Convexe => BarPosition::Top,
        }
    }
    /// Marge au bord proposée quand `hd.bar` ne la fixe pas (0 = collée).
    pub fn default_margin(self) -> u16 {
        match self {
            BarStyle::Continuite => 0,
            BarStyle::Graphite => 0,
            // Capsule détachée du bord, jour visible dessous.
            BarStyle::Island => 8,
            BarStyle::Convexe => 0,
        }
    }
    /// La composition proposée par le style (gauche, centre, droite) :
    /// choisir un style pose toute la barre — rendu, pose et modules. Une
    /// zone écrite explicitement dans `hd.bar` reste maîtresse ; Graphite
    /// conserve son lanceur dans la zone centrale.
    pub fn preset(self) -> [Vec<BarPreset>; 3] {
        match self {
            // Continuité, le style par défaut : la barre des menus de macOS.
            // À gauche, les menus du shell ; au centre, la fenêtre active et
            // les espaces de travail ; à droite, les icônes d'état, le centre
            // de contrôle et l'horloge en bout de barre. Session est dans le
            // menu hyprdesk, comme le menu Pomme. Les modules qui se cachent
            // d'eux-mêmes (clavier, éclairage nocturne, luminosité…) n'y sont
            // pas d'office : Réglages → Barre les ajoute.
            BarStyle::Continuite => [
                vec![sans_option("menus")],
                vec![sans_option("window"), sans_option("workspaces")],
                vec![
                    sans_option("tray"),
                    sans_option("media"),
                    sans_option("network"),
                    sans_option("bluetooth"),
                    sans_option("battery"),
                    sans_option("volume"),
                    sans_option("desktop"),
                    sans_option("notifications"),
                    sans_option("controls"),
                    BarPreset {
                        type_id: "clock",
                        options: &[("format", "%a %d %b  %H:%M")],
                    },
                ],
            ],
            // Island et Convexe gardent la pose d'origine : ils ne changent
            // que la silhouette (capsule centrale, lame à niches).
            BarStyle::Island | BarStyle::Convexe => [
                vec![
                    sans_option("menus"),
                    sans_option("workspaces"),
                    sans_option("window"),
                ],
                vec![BarPreset {
                    type_id: "clock",
                    options: if self == BarStyle::Convexe {
                        &[("format", "%H:%M")]
                    } else {
                        &[("format", "%a %d %b  %H:%M")]
                    },
                }],
                vec![
                    sans_option("tray"),
                    sans_option("media"),
                    sans_option("network"),
                    sans_option("bluetooth"),
                    sans_option("battery"),
                    sans_option("volume"),
                    sans_option("brightness"),
                    // Cachés d'eux-mêmes quand ils n'ont rien à montrer.
                    sans_option("keyboard"),
                    sans_option("nightlight"),
                    sans_option("removable"),
                    sans_option("capture"),
                    sans_option("updates"),
                    sans_option("notifications"),
                    // Accès au bureau vide, toujours en bout de barre.
                    sans_option("desktop"),
                    sans_option("session"),
                    sans_option("controls"),
                ],
            ],
            // Graphite : le plugin Lanceur au centre, les widgets à droite.
            BarStyle::Graphite => [
                vec![sans_option("menus")],
                vec![sans_option("launcher")],
                vec![
                    sans_option("tray"),
                    sans_option("network"),
                    sans_option("volume"),
                    sans_option("battery"),
                    BarPreset {
                        type_id: "clock",
                        options: &[("format", "%H:%M\n%d/%m/%Y")],
                    },
                ],
            ],
        }
    }
}

/// Un module du préréglage d'un style : son type et ses options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarPreset {
    pub type_id: &'static str,
    pub options: &'static [(&'static str, &'static str)],
}

/// Entrée de préréglage sans option.
fn sans_option(type_id: &'static str) -> BarPreset {
    BarPreset {
        type_id,
        options: &[],
    }
}

/// Barre compacte : titres raccourcis, heure seule, icônes sans libellé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarCompact {
    /// Selon la largeur mesurée des modules, et sur les écrans verticaux.
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
    /// Mode suivant : no-desktop → launcher-only → full-desktop → no-desktop.
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
    /// Ouvrir ou refermer LYRA, l'assistant IA (`hyprdesk assistant`).
    pub assistant: String,
    /// Le clavier va à la barre de l'écran actif (`hyprdesk focus bar`).
    pub focus_bar: String,
    /// Le clavier va au dock de l'écran actif (`hyprdesk focus dock`).
    pub focus_dock: String,
    /// Le centre de notifications s'ouvre et prend le clavier
    /// (`hyprdesk focus notifications`).
    pub focus_notifications: String,
    /// Le volet des Réglages s'ouvre s'il est fermé et prend le clavier
    /// (`hyprdesk focus settings`).
    pub focus_settings: String,
    /// Forcer une application à quitter (`hyprdesk force-quit`) : la boîte qui
    /// liste les applications et tue leurs fenêtres.
    pub force_quit: String,
    /// Basculer l'accès clavier complet (`hyprdesk keyboard-access`,
    /// `general.keyboard_access`, HIG ENT-01).
    pub keyboard_access: String,
    /// Raccourcis que hyprdesk prend à hyprland.lua : la combinaison y est
    /// déliée tant qu'elle figure ici, rendue (Hyprland relit sa config) sinon.
    pub take: Vec<String>,
    /// Touches multimédia (volume, luminosité, lecture) par hyprdesk : là où
    /// hyprland.lua ne les lie pas déjà, et toujours bureau affiché.
    pub media: bool,
    /// Raccourcis essentiels de fenêtres, de workspaces et d'écrans posés
    /// par hyprdesk là où hyprland.lua n'en a pas (`crate::essentials`).
    pub essentials: bool,
    /// Essentiels coupés (identifiants de `essentials::CATALOGUE`).
    pub essentials_off: Vec<String>,
    /// Combinaisons choisies pour des essentiels (`swap.left` →
    /// `SUPER + CTRL + h`) ; absentes : celles que hyprdesk déduit.
    pub essential_keys: std::collections::BTreeMap<String, String>,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            toggle: "SUPER + D".into(),
            launcher: "SUPER + ALT + D".into(),
            // Raccourcis de macOS (HIG ENT-06, PAT-14 ; décision D2) :
            // Commande-Tab, le commutateur ; Contrôle-Flèche haut, Mission
            // Control. Le lanceur et les Réglages gardent ceux de hyprdesk
            // (SUPER + R, SUPER + SHIFT + D).
            switch: "SUPER + Tab".into(),
            // Le lanceur de hyprdesk, dans tous les modes (apps_unified).
            search: "SUPER + R".into(),
            search_desktop: String::new(),
            overview: "CTRL + Up".into(),
            assistant: "SUPER + SHIFT + L".into(),
            // Un raccourci par surface du shell (HIG ENT-02), sur le modèle
            // de macOS : Control-F2, F3, F6 ; F8, libre, pour le centre.
            focus_bar: "CTRL + F2".into(),
            focus_dock: "CTRL + F3".into(),
            focus_notifications: "CTRL + F8".into(),
            focus_settings: "CTRL + F6".into(),
            // Forcer une application à quitter (HIG ENT-10), sur le modèle de
            // macOS : Option-Command-Esc. Les modificateurs dans l'ordre du dépôt.
            force_quit: "ALT + SUPER + Escape".into(),
            // Accès clavier complet (HIG ENT-01), sur le modèle de macOS :
            // Control-F1.
            keyboard_access: "CTRL + F1".into(),
            take: Vec::new(),
            hide: "SUPER + CTRL + D".into(),
            // SUPER + SHIFT + D, comme les autres raccourcis du bureau (SUPER + D
            // et ses variantes), plutôt que la virgule de macOS.
            settings: "SUPER + SHIFT + D".into(),
            // Vide : le verrouillage de hyprland.lua (SUPER + L, posé par
            // scripts/install.sh). Réglé, hyprdesk le remplace.
            lock: String::new(),
            leave: "SUPER + Escape".into(),
            media: true,
            essentials: true,
            essentials_off: Vec::new(),
            essential_keys: Default::default(),
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
    /// Index de fichiers (recherche `:f`) : racines à indexer. Vide :
    /// home et volumes montés (clés USB, partitions Windows…).
    pub index_paths: Vec<String>,
    /// Index de fichiers : noms de dossiers exclus, en plus des défauts
    /// (.git, node_modules, target…).
    pub index_exclude: Vec<String>,
    /// Index de fichiers : expressions régulières exclues, appliquées au
    /// chemin complet (ex. « \\.iso$ », « /Volumes/ »).
    pub index_exclude_regex: Vec<String>,
    /// Index de fichiers : extensions écartées (ex. « iso », « vdi »).
    pub index_exclude_ext: Vec<String>,
    /// Index de fichiers : catégories exclues (documents, images, vidéos…).
    pub index_exclude_cat: Vec<String>,
    /// Index de fichiers : taille maximale indexée en Mio (0 : illimité).
    pub index_max_size: u64,
    /// Index de fichiers : secondes entre deux rescans de sûreté (inotify
    /// peut rater des changements sur FUSE/NTFS).
    pub index_interval: u64,
    /// Index de fichiers : démon lancé avec hyprdesk (sinon : au premier
    /// `:f`). Il charge son snapshot ou fait son premier scan en arrière-plan.
    pub index_autostart: bool,
    /// Démarrage automatique XDG (`crate::autostart`), une fois par session.
    pub autostart: bool,
    /// Bureau type gestionnaire de fichiers : le dossier Bureau XDG en icônes (`crate::desktop_files`).
    pub desktop_files: bool,
    pub suspend_binds: bool,
    /// Fenêtres ouvertes depuis le bureau resté affiché : flottantes et centrées,
    /// le bureau autour d'elles.
    pub float_launched: bool,
    /// Focus en plein bureau : au clic ou en suivant la souris.
    pub desktop_focus: DesktopFocus,
    /// Barres de titre Hyprbars (plugin de Hyprland) en plein bureau seulement.
    pub hyprbars: bool,
    /// Chemin de `hyprbars.so` ; vide : cherché aux emplacements connus.
    pub hyprbars_path: String,
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
    /// Obsolète : lancer depuis le bureau ne change jamais de mode (clé lue,
    /// sans effet).
    pub close_on_launch: bool,
    /// Portée des icônes : globales, par écran ou par workspace.
    pub icon_scope: IconScope,
    /// Style du sélecteur de workspaces dans la barre.
    pub workspace_selector: WsSelector,
    /// Icônes ouvertes d'un double clic (un clic : sélection) ; sinon un clic.
    pub icon_double_click: bool,
    /// Transparence réduite (MLG-269) : surfaces presque opaques, sans
    /// reflet ni réfraction, quel que soit le thème.
    pub reduced_transparency: bool,
    /// Mouvement réduit (MLG-271) : ouvertures en fondu court, sans échelle
    /// ni déplacement.
    pub reduced_motion: bool,
    /// Accès clavier complet (HIG ENT-01) : Tab atteint chaque commande des
    /// Réglages, pas seulement les champs de saisie.
    pub keyboard_access: bool,
    /// Contraste renforcé (MLG-282) : texte à 7:1, repères à 4,5:1,
    /// surfaces plus denses et filets marqués.
    pub increase_contrast: bool,
    /// Échelle du texte de l'interface (MLG-282), 1 = celle du thème.
    pub text_scale: f32,
    /// Écran Réglages : tous les détails (sinon l'essentiel seulement).
    pub advanced_settings: bool,
    /// Visite guidée faite ou écartée : plus proposée à l'ouverture des Réglages.
    pub tour_seen: bool,
    /// Applications qui prennent le thème de hyprdesk (`kitty`…) ; par
    /// défaut toutes, vide : aucune.
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
    /// Écrans découpés en zones (MLG-172) : connecteur → découpage (`crate::zones`).
    pub screen_zones: std::collections::BTreeMap<String, crate::zones::Split>,
    /// Couleurs de chaque écran (Réglages → Couleurs des écrans).
    pub screen_colors: std::collections::BTreeMap<String, crate::colors::Adjust>,
    /// Panorama : écrans bandeau tactiles et leurs pages (MLG-164).
    pub panorama: crate::panorama::Config,
    /// Ouverture de l'assistant IA (LYRA) : fenêtre ou terminal.
    pub assistant_mode: crate::assistant::AssistantMode,
    /// Autres services IA : fournisseurs (endpoint, modèle) et fonction →
    /// fournisseur. Les clés API restent dans le trousseau.
    pub ai: crate::ai_services::AiCfg,
    /// Langue de l'interface : `auto` (celle du système), `fr` ou `en`.
    pub language: String,
    /// Moteur de rendu de l'interface (Réglages → Performance) : lu au
    /// démarrage (`crate::render::init`), changé à la relance suivante.
    pub renderer: crate::render::Renderer,
    /// Carte graphique préférée du rendu matériel (à la relance).
    pub gpu: crate::render::Gpu,
    /// Carte de rendu de Hyprland (adresse PCI, vide : son choix) ; posée
    /// par `env.lua` au démarrage de Hyprland (`crate::render_gpu`).
    pub hyprland_gpu: String,
    /// Fils d'exécution de l'interface (0 : un par cœur ; à la relance).
    pub worker_threads: u16,
    /// Layouts de Hyprland réglés dans hyprdesk (paquet « Layouts »).
    pub tiling: crate::tiling::Tiling,
    /// Animations et effets de Hyprland réglés dans hyprdesk (paquet « Animations »).
    pub fx: crate::fx::Fx,
    /// Autres options de Hyprland réglées dans hyprdesk (fenêtres, groupes,
    /// curseur, comportement…, `crate::hyprset`).
    pub hypr: crate::hyprset::Hypr,
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
            // Coupé par défaut : une copie peut être un mot de passe ou un code.
            clipboard_history: false,
            index_paths: Vec::new(),
            index_exclude: Vec::new(),
            index_exclude_regex: Vec::new(),
            index_exclude_ext: Vec::new(),
            index_exclude_cat: Vec::new(),
            index_max_size: 0,
            index_interval: crate::fileindex::DEFAULT_INTERVAL_SECS,
            index_autostart: true,
            autostart: false,
            desktop_files: false,
            float_launched: true,
            desktop_focus: DesktopFocus::default(),
            hyprbars: false,
            hyprbars_path: String::new(),
            mode: Mode::Opaque,
            fade_ms: 150,
            transition: Transition::Fade,
            crossfade_ms: 700,
            // Le dock montre les fenêtres du workspace, une entrée par application.
            running_apps: RunningApps::Workspace,
            launcher_group: LauncherGroup::App,
            launcher: LauncherCfg::default(),
            apps_launcher: AppsCfg::default(),
            lock: LockCfg::default(),
            idle: IdleCfg::default(),
            show_on: ShowOn::All,
            close_on_launch: true,
            icon_scope: IconScope::Global,
            workspace_selector: WsSelector::Rail,
            icon_double_click: true,
            reduced_transparency: false,
            reduced_motion: false,
            keyboard_access: true,
            increase_contrast: false,
            text_scale: 1.0,
            advanced_settings: false,
            tour_seen: false,
            // Thème des applications actif d'emblée (MLG-207) : chaque
            // application connue prend le thème de hyprdesk.
            app_themes: crate::apptheme::TARGETS
                .iter()
                .map(|t| t.id.to_string())
                .collect(),
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
            screen_zones: Default::default(),
            screen_colors: Default::default(),
            panorama: Default::default(),
            overview_command: String::new(),
            plugin_options: Default::default(),
            update_commands: Default::default(),
            workspace_dedicated: Default::default(),
            primary: None,
            apps: Default::default(),
            disabled_plugins: Default::default(),
            assistant_mode: Default::default(),
            ai: Default::default(),
            language: "auto".into(),
            renderer: Default::default(),
            gpu: Default::default(),
            hyprland_gpu: String::new(),
            worker_threads: crate::render::DEFAULT_WORKERS,
            tiling: Default::default(),
            fx: Default::default(),
            hypr: Default::default(),
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
    /// Présentation du plugin Lanceur, avec son état et ses actions partagés.
    Launcher,
    /// Widget composite : réglages fréquents, sources partagées avec les plugins.
    Controls,
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
    /// `on_click`, `on_scroll`, `menu` et `on_toggle`.
    Lua {
        func: usize,
        click: Option<usize>,
        scroll: Option<usize>,
        menu: Option<usize>,
        toggle: Option<usize>,
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
    /// Écran du widget, et que faire si cet écran est débranché.
    pub screen: WidgetScreen,
    pub if_missing: IfMissing,
    /// Taille du widget, couches d'information comprises (HIG SYS-15).
    pub family: WidgetFamily,
}

impl WidgetKind {
    /// Le widget a plusieurs tailles (`family`, HIG SYS-15).
    pub fn has_family(&self) -> bool {
        matches!(
            self,
            WidgetKind::Clock { .. }
                | WidgetKind::Calendar
                | WidgetKind::System { .. }
                | WidgetKind::Media { .. }
        )
    }
}

impl WidgetDef {
    pub fn placement(&self) -> Placement {
        Placement {
            anchor: self.anchor,
            x: self.x,
            y: self.y,
            size: self.size,
            card: self.card,
            screen: self.screen.clone(),
            if_missing: self.if_missing,
            family: self.family,
        }
    }

    /// Le widget s'affiche-t-il sur cet écran ? Un écran nommé par le
    /// sélecteur Lua (`monitor = …`) décide seul ; sinon l'écran du widget
    /// (principal par défaut), et, s'il est débranché, l'écran principal ou rien.
    pub fn on_screen(&self, monitor: &str, primary: Option<&str>, connected: &[&str]) -> bool {
        if self.sel.monitors.is_some() {
            return true;
        }
        let primary_here = primary.is_none_or(|p| p == monitor);
        match &self.screen {
            WidgetScreen::All => true,
            WidgetScreen::Primary => primary_here,
            WidgetScreen::Named(n) if connected.contains(&n.as_str()) => n == monitor,
            WidgetScreen::Named(_) => self.if_missing == IfMissing::Primary && primary_here,
        }
    }

    pub fn set_placement(&mut self, p: &Placement) {
        self.anchor = p.anchor;
        self.x = p.x;
        self.y = p.y;
        self.size = p.size;
        self.card = p.card;
        self.screen = p.screen.clone();
        self.if_missing = p.if_missing;
        self.family = p.family;
    }

    /// Même widget, en ignorant sa position : un déplacement ne doit pas
    /// relancer ses relevés.
    pub fn same_content(&self, o: &WidgetDef) -> bool {
        self.id == o.id
            && self.type_id == o.type_id
            && self.kind == o.kind
            && self.interval == o.interval
            && self.sel == o.sel
            // La taille change ce que le widget relève (HIG SYS-15).
            && self.family == o.family
    }
}

/// Position d'un widget à l'écran (`hd.place`, ou glissé dans les réglages).
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub anchor: Anchor,
    pub x: f32,
    pub y: f32,
    /// 0 = taille par défaut du widget.
    pub size: f32,
    pub card: bool,
    pub screen: WidgetScreen,
    pub if_missing: IfMissing,
    /// Taille du widget (HIG SYS-15).
    pub family: WidgetFamily,
}

/// Écran d'un widget du bureau (plusieurs écrans : un widget n'est pas
/// répété sur chacun).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WidgetScreen {
    /// L'écran principal (défaut).
    #[default]
    Primary,
    /// Tous les écrans.
    All,
    /// Un écran par son nom (`DP-1`).
    Named(String),
}

impl WidgetScreen {
    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "" | "primary" | "principal" => WidgetScreen::Primary,
            "all" | "*" | "tous" => WidgetScreen::All,
            name => WidgetScreen::Named(name.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            WidgetScreen::Primary => "primary",
            WidgetScreen::All => "all",
            WidgetScreen::Named(n) => n,
        }
    }
}

/// Taille d'un widget du bureau : chaque taille ajoute une couche
/// d'information au lieu d'agrandir la précédente ; `Medium` est l'affichage
/// historique (HIG SYS-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidgetFamily {
    Small,
    #[default]
    Medium,
    Large,
    XLarge,
}

impl WidgetFamily {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "small" | "petit" => WidgetFamily::Small,
            "medium" | "moyen" => WidgetFamily::Medium,
            "large" | "grand" => WidgetFamily::Large,
            "xlarge" | "tres-grand" | "très-grand" => WidgetFamily::XLarge,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            WidgetFamily::Small => "small",
            WidgetFamily::Medium => "medium",
            WidgetFamily::Large => "large",
            WidgetFamily::XLarge => "xlarge",
        }
    }
}

/// Écran du widget débranché : le widget se montre sur l'écran principal, ou se masque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IfMissing {
    #[default]
    Primary,
    Hide,
}

impl IfMissing {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "primary" | "principal" => IfMissing::Primary,
            "hide" | "masquer" => IfMissing::Hide,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            IfMissing::Primary => "primary",
            IfMissing::Hide => "hide",
        }
    }
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
/// Paquets du cœur : toujours actifs. Le lanceur d'applications en est : il
/// reste ouvrable (raccourci, `hyprdesk apps`) même dock coupé.
pub const CORE_PLUGINS: &[&str] = &["desktop", "plugins", "modules", "theme", "launcher"];

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
    fn styles_de_barre() {
        // Aller-retour par l'id Lua, défauts de géométrie par style.
        assert_eq!(BarStyle::parse("continuity"), Some(BarStyle::Continuite));
        assert_eq!(BarStyle::parse("continuite"), Some(BarStyle::Continuite));
        assert_eq!(BarStyle::parse("convexe"), Some(BarStyle::Convexe));
        assert_eq!(BarStyle::parse("graphite"), Some(BarStyle::Graphite));
        assert_eq!(BarStyle::parse("pastel"), None);
        assert_eq!(BarStyle::Continuite.as_str(), "continuity");
        assert_eq!(BarStyle::Graphite.as_str(), "graphite");
        assert_eq!(BarStyle::default(), BarStyle::Continuite);
        assert_eq!(
            (
                BarStyle::Continuite.default_position(),
                BarStyle::Continuite.default_height(),
                BarStyle::Continuite.default_margin()
            ),
            (BarPosition::Top, 36, 0)
        );
        assert_eq!(
            (
                BarStyle::Graphite.default_position(),
                BarStyle::Graphite.default_height(),
                BarStyle::Graphite.default_margin()
            ),
            (BarPosition::Bottom, 48, 0)
        );
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

    #[test]
    fn barre_et_widgets_se_densifient_sur_le_fond_par_defaut() {
        // HIG FON-01 : les surfaces posées sur le fond d'écran s'adaptent
        // par défaut ; celles qui passent devant les fenêtres gardent une
        // opacité fixe.
        for p in [Place::Bar, Place::Widgets] {
            assert_eq!(p.default_effect().adaptive, Some(0.95));
        }
        for p in [Place::Menus, Place::Launcher, Place::Settings] {
            assert_eq!(p.default_effect().adaptive, None);
        }
        let theme = Theme::default();
        assert_eq!(theme.effect(Place::Bar).adaptive, Some(0.95));
        assert_eq!(theme.effect(Place::Widgets).adaptive, Some(0.95));
    }
}
