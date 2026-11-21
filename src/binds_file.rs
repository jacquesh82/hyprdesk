//! Raccourcis de hyprland.lua dans un fichier dédié (MLG-111).
//!
//! Au démarrage, hyprdesk sort les `hl.bind(…)` de premier niveau de
//! hyprland.lua (boucles qui en posent et raccourcis mis en commentaire
//! compris, avec les commentaires qui les annoncent) vers `binds.lua`, à côté,
//! que hyprland.lua charge par `require("binds")` à la place du premier
//! raccourci déplacé. Les variables locales dont ils se servent (`mainMod`,
//! `scriptsDir`…) les suivent : déplacées si plus rien ne s'en sert dans
//! hyprland.lua, copiées sinon.
//!
//! `binds.lua` appartient à l'utilisateur : hyprdesk le lit (Réglages →
//! Raccourcis) et n'y fait qu'ajouter, quand de nouveaux raccourcis
//! apparaissent dans hyprland.lua. Copie de hyprland.lua avant chaque
//! déplacement ; si Hyprland signale une nouvelle erreur après le
//! rechargement, les deux fichiers sont rendus tels qu'ils étaient.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Nom du fichier des raccourcis, à côté de hyprland.lua.
pub const FILE: &str = "binds.lua";

/// Ligne qui le charge.
const REQUIRE: &str = "require(\"binds\")";

/// `binds.lua`, à côté de `hyprdesk.lua`.
pub fn path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(FILE)
}

/// Une ligne, vue par le découpage : son code (chaînes vidées, commentaires
/// retirés) et si elle n'est que commentaire.
pub(crate) struct Lexed {
    pub(crate) code: String,
    /// Ligne de commentaire seul (ou dans un long commentaire).
    pub(crate) comment: bool,
}

/// Code de chaque ligne, sans chaînes ni commentaires (longs compris).
pub(crate) fn lex(src: &str) -> Vec<Lexed> {
    #[derive(Clone, Copy, PartialEq)]
    enum St {
        Code,
        LongComment(usize),
        LongString(usize),
    }
    let mut st = St::Code;
    let mut out = Vec::new();
    for line in src.lines() {
        let mut code = String::new();
        let mut had_code = false;
        let mut in_comment = !matches!(st, St::Code);
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        // Fin d'un long crochet de niveau `lvl` (`]==]`) à la position i.
        let closes = |i: usize, lvl: usize| {
            chars.get(i) == Some(&']')
                && (1..=lvl).all(|k| chars.get(i + k) == Some(&'='))
                && chars.get(i + lvl + 1) == Some(&']')
        };
        // Ouverture d'un long crochet (`[==[`) à la position i : son niveau.
        let opens = |i: usize| -> Option<usize> {
            if chars.get(i) != Some(&'[') {
                return None;
            }
            let mut lvl = 0;
            while chars.get(i + 1 + lvl) == Some(&'=') {
                lvl += 1;
            }
            (chars.get(i + 1 + lvl) == Some(&'[')).then_some(lvl)
        };
        while i < chars.len() {
            match st {
                St::LongComment(lvl) | St::LongString(lvl) => {
                    if closes(i, lvl) {
                        if matches!(st, St::LongString(_)) {
                            code.push('"');
                        }
                        i += lvl + 2;
                        st = St::Code;
                    } else {
                        i += 1;
                    }
                }
                St::Code => {
                    let c = chars[i];
                    if c == '-' && chars.get(i + 1) == Some(&'-') {
                        if let Some(lvl) = opens(i + 2) {
                            st = St::LongComment(lvl);
                            in_comment |= !had_code;
                            i += lvl + 4;
                            continue;
                        }
                        in_comment |= !had_code;
                        break;
                    }
                    if let Some(lvl) = opens(i) {
                        code.push('"');
                        had_code = true;
                        st = St::LongString(lvl);
                        i += lvl + 2;
                        continue;
                    }
                    if c == '"' || c == '\'' {
                        // Chaîne vidée : seuls ses guillemets restent.
                        code.push('"');
                        i += 1;
                        while i < chars.len() && chars[i] != c {
                            if chars[i] == '\\' {
                                i += 1;
                            }
                            i += 1;
                        }
                        code.push('"');
                        had_code = true;
                        i += 1;
                        continue;
                    }
                    if !c.is_whitespace() {
                        had_code = true;
                    }
                    code.push(c);
                    i += 1;
                }
            }
        }
        out.push(Lexed {
            comment: in_comment && !had_code,
            code,
        });
    }
    out
}

/// Mots (identifiants Lua) d'un bout de code.
fn words(code: &str) -> Vec<&str> {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty() && !w.starts_with(|c: char| c.is_ascii_digit()))
        .collect()
}

/// Variation de profondeur de bloc d'une ligne de code (`function`, `if`,
/// `do`, `repeat` ouvrent ; `end`, `until` ferment), et de parenthèses.
fn depth(code: &str) -> (i32, i32) {
    let mut block = 0;
    for w in words(code) {
        match w {
            "function" | "if" | "do" | "repeat" => block += 1,
            "end" | "until" => block -= 1,
            _ => {}
        }
    }
    let paren = code
        .chars()
        .map(|c| match c {
            '(' | '{' | '[' => 1,
            ')' | '}' | ']' => -1,
            _ => 0,
        })
        .sum();
    (block, paren)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Blank,
    Comment,
    /// Un `hl.bind(…)`, ou un bloc qui en pose.
    Bind,
    /// Commentaires contenant un `hl.bind` (raccourci mis de côté).
    CommentedBind,
    /// `local nom = …` ou `local function nom`.
    Local,
    Other,
}

/// Instruction de premier niveau : lignes [start, end).
pub(crate) struct Item {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: Kind,
    /// Variable locale définie.
    pub(crate) defines: Option<String>,
    /// Identifiants employés.
    pub(crate) uses: BTreeSet<String>,
}

pub(crate) fn items(src: &str) -> Vec<Item> {
    let lines: Vec<&str> = src.lines().collect();
    let lexed = lex(src);
    let mut out: Vec<Item> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim().is_empty() {
            out.push(Item {
                start: i,
                end: i + 1,
                kind: Kind::Blank,
                defines: None,
                uses: BTreeSet::new(),
            });
            i += 1;
            continue;
        }
        if lexed[i].comment {
            // Commentaires consécutifs : un seul élément.
            let start = i;
            while i < lines.len() && lexed[i].comment && !lines[i].trim().is_empty() {
                i += 1;
            }
            // Un long commentaire (`--[[ … ]]`) reste où il est : il tient
            // souvent tout un bloc mis de côté, pas seulement des raccourcis.
            let long = lines[start].trim_start().starts_with("--[");
            let bind = !long && lines[start..i].iter().any(|l| l.contains("hl.bind("));
            out.push(Item {
                start,
                end: i,
                kind: if bind {
                    Kind::CommentedBind
                } else {
                    Kind::Comment
                },
                defines: None,
                uses: BTreeSet::new(),
            });
            continue;
        }
        let start = i;
        let (mut block, mut paren) = (0, 0);
        let mut code = String::new();
        loop {
            let (b, p) = depth(&lexed[i].code);
            block += b;
            paren += p;
            code.push_str(&lexed[i].code);
            code.push('\n');
            i += 1;
            if (block <= 0 && paren <= 0) || i >= lines.len() {
                break;
            }
        }
        let t = code.trim_start();
        let defines = t.strip_prefix("local ").and_then(|rest| {
            let rest = rest.trim_start();
            let rest = rest.strip_prefix("function ").unwrap_or(rest);
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let after = rest[name.len()..].trim_start();
            (!name.is_empty() && !after.starts_with(',')).then_some(name)
        });
        let bind = code.contains("hl.bind(");
        let mut uses: BTreeSet<String> = words(&code).into_iter().map(str::to_string).collect();
        if let Some(d) = &defines {
            uses.remove(d);
        }
        out.push(Item {
            start,
            end: i,
            kind: if bind {
                Kind::Bind
            } else if defines.is_some() {
                Kind::Local
            } else {
                Kind::Other
            },
            defines,
            uses,
        });
    }
    out
}

/// Ce qu'un déplacement produit.
#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    /// hyprland.lua sans ses raccourcis, avec `require("binds")`.
    pub hyprland: String,
    /// Ce qui s'ajoute à binds.lua : locales nécessaires, puis raccourcis.
    pub binds: String,
    /// Raccourcis déplacés (`hl.bind` et blocs).
    pub moved: usize,
    /// Locales copiées (gardées aussi dans hyprland.lua).
    pub copied: Vec<String>,
    /// Locales déplacées.
    pub taken: Vec<String>,
}

/// Locales déjà définies dans un fichier (binds.lua existant).
fn defined(src: &str) -> BTreeSet<String> {
    items(src).into_iter().filter_map(|it| it.defines).collect()
}

/// Sépare les raccourcis de hyprland.lua. `existing` : binds.lua déjà là
/// (ses locales ne sont pas redéfinies). `None` : rien à déplacer, ou une
/// locale définie par un raccourci sert encore ailleurs.
pub fn split(src: &str, existing: Option<&str>) -> Option<Split> {
    let lines: Vec<&str> = src.lines().collect();
    let its = items(src);
    // Raccourcis, et commentaires qui les annoncent (collés au-dessus).
    let mut moved = vec![false; its.len()];
    for (k, it) in its.iter().enumerate() {
        if matches!(it.kind, Kind::Bind | Kind::CommentedBind) {
            moved[k] = true;
            let mut j = k;
            while j > 0 && its[j - 1].kind == Kind::Comment {
                j -= 1;
                moved[j] = true;
            }
        }
    }
    let count = its
        .iter()
        .zip(&moved)
        .filter(|(it, m)| **m && it.kind == Kind::Bind)
        .count();
    if count == 0 {
        return None;
    }
    // Une locale posée par un raccourci (`local b = hl.bind(…)`) qui sert
    // encore ailleurs : on ne touche à rien.
    let bind_locals: BTreeSet<&String> = its
        .iter()
        .zip(&moved)
        .filter(|(it, m)| **m && it.kind == Kind::Bind)
        .filter_map(|(it, _)| it.defines.as_ref())
        .collect();
    let rest_uses: BTreeSet<&String> = its
        .iter()
        .zip(&moved)
        .filter(|(it, m)| !**m && it.kind != Kind::Local)
        .flat_map(|(it, _)| it.uses.iter())
        .collect();
    if bind_locals.iter().any(|l| rest_uses.contains(l)) {
        return None;
    }
    // Locales dont les raccourcis ont besoin (de proche en proche).
    let locals: Vec<usize> = (0..its.len())
        .filter(|&k| its[k].kind == Kind::Local)
        .collect();
    let name = |k: usize| its[k].defines.clone().unwrap_or_default();
    let mut needed: BTreeSet<String> = its
        .iter()
        .zip(&moved)
        .filter(|(_, m)| **m)
        .flat_map(|(it, _)| it.uses.iter().cloned())
        .collect();
    loop {
        let before = needed.len();
        for &k in &locals {
            if needed.contains(&name(k)) {
                needed.extend(its[k].uses.iter().cloned());
            }
        }
        if needed.len() == before {
            break;
        }
    }
    // Locales qui restent utiles à hyprland.lua (de proche en proche).
    let mut kept: BTreeSet<String> = rest_uses.iter().map(|s| s.to_string()).collect();
    loop {
        let before = kept.len();
        for &k in &locals {
            if kept.contains(&name(k)) {
                kept.extend(its[k].uses.iter().cloned());
            }
        }
        if kept.len() == before {
            break;
        }
    }
    let already = existing.map(defined).unwrap_or_default();
    let mut copied = Vec::new();
    let mut taken = Vec::new();
    let mut local_lines: Vec<String> = Vec::new();
    for &k in &locals {
        let n = name(k);
        if !needed.contains(&n) {
            continue;
        }
        if !already.contains(&n) {
            local_lines.extend(
                lines[its[k].start..its[k].end]
                    .iter()
                    .map(|l| l.to_string()),
            );
        }
        if kept.contains(&n) {
            copied.push(n);
        } else {
            taken.push(n);
            moved[k] = true;
        }
    }
    // binds.lua : locales, puis raccourcis dans l'ordre, séparés comme avant.
    let mut binds: Vec<String> = Vec::new();
    if !local_lines.is_empty() {
        binds.extend(local_lines);
        binds.push(String::new());
    }
    let mut gap = false;
    let mut started = false;
    for (k, it) in its.iter().enumerate() {
        let is_local = it.kind == Kind::Local;
        if moved[k] && !is_local {
            if gap && started {
                binds.push(String::new());
            }
            binds.extend(lines[it.start..it.end].iter().map(|l| l.to_string()));
            started = true;
            gap = false;
        } else if it.kind == Kind::Blank {
            gap = true;
        } else if !moved[k] {
            // Une instruction gardée entre deux raccourcis : un saut de ligne.
            gap = true;
        }
    }
    // hyprland.lua : sans ce qui part, `require("binds")` à la place du
    // premier raccourci ; les lignes vides entre deux éléments partis aussi.
    let first = (0..its.len())
        .find(|&k| moved[k] && its[k].kind != Kind::Local)
        .unwrap_or(0);
    let has_require = lines.iter().any(|l| l.trim_start().starts_with(REQUIRE));
    let mut out: Vec<String> = Vec::new();
    for (k, it) in its.iter().enumerate() {
        if k == first && !has_require {
            out.push(format!(
                "{REQUIRE} {}",
                crate::i18n::tr(
                    "-- hyprdesk : raccourcis clavier dans binds.lua (Réglages → Raccourcis)"
                )
            ));
        }
        if moved[k] {
            continue;
        }
        if it.kind == Kind::Blank {
            // Entre deux éléments partis (ou un parti et la fin) : retirée.
            let prev = (0..k).rev().find(|&j| its[j].kind != Kind::Blank);
            let next = (k + 1..its.len()).find(|&j| its[j].kind != Kind::Blank);
            let gone = |j: Option<usize>| j.is_some_and(|j| moved[j]);
            if gone(prev) && (gone(next) || next.is_none()) {
                continue;
            }
        }
        out.extend(lines[it.start..it.end].iter().map(|l| l.to_string()));
    }
    let mut hyprland = out.join("\n");
    if src.ends_with('\n') {
        hyprland.push('\n');
    }
    let mut b = binds.join("\n");
    b.push('\n');
    Some(Split {
        hyprland,
        binds: b,
        moved: count,
        copied,
        taken,
    })
}

/// En-tête de binds.lua à sa création.
fn header() -> String {
    let mut h = [
        "-- Raccourcis clavier de Hyprland, sortis de hyprland.lua par hyprdesk.",
        "-- hyprland.lua les charge à l'endroit où ils étaient (ligne require binds).",
        "-- Ce fichier est à vous : hyprdesk le lit (Réglages → Raccourcis) et n'y",
        "-- fait qu'ajouter les raccourcis apparus depuis dans hyprland.lua.",
    ]
    .map(crate::i18n::tr)
    .join("\n");
    h.push('\n');
    h
}

/// Copie de sauvegarde : la première sous son nom, les suivantes datées.
fn backup(path: &Path, src: &str) -> Result<PathBuf, String> {
    let mut bak = path.with_extension("lua.avant-hyprdesk-binds.bak");
    if bak.exists() {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        bak = path.with_extension(format!("lua.avant-hyprdesk-binds.bak-{t}"));
    }
    std::fs::write(&bak, src).map_err(|e| e.to_string())?;
    Ok(bak)
}

/// Ce qu'a fait le déplacement.
#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    pub moved: usize,
    pub backup: PathBuf,
}

/// Au démarrage : déplace les raccourcis de hyprland.lua vers binds.lua,
/// recharge Hyprland, et rend tout tel quel si une erreur nouvelle apparaît.
/// `Ok(None)` : rien à déplacer.
pub fn migrate(config_path: &Path) -> Result<Option<Done>, String> {
    let hypr = crate::session_start::hyprland_path(config_path);
    let Ok(src) = std::fs::read_to_string(&hypr) else {
        return Ok(None);
    };
    let file = path(config_path);
    let old = std::fs::read_to_string(&file).ok();
    let Some(s) = split(&src, old.as_deref()) else {
        return Ok(None);
    };
    let before: BTreeSet<String> = crate::hyprcheck::config_errors().into_iter().collect();
    let bak = backup(&hypr, &src)?;
    if let Some(o) = &old {
        let _ = std::fs::write(file.with_extension("lua.avant-hyprdesk.bak"), o);
    }
    let new_binds = match &old {
        Some(o) => format!(
            "{}\n{}\n{}",
            o.trim_end(),
            crate::i18n::tr("-- Ajoutés depuis hyprland.lua par hyprdesk"),
            s.binds
        ),
        None => format!("{}\n{}", header(), s.binds),
    };
    // Fichier tenu par hyprdesk (MLG-198) : écrit sous sa forme générée.
    let new_binds = generate(&new_binds);
    crate::history::write_file(&file, new_binds, "raccourcis").map_err(|e| e.to_string())?;
    crate::history::write_file(&hypr, &s.hyprland, "raccourcis").map_err(|e| e.to_string())?;
    let _ = std::process::Command::new("hyprctl").arg("reload").output();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let fresh: Vec<String> = crate::hyprcheck::config_errors()
        .into_iter()
        .filter(|e| !before.contains(e))
        .collect();
    if !fresh.is_empty() {
        // Retour arrière : les deux fichiers comme avant.
        let _ = std::fs::write(&hypr, &src);
        match &old {
            Some(o) => {
                let _ = std::fs::write(&file, o);
            }
            None => {
                let _ = std::fs::remove_file(&file);
            }
        }
        let _ = std::process::Command::new("hyprctl").arg("reload").output();
        return Err(crate::tr!(
            "raccourcis laissés dans hyprland.lua : Hyprland signalait {e}",
            e = fresh.join(" ; ")
        ));
    }
    Ok(Some(Done {
        moved: s.moved,
        backup: bak,
    }))
}

// ── Édition d'un raccourci de binds.lua (MLG-116) ─────────────────────────
//
// Réglages → Raccourcis désactive (met en commentaire, avec une marque),
// réactive, supprime ou réécrit un `hl.bind(…)` de binds.lua, désigné par sa
// ligne et sa combinaison : si la ligne ne porte plus cette combinaison (le
// fichier a changé depuis le relevé), rien n'est touché.

/// Marque d'un raccourci désactivé dans les Réglages.
pub const OFF: &str = "-- [hyprdesk off] ";

/// Un raccourci de binds.lua désactivé dans les Réglages.
#[derive(Debug, Clone, PartialEq)]
pub struct Disabled {
    /// Ligne du `hl.bind(` (à partir de 1).
    pub line: usize,
    pub combo: (u32, String),
    /// Ce qu'il fait (commande lancée, sinon le dispatcher).
    pub what: String,
}

/// Une modification de binds.lua.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    Disable,
    Enable,
    Delete,
    /// Nouvelle combinaison (`SUPER + SHIFT + d`).
    Rebind(String),
    /// Nouvelle action, la ligne d'origine gardée en commentaire au-dessus
    /// (`-- [hyprdesk off] …`) : ex. `hl.dsp.exec_cmd("hyprdesk switch")`.
    Action(String),
}

/// binds.lua avec ses raccourcis désactivés rendus lisibles (marque ôtée),
/// et pour chaque ligne si elle portait la marque.
fn unmasked(src: &str) -> (String, Vec<bool>) {
    let mut marked = Vec::new();
    let text: Vec<&str> = src
        .lines()
        .map(|l| match l.strip_prefix(OFF) {
            Some(rest) => {
                marked.push(true);
                rest
            }
            None => {
                marked.push(false);
                l
            }
        })
        .collect();
    (text.join("\n"), marked)
}

/// Raccourcis désactivés dans les Réglages.
pub fn disabled_in(src: &str) -> Vec<Disabled> {
    let (text, marked) = unmasked(src);
    crate::keymap::parse_source(&text)
        .into_iter()
        .filter(|b| {
            marked.get(b.line - 1) == Some(&true)
                && (b.line < 2 || marked.get(b.line - 2) != Some(&true))
        })
        .map(|b| Disabled {
            line: b.line,
            combo: b.combo,
            what: b.action,
        })
        .collect()
}

// ── binds.lua généré par hyprdesk (MLG-198) ───────────────────────────────
//
// hyprdesk tient binds.lua : à chaque écriture, le fichier est régénéré
// d'après ce qu'il définit vraiment (évaluation, comme Hyprland le lit) —
// variables remplacées par leur valeur, boucles dépliées, un raccourci par
// ligne. Les commentaires (titres de section, raccourcis mis de côté par
// l'utilisateur) restent à leur place ; les raccourcis désactivés gardent leur
// marque. Ce qui ne se réécrit pas en données (action en fonction Lua,
// raccourci posé dans `hl.on`, autre code) est gardé tel quel, avec les
// locales dont il se sert.

/// Première ligne d'un en-tête de binds.lua, ancien (« à vous ») ou généré.
const HEADERS: &[&str] = &[
    "-- Généré par hyprdesk",
    "-- Generated by hyprdesk",
    "-- Raccourcis clavier de Hyprland, sortis de hyprland.lua par hyprdesk.",
    "-- Hyprland key bindings, moved out of hyprland.lua by hyprdesk.",
];

/// Nom de la locale du dossier personnel dans binds.lua généré.
const HOME_VAR: &str = "HOME";

/// En-tête de binds.lua généré.
fn generated_header() -> String {
    let mut h = [
        "-- Généré par hyprdesk : raccourcis clavier de Hyprland (Réglages → Raccourcis).",
        "-- hyprland.lua le charge par require(\"binds\"). Réécrit à chaque changement :",
        "-- variables remplacées par leur valeur, boucles dépliées, un raccourci par ligne.",
        "-- Une retouche à la main est reprise, puis remise en forme à l'écriture suivante.",
    ]
    .map(crate::i18n::tr)
    .join("\n");
    h.push('\n');
    h
}

/// Chaîne Lua ; le dossier personnel s'y écrit `HOME` (`"… -o " .. HOME .. "/Images"`).
fn lua_string(s: &str, home: &str, need_home: &mut bool) -> String {
    let q = |t: &str| format!("\"{}\"", hyprpanel_core::ipc::lua_escape(t));
    // Morceaux : texte (`Some`) ou dossier personnel (`None`).
    let mut parts: Vec<Option<String>> = Vec::new();
    let mut lit = String::new();
    let mut rest = s;
    while home.len() > 1
        && let Some(i) = rest.find(home)
    {
        let after = &rest[i + home.len()..];
        lit.push_str(&rest[..i]);
        // Le dossier entier seulement (`/home/jacques/…`, pas `/home/jacquesX`).
        if after.is_empty() || after.starts_with('/') {
            if !lit.is_empty() {
                parts.push(Some(std::mem::take(&mut lit)));
            }
            parts.push(None);
            *need_home = true;
        } else {
            lit.push_str(home);
        }
        rest = after;
    }
    lit.push_str(rest);
    if !lit.is_empty() || parts.is_empty() {
        parts.push(Some(lit));
    }
    parts
        .iter()
        .map(|p| p.as_deref().map_or_else(|| HOME_VAR.to_string(), q))
        .collect::<Vec<_>>()
        .join(" .. ")
}

/// Arguments d'un `table.pack` relu en JSON (tableau, ou clés « 1 », « 2 »…).
fn packed(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    match v {
        serde_json::Value::Array(a) => a.iter().collect(),
        serde_json::Value::Object(m) => {
            let mut l: Vec<(usize, &serde_json::Value)> = m
                .iter()
                .filter_map(|(k, v)| k.parse::<usize>().ok().map(|i| (i, v)))
                .collect();
            l.sort_by_key(|(i, _)| *i);
            l.into_iter().map(|(_, v)| v).collect()
        }
        _ => Vec::new(),
    }
}

/// Valeur relue → texte Lua ; `None` si elle ne se réécrit pas (fonction).
fn lua_value(v: &serde_json::Value, home: &str, need_home: &mut bool) -> Option<String> {
    use serde_json::Value as J;
    Some(match v {
        J::Null => "nil".into(),
        J::Bool(b) => b.to_string(),
        J::Number(n) => n.to_string(),
        J::String(s) if s == "<fonction>" => return None,
        J::String(s) => lua_string(s, home, need_home),
        J::Array(a) if a.is_empty() => "{}".into(),
        J::Array(a) => {
            let items: Option<Vec<String>> =
                a.iter().map(|x| lua_value(x, home, need_home)).collect();
            format!("{{ {} }}", items?.join(", "))
        }
        J::Object(m) => {
            if let Some(J::String(name)) = m.get("__dsp") {
                let args: Option<Vec<String>> = m
                    .get("args")
                    .map(packed)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|x| lua_value(x, home, need_home))
                    .collect();
                return Some(format!("hl.dsp.{name}({})", args?.join(", ")));
            }
            if m.is_empty() {
                return Some("{}".into());
            }
            let mut parts = Vec::new();
            for (k, x) in m {
                let val = lua_value(x, home, need_home)?;
                let ident = k
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
                    && k.chars().all(|c| c.is_alphanumeric() || c == '_');
                parts.push(if ident {
                    format!("{k} = {val}")
                } else if let Ok(i) = k.parse::<i64>() {
                    format!("[{i}] = {val}")
                } else {
                    format!("[\"{}\"] = {val}", hyprpanel_core::ipc::lua_escape(k))
                });
            }
            format!("{{ {} }}", parts.join(", "))
        }
    })
}

/// Un `hl.bind(…)` relu → sa ligne, ou `None` s'il ne se réécrit pas.
fn bind_line(args: &[serde_json::Value], home: &str, need_home: &mut bool) -> Option<String> {
    let combo = match args.first()? {
        serde_json::Value::String(c) => format!("\"{}\"", hyprpanel_core::ipc::lua_escape(c)),
        _ => return None,
    };
    let action = lua_value(args.get(1)?, home, need_home)?;
    if !action.starts_with("hl.dsp.") {
        return None;
    }
    let opts = match args.get(2) {
        None | Some(serde_json::Value::Null) => String::new(),
        Some(o) => format!(", {}", lua_value(o, home, need_home)?),
    };
    Some(format!("hl.bind({combo}, {action}{opts})"))
}

/// binds.lua tel que hyprdesk l'écrit, d'après son contenu `src` (ancien
/// fichier à la main ou déjà généré). Idempotent : `generate(generate(s))`
/// rend `generate(s)`.
pub fn generate(src: &str) -> String {
    let (text, marked) = unmasked(src);
    let orig: Vec<&str> = src.lines().collect();
    let list = items(&text);
    let evaluated = crate::hyprsrc::evaluate_text(&text, Path::new("binds.lua"));
    let home = std::env::var("HOME").unwrap_or_default();
    let mut need_home = false;
    // Chaque instruction : ses lignes générées, ou `None` (gardée telle quelle).
    let mut rendered: Vec<Option<Vec<String>>> = Vec::new();
    for it in &list {
        if it.kind != Kind::Bind {
            rendered.push(None);
            continue;
        }
        let calls: Vec<&crate::hyprsrc::Call> = evaluated
            .calls
            .iter()
            .filter(|c| c.func == "bind" && it.start < c.line && c.line <= it.end)
            .collect();
        let off = marked.get(it.start) == Some(&true);
        let lines: Option<Vec<String>> = (!calls.is_empty()
            && calls.iter().all(|c| c.context.is_none()))
        .then(|| {
            calls
                .iter()
                .map(|c| {
                    bind_line(&c.args, &home, &mut need_home)
                        .map(|l| if off { format!("{OFF}{l}") } else { l })
                })
                .collect()
        })
        .flatten();
        rendered.push(lines);
    }
    // Locales encore nécessaires : celles qu'emploie le code gardé tel quel
    // (et, de proche en proche, celles dont elles dépendent).
    let mut needed: BTreeSet<String> = list
        .iter()
        .zip(&rendered)
        .filter(|(it, r)| r.is_none() && matches!(it.kind, Kind::Bind | Kind::Other))
        .flat_map(|(it, _)| it.uses.iter().cloned())
        .collect();
    loop {
        let more: BTreeSet<String> = list
            .iter()
            .filter(|it| it.defines.as_ref().is_some_and(|d| needed.contains(d)))
            .flat_map(|it| it.uses.iter().cloned())
            .filter(|u| !needed.contains(u))
            .collect();
        if more.is_empty() {
            break;
        }
        needed.extend(more);
    }
    let mut body: Vec<String> = Vec::new();
    for (i, (it, r)) in list.iter().zip(&rendered).enumerate() {
        let verbatim = || orig[it.start..it.end].iter().map(|l| l.to_string());
        match (it.kind, r) {
            (Kind::Blank, _) => {
                if body.last().is_some_and(|l| !l.is_empty()) {
                    body.push(String::new());
                }
            }
            // Ancien en-tête (ou en-tête généré) : remplacé par le nouveau.
            (Kind::Comment, _)
                if i == list.iter().position(|x| x.kind != Kind::Blank).unwrap_or(0)
                    && HEADERS
                        .iter()
                        .any(|h| orig[it.start].trim_start().starts_with(h)) => {}
            (Kind::Local, _) => {
                if it.defines.as_ref().is_some_and(|d| needed.contains(d)) {
                    body.extend(verbatim());
                }
            }
            (_, Some(lines)) => body.extend(lines.iter().cloned()),
            _ => body.extend(verbatim()),
        }
    }
    while body.first().is_some_and(String::is_empty) {
        body.remove(0);
    }
    while body.last().is_some_and(String::is_empty) {
        body.pop();
    }
    let mut out = generated_header();
    if need_home && !needed.contains(HOME_VAR) {
        out += &format!("\nlocal {HOME_VAR} = os.getenv(\"HOME\")\n");
    }
    if !body.is_empty() {
        out.push('\n');
        out += &body.join("\n");
        out.push('\n');
    }
    out
}

/// Au démarrage : binds.lua pris en charge par hyprdesk (MLG-198). Un fichier
/// écrit à la main est copié puis régénéré ; un fichier généré retouché à la
/// main est remis en forme. Hyprland vérifie : tout est rendu tel quel s'il
/// signale une erreur nouvelle. `Ok(Some(copie))` : fichier repris.
pub fn adopt(config_path: &Path) -> Result<Option<PathBuf>, String> {
    let file = path(config_path);
    let Ok(src) = std::fs::read_to_string(&file) else {
        return Ok(None);
    };
    let new = generate(&src);
    if new == src {
        return Ok(None);
    }
    let bak = if crate::hyprsrc::generated(&src) {
        None
    } else {
        Some(backup(&file, &src)?)
    };
    write_checked(&file, &src, &new)?;
    Ok(bak)
}

/// Raccourcis désactivés de binds.lua (fichier absent : aucun).
pub fn disabled(config_path: &Path) -> Vec<Disabled> {
    std::fs::read_to_string(path(config_path))
        .map(|s| disabled_in(&s))
        .unwrap_or_default()
}

/// Même combinaison (même touche physique : `1`, `ampersand`, `code:10`).
pub fn same_combo(a: &(u32, String), b: &(u32, String)) -> bool {
    let key = |k: &str| crate::hypr::physical_key(k);
    a.0 == b.0
        && (a.1.eq_ignore_ascii_case(&b.1)
            || matches!((key(&a.1), key(&b.1)), (Some(x), Some(y)) if x == y))
}

/// Instruction `hl.bind(…)` qui commence à la ligne `line` (à partir de 1),
/// en lignes [début, fin), si elle porte `combo`. Une boucle qui pose des
/// raccourcis n'est pas une instruction seule : refusée.
fn statement(src: &str, line: usize, combo: &(u32, String)) -> Result<(usize, usize), String> {
    let changed = || {
        crate::tr!(
            "binds.lua a changé depuis le relevé (ligne {n}) : rien n'est modifié.",
            n = line
        )
    };
    let lines: Vec<&str> = src.lines().collect();
    let first = lines.get(line.wrapping_sub(1)).ok_or_else(changed)?;
    if !first.trim_start().starts_with("hl.bind(") {
        return Err(crate::tr!(
            "Ligne {n} de binds.lua : raccourci posé par un bloc (boucle, fonction), à modifier dans le fichier.",
            n = line
        ));
    }
    let found = crate::keymap::parse_source(src)
        .into_iter()
        .any(|b| b.line == line && same_combo(&b.combo, combo));
    if !found {
        return Err(changed());
    }
    items(src)
        .into_iter()
        .find(|it| it.kind == Kind::Bind && it.start + 1 == line)
        .map(|it| (it.start, it.end))
        .ok_or_else(changed)
}

/// Bloc désactivé (lignes marquées) qui commence à `line`, s'il porte `combo`.
fn masked_statement(
    src: &str,
    line: usize,
    combo: &(u32, String),
) -> Result<(usize, usize), String> {
    let (text, marked) = unmasked(src);
    if marked.get(line.wrapping_sub(1)) != Some(&true) {
        return Err(crate::tr!(
            "binds.lua a changé depuis le relevé (ligne {n}) : rien n'est modifié.",
            n = line
        ));
    }
    let (start, end) = statement(&text, line, combo)?;
    // Le bloc entier doit être marqué : sinon ce n'est pas notre commentaire.
    if marked[start..end].iter().all(|m| *m) {
        Ok((start, end))
    } else {
        Err(crate::tr!(
            "binds.lua a changé depuis le relevé (ligne {n}) : rien n'est modifié.",
            n = line
        ))
    }
}

/// Première combinaison d'un `hl.bind(` : position (octets) de l'argument
/// dans la ligne.
fn first_arg(line: &str) -> Option<(usize, usize)> {
    let open = line.find("hl.bind(")? + "hl.bind(".len();
    let (mut quote, mut depth) = (None::<char>, 0i32);
    for (i, c) in line[open..].char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(' | '{' | '[') => depth += 1,
            (None, ')' | '}' | ']') => depth -= 1,
            (None, ',') if depth == 0 => return Some((open, open + i)),
            _ => {}
        }
    }
    None
}

/// binds.lua après la modification `edit` du raccourci de la ligne `line`
/// (combinaison `combo`).
pub fn edit_src(
    src: &str,
    line: usize,
    combo: &(u32, String),
    edit: &Edit,
) -> Result<String, String> {
    let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
    match edit {
        Edit::Disable => {
            let (s, e) = statement(src, line, combo)?;
            for l in &mut lines[s..e] {
                *l = format!("{OFF}{l}");
            }
        }
        Edit::Enable => {
            let (s, e) = masked_statement(src, line, combo)?;
            for l in &mut lines[s..e] {
                *l = l.strip_prefix(OFF).unwrap_or(l).to_string();
            }
        }
        Edit::Delete => {
            let (s, e) =
                statement(src, line, combo).or_else(|_| masked_statement(src, line, combo))?;
            lines.drain(s..e);
        }
        Edit::Action(call) => {
            let (s, e) = statement(src, line, combo)?;
            if e - s != 1 {
                return Err(crate::tr!(
                    "Ligne {n} de binds.lua : raccourci sur plusieurs lignes, laissé tel quel.",
                    n = line
                ));
            }
            let (_, b) = first_arg(&lines[s]).ok_or_else(|| {
                crate::tr!(
                    "Ligne {n} de binds.lua : combinaison introuvable.",
                    n = line
                )
            })?;
            // Deuxième argument : de la virgule qui suit la combinaison à la
            // virgule (ou parenthèse) de même niveau suivante.
            let old = lines[s].clone();
            let rest = &old[b..];
            let start =
                b + rest.find(',').ok_or_else(|| {
                    crate::tr!("Ligne {n} de binds.lua : action introuvable.", n = line)
                })? + 1;
            let (mut depth, mut quote, mut end) = (0i32, None::<char>, old.len());
            for (i, c) in old[start..].char_indices() {
                match (quote, c) {
                    (Some(q), c) if c == q => quote = None,
                    (Some(_), _) => {}
                    (None, '"' | '\'') => quote = Some(c),
                    (None, '(' | '{') => depth += 1,
                    (None, ')' | '}') if depth > 0 => depth -= 1,
                    (None, ',' | ')') if depth == 0 => {
                        end = start + i;
                        break;
                    }
                    _ => {}
                }
            }
            let mut new_line = old.clone();
            new_line.replace_range(start..end, &format!(" {call}"));
            lines[s] = new_line;
            lines.insert(s, format!("{OFF}{old}"));
        }
        Edit::Rebind(new) => {
            if crate::hypr::parse_combo(new).is_none() {
                return Err(crate::tr!("« {v} » : raccourci illisible", v = new));
            }
            let (s, _) = statement(src, line, combo)?;
            let (a, b) = first_arg(&lines[s]).ok_or_else(|| {
                crate::tr!(
                    "Ligne {n} de binds.lua : combinaison introuvable.",
                    n = line
                )
            })?;
            lines[s].replace_range(a..b, &format!("\"{new}\""));
        }
    }
    let mut out = lines.join("\n");
    if src.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// Ce qu'a fait une modification : de quoi l'annuler.
#[derive(Debug, Clone, PartialEq)]
pub struct Edited {
    pub before: String,
    pub after: String,
}

/// Écrit `new` dans binds.lua, recharge Hyprland, et rend l'ancien contenu
/// si Hyprland signale une erreur nouvelle.
fn write_checked(file: &Path, old: &str, new: &str) -> Result<(), String> {
    let before: BTreeSet<String> = crate::hyprcheck::config_errors().into_iter().collect();
    crate::history::write_file(file, new, "raccourcis").map_err(|e| e.to_string())?;
    let _ = std::process::Command::new("hyprctl").arg("reload").output();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let fresh: Vec<String> = crate::hyprcheck::config_errors()
        .into_iter()
        .filter(|e| !before.contains(e))
        .collect();
    if !fresh.is_empty() {
        let _ = std::fs::write(file, old);
        let _ = std::process::Command::new("hyprctl").arg("reload").output();
        return Err(crate::tr!(
            "binds.lua laissé tel quel : Hyprland signalait {e}",
            e = fresh.join(" ; ")
        ));
    }
    Ok(())
}

/// Configuration par défaut des raccourcis (MLG-202) : plus aucun raccourci
/// coupé depuis les Réglages. Rend le nombre de raccourcis réactivés.
/// `live` : Hyprland recharge et vérifie (retour arrière sur erreur).
pub fn enable_all(config_path: &Path, live: bool) -> Result<usize, String> {
    let file = path(config_path);
    let Ok(src) = std::fs::read_to_string(&file) else {
        return Ok(0);
    };
    let (text, marked) = unmasked(&src);
    let n = marked.iter().filter(|m| **m).count();
    if n == 0 {
        return Ok(0);
    }
    let new = generate(&(text + "\n"));
    if live {
        write_checked(&file, &src, &new)?;
    } else {
        std::fs::write(&file, new).map_err(|e| e.to_string())?;
    }
    Ok(n)
}

/// Modifie un raccourci de binds.lua, puis recharge Hyprland.
pub fn edit(
    config_path: &Path,
    line: usize,
    combo: &(u32, String),
    e: &Edit,
) -> Result<Edited, String> {
    let file = path(config_path);
    let src = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let new = generate(&edit_src(&src, line, combo, e)?);
    write_checked(&file, &src, &new)?;
    Ok(Edited {
        before: src,
        after: new,
    })
}

/// Annule une modification, si binds.lua n'a pas changé depuis.
pub fn undo(config_path: &Path, done: &Edited) -> Result<(), String> {
    let file = path(config_path);
    let now = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    if now != done.after {
        return Err(
            crate::i18n::tr("binds.lua a changé depuis : annulation impossible.").to_string(),
        );
    }
    write_checked(&file, &now, &done.before)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"-- Hyprland
require("monitors")

hl.config({ general = { gaps_in = 4 } })

---------------------
---- KEYBINDINGS ----
---------------------

local mainMod    = "SUPER"
local scriptsDir = os.getenv("HOME") .. "/.config/hypr/scripts"
local term       = "kitty"
local colorpicker = scriptsDir .. "/colorpicker"

-- Screenshots
hl.bind("Print", hl.dsp.exec_cmd("hyprshot -m region"), { submap_universal = true })

-- Apps
hl.bind(mainMod .. " + V", hl.dsp.exec_cmd(scriptsDir .. "/clipboard"))
-- Secours du verrouillage
hl.config({ misc = { allow_session_lock_restore = true } })
hl.bind(mainMod .. " + Return", hl.dsp.exec_cmd(term))
hl.bind(mainMod .. " + SHIFT + X", hl.dsp.exec_cmd(colorpicker))

-- -- Tab cycling
-- hl.bind(mainMod .. " + Tab", function()
--     hl.dispatch(hl.dsp.window.cycle_next())
-- end)

-- Send the window and stay
for ws, code in ipairs({ 10, 11 }) do
    hl.bind(mainMod .. " + ALT + SHIFT + code:" .. code, hl.dsp.window.move({ workspace = ws, follow = false }))
end
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(),
    { mouse = true })


-------------
---- RULES ----
-------------
hl.on("hyprland.start", function()
    hl.exec_cmd(scriptsDir .. "/start")
end)
--[[
hl.bind("SUPER + X", hl.dsp.exec_cmd("vieux"))
--]]
"#;

    #[test]
    fn deplace_les_raccourcis_et_leurs_locales() {
        let s = split(SRC, None).expect("des raccourcis à déplacer");
        assert_eq!(s.moved, 6, "Print, V, Return, X, la boucle, la souris");
        // scriptsDir sert encore (démarrage) : copiée ; les autres partent.
        assert_eq!(s.copied, ["scriptsDir"]);
        assert_eq!(s.taken, ["mainMod", "term", "colorpicker"]);
        let h = &s.hyprland;
        assert!(
            h.lines()
                .filter(|l| l.trim_start().starts_with("hl.bind("))
                .all(|l| l.contains("vieux")),
            "hyprland.lua sans raccourci :\n{h}"
        );
        assert!(h.contains("vieux"), "le long commentaire reste");
        assert!(h.contains("local scriptsDir"));
        assert!(!h.contains("local mainMod"));
        assert!(h.contains("hl.config({ misc = { allow_session_lock_restore = true } })"));
        assert!(
            h.contains("-- Secours du verrouillage"),
            "son commentaire reste"
        );
        assert!(h.contains("---- KEYBINDINGS ----"));
        assert!(h.contains("hl.exec_cmd(scriptsDir"));
        assert_eq!(h.matches("require(\"binds\")").count(), 1);
        // require à la place du premier raccourci (après les locales gardées).
        let req = h.find("require(\"binds\")").unwrap();
        assert!(req > h.find("local scriptsDir").unwrap());
        assert!(req < h.find("allow_session_lock_restore").unwrap());
        let b = &s.binds;
        // Locales d'abord, dans l'ordre ; colorpicker après scriptsDir.
        assert!(b.starts_with("local mainMod"));
        assert!(b.find("local scriptsDir").unwrap() < b.find("local colorpicker").unwrap());
        assert!(b.contains("-- Screenshots\nhl.bind(\"Print\""));
        assert!(b.contains("-- -- Tab cycling\n-- hl.bind(mainMod .. \" + Tab\""));
        assert!(b.contains("for ws, code in ipairs({ 10, 11 }) do"));
        assert!(b.contains("    { mouse = true })"));
        assert!(!b.contains("allow_session_lock_restore"));
        assert!(!b.contains("vieux"), "un long commentaire ne bouge pas");
        // Rien à refaire ensuite.
        assert_eq!(split(&s.hyprland, Some(&s.binds)), None);
    }

    #[test]
    fn ajoute_sans_redefinir() {
        let first = split(SRC, None).unwrap();
        let later = format!(
            "{}\nhl.bind(mainMod .. \" + B\", hl.dsp.exec_cmd(term))\n",
            first.hyprland.trim_end()
        );
        // mainMod et term ne sont plus dans hyprland.lua : la ligne ajoutée
        // les emploie sans les définir ; binds.lua les a déjà.
        let s = split(&later, Some(&first.binds)).unwrap();
        assert_eq!(s.moved, 1);
        assert!(!s.binds.contains("local mainMod"));
        assert_eq!(s.hyprland.matches("require(\"binds\")").count(), 1);
    }

    #[test]
    fn rien_a_faire() {
        assert_eq!(split("hl.config({})\n", None), None);
        // Une locale posée par un raccourci et reprise ailleurs : on ne touche à rien.
        let src = "local b = hl.bind(\"SUPER + C\", hl.dsp.window.close())\nb:set_enabled(false)\n";
        assert_eq!(split(src, None), None);
    }

    const BINDS: &str = r#"local mainMod = "SUPER"
-- Apps
hl.bind(mainMod .. " + Return", hl.dsp.exec_cmd("kitty"))
hl.bind(mainMod .. " + Q", hl.dsp.window.close())
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(),
    { mouse = true })
for ws, code in ipairs({ 10, 11 }) do
    hl.bind(mainMod .. " + code:" .. code, hl.dsp.focus({ workspace = ws }))
end
"#;

    fn c(s: &str) -> (u32, String) {
        crate::hypr::parse_combo(s).unwrap()
    }

    #[test]
    fn desactive_puis_reactive() {
        let off = edit_src(BINDS, 5, &c("SUPER + mouse:272"), &Edit::Disable).unwrap();
        assert!(off.contains(&format!("{OFF}hl.bind(mainMod .. \" + mouse:272\"")));
        assert!(off.contains(&format!("{OFF}    {{ mouse = true }})")));
        // Plus actif, mais listé comme désactivé, avec sa ligne.
        assert!(
            !crate::keymap::parse_source(&off)
                .iter()
                .any(|b| b.combo == c("SUPER + mouse:272"))
        );
        let d = disabled_in(&off);
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].line, d[0].combo.clone()), (5, c("SUPER + mouse:272")));
        assert_eq!(d[0].what, "window.drag");
        let on = edit_src(&off, 5, &c("SUPER + mouse:272"), &Edit::Enable).unwrap();
        assert_eq!(on, BINDS);
    }

    #[test]
    fn supprime_et_reecrit() {
        let del = edit_src(BINDS, 4, &c("SUPER + Q"), &Edit::Delete).unwrap();
        assert!(!del.contains("window.close"));
        assert_eq!(del.lines().count(), BINDS.lines().count() - 1);
        // Un raccourci désactivé se supprime aussi.
        let off = edit_src(BINDS, 5, &c("SUPER + mouse:272"), &Edit::Disable).unwrap();
        let gone = edit_src(&off, 5, &c("SUPER + mouse:272"), &Edit::Delete).unwrap();
        assert!(!gone.contains("mouse:272") && gone.contains("window.close"));
        let re = edit_src(
            BINDS,
            3,
            &c("SUPER + Return"),
            &Edit::Rebind("SUPER + ALT + t".into()),
        )
        .unwrap();
        assert!(re.contains("hl.bind(\"SUPER + ALT + t\", hl.dsp.exec_cmd(\"kitty\"))"));
        assert!(
            crate::keymap::parse_source(&re)
                .iter()
                .any(|b| b.line == 3 && b.combo == c("SUPER + ALT + t"))
        );
    }

    #[test]
    fn refuse_ce_qui_a_change() {
        // Autre combinaison à cette ligne (fichier modifié depuis le relevé).
        assert!(edit_src(BINDS, 4, &c("SUPER + E"), &Edit::Delete).is_err());
        // Raccourci posé par une boucle.
        assert!(edit_src(BINDS, 8, &c("SUPER + code:10"), &Edit::Disable).is_err());
        // Réactiver une ligne qui n'est pas désactivée.
        assert!(edit_src(BINDS, 4, &c("SUPER + Q"), &Edit::Enable).is_err());
        // Combinaison illisible.
        assert!(edit_src(BINDS, 4, &c("SUPER + Q"), &Edit::Rebind("SUPER +".into())).is_err());
    }

    #[test]
    fn lexeur() {
        let l = lex("hl.bind(\"a -- b\", x) -- note\n--[[ long\nhl.bind( ]] y = 1\n");
        assert_eq!(l[0].code.trim(), "hl.bind(\"\", x)");
        assert!(!l[0].comment);
        assert!(l[1].comment);
        assert_eq!(l[2].code.trim(), "y = 1");
        assert!(!l[2].comment);
    }
}

/// Équivalents hyprdesk d'outils courants lancés par un raccourci :
/// (morceau de la commande, sous-commande de hyprdesk).
const EQUIVALENTS: &[(&str, &str)] = &[
    ("cliphist", "clipboard"),
    ("/clipboard", "clipboard"),
    ("windowswitch", "switch"),
    ("rofi -show window", "switch"),
    ("powermenu", "menu session"),
    ("wlogout", "menu session"),
    ("hyprshot -m output", "capture ecran"),
    ("hyprshot -m window", "capture fenetre"),
    ("hyprshot -m region", "capture zone"),
];

/// Accord de binds.lua avec hyprdesk (MLG-140) :
/// - les combinaisons que hyprdesk tient (lanceur unifié, `keys.take`,
///   touches multimédia et captures si `keys.media`) sont désactivées
///   (`-- [hyprdesk off]`) : binds.lua décrit ce qui tourne ;
/// - un raccourci qui lance un outil dont hyprdesk a l'équivalent est
///   rebranché sur hyprdesk, la ligne d'origine gardée en commentaire.
///
/// Renvoie ce qui a été fait (lignes lisibles) ; rien si tout est en accord.
pub fn sync_hyprdesk(
    config_path: &Path,
    cfg: &crate::config::Config,
    dry_run: bool,
) -> Result<Vec<String>, String> {
    let file = path(config_path);
    let Ok(src) = std::fs::read_to_string(&file) else {
        return Ok(Vec::new());
    };
    let keys = &cfg.general.keys;
    let mut taken: Vec<(u32, String)> = keys
        .take
        .iter()
        .filter_map(|k| crate::hypr::parse_combo(k))
        .collect();
    if cfg.general.apps_launcher.unified
        && let Some(c) = crate::hypr::parse_combo(&keys.search)
    {
        taken.push(c);
    }
    if keys.media {
        taken.extend(
            crate::hypr::MEDIA_KEYS
                .iter()
                .filter_map(|(k, _, _)| crate::hypr::parse_combo(k)),
        );
    }
    let binds = crate::keymap::parse_source(&src);
    let mut out = src.clone();
    let mut done = Vec::new();
    // Du bas vers le haut : une ligne insérée ne décale pas les suivantes.
    let mut list: Vec<&crate::keymap::SourceBind> = binds.iter().collect();
    list.sort_by_key(|b| std::cmp::Reverse(b.line));
    let exe = "hyprdesk";
    for b in list {
        let same = |c: &(u32, String)| c.0 == b.combo.0 && c.1.eq_ignore_ascii_case(&b.combo.1);
        if taken.iter().any(same) {
            if let Ok(n) = edit_src(&out, b.line, &b.combo, &Edit::Disable) {
                out = n;
                done.push(crate::tr!(
                    "binds.lua:{n} : désactivé, tenu par hyprdesk ({a})",
                    n = b.line,
                    a = b.action
                ));
            }
            continue;
        }
        let twin = |o: &&crate::keymap::SourceBind| {
            o.line != b.line && o.combo.0 == b.combo.0 && o.combo.1.eq_ignore_ascii_case(&b.combo.1)
        };
        if b.action.contains("hyprdesk") {
            // Le même raccourci écrit deux fois (MLG-206) : la commande
            // partirait deux fois, un menu s'ouvrirait puis se refermerait.
            if binds
                .iter()
                .filter(twin)
                .any(|o| o.line < b.line && o.call == b.call)
                && let Ok(n) = edit_src(&out, b.line, &b.combo, &Edit::Delete)
            {
                out = n;
                done.push(crate::tr!(
                    "binds.lua:{n} : doublon retiré ({a})",
                    n = b.line,
                    a = b.action
                ));
            }
            continue;
        }
        if let Some((_, sub)) = EQUIVALENTS.iter().find(|(m, _)| b.action.contains(m)) {
            // L'équivalent hyprdesk est déjà là (ligne d'origine réactivée
            // par une remise à zéro) : l'outil d'origine est seulement coupé.
            let wanted = format!("{exe} {sub}");
            if binds
                .iter()
                .filter(twin)
                .any(|o| o.action.contains(&wanted))
            {
                if let Ok(n) = edit_src(&out, b.line, &b.combo, &Edit::Disable) {
                    out = n;
                    done.push(crate::tr!(
                        "binds.lua:{n} : désactivé, tenu par hyprdesk ({a})",
                        n = b.line,
                        a = b.action
                    ));
                }
                continue;
            }
            let call = format!("hl.dsp.exec_cmd(\"{exe} {sub}\")");
            if let Ok(n) = edit_src(&out, b.line, &b.combo, &Edit::Action(call)) {
                out = n;
                done.push(crate::tr!(
                    "binds.lua:{n} : « {a} » remplacé par « hyprdesk {s} »",
                    n = b.line,
                    a = b.action,
                    s = sub
                ));
            }
        }
    }
    if out == src || dry_run {
        return Ok(done);
    }
    let out = generate(&out);
    let _reason = crate::history::reason("raccourcis");
    crate::hyprwrite::write_checked(&[crate::hyprwrite::Change {
        path: file,
        content: Some(out),
    }])?;
    Ok(done)
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    const HAND: &str = r#"-- Raccourcis clavier de Hyprland, sortis de hyprland.lua par hyprdesk.
-- Ce fichier est à vous.

local mainMod = "SUPER"
local term    = "kitty"
local scripts = os.getenv("HOME") .. "/.config/hypr/scripts"

-- Apps
hl.bind(mainMod .. " + Return", hl.dsp.exec_cmd(term))
-- [hyprdesk off] hl.bind(mainMod .. " + V", hl.dsp.exec_cmd(scripts .. "/clip"))
hl.bind(mainMod .. " + CTRL + left", hl.dsp.window.resize({ x = -20, y = 0, relative = true }), { repeating = true })

-- -- mis de côté par l'utilisateur
-- hl.bind(mainMod .. " + Tab", function() end)

for ws, code in ipairs({ 10, 11 }) do
    hl.bind(mainMod .. " + code:" .. code, hl.dsp.focus({ workspace = ws }))
end
hl.bind(mainMod .. " + T", function() hl.dispatch(hl.dsp.window.close()) end)
"#;

    #[test]
    fn remise_a_zero_sans_doublon() {
        // Après « tous les raccourcis réactivés » : la ligne d'origine et son
        // équivalent hyprdesk sont actifs tous les deux ; un doublon traîne.
        let dir = std::env::temp_dir().join(format!("hyprdesk-sync-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = dir.join("hyprdesk.lua");
        let src = "hl.bind(\"SUPER + V\", hl.dsp.exec_cmd(\"/x/scripts/clipboard\"))\n\
                   hl.bind(\"SUPER + V\", hl.dsp.exec_cmd(\"hyprdesk clipboard\"))\n\
                   hl.bind(\"SUPER + W\", hl.dsp.exec_cmd(\"hyprdesk switch\"))\n\
                   hl.bind(\"SUPER + W\", hl.dsp.exec_cmd(\"hyprdesk switch\"))\n";
        std::fs::write(path(&cfg_path), src).unwrap();
        let mut cfg = crate::config::Config::default();
        cfg.general.keys.media = false;
        cfg.general.keys.take.clear();
        cfg.general.apps_launcher.unified = false;
        let done = sync_hyprdesk(&cfg_path, &cfg, true).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(done.len(), 2, "{done:?}");
        assert!(done.iter().any(|d| d.contains("binds.lua:1 ")), "{done:?}");
        assert!(done.iter().any(|d| d.contains("binds.lua:4 ")), "{done:?}");
    }

    #[test]
    fn binds_genere_deplie_et_garde() {
        let home = std::env::var("HOME").unwrap();
        let g = generate(HAND);
        assert!(crate::hyprsrc::generated(&g), "en-tête généré");
        assert!(
            !g.contains("Ce fichier est à vous"),
            "ancien en-tête retiré"
        );
        assert!(g.contains("hl.bind(\"SUPER + Return\", hl.dsp.exec_cmd(\"kitty\"))"));
        assert!(g.contains(&format!(
            "{OFF}hl.bind(\"SUPER + V\", hl.dsp.exec_cmd(HOME .. \"/.config/hypr/scripts/clip\"))"
        )));
        assert!(g.contains("{ repeating = true }"));
        // Boucle dépliée.
        assert!(g.contains("hl.bind(\"SUPER + code:10\", hl.dsp.focus({ workspace = 1 }))"));
        assert!(g.contains("hl.bind(\"SUPER + code:11\", hl.dsp.focus({ workspace = 2 }))"));
        assert!(!g.contains("ipairs"));
        // Commentaires gardés ; action en fonction gardée telle quelle, avec sa locale.
        assert!(g.contains("-- Apps") && g.contains("-- -- mis de côté par l'utilisateur"));
        assert!(g.contains("hl.bind(mainMod .. \" + T\", function()"));
        assert!(g.contains("local mainMod = \"SUPER\""));
        assert!(!g.contains("local term"), "locale inutile retirée");
        assert!(g.contains("local HOME = os.getenv(\"HOME\")"));
        assert!(!g.contains(&home), "dossier personnel écrit HOME");
        // Idempotent, et lu par le relevé des Réglages comme l'original.
        assert_eq!(generate(&g), g);
        let combos = |src: &str| -> Vec<String> {
            let mut v: Vec<String> = crate::keymap::parse_source(src)
                .into_iter()
                .map(|b| format!("{:?}", b.combo))
                .collect();
            v.sort();
            v
        };
        let before = combos(HAND);
        let after = combos(&g);
        assert!(
            before.iter().all(|c| after.contains(c)),
            "{before:?} ⊄ {after:?}"
        );
        assert!(after.len() > before.len(), "la boucle dépliée se voit");
        // Désactivé : toujours relevé comme tel.
        assert_eq!(disabled_in(&g).len(), 1);
    }

    #[test]
    fn action_remplacee_ligne_gardee() {
        let src = "hl.bind(\"SUPER + W\", hl.dsp.exec_cmd(scriptsDir .. \"/windowswitch\"), { repeating = false })\n";
        let combo = crate::hypr::parse_combo("SUPER + W").unwrap();
        let out = edit_src(
            src,
            1,
            &combo,
            &Edit::Action("hl.dsp.exec_cmd(\"hyprdesk switch\")".into()),
        )
        .unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].starts_with(OFF));
        assert_eq!(
            lines[1],
            "hl.bind(\"SUPER + W\", hl.dsp.exec_cmd(\"hyprdesk switch\"), { repeating = false })"
        );
    }
}
