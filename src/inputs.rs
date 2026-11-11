//! Clavier, souris et pavé tactile : hyprdesk tient `inputs.lua`, comme
//! `monitors.lua` (Réglages → Écrans). Les valeurs en vigueur sont lues dans
//! Hyprland (`getoption`), appliquées à chaud (`hl.config`), puis écrites
//! dans `inputs.lua` (requis par hyprland.lua : `require("inputs")`). Un
//! fichier d'un autre outil (hyprdmc…) est gardé une fois en `.bak`.

use std::path::{Path, PathBuf};

use hyprpanel_core::ipc;
use serde_json::Value;

/// Réglages d'entrée tenus par hyprdesk.
#[derive(Debug, Clone, PartialEq)]
pub struct Inputs {
    /// Dispositions, séparées par des virgules (« fr,us »).
    pub kb_layout: String,
    pub kb_variant: String,
    /// Options xkb (« grp:alt_shift_toggle,caps:escape »).
    pub kb_options: String,
    /// Répétition : touches par seconde, et délai avant (ms).
    pub repeat_rate: i64,
    pub repeat_delay: i64,
    pub numlock: bool,
    /// Vitesse du pointeur, -1 à 1.
    pub sensitivity: f64,
    /// Accélération : « » (celle du périphérique), « adaptive », « flat ».
    pub accel_profile: String,
    pub natural_scroll: bool,
    pub left_handed: bool,
    /// Focus : 0 au clic, 1 suit la souris, 2 et 3 variantes de Hyprland.
    pub follow_mouse: i64,
    pub tp_natural_scroll: bool,
    pub tp_tap_to_click: bool,
    pub tp_disable_while_typing: bool,
    /// Clic à deux doigts = clic droit (clickfinger).
    pub tp_clickfinger: bool,
    pub tp_scroll_factor: f64,
}

impl Default for Inputs {
    /// Valeurs par défaut de Hyprland.
    fn default() -> Self {
        Inputs {
            kb_layout: "us".into(),
            kb_variant: String::new(),
            kb_options: String::new(),
            repeat_rate: 25,
            repeat_delay: 600,
            numlock: false,
            sensitivity: 0.0,
            accel_profile: String::new(),
            natural_scroll: false,
            left_handed: false,
            follow_mouse: 1,
            tp_natural_scroll: false,
            tp_tap_to_click: true,
            tp_disable_while_typing: true,
            tp_clickfinger: false,
            tp_scroll_factor: 1.0,
        }
    }
}

/// Valeur d'une réponse `getoption -j` (`str`, `int`, `float`, `bool`).
pub fn option_value(v: &Value) -> Option<Value> {
    ["str", "int", "float", "bool"]
        .iter()
        .find_map(|k| v.get(*k).cloned())
}

fn get(path: &str) -> Option<Value> {
    ipc::query_json(&format!("getoption {path}"))
        .ok()
        .and_then(|v| option_value(&v))
}

/// Valeurs en vigueur dans Hyprland (les défauts pour ce qui ne répond pas).
pub fn read() -> Inputs {
    let mut i = Inputs::default();
    let s = |p: &str, d: &mut String| {
        if let Some(v) = get(p).and_then(|v| v.as_str().map(|s| s.trim().to_string())) {
            *d = if v == "[[EMPTY]]" { String::new() } else { v };
        }
    };
    let n = |p: &str, d: &mut i64| {
        if let Some(v) = get(p).and_then(|v| v.as_i64()) {
            *d = v;
        }
    };
    let f = |p: &str, d: &mut f64| {
        if let Some(v) = get(p).and_then(|v| v.as_f64()) {
            *d = v;
        }
    };
    let b = |paths: &[&str], d: &mut bool| {
        for p in paths {
            if let Some(v) = get(p) {
                *d = v.as_bool().unwrap_or_else(|| v.as_i64().unwrap_or(0) != 0);
                return;
            }
        }
    };
    s("input:kb_layout", &mut i.kb_layout);
    s("input:kb_variant", &mut i.kb_variant);
    s("input:kb_options", &mut i.kb_options);
    n("input:repeat_rate", &mut i.repeat_rate);
    n("input:repeat_delay", &mut i.repeat_delay);
    b(&["input:numlock_by_default"], &mut i.numlock);
    f("input:sensitivity", &mut i.sensitivity);
    s("input:accel_profile", &mut i.accel_profile);
    b(&["input:natural_scroll"], &mut i.natural_scroll);
    b(&["input:left_handed"], &mut i.left_handed);
    n("input:follow_mouse", &mut i.follow_mouse);
    b(&["input:touchpad:natural_scroll"], &mut i.tp_natural_scroll);
    b(
        &["input:touchpad:tap_to_click", "input:touchpad:tap-to-click"],
        &mut i.tp_tap_to_click,
    );
    b(
        &["input:touchpad:disable_while_typing"],
        &mut i.tp_disable_while_typing,
    );
    b(
        &["input:touchpad:clickfinger_behavior"],
        &mut i.tp_clickfinger,
    );
    f("input:touchpad:scroll_factor", &mut i.tp_scroll_factor);
    i
}

fn q(s: &str) -> String {
    format!("\"{}\"", ipc::lua_escape(s))
}

fn num(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

impl Inputs {
    /// `hl.config({ input = { … } })`.
    pub fn lua(&self) -> String {
        format!(
            "hl.config({{ input = {{ kb_layout = {}, kb_variant = {}, kb_options = {}, \
             repeat_rate = {}, repeat_delay = {}, numlock_by_default = {}, \
             sensitivity = {}, accel_profile = {}, natural_scroll = {}, left_handed = {}, \
             follow_mouse = {}, touchpad = {{ natural_scroll = {}, tap_to_click = {}, \
             disable_while_typing = {}, clickfinger_behavior = {}, scroll_factor = {} }} }} }})",
            q(&self.kb_layout),
            q(&self.kb_variant),
            q(&self.kb_options),
            self.repeat_rate,
            self.repeat_delay,
            self.numlock,
            num(self.sensitivity),
            q(&self.accel_profile),
            self.natural_scroll,
            self.left_handed,
            self.follow_mouse,
            self.tp_natural_scroll,
            self.tp_tap_to_click,
            self.tp_disable_while_typing,
            self.tp_clickfinger,
            num(self.tp_scroll_factor),
        )
    }
}

/// `inputs.lua`, à côté de `hyprdesk.lua` (requis par `hyprland.lua`).
pub fn inputs_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("inputs.lua")
}

/// Contenu de `inputs.lua`.
pub fn render(i: &Inputs) -> String {
    let head = crate::i18n::tr_lines(
        "-- Généré par hyprdesk (Réglages → Clavier et souris) — NE PAS ÉDITER : réécrit à chaque enregistrement.\n\
         -- Requis par hyprland.lua : require(\"inputs\").\n",
    );
    format!("{head}\n{}\n", i.lua())
}

/// Fichier écrit par hyprdesk (en-tête français ou anglais).
fn ours(content: &str) -> bool {
    content.starts_with("-- Généré par hyprdesk") || content.starts_with("-- Generated by hyprdesk")
}

/// Applique à chaud (Hyprland relit aussitôt).
pub fn apply(i: &Inputs) -> Result<(), String> {
    let r = ipc::eval(&i.lua()).map_err(|e| e.to_string())?;
    if r.trim() != "ok" {
        return Err(r.trim().to_string());
    }
    Ok(())
}

/// Enregistre `inputs.lua` ; celui d'un autre outil est gardé une fois en `.bak`.
pub fn save(path: &Path, i: &Inputs) -> std::io::Result<()> {
    let previous = std::fs::read_to_string(path).unwrap_or_default();
    if !previous.is_empty() && !ours(&previous) {
        let bak = path.with_extension("lua.bak");
        if !bak.exists() {
            std::fs::write(&bak, &previous)?;
        }
    }
    crate::storage::write(path, render(i).as_bytes(), crate::storage::revision())
}

/// Dispositions et variantes de clavier (`evdev.lst` de xkeyboard-config) :
/// (code, description) ; variantes : (disposition, code, description).
pub type Layouts = (Vec<(String, String)>, Vec<(String, String, String)>);

pub fn parse_evdev(src: &str) -> Layouts {
    let (mut layouts, mut variants) = (Vec::new(), Vec::new());
    let mut section = "";
    for line in src.lines() {
        if let Some(s) = line.strip_prefix("! ") {
            section = s.trim();
            continue;
        }
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let Some((code, desc)) = l.split_once(char::is_whitespace) else {
            continue;
        };
        let desc = desc.trim();
        match section {
            "layout" => layouts.push((code.to_string(), desc.to_string())),
            "variant" => {
                if let Some((layout, d)) = desc.split_once(':') {
                    variants.push((layout.to_string(), code.to_string(), d.trim().to_string()));
                }
            }
            _ => {}
        }
    }
    (layouts, variants)
}

pub fn layouts() -> Layouts {
    std::fs::read_to_string("/usr/share/X11/xkb/rules/evdev.lst")
        .map(|s| parse_evdev(&s))
        .unwrap_or_default()
}

/// Un pavé tactile est branché (Hyprland : `devices`).
pub fn has_touchpad() -> bool {
    ipc::query_json("devices")
        .ok()
        .and_then(|v| v["mice"].as_array().cloned())
        .is_some_and(|m| {
            m.iter().any(|d| {
                d["name"]
                    .as_str()
                    .is_some_and(|n| n.to_lowercase().contains("touchpad"))
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_lua() {
        let i = Inputs {
            kb_layout: "fr,us".into(),
            kb_variant: "azerty".into(),
            sensitivity: -0.25,
            tp_natural_scroll: true,
            ..Default::default()
        };
        let lua = i.lua();
        assert!(
            lua.starts_with("hl.config({ input = { kb_layout = \"fr,us\", kb_variant = \"azerty\"")
        );
        assert!(lua.contains("sensitivity = -0.25"));
        assert!(lua.contains("touchpad = { natural_scroll = true, tap_to_click = true"));
        assert!(lua.contains("scroll_factor = 1 }"));
        let file = render(&i);
        assert!(ours(&file));
        assert!(!ours("-- Generated by hyprdmc — DO NOT EDIT BY HAND.\n"));
    }

    #[test]
    fn valeurs_de_getoption() {
        let v: Value =
            serde_json::from_str(r#"{"option": "input:kb_layout", "str": "fr", "set": true}"#)
                .unwrap();
        assert_eq!(option_value(&v), Some(Value::String("fr".into())));
        let v: Value =
            serde_json::from_str(r#"{"option": "x", "bool": true, "set": false}"#).unwrap();
        assert_eq!(option_value(&v), Some(Value::Bool(true)));
    }

    #[test]
    fn dispositions_xkb() {
        let src = "! model\n  pc105  Generic 105-key PC\n\n! layout\n  us   English (US)\n  fr   French\n\n! variant\n  azerty   fr: French (AZERTY)\n  dvorak   us: English (Dvorak)\n\n! option\n  grp  Switching\n";
        let (l, v) = parse_evdev(src);
        assert_eq!(
            l,
            vec![
                ("us".into(), "English (US)".into()),
                ("fr".into(), "French".into())
            ]
        );
        assert_eq!(
            v[0],
            ("fr".into(), "azerty".into(), "French (AZERTY)".into())
        );
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn sauvegarde_et_bak() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-inputs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("inputs.lua");
        std::fs::write(
            &p,
            "-- Generated by hyprdmc — DO NOT EDIT BY HAND.\nhl.config({})\n",
        )
        .unwrap();
        save(&p, &Inputs::default()).unwrap();
        assert!(ours(&std::fs::read_to_string(&p).unwrap()));
        assert!(
            std::fs::read_to_string(dir.join("inputs.lua.bak"))
                .unwrap()
                .contains("hyprdmc")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
