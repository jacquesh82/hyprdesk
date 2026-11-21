//! Options de Hyprland réglées dans hyprdesk hors effets et layouts
//! (MLG-137, MLG-141 à MLG-144) : gaps et bordures, groupes, curseur,
//! comportement (`misc`), raccourcis, XWayland, rendu, écosystème, entrées
//! avancées et gestes.
//!
//! Même modèle que `crate::fx` et `crate::tiling` : un registre d'options
//! (`tiling::Opt`/`Kind`), un stockage creux (`Hypr`, seules les valeurs
//! réglées ici), un rendu Lua imbriqué pour `hl.config`. Chaque option sait
//! dans quel fichier généré elle va (`Domain`).

use std::collections::BTreeMap;

use crate::tiling::{Kind, Opt, OptValue};

/// Fichier généré par domaine (voir `crate::genfiles`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Domain {
    /// `appearance.lua` : gaps, bordures, décoration, groupes.
    Appearance,
    /// `inputs.lua` : entrées avancées, gestes.
    Inputs,
    /// `system.lua` : comportement, curseur, raccourcis, XWayland, rendu…
    System,
}

macro_rules! o {
    ($section:literal, $key:literal, $label:literal, $kind:expr) => {
        Opt {
            section: $section,
            key: $key,
            label: $label,
            kind: $kind,
        }
    };
}

const fn num(min: f64, max: f64, step: f64) -> Kind {
    Kind::Num { min, max, step }
}

const HW_CURSOR: &[(i64, &str)] = &[(0, "Matériel"), (1, "Logiciel"), (2, "Automatique")];
const WARP_WS: &[(i64, &str)] = &[(0, "Jamais"), (1, "Toujours"), (2, "Sauf au clavier")];
const DEFAULT_WALLPAPER: &[(i64, &str)] = &[
    (-1, "Au hasard"),
    (0, "Aucun"),
    (1, "Anime 1"),
    (2, "Anime 2"),
];
const FULLSCREEN_TAKEOVER: &[(i64, &str)] = &[
    (0, "Reste derrière"),
    (1, "Prend le plein écran"),
    (2, "Sort du plein écran"),
];
const VRR: &[(i64, &str)] = &[(0, "Coupé"), (1, "Toujours"), (2, "En plein écran")];
const INITIAL_WS: &[(i64, &str)] = &[(0, "Non"), (1, "Seul workspace"), (2, "Partout")];
const SCROLL_METHOD: &[(&str, &str)] = &[
    ("2fg", "Deux doigts"),
    ("edge", "Bord du pavé"),
    ("on_button_down", "Bouton maintenu"),
    ("no_scroll", "Aucun"),
];
const DRAG_INTO_GROUP: &[(i64, &str)] = &[
    (0, "Jamais"),
    (1, "Toujours"),
    (2, "Sur la barre du groupe"),
];

/// Options du registre (aucune n'est dans `fx::OPTIONS` ni `tiling::OPTIONS`).
pub const OPTIONS: &[Opt] = &[
    // — Fenêtres : apparence —
    o!(
        "general",
        "gaps_in",
        "Espace entre les fenêtres",
        Kind::Sides { max: 60 }
    ),
    o!(
        "general",
        "gaps_out",
        "Marge autour des fenêtres",
        Kind::Sides { max: 200 }
    ),
    o!(
        "general",
        "gaps_workspaces",
        "Espace entre workspaces (défilement)",
        num(0.0, 200.0, 1.0)
    ),
    o!(
        "general",
        "border_size",
        "Épaisseur des bordures",
        num(0.0, 20.0, 1.0)
    ),
    o!(
        "general",
        "col.active_border",
        "Bordure de la fenêtre active",
        Kind::Gradient
    ),
    o!(
        "general",
        "col.inactive_border",
        "Bordure des autres fenêtres",
        Kind::Gradient
    ),
    o!(
        "general",
        "col.nogroup_border",
        "Bordure : fenêtre hors groupe",
        Kind::Gradient
    ),
    o!(
        "general",
        "col.nogroup_border_active",
        "Bordure active : fenêtre hors groupe",
        Kind::Gradient
    ),
    o!(
        "general",
        "resize_on_border",
        "Redimensionner en tirant la bordure",
        Kind::Bool
    ),
    o!(
        "general",
        "extend_border_grab_area",
        "Zone de prise autour de la bordure",
        num(0.0, 40.0, 1.0)
    ),
    o!(
        "general",
        "hover_icon_on_border",
        "Curseur de redimensionnement sur la bordure",
        Kind::Bool
    ),
    o!(
        "general",
        "allow_tearing",
        "Autoriser le tearing (jeux)",
        Kind::Bool
    ),
    o!(
        "general",
        "snap.enabled",
        "Aimanter les fenêtres flottantes",
        Kind::Bool
    ),
    o!(
        "decoration",
        "border_part_of_window",
        "Bordure comptée dans la fenêtre",
        Kind::Bool
    ),
    o!(
        "decoration",
        "shadow.color",
        "Couleur de l'ombre",
        Kind::Color
    ),
    o!(
        "decoration",
        "blur.ignore_opacity",
        "Flou à travers les fenêtres transparentes",
        Kind::Bool
    ),
    o!(
        "decoration",
        "blur.proportional",
        "Flou proportionnel à l'opacité (mélange par shader)",
        Kind::Bool
    ),
    // — Groupes —
    o!(
        "group",
        "auto_group",
        "Grouper les nouvelles fenêtres d'un groupe actif",
        Kind::Bool
    ),
    o!(
        "group",
        "insert_after_current",
        "Insérer après l'onglet actif",
        Kind::Bool
    ),
    o!(
        "group",
        "focus_removed_window",
        "Activer la fenêtre sortie du groupe",
        Kind::Bool
    ),
    o!(
        "group",
        "drag_into_group",
        "Glisser une fenêtre dans un groupe",
        Kind::IntChoice(DRAG_INTO_GROUP)
    ),
    o!(
        "group",
        "merge_groups_on_drag",
        "Fusionner les groupes au glisser",
        Kind::Bool
    ),
    o!(
        "group",
        "col.border_active",
        "Bordure du groupe actif",
        Kind::Gradient
    ),
    o!(
        "group",
        "col.border_inactive",
        "Bordure des autres groupes",
        Kind::Gradient
    ),
    o!(
        "group",
        "col.border_locked_active",
        "Bordure du groupe verrouillé actif",
        Kind::Gradient
    ),
    o!(
        "group",
        "col.border_locked_inactive",
        "Bordure des groupes verrouillés",
        Kind::Gradient
    ),
    o!(
        "group",
        "groupbar.enabled",
        "Barre d'onglets du groupe",
        Kind::Bool
    ),
    o!(
        "group",
        "groupbar.disable_when_only",
        "Masquer la barre d'un groupe d'une seule fenêtre",
        Kind::Bool
    ),
    o!(
        "group",
        "groupbar.font_size",
        "Taille du texte des onglets",
        num(6.0, 32.0, 1.0)
    ),
    o!(
        "group",
        "groupbar.height",
        "Hauteur de la barre d'onglets",
        num(4.0, 48.0, 1.0)
    ),
    o!(
        "group",
        "groupbar.rounding",
        "Arrondi des onglets",
        num(0.0, 20.0, 1.0)
    ),
    o!(
        "group",
        "groupbar.gradients",
        "Onglets en dégradé",
        Kind::Bool
    ),
    o!(
        "group",
        "groupbar.render_titles",
        "Titres dans les onglets",
        Kind::Bool
    ),
    o!(
        "group",
        "groupbar.scrolling",
        "Changer d'onglet à la molette",
        Kind::Bool
    ),
    o!(
        "group",
        "groupbar.text_color",
        "Couleur du texte des onglets",
        Kind::Color
    ),
    o!(
        "group",
        "groupbar.col.active",
        "Onglet actif",
        Kind::Gradient
    ),
    o!(
        "group",
        "groupbar.col.inactive",
        "Autres onglets",
        Kind::Gradient
    ),
    o!(
        "group",
        "groupbar.col.locked_active",
        "Onglet actif verrouillé",
        Kind::Gradient
    ),
    o!(
        "group",
        "groupbar.col.locked_inactive",
        "Onglets verrouillés",
        Kind::Gradient
    ),
    // — Curseur —
    o!(
        "cursor",
        "no_hardware_cursors",
        "Rendu du curseur",
        Kind::IntChoice(HW_CURSOR)
    ),
    o!(
        "cursor",
        "inactive_timeout",
        "Masquer le curseur immobile (secondes, 0 : jamais)",
        num(0.0, 60.0, 1.0)
    ),
    o!(
        "cursor",
        "hide_on_key_press",
        "Masquer le curseur à la frappe",
        Kind::Bool
    ),
    o!(
        "cursor",
        "hide_on_touch",
        "Masquer le curseur au toucher",
        Kind::Bool
    ),
    o!(
        "cursor",
        "no_warps",
        "Ne jamais déplacer le curseur",
        Kind::Bool
    ),
    o!(
        "cursor",
        "persistent_warps",
        "Revenir à la dernière position dans la fenêtre",
        Kind::Bool
    ),
    o!(
        "cursor",
        "warp_on_monitor_change",
        "Curseur au centre de l'écran activé au clavier",
        Kind::Bool
    ),
    o!(
        "cursor",
        "warp_on_change_workspace",
        "Curseur sur la fenêtre active au changement de workspace",
        Kind::IntChoice(WARP_WS)
    ),
    o!(
        "cursor",
        "zoom_factor",
        "Loupe autour du curseur",
        num(1.0, 5.0, 0.1)
    ),
    o!(
        "cursor",
        "enable_hyprcursor",
        "Thèmes hyprcursor",
        Kind::Bool
    ),
    // — Comportement —
    o!(
        "misc",
        "disable_hyprland_logo",
        "Masquer le logo de Hyprland",
        Kind::Bool
    ),
    o!(
        "misc",
        "disable_splash_rendering",
        "Masquer le texte d'accueil",
        Kind::Bool
    ),
    o!(
        "misc",
        "force_default_wallpaper",
        "Fond par défaut de Hyprland",
        Kind::IntChoice(DEFAULT_WALLPAPER)
    ),
    o!(
        "misc",
        "background_color",
        "Couleur de fond sous le bureau",
        Kind::Color
    ),
    o!(
        "misc",
        "mouse_move_enables_dpms",
        "La souris rallume les écrans",
        Kind::Bool
    ),
    o!(
        "misc",
        "key_press_enables_dpms",
        "Le clavier rallume les écrans",
        Kind::Bool
    ),
    o!(
        "misc",
        "mouse_move_focuses_monitor",
        "La souris change d'écran actif",
        Kind::Bool
    ),
    o!(
        "misc",
        "focus_on_activate",
        "Activer une fenêtre qui le demande",
        Kind::Bool
    ),
    o!(
        "misc",
        "enable_swallow",
        "Une application lancée depuis un terminal le remplace",
        Kind::Bool
    ),
    o!(
        "misc",
        "swallow_regex",
        "Terminaux concernés (classe, regex)",
        Kind::Text
    ),
    o!(
        "misc",
        "swallow_exception_regex",
        "Exceptions (titre, regex)",
        Kind::Text
    ),
    o!(
        "misc",
        "allow_session_lock_restore",
        "Relancer un verrouillage interrompu",
        Kind::Bool
    ),
    o!(
        "misc",
        "close_special_on_empty",
        "Fermer un workspace spécial vide",
        Kind::Bool
    ),
    o!(
        "misc",
        "on_focus_under_fullscreen",
        "Fenêtre activée sous une fenêtre plein écran",
        Kind::IntChoice(FULLSCREEN_TAKEOVER)
    ),
    o!(
        "misc",
        "exit_window_retains_fullscreen",
        "Le plein écran passe à la fenêtre suivante à la fermeture",
        Kind::Bool
    ),
    o!(
        "misc",
        "middle_click_paste",
        "Coller au clic du milieu",
        Kind::Bool
    ),
    o!(
        "misc",
        "vrr",
        "Fréquence variable (tous les écrans)",
        Kind::IntChoice(VRR)
    ),
    o!(
        "misc",
        "initial_workspace_tracking",
        "Ouvrir une application sur son workspace de lancement",
        Kind::IntChoice(INITIAL_WS)
    ),
    o!(
        "misc",
        "layers_hog_keyboard_focus",
        "Les surfaces gardent le clavier",
        Kind::Bool
    ),
    o!(
        "misc",
        "disable_autoreload",
        "Ne pas relire la config à chaque enregistrement",
        Kind::Bool
    ),
    // — Raccourcis —
    o!(
        "binds",
        "workspace_back_and_forth",
        "Rappeler un workspace ramène au précédent",
        Kind::Bool
    ),
    o!(
        "binds",
        "allow_workspace_cycles",
        "Alterner entre les deux derniers workspaces",
        Kind::Bool
    ),
    o!(
        "binds",
        "scroll_event_delay",
        "Délai entre deux crans de molette (ms)",
        num(0.0, 1000.0, 10.0)
    ),
    o!(
        "binds",
        "movefocus_cycles_fullscreen",
        "Le focus passe aussi les fenêtres plein écran",
        Kind::Bool
    ),
    o!(
        "binds",
        "hide_special_on_workspace_change",
        "Fermer le workspace spécial en changeant de workspace",
        Kind::Bool
    ),
    // — XWayland —
    o!(
        "xwayland",
        "enabled",
        "XWayland (applications X11)",
        Kind::Bool
    ),
    o!(
        "xwayland",
        "use_nearest_neighbor",
        "Applications X11 nettes à l'échelle (plus pixelisées)",
        Kind::Bool
    ),
    o!(
        "xwayland",
        "force_zero_scaling",
        "Applications X11 sans mise à l'échelle",
        Kind::Bool
    ),
    // — Rendu —
    o!(
        "render",
        "use_shader_blur_blend",
        "Mélange du flou par shader",
        Kind::Bool
    ),
    o!(
        "render",
        "expand_undersized_textures",
        "Étirer les textures trop petites",
        Kind::Bool
    ),
    // — Écosystème —
    o!(
        "ecosystem",
        "no_update_news",
        "Pas d'annonce de mise à jour",
        Kind::Bool
    ),
    o!(
        "ecosystem",
        "no_donation_nag",
        "Pas d'appel aux dons",
        Kind::Bool
    ),
    o!(
        "ecosystem",
        "enforce_permissions",
        "Appliquer les permissions",
        Kind::Bool
    ),
    // — Débogage —
    o!(
        "debug",
        "disable_logs",
        "Couper les journaux de Hyprland",
        Kind::Bool
    ),
    o!(
        "debug",
        "overlay",
        "Afficher les mesures de rendu",
        Kind::Bool
    ),
    // — Entrées avancées —
    o!(
        "input",
        "force_no_accel",
        "Sans accélération (ignore le profil)",
        Kind::Bool
    ),
    o!(
        "input",
        "scroll_method",
        "Méthode de défilement",
        Kind::Choice(SCROLL_METHOD)
    ),
    o!(
        "input",
        "scroll_factor",
        "Vitesse de la molette",
        num(0.1, 5.0, 0.1)
    ),
    o!(
        "input",
        "special_fallthrough",
        "Clic à travers un workspace spécial vide",
        Kind::Bool
    ),
    o!(
        "input",
        "mouse_refocus",
        "Recentrer le focus sous la souris",
        Kind::Bool
    ),
    o!(
        "input",
        "touchpad.middle_button_emulation",
        "Clic du milieu à deux boutons",
        Kind::Bool
    ),
    o!(
        "input",
        "touchpad.drag_lock",
        "Glisser sans maintenir",
        Kind::Bool
    ),
    o!(
        "input",
        "touchpad.tap-and-drag",
        "Toucher puis glisser",
        Kind::Bool
    ),
    // — Gestes —
    o!(
        "gestures",
        "workspace_swipe_distance",
        "Distance d'un glissement de workspace (px)",
        num(50.0, 1500.0, 10.0)
    ),
    o!(
        "gestures",
        "workspace_swipe_invert",
        "Glissement inversé",
        Kind::Bool
    ),
    o!(
        "gestures",
        "workspace_swipe_forever",
        "Glisser plusieurs workspaces d'affilée",
        Kind::Bool
    ),
    o!(
        "gestures",
        "workspace_swipe_cancel_ratio",
        "Seuil d'annulation du glissement",
        num(0.0, 1.0, 0.05)
    ),
];

/// Groupes d'affichage : (clé de placement, titre de section, chemins).
/// La clé dit où le groupe s'affiche : un onglet qui lui est propre
/// (`fenetres`, `comportement-fenetres`, `groupes`, `curseur`, `gestes`,
/// `avance`), ou une section avancée ajoutée sous une page existante
/// (`ui::plugins::ADVANCED_PLACES` : souris, pavé, workspaces, raccourcis,
/// veille, fond, flou) — là où on cherche le réglage.
pub const GROUPS: &[(&str, &str, &[&str])] = &[
    (
        "fenetres",
        "Espacement",
        &["general.gaps_in", "general.gaps_out"],
    ),
    (
        "fenetres",
        "Bordures",
        &[
            "general.border_size",
            "general.col.active_border",
            "general.col.inactive_border",
        ],
    ),
    (
        "fenetres-details",
        "Bordures et espacement : détails",
        &[
            "general.gaps_workspaces",
            "general.col.nogroup_border",
            "general.col.nogroup_border_active",
            "general.resize_on_border",
            "general.extend_border_grab_area",
            "general.hover_icon_on_border",
            "decoration.border_part_of_window",
            "decoration.shadow.color",
        ],
    ),
    (
        "comportement-fenetres",
        "Focus et plein écran",
        &[
            "misc.focus_on_activate",
            "misc.mouse_move_focuses_monitor",
            "misc.on_focus_under_fullscreen",
            "misc.exit_window_retains_fullscreen",
            "misc.initial_workspace_tracking",
            "misc.layers_hog_keyboard_focus",
            "general.snap.enabled",
            "general.allow_tearing",
        ],
    ),
    (
        "comportement-fenetres",
        "Terminal avalé",
        &[
            "misc.enable_swallow",
            "misc.swallow_regex",
            "misc.swallow_exception_regex",
        ],
    ),
    (
        "groupes",
        "Groupes",
        &[
            "group.auto_group",
            "group.insert_after_current",
            "group.focus_removed_window",
            "group.drag_into_group",
            "group.merge_groups_on_drag",
            "group.col.border_active",
            "group.col.border_inactive",
            "group.col.border_locked_active",
            "group.col.border_locked_inactive",
        ],
    ),
    (
        "groupes",
        "Barre d'onglets",
        &[
            "group.groupbar.enabled",
            "group.groupbar.disable_when_only",
            "group.groupbar.font_size",
            "group.groupbar.height",
            "group.groupbar.rounding",
            "group.groupbar.gradients",
            "group.groupbar.render_titles",
            "group.groupbar.scrolling",
            "group.groupbar.text_color",
            "group.groupbar.col.active",
            "group.groupbar.col.inactive",
            "group.groupbar.col.locked_active",
            "group.groupbar.col.locked_inactive",
        ],
    ),
    (
        "curseur",
        "Curseur",
        &[
            "cursor.no_hardware_cursors",
            "cursor.inactive_timeout",
            "cursor.hide_on_key_press",
            "cursor.hide_on_touch",
            "cursor.no_warps",
            "cursor.persistent_warps",
            "cursor.warp_on_monitor_change",
            "cursor.warp_on_change_workspace",
            "cursor.zoom_factor",
            "cursor.enable_hyprcursor",
        ],
    ),
    (
        "gestes",
        "Glissement entre workspaces",
        &[
            "gestures.workspace_swipe_distance",
            "gestures.workspace_swipe_invert",
            "gestures.workspace_swipe_forever",
            "gestures.workspace_swipe_cancel_ratio",
        ],
    ),
    (
        "souris",
        "Pointeur et défilement (avancé)",
        &[
            "input.force_no_accel",
            "input.scroll_method",
            "input.scroll_factor",
            "input.special_fallthrough",
            "input.mouse_refocus",
            "misc.middle_click_paste",
        ],
    ),
    (
        "pave",
        "Pavé tactile (avancé)",
        &[
            "input.touchpad.middle_button_emulation",
            "input.touchpad.drag_lock",
            "input.touchpad.tap-and-drag",
        ],
    ),
    (
        "workspaces",
        "Navigation (avancé)",
        &[
            "binds.workspace_back_and_forth",
            "binds.allow_workspace_cycles",
            "misc.close_special_on_empty",
            "binds.hide_special_on_workspace_change",
        ],
    ),
    (
        "raccourcis",
        "Comportement des raccourcis (avancé)",
        &[
            "binds.scroll_event_delay",
            "binds.movefocus_cycles_fullscreen",
        ],
    ),
    (
        "veille",
        "Réveil et verrouillage (avancé)",
        &[
            "misc.mouse_move_enables_dpms",
            "misc.key_press_enables_dpms",
            "misc.allow_session_lock_restore",
        ],
    ),
    (
        "fond",
        "Sous le bureau (avancé)",
        &[
            "misc.disable_hyprland_logo",
            "misc.disable_splash_rendering",
            "misc.force_default_wallpaper",
            "misc.background_color",
        ],
    ),
    (
        "flou",
        "Flou (avancé)",
        &[
            "decoration.blur.ignore_opacity",
            "decoration.blur.proportional",
            "render.use_shader_blur_blend",
        ],
    ),
    (
        "avance",
        "XWayland",
        &[
            "xwayland.enabled",
            "xwayland.use_nearest_neighbor",
            "xwayland.force_zero_scaling",
        ],
    ),
    (
        "avance",
        "Rendu et rechargement",
        &[
            "render.expand_undersized_textures",
            "misc.vrr",
            "misc.disable_autoreload",
        ],
    ),
    (
        "avance",
        "Écosystème et débogage",
        &[
            "ecosystem.no_update_news",
            "ecosystem.no_donation_nag",
            "debug.disable_logs",
            "debug.overlay",
        ],
    ),
    (
        "permissions",
        "Contrôle des accès",
        &["ecosystem.enforce_permissions"],
    ),
];

/// Groupes d'options affichés en configuration de base (le reste du
/// registre est réservé à la configuration avancée).
pub const BASIC_GROUPS: &[&str] = &["fenetres"];

/// Option du registre par chemin pointé.
pub fn opt(path: &str) -> Option<&'static Opt> {
    OPTIONS.iter().find(|o| o.path() == path)
}

/// Fichier généré d'une option.
pub fn domain(path: &str) -> Domain {
    match path.split('.').next().unwrap_or("") {
        "general" | "decoration" | "group" => Domain::Appearance,
        "input" | "gestures" => Domain::Inputs,
        _ => Domain::System,
    }
}

/// Sorte de règle de Hyprland.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    Window,
    Layer,
    Workspace,
}

impl RuleKind {
    pub fn id(self) -> &'static str {
        match self {
            RuleKind::Window => "window",
            RuleKind::Layer => "layer",
            RuleKind::Workspace => "workspace",
        }
    }

    pub fn from_id(s: &str) -> Option<RuleKind> {
        match s {
            "window" => Some(RuleKind::Window),
            "layer" => Some(RuleKind::Layer),
            "workspace" => Some(RuleKind::Workspace),
            _ => None,
        }
    }

    /// Fonction Lua de Hyprland.
    pub fn func(self) -> &'static str {
        match self {
            RuleKind::Window => "hl.window_rule",
            RuleKind::Layer => "hl.layer_rule",
            RuleKind::Workspace => "hl.workspace_rule",
        }
    }
}

/// Règle de Hyprland tenue par hyprdesk : ses champs tels quels
/// (`{ name, match = { class }, float = true, … }`), pour ne rien perdre
/// à l'import ; l'éditeur des Réglages en connaît les plus courants.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub kind: RuleKind,
    pub body: serde_json::Value,
}

/// Options réglées dans hyprdesk (creux : seules celles posées ici).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hypr {
    pub options: BTreeMap<String, OptValue>,
    /// Variables d'environnement (`hl.env`), dans l'ordre.
    pub env: Vec<(String, String)>,
    /// Règles de fenêtres, de couches et de workspaces, dans l'ordre.
    pub rules: Vec<Rule>,
    /// Bordures aux couleurs du thème de hyprdesk (accent pour la fenêtre
    /// active, fond pour les autres) ; gagne sur les couleurs réglées.
    pub theme_borders: bool,
    /// Gestes du pavé tactile (`hl.gesture`), dans l'ordre : leurs champs
    /// tels quels (`{ fingers = 3, direction = "horizontal", action = "workspace" }`).
    pub gestures: Vec<serde_json::Value>,
    /// Réglages par périphérique (`hl.device`) : `{ name = "…", sensitivity = … }`.
    pub devices: Vec<serde_json::Value>,
    /// Permissions (`hl.permission`), lues au démarrage de Hyprland seulement.
    pub permissions: Vec<Permission>,
}

/// Permission de Hyprland : un programme (regex de son chemin) ou un
/// clavier (regex de son nom), une sorte d'accès, une décision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub target: String,
    pub kind: String,
    pub mode: String,
}

/// Sortes de permission de Hyprland : (valeur, libellé).
pub const PERMISSION_KINDS: &[(&str, &str)] = &[
    ("screencopy", "Capture de l'écran"),
    ("plugin", "Chargement de plugins"),
    ("keyboard", "Clavier (nom du périphérique)"),
];

/// Décisions d'une permission : (valeur, libellé).
pub const PERMISSION_MODES: &[(&str, &str)] = &[
    ("allow", "Autoriser"),
    ("ask", "Demander"),
    ("deny", "Refuser"),
];

/// Permissions conseillées quand `ecosystem.enforce_permissions` est actif :
/// captures (grim, portail, hyprdesk) et plugins (hyprpm).
pub const PERMISSION_PRESETS: &[(&str, &str, &str)] = &[
    ("/usr/(bin|local/bin)/grim", "screencopy", "allow"),
    (
        "/usr/(lib|libexec|lib64)/xdg-desktop-portal-hyprland",
        "screencopy",
        "allow",
    ),
    ("/usr/(bin|local/bin)/hyprdesk", "screencopy", "allow"),
    ("/usr/(bin|local/bin)/hyprpm", "plugin", "allow"),
];

/// Valeur JSON en littéral Lua (tables, chaînes échappées).
pub fn json_lua(v: &serde_json::Value) -> String {
    use serde_json::Value as J;
    match v {
        J::Null => "nil".into(),
        J::Bool(b) => b.to_string(),
        J::Number(n) => n.to_string(),
        J::String(s) => format!("\"{}\"", hyprpanel_core::ipc::lua_escape(s)),
        J::Array(a) => format!(
            "{{ {} }}",
            a.iter().map(json_lua).collect::<Vec<_>>().join(", ")
        ),
        J::Object(m) => {
            let f: Vec<String> = m
                .iter()
                .map(|(k, v)| {
                    let key = if k.chars().all(|c| c.is_alphanumeric() || c == '_')
                        && !k.starts_with(|c: char| c.is_ascii_digit())
                    {
                        k.clone()
                    } else {
                        format!("[\"{}\"]", hyprpanel_core::ipc::lua_escape(k))
                    };
                    format!("{key} = {}", json_lua(v))
                })
                .collect();
            format!("{{ {} }}", f.join(", "))
        }
    }
}

impl Hypr {
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
            && self.env.is_empty()
            && self.rules.is_empty()
            && !self.theme_borders
            && self.gestures.is_empty()
            && self.devices.is_empty()
            && self.permissions.is_empty()
    }

    /// Forme de settings.lua : `{ ["general.gaps_in"] = 5, env = {…}, rules = {…} }`.
    pub fn to_lua(&self) -> String {
        let mut items: Vec<String> = self
            .options
            .iter()
            .map(|(k, v)| format!("[\"{k}\"] = {}", v.lua()))
            .collect();
        if !self.env.is_empty() {
            let e: Vec<String> = self
                .env
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{{ \"{}\", \"{}\" }}",
                        hyprpanel_core::ipc::lua_escape(k),
                        hyprpanel_core::ipc::lua_escape(v)
                    )
                })
                .collect();
            items.push(format!("env = {{ {} }}", e.join(", ")));
        }
        if self.theme_borders {
            items.push("theme_borders = true".into());
        }
        if !self.rules.is_empty() {
            let r: Vec<String> = self
                .rules
                .iter()
                .map(|r| {
                    format!(
                        "{{ kind = \"{}\", rule = {} }}",
                        r.kind.id(),
                        json_lua(&r.body)
                    )
                })
                .collect();
            items.push(format!("rules = {{ {} }}", r.join(", ")));
        }
        for (key, list) in [("gestures", &self.gestures), ("devices", &self.devices)] {
            if !list.is_empty() {
                let l: Vec<String> = list.iter().map(json_lua).collect();
                items.push(format!("{key} = {{ {} }}", l.join(", ")));
            }
        }
        if !self.permissions.is_empty() {
            let l: Vec<String> = self
                .permissions
                .iter()
                .map(|p| {
                    format!(
                        "{{ \"{}\", \"{}\", \"{}\" }}",
                        hyprpanel_core::ipc::lua_escape(&p.target),
                        hyprpanel_core::ipc::lua_escape(&p.kind),
                        hyprpanel_core::ipc::lua_escape(&p.mode)
                    )
                })
                .collect();
            items.push(format!("permissions = {{ {} }}", l.join(", ")));
        }
        format!("{{ {} }}", items.join(", "))
    }
}

/// Lit `hypr = { … }` de settings.lua ou hyprdesk.lua.
pub fn parse(t: &mlua::Table) -> Result<Hypr, String> {
    let mut h = Hypr::default();
    if let Some(env) = t
        .get::<Option<mlua::Table>>("env")
        .map_err(|e| e.to_string())?
    {
        for e in env.sequence_values::<mlua::Table>() {
            let e = e.map_err(|e| e.to_string())?;
            let k: String = e.get(1).map_err(|e| e.to_string())?;
            let v: String = e.get(2).map_err(|e| e.to_string())?;
            h.env.push((k, v));
        }
    }
    if let Some(rules) = t
        .get::<Option<mlua::Table>>("rules")
        .map_err(|e| e.to_string())?
    {
        for r in rules.sequence_values::<mlua::Table>() {
            let r = r.map_err(|e| e.to_string())?;
            let kind: String = r.get("kind").map_err(|e| e.to_string())?;
            let kind = RuleKind::from_id(&kind)
                .ok_or_else(|| crate::tr!("hypr.rules : sorte « {k} » inconnue", k = kind))?;
            let body: mlua::Value = r.get("rule").map_err(|e| e.to_string())?;
            h.rules.push(Rule {
                kind,
                body: lua_to_json(&body),
            });
        }
    }
    for (key, list) in [("gestures", &mut h.gestures), ("devices", &mut h.devices)] {
        if let Some(t) = t
            .get::<Option<mlua::Table>>(key)
            .map_err(|e| e.to_string())?
        {
            for g in t.sequence_values::<mlua::Value>() {
                let g = g.map_err(|e| e.to_string())?;
                list.push(lua_to_json(&g));
            }
        }
    }
    if let Some(perms) = t
        .get::<Option<mlua::Table>>("permissions")
        .map_err(|e| e.to_string())?
    {
        for p in perms.sequence_values::<mlua::Table>() {
            let p = p.map_err(|e| e.to_string())?;
            let field = |i: i64| -> Result<String, String> { p.get(i).map_err(|e| e.to_string()) };
            h.permissions.push(Permission {
                target: field(1)?,
                kind: field(2)?,
                mode: field(3)?,
            });
        }
    }
    for pair in t.clone().pairs::<String, mlua::Value>() {
        let (k, v) = pair.map_err(|e| e.to_string())?;
        if k == "theme_borders" {
            h.theme_borders = matches!(v, mlua::Value::Boolean(true));
            continue;
        }
        if matches!(
            k.as_str(),
            "env" | "rules" | "gestures" | "devices" | "permissions"
        ) {
            continue;
        }
        let Some(o) = opt(&k) else {
            return Err(crate::tr!("hypr : option inconnue « {k} »", k = k));
        };
        let json = lua_to_json(&v);
        let value = OptValue::from_json(&o.kind, &json)
            .ok_or_else(|| crate::tr!("hypr : valeur invalide pour « {k} »", k = k))?;
        h.options.insert(k, value);
    }
    Ok(h)
}

/// Chaîne non vide à `path` (pointé) dans `v`.
fn filled(v: &serde_json::Value, path: &str) -> bool {
    path.split('.')
        .try_fold(v, |acc, k| acc.get(k))
        .and_then(|x| x.as_str())
        .is_some_and(|s| !s.trim().is_empty())
}

/// Élément assez rempli pour être écrit : une règle sans cible, un geste
/// sans action, un périphérique sans nom restent dans les Réglages sans
/// aller dans les fichiers de Hyprland (qui les refuseraient).
pub fn rule_complete(r: &Rule) -> bool {
    match r.kind {
        RuleKind::Window => r
            .body
            .get("match")
            .and_then(|m| m.as_object())
            .is_some_and(|m| {
                m.values()
                    .any(|v| v.as_str().is_none_or(|s| !s.trim().is_empty()))
            }),
        RuleKind::Layer => filled(&r.body, "match.namespace"),
        RuleKind::Workspace => filled(&r.body, "workspace"),
    }
}

/// Geste complet : doigts, sens, action.
pub fn gesture_complete(g: &serde_json::Value) -> bool {
    g.get("fingers").and_then(|v| v.as_i64()).is_some()
        && filled(g, "direction")
        && filled(g, "action")
}

/// `hl.gesture(…)` ou `hl.device(…)` des éléments complets, une ligne
/// chacun, dans l'ordre.
pub fn calls_lua(func: &str, list: &[serde_json::Value]) -> String {
    list.iter()
        .filter(|v| match func {
            "gesture" => gesture_complete(v),
            _ => filled(v, "name"),
        })
        .map(|v| format!("hl.{func}({})\n", json_lua(v)))
        .collect()
}

/// `hl.permission("cible", "sorte", "décision")`, une ligne chacune.
pub fn permissions_lua(list: &[Permission]) -> String {
    list.iter()
        .filter(|p| !p.target.trim().is_empty())
        .map(|p| {
            format!(
                "hl.permission(\"{}\", \"{}\", \"{}\")\n",
                hyprpanel_core::ipc::lua_escape(&p.target),
                hyprpanel_core::ipc::lua_escape(&p.kind),
                hyprpanel_core::ipc::lua_escape(&p.mode)
            )
        })
        .collect()
}

fn lua_to_json(v: &mlua::Value) -> serde_json::Value {
    use serde_json::Value as J;
    match v {
        mlua::Value::Boolean(b) => J::Bool(*b),
        mlua::Value::Integer(i) => J::from(*i),
        mlua::Value::Number(n) => serde_json::Number::from_f64(*n).map_or(J::Null, J::Number),
        mlua::Value::String(s) => J::String(s.to_string_lossy().to_string()),
        mlua::Value::Table(t) => {
            if t.raw_len() > 0 {
                J::Array(
                    t.clone()
                        .sequence_values::<mlua::Value>()
                        .flatten()
                        .map(|v| lua_to_json(&v))
                        .collect(),
                )
            } else {
                let mut m = serde_json::Map::new();
                for (k, v) in t.clone().pairs::<String, mlua::Value>().flatten() {
                    m.insert(k, lua_to_json(&v));
                }
                J::Object(m)
            }
        }
        _ => J::Null,
    }
}

/// Tables imbriquées pour `hl.config` : `general.col.active_border` →
/// `general = { col = { active_border = … } }`.
pub fn nested(opts: &BTreeMap<String, OptValue>) -> String {
    #[derive(Default)]
    struct Node {
        leaf: Option<String>,
        kids: BTreeMap<String, Node>,
    }
    let mut root = Node::default();
    for (path, v) in opts {
        let mut n = &mut root;
        for part in path.split('.') {
            n = n.kids.entry(part.to_string()).or_default();
        }
        n.leaf = Some(v.lua());
    }
    fn key(k: &str) -> String {
        if k.chars().all(|c| c.is_alphanumeric() || c == '_')
            && !k.starts_with(|c: char| c.is_ascii_digit())
        {
            k.to_string()
        } else {
            format!("[\"{k}\"]")
        }
    }
    fn render(n: &Node, indent: usize, out: &mut String) {
        for (k, kid) in &n.kids {
            let pad = "    ".repeat(indent);
            match &kid.leaf {
                Some(v) => out.push_str(&format!("{pad}{} = {v},\n", key(k))),
                None => {
                    out.push_str(&format!("{pad}{} = {{\n", key(k)));
                    render(kid, indent + 1, out);
                    out.push_str(&format!("{pad}}},\n"));
                }
            }
        }
    }
    let mut out = String::from("{\n");
    render(&root, 1, &mut out);
    out.push('}');
    out
}

/// Lua appliqué à chaud (sans fichier) : une seule `hl.config`.
pub fn apply_lua(h: &Hypr) -> String {
    if h.options.is_empty() {
        return String::new();
    }
    format!("pcall(hl.config, {})\n", nested(&h.options))
}

/// Applique à chaud ; une option retirée demande une relecture de la config
/// de Hyprland (les fichiers générés n'en contiennent plus la valeur).
pub fn apply(h: &Hypr, removed: bool) -> Result<(), String> {
    use hyprpanel_core::ipc;
    if removed {
        return ipc::eval("hl.dispatch(hl.dsp.exec_cmd(\"hyprctl reload\"))")
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    let lua = apply_lua(h);
    if lua.is_empty() {
        return Ok(());
    }
    ipc::eval(&lua).map(|_| ()).map_err(|e| e.to_string())
}

/// Valeurs en vigueur dans Hyprland (`hl.get_config`), forme lisible.
pub fn current() -> BTreeMap<String, String> {
    use hyprpanel_core::ipc;
    let keys: Vec<String> = OPTIONS.iter().map(|o| o.path()).collect();
    let lua = format!(
        "local out = {{}} for _, k in ipairs({{ {} }}) do local ok, v = pcall(hl.get_config, k) if ok and v ~= nil then out[#out + 1] = k .. '=' .. tostring(v) end end return table.concat(out, '\\n')",
        keys.iter()
            .map(|k| format!("\"{k}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let Ok(text) = ipc::repl(&lua) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_string(), v.trim_matches('"').to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registre_sans_doublon_ni_recouvrement() {
        let mut seen = std::collections::BTreeSet::new();
        for o in OPTIONS {
            let p = o.path();
            assert!(seen.insert(p.clone()), "doublon : {p}");
            assert!(crate::fx::opt(&p).is_none(), "déjà dans fx : {p}");
            assert!(crate::tiling::opt(&p).is_none(), "déjà dans tiling : {p}");
        }
        for (_, _, paths) in GROUPS {
            for p in *paths {
                assert!(opt(p).is_some(), "groupe vers une option absente : {p}");
            }
        }
        // Chaque option est montrée quelque part.
        for o in OPTIONS {
            let p = o.path();
            assert!(
                GROUPS.iter().any(|(_, _, ps)| ps.contains(&p.as_str())),
                "option sans groupe : {p}"
            );
        }
    }

    #[test]
    fn rendu_imbrique() {
        let mut h = Hypr::default();
        h.options
            .insert("general.gaps_in".into(), OptValue::Sides([5; 4]));
        h.options.insert(
            "general.col.active_border".into(),
            OptValue::Gradient(vec![crate::config::Rgba(0x45, 0x41, 0x58, 0xff)], 90),
        );
        h.options
            .insert("input.touchpad.tap-and-drag".into(), OptValue::Bool(true));
        let lua = nested(&h.options);
        assert!(lua.contains("general = {"));
        assert!(lua.contains("col = {"));
        assert!(lua.contains("active_border = { colors = { \"rgba(454158ff)\" }, angle = 90 },"));
        assert!(lua.contains("[\"tap-and-drag\"] = true,"));
        assert!(lua.contains("gaps_in = { top = 5, right = 5, bottom = 5, left = 5 },"));
    }

    #[test]
    fn textes_traduits() {
        let mut texts: Vec<&str> = OPTIONS.iter().map(|o| o.label).collect();
        for (_, title, _) in GROUPS {
            texts.push(title);
        }
        for o in OPTIONS {
            match &o.kind {
                Kind::IntChoice(l) => texts.extend(l.iter().map(|(_, s)| *s)),
                Kind::Choice(l) => texts.extend(l.iter().map(|(_, s)| *s)),
                _ => {}
            }
        }
        let missing = crate::i18n::untranslated("hyprset", texts);
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }
}
