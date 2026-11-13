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
struct Lexed {
    code: String,
    /// Ligne de commentaire seul (ou dans un long commentaire).
    comment: bool,
}

/// Code de chaque ligne, sans chaînes ni commentaires (longs compris).
fn lex(src: &str) -> Vec<Lexed> {
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
enum Kind {
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
struct Item {
    start: usize,
    end: usize,
    kind: Kind,
    /// Variable locale définie.
    defines: Option<String>,
    /// Identifiants employés.
    uses: BTreeSet<String>,
}

fn items(src: &str) -> Vec<Item> {
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
    std::fs::write(&file, new_binds).map_err(|e| e.to_string())?;
    std::fs::write(&hypr, &s.hyprland).map_err(|e| e.to_string())?;
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
