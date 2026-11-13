//! Layouts de Hyprland (paquet « Layouts ») : layout par défaut, layout par
//! workspace, options de dwindle, master et scrolling, et layouts
//! personnalisés fournis par hyprdesk.
//!
//! Hyprland 0.57 : les layouts intégrés se nomment `dwindle`, `master`,
//! `scrolling` et `monocle` ; un layout écrit en Lua s'enregistre avec
//! `hl.layout.register(nom, { recalculate = … })` et se désigne `lua:<nom>`
//! (cibles numérotées à partir de 1). Le layout par défaut (`general.layout`)
//! et une règle de workspace (`layout = …`) s'appliquent à chaud.
//!
//! Seuls les réglages faits dans hyprdesk sont posés : le reste vient de
//! hyprland.lua. Un réglage retiré fait relire sa config à Hyprland (rien ne
//! « défait » une option posée à chaud), puis hyprdesk repose les siens.

use std::collections::BTreeMap;

/// Layouts intégrés de Hyprland : (nom, libellé).
pub const BUILTIN: &[(&str, &str)] = &[
    ("dwindle", "Dwindle"),
    ("master", "Master"),
    ("scrolling", "Scrolling"),
    ("monocle", "Monocle"),
];

/// Layout personnalisé fourni par hyprdesk (Lua de Hyprland).
pub struct Custom {
    /// Nom dans hyprdesk (`grid`) ; Hyprland le connaît comme `lua:hyprdesk-grid`.
    pub name: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    /// Corps de `recalculate(ctx)`.
    pub lua: &'static str,
}

pub const CUSTOM: &[Custom] = &[
    Custom {
        name: "grid",
        label: "Grille",
        summary: "Toutes les fenêtres en grille régulière, ligne par ligne.",
        lua: "local n = #ctx.targets\n\
              if n == 0 then return end\n\
              local cols = math.ceil(math.sqrt(n))\n\
              local rows = math.ceil(n / cols)\n\
              for i, t in ipairs(ctx.targets) do t:place(ctx:grid_cell(i, cols, rows)) end",
    },
    Custom {
        name: "columns",
        label: "Colonnes",
        summary: "Une colonne par fenêtre, côte à côte, de même largeur.",
        lua: "for i, t in ipairs(ctx.targets) do t:place(ctx:column(i, #ctx.targets)) end",
    },
    Custom {
        name: "rows",
        label: "Rangées",
        summary: "Une rangée par fenêtre, empilées, de même hauteur.",
        lua: "for i, t in ipairs(ctx.targets) do t:place(ctx:row(i, #ctx.targets)) end",
    },
    Custom {
        name: "centered",
        label: "Maître centré",
        summary: "La première fenêtre au centre, les autres en piles à gauche et à droite \
                  (écrans larges).",
        lua: "local n = #ctx.targets\n\
              if n == 0 then return end\n\
              local a = ctx.area\n\
              if n == 1 then ctx.targets[1]:place(a) return end\n\
              local mw = math.floor(a.w * 0.5)\n\
              local side = math.floor((a.w - mw) / 2)\n\
              local left, right = {}, {}\n\
              for i = 2, n do\n\
                if i % 2 == 0 then right[#right + 1] = ctx.targets[i] else left[#left + 1] = ctx.targets[i] end\n\
              end\n\
              local function stack(list, x, w)\n\
                local h = a.h / #list\n\
                for i, t in ipairs(list) do t:place({ x = x, y = a.y + (i - 1) * h, w = w, h = h }) end\n\
              end\n\
              if #left == 0 then\n\
                ctx.targets[1]:place({ x = a.x, y = a.y, w = a.w - side, h = a.h })\n\
                stack(right, a.x + a.w - side, side)\n\
              else\n\
                ctx.targets[1]:place({ x = a.x + side, y = a.y, w = mw, h = a.h })\n\
                stack(left, a.x, side)\n\
                stack(right, a.x + side + mw, side)\n\
              end",
    },
];

/// Valeur d'une option de layout.
#[derive(Debug, Clone, PartialEq)]
pub enum OptValue {
    Bool(bool),
    Num(f64),
    Str(String),
}

impl OptValue {
    pub fn lua(&self) -> String {
        match self {
            OptValue::Bool(b) => b.to_string(),
            OptValue::Num(n) if n.fract() == 0.0 => format!("{}", *n as i64),
            OptValue::Num(n) => format!("{n}"),
            OptValue::Str(s) => format!("\"{}\"", hyprpanel_core::ipc::lua_escape(s)),
        }
    }

    pub fn display(&self) -> String {
        match self {
            OptValue::Bool(b) => if *b { "on" } else { "off" }.into(),
            OptValue::Num(n) => format!("{}", (n * 100.0).round() / 100.0),
            OptValue::Str(s) => s.clone(),
        }
    }
}

/// Nature d'une option (contrôle de l'écran Réglages, validation).
pub enum Kind {
    Bool,
    Num {
        min: f64,
        max: f64,
        step: f64,
    },
    /// Entier parmi des valeurs nommées : (valeur, libellé).
    IntChoice(&'static [(i64, &'static str)]),
    /// Chaîne parmi des valeurs : (valeur, libellé).
    Choice(&'static [(&'static str, &'static str)]),
}

/// Option d'un layout de Hyprland.
pub struct Opt {
    /// `dwindle`, `master`, `scrolling`.
    pub section: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
}

impl Opt {
    /// `dwindle.preserve_split`.
    pub fn path(&self) -> String {
        format!("{}.{}", self.section, self.key)
    }

    /// Valeur venue du Lua ou de la ligne de commande, vérifiée.
    pub fn parse(&self, v: &str) -> Result<OptValue, String> {
        let v = v.trim();
        match &self.kind {
            Kind::Bool => match v {
                "on" | "true" | "yes" | "1" => Ok(OptValue::Bool(true)),
                "off" | "false" | "no" | "0" => Ok(OptValue::Bool(false)),
                _ => Err(crate::tr!("« {v} » : on ou off attendu", v = v)),
            },
            Kind::Num { min, max, .. } => match v.parse::<f64>() {
                Ok(n) if (*min..=*max).contains(&n) => Ok(OptValue::Num(n)),
                _ => Err(crate::tr!(
                    "« {v} » : entre {min} et {max}",
                    v = v,
                    min = min,
                    max = max
                )),
            },
            Kind::IntChoice(list) => v
                .parse::<i64>()
                .ok()
                .filter(|n| list.iter().any(|(x, _)| x == n))
                .map(|n| OptValue::Num(n as f64))
                .ok_or_else(|| {
                    let all: Vec<String> = list.iter().map(|(x, _)| x.to_string()).collect();
                    crate::tr!(
                        "« {v} » : valeurs admises {allowed}",
                        v = v,
                        allowed = all.join(", ")
                    )
                }),
            Kind::Choice(list) => list
                .iter()
                .find(|(x, _)| *x == v)
                .map(|(x, _)| OptValue::Str(x.to_string()))
                .ok_or_else(|| {
                    let all: Vec<&str> = list.iter().map(|(x, _)| *x).collect();
                    crate::tr!(
                        "« {v} » : valeurs admises {allowed}",
                        v = v,
                        allowed = all.join(", ")
                    )
                }),
        }
    }
}

const ON_OFF: Kind = Kind::Bool;

/// Options réglables, d'après les définitions de Hyprland 0.57 (`hl.meta.lua`).
pub const OPTIONS: &[Opt] = &[
    // Dwindle
    Opt {
        section: "dwindle",
        key: "preserve_split",
        label: "Garder le sens de chaque découpe",
        kind: ON_OFF,
    },
    Opt {
        section: "dwindle",
        key: "smart_split",
        label: "Découper selon la position de la souris",
        kind: ON_OFF,
    },
    Opt {
        section: "dwindle",
        key: "smart_resizing",
        label: "Redimensionner selon le coin le plus proche",
        kind: ON_OFF,
    },
    Opt {
        section: "dwindle",
        key: "force_split",
        label: "Côté de la nouvelle fenêtre",
        kind: Kind::IntChoice(&[
            (0, "Suit la souris"),
            (1, "Toujours à gauche ou en haut"),
            (2, "Toujours à droite ou en bas"),
        ]),
    },
    Opt {
        section: "dwindle",
        key: "default_split_ratio",
        label: "Proportion d'une découpe",
        kind: Kind::Num {
            min: 0.1,
            max: 1.9,
            step: 0.05,
        },
    },
    Opt {
        section: "dwindle",
        key: "split_width_multiplier",
        label: "Facteur largeur/hauteur pour choisir le sens",
        kind: Kind::Num {
            min: 0.1,
            max: 3.0,
            step: 0.1,
        },
    },
    Opt {
        section: "dwindle",
        key: "use_active_for_splits",
        label: "Découper la fenêtre active (sinon celle sous la souris)",
        kind: ON_OFF,
    },
    Opt {
        section: "dwindle",
        key: "special_scale_factor",
        label: "Échelle du workspace spécial",
        kind: Kind::Num {
            min: 0.1,
            max: 1.0,
            step: 0.05,
        },
    },
    // Master
    Opt {
        section: "master",
        key: "orientation",
        label: "Place de la fenêtre maître",
        kind: Kind::Choice(&[
            ("left", "À gauche"),
            ("right", "À droite"),
            ("top", "En haut"),
            ("bottom", "En bas"),
            ("center", "Au centre"),
        ]),
    },
    Opt {
        section: "master",
        key: "mfact",
        label: "Part de la fenêtre maître",
        kind: Kind::Num {
            min: 0.05,
            max: 0.95,
            step: 0.05,
        },
    },
    Opt {
        section: "master",
        key: "new_status",
        label: "Nouvelle fenêtre",
        kind: Kind::Choice(&[
            ("slave", "Dans la pile"),
            ("master", "Devient maître"),
            ("inherit", "Comme la fenêtre active"),
        ]),
    },
    Opt {
        section: "master",
        key: "new_on_top",
        label: "Nouvelle fenêtre en tête de pile",
        kind: ON_OFF,
    },
    Opt {
        section: "master",
        key: "new_on_active",
        label: "Placer la nouvelle fenêtre",
        kind: Kind::Choice(&[
            ("none", "Selon « en tête de pile »"),
            ("before", "Avant la fenêtre active"),
            ("after", "Après la fenêtre active"),
        ]),
    },
    Opt {
        section: "master",
        key: "slave_count_for_center_master",
        label: "Fenêtres en pile avant de centrer le maître",
        kind: Kind::IntChoice(&[(0, "0"), (1, "1"), (2, "2"), (3, "3"), (4, "4")]),
    },
    Opt {
        section: "master",
        key: "center_master_fallback",
        label: "Maître centré, s'il n'y a pas assez de fenêtres",
        kind: Kind::Choice(&[
            ("left", "À gauche"),
            ("right", "À droite"),
            ("top", "En haut"),
            ("bottom", "En bas"),
        ]),
    },
    Opt {
        section: "master",
        key: "allow_small_split",
        label: "Autoriser plusieurs maîtres côte à côte",
        kind: ON_OFF,
    },
    Opt {
        section: "master",
        key: "smart_resizing",
        label: "Redimensionner selon le coin le plus proche",
        kind: ON_OFF,
    },
    Opt {
        section: "master",
        key: "drop_at_cursor",
        label: "Déposer une fenêtre là où est la souris",
        kind: ON_OFF,
    },
    Opt {
        section: "master",
        key: "focus_master_on_close",
        label: "Revenir au maître à la fermeture d'une fenêtre",
        kind: ON_OFF,
    },
    // Scrolling
    Opt {
        section: "scrolling",
        key: "direction",
        label: "Sens du défilement",
        kind: Kind::Choice(&[
            ("right", "Vers la droite"),
            ("left", "Vers la gauche"),
            ("down", "Vers le bas"),
            ("up", "Vers le haut"),
        ]),
    },
    Opt {
        section: "scrolling",
        key: "column_width",
        label: "Largeur d'une colonne (part de l'écran)",
        kind: Kind::Num {
            min: 0.1,
            max: 1.0,
            step: 0.05,
        },
    },
    Opt {
        section: "scrolling",
        key: "fullscreen_on_one_column",
        label: "Une seule colonne : plein écran",
        kind: ON_OFF,
    },
    Opt {
        section: "scrolling",
        key: "focus_fit_method",
        label: "Amener la colonne active",
        kind: Kind::IntChoice(&[(0, "Au centre"), (1, "Juste dans l'écran")]),
    },
    Opt {
        section: "scrolling",
        key: "follow_focus",
        label: "Suivre la fenêtre active",
        kind: ON_OFF,
    },
    Opt {
        section: "scrolling",
        key: "follow_min_visible",
        label: "Part visible minimale avant de suivre",
        kind: Kind::Num {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Opt {
        section: "scrolling",
        key: "wrap_focus",
        label: "Boucler en passant d'une colonne à l'autre",
        kind: ON_OFF,
    },
    Opt {
        section: "scrolling",
        key: "wrap_swapcol",
        label: "Boucler en échangeant les colonnes",
        kind: ON_OFF,
    },
];

/// Sections d'options (onglets).
pub const SECTIONS: &[&str] = &["dwindle", "master", "scrolling"];

pub fn opt(path: &str) -> Option<&'static Opt> {
    OPTIONS.iter().find(|o| o.path() == path)
}

/// Réglages de layouts faits dans hyprdesk.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tiling {
    /// Layout par défaut (nom hyprdesk : `master`, `grid`, `lua:mon-layout`…).
    pub layout: Option<String>,
    /// Layout de certains workspaces.
    pub workspaces: BTreeMap<String, String>,
    /// Options : `dwindle.preserve_split` → valeur.
    pub options: BTreeMap<String, OptValue>,
}

/// Layouts demandés par hyprdesk : le layout par défaut et ceux des
/// workspaces (celui de hyprland.lua s'y ajoute, lu dans Hyprland).
pub fn in_use(t: &Tiling) -> Vec<String> {
    t.layout
        .iter()
        .chain(t.workspaces.values())
        .cloned()
        .collect()
}

/// Nom accepté : layout intégré, personnalisé de hyprdesk, ou `lua:<nom>`.
pub fn valid_name(name: &str) -> bool {
    BUILTIN.iter().any(|(n, _)| *n == name)
        || CUSTOM.iter().any(|c| c.name == name)
        || name
            .strip_prefix("lua:")
            .is_some_and(|n| !n.is_empty() && !n.contains(['"', '\\', '\n']))
}

/// Nom pour Hyprland.
pub fn hypr_name(name: &str) -> String {
    if CUSTOM.iter().any(|c| c.name == name) {
        format!("lua:hyprdesk-{name}")
    } else {
        name.to_string()
    }
}

/// Clés posées par hyprdesk (pour savoir ce qui a été retiré).
fn applied_keys(t: &Tiling) -> Vec<String> {
    let mut k: Vec<String> = t.options.keys().cloned().collect();
    if t.layout.is_some() {
        k.push("general.layout".into());
    }
    k.extend(t.workspaces.keys().map(|w| format!("ws:{w}")));
    k.sort();
    k
}

/// Lua qui enregistre les layouts personnalisés de hyprdesk.
/// Une seule fois par état Lua de Hyprland : réenregistrer un layout est une
/// erreur, signalée même sous `pcall`. Un rechargement qui efface l'état Lua
/// efface aussi la marque, et les layouts sont réenregistrés.
pub fn register_lua() -> String {
    let regs: String = CUSTOM
        .iter()
        .map(|c| {
            format!(
                "  pcall(hl.layout.register, \"hyprdesk-{}\", {{ recalculate = function(ctx)\n{}\nend }})\n",
                c.name, c.lua
            )
        })
        .collect();
    format!("if not __hyprdesk_layouts then\n  __hyprdesk_layouts = true\n{regs}end\n")
}

/// Lua qui pose les réglages (`keys` : clés posées, gardées pour la suite).
pub fn apply_lua(t: &Tiling) -> String {
    let mut lua = register_lua();
    lua += "if __hyprdesk_tiling then for _, r in ipairs(__hyprdesk_tiling) do \
            pcall(function() r:set_enabled(false) end) end end\n\
            __hyprdesk_tiling = {}\n";
    if let Some(l) = &t.layout {
        lua += &format!(
            "pcall(hl.config, {{ general = {{ layout = \"{}\" }} }})\n",
            hyprpanel_core::ipc::lua_escape(&hypr_name(l))
        );
    }
    for section in SECTIONS {
        let fields: Vec<String> = t
            .options
            .iter()
            .filter_map(|(path, v)| {
                let (s, k) = path.split_once('.')?;
                (s == *section).then(|| format!("{k} = {}", v.lua()))
            })
            .collect();
        if !fields.is_empty() {
            lua += &format!(
                "pcall(hl.config, {{ {section} = {{ {} }} }})\n",
                fields.join(", ")
            );
        }
    }
    for (ws, l) in &t.workspaces {
        lua += &format!(
            "pcall(function() table.insert(__hyprdesk_tiling, hl.workspace_rule({{ workspace = \"{}\", layout = \"{}\" }})) end)\n",
            hyprpanel_core::ipc::lua_escape(ws),
            hyprpanel_core::ipc::lua_escape(&hypr_name(l))
        );
    }
    lua += &format!(
        "__hyprdesk_tiling_keys = \"{}\"\n",
        applied_keys(t).join(" ")
    );
    lua
}

/// Pose les réglages. Si un réglage posé auparavant a été retiré, Hyprland
/// relit sa config (les valeurs de hyprland.lua reviennent) ; hyprdesk
/// repose alors les siens sur l'événement de relecture.
pub fn apply(t: &Tiling) -> Result<(), String> {
    use hyprpanel_core::ipc;
    let before = ipc::repl("__hyprdesk_tiling_keys or \"\"").map_err(|e| e.to_string())?;
    let before = before.trim().trim_matches('"');
    let now = applied_keys(t);
    let removed = before
        .split_whitespace()
        .any(|k| !now.iter().any(|n| n == k));
    if removed {
        ipc::eval(
            "__hyprdesk_tiling_keys = nil\n\
             hl.dispatch(hl.dsp.exec_cmd(\"hyprctl reload\"))",
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let r = ipc::eval(&apply_lua(t)).map_err(|e| e.to_string())?;
    if r.trim() != "ok" && !r.trim().is_empty() {
        return Err(r);
    }
    Ok(())
}

/// Valeurs en place dans Hyprland (fichiers de config), pour l'écran Réglages.
pub fn current() -> BTreeMap<String, String> {
    let keys: Vec<String> = OPTIONS
        .iter()
        .map(|o| o.path())
        .chain(std::iter::once("general.layout".to_string()))
        .collect();
    let lua = format!(
        "local out = {{}} for _, k in ipairs({{ {} }}) do local ok, v = pcall(hl.get_config, k) \
         if ok and v ~= nil then out[#out + 1] = k .. \"=\" .. tostring(v) end end \
         return table.concat(out, \"\\n\")",
        keys.iter()
            .map(|k| format!("\"{k}\""))
            .collect::<Vec<_>>()
            .join(", ")
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
    fn noms_et_lua() {
        assert!(valid_name("master") && valid_name("grid") && valid_name("lua:mien"));
        assert!(!valid_name("nimporte") && !valid_name("lua:") && !valid_name("lua:a\"b"));
        assert_eq!(hypr_name("grid"), "lua:hyprdesk-grid");
        assert_eq!(hypr_name("lua:mien"), "lua:mien");
        let mut t = Tiling {
            layout: Some("centered".into()),
            ..Default::default()
        };
        t.workspaces.insert("3".into(), "monocle".into());
        t.options
            .insert("dwindle.preserve_split".into(), OptValue::Bool(true));
        t.options
            .insert("master.orientation".into(), OptValue::Str("center".into()));
        t.options.insert("master.mfact".into(), OptValue::Num(0.6));
        let lua = apply_lua(&t);
        assert!(
            lua.contains(r#"general = { layout = "lua:hyprdesk-centered" }"#),
            "{lua}"
        );
        assert!(lua.contains("dwindle = { preserve_split = true }"), "{lua}");
        assert!(
            lua.contains(r#"master = { mfact = 0.6, orientation = "center" }"#),
            "{lua}"
        );
        assert!(
            lua.contains(r#"workspace = "3", layout = "monocle""#),
            "{lua}"
        );
        assert!(lua.contains("pcall(hl.layout.register, \"hyprdesk-grid\""));
        assert!(lua.contains(
            "__hyprdesk_tiling_keys = \"dwindle.preserve_split general.layout master.mfact master.orientation ws:3\""
        ));
        // Chaque option désigne bien une option de Hyprland, et se vérifie.
        for o in OPTIONS {
            assert!(SECTIONS.contains(&o.section), "{}", o.path());
        }
        let o = opt("master.mfact").unwrap();
        assert_eq!(o.parse("0.5"), Ok(OptValue::Num(0.5)));
        assert!(o.parse("2").is_err());
        assert_eq!(
            opt("dwindle.force_split").unwrap().parse("2"),
            Ok(OptValue::Num(2.0))
        );
        assert!(
            opt("scrolling.direction")
                .unwrap()
                .parse("sideways")
                .is_err()
        );
    }
}

// ---------------------------------------------------------------------------
// Raccourcis : ceux de hyprland.lua qui agissent sur les layouts, et ceux que
// hyprdesk propose pour les actions propres à chaque layout.

/// Action utile pour un layout.
pub struct Action {
    /// Layout concerné (`""` : tous).
    pub layout: &'static str,
    /// Dispatcher : `layout` (message de layout), `focus`, `move`, `resize`,
    /// `fullscreen`, `float`.
    pub dsp: &'static str,
    /// Argument : message de layout, ou direction.
    pub arg: &'static str,
    pub label: &'static str,
    /// Raccourci proposé s'il n'y en a pas.
    pub suggest: Option<&'static str>,
}

const fn act(
    layout: &'static str,
    dsp: &'static str,
    arg: &'static str,
    label: &'static str,
    suggest: Option<&'static str>,
) -> Action {
    Action {
        layout,
        dsp,
        arg,
        label,
        suggest,
    }
}

/// Actions par layout (messages de layout de Hyprland 0.57).
pub const ACTIONS: &[Action] = &[
    // Tous les layouts.
    act("", "focus", "left", "Fenêtre de gauche", None),
    act("", "focus", "right", "Fenêtre de droite", None),
    act("", "focus", "up", "Fenêtre du dessus", None),
    act("", "focus", "down", "Fenêtre du dessous", None),
    act("", "move", "left", "Déplacer la fenêtre à gauche", None),
    act("", "move", "right", "Déplacer la fenêtre à droite", None),
    act("", "move", "up", "Déplacer la fenêtre en haut", None),
    act("", "move", "down", "Déplacer la fenêtre en bas", None),
    act("", "resize", "", "Redimensionner la fenêtre", None),
    act("", "fullscreen", "", "Plein écran", None),
    act("", "float", "", "Fenêtre flottante", None),
    // Dwindle.
    act(
        "dwindle",
        "layout",
        "togglesplit",
        "Basculer le sens de la découpe",
        Some("SUPER + ALT + S"),
    ),
    act(
        "dwindle",
        "layout",
        "swapsplit",
        "Échanger les deux moitiés",
        Some("SUPER + ALT + X"),
    ),
    act(
        "dwindle",
        "layout",
        "movetoroot",
        "Remonter la fenêtre à la racine",
        Some("SUPER + ALT + R"),
    ),
    // Master.
    act(
        "master",
        "layout",
        "swapwithmaster",
        "Échanger avec la fenêtre maître",
        Some("SUPER + ALT + Return"),
    ),
    act(
        "master",
        "layout",
        "focusmaster",
        "Aller à la fenêtre maître",
        Some("SUPER + ALT + M"),
    ),
    act(
        "master",
        "layout",
        "cyclenext",
        "Fenêtre suivante",
        Some("SUPER + ALT + N"),
    ),
    act(
        "master",
        "layout",
        "cycleprev",
        "Fenêtre précédente",
        Some("SUPER + ALT + P"),
    ),
    act(
        "master",
        "layout",
        "addmaster",
        "Une fenêtre maître de plus",
        Some("SUPER + ALT + I"),
    ),
    act(
        "master",
        "layout",
        "removemaster",
        "Une fenêtre maître de moins",
        Some("SUPER + ALT + U"),
    ),
    act(
        "master",
        "layout",
        "orientationnext",
        "Changer la place du maître",
        Some("SUPER + ALT + O"),
    ),
    act(
        "master",
        "layout",
        "rollnext",
        "Faire tourner les fenêtres",
        Some("SUPER + ALT + T"),
    ),
    // Scrolling : focus gauche/droite change de colonne, déplacer à gauche/droite
    // change la fenêtre de colonne.
    act(
        "scrolling",
        "layout",
        "move -col",
        "Défiler d'une colonne vers la gauche",
        Some("SUPER + ALT + left"),
    ),
    act(
        "scrolling",
        "layout",
        "move +col",
        "Défiler d'une colonne vers la droite",
        Some("SUPER + ALT + right"),
    ),
    act(
        "scrolling",
        "layout",
        "colresize +conf",
        "Colonne plus large (largeurs prédéfinies)",
        Some("SUPER + ALT + equal"),
    ),
    act(
        "scrolling",
        "layout",
        "colresize -conf",
        "Colonne plus étroite (largeurs prédéfinies)",
        Some("SUPER + ALT + minus"),
    ),
    act(
        "scrolling",
        "layout",
        "fit active",
        "Ajuster la colonne active à l'écran",
        Some("SUPER + ALT + F"),
    ),
    act(
        "scrolling",
        "layout",
        "promote",
        "Sortir la fenêtre dans sa propre colonne",
        Some("SUPER + ALT + P"),
    ),
    act(
        "scrolling",
        "layout",
        "consume_or_expel",
        "Joindre ou sortir de la colonne voisine",
        Some("SUPER + ALT + C"),
    ),
    // Monocle.
    act(
        "monocle",
        "layout",
        "cyclenext",
        "Fenêtre suivante",
        Some("SUPER + ALT + N"),
    ),
    act(
        "monocle",
        "layout",
        "cycleprev",
        "Fenêtre précédente",
        Some("SUPER + ALT + P"),
    ),
];

/// Raccourci lu dans hyprland.lua.
#[derive(Debug, Clone, PartialEq)]
pub struct Bind {
    /// `SUPER + S`.
    pub combo: String,
    /// `layout`, `focus`, `move`, `resize`, `fullscreen`, `float`, ou autre.
    pub dsp: String,
    /// Message de layout ou direction.
    pub arg: String,
}

/// Combinaison normalisée pour comparer (`super + alt + s`).
fn norm(combo: &str) -> String {
    combo
        .split('+')
        .map(|p| p.trim().to_lowercase())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Raccourcis de hyprland.lua (`hl.bind(combo, hl.dsp.…)`), variables
/// `local x = "…"` résolues. Les raccourcis trop dynamiques sont ignorés.
pub fn parse_binds(src: &str) -> Vec<Bind> {
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    let mut out = Vec::new();
    for line in src.lines() {
        let l = line.trim();
        if l.starts_with("--") {
            continue;
        }
        if let Some(rest) = l.strip_prefix("local ")
            && let Some((name, value)) = rest.split_once('=')
        {
            let v = value.trim();
            if let Some(s) = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                vars.insert(name.trim().to_string(), s.to_string());
            }
            continue;
        }
        let Some(i) = l.find("hl.bind(") else {
            continue;
        };
        let rest = &l[i + "hl.bind(".len()..];
        // Combinaison : jusqu'à la première virgule hors chaîne.
        let mut in_str = false;
        let Some(end) = rest.char_indices().find_map(|(i, c)| {
            if c == '"' {
                in_str = !in_str;
            }
            (c == ',' && !in_str).then_some(i)
        }) else {
            continue;
        };
        let combo: Option<String> = rest[..end]
            .split("..")
            .map(|part| {
                let p = part.trim();
                p.strip_prefix('"')
                    .and_then(|p| p.strip_suffix('"'))
                    .map(str::to_string)
                    .or_else(|| vars.get(p).cloned())
            })
            .collect();
        let Some(combo) = combo else { continue };
        let action = rest[end + 1..].trim();
        let Some(call) = action.strip_prefix("hl.dsp.") else {
            continue;
        };
        let name = call.split('(').next().unwrap_or("");
        let dsp = match name {
            "window.move" => "move",
            "window.resize" => "resize",
            "window.fullscreen" | "fullscreen" => "fullscreen",
            "window.float" | "float" | "toggle_float" => "float",
            other => other.rsplit('.').next().unwrap_or(other),
        };
        let first_str = |s: &str| -> Option<String> {
            let a = s.find('"')? + 1;
            let b = s[a..].find('"')? + a;
            Some(s[a..b].to_string())
        };
        let args = call.split_once('(').map_or("", |(_, a)| a);
        let arg = if dsp == "layout" {
            first_str(args).unwrap_or_default()
        } else if let Some(d) = args.split("direction").nth(1) {
            first_str(d).unwrap_or_default()
        } else {
            String::new()
        };
        // Un déplacement vers un workspace n'est pas une action de layout.
        if (dsp == "move" || dsp == "focus") && arg.is_empty() {
            continue;
        }
        out.push(Bind {
            combo: combo
                .split('+')
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(" + "),
            dsp: dsp.to_string(),
            arg,
        });
    }
    out
}

/// Ligne d'un layout : action, raccourci lié (s'il y en a), proposition.
pub struct KeyLine {
    pub label: &'static str,
    pub bound: Vec<String>,
    /// Raccourci proposé (non lié) et s'il est libre.
    pub suggest: Option<(&'static str, bool)>,
    /// Ligne à ajouter à hyprland.lua pour la proposition.
    pub lua: Option<String>,
}

/// Raccourcis d'un layout : les communs, puis les siens.
pub fn key_lines(layout: &str, binds: &[Bind]) -> Vec<KeyLine> {
    ACTIONS
        .iter()
        .filter(|a| a.layout.is_empty() || a.layout == layout)
        .map(|a| {
            let bound: Vec<String> = binds
                .iter()
                .filter(|b| b.dsp == a.dsp && (a.arg.is_empty() || b.arg == a.arg))
                .map(|b| b.combo.clone())
                .collect();
            let suggest = a.suggest.filter(|_| bound.is_empty()).map(|s| {
                let free = !binds.iter().any(|b| norm(&b.combo) == norm(s));
                (s, free)
            });
            let lua =
                suggest.map(|(s, _)| format!("hl.bind(\"{s}\", hl.dsp.layout(\"{}\"))", a.arg));
            KeyLine {
                label: a.label,
                bound,
                suggest,
                lua,
            }
        })
        // Les actions communes sans raccourci n'apportent rien à l'affichage.
        .filter(|l| !l.bound.is_empty() || l.suggest.is_some())
        .collect()
}

/// Raccourcis de hyprland.lua (dossier de la config de hyprdesk).
pub fn load_binds(config_path: &std::path::Path) -> Vec<Bind> {
    let dir = config_path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    // hyprland.lua, puis binds.lua où hyprdesk range les raccourcis (MLG-111).
    ["hyprland.lua", crate::binds_file::FILE]
        .iter()
        .filter_map(|f| std::fs::read_to_string(dir.join(f)).ok())
        .flat_map(|s| parse_binds(&s))
        .collect()
}

#[cfg(test)]
mod binds_tests {
    use super::*;

    /// Aperçu sur le hyprland.lua du poste : `cargo test -- --ignored --nocapture apercu_raccourcis`.
    #[test]
    #[ignore]
    fn apercu_raccourcis() {
        let home = std::env::var("HOME").unwrap();
        let b = load_binds(&std::path::Path::new(&home).join(".config/hypr/hyprdesk.lua"));
        for layout in ["dwindle", "master", "scrolling", "monocle"] {
            println!("== {layout}");
            for l in key_lines(layout, &b) {
                println!("  {:<48} {:?} {:?}", l.label, l.bound, l.suggest);
            }
        }
    }

    #[test]
    fn raccourcis_de_hyprland_lua() {
        let src = r#"
local mainMod = "SUPER"
hl.bind(mainMod .. " + S",         hl.dsp.layout("togglesplit"))
hl.bind(mainMod .. " + h", hl.dsp.focus({ direction = "left" }))
hl.bind(mainMod .. " + SHIFT + left",  hl.dsp.window.move({ direction = "left" }))
hl.bind(mainMod .. " + ALT + ampersand", hl.dsp.window.move({ workspace = 1 }))
hl.bind("SUPER + comma", hl.dsp.layout("move -col"))
-- hl.bind("SUPER + X", hl.dsp.layout("swapsplit"))
"#;
        let b = parse_binds(src);
        assert_eq!(b.len(), 4, "{b:?}");
        assert_eq!(b[0].combo, "SUPER + S");
        assert_eq!(
            (b[0].dsp.as_str(), b[0].arg.as_str()),
            ("layout", "togglesplit")
        );
        assert_eq!((b[1].dsp.as_str(), b[1].arg.as_str()), ("focus", "left"));
        assert_eq!((b[2].dsp.as_str(), b[2].arg.as_str()), ("move", "left"));
        // Dwindle : togglesplit lié, swapsplit proposé (libre).
        let d = key_lines("dwindle", &b);
        let t = d
            .iter()
            .find(|l| l.label.contains("sens de la découpe"))
            .unwrap();
        assert_eq!(t.bound, ["SUPER + S"]);
        let s = d.iter().find(|l| l.label.contains("deux moitiés")).unwrap();
        assert_eq!(s.suggest, Some(("SUPER + ALT + X", true)));
        assert!(
            s.lua
                .as_deref()
                .unwrap()
                .contains(r#"hl.dsp.layout("swapsplit")"#)
        );
        // Scrolling : la proposition déjà prise est signalée.
        let sc = key_lines("scrolling", &b);
        let left = sc
            .iter()
            .find(|l| l.label.contains("vers la gauche"))
            .unwrap();
        assert_eq!(left.bound, ["SUPER + comma"]);
        let right = sc
            .iter()
            .find(|l| l.label.contains("vers la droite"))
            .unwrap();
        assert_eq!(right.suggest, Some(("SUPER + ALT + right", true)));
    }
}
