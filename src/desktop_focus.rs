//! Focus en plein bureau (`hd.config({ focus = { … } })`) : au clic ou en
//! suivant la souris, et ce qui passe au premier plan.
//!
//! Hyprland n'a pas de réglage « premier plan » à part : une fenêtre
//! flottante qui prend le focus passe devant les autres. Le choix porte donc
//! sur ce qui donne le focus — le clic, ou le survol — et sur ce qui l'entoure
//! (le clic qui met au premier plan est-il transmis, une application
//! peut-elle se mettre devant d'elle-même, où va le focus quand une fenêtre se
//! ferme).
//!
//! Ces options de Hyprland sont globales : hyprdesk les pose tant qu'un écran
//! est en plein bureau, après en avoir relevé les valeurs en vigueur, et les
//! rend en sortant. Une relecture de la config de Hyprland les remet à
//! celles des fichiers : l'instantané est alors oublié et repris.

use std::sync::Mutex;

use crate::config::{DesktopFocus, FocusMode, FocusOnClose};
use hyprpanel_core::ipc;

/// Options touchées : clé de `hyprctl getoption`, section et nom pour
/// `hl.config`.
const OPTIONS: &[(&str, &str, &str)] = &[
    ("input:follow_mouse", "input", "follow_mouse"),
    (
        "input:float_switch_override_focus",
        "input",
        "float_switch_override_focus",
    ),
    (
        "input:follow_mouse_threshold",
        "input",
        "follow_mouse_threshold",
    ),
    ("input:swallow_focus_click", "input", "swallow_focus_click"),
    ("input:focus_on_close", "input", "focus_on_close"),
    ("misc:focus_on_activate", "misc", "focus_on_activate"),
];

/// Valeurs relevées avant la première pose, rendues en sortant.
static SAVED: Mutex<Option<String>> = Mutex::new(None);

/// Valeurs à poser pour `f`, dans l'ordre de `OPTIONS` ; rien quand le focus
/// reste celui de Hyprland.
pub fn values(f: &DesktopFocus) -> Option<[String; 6]> {
    let (follow, switch) = match f.mode {
        FocusMode::Hyprland => return None,
        // Le survol désigne la fenêtre pour le pointeur (défilement), le clic
        // lui donne le clavier et la met au premier plan.
        FocusMode::Click => (2, 0),
        // La fenêtre survolée prend le clavier, et passe donc devant ; aussi
        // d'une fenêtre flottante à l'autre (le plein bureau les fait flotter).
        FocusMode::Mouse => (1, 2),
    };
    let on_close = match f.on_close {
        FocusOnClose::Next => 0,
        FocusOnClose::Cursor => 1,
        FocusOnClose::Recent => 2,
    };
    Some([
        follow.to_string(),
        switch.to_string(),
        f.threshold.to_string(),
        f.swallow_click.to_string(),
        on_close.to_string(),
        f.on_activate.to_string(),
    ])
}

/// `hl.config` qui pose `values` (dans l'ordre de `OPTIONS`).
fn lua(values: &[String]) -> String {
    let mut input = Vec::new();
    let mut misc = Vec::new();
    for ((_, section, name), v) in OPTIONS.iter().zip(values) {
        let item = format!("{name} = {v}");
        match *section {
            "misc" => misc.push(item),
            _ => input.push(item),
        }
    }
    format!(
        "pcall(hl.config, {{ input = {{ {} }}, misc = {{ {} }} }})",
        input.join(", "),
        misc.join(", ")
    )
}

/// Valeurs en vigueur des options touchées, en Lua ; une option qui ne
/// répond pas garde le défaut de Hyprland.
fn snapshot() -> String {
    const DEFAULTS: [&str; 6] = ["1", "1", "0", "false", "0", "false"];
    let values: Vec<String> = OPTIONS
        .iter()
        .zip(DEFAULTS)
        .map(|((key, ..), default)| {
            crate::inputs::option(key)
                .map(|v| match v {
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => default.to_string(),
                })
                .unwrap_or_else(|| default.to_string())
        })
        .collect();
    lua(&values)
}

/// Pose `values` ; la première fois, relève d'abord ce qu'elles remplacent.
pub fn apply(values: &[String]) -> Result<(), String> {
    {
        let mut saved = SAVED.lock().map_err(|e| e.to_string())?;
        if saved.is_none() {
            *saved = Some(snapshot());
        }
    }
    ipc::eval(&lua(values))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Rend les valeurs relevées avant la première pose.
pub fn restore() -> Result<(), String> {
    let saved = SAVED.lock().map_err(|e| e.to_string())?.take();
    match saved {
        Some(lua) => ipc::eval(&lua).map(|_| ()).map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

/// Après une relecture de Hyprland : les options sont celles des fichiers,
/// l'instantané ne vaut plus.
pub fn forget() {
    if let Ok(mut saved) = SAVED.lock() {
        *saved = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn au_clic_ou_en_suivant_la_souris() {
        let mut f = DesktopFocus::default();
        assert!(values(&f).is_none(), "Hyprland : rien n'est touché");
        f.mode = FocusMode::Click;
        f.swallow_click = true;
        f.on_close = FocusOnClose::Recent;
        let v = values(&f).unwrap();
        assert_eq!(v, ["2", "0", "0", "true", "2", "false"]);
        assert_eq!(
            lua(&v),
            "pcall(hl.config, { input = { follow_mouse = 2, float_switch_override_focus = 0, \
             follow_mouse_threshold = 0, swallow_focus_click = true, focus_on_close = 2 }, \
             misc = { focus_on_activate = false } })"
        );
        f.mode = FocusMode::Mouse;
        f.threshold = 12;
        f.on_activate = true;
        let v = values(&f).unwrap();
        assert_eq!(&v[..3], ["1", "2", "12"]);
        assert_eq!(v[5], "true");
    }
}
