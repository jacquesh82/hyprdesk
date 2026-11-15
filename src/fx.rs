//! Animations et effets de Hyprland (paquet « Animations ») : animations par
//! élément (`hl.animation`), courbes (`hl.curve`), et effets de décoration
//! (arrondis, opacité, flou et ses variantes, ombre, lueur, flou de
//! mouvement, ondulation) par `hl.config`.
//!
//! Relevé sur Hyprland 0.57.1 : options de `hl.meta.lua`, arbre des
//! animations, styles et variantes de flou lus dans le binaire. Comme pour
//! les layouts (`crate::tiling`), seuls les réglages faits dans hyprdesk sont
//! posés, le reste vient de hyprland.lua ; un réglage retiré fait relire sa
//! config à Hyprland, puis hyprdesk repose les siens.

use std::collections::BTreeMap;

use crate::tiling::{Kind, Opt, OptValue};
use hyprpanel_core::ipc::lua_escape;

const ON_OFF: Kind = Kind::Bool;

/// Variantes de flou de Hyprland 0.57 (`decoration.blur.variant`, entier).
pub const BLUR_VARIANTS: &[(i64, &str)] = &[
    (0, "Classique (kawase)"),
    (1, "Givre"),
    (2, "Ondulations"),
    (3, "Gouttes"),
    (4, "Eau"),
    (5, "Fluide"),
    (6, "Prisme"),
    (7, "Air chaud"),
    (8, "Acrylique"),
    (9, "Aurore"),
    (10, "Brume"),
];

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

/// Effets réglables, par groupe d'affichage.
pub const OPTIONS: &[Opt] = &[
    o!("animations", "enabled", "Animations", ON_OFF),
    // Fenêtres
    o!(
        "decoration",
        "rounding",
        "Arrondi des coins",
        num(0.0, 30.0, 1.0)
    ),
    o!(
        "decoration",
        "rounding_power",
        "Forme des coins (2 : cercle, plus : carré arrondi)",
        num(1.0, 10.0, 0.5)
    ),
    o!(
        "decoration",
        "active_opacity",
        "Opacité de la fenêtre active",
        num(0.1, 1.0, 0.05)
    ),
    o!(
        "decoration",
        "inactive_opacity",
        "Opacité des autres fenêtres",
        num(0.1, 1.0, 0.05)
    ),
    o!(
        "decoration",
        "dim_inactive",
        "Assombrir les fenêtres inactives",
        ON_OFF
    ),
    o!(
        "decoration",
        "dim_strength",
        "Force de l'assombrissement",
        num(0.0, 1.0, 0.05)
    ),
    // Flou
    o!(
        "decoration",
        "blur.enabled",
        "Flou derrière les fenêtres",
        ON_OFF
    ),
    o!(
        "decoration",
        "blur.variant",
        "Variante",
        Kind::IntChoice(BLUR_VARIANTS)
    ),
    o!("decoration", "blur.size", "Rayon", num(1.0, 20.0, 1.0)),
    o!("decoration", "blur.passes", "Passes", num(1.0, 6.0, 1.0)),
    o!(
        "decoration",
        "blur.vibrancy",
        "Saturation",
        num(0.0, 1.0, 0.05)
    ),
    o!("decoration", "blur.noise", "Grain", num(0.0, 0.5, 0.01)),
    // Ombre
    o!("decoration", "shadow.enabled", "Ombre portée", ON_OFF),
    o!("decoration", "shadow.range", "Étendue", num(0.0, 50.0, 1.0)),
    o!(
        "decoration",
        "shadow.render_power",
        "Atténuation",
        num(1.0, 4.0, 1.0)
    ),
    // Lueur
    o!("decoration", "glow.enabled", "Lueur intérieure", ON_OFF),
    o!("decoration", "glow.range", "Étendue", num(0.0, 50.0, 1.0)),
    o!(
        "decoration",
        "glow.render_power",
        "Atténuation",
        num(1.0, 4.0, 1.0)
    ),
    // Mouvement
    o!(
        "decoration",
        "motion_blur.enabled",
        "Flou de mouvement",
        ON_OFF
    ),
    o!(
        "decoration",
        "wobble.enabled",
        "Fenêtres ondulantes (déplacement, redimensionnement)",
        ON_OFF
    ),
    o!(
        "decoration",
        "wobble.intensity",
        "Intensité de l'ondulation",
        num(0.1, 3.0, 0.1)
    ),
    // Coût du rendu (paquet « Performance », `crate::perf`).
    o!(
        "decoration",
        "blur.new_optimizations",
        "Optimisations du flou",
        ON_OFF
    ),
    o!(
        "decoration",
        "blur.xray",
        "Flou des fenêtres flottantes calculé sur le fond seul",
        ON_OFF
    ),
    o!(
        "decoration",
        "blur.popups",
        "Flou derrière les menus contextuels",
        ON_OFF
    ),
    o!(
        "decoration",
        "blur.special",
        "Flou derrière le workspace spécial",
        ON_OFF
    ),
    o!(
        "misc",
        "animate_manual_resizes",
        "Animer les redimensionnements à la souris",
        ON_OFF
    ),
    o!(
        "misc",
        "animate_mouse_windowdragging",
        "Animer le glissé des fenêtres",
        ON_OFF
    ),
    o!(
        "debug",
        "vfr",
        "Ne redessiner que ce qui change (VFR)",
        ON_OFF
    ),
    o!(
        "render",
        "direct_scanout",
        "Affichage direct d'une fenêtre plein écran",
        Kind::IntChoice(SCANOUT)
    ),
    o!(
        "render",
        "new_render_scheduling",
        "Triple tampon quand l'image prend du retard",
        ON_OFF
    ),
];

/// `render.direct_scanout` de Hyprland 0.57 (entier).
pub const SCANOUT: &[(i64, &str)] = &[(0, "Jamais"), (1, "Toujours"), (2, "Jeux seulement")];

/// Groupes de l'onglet Effets : (titre, préfixe des clés).
pub const GROUPS: &[(&str, &[&str])] = &[
    (
        "Fenêtres",
        &[
            "decoration.rounding",
            "decoration.rounding_power",
            "decoration.active_opacity",
            "decoration.inactive_opacity",
            "decoration.dim_inactive",
            "decoration.dim_strength",
        ],
    ),
    (
        "Flou",
        &[
            "decoration.blur.enabled",
            "decoration.blur.variant",
            "decoration.blur.size",
            "decoration.blur.passes",
            "decoration.blur.vibrancy",
            "decoration.blur.noise",
        ],
    ),
    (
        "Ombre et lueur",
        &[
            "decoration.shadow.enabled",
            "decoration.shadow.range",
            "decoration.shadow.render_power",
            "decoration.glow.enabled",
            "decoration.glow.range",
            "decoration.glow.render_power",
        ],
    ),
    (
        "Mouvement",
        &[
            "decoration.motion_blur.enabled",
            "decoration.wobble.enabled",
            "decoration.wobble.intensity",
        ],
    ),
];

pub fn opt(path: &str) -> Option<&'static Opt> {
    OPTIONS.iter().find(|o| o.path() == path)
}

/// Élément animé de Hyprland : (nom, libellé, styles admis).
pub struct Leaf {
    pub name: &'static str,
    pub label: &'static str,
    pub styles: &'static [&'static str],
    /// Montré dans l'onglet (les autres : Lua et ligne de commande).
    pub main: bool,
}

const WIN_STYLES: &[&str] = &["slide", "popin", "gnomed"];
const LAYER_STYLES: &[&str] = &["slide", "popin", "fade"];
const WS_STYLES: &[&str] = &["slide", "slidevert", "fade", "slidefade", "slidefadevert"];

macro_rules! leaf {
    ($name:literal, $label:literal, $styles:expr, $main:literal) => {
        Leaf {
            name: $name,
            label: $label,
            styles: $styles,
            main: $main,
        }
    };
}

/// Arbre des animations de Hyprland 0.57 (un parent règle ses enfants
/// non réglés).
pub const LEAVES: &[Leaf] = &[
    leaf!("global", "Toutes les animations", &[], true),
    leaf!("windows", "Fenêtres", WIN_STYLES, true),
    leaf!("windowsIn", "Ouverture des fenêtres", WIN_STYLES, true),
    leaf!("windowsOut", "Fermeture des fenêtres", WIN_STYLES, true),
    leaf!("windowsMove", "Déplacement des fenêtres", &[], true),
    leaf!("layers", "Barres et menus (calques)", LAYER_STYLES, true),
    leaf!("layersIn", "Apparition des calques", LAYER_STYLES, false),
    leaf!("layersOut", "Disparition des calques", LAYER_STYLES, false),
    leaf!("fade", "Fondus", &[], true),
    leaf!("fadeIn", "Fondu d'ouverture", &[], false),
    leaf!("fadeOut", "Fondu de fermeture", &[], false),
    leaf!("fadeSwitch", "Fondu au changement de focus", &[], false),
    leaf!("fadeShadow", "Fondu des ombres", &[], false),
    leaf!("fadeDim", "Fondu de l'assombrissement", &[], false),
    leaf!("fadeLayersIn", "Fondu d'apparition des calques", &[], false),
    leaf!(
        "fadeLayersOut",
        "Fondu de disparition des calques",
        &[],
        false
    ),
    leaf!("fadePopupsIn", "Fondu d'apparition des popups", &[], false),
    leaf!(
        "fadePopupsOut",
        "Fondu de disparition des popups",
        &[],
        false
    ),
    leaf!("fadeDpms", "Fondu à l'extinction des écrans", &[], false),
    leaf!("border", "Couleur des bordures", &[], true),
    leaf!(
        "borderangle",
        "Rotation du dégradé des bordures",
        &[],
        false
    ),
    leaf!("workspaces", "Changement de workspace", WS_STYLES, true),
    leaf!("workspacesIn", "Arrivée d'un workspace", WS_STYLES, false),
    leaf!("workspacesOut", "Départ d'un workspace", WS_STYLES, false),
    leaf!("specialWorkspace", "Workspace spécial", WS_STYLES, true),
    leaf!(
        "specialWorkspaceIn",
        "Ouverture du workspace spécial",
        WS_STYLES,
        false
    ),
    leaf!(
        "specialWorkspaceOut",
        "Fermeture du workspace spécial",
        WS_STYLES,
        false
    ),
    leaf!("zoomFactor", "Zoom de l'écran", &[], false),
    leaf!("monitorAdded", "Écran branché", &[], false),
];

pub fn leaf(name: &str) -> Option<&'static Leaf> {
    LEAVES.iter().find(|l| l.name == name)
}

/// Libellé d'un style d'animation.
pub fn style_label(s: &str) -> &'static str {
    match s {
        "slide" => "Glisser",
        "slidevert" => "Glisser (vertical)",
        "popin" => "Surgir",
        "gnomed" => "GNOME",
        "fade" => "Fondu",
        "slidefade" => "Glisser en fondu",
        "slidefadevert" => "Glisser en fondu (vertical)",
        _ => "?",
    }
}

/// Courbe de mouvement.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveDef {
    /// Courbe intégrée de Hyprland (`default`).
    Builtin,
    Bezier([f64; 4]),
    Spring {
        mass: f64,
        stiffness: f64,
        damping: f64,
    },
}

pub struct Curve {
    /// Nom dans hyprdesk ; Hyprland la connaît comme `hd-<nom>` (sauf `default`).
    pub name: &'static str,
    pub label: &'static str,
    pub def: CurveDef,
}

/// Courbes proposées : celles de la config livrée avec Hyprland 0.57, et un rebond.
pub const CURVES: &[Curve] = &[
    Curve {
        name: "default",
        label: "Par défaut (Hyprland)",
        def: CurveDef::Builtin,
    },
    Curve {
        name: "ease-out",
        label: "Douce à l'arrivée",
        def: CurveDef::Bezier([0.23, 1.0, 0.32, 1.0]),
    },
    Curve {
        name: "ease-in-out",
        label: "Douce aux deux bouts",
        def: CurveDef::Bezier([0.65, 0.05, 0.36, 1.0]),
    },
    Curve {
        name: "quick",
        label: "Vive",
        def: CurveDef::Bezier([0.15, 0.0, 0.1, 1.0]),
    },
    Curve {
        name: "almost-linear",
        label: "Presque linéaire",
        def: CurveDef::Bezier([0.5, 0.5, 0.75, 1.0]),
    },
    Curve {
        name: "linear",
        label: "Linéaire",
        def: CurveDef::Bezier([0.0, 0.0, 1.0, 1.0]),
    },
    Curve {
        name: "overshoot",
        label: "Rebond",
        def: CurveDef::Bezier([0.05, 0.9, 0.1, 1.05]),
    },
    Curve {
        name: "spring",
        label: "Ressort",
        def: CurveDef::Spring {
            mass: 1.0,
            stiffness: 238.1191,
            damping: 24.21279333,
        },
    },
];

pub fn curve(name: &str) -> Option<&'static Curve> {
    CURVES.iter().find(|c| c.name == name)
}

/// Réglage d'un élément animé.
#[derive(Debug, Clone, PartialEq)]
pub struct Anim {
    pub enabled: bool,
    /// Durée en dixièmes de seconde (plus petit : plus rapide).
    pub speed: f64,
    pub curve: String,
    pub style: Option<String>,
}

impl Anim {
    /// Point de départ quand on personnalise un élément.
    pub fn new_for(leaf: &Leaf) -> Anim {
        Anim {
            enabled: true,
            speed: 5.0,
            curve: "ease-out".into(),
            style: leaf.styles.first().map(|s| s.to_string()),
        }
    }

    fn lua(&self, leaf: &str, user: &BTreeMap<String, CurveDef>) -> String {
        let mut f = vec![
            format!("leaf = \"{}\"", lua_escape(leaf)),
            format!("enabled = {}", self.enabled),
        ];
        if self.enabled {
            f.push(format!("speed = {}", OptValue::Num(self.speed).lua()));
            // Courbe de l'utilisateur (gardée sous son nom), puis catalogue
            // de hyprdesk (`hd-<nom>`), sinon celle de Hyprland.
            match (user.get(&self.curve), curve(&self.curve).map(|c| &c.def)) {
                (Some(CurveDef::Spring { .. }), _) => {
                    f.push(format!("spring = \"{}\"", lua_escape(&self.curve)))
                }
                (Some(_), _) => f.push(format!("bezier = \"{}\"", lua_escape(&self.curve))),
                (None, Some(CurveDef::Spring { .. })) => {
                    f.push(format!("spring = \"hd-{}\"", self.curve))
                }
                (None, Some(CurveDef::Bezier(_))) => {
                    f.push(format!("bezier = \"hd-{}\"", self.curve))
                }
                _ => f.push("bezier = \"default\"".into()),
            }
            if let Some(s) = &self.style {
                f.push(format!("style = \"{}\"", lua_escape(s)));
            }
        }
        format!("{{ {} }}", f.join(", "))
    }
}

/// Réglages faits dans hyprdesk.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fx {
    /// `decoration.blur.size` → valeur.
    pub options: BTreeMap<String, OptValue>,
    /// Élément (`windowsIn`) → réglage.
    pub anims: BTreeMap<String, Anim>,
    /// Courbes de l'utilisateur (reprises de hyprland.lua ou créées ici),
    /// connues de Hyprland sous leur nom.
    pub curves: BTreeMap<String, CurveDef>,
}

/// Courbe en Lua (`hl.curve`).
fn curve_lua(name: &str, def: &CurveDef) -> Option<String> {
    match def {
        CurveDef::Builtin => None,
        CurveDef::Bezier([x1, y1, x2, y2]) => Some(format!(
            "hl.curve(\"{}\", {{ type = \"bezier\", points = {{ {{{x1}, {y1}}}, {{{x2}, {y2}}} }} }})",
            lua_escape(name)
        )),
        CurveDef::Spring {
            mass,
            stiffness,
            damping,
        } => Some(format!(
            "hl.curve(\"{}\", {{ type = \"spring\", mass = {mass}, stiffness = {stiffness}, damping = {damping} }})",
            lua_escape(name)
        )),
    }
}

/// Courbe lue dans un fichier Lua (`hyprsrc`) : `{ type, points | mass… }`.
pub fn curve_from_json(v: &serde_json::Value) -> Option<CurveDef> {
    let n = |x: &serde_json::Value| x.as_f64();
    match v.get("type").and_then(|t| t.as_str()).unwrap_or("bezier") {
        "spring" => Some(CurveDef::Spring {
            mass: n(v.get("mass")?)?,
            stiffness: n(v.get("stiffness")?)?,
            damping: n(v.get("damping")?)?,
        }),
        _ => {
            let p = v.get("points")?.as_array()?;
            let a = p.first()?.as_array()?;
            let b = p.get(1)?.as_array()?;
            Some(CurveDef::Bezier([
                n(&a[0])?,
                n(&a[1])?,
                n(&b[0])?,
                n(&b[1])?,
            ]))
        }
    }
}

impl Fx {
    pub fn is_empty(&self) -> bool {
        self.options.is_empty() && self.anims.is_empty() && self.curves.is_empty()
    }

    /// Courbe connue (catalogue ou utilisateur).
    pub fn knows_curve(&self, name: &str) -> bool {
        curve(name).is_some() || self.curves.contains_key(name)
    }

    /// Table Lua pour `hd.config({ fx = … })` (settings.lua généré).
    pub fn to_lua(&self) -> String {
        let mut f: Vec<String> = self
            .options
            .iter()
            .map(|(k, v)| format!("[\"{k}\"] = {}", v.lua()))
            .collect();
        if !self.anims.is_empty() {
            let a: Vec<String> = self
                .anims
                .iter()
                .map(|(leaf, a)| {
                    let mut g = vec![
                        format!("enabled = {}", a.enabled),
                        format!("speed = {}", OptValue::Num(a.speed).lua()),
                        format!("curve = \"{}\"", lua_escape(&a.curve)),
                    ];
                    if let Some(s) = &a.style {
                        g.push(format!("style = \"{}\"", lua_escape(s)));
                    }
                    format!("{leaf} = {{ {} }}", g.join(", "))
                })
                .collect();
            f.push(format!("animations = {{ {} }}", a.join(", ")));
        }
        if !self.curves.is_empty() {
            let c: Vec<String> = self
                .curves
                .iter()
                .filter_map(|(name, d)| {
                    let body = match d {
                        CurveDef::Builtin => return None,
                        CurveDef::Bezier([a, b, c, e]) => format!("{{ {a}, {b}, {c}, {e} }}"),
                        CurveDef::Spring {
                            mass,
                            stiffness,
                            damping,
                        } => format!(
                            "{{ mass = {mass}, stiffness = {stiffness}, damping = {damping} }}"
                        ),
                    };
                    Some(format!("[\"{}\"] = {body}", lua_escape(name)))
                })
                .collect();
            f.push(format!("curves = {{ {} }}", c.join(", ")));
        }
        format!("{{ {} }}", f.join(", "))
    }
}

/// Lit `fx = { ["decoration.blur.size"] = 8, animations = { windows = { … } } }`.
pub fn parse(t: &mlua::Table) -> Result<Fx, String> {
    use mlua::Value;
    let mut fx = Fx::default();
    // Courbes d'abord : les animations peuvent s'y référer.
    if let Some(curves) = t
        .get::<Option<mlua::Table>>("curves")
        .map_err(|e| e.to_string())?
    {
        for pair in curves.pairs::<String, mlua::Table>() {
            let (name, c) = pair.map_err(|e| e.to_string())?;
            let def = if c.contains_key("mass").unwrap_or(false) {
                CurveDef::Spring {
                    mass: c.get("mass").map_err(|e| e.to_string())?,
                    stiffness: c.get("stiffness").map_err(|e| e.to_string())?,
                    damping: c.get("damping").map_err(|e| e.to_string())?,
                }
            } else {
                let p: Vec<f64> = c
                    .sequence_values::<f64>()
                    .collect::<Result<_, _>>()
                    .map_err(|e| e.to_string())?;
                let [a, b, cc, d] = p[..] else {
                    return Err(crate::tr!(
                        "fx.curves.{name} : quatre nombres attendus",
                        name = name
                    ));
                };
                CurveDef::Bezier([a, b, cc, d])
            };
            fx.curves.insert(name, def);
        }
    }
    for pair in t.pairs::<String, Value>() {
        let (k, v) = pair.map_err(|e| e.to_string())?;
        if k == "curves" {
            continue;
        }
        if k == "animations" {
            let Value::Table(list) = v else {
                return Err(crate::i18n::tr("fx.animations : table attendue").to_string());
            };
            for pair in list.pairs::<String, mlua::Table>() {
                let (name, a) = pair.map_err(|e| e.to_string())?;
                let Some(l) = leaf(&name) else {
                    let all: Vec<&str> = LEAVES.iter().map(|l| l.name).collect();
                    return Err(crate::tr!(
                        "fx.animations : élément « {name} » inconnu (connus : {all})",
                        name = name,
                        all = all.join(", ")
                    ));
                };
                let curve_name: String = a
                    .get::<Option<String>>("curve")
                    .map_err(|e| e.to_string())?
                    .unwrap_or_else(|| "default".into());
                if !fx.knows_curve(&curve_name) {
                    let all: Vec<&str> = CURVES.iter().map(|c| c.name).collect();
                    return Err(crate::tr!(
                        "fx.animations.{name} : courbe « {c} » inconnue (connues : {all})",
                        name = name,
                        c = curve_name,
                        all = all.join(", ")
                    ));
                }
                let style: Option<String> = a.get("style").map_err(|e| e.to_string())?;
                if let Some(s) = &style
                    && !l.styles.contains(&s.as_str())
                {
                    return Err(crate::tr!(
                        "fx.animations.{name} : style « {s} » non admis ici ({all})",
                        name = name,
                        s = s,
                        all = if l.styles.is_empty() {
                            "aucun".to_string()
                        } else {
                            l.styles.join(", ")
                        }
                    ));
                }
                let speed: f64 = a
                    .get::<Option<f64>>("speed")
                    .map_err(|e| e.to_string())?
                    .unwrap_or(5.0);
                fx.anims.insert(
                    name,
                    Anim {
                        enabled: a
                            .get::<Option<bool>>("enabled")
                            .map_err(|e| e.to_string())?
                            .unwrap_or(true),
                        speed: speed.clamp(0.1, 50.0),
                        curve: curve_name,
                        style,
                    },
                );
            }
            continue;
        }
        let Some(o) = opt(&k) else {
            return Err(crate::tr!("fx : option « {k} » inconnue", k = k));
        };
        let text = match v {
            Value::Boolean(b) => b.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.to_str().map_err(|e| e.to_string())?.to_string(),
            _ => return Err(crate::tr!("fx : « {k} » : valeur illisible", k = k)),
        };
        fx.options.insert(
            k.clone(),
            o.parse(&text).map_err(|e| format!("fx.{k} : {e}"))?,
        );
    }
    Ok(fx)
}

/// Table Lua imbriquée pour `hl.config` : `decoration = { blur = { size = 8 } }`.
fn nested(options: &BTreeMap<String, OptValue>) -> String {
    #[derive(Default)]
    struct Node(BTreeMap<String, Node>, Option<String>);
    let mut root = Node::default();
    for (path, v) in options {
        let mut n = &mut root;
        for part in path.split('.') {
            n = n.0.entry(part.to_string()).or_default();
        }
        n.1 = Some(v.lua());
    }
    fn render(n: &Node) -> String {
        let f: Vec<String> =
            n.0.iter()
                .map(|(k, c)| match &c.1 {
                    Some(v) => format!("{k} = {v}"),
                    None => format!("{k} = {}", render(c)),
                })
                .collect();
        format!("{{ {} }}", f.join(", "))
    }
    render(&root)
}

/// Clés posées par hyprdesk (pour savoir s'il faut faire relire hyprland.lua).
fn applied_keys(fx: &Fx) -> Vec<String> {
    fx.options
        .keys()
        .cloned()
        .chain(fx.anims.keys().map(|l| format!("anim:{l}")))
        .collect()
}

/// Courbes et éléments animés pour `animations.lua` (MLG-139) : sans
/// `pcall` (une erreur remonte comme erreur de config) ni marqueur d'état.
pub fn file_lua(fx: &Fx) -> String {
    let mut lua = String::new();
    let used: std::collections::BTreeSet<&str> =
        fx.anims.values().map(|a| a.curve.as_str()).collect();
    for (name, def) in &fx.curves {
        if let Some(l) = curve_lua(name, def) {
            lua += &l;
            lua.push('\n');
        }
    }
    for c in CURVES
        .iter()
        .filter(|c| used.contains(c.name) && !fx.curves.contains_key(c.name))
    {
        if let Some(l) = curve_lua(&format!("hd-{}", c.name), &c.def) {
            lua += &l;
            lua.push('\n');
        }
    }
    for l in LEAVES {
        if let Some(a) = fx.anims.get(l.name) {
            lua += &format!("hl.animation({})\n", a.lua(l.name, &fx.curves));
        }
    }
    lua
}

/// Lua qui pose les réglages.
pub fn apply_lua(fx: &Fx) -> String {
    let mut lua = String::new();
    let used: std::collections::BTreeSet<&str> =
        fx.anims.values().map(|a| a.curve.as_str()).collect();
    for (name, def) in &fx.curves {
        if let Some(l) = curve_lua(name, def) {
            lua += &format!("pcall(function() {l} end)\n");
        }
    }
    for c in CURVES
        .iter()
        .filter(|c| used.contains(c.name) && !fx.curves.contains_key(c.name))
    {
        match &c.def {
            CurveDef::Builtin => {}
            CurveDef::Bezier([x1, y1, x2, y2]) => {
                lua += &format!(
                    "pcall(hl.curve, \"hd-{}\", {{ type = \"bezier\", points = {{ {{{x1}, {y1}}}, {{{x2}, {y2}}} }} }})\n",
                    c.name
                );
            }
            CurveDef::Spring {
                mass,
                stiffness,
                damping,
            } => {
                lua += &format!(
                    "pcall(hl.curve, \"hd-{}\", {{ type = \"spring\", mass = {mass}, stiffness = {stiffness}, damping = {damping} }})\n",
                    c.name
                );
            }
        }
    }
    if !fx.options.is_empty() {
        lua += &format!("pcall(hl.config, {})\n", nested(&fx.options));
    }
    // Parents d'abord : un enfant réglé l'emporte sur son parent.
    for l in LEAVES {
        if let Some(a) = fx.anims.get(l.name) {
            lua += &format!("pcall(hl.animation, {})\n", a.lua(l.name, &fx.curves));
        }
    }
    lua += &format!("__hyprdesk_fx_keys = \"{}\"\n", applied_keys(fx).join(" "));
    lua
}

/// Pose les réglages ; si un réglage posé auparavant a été retiré, Hyprland
/// relit sa config et hyprdesk repose les siens à l'événement de relecture.
pub fn apply(fx: &Fx) -> Result<(), String> {
    use hyprpanel_core::ipc;
    let before = ipc::repl("__hyprdesk_fx_keys or \"\"").map_err(|e| e.to_string())?;
    let before = before.trim().trim_matches('"');
    let now = applied_keys(fx);
    if before
        .split_whitespace()
        .any(|k| !now.iter().any(|n| n == k))
    {
        ipc::eval(
            "__hyprdesk_fx_keys = nil\n\
             hl.dispatch(hl.dsp.exec_cmd(\"hyprctl reload\"))",
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if fx.is_empty() {
        return Ok(());
    }
    let r = ipc::eval(&apply_lua(fx)).map_err(|e| e.to_string())?;
    if r.trim() != "ok" && !r.trim().is_empty() {
        return Err(r);
    }
    Ok(())
}

/// Valeurs en place dans Hyprland (hyprland.lua), pour l'écran Réglages.
pub fn current() -> BTreeMap<String, String> {
    let keys: Vec<String> = OPTIONS
        .iter()
        .map(|o| format!("\"{}\"", o.path()))
        .collect();
    let lua = format!(
        "local out = {{}} for _, k in ipairs({{ {} }}) do local ok, v = pcall(hl.get_config, k) \
         if ok and v ~= nil then out[#out + 1] = k .. \"=\" .. tostring(v) end end \
         return table.concat(out, \"\\n\")",
        keys.join(", ")
    );
    hyprpanel_core::ipc::repl(&lua)
        .map(|s| {
            s.trim()
                .trim_matches('"')
                .replace("\\n", "\n")
                .lines()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_et_elements_connus() {
        let mut paths: Vec<String> = OPTIONS.iter().map(|o| o.path()).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), OPTIONS.len());
        for (_, keys) in GROUPS {
            for k in *keys {
                assert!(opt(k).is_some(), "{k}");
            }
        }
        assert!(leaf("windowsIn").is_some() && leaf("fadeDpms").is_some());
        assert!(
            LEAVES
                .iter()
                .all(|l| l.styles.iter().all(|s| style_label(s) != "?"))
        );
        let labels = OPTIONS
            .iter()
            .map(|o| o.label)
            .chain(LEAVES.iter().map(|l| l.label))
            .chain(CURVES.iter().map(|c| c.label))
            .chain(BLUR_VARIANTS.iter().map(|(_, l)| *l))
            .chain(SCANOUT.iter().map(|(_, l)| *l))
            .chain(GROUPS.iter().map(|(t, _)| *t))
            .chain(
                LEAVES
                    .iter()
                    .flat_map(|l| l.styles.iter().map(|s| style_label(s))),
            );
        let missing = crate::i18n::untranslated("fx", labels);
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }

    #[test]
    fn lua_pour_hyprland() {
        let mut fx = Fx::default();
        fx.options
            .insert("decoration.blur.size".into(), OptValue::Num(8.0));
        fx.options
            .insert("decoration.blur.variant".into(), OptValue::Num(9.0));
        fx.options
            .insert("decoration.rounding".into(), OptValue::Num(12.0));
        fx.options
            .insert("animations.enabled".into(), OptValue::Bool(true));
        fx.anims.insert(
            "windowsIn".into(),
            Anim {
                enabled: true,
                speed: 4.5,
                curve: "spring".into(),
                style: Some("popin".into()),
            },
        );
        fx.anims.insert(
            "windows".into(),
            Anim {
                enabled: true,
                speed: 5.0,
                curve: "overshoot".into(),
                style: None,
            },
        );
        fx.anims.insert(
            "borderangle".into(),
            Anim {
                enabled: false,
                speed: 1.0,
                curve: "default".into(),
                style: None,
            },
        );
        let lua = apply_lua(&fx);
        assert!(lua.contains("pcall(hl.config, { animations = { enabled = true }, decoration = { blur = { size = 8, variant = 9 }, rounding = 12 } })"), "{lua}");
        assert!(
            lua.contains("pcall(hl.curve, \"hd-spring\", { type = \"spring\", mass = 1,"),
            "{lua}"
        );
        assert!(lua.contains("pcall(hl.curve, \"hd-overshoot\", { type = \"bezier\", points = { {0.05, 0.9}, {0.1, 1.05} } })"), "{lua}");
        assert!(lua.contains("pcall(hl.animation, { leaf = \"windowsIn\", enabled = true, speed = 4.5, spring = \"hd-spring\", style = \"popin\" })"), "{lua}");
        assert!(
            lua.contains("pcall(hl.animation, { leaf = \"borderangle\", enabled = false })"),
            "{lua}"
        );
        // Parent avant enfant.
        assert!(
            lua.find("leaf = \"windows\"").unwrap() < lua.find("leaf = \"windowsIn\"").unwrap()
        );
        assert!(!lua.contains("hd-default"));
        assert!(lua.contains("__hyprdesk_fx_keys = \""));
    }

    #[test]
    fn aller_retour_lua() {
        let lua = mlua::Lua::new();
        let mut fx = Fx::default();
        fx.options
            .insert("decoration.shadow.enabled".into(), OptValue::Bool(false));
        fx.options
            .insert("decoration.blur.vibrancy".into(), OptValue::Num(0.2));
        fx.anims.insert(
            "workspaces".into(),
            Anim {
                enabled: true,
                speed: 6.0,
                curve: "quick".into(),
                style: Some("slidefade".into()),
            },
        );
        let t: mlua::Table = lua.load(format!("return {}", fx.to_lua())).eval().unwrap();
        assert_eq!(parse(&t).unwrap(), fx);
        let bad = |src: &str| {
            let t: mlua::Table = lua.load(src).eval().unwrap();
            parse(&t).is_err()
        };
        assert!(bad("return { ['decoration.inconnu'] = 1 }"));
        assert!(bad("return { ['decoration.blur.variant'] = 42 }"));
        assert!(bad(
            "return { animations = { windows = { style = 'slidevert' } } }"
        ));
        assert!(bad("return { animations = { fenetres = {} } }"));
        assert!(bad(
            "return { animations = { windows = { curve = 'magique' } } }"
        ));
    }
}
