//! Listes de Hyprland éditées dans les Réglages : règles (fenêtres, couches,
//! workspaces), gestes du pavé tactile (`hl.gesture`), réglages par
//! périphérique (`hl.device`) et permissions (`hl.permission`). Chaque
//! élément est une carte de champs ; un champ non proposé ici est gardé tel
//! quel. Les éléments incomplets (sans nom, sans workspace…) restent dans
//! les Réglages mais ne sont pas écrits (`hyprset::complete`).

use iced::widget::{Space, button, column, container, pick_list, row, text_input, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::hyprset::{Msg, wrap};
use super::{App, Msg as UiMsg, color};
use crate::hyprset::{Hypr, Permission, RuleKind};
use crate::i18n::text;

/// Liste éditable de `Hypr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Rules(RuleKind),
    Gestures,
    Devices,
}

/// Nature d'un champ.
#[derive(Clone, Copy)]
pub enum Field {
    /// Chaîne.
    Text,
    /// Nombre (vide : retiré).
    Num,
    /// Nombre si la saisie en est un, sinon chaîne (`opacity`).
    TextOrNum,
    /// Interrupteur : coupé = champ retiré.
    Bool,
    /// Trois états : comme ailleurs (retiré), oui, non.
    Bool3,
    /// Liste de chaînes (première : retiré si sa valeur est vide).
    Choice(&'static [(&'static str, &'static str)]),
    /// Liste de nombres.
    IntChoice(&'static [(i64, &'static str)]),
}

/// Champ proposé : (chemin pointé, libellé, nature, affiché seulement si
/// le champ `when.0` vaut l'une des valeurs `when.1`).
pub type Spec = (
    &'static str,
    &'static str,
    Field,
    Option<(&'static str, &'static [&'static str])>,
);

const IDLE: &[(&str, &str)] = &[
    ("", "Non"),
    ("focus", "Quand elle est active"),
    ("fullscreen", "En plein écran"),
    ("always", "Toujours"),
];

/// Règles de fenêtres.
pub const WINDOW_FIELDS: &[Spec] = &[
    ("match.class", "Classe (regex)", Field::Text, None),
    ("match.title", "Titre (regex)", Field::Text, None),
    ("float", "Flottante", Field::Bool, None),
    ("pin", "Épinglée sur tous les workspaces", Field::Bool, None),
    ("no_blur", "Sans flou", Field::Bool, None),
    (
        "opacity",
        "Opacité (0 à 1, ou « active inactive »)",
        Field::TextOrNum,
        None,
    ),
    ("border_color", "Couleur de bordure", Field::Text, None),
    ("size", "Taille (« 800 600 »)", Field::Text, None),
    (
        "idle_inhibit",
        "Empêche la veille",
        Field::Choice(IDLE),
        None,
    ),
    ("workspace", "Workspace", Field::Text, None),
];

/// Règles de couches (barres, lanceurs, notifications…).
pub const LAYER_FIELDS: &[Spec] = &[
    (
        "match.namespace",
        "Espace de noms (regex)",
        Field::Text,
        None,
    ),
    ("blur", "Flou derrière", Field::Bool, None),
    ("blur_popups", "Flou derrière ses menus", Field::Bool, None),
    (
        "ignore_alpha",
        "Pas de flou sous cette opacité (0 à 1)",
        Field::Num,
        None,
    ),
    (
        "xray",
        "Flou du seul fond d'écran (xray)",
        Field::Bool,
        None,
    ),
    ("dim_around", "Assombrir autour", Field::Bool, None),
    ("no_anim", "Sans animation", Field::Bool, None),
    (
        "animation",
        "Animation (slide, popin, fade…)",
        Field::Text,
        None,
    ),
    (
        "no_screen_share",
        "Masquée au partage d'écran",
        Field::Bool,
        None,
    ),
    (
        "above_lock",
        "Au-dessus de l'écran de verrouillage",
        Field::Bool,
        None,
    ),
    ("order", "Ordre d'empilement", Field::Num, None),
];

/// Règles de workspaces.
pub const WORKSPACE_FIELDS: &[Spec] = &[
    (
        "workspace",
        "Workspace (1, name:web, m[DP-1]…)",
        Field::Text,
        None,
    ),
    ("monitor", "Écran", Field::Text, None),
    (
        "default",
        "Workspace par défaut de l'écran",
        Field::Bool,
        None,
    ),
    ("persistent", "Toujours présent", Field::Bool, None),
    ("default_name", "Nom affiché", Field::Text, None),
    (
        "layout",
        "Layout (dwindle, master, scrolling, monocle)",
        Field::Text,
        None,
    ),
    ("gaps_in", "Espace entre fenêtres", Field::Num, None),
    ("gaps_out", "Espace au bord de l'écran", Field::Num, None),
    ("border_size", "Épaisseur des bordures", Field::Num, None),
    ("no_rounding", "Sans arrondi", Field::Bool, None),
    ("no_shadow", "Sans ombre", Field::Bool, None),
    ("no_border", "Sans bordure", Field::Bool, None),
    (
        "on_created_empty",
        "Commande à l'ouverture du workspace vide",
        Field::Text,
        None,
    ),
];

const FINGERS: &[(i64, &str)] = &[(3, "3 doigts"), (4, "4 doigts"), (5, "5 doigts")];

/// Sens d'un geste (Hyprland : swipe, horizontal… pinch).
pub const DIRECTIONS: &[(&str, &str)] = &[
    ("horizontal", "Horizontal (gauche ou droite)"),
    ("vertical", "Vertical (haut ou bas)"),
    ("left", "Vers la gauche"),
    ("right", "Vers la droite"),
    ("up", "Vers le haut"),
    ("down", "Vers le bas"),
    ("swipe", "Dans tous les sens"),
    ("pinch", "Pincer ou écarter"),
    ("pinchin", "Pincer"),
    ("pinchout", "Écarter"),
];

/// Actions d'un geste.
pub const ACTIONS: &[(&str, &str)] = &[
    ("workspace", "Changer de workspace"),
    ("special", "Afficher un workspace spécial"),
    ("move", "Déplacer la fenêtre"),
    ("resize", "Redimensionner la fenêtre"),
    ("close", "Fermer la fenêtre"),
    ("fullscreen", "Plein écran"),
    ("float", "Flottante ou en mosaïque"),
    ("cursorZoom", "Zoom autour du curseur"),
];

const FULLSCREEN_MODES: &[(&str, &str)] = &[
    ("", "Plein écran"),
    ("maximize", "Agrandie (barre visible)"),
];
const FLOAT_MODES: &[(&str, &str)] = &[
    ("", "Basculer"),
    ("float", "Rendre flottante"),
    ("tile", "Remettre en mosaïque"),
];
const ZOOM_MODES: &[(&str, &str)] = &[("", "Suivre le geste"), ("mult", "Multiplier le zoom")];

/// Gestes du pavé tactile.
pub const GESTURE_FIELDS: &[Spec] = &[
    ("fingers", "Doigts", Field::IntChoice(FINGERS), None),
    ("direction", "Geste", Field::Choice(DIRECTIONS), None),
    ("action", "Action", Field::Choice(ACTIONS), None),
    (
        "workspace_name",
        "Workspace spécial (nom)",
        Field::Text,
        Some(("action", &["special"])),
    ),
    (
        "mode",
        "Mode",
        Field::Choice(FULLSCREEN_MODES),
        Some(("action", &["fullscreen"])),
    ),
    (
        "mode",
        "Mode",
        Field::Choice(FLOAT_MODES),
        Some(("action", &["float"])),
    ),
    (
        "mode",
        "Mode",
        Field::Choice(ZOOM_MODES),
        Some(("action", &["cursorZoom"])),
    ),
    (
        "zoom_level",
        "Zoom",
        Field::Num,
        Some(("action", &["cursorZoom"])),
    ),
    (
        "mods",
        "Touches maintenues (SUPER, ALT… ; vide : aucune)",
        Field::Text,
        None,
    ),
    ("scale", "Sensibilité (1 : normale)", Field::Num, None),
];

const ACCEL: &[(&str, &str)] = &[
    ("", "Comme partout"),
    ("adaptive", "Adaptative"),
    ("flat", "Sans accélération"),
];

/// Réglages par périphérique (le nom est choisi à part).
pub const DEVICE_FIELDS: &[Spec] = &[
    ("enabled", "Activé", Field::Bool3, None),
    ("sensitivity", "Vitesse (−1 à 1)", Field::Num, None),
    ("accel_profile", "Accélération", Field::Choice(ACCEL), None),
    ("natural_scroll", "Défilement naturel", Field::Bool3, None),
    ("scroll_factor", "Facteur de défilement", Field::Num, None),
    ("left_handed", "Gaucher", Field::Bool3, None),
    ("tap_to_click", "Toucher pour cliquer", Field::Bool3, None),
    (
        "disable_while_typing",
        "Coupé pendant la frappe",
        Field::Bool3,
        None,
    ),
    ("kb_layout", "Disposition du clavier", Field::Text, None),
    ("kb_variant", "Variante du clavier", Field::Text, None),
    (
        "output",
        "Écran associé (tablette, écran tactile)",
        Field::Text,
        None,
    ),
];

const BOOL3: [&str; 3] = ["Comme partout", "Oui", "Non"];

/// Valeur pointée d'un objet JSON (`match.class`).
pub fn json_get<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    path.split('.').try_fold(v, |acc, k| acc.get(k))
}

/// Pose (ou retire) une valeur pointée dans un objet JSON ; un objet
/// intermédiaire vidé est retiré.
pub fn json_set(v: &mut serde_json::Value, path: &str, val: Option<serde_json::Value>) {
    let parts: Vec<&str> = path.split('.').collect();
    fn go(cur: &mut serde_json::Value, parts: &[&str], val: Option<serde_json::Value>) {
        if !cur.is_object() {
            *cur = serde_json::json!({});
        }
        let obj = cur.as_object_mut().expect("objet");
        let k = parts[0];
        if parts.len() == 1 {
            match val {
                Some(x) => {
                    obj.insert(k.to_string(), x);
                }
                None => {
                    obj.remove(k);
                }
            }
            return;
        }
        let kid = obj
            .entry(k.to_string())
            .or_insert_with(|| serde_json::json!({}));
        go(kid, &parts[1..], val);
        if kid.as_object().is_some_and(|m| m.is_empty()) {
            obj.remove(k);
        }
    }
    go(v, &parts, val);
}

/// Élément `i` d'une liste.
fn item_mut(h: &mut Hypr, list: List, i: usize) -> Option<&mut serde_json::Value> {
    match list {
        List::Rules(_) => h.rules.get_mut(i).map(|r| &mut r.body),
        List::Gestures => h.gestures.get_mut(i),
        List::Devices => h.devices.get_mut(i),
    }
}

/// Geste ajouté : la première combinaison libre parmi les courantes.
fn new_gesture(h: &Hypr) -> serde_json::Value {
    let taken = |f: i64, d: &str| {
        h.gestures.iter().any(|g| {
            g.get("fingers").and_then(|v| v.as_i64()) == Some(f)
                && g.get("direction").and_then(|v| v.as_str()) == Some(d)
                && g.get("mods").is_none()
        })
    };
    let (f, d, a) = [
        (3, "horizontal", "workspace"),
        (4, "vertical", "fullscreen"),
        (3, "vertical", "special"),
        (4, "horizontal", "workspace"),
        (3, "pinch", "float"),
    ]
    .into_iter()
    .find(|(f, d, _)| !taken(*f, d))
    .unwrap_or((5, "swipe", "move"));
    let mut g = serde_json::json!({ "fingers": f, "direction": d, "action": a });
    if a == "special" {
        g["workspace_name"] = serde_json::Value::String("magic".into());
    }
    g
}

impl App {
    /// Carte d'un élément : en-tête (titre, ↑ ↓ ×) et champs.
    #[allow(clippy::too_many_arguments)]
    fn spec_card<'a>(
        &'a self,
        list: List,
        i: usize,
        body: &'a serde_json::Value,
        title: String,
        fields: &'static [Spec],
        movable: (bool, bool),
        head_extra: Option<Element<'a, UiMsg>>,
    ) -> Element<'a, UiMsg> {
        let mut head = row![
            text(title)
                .font(self.title_font())
                .size(crate::ui::tokens::ts(14.0))
                .width(Length::Fill)
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if movable.0 || movable.1 {
            head = head
                .push(
                    button(text("↑").font(self.font).size(crate::ui::tokens::ts(13.0)))
                        .padding([2, 8])
                        .style(button::text)
                        .on_press_maybe(movable.0.then(|| wrap(Msg::SpecMove(list, i, -1)))),
                )
                .push(
                    button(text("↓").font(self.font).size(crate::ui::tokens::ts(13.0)))
                        .padding([2, 8])
                        .style(button::text)
                        .on_press_maybe(movable.1.then(|| wrap(Msg::SpecMove(list, i, 1)))),
                );
        }
        head = head.push(
            button(text("×").font(self.font).size(crate::ui::tokens::ts(14.0)))
                .padding([2, 10])
                .style(button::text)
                .on_press(wrap(Msg::SpecDel(list, i))),
        );
        let mut card = column![head].spacing(6);
        if let Some(e) = head_extra {
            card = card.push(e);
        }
        for (path, label, kind, when) in fields {
            if let Some((p, vals)) = when {
                let v = json_get(body, p).and_then(|v| v.as_str()).unwrap_or("");
                if !vals.contains(&v) {
                    continue;
                }
            }
            card = card.push(
                row![
                    text(crate::i18n::tr(label))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
                        .width(260),
                    self.spec_control(list, i, body, path, *kind)
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        let accent = color(self.cfg.general.theme.accent, 1.0);
        container(card)
            .padding(12)
            .width(Length::Fill)
            .style(move |_| container::Style {
                border: iced::Border::default()
                    .rounded(10)
                    .width(1)
                    .color(Color { a: 0.35, ..accent }),
                ..Default::default()
            })
            .into()
    }

    /// Contrôle d'un champ.
    fn spec_control<'a>(
        &'a self,
        list: List,
        i: usize,
        body: &'a serde_json::Value,
        path: &'static str,
        kind: Field,
    ) -> Element<'a, UiMsg> {
        use serde_json::Value as J;
        let cur = json_get(body, path);
        let set = move |v: Option<J>| wrap(Msg::Spec(list, i, path.to_string(), v));
        match kind {
            Field::Bool => toggler(cur.and_then(J::as_bool).unwrap_or(false))
                .on_toggle(move |v| set(v.then_some(J::Bool(true))))
                .into(),
            Field::Bool3 => {
                let opts: Vec<String> = BOOL3
                    .iter()
                    .map(|l| crate::i18n::tr(l).to_string())
                    .collect();
                let sel = match cur.and_then(J::as_bool) {
                    None => 0,
                    Some(true) => 1,
                    Some(false) => 2,
                };
                let o2 = opts.clone();
                pick_list(
                    opts.clone(),
                    Some(opts[sel].clone()),
                    move |l: String| match o2.iter().position(|o| *o == l) {
                        Some(1) => set(Some(J::Bool(true))),
                        Some(2) => set(Some(J::Bool(false))),
                        _ => set(None),
                    },
                )
                .font(self.font)
                .width(240)
                .into()
            }
            Field::Choice(choices) => {
                let opts: Vec<String> = choices
                    .iter()
                    .map(|(_, l)| crate::i18n::tr(l).to_string())
                    .collect();
                let now = cur.and_then(J::as_str).unwrap_or("");
                let sel = choices
                    .iter()
                    .position(|(v, _)| *v == now)
                    .map(|j| opts[j].clone());
                let o2 = opts.clone();
                pick_list(opts, sel, move |l: String| {
                    let j = o2.iter().position(|o| *o == l).unwrap_or(0);
                    let v = choices[j].0;
                    set((!v.is_empty()).then(|| J::String(v.into())))
                })
                .font(self.font)
                .width(320)
                .into()
            }
            Field::IntChoice(choices) => {
                let opts: Vec<String> = choices
                    .iter()
                    .map(|(_, l)| crate::i18n::tr(l).to_string())
                    .collect();
                let now = cur.and_then(J::as_i64);
                let sel = choices
                    .iter()
                    .position(|(v, _)| Some(*v) == now)
                    .map(|j| opts[j].clone());
                let o2 = opts.clone();
                pick_list(opts, sel, move |l: String| {
                    let j = o2.iter().position(|o| *o == l).unwrap_or(0);
                    set(Some(J::from(choices[j].0)))
                })
                .font(self.font)
                .width(180)
                .into()
            }
            Field::Text | Field::Num | Field::TextOrNum => {
                let shown = match cur {
                    Some(J::String(s)) => s.clone(),
                    Some(J::Null) | None => String::new(),
                    Some(v) => v.to_string(),
                };
                text_input("", &shown)
                    .on_input(move |s| {
                        let num = || {
                            s.trim().replace(',', ".").parse::<f64>().ok().map(|n| {
                                if n.fract() == 0.0 && n.abs() < 1e15 {
                                    J::from(n as i64)
                                } else {
                                    serde_json::Number::from_f64(n).map_or(J::Null, J::Number)
                                }
                            })
                        };
                        let v = if s.is_empty() {
                            None
                        } else {
                            match kind {
                                Field::Num => num().or(Some(J::String(s.clone()))),
                                Field::TextOrNum => num().or(Some(J::String(s.clone()))),
                                _ => Some(J::String(s.clone())),
                            }
                        };
                        wrap(Msg::SpecText(list, i, path.to_string(), v))
                    })
                    .on_submit(wrap(Msg::Commit))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .width(320)
                    .into()
            }
        }
    }

    /// Lignes de hyprland.lua restées en place pour la fonction `func`
    /// (valeur calculée : action en fonction Lua, boucle…).
    fn left_in_hyprland(&self, func: &str) -> Option<Element<'_, UiMsg>> {
        let s = self.hypr_source.as_ref()?;
        let lines: Vec<String> = s
            .calls
            .iter()
            .filter(|c| c.func == func && c.context.is_none())
            .map(|c| c.line.to_string())
            .collect();
        if lines.is_empty() {
            return None;
        }
        Some(
            text(crate::tr!(
                "Aussi dans hyprland.lua (lignes {l}) : valeurs calculées, gardées là-bas.",
                l = lines.join(", ")
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(self.ink().dim)
            .into(),
        )
    }

    fn add_button(&self, label: &'static str, msg: Msg) -> Element<'_, UiMsg> {
        button(
            text(crate::i18n::tr(label))
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0)),
        )
        .padding([4, 12])
        .on_press(wrap(msg))
        .into()
    }

    /// Règles d'une sorte (indices dans `hypr.rules`).
    pub(in crate::ui) fn rules_of(&self, kind: RuleKind) -> Element<'_, UiMsg> {
        let fields = match kind {
            RuleKind::Window => WINDOW_FIELDS,
            RuleKind::Layer => LAYER_FIELDS,
            RuleKind::Workspace => WORKSPACE_FIELDS,
        };
        let rules = &self.cfg.general.hypr.rules;
        let idx: Vec<usize> = (0..rules.len())
            .filter(|&i| rules[i].kind == kind)
            .collect();
        let mut col = column![].spacing(12);
        for (n, &i) in idx.iter().enumerate() {
            let r = &rules[i];
            let title = json_get(&r.body, "name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .or_else(|| {
                    (kind == RuleKind::Workspace)
                        .then(|| json_get(&r.body, "workspace").and_then(|v| v.as_str()))
                        .flatten()
                        .filter(|s| !s.is_empty())
                        .map(|w| crate::tr!("Workspace {w}", w = w))
                })
                .unwrap_or_else(|| crate::tr!("Règle {n}", n = n + 1));
            col = col.push(self.spec_card(
                List::Rules(kind),
                i,
                &r.body,
                title,
                fields,
                (n > 0, n + 1 < idx.len()),
                None,
            ));
        }
        col = col.push(self.add_button("Ajouter une règle", Msg::SpecAdd(List::Rules(kind))));
        if let Some(e) = self.left_in_hyprland(match kind {
            RuleKind::Window => "window_rule",
            RuleKind::Layer => "layer_rule",
            RuleKind::Workspace => "workspace_rule",
        }) {
            col = col.push(e);
        }
        col.into()
    }

    /// Fenêtres → Règles : fenêtres, puis (avancé) couches et workspaces.
    pub(in crate::ui) fn rules_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        column![
            self.section("Règles de fenêtres"),
            text(
                "Appliquées dans l'ordre aux fenêtres dont la classe ou le titre correspond. \
                 Les champs non proposés ici sont gardés tels quels."
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
            self.rules_of(RuleKind::Window),
            self.adv(
                column![
                    self.section("Règles de couches"),
                    text(
                        "Surfaces des barres, lanceurs et notifications (layer-shell), \
                         repérées par leur espace de noms : hyprctl layers les liste."
                    )
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
                    self.rules_of(RuleKind::Layer),
                    self.section("Règles de workspaces"),
                    text(
                        "Écran, nom, espacements et décorations d'un workspace. Le nombre de \
                         workspaces et leurs écrans se règlent dans Workspaces."
                    )
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
                    self.rules_of(RuleKind::Workspace),
                ]
                .spacing(12)
            ),
        ]
        .spacing(12)
        .into()
    }

    /// Clavier et souris → Gestes : liaisons, puis options de Hyprland.
    pub(in crate::ui) fn gestures_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let mut col = column![
            self.section("Gestes du pavé tactile"),
            text(
                "Un nombre de doigts et un sens déclenchent une action. Écrits dans inputs.lua \
                 et pris en compte aussitôt."
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(12);
        let touchpad = self
            .panel
            .as_ref()
            .map(|p| &p.input)
            .is_some_and(|s| !s.loaded || s.touchpad);
        if !touchpad {
            col = col.push(
                text(
                    "Aucun pavé tactile détecté sur ce poste : ces gestes servent au portable \
                     qui partage cette configuration.",
                )
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(crate::ui::tone::warning()),
            );
        }
        let gestures = &self.cfg.general.hypr.gestures;
        let n = gestures.len();
        for (i, g) in gestures.iter().enumerate() {
            let label = |path: &str, table: &[(&str, &str)]| {
                let v = json_get(g, path).and_then(|v| v.as_str()).unwrap_or("");
                table
                    .iter()
                    .find(|(k, _)| *k == v)
                    .map(|(_, l)| crate::i18n::tr(l).to_string())
                    .unwrap_or_else(|| v.to_string())
            };
            let fingers = json_get(g, "fingers").and_then(|v| v.as_i64()).unwrap_or(0);
            let title = crate::tr!(
                "{f} doigts · {d} → {a}",
                f = fingers,
                d = label("direction", DIRECTIONS),
                a = label("action", ACTIONS)
            );
            col = col.push(self.spec_card(
                List::Gestures,
                i,
                g,
                title,
                GESTURE_FIELDS,
                (i > 0, i + 1 < n),
                None,
            ));
        }
        col = col.push(self.add_button("Ajouter un geste", Msg::SpecAdd(List::Gestures)));
        if let Some(e) = self.left_in_hyprland("gesture") {
            col = col.push(e);
        }
        col.into()
    }

    /// Clavier et souris → Périphériques : réglages propres à un appareil.
    pub(in crate::ui) fn devices_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let detected: Vec<String> = self
            .panel
            .as_ref()
            .map(|p| p.input.devices.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default();
        let mut col = column![
            self.section("Réglages par périphérique"),
            text(
                "Ce qui est réglé ici l'emporte, pour ce seul appareil, sur les onglets Clavier, \
                 Souris et Pavé tactile ; « Comme partout » garde le réglage commun."
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(12);
        for (i, d) in self.cfg.general.hypr.devices.iter().enumerate() {
            let name = json_get(d, "name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut opts = detected.clone();
            if !name.is_empty() && !opts.contains(&name) {
                opts.push(name.clone());
            }
            let present = name.is_empty() || detected.contains(&name);
            let mut pick = row![
                text(crate::i18n::tr("Périphérique"))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .width(260),
                pick_list(
                    opts,
                    (!name.is_empty()).then(|| name.clone()),
                    move |n: String| wrap(Msg::Spec(
                        List::Devices,
                        i,
                        "name".into(),
                        Some(serde_json::Value::String(n))
                    ))
                )
                .placeholder(crate::i18n::tr("Choisir un périphérique…"))
                .font(self.font)
                .width(320),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            if !present {
                pick = pick.push(
                    text(crate::i18n::tr("non branché"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim),
                );
            }
            let title = if name.is_empty() {
                crate::i18n::tr("Nouveau périphérique").to_string()
            } else {
                name.clone()
            };
            col = col.push(self.spec_card(
                List::Devices,
                i,
                d,
                title,
                DEVICE_FIELDS,
                (false, false),
                Some(pick.into()),
            ));
        }
        col = col.push(self.add_button("Ajouter un périphérique", Msg::SpecAdd(List::Devices)));
        if let Some(e) = self.left_in_hyprland("device") {
            col = col.push(e);
        }
        if !detected.is_empty() {
            col = col.push(
                text(crate::tr!("Branchés : {l}", l = detected.join(", ")))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
            );
        }
        col.into()
    }

    /// Configuration de Hyprland → Permissions.
    pub(in crate::ui) fn permissions_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let warn = crate::ui::tone::warning();
        let h = &self.cfg.general.hypr;
        let mut col = column![
            self.hyprset_tab("permissions"),
            text(
                "Contrôle des accès : un programme (regex de son chemin) ou un clavier (regex de \
                 son nom) est autorisé, refusé, ou Hyprland demande. Lu au démarrage de \
                 Hyprland seulement : une modification attend la prochaine session."
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
            self.section("Permissions"),
        ]
        .spacing(12);
        let enforced = matches!(
            h.options.get("ecosystem.enforce_permissions"),
            Some(crate::tiling::OptValue::Bool(true))
        );
        let screencopy_ok = h
            .permissions
            .iter()
            .any(|p| p.kind == "screencopy" && p.mode == "allow");
        if enforced && !screencopy_ok {
            col = col.push(
                text(
                    "Contrôle actif sans aucune capture autorisée : captures d'écran, partage \
                     d'écran et vignettes demanderont une autorisation à chaque fois.",
                )
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(warn),
            );
        }
        let kinds: Vec<String> = crate::hyprset::PERMISSION_KINDS
            .iter()
            .map(|(_, l)| crate::i18n::tr(l).to_string())
            .collect();
        let modes: Vec<String> = crate::hyprset::PERMISSION_MODES
            .iter()
            .map(|(_, l)| crate::i18n::tr(l).to_string())
            .collect();
        let label_of = |table: &[(&str, &str)], v: &str| {
            table
                .iter()
                .find(|(k, _)| *k == v)
                .map(|(_, l)| crate::i18n::tr(l).to_string())
                .unwrap_or_else(|| v.to_string())
        };
        for (i, p) in h.permissions.iter().enumerate() {
            let (p1, p2, p3) = (p.clone(), p.clone(), p.clone());
            let (k2, m2) = (kinds.clone(), modes.clone());
            col = col.push(
                row![
                    text_input(crate::i18n::tr("chemin ou nom (regex)"), &p.target)
                        .on_input(move |s| {
                            wrap(Msg::Perm(
                                i,
                                Some(Permission {
                                    target: s,
                                    ..p1.clone()
                                }),
                                false,
                            ))
                        })
                        .on_submit(wrap(Msg::Commit))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
                        .width(Length::Fill),
                    pick_list(
                        kinds.clone(),
                        Some(label_of(crate::hyprset::PERMISSION_KINDS, &p.kind)),
                        move |l: String| {
                            let j = k2.iter().position(|o| *o == l).unwrap_or(0);
                            wrap(Msg::Perm(
                                i,
                                Some(Permission {
                                    kind: crate::hyprset::PERMISSION_KINDS[j].0.into(),
                                    ..p2.clone()
                                }),
                                true,
                            ))
                        }
                    )
                    .font(self.font)
                    .width(240),
                    pick_list(
                        modes.clone(),
                        Some(label_of(crate::hyprset::PERMISSION_MODES, &p.mode)),
                        move |l: String| {
                            let j = m2.iter().position(|o| *o == l).unwrap_or(0);
                            wrap(Msg::Perm(
                                i,
                                Some(Permission {
                                    mode: crate::hyprset::PERMISSION_MODES[j].0.into(),
                                    ..p3.clone()
                                }),
                                true,
                            ))
                        }
                    )
                    .font(self.font)
                    .width(140),
                    button(text("×").font(self.font).size(crate::ui::tokens::ts(14.0)))
                        .padding([2, 10])
                        .style(button::text)
                        .on_press(wrap(Msg::Perm(i, None, true))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        let missing = crate::hyprset::PERMISSION_PRESETS
            .iter()
            .filter(|(t, k, _)| !h.permissions.iter().any(|p| p.target == *t && p.kind == *k))
            .count();
        col = col.push(
            row![
                self.add_button("Ajouter une permission", Msg::PermAdd),
                button(
                    text(crate::tr!(
                        "Ajouter les permissions conseillées ({n})",
                        n = missing
                    ))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                )
                .padding([4, 12])
                .style(button::secondary)
                .on_press_maybe((missing > 0).then(|| wrap(Msg::PermPresets))),
            ]
            .spacing(10),
        );
        col = col.push(
            text(
                "Conseillées : captures pour grim, le portail de Hyprland et hyprdesk ; \
                 chargement de plugins pour hyprpm.",
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        );
        if let Some(e) = self.left_in_hyprland("permission") {
            col = col.push(e);
        }
        col.push(Space::new().height(4)).into()
    }

    /// Messages des listes ; `None` : message d'un autre éditeur.
    pub(in crate::ui) fn hyprlists_update(&mut self, msg: Msg) -> Result<Task<UiMsg>, Msg> {
        Ok(match msg {
            Msg::Spec(list, i, path, v) => {
                self.edit_hypr(|h| {
                    if let Some(b) = item_mut(h, list, i) {
                        json_set(b, &path, v);
                    }
                });
                self.commit()
            }
            Msg::SpecText(list, i, path, v) => {
                self.edit_hypr(|h| {
                    if let Some(b) = item_mut(h, list, i) {
                        json_set(b, &path, v);
                    }
                });
                self.apply()
            }
            Msg::SpecAdd(list) => {
                let detected: Vec<String> = self
                    .panel
                    .as_ref()
                    .map(|p| p.input.devices.iter().map(|(n, _)| n.clone()).collect())
                    .unwrap_or_default();
                self.edit_hypr(|h| match list {
                    List::Rules(kind) => {
                        let n = h.rules.iter().filter(|r| r.kind == kind).count() + 1;
                        let body = match kind {
                            RuleKind::Window => serde_json::json!({
                                "name": format!("hyprdesk-{n}"), "match": { "class": "" }
                            }),
                            RuleKind::Layer => serde_json::json!({
                                "name": format!("hyprdesk-layer-{n}"), "match": { "namespace": "" }
                            }),
                            RuleKind::Workspace => serde_json::json!({ "workspace": "" }),
                        };
                        h.rules.push(crate::hyprset::Rule { kind, body });
                    }
                    List::Gestures => {
                        let g = new_gesture(h);
                        h.gestures.push(g);
                    }
                    List::Devices => {
                        let name = detected
                            .iter()
                            .find(|d| {
                                !h.devices
                                    .iter()
                                    .any(|x| x.get("name").and_then(|v| v.as_str()) == Some(d))
                            })
                            .cloned()
                            .unwrap_or_default();
                        h.devices.push(serde_json::json!({ "name": name }));
                    }
                });
                self.commit()
            }
            Msg::SpecDel(list, i) => {
                self.edit_hypr(|h| match list {
                    List::Rules(_) if i < h.rules.len() => {
                        h.rules.remove(i);
                    }
                    List::Gestures if i < h.gestures.len() => {
                        h.gestures.remove(i);
                    }
                    List::Devices if i < h.devices.len() => {
                        h.devices.remove(i);
                    }
                    _ => {}
                });
                self.commit()
            }
            Msg::SpecMove(list, i, d) => {
                self.edit_hypr(|h| match list {
                    // Avec la voisine de la même sorte.
                    List::Rules(kind) => {
                        let mut j = i as i64 + d as i64;
                        while j >= 0 && (j as usize) < h.rules.len() {
                            if h.rules[j as usize].kind == kind {
                                h.rules.swap(i, j as usize);
                                break;
                            }
                            j += d as i64;
                        }
                    }
                    List::Gestures => {
                        let j = i as i64 + d as i64;
                        if j >= 0 && (j as usize) < h.gestures.len() {
                            h.gestures.swap(i, j as usize);
                        }
                    }
                    List::Devices => {}
                });
                self.commit()
            }
            Msg::Perm(i, p, commit) => {
                self.edit_hypr(|h| match p {
                    Some(p) if i < h.permissions.len() => h.permissions[i] = p,
                    None if i < h.permissions.len() => {
                        h.permissions.remove(i);
                    }
                    _ => {}
                });
                if commit { self.commit() } else { self.apply() }
            }
            Msg::PermAdd => {
                self.edit_hypr(|h| {
                    h.permissions.push(Permission {
                        target: String::new(),
                        kind: "screencopy".into(),
                        mode: "allow".into(),
                    })
                });
                self.commit()
            }
            Msg::PermPresets => {
                self.edit_hypr(|h| {
                    for (t, k, m) in crate::hyprset::PERMISSION_PRESETS {
                        if !h.permissions.iter().any(|p| p.target == *t && p.kind == *k) {
                            h.permissions.push(Permission {
                                target: (*t).into(),
                                kind: (*k).into(),
                                mode: (*m).into(),
                            });
                        }
                    }
                });
                self.commit()
            }
            other => return Err(other),
        })
    }
}

/// Libellés traduits de ces tables (test des traductions).
#[cfg(test)]
pub fn texts() -> Vec<&'static str> {
    let mut all = Vec::new();
    for t in [
        WINDOW_FIELDS,
        LAYER_FIELDS,
        WORKSPACE_FIELDS,
        GESTURE_FIELDS,
        DEVICE_FIELDS,
    ] {
        for (_, label, kind, _) in t {
            all.push(*label);
            match kind {
                Field::Choice(c) => all.extend(c.iter().map(|(_, l)| *l)),
                Field::IntChoice(c) => all.extend(c.iter().map(|(_, l)| *l)),
                _ => {}
            }
        }
    }
    all.extend(IDLE.iter().map(|(_, l)| *l));
    all.extend(crate::hyprset::PERMISSION_KINDS.iter().map(|(_, l)| *l));
    all.extend(crate::hyprset::PERMISSION_MODES.iter().map(|(_, l)| *l));
    all.extend(BOOL3.iter().copied());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pose_et_retire_un_champ_pointe() {
        let mut v = serde_json::json!({ "name": "x" });
        json_set(&mut v, "match.namespace", Some("^bar$".into()));
        assert_eq!(v["match"]["namespace"], "^bar$");
        json_set(&mut v, "match.namespace", None);
        assert!(v.get("match").is_none(), "objet vidé retiré");
    }

    #[test]
    fn geste_ajoute_libre() {
        let mut h = Hypr::default();
        let a = new_gesture(&h);
        assert_eq!(a["direction"], "horizontal");
        h.gestures.push(a);
        let b = new_gesture(&h);
        assert_ne!(b["direction"], "horizontal");
    }
}
