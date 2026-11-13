//! Raccourcis essentiels de fenêtres, de workspaces et d'écrans (MLG-109).
//!
//! hyprdesk les pose lui-même, à chaud, comme ses propres raccourcis : là
//! seulement où hyprland.lua ne fait pas déjà la même chose, et sur une
//! combinaison libre. hyprland.lua n'est jamais modifié. Chacun se coupe
//! dans Réglages → Raccourcis (`keys.essentials_off`), tous d'un coup avec
//! `keys.essentials = false`. Une combinaison tenue par hyprland.lua ne se
//! pose que prise (`keys.take`) : hyprdesk la délie alors là-bas.
//!
//! Les combinaisons suivent celles de hyprland.lua quand il y en a : le
//! modificateur des workspaces (SUPER + chiffre), celui de « envoyer au
//! workspace » (SUPER + ALT + chiffre…), les touches du focus (h, j, k…)
//! pour échanger les fenêtres.

use serde_json::Value;

use crate::config::Keys;
use crate::hypr::{modmask_name, physical_key};
use crate::keymap::{Foreign, SourceBind};

/// Rubrique des Réglages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Windows,
    Workspaces,
    Screens,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Windows, Group::Workspaces, Group::Screens];

    pub fn label(self) -> &'static str {
        match self {
            Group::Windows => "Fenêtres",
            Group::Workspaces => "Workspaces",
            Group::Screens => "Écrans",
        }
    }
}

/// Un essentiel : une action, un ou plusieurs raccourcis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Essential {
    /// Identifiant (`keys.essentials_off`).
    pub id: &'static str,
    pub group: Group,
    /// Libellé, en français : traduit à l'affichage.
    pub label: &'static str,
    /// Bureau affiché aussi : l'action y a un sens (le bureau suit le
    /// workspace, l'écran).
    pub desktop: bool,
}

const fn e(id: &'static str, group: Group, label: &'static str, desktop: bool) -> Essential {
    Essential {
        id,
        group,
        label,
        desktop,
    }
}

/// Les essentiels, dans l'ordre des Réglages.
pub const CATALOGUE: &[Essential] = &[
    e("close", Group::Windows, "Fermer la fenêtre", false),
    e(
        "focus",
        Group::Windows,
        "Passer à la fenêtre voisine",
        false,
    ),
    e("move", Group::Windows, "Déplacer la fenêtre", false),
    e(
        "swap",
        Group::Windows,
        "Échanger avec la fenêtre voisine",
        false,
    ),
    e("fullscreen", Group::Windows, "Plein écran", false),
    e(
        "maximize",
        Group::Windows,
        "Maximiser (la barre reste visible)",
        false,
    ),
    e("float", Group::Windows, "Fenêtre flottante", false),
    e(
        "pin",
        Group::Windows,
        "Épingler sur tous les workspaces",
        false,
    ),
    e(
        "mouse",
        Group::Windows,
        "Déplacer et redimensionner à la souris",
        false,
    ),
    e(
        "group",
        Group::Windows,
        "Grouper les fenêtres (onglets)",
        false,
    ),
    e(
        "group-tabs",
        Group::Windows,
        "Onglet suivant / précédent du groupe",
        false,
    ),
    e("ws", Group::Workspaces, "Aller au workspace 1 à 10", false),
    e(
        "ws-move",
        Group::Workspaces,
        "Envoyer la fenêtre au workspace 1 à 10",
        false,
    ),
    e(
        "ws-step",
        Group::Workspaces,
        "Workspace suivant / précédent",
        false,
    ),
    e(
        "ws-previous",
        Group::Workspaces,
        "Workspace précédent (aller-retour)",
        true,
    ),
    e(
        "ws-wheel",
        Group::Workspaces,
        "Molette : workspace suivant / précédent",
        true,
    ),
    e(
        "mon-focus",
        Group::Screens,
        "Passer à l'écran suivant",
        true,
    ),
    e(
        "mon-window",
        Group::Screens,
        "Envoyer la fenêtre à l'écran suivant",
        false,
    ),
    e(
        "mon-workspace",
        Group::Screens,
        "Envoyer le workspace à l'écran suivant",
        false,
    ),
];

/// Précisions d'un raccourci (direction, sens), traduites à l'affichage.
#[cfg_attr(not(test), allow(dead_code))]
const DETAILS: &[&str] = &[
    "gauche",
    "droite",
    "haut",
    "bas",
    "déplacer",
    "redimensionner",
    "suivant",
    "précédent",
];

/// Libellés à traduire (catalogue anglais vérifié par `libelles_traduits`).
#[cfg_attr(not(test), allow(dead_code))]
fn labels() -> impl Iterator<Item = &'static str> {
    CATALOGUE
        .iter()
        .map(|e| e.label)
        .chain(Group::ALL.iter().map(|g| g.label()))
        .chain(DETAILS.iter().copied())
        .chain(Category::ALL.iter().map(|c| c.label()))
}

/// Ce que devient un raccourci d'un essentiel.
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// Posé par hyprdesk.
    Installed,
    /// Posé par hyprdesk sur une combinaison prise à hyprland.lua (`keys.take`).
    TakenOver,
    /// Déjà fait par hyprland.lua (ou binds.lua) : sa combinaison, le
    /// fichier, sa ligne si elle est lue.
    Present {
        combo: String,
        file: String,
        line: Option<usize>,
    },
    /// Combinaison tenue ailleurs : rien n'est posé. `inert` : le raccourci
    /// en place ne fait rien (message d'un layout inutilisé).
    Taken { by: String, inert: Option<String> },
    /// Coupé dans les réglages.
    Off,
}

impl State {
    /// hyprdesk pose ce raccourci.
    pub fn installs(&self) -> bool {
        matches!(self, State::Installed | State::TakenOver)
    }
}

/// Un raccourci d'un essentiel, résolu.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub id: &'static str,
    pub group: Group,
    /// Libellé de l'essentiel (français).
    pub label: &'static str,
    /// Précision (« 8 », « gauche »), vide s'il n'y a qu'un raccourci.
    pub detail: String,
    /// Identifiant stable du raccourci (`swap.left`, `pin`) : clé de
    /// `keys.essential_keys`. Vide pour la rangée des chiffres (non réglable).
    pub slot: String,
    /// Combinaison posée : celle choisie, sinon celle déduite.
    pub combo: String,
    /// Combinaison déduite de hyprland.lua (valeur par défaut).
    pub default: String,
    /// Dispatcher Lua.
    pub lua: String,
    /// Options de `hl.bind` en plus de la description (`mouse = true`).
    pub opts: &'static str,
    pub state: State,
    /// Posé aussi bureau affiché (submap « hyprdesk »).
    pub desktop: bool,
    /// Remarque : un raccourci de hyprland.lua pour la même action, qui ne
    /// se déclenche sans doute jamais.
    pub note: Option<String>,
}

/// Un raccourci voulu, avant résolution.
struct Want {
    detail: String,
    combo: String,
    lua: String,
    opts: &'static str,
    /// Appels de hyprland.lua qui font déjà la même chose (`keymap::call_key`) ;
    /// chaque motif : morceaux séparés par `&`, tous présents.
    calls: Vec<String>,
    /// … sur une touche de ce genre seulement (`mouse_` pour la molette).
    key_like: Option<&'static str>,
    /// Rangée des chiffres : (code physique, dispatcher de `j/binds`).
    number: Option<(u32, &'static str)>,
}

fn want(detail: &str, combo: String, lua: &str, calls: &[&str]) -> Want {
    Want {
        detail: detail.to_string(),
        combo,
        lua: lua.to_string(),
        opts: "",
        calls: calls.iter().map(|c| c.to_string()).collect(),
        key_like: None,
        number: None,
    }
}

/// Ce qu'on sait de la config en place.
struct Ctx {
    /// Modificateur des workspaces (« SUPER »).
    ws: String,
    /// Modificateur de « envoyer la fenêtre au workspace ».
    ws_move: String,
    /// Combinaisons du focus par direction (gauche, droite, haut, bas).
    focus: [Option<(u32, String)>; 4],
}

const DIRS: [(&str, &str); 4] = [
    ("left", "gauche"),
    ("right", "droite"),
    ("up", "haut"),
    ("down", "bas"),
];

fn popcount(m: u32) -> u32 {
    m.count_ones()
}

impl Ctx {
    fn read(binds: &Value, foreign: &[Foreign], source: &[SourceBind]) -> Ctx {
        let ws = crate::hypr::workspace_modifier(binds);
        let ws_mask = crate::hypr::parse_combo(&format!("{ws} + x"))
            .map(|(m, _)| m)
            .unwrap_or(64);
        // Le moins chargé des modificateurs posés sur la rangée des chiffres
        // pour envoyer une fenêtre à un workspace (SUPER + ALT plutôt que
        // SUPER + ALT + SHIFT) : d'après hyprland.lua, qui distingue un
        // workspace d'un écran ; à défaut d'après Hyprland (boucles).
        let on_row = |k: &str| physical_key(k).is_some_and(|c| (10..=19).contains(&c));
        let lowest = |masks: Vec<u32>| {
            masks
                .into_iter()
                .filter(|m| *m != ws_mask)
                .min_by_key(|m| (popcount(*m), *m))
        };
        let ws_move = lowest(
            source
                .iter()
                .filter(|s| s.call.starts_with("window.move({workspace=") && on_row(&s.combo.1))
                .map(|s| s.combo.0)
                .collect(),
        )
        .or_else(|| {
            lowest(
                foreign
                    .iter()
                    .filter(|f| f.what == "move" && on_row(&f.combo.1))
                    .map(|f| f.combo.0)
                    .collect(),
            )
        })
        .unwrap_or(ws_mask | 1);
        let focus = DIRS.map(|(d, _)| {
            source
                .iter()
                .find(|s| s.call.starts_with(&format!("focus({{direction=\"{d}\"")))
                .map(|s| s.combo.clone())
        });
        Ctx {
            ws,
            ws_move: modmask_name(ws_move as u64),
            focus,
        }
    }

    fn wants(&self, id: &str) -> Vec<Want> {
        let ws = &self.ws;
        let dir_wants = |combo: &dyn Fn(usize, &str) -> String, lua: &str, call: &str| {
            DIRS.iter()
                .enumerate()
                .map(|(i, (d, fr))| {
                    want(
                        fr,
                        combo(i, d),
                        &lua.replace("{d}", d),
                        &[&call.replace("{d}", d)],
                    )
                })
                .collect::<Vec<_>>()
        };
        match id {
            "close" => vec![want(
                "",
                "SUPER + Q".into(),
                "hl.dsp.window.close()",
                &["window.close(", "window.kill("],
            )],
            "focus" => dir_wants(
                &|_, d| format!("SUPER + {d}"),
                "hl.dsp.focus({ direction = \"{d}\" })",
                "focus({direction=\"{d}\"",
            ),
            "move" => dir_wants(
                &|_, d| format!("SUPER + SHIFT + {d}"),
                "hl.dsp.window.move({ direction = \"{d}\" })",
                "window.move({direction=\"{d}\"",
            ),
            // Les touches du focus, avec CTRL ; à défaut, les flèches.
            "swap" => dir_wants(
                &|i, d| match &self.focus[i] {
                    Some((m, k)) if m & 4 == 0 => {
                        format!("{} + {k}", modmask_name((m | 4) as u64))
                    }
                    _ => format!("SUPER + CTRL + {d}"),
                },
                "hl.dsp.window.swap({ direction = \"{d}\" })",
                "window.swap({direction=\"{d}\"",
            ),
            "fullscreen" => vec![want(
                "",
                "SUPER + F".into(),
                "hl.dsp.window.fullscreen()",
                &[
                    "window.fullscreen()",
                    "window.fullscreen(&mode=\"fullscreen\"",
                ],
            )],
            "maximize" => vec![want(
                "",
                "SUPER + SHIFT + F".into(),
                "hl.dsp.window.fullscreen({ mode = \"maximized\", action = \"toggle\" })",
                &["window.fullscreen(&mode=\"maximized\""],
            )],
            "float" => vec![want(
                "",
                "SUPER + V".into(),
                "hl.dsp.window.float({ action = \"toggle\" })",
                &["window.float("],
            )],
            "pin" => vec![want(
                "",
                "SUPER + SHIFT + P".into(),
                "hl.dsp.window.pin()",
                &["window.pin("],
            )],
            "mouse" => {
                let mut v = vec![
                    want(
                        "déplacer",
                        "SUPER + mouse:272".into(),
                        "hl.dsp.window.drag()",
                        &["window.drag("],
                    ),
                    want(
                        "redimensionner",
                        "SUPER + mouse:273".into(),
                        "hl.dsp.window.resize()",
                        &["window.resize()"],
                    ),
                ];
                for w in &mut v {
                    w.opts = "mouse = true";
                }
                v
            }
            "group" => vec![want(
                "",
                "SUPER + G".into(),
                "hl.dsp.group.toggle()",
                &["group.toggle("],
            )],
            "group-tabs" => vec![
                want(
                    "suivant",
                    "SUPER + CTRL + Tab".into(),
                    "hl.dsp.group.next()",
                    &["group.next("],
                ),
                want(
                    "précédent",
                    "SUPER + CTRL + SHIFT + Tab".into(),
                    "hl.dsp.group.prev()",
                    &["group.prev("],
                ),
            ],
            "ws" | "ws-move" => (1..=10u32)
                .map(|n| {
                    let code = 9 + n;
                    let (m, lua, dsp) = if id == "ws" {
                        (ws, format!("hl.dsp.focus({{ workspace = {n} }})"), "focus")
                    } else {
                        (
                            &self.ws_move,
                            format!("hl.dsp.window.move({{ workspace = {n} }})"),
                            "move",
                        )
                    };
                    Want {
                        detail: n.to_string(),
                        combo: format!("{m} + code:{code}"),
                        lua,
                        opts: "",
                        calls: Vec::new(),
                        key_like: None,
                        number: Some((code, dsp)),
                    }
                })
                .collect(),
            "ws-step" => vec![
                want(
                    "suivant",
                    format!("{ws} + Right"),
                    "hl.dsp.focus({ workspace = \"+1\" })",
                    &[
                        "focus({workspace=\"+1\"",
                        "focus({workspace=\"e+1\"",
                        "focus({workspace=\"r+1\"",
                        "focus({workspace=\"m+1\"",
                    ],
                ),
                want(
                    "précédent",
                    format!("{ws} + Left"),
                    "hl.dsp.focus({ workspace = \"-1\" })",
                    &[
                        "focus({workspace=\"-1\"",
                        "focus({workspace=\"e-1\"",
                        "focus({workspace=\"r-1\"",
                        "focus({workspace=\"m-1\"",
                    ],
                ),
            ],
            "ws-previous" => vec![want(
                "",
                format!("{ws} + code:49"),
                "hl.dsp.focus({ workspace = \"previous\" })",
                &["focus({workspace=\"previous"],
            )],
            "ws-wheel" => {
                let mut v = vec![
                    want(
                        "suivant",
                        format!("{ws} + mouse_down"),
                        "hl.dsp.focus({ workspace = \"e+1\" })",
                        &["focus({workspace=\"e+1\"", "focus({workspace=\"+1\""],
                    ),
                    want(
                        "précédent",
                        format!("{ws} + mouse_up"),
                        "hl.dsp.focus({ workspace = \"e-1\" })",
                        &["focus({workspace=\"e-1\"", "focus({workspace=\"-1\""],
                    ),
                ];
                for w in &mut v {
                    w.key_like = Some("mouse_");
                }
                v
            }
            // Virgule : une vraie touche en QWERTY comme en AZERTY ; ALT et
            // CTRL plutôt que SHIFT, qui change la touche sur certains claviers.
            "mon-focus" => vec![want(
                "",
                format!("{ws} + comma"),
                "hl.dsp.focus({ monitor = \"+1\" })",
                &["focus({monitor="],
            )],
            "mon-window" => vec![want(
                "",
                format!("{ws} + ALT + comma"),
                "hl.dsp.window.move({ monitor = \"+1\" })",
                &["window.move({monitor="],
            )],
            "mon-workspace" => vec![want(
                "",
                format!("{ws} + CTRL + comma"),
                "hl.dsp.workspace.move({ monitor = \"+1\" })",
                &["workspace.move({monitor=", "workspace.swap_monitors("],
            )],
            _ => Vec::new(),
        }
    }
}

/// Identifiant stable d'un raccourci d'essentiel : `swap.left`, `pin`.
/// Vide pour la rangée des chiffres, dont la combinaison suit hyprland.lua.
fn slot_of(id: &str, w: &Want) -> String {
    if w.number.is_some() {
        return String::new();
    }
    let sub = match w.detail.as_str() {
        "" => return id.to_string(),
        "gauche" => "left",
        "droite" => "right",
        "haut" => "up",
        "bas" => "down",
        "suivant" => "next",
        "précédent" => "prev",
        "déplacer" => "drag",
        "redimensionner" => "resize",
        other => other,
    };
    format!("{id}.{sub}")
}

/// Identifiants réglables (`keys.essential_keys`), dans l'ordre des Réglages.
pub fn slots() -> Vec<String> {
    let ctx = Ctx {
        ws: "SUPER".into(),
        ws_move: "SUPER + ALT".into(),
        focus: [None, None, None, None],
    };
    CATALOGUE
        .iter()
        .flat_map(|e| {
            ctx.wants(e.id)
                .into_iter()
                .map(|w| slot_of(e.id, &w))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Même touche : identique, ou même touche physique (`1`, `ampersand`, `code:10`).
fn same_key(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
        || matches!((physical_key(a), physical_key(b)), (Some(x), Some(y)) if x == y)
}

fn same_combo(a: &(u32, String), b: &(u32, String)) -> bool {
    a.0 == b.0 && same_key(&a.1, &b.1)
}

/// Un motif d'appel (`window.fullscreen(&mode="maximized"`) dans `call`.
fn call_matches(call: &str, pattern: &str) -> bool {
    let mut parts = pattern.split('&');
    let first = parts.next().unwrap_or("");
    call.starts_with(first) && parts.all(|p| call.contains(p))
}

/// Clavier où les chiffres ne sont pas sur la rangée sans Maj (AZERTY) : un
/// raccourci de hyprland.lua y est écrit avec ses touches (`ampersand`…).
fn digits_shifted(foreign: &[Foreign]) -> bool {
    const AZERTY: &[&str] = &[
        "ampersand",
        "eacute",
        "quotedbl",
        "apostrophe",
        "parenleft",
        "egrave",
        "underscore",
        "ccedilla",
        "agrave",
    ];
    foreign.iter().any(|f| AZERTY.contains(&f.combo.1.as_str()))
}

/// Combinaison lisible : (64, "code:49") → « SUPER + code:49 ».
fn combo_text(c: &(u32, String)) -> String {
    let m = modmask_name(c.0 as u64);
    if m.is_empty() {
        c.1.clone()
    } else {
        format!("{m} + {}", c.1)
    }
}

/// Message de layout (`layout("move +col")`) d'un layout inutilisé : lequel.
fn inert_layout(call: &str, layouts: &[String]) -> Option<String> {
    let msg = call.strip_prefix("layout(\"")?.split('"').next()?;
    let owner = crate::tiling::ACTIONS
        .iter()
        .find(|a| a.dsp == "layout" && a.arg.replace(' ', "") == msg)?
        .layout;
    (!owner.is_empty() && !layouts.iter().any(|l| l == owner)).then(|| owner.to_string())
}

/// Les essentiels face à Hyprland (`j/binds`), à hyprland.lua et aux
/// raccourcis de hyprdesk. `layouts` : les layouts en usage (dwindle…).
pub fn plan(keys: &Keys, binds: &Value, source: &[SourceBind], layouts: &[String]) -> Vec<Line> {
    let foreign = crate::keymap::foreign(binds);
    let ctx = Ctx::read(binds, &foreign, source);
    let shifted = digits_shifted(&foreign);
    // Raccourcis de hyprdesk : hors plein bureau et bureau affiché.
    let mine: Vec<((u32, String), bool, bool, &'static str)> = crate::keymap::actions(keys)
        .into_iter()
        .filter_map(|a| {
            crate::keymap::norm(&a.combo)
                .map(|n| (n, a.scope.outside(), a.scope.desktop(), a.label))
        })
        .collect();
    let taken: Vec<(u32, String)> = keys
        .take
        .iter()
        .filter_map(|t| crate::keymap::norm(t))
        .collect();
    let mut out: Vec<Line> = Vec::new();
    for ess in CATALOGUE {
        let off = !keys.essentials || keys.essentials_off.iter().any(|o| o == ess.id);
        for mut w in ctx.wants(ess.id) {
            let slot = slot_of(ess.id, &w);
            let default = w.combo.clone();
            if let Some(c) = keys
                .essential_keys
                .get(&slot)
                .filter(|c| !slot.is_empty() && crate::hypr::parse_combo(c).is_some())
            {
                w.combo = c.clone();
            }
            let Some(n) = crate::keymap::norm(&w.combo) else {
                continue;
            };
            // Déjà fait par hyprland.lua : sur la rangée des chiffres, d'après
            // Hyprland (boucles comprises) ; sinon d'après les appels lus.
            let present = if let Some((code, dsp)) = w.number {
                foreign
                    .iter()
                    .find(|f| f.what == dsp && physical_key(&f.combo.1) == Some(code))
                    .map(|f| {
                        let at = source
                            .iter()
                            .find(|s| same_combo(&s.combo, &f.combo))
                            .map(|s| (s.file.clone(), s.line));
                        (f.combo.clone(), at)
                    })
            } else {
                source
                    .iter()
                    .filter(|s| w.key_like.is_none_or(|k| s.combo.1.starts_with(k)))
                    .find(|s| w.calls.iter().any(|p| call_matches(&s.call, p)))
                    .map(|s| (s.combo.clone(), Some((s.file.clone(), s.line))))
            };
            let holder = foreign.iter().find(|f| same_combo(&f.combo, &n));
            // Un chiffre écrit tel quel sur un clavier AZERTY : Hyprland compare
            // la touche sans Maj (« & »), ce raccourci ne se déclenche jamais.
            // hyprdesk pose quand même le sien, et le dit.
            let mut note = None;
            let present = present.filter(|(c, at)| {
                let dead = shifted && c.1.len() == 1 && c.1.chars().all(|ch| ch.is_ascii_digit());
                if dead {
                    let (file, line) = match at {
                        Some((f, n)) => (f.clone(), Some(*n)),
                        None => ("hyprland.lua".to_string(), None),
                    };
                    let at = match line {
                        Some(n) => crate::tr!("{c} (ligne {n})", c = combo_text(c), n = n),
                        None => combo_text(c),
                    };
                    note = Some(crate::tr!(
                        "{f} a {at}, qui ne se déclenche sans doute jamais : sur ce clavier, la touche {k} donne un autre caractère sans Maj",
                        f = file,
                        at = at,
                        k = c.1
                    ));
                }
                !dead
            });
            let state = if let Some((c, at)) = present {
                State::Present {
                    combo: combo_text(&c),
                    file: at
                        .as_ref()
                        .map(|(f, _)| f.clone())
                        .unwrap_or_else(|| "hyprland.lua".into()),
                    line: at.map(|(_, n)| n),
                }
            } else if off {
                State::Off
            } else if let Some((_, _, _, label)) = mine
                .iter()
                .find(|(m, outside, _, _)| *outside && same_combo(m, &n))
            {
                State::Taken {
                    by: crate::tr!("hyprdesk : {l}", l = crate::i18n::tr(label)),
                    inert: None,
                }
            } else if let Some(o) = out.iter().find(|l| {
                l.state.installs()
                    && crate::keymap::norm(&l.combo).is_some_and(|c| same_combo(&c, &n))
            }) {
                State::Taken {
                    by: crate::i18n::tr(o.label).to_string(),
                    inert: None,
                }
            } else if let Some(f) = holder {
                if taken.iter().any(|t| same_combo(t, &n)) {
                    State::TakenOver
                } else {
                    let src = source.iter().find(|s| same_combo(&s.combo, &f.combo));
                    let h = crate::keymap::Holder {
                        what: src
                            .map(|s| s.action.clone())
                            .unwrap_or_else(|| f.what.clone()),
                        line: src.map(|s| s.line),
                        file: src.map(|s| s.file.clone()).unwrap_or_default(),
                    };
                    State::Taken {
                        by: h.describe(),
                        inert: src.and_then(|s| inert_layout(&s.call, layouts)).map(|l| {
                            crate::tr!(
                                "il envoie un message du layout {l}, qui ne sert nulle part ici",
                                l = l
                            )
                        }),
                    }
                }
            } else {
                State::Installed
            };
            // Bureau affiché : pas sur un raccourci universel de hyprland.lua
            // (il y agit déjà), ni sur un raccourci de hyprdesk du bureau.
            let desktop = ess.desktop
                && state.installs()
                && !holder.is_some_and(|f| f.universal)
                && !mine.iter().any(|(m, _, d, _)| *d && same_combo(m, &n));
            out.push(Line {
                id: ess.id,
                group: ess.group,
                label: ess.label,
                detail: w.detail,
                slot,
                combo: w.combo,
                default,
                lua: w.lua,
                opts: w.opts,
                state,
                desktop,
                note,
            });
        }
    }
    out
}

impl State {
    /// État lisible (traduit).
    pub fn describe(&self) -> String {
        match self {
            State::Installed => crate::i18n::tr("posé par hyprdesk").to_string(),
            State::TakenOver => {
                crate::i18n::tr("posé par hyprdesk, pris à hyprland.lua").to_string()
            }
            State::Present { combo, file, line } => match line {
                Some(n) => crate::tr!(
                    "dans {f} : {c} (ligne {n})",
                    f = file,
                    c = pretty(combo),
                    n = n
                ),
                None => crate::tr!("dans {f} : {c}", f = file, c = pretty(combo)),
            },
            State::Taken { by, inert } => match inert {
                Some(i) => crate::tr!(
                    "combinaison prise par {by}, inopérant : {i}",
                    by = by,
                    i = i
                ),
                None => crate::tr!("combinaison prise par {by}", by = by),
            },
            State::Off => crate::i18n::tr("coupé").to_string(),
        }
    }
}

/// Libellé d'une ligne : « Déplacer la fenêtre (gauche) ».
pub fn line_label(l: &Line) -> String {
    let label = crate::i18n::tr(l.label);
    if l.detail.is_empty() {
        label.to_string()
    } else if l.detail.chars().all(|c| c.is_ascii_digit()) {
        crate::tr!("{label} — {d}", label = label, d = l.detail)
    } else {
        format!("{label} ({})", crate::i18n::tr(&l.detail))
    }
}

/// `hyprdesk keys` : les essentiels et ce qu'ils deviennent.
pub fn text(lines: &[Line]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut o = crate::i18n::tr("\nEssentiels (fenêtres, workspaces, écrans)\n").to_string();
    for g in Group::ALL {
        o += &format!("  {}\n", crate::i18n::tr(g.label()));
        for l in lines.iter().filter(|l| l.group == g) {
            let combo = match &l.state {
                State::Present { combo, .. } => combo.clone(),
                _ => l.combo.clone(),
            };
            o += &format!(
                "    {:<28} {} — {}{}\n",
                combo,
                line_label(l),
                l.state.describe(),
                if l.desktop {
                    crate::i18n::tr(", bureau affiché aussi")
                } else {
                    ""
                }
            );
            if let Some(n) = &l.note {
                o += &format!("    {:<28} ({n})\n", "");
            }
        }
    }
    o
}

/// Combinaison lisible, telle qu'on la tape : `SUPER + code:17` → « SUPER + 8 »,
/// `ampersand` → « & », `mouse_down` → « molette ↓ ».
pub fn pretty(combo: &str) -> String {
    let parts: Vec<&str> = combo
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let Some((key, mods)) = parts.split_last() else {
        return combo.to_string();
    };
    let k = key.to_lowercase();
    let row = "1234567890";
    let shown: String = if let Some(c) = k
        .strip_prefix("code:")
        .and_then(|c| c.parse::<usize>().ok())
    {
        match c {
            10..=19 => row[c - 10..c - 9].to_string(),
            49 => "²".into(),
            _ => key.to_string(),
        }
    } else {
        match k.as_str() {
            "ampersand" => "&".into(),
            "eacute" => "é".into(),
            "quotedbl" => "\"".into(),
            "apostrophe" => "'".into(),
            "parenleft" => "(".into(),
            "parenright" => ")".into(),
            "minus" => "-".into(),
            "egrave" => "è".into(),
            "underscore" => "_".into(),
            "ccedilla" => "ç".into(),
            "agrave" => "à".into(),
            "comma" => ",".into(),
            "semicolon" => ";".into(),
            "period" => ".".into(),
            "equal" => "=".into(),
            "twosuperior" => "²".into(),
            "grave" => "`".into(),
            "left" => "←".into(),
            "right" => "→".into(),
            "up" => "↑".into(),
            "down" => "↓".into(),
            "space" => crate::i18n::tr("Espace").into(),
            "return" => "⏎".into(),
            "escape" => crate::i18n::tr("Échap").into(),
            "print" => crate::i18n::tr("Impr. écran").into(),
            "mouse_down" => crate::i18n::tr("molette ↓").into(),
            "mouse_up" => crate::i18n::tr("molette ↑").into(),
            "mouse:272" => crate::i18n::tr("clic gauche").into(),
            "mouse:273" => crate::i18n::tr("clic droit").into(),
            "mouse:274" => crate::i18n::tr("clic milieu").into(),
            _ if k.chars().count() == 1 => k.to_uppercase(),
            _ => key.to_string(),
        }
    };
    let mut out: Vec<String> = mods.iter().map(|m| m.to_uppercase()).collect();
    out.push(shown);
    out.join(" + ")
}

/// L'action d'un appel de hyprland.lua, si c'est un essentiel connu :
/// `window.close()` → « Fermer la fenêtre ».
pub fn known_action(call: &str) -> Option<String> {
    let num = |rest: &str| -> Option<u32> {
        let d: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        d.parse().ok()
    };
    if let Some(n) = call.strip_prefix("focus({workspace=").and_then(num) {
        return Some(crate::tr!("Aller au workspace {n}", n = n));
    }
    if let Some(n) = call.strip_prefix("window.move({workspace=").and_then(num) {
        return Some(crate::tr!("Envoyer la fenêtre au workspace {n}", n = n));
    }
    let ctx = Ctx {
        ws: "SUPER".into(),
        ws_move: "SUPER + ALT".into(),
        focus: [None, None, None, None],
    };
    CATALOGUE.iter().find_map(|e| {
        ctx.wants(e.id).into_iter().find_map(|w| {
            w.calls.iter().any(|p| call_matches(call, p)).then(|| {
                if w.detail.is_empty() {
                    crate::i18n::tr(e.label).to_string()
                } else {
                    format!(
                        "{} ({})",
                        crate::i18n::tr(e.label),
                        crate::i18n::tr(&w.detail)
                    )
                }
            })
        })
    })
}

/// Combinaisons de `keys.take` qui servent à un essentiel posé.
pub fn taken_combos(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .filter(|l| l.state == State::TakenOver)
        .map(|l| l.combo.clone())
        .collect()
}

/// Rubrique d'un raccourci de Hyprland.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Windows,
    Workspaces,
    Screens,
    Apps,
    Captures,
    Media,
    Hyprdesk,
    Other,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::Windows,
        Category::Workspaces,
        Category::Screens,
        Category::Apps,
        Category::Captures,
        Category::Media,
        Category::Hyprdesk,
        Category::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Windows => "Fenêtres",
            Category::Workspaces => "Workspaces",
            Category::Screens => "Écrans",
            Category::Apps => "Applications et scripts",
            Category::Captures => "Captures d'écran",
            Category::Media => "Volume, luminosité, lecture",
            Category::Hyprdesk => "Bureau hyprdesk",
            Category::Other => "Autres",
        }
    }
}

/// D'où vient un raccourci actif.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    /// hyprland.lua (ligne, si elle est lue).
    /// Un fichier de la config de Hyprland : (fichier, ligne) s'il est lu.
    HyprlandLua(Option<(String, usize)>),
    /// Posé par hyprdesk.
    Hyprdesk,
}

/// Un raccourci actif dans Hyprland, dans un mode donné.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    pub combo: String,
    /// Ce qu'il fait : la description de hyprdesk, la commande lancée,
    /// l'appel (`focus({workspace=1})`), sinon le dispatcher.
    pub what: String,
    pub origin: Origin,
    /// Combinaison lue dans le fichier (raccourci de la config de Hyprland
    /// retrouvé à sa ligne) : de quoi le modifier (MLG-116).
    pub bind: Option<(u32, String)>,
    pub category: Category,
    /// Actif hors plein bureau (no-desktop, launcher-only).
    pub outside: bool,
    /// Actif bureau affiché (full-desktop).
    pub desktop: bool,
}

fn category(key: &str, what: &str, call: &str) -> Category {
    let k = key.to_lowercase();
    if k.starts_with("xf86") {
        Category::Media
    } else if k == "print" || call.contains("hyprshot") || call.contains("grim") {
        Category::Captures
    } else if what == "exec_cmd" || call.starts_with("exec_cmd") || call.starts_with("function") {
        Category::Apps
    } else if call.contains("monitor=") || what.contains("monitor") {
        Category::Screens
    } else if call.contains("workspace") || what.contains("special") || what.contains("workspace") {
        Category::Workspaces
    } else if [
        "close",
        "kill",
        "fullscreen",
        "float",
        "pseudo",
        "pin",
        "move",
        "resize",
        "drag",
        "swap",
        "center",
        "focus",
        "toggle",
        "layout",
        "next",
        "prev",
        "cycle_next",
        "alter_zorder",
    ]
    .iter()
    .any(|w| what == *w || what.ends_with(&format!(".{w}")))
    {
        Category::Windows
    } else {
        Category::Other
    }
}

/// Rubrique d'un raccourci de hyprdesk, d'après sa description.
fn hyprdesk_category(desc: &str) -> Category {
    let d = desc.trim_start_matches("hyprdesk : ");
    if let Some(e) = CATALOGUE.iter().find(|e| d.starts_with(e.label)) {
        return match e.group {
            Group::Windows => Category::Windows,
            Group::Workspaces => Category::Workspaces,
            Group::Screens => Category::Screens,
        };
    }
    if d.contains("workspace") {
        Category::Workspaces
    } else if d.contains("multimédia") {
        Category::Media
    } else {
        Category::Hyprdesk
    }
}

/// Tous les raccourcis actifs (`j/binds`, après l'installation) : ceux de
/// hyprland.lua avec leur ligne, ceux de hyprdesk avec leur description ;
/// pour chacun, s'il agit hors plein bureau et bureau affiché.
pub fn inventory(binds: &Value, source: &[SourceBind]) -> Vec<Listed> {
    let mut out: Vec<Listed> = Vec::new();
    for b in binds.as_array().into_iter().flatten() {
        let submap = b["submap"].as_str().unwrap_or("");
        let inside = submap == crate::hypr::SUBMAP;
        if !submap.is_empty() && !inside {
            continue;
        }
        let (Some(mask), Some(key)) = (b["modmask"].as_u64(), crate::hypr::bind_key(b)) else {
            continue;
        };
        let combo = (mask as u32, key);
        // Affichée telle qu'écrite (casse d'origine), lisible.
        let shown = {
            let raw = b["key"]
                .as_str()
                .filter(|k| !k.is_empty())
                .map(str::to_string);
            let key = raw.unwrap_or_else(|| combo.1.clone());
            let m = modmask_name(mask);
            pretty(&if m.is_empty() {
                key
            } else {
                format!("{m} + {key}")
            })
        };
        let dsp = b["dispatcher"]
            .as_str()
            .unwrap_or("?")
            .replace("HL.Dispatcher(", "")
            .replace(')', "");
        let desc = b["description"].as_str().unwrap_or("");
        let universal = matches!(&b["submap_universal"], Value::Bool(true))
            || b["submap_universal"].as_str() == Some("true");
        let l = if desc.starts_with("hyprdesk") {
            Listed {
                combo: shown,
                what: desc.trim_start_matches("hyprdesk : ").to_string(),
                origin: Origin::Hyprdesk,
                bind: None,
                category: hyprdesk_category(desc),
                outside: !inside,
                desktop: inside || universal,
            }
        } else {
            let src = source.iter().find(|s| same_combo(&s.combo, &combo));
            let call = src
                .map(|s| s.call.trim_end_matches(')').to_string())
                .unwrap_or_default();
            let what = match src {
                Some(s) if s.action == "function" => {
                    crate::i18n::tr("une fonction Lua").to_string()
                }
                Some(s) if s.call.starts_with("exec_cmd") => s.action.clone(),
                Some(s) if known_action(&s.call).is_some() => {
                    known_action(&s.call).unwrap_or_default()
                }
                // Appel refermé : `focus({workspace=1})`.
                Some(_) => {
                    let open = call.matches('(').count();
                    let close = call.matches(')').count();
                    format!("{call}{}", ")".repeat(open.saturating_sub(close)))
                        .chars()
                        .take(80)
                        .collect()
                }
                None => dsp.clone(),
            };
            Listed {
                combo: shown,
                category: category(&combo.1, &dsp, &call),
                what,
                origin: Origin::HyprlandLua(src.map(|s| (s.file.clone(), s.line))),
                bind: src.map(|s| s.combo.clone()),
                outside: !inside,
                desktop: inside || universal,
            }
        };
        out.push(l);
    }
    let line = |l: &Listed| match &l.origin {
        // binds.lua après hyprland.lua.
        Origin::HyprlandLua(Some((f, n))) => {
            *n + if f.as_str() == "hyprland.lua" {
                0
            } else {
                1_000_000
            }
        }
        Origin::HyprlandLua(None) => usize::MAX - 1,
        Origin::Hyprdesk => usize::MAX,
    };
    out.sort_by(|a, b| (a.category, line(a), &a.combo).cmp(&(b.category, line(b), &b.combo)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(lua: &str) -> Vec<SourceBind> {
        crate::keymap::parse_source(lua)
    }

    fn find<'a>(lines: &'a [Line], id: &str, detail: &str) -> &'a Line {
        lines
            .iter()
            .find(|l| l.id == id && l.detail == detail)
            .unwrap_or_else(|| panic!("{id} {detail} absent"))
    }

    /// La config de Jacques : AZERTY, workspaces 1 à 7, molette du layout
    /// scrolling en dwindle, envoi vers un écran sur SHIFT + 1.
    #[test]
    fn essentiels_face_a_hyprland_lua() {
        let lua = r#"
local mainMod = "SUPER"
hl.bind(mainMod .. " + Q",         hl.dsp.window.close())
hl.bind(mainMod .. " + F",         hl.dsp.window.fullscreen())
hl.bind(mainMod .. " + h", hl.dsp.focus({ direction = "left" }))
hl.bind(mainMod .. " + m", hl.dsp.focus({ direction = "right" }))
hl.bind(mainMod .. " + ampersand", hl.dsp.focus({ workspace = 1 }))
hl.bind(mainMod .. " + ALT + ampersand", hl.dsp.window.move({ workspace = 1 }))
hl.bind(mainMod .. " + Right",     hl.dsp.focus({ workspace = "+1" }))
hl.bind(mainMod .. " + SHIFT + 1", hl.dsp.window.move({ monitor = "HDMI-A-2" }))
hl.bind(mainMod .. " + mouse_down", hl.dsp.layout("move +col"))
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true })
"#;
        let binds = serde_json::json!([
            { "submap": "", "dispatcher": "HL.Dispatcher(close)", "key": "Q", "modmask": 64 },
            { "submap": "", "dispatcher": "HL.Dispatcher(focus)", "key": "ampersand", "modmask": 64 },
            { "submap": "", "dispatcher": "HL.Dispatcher(move)", "key": "ampersand", "modmask": 72 },
            { "submap": "", "dispatcher": "HL.Dispatcher(move)", "key": "", "keycode": 10, "modmask": 73 },
            { "submap": "", "dispatcher": "HL.Dispatcher(move)", "key": "1", "modmask": 65 },
            { "submap": "", "dispatcher": "HL.Dispatcher(layout)", "key": "mouse_down", "modmask": 64 },
            { "submap": "", "dispatcher": "HL.Dispatcher(focus)", "key": "Right", "modmask": 64 },
            { "submap": "", "dispatcher": "HL.Dispatcher(focus)", "key": "h", "modmask": 64 },
        ]);
        let keys = Keys::default();
        let lines = plan(&keys, &binds, &src(lua), &["dwindle".into()]);
        // Workspace 1 : SUPER + & déjà là ; 8 : posé, même modificateur.
        assert!(matches!(
            find(&lines, "ws", "1").state,
            State::Present { .. }
        ));
        let ws8 = find(&lines, "ws", "8");
        assert_eq!(ws8.state, State::Installed);
        assert_eq!(ws8.combo, "SUPER + code:17");
        // Envoyer au workspace : le modificateur de hyprland.lua (SUPER + ALT).
        assert_eq!(find(&lines, "ws-move", "8").combo, "SUPER + ALT + code:17");
        // Échanger : les touches du focus, avec CTRL.
        assert_eq!(find(&lines, "swap", "gauche").combo, "SUPER + CTRL + h");
        assert_eq!(find(&lines, "swap", "droite").combo, "SUPER + CTRL + m");
        // Envoyer à l'écran : SHIFT + 1 en AZERTY est mort, hyprdesk pose le sien.
        let mon = find(&lines, "mon-window", "");
        assert_eq!(mon.state, State::Installed);
        assert!(
            mon.note
                .as_deref()
                .unwrap_or("")
                .contains("SUPER + SHIFT + 1")
        );
        // Molette : tenue par un message du layout scrolling, inutilisé.
        match &find(&lines, "ws-wheel", "suivant").state {
            State::Taken { inert, .. } => {
                assert!(inert.as_deref().unwrap_or("").contains("scrolling"))
            }
            s => panic!("{s:?}"),
        }
        // Workspace précédent : posé, bureau affiché aussi (le bureau suit).
        let prev = find(&lines, "ws-previous", "");
        assert_eq!(prev.state, State::Installed);
        assert!(prev.desktop, "bureau affiché aussi");
        // Maximiser, épingler : posés.
        assert_eq!(find(&lines, "maximize", "").state, State::Installed);
        assert!(!find(&lines, "maximize", "").desktop);
        // Souris : déjà là pour déplacer.
        assert!(matches!(
            find(&lines, "mouse", "déplacer").state,
            State::Present { .. }
        ));
    }

    #[test]
    fn coupes_et_prises() {
        let binds = serde_json::json!([
            { "submap": "", "dispatcher": "HL.Dispatcher(layout)", "key": "mouse_down", "modmask": 64 },
        ]);
        let mut keys = Keys {
            essentials_off: vec!["pin".into()],
            take: vec!["SUPER + mouse_down".into()],
            ..Default::default()
        };
        let lines = plan(&keys, &binds, &[], &[]);
        assert_eq!(find(&lines, "pin", "").state, State::Off);
        assert_eq!(find(&lines, "ws-wheel", "suivant").state, State::TakenOver);
        assert_eq!(taken_combos(&lines), ["SUPER + mouse_down"]);
        keys.essentials = false;
        let lines = plan(&keys, &binds, &[], &[]);
        assert!(lines.iter().all(|l| !l.state.installs()));
    }

    /// Une combinaison choisie remplace celle déduite ; la déduite reste la
    /// valeur par défaut.
    #[test]
    fn combinaison_choisie() {
        let mut keys = Keys::default();
        keys.essential_keys
            .insert("swap.left".into(), "SUPER + ALT + h".into());
        let lines = plan(&keys, &Value::Null, &[], &[]);
        let l = find(&lines, "swap", "gauche");
        assert_eq!(l.slot, "swap.left");
        assert_eq!(l.combo, "SUPER + ALT + h");
        assert_eq!(l.default, "SUPER + CTRL + left");
        assert_eq!(find(&lines, "pin", "").slot, "pin");
        assert_eq!(find(&lines, "ws", "8").slot, "");
        assert!(slots().contains(&"group-tabs.prev".to_string()));
        assert!(!slots().iter().any(|s| s.starts_with("ws.") || s == "ws"));
    }

    /// Une combinaison de hyprdesk n'est jamais doublée par un essentiel.
    #[test]
    fn jamais_sur_un_raccourci_de_hyprdesk() {
        let keys = Keys {
            overview: "SUPER + G".into(),
            ..Default::default()
        };
        let lines = plan(&keys, &Value::Null, &[], &[]);
        assert!(matches!(
            find(&lines, "group", "").state,
            State::Taken { .. }
        ));
    }

    /// Inventaire : hyprland.lua hors plein bureau (sauf universel), la submap
    /// de hyprdesk bureau affiché.
    #[test]
    fn inventaire_par_mode() {
        let lua = r#"
local mainMod = "SUPER"
hl.bind("Print", hl.dsp.exec_cmd("hyprshot -m region"), { submap_universal = true })
hl.bind(mainMod .. " + ampersand", hl.dsp.focus({ workspace = 1 }))
"#;
        let binds = serde_json::json!([
            { "submap": "", "dispatcher": "HL.Dispatcher(exec_cmd)", "key": "Print", "modmask": 0, "submap_universal": "true" },
            { "submap": "", "dispatcher": "HL.Dispatcher(focus)", "key": "ampersand", "modmask": 64 },
            { "submap": "hyprdesk", "dispatcher": "HL.Dispatcher(focus)", "key": "", "keycode": 10, "modmask": 64, "description": "hyprdesk : workspace 1" },
            { "submap": "", "dispatcher": "HL.Dispatcher(exec_cmd)", "key": "D", "modmask": 64, "description": "hyprdesk : changer de mode" },
            { "submap": "autre", "dispatcher": "HL.Dispatcher(exec_cmd)", "key": "X", "modmask": 0 },
        ]);
        let inv = inventory(&binds, &src(lua));
        assert_eq!(inv.len(), 4, "{inv:?}");
        let print = inv.iter().find(|l| l.combo == "Impr. écran").unwrap();
        assert!(print.outside && print.desktop);
        assert_eq!(print.category, Category::Captures);
        assert_eq!(
            print.origin,
            Origin::HyprlandLua(Some(("hyprland.lua".into(), 3)))
        );
        let ws1 = inv.iter().find(|l| l.combo == "SUPER + &").unwrap();
        assert!(ws1.outside && !ws1.desktop);
        assert_eq!(ws1.what, "Aller au workspace 1");
        let sub = inv.iter().find(|l| l.combo == "SUPER + 1").unwrap();
        assert!(!sub.outside && sub.desktop && sub.origin == Origin::Hyprdesk);
        assert_eq!(sub.category, Category::Workspaces);
        let d = inv.iter().find(|l| l.combo == "SUPER + D").unwrap();
        assert_eq!(d.category, Category::Hyprdesk);
    }

    #[test]
    fn combinaisons_lisibles() {
        assert_eq!(pretty("SUPER + code:17"), "SUPER + 8");
        assert_eq!(pretty("SUPER + ALT + code:19"), "SUPER + ALT + 0");
        assert_eq!(pretty("SUPER + code:49"), "SUPER + ²");
        assert_eq!(pretty("super + ampersand"), "SUPER + &");
        assert_eq!(pretty("SUPER + SHIFT + left"), "SUPER + SHIFT + ←");
        assert_eq!(pretty("SUPER + q"), "SUPER + Q");
        assert_eq!(pretty("XF86AudioMute"), "XF86AudioMute");
        assert_eq!(
            known_action("window.close())").as_deref(),
            Some("Fermer la fenêtre")
        );
        assert_eq!(
            known_action("focus({direction=\"left\"}))").as_deref(),
            Some("Passer à la fenêtre voisine (gauche)")
        );
        assert_eq!(known_action("layout(\"move+col\"))"), None);
    }

    #[test]
    fn libelles_traduits() {
        let missing = crate::i18n::untranslated("essentials", labels());
        assert!(missing.is_empty(), "{missing:?}");
    }
}
