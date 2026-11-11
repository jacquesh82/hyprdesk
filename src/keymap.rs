//! Raccourcis de hyprdesk face à ceux de hyprland.lua (MLG-40) : où chacun
//! agit, ce que hyprdesk prend à hyprland.lua, et les conflits — deux
//! actions de hyprdesk sur la même combinaison, une combinaison déjà prise
//! par hyprland.lua, une combinaison `submap_universal` qui agirait en double
//! bureau affiché.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::config::Keys;
use crate::hypr::{LOCK_KEY_DEFAULT, MEDIA_KEYS, parse_combo};

/// Où agit un raccourci de hyprdesk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Hors plein bureau et bureau affiché (submap « hyprdesk »).
    Everywhere,
    /// Hors plein bureau seulement.
    Outside,
    /// Bureau affiché seulement : les raccourcis de Hyprland y sont suspendus.
    Desktop,
}

impl Scope {
    pub fn outside(self) -> bool {
        self != Scope::Desktop
    }
    pub fn desktop(self) -> bool {
        self != Scope::Outside
    }
    fn overlaps(self, o: Scope) -> bool {
        (self.outside() && o.outside()) || (self.desktop() && o.desktop())
    }
    pub fn label(self) -> &'static str {
        match self {
            Scope::Everywhere => "partout",
            Scope::Outside => "hors plein bureau",
            Scope::Desktop => "bureau affiché",
        }
    }
}

/// Une action de hyprdesk et sa combinaison.
#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    /// Identifiant unique (« switch-back »).
    pub id: &'static str,
    /// Clé de configuration (`keys.switch`).
    pub key: &'static str,
    /// Libellé, en français : traduit à l'affichage.
    pub label: &'static str,
    pub combo: String,
    pub scope: Scope,
}

/// Libellés des actions (catalogue anglais vérifié par `libelles_traduits`).
pub const LABELS: &[&str] = &[
    "full-desktop (bureau complet)",
    "launcher-only (lanceur seul)",
    "no-desktop (masquer)",
    "Ouvrir les réglages",
    "Verrouiller la session",
    "Sortie de secours (bureau affiché)",
    "Fenêtres (Alt+Tab)",
    "Fenêtres, en arrière",
    "Lanceur d'applications",
    "Lanceur d'applications (plein bureau)",
    "Vue d'ensemble des fenêtres",
    "partout",
    "hors plein bureau",
    "bureau affiché",
];

/// Combinaison comparable : (masque des modificateurs, touche en minuscules).
pub fn norm(combo: &str) -> Option<(u32, String)> {
    parse_combo(combo)
}

fn same(a: &str, b: &str) -> bool {
    match (norm(a), norm(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// Les raccourcis de hyprdesk, dans l'ordre des Réglages.
pub fn actions(k: &Keys) -> Vec<Action> {
    let mut v = Vec::new();
    let mut push = |id, key, label, combo: &str, scope| {
        let combo = combo.trim();
        if !combo.is_empty() {
            v.push(Action {
                id,
                key,
                label,
                combo: combo.to_string(),
                scope,
            });
        }
    };
    use Scope::*;
    push("toggle", "keys.toggle", LABELS[0], &k.toggle, Everywhere);
    push(
        "launcher",
        "keys.launcher",
        LABELS[1],
        &k.launcher,
        Everywhere,
    );
    push("hide", "keys.hide", LABELS[2], &k.hide, Everywhere);
    push(
        "settings",
        "keys.settings",
        LABELS[3],
        &k.settings,
        Everywhere,
    );
    if k.search_desktop.trim().is_empty() {
        push("search", "keys.search", LABELS[8], &k.search, Everywhere);
    } else {
        push("search", "keys.search", LABELS[8], &k.search, Outside);
        push(
            "search-desktop",
            "keys.search_desktop",
            LABELS[9],
            &k.search_desktop,
            Desktop,
        );
    }
    push("switch", "keys.switch", LABELS[6], &k.switch, Everywhere);
    let back = crate::switcher::with_shift(&k.switch).unwrap_or_default();
    push("switch-back", "keys.switch", LABELS[7], &back, Everywhere);
    push(
        "overview",
        "keys.overview",
        LABELS[10],
        &k.overview,
        Everywhere,
    );
    // Verrouillage non réglé : celui de hyprland.lua hors du bureau, celui
    // de hyprdesk bureau affiché (même combinaison).
    if k.lock.trim().is_empty() {
        push("lock", "keys.lock", LABELS[4], LOCK_KEY_DEFAULT, Desktop);
    } else {
        push("lock", "keys.lock", LABELS[4], &k.lock, Everywhere);
    }
    push("leave", "keys.leave", LABELS[5], &k.leave, Desktop);
    v
}

/// Raccourci de Hyprland qui n'est pas à hyprdesk (hors submap).
#[derive(Debug, Clone, PartialEq)]
pub struct Foreign {
    pub combo: (u32, String),
    /// `submap_universal` : actif aussi bureau affiché.
    pub universal: bool,
    /// Dispatcher (`exec_cmd`).
    pub what: String,
}

pub fn foreign(binds: &Value) -> Vec<Foreign> {
    binds
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["submap"].as_str().unwrap_or("").is_empty())
        .filter(|b| {
            !b["description"]
                .as_str()
                .unwrap_or("")
                .starts_with("hyprdesk")
        })
        .filter_map(|b| {
            let mask = b["modmask"].as_u64()? as u32;
            let key = b["key"].as_str()?.to_lowercase();
            if key.is_empty() {
                return None;
            }
            Some(Foreign {
                combo: (mask, key),
                universal: matches!(&b["submap_universal"], Value::Bool(true))
                    || b["submap_universal"].as_str() == Some("true"),
                what: b["dispatcher"]
                    .as_str()
                    .unwrap_or("?")
                    .replace("HL.Dispatcher(", "")
                    .replace(')', ""),
            })
        })
        .collect()
}

/// Un `hl.bind` lu dans hyprland.lua : sa ligne et ce qu'il lance.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceBind {
    pub combo: (u32, String),
    /// Numéro de ligne (à partir de 1).
    pub line: usize,
    /// La commande lancée (`rofi -show drun`), sinon le dispatcher (`window.float`).
    pub action: String,
}

/// Coupe `s` sur `sep` hors des chaînes et des parenthèses.
fn split_top(s: &str, sep: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quote, mut depth) = (None::<char>, 0i32);
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(' | '{' | '[') => depth += 1,
            (None, ')' | '}' | ']') => depth -= 1,
            (None, _) if depth == 0 && s[i..].starts_with(sep) => {
                out.push(std::mem::take(&mut cur));
                for _ in 1..sep.len() {
                    it.next();
                }
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out
}

/// Concaténation de chaînes et de locales connues (`mainMod .. " + R"`).
fn eval(expr: &str, vars: &BTreeMap<String, String>) -> Option<String> {
    let mut out = String::new();
    for part in split_top(expr, "..") {
        let p = part.trim();
        let quoted = p
            .strip_prefix('"')
            .and_then(|p| p.strip_suffix('"'))
            .or_else(|| p.strip_prefix('\'').and_then(|p| p.strip_suffix('\'')));
        match quoted {
            Some(s) => out += s,
            None => out += vars.get(p)?,
        }
    }
    Some(out)
}

/// Contenu de la parenthèse ouverte au début de `s` (`(a, b(c)) …` → `a, b(c)`).
fn paren(s: &str) -> Option<&str> {
    let s = s.strip_prefix('(')?;
    let (mut quote, mut depth) = (None::<char>, 0i32);
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') if depth == 0 => return Some(&s[..i]),
            (None, ')') => depth -= 1,
            _ => {}
        }
    }
    None
}

pub fn parse_source(src: &str) -> Vec<SourceBind> {
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    let mut out = Vec::new();
    for (n, line) in src.lines().enumerate() {
        let l = line.trim();
        if l.starts_with("--") {
            continue;
        }
        if let Some(rest) = l.strip_prefix("local ")
            && let Some((name, value)) = rest.split_once('=')
            && !name.contains(',')
        {
            if let Some(v) = eval(value.trim(), &vars) {
                vars.insert(name.trim().to_string(), v);
            }
            continue;
        }
        let Some(i) = l.find("hl.bind(") else {
            continue;
        };
        let args = split_top(&l[i + "hl.bind(".len()..], ",");
        let Some(combo) = args.first().and_then(|a| eval(a, &vars)) else {
            continue;
        };
        let Some(combo) = norm(&combo) else {
            continue;
        };
        let call = args.get(1).map(|a| a.trim()).unwrap_or("");
        let action = if let Some(inner) = call
            .strip_prefix("hl.dsp.exec_cmd")
            .and_then(|c| paren(c.trim_start()))
        {
            eval(inner, &vars).unwrap_or_else(|| inner.trim().to_string())
        } else if call.starts_with("function") {
            "function".to_string()
        } else {
            call.trim_start_matches("hl.dsp.")
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        };
        out.push(SourceBind {
            combo,
            line: n + 1,
            action,
        });
    }
    out
}

/// Raccourcis de hyprland.lua (dossier de la config de hyprdesk).
pub fn load_source(config_path: &Path) -> Vec<SourceBind> {
    let dir = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    std::fs::read_to_string(dir.join("hyprland.lua"))
        .map(|s| parse_source(&s))
        .unwrap_or_default()
}

/// Ce qui tient une combinaison dans hyprland.lua.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Holder {
    pub what: String,
    pub line: Option<usize>,
}

impl Holder {
    /// « rofi -show drun » (hyprland.lua, ligne 232)
    pub fn describe(&self) -> String {
        let what = if self.what == "function" {
            crate::i18n::tr("une fonction Lua")
        } else {
            self.what.as_str()
        };
        match self.line {
            Some(n) => crate::tr!("« {what} » (hyprland.lua, ligne {n})", what = what, n = n),
            None => crate::tr!("« {what} » (hyprland.lua)", what = what),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Même combinaison qu'une autre action de hyprdesk (son libellé) :
    /// celle-ci n'est pas posée.
    Duplicate(&'static str),
    /// Liée dans hyprland.lua : hyprdesk ne la pose que bureau affiché.
    Hyprland(Holder),
    /// Liée dans hyprland.lua avec `submap_universal` : elle y agit partout,
    /// hyprdesk ne la pose pas (sinon, bureau affiché, les deux agiraient).
    Universal(Holder),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub action: Action,
    pub kind: Kind,
}

impl Issue {
    pub fn holder(&self) -> Option<&Holder> {
        match &self.kind {
            Kind::Hyprland(h) | Kind::Universal(h) => Some(h),
            Kind::Duplicate(_) => None,
        }
    }

    pub fn message(&self) -> String {
        let k = &self.action.combo;
        let label = crate::i18n::tr(self.action.label);
        match &self.kind {
            Kind::Duplicate(other) => crate::tr!(
                "{k} sert déjà à « {other} » : « {label} » n'est pas installé. Choisissez une autre combinaison.",
                k = k,
                other = crate::i18n::tr(other),
                label = label
            ),
            Kind::Hyprland(h) => crate::tr!(
                "{k} est déjà lié à {h} : « {label} » n'agit que bureau affiché, là où les raccourcis de Hyprland sont suspendus.",
                k = k,
                h = h.describe(),
                label = label
            ),
            Kind::Universal(h) => crate::tr!(
                "{k} est lié à {h} avec submap_universal, qui agit aussi bureau affiché : « {label} » n'est installé nulle part.",
                k = k,
                h = h.describe(),
                label = label
            ),
        }
    }
}

/// Une combinaison de hyprland.lua prise par hyprdesk, ou laissée à
/// hyprland.lua hors plein bureau (lanceur non unifié).
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    pub action: Action,
    pub holder: Holder,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub issues: Vec<Issue>,
    /// Prises à hyprland.lua (`keys.take`, verrouillage réglé, lanceur unifié).
    pub taken: Vec<Held>,
    /// Laissées à hyprland.lua hors plein bureau.
    pub left: Vec<Held>,
    /// Raccourcis de Hyprland `submap_universal`, lisibles.
    pub universal: Vec<String>,
}

impl Report {
    /// Conflit sur une clé de configuration (`keys.search`).
    pub fn issue_for(&self, key: &str) -> Option<&Issue> {
        self.issues.iter().find(|i| i.action.key == key)
    }
}

/// Ce que l'installation des raccourcis en déduit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Plan {
    /// Combinaisons déliées de hyprland.lua avant de poser celles de hyprdesk.
    pub take: Vec<String>,
    skip_outside: Vec<(u32, String)>,
    skip_desktop: Vec<(u32, String)>,
    /// Actions en doublon : non posées.
    dropped: Vec<&'static str>,
}

impl Plan {
    /// Le raccourci `combo` de l'action `id` se pose hors plein bureau.
    pub fn outside(&self, id: &str, combo: &str) -> bool {
        !combo.is_empty()
            && !self.dropped.contains(&id)
            && norm(combo).is_none_or(|n| !self.skip_outside.contains(&n))
    }
    /// … et bureau affiché.
    pub fn desktop(&self, id: &str, combo: &str) -> bool {
        !combo.is_empty()
            && !self.dropped.contains(&id)
            && norm(combo).is_none_or(|n| !self.skip_desktop.contains(&n))
    }
    /// Pour les essais : combinaisons laissées à hyprland.lua hors du bureau.
    #[cfg(test)]
    pub fn skipping(combos: &[&str]) -> Self {
        Plan {
            skip_outside: combos.iter().filter_map(|c| norm(c)).collect(),
            ..Default::default()
        }
    }
}

/// Combinaisons que hyprdesk prend à hyprland.lua : `keys.take`, le
/// verrouillage réglé, le lanceur unifié — seulement celles d'une action
/// qui agit hors plein bureau.
pub fn wanted(keys: &Keys, unified: bool) -> Vec<String> {
    let acts = actions(keys);
    let mut v: Vec<String> = keys.take.clone();
    if !keys.lock.trim().is_empty() {
        v.push(keys.lock.trim().to_string());
    }
    if unified && !keys.search.trim().is_empty() {
        v.push(keys.search.trim().to_string());
    }
    let mut out: Vec<String> = Vec::new();
    for c in v {
        if acts.iter().any(|a| a.scope.outside() && same(&a.combo, &c))
            && !out.iter().any(|o| same(o, &c))
        {
            out.push(c);
        }
    }
    out
}

/// Analyse les raccourcis de hyprdesk face à ceux de Hyprland (`j/binds`)
/// et à hyprland.lua (lignes, commandes).
pub fn analyse(keys: &Keys, unified: bool, binds: &Value, source: &[SourceBind]) -> (Report, Plan) {
    let acts = actions(keys);
    let foreign = foreign(binds);
    let mut report = Report::default();
    let mut plan = Plan {
        take: wanted(keys, unified),
        ..Default::default()
    };
    let holder = |n: &(u32, String), f: Option<&Foreign>| -> Option<Holder> {
        match (source.iter().find(|s| &s.combo == n), f) {
            (Some(s), _) => Some(Holder {
                what: s.action.clone(),
                line: Some(s.line),
            }),
            (None, Some(f)) => Some(Holder {
                what: f.what.clone(),
                line: None,
            }),
            _ => None,
        }
    };
    for (i, a) in acts.iter().enumerate() {
        let Some(n) = norm(&a.combo) else { continue };
        if let Some(b) = acts[..i].iter().find(|b| {
            !plan.dropped.contains(&b.id) && same(&b.combo, &a.combo) && b.scope.overlaps(a.scope)
        }) {
            plan.dropped.push(a.id);
            report.issues.push(Issue {
                action: a.clone(),
                kind: Kind::Duplicate(b.label),
            });
            continue;
        }
        let f = foreign.iter().find(|f| f.combo == n);
        if a.scope.outside() && plan.take.iter().any(|t| same(t, &a.combo)) {
            // Déjà déliée lors d'une installation précédente : hyprland.lua le dit encore.
            if let Some(h) = holder(&n, f) {
                report.taken.push(Held {
                    action: a.clone(),
                    holder: h,
                });
            }
            continue;
        }
        let Some(f) = f else { continue };
        // Bureau affiché seulement, face à un raccourci suspendu : aucun conflit.
        if !f.universal && !a.scope.outside() {
            continue;
        }
        let h = holder(&n, Some(f)).unwrap_or_default();
        if a.scope.outside() && !plan.skip_outside.contains(&n) {
            plan.skip_outside.push(n.clone());
        }
        if f.universal && a.scope.desktop() && !plan.skip_desktop.contains(&n) {
            plan.skip_desktop.push(n.clone());
        }
        if a.id == "search" && !unified && !f.universal {
            report.left.push(Held {
                action: a.clone(),
                holder: h,
            });
            continue;
        }
        report.issues.push(Issue {
            action: a.clone(),
            kind: if f.universal {
                Kind::Universal(h)
            } else {
                Kind::Hyprland(h)
            },
        });
    }
    // Touches multimédia : celles de hyprland.lua gardent la main hors du
    // bureau ; marquées submap_universal, bureau affiché aussi (sinon une
    // capture d'écran partirait deux fois).
    for (k, _, _) in MEDIA_KEYS {
        let Some(n) = norm(k) else { continue };
        if let Some(f) = foreign.iter().find(|f| f.combo == n) {
            if !plan.skip_outside.contains(&n) {
                plan.skip_outside.push(n.clone());
            }
            if f.universal && !plan.skip_desktop.contains(&n) {
                plan.skip_desktop.push(n);
            }
        }
    }
    (report, plan)
}

/// L'action réglée par `key` (`keys.search`) partage sa combinaison avec
/// une autre action de hyprdesk : le libellé de celle-ci.
pub fn duplicate_of(keys: &Keys, unified: bool, key: &str) -> Option<&'static str> {
    let mine: Vec<&'static str> = actions(keys)
        .into_iter()
        .filter(|a| a.key == key)
        .map(|a| a.label)
        .collect();
    let (report, _) = analyse(keys, unified, &Value::Null, &[]);
    report.issues.iter().find_map(|i| match i.kind {
        Kind::Duplicate(other) if i.action.key == key => Some(other),
        Kind::Duplicate(other) if mine.contains(&other) => Some(i.action.label),
        _ => None,
    })
}

/// Demande à LYRA de résoudre les conflits.
pub fn lyra_prompt(report: &Report) -> String {
    let mut list = String::new();
    for i in &report.issues {
        list += &format!("- {}\n", i.message());
    }
    crate::tr!(
        "Dans ma session Hyprland, des raccourcis de hyprdesk sont en conflit :\n{list}\nLis ~/.config/hypr/hyprland.lua et `hyprdesk keys`. Pour chaque conflit, propose la meilleure issue : une autre combinaison pour hyprdesk (`hyprdesk config set keys.<nom> \"…\"`), la laisser prendre par hyprdesk (`hyprdesk config set keys.take \"…\"`), ou déplacer ou retirer la ligne de hyprland.lua. Demande-moi avant de modifier quoi que ce soit, et ne touche jamais à inputs.lua ni à monitors.lua.",
        list = list
    )
}

/// `hyprdesk keys` : chaque raccourci, où il agit, et les conflits.
pub fn text_report(keys: &Keys, report: &Report) -> String {
    let mut o = crate::i18n::tr("Raccourcis de hyprdesk\n\n").to_string();
    for a in actions(keys) {
        let state = if let Some(i) = report.issues.iter().find(|i| i.action.id == a.id) {
            match i.kind {
                Kind::Duplicate(_) => crate::i18n::tr("doublon, non installé"),
                Kind::Hyprland(_) => crate::i18n::tr("bureau affiché seulement (conflit)"),
                Kind::Universal(_) => crate::i18n::tr("non installé (conflit)"),
            }
            .to_string()
        } else if report.taken.iter().any(|h| h.action.id == a.id) {
            crate::tr!(
                "{s}, pris à hyprland.lua",
                s = crate::i18n::tr(a.scope.label())
            )
        } else if report.left.iter().any(|h| h.action.id == a.id) {
            crate::i18n::tr("bureau affiché ; hyprland.lua hors plein bureau").to_string()
        } else {
            crate::i18n::tr(a.scope.label()).to_string()
        };
        o += &format!("  {:<22} {} — {state}\n", a.combo, crate::i18n::tr(a.label));
    }
    if report.issues.is_empty() {
        o += crate::i18n::tr("\nAucun conflit.\n");
    } else {
        o += crate::i18n::tr("\nConflits\n");
        for i in &report.issues {
            o += &format!("  - {}\n", i.message());
        }
    }
    for (title, list) in [
        (crate::i18n::tr("\nPris à hyprland.lua\n"), &report.taken),
        (
            crate::i18n::tr("\nLaissés à hyprland.lua hors plein bureau\n"),
            &report.left,
        ),
    ] {
        if !list.is_empty() {
            o += title;
            for h in list {
                o += &format!("  - {} : {}\n", h.action.combo, h.holder.describe());
            }
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    const LUA: &str = r#"
local mainMod = "SUPER"
local scriptsDir = "/home/u/.config/hypr/scripts"
local menu       = "rofi -show drun -show-icons"
hl.bind("Print", hl.dsp.exec_cmd("hyprshot -m region"), { submap_universal = true })
-- hl.bind(mainMod .. " + D", hl.dsp.exec_cmd("commentée"))
hl.bind(mainMod .. " + V",         hl.dsp.exec_cmd(scriptsDir .. "/clipboard"))
hl.bind(mainMod .. " + R",         hl.dsp.exec_cmd(menu))
hl.bind(mainMod .. " + Space",     hl.dsp.window.float({ action = "toggle" }))
hl.bind(mainMod .. " + Tab", function()
"#;

    fn binds() -> Value {
        serde_json::from_str(
            r#"[
              {"modmask":0,"key":"Print","submap":"","submap_universal":"true","description":"","dispatcher":"HL.Dispatcher(exec_cmd)"},
              {"modmask":64,"key":"R","submap":"","submap_universal":"false","description":"","dispatcher":"HL.Dispatcher(exec_cmd)"},
              {"modmask":64,"key":"Space","submap":"","submap_universal":"false","description":"","dispatcher":"HL.Dispatcher(float)"},
              {"modmask":64,"key":"Tab","submap":"","submap_universal":"false","description":"","dispatcher":"function: 0x1"},
              {"modmask":64,"key":"Tab","submap":"hyprdesk","submap_universal":"false","description":"hyprdesk : vue d'ensemble","dispatcher":"HL.Dispatcher(exec_cmd)"}
            ]"#,
        )
        .unwrap()
    }

    #[test]
    fn lecture_de_hyprland_lua() {
        let s = parse_source(LUA);
        let r = s.iter().find(|b| b.combo == (64, "r".into())).unwrap();
        assert_eq!(
            (r.line, r.action.as_str()),
            (8, "rofi -show drun -show-icons")
        );
        let v = s.iter().find(|b| b.combo == (64, "v".into())).unwrap();
        assert_eq!(v.action, "/home/u/.config/hypr/scripts/clipboard");
        let f = s.iter().find(|b| b.combo == (64, "space".into())).unwrap();
        assert_eq!(f.action, "window.float");
        assert!(
            s.iter()
                .any(|b| b.combo == (64, "tab".into()) && b.action == "function")
        );
        assert!(
            !s.iter().any(|b| b.combo == (64, "d".into())),
            "ligne commentée ignorée"
        );
    }

    #[test]
    fn lanceur_unifie_ou_non() {
        let k = Keys::default();
        let src = parse_source(LUA);
        // Unifié (par défaut) : SUPER + R pris à hyprland.lua, rien en conflit.
        let (r, plan) = analyse(&k, true, &binds(), &src);
        assert!(plan.take.iter().any(|t| t == "SUPER + R"), "{plan:?}");
        assert!(plan.outside("search", "SUPER + R") && plan.desktop("search", "SUPER + R"));
        assert!(r.issue_for("keys.search").is_none(), "{r:?}");
        let held = &r
            .taken
            .iter()
            .find(|h| h.action.id == "search")
            .unwrap()
            .holder;
        assert_eq!(held.what, "rofi -show drun -show-icons");
        // Non unifié : rofi hors plein bureau, hyprdesk bureau affiché, sans conflit.
        let (r, plan) = analyse(&k, false, &binds(), &src);
        assert!(!plan.take.iter().any(|t| t == "SUPER + R"));
        assert!(!plan.outside("search", "SUPER + R") && plan.desktop("search", "SUPER + R"));
        assert!(
            r.issue_for("keys.search").is_none() && r.left.len() == 1,
            "{r:?}"
        );
        // Surcharge du plein bureau : une autre combinaison, bureau affiché seulement.
        let desk = Keys {
            search_desktop: "SUPER + A".into(),
            ..k.clone()
        };
        let a = actions(&desk);
        let s = a.iter().find(|a| a.id == "search").unwrap();
        let d = a.iter().find(|a| a.id == "search-desktop").unwrap();
        assert_eq!((s.scope, d.scope), (Scope::Outside, Scope::Desktop));
    }

    #[test]
    fn conflits() {
        let src = parse_source(LUA);
        let k = Keys {
            search: "SUPER + Space".into(),
            overview: "Print".into(),
            ..Keys::default()
        };
        let (r, plan) = analyse(&k, false, &binds(), &src);
        // Lanceur non unifié sur SUPER + Space : pris par window.float, laissé.
        assert_eq!(r.left.len(), 1);
        // Print marqué submap_universal : la vue d'ensemble n'est posée nulle part.
        let i = r.issue_for("keys.overview").unwrap();
        assert!(matches!(i.kind, Kind::Universal(_)), "{i:?}");
        assert!(!plan.outside("overview", "Print") && !plan.desktop("overview", "Print"));
        // SUPER + Tab de hyprland.lua face à la vue d'ensemble par défaut : conflit hors du bureau.
        let (r, plan) = analyse(&Keys::default(), true, &binds(), &src);
        let i = r.issue_for("keys.overview").unwrap();
        assert!(matches!(&i.kind, Kind::Hyprland(h) if h.what == "function" && h.line == Some(10)));
        assert!(
            !plan.outside("overview", "SUPER + Tab") && plan.desktop("overview", "SUPER + Tab")
        );
        // Prise en main : plus de conflit, la combinaison est déliée.
        let taken = Keys {
            take: vec!["SUPER + Tab".into()],
            ..Keys::default()
        };
        let (r, plan) = analyse(&taken, true, &binds(), &src);
        assert!(r.issue_for("keys.overview").is_none());
        assert!(plan.take.iter().any(|t| t == "SUPER + Tab"));
        // Une combinaison prise sans action de hyprdesk derrière n'est pas déliée.
        let useless = Keys {
            take: vec!["SUPER + V".into()],
            ..Keys::default()
        };
        assert!(!wanted(&useless, true).iter().any(|t| t == "SUPER + V"));
    }

    #[test]
    fn doublons_et_touches_multimedia() {
        let k = Keys {
            settings: "SUPER + D".into(),
            ..Keys::default()
        };
        let (r, plan) = analyse(&k, true, &Value::Null, &[]);
        let i = r.issue_for("keys.settings").unwrap();
        assert_eq!(i.kind, Kind::Duplicate("full-desktop (bureau complet)"));
        assert!(plan.outside("toggle", "SUPER + D") && !plan.outside("settings", "SUPER + D"));
        // Sortie de secours (bureau affiché) et un raccourci hors du bureau : pas de doublon.
        let k = Keys {
            leave: "SUPER + Q".into(),
            search: "SUPER + Q".into(),
            search_desktop: "SUPER + A".into(),
            ..Keys::default()
        };
        assert!(analyse(&k, true, &Value::Null, &[]).0.issues.is_empty());
        // Impr universel dans hyprland.lua : hyprdesk ne le pose nulle part
        // (la capture partirait deux fois bureau affiché).
        let (_, plan) = analyse(&Keys::default(), true, &binds(), &[]);
        assert!(!plan.outside("media", "Print") && !plan.desktop("media", "Print"));
        assert!(plan.outside("media", "XF86AudioMute") && plan.desktop("media", "XF86AudioMute"));
    }

    #[test]
    fn libelles_traduits() {
        let missing = crate::i18n::untranslated("keymap", LABELS.iter().copied());
        assert!(missing.is_empty(), "{missing:?}");
    }
}
