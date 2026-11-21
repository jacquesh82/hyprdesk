//! Adoption des lignes de hyprland.lua par hyprdesk (MLG-139) : ce que
//! hyprdesk tient (options, courbes, animations, variables
//! d'environnement, règles de fenêtres et de couches) est repris dans ses
//! réglages, écrit dans les fichiers générés, et les lignes de hyprland.lua
//! qui le définissaient sont mises en commentaire (`-- [hyprdesk] …`) : le
//! fichier ne ment plus sur ce qui tourne.
//!
//! Qui gagne quand les deux disent quelque chose :
//! - **première adoption** (le bloc de `require` de hyprdesk n'est pas
//!   encore dans hyprland.lua) : hyprdesk, dont les valeurs sont déjà celles
//!   en vigueur (il les posait à chaud par-dessus le fichier) — l'état ne
//!   change pas ;
//! - **ensuite**, une ligne de hyprland.lua non commentée sur un domaine
//!   tenu par hyprdesk ne peut venir que d'une modification à la main : elle
//!   gagne, puis est adoptée à son tour.
//!
//! Restent dans hyprland.lua, signalés : les valeurs calculées (`os.getenv`…),
//! les options qu'aucun registre ne connaît, les lignes qui portent plusieurs
//! clés (on ne coupe pas une ligne en deux), les règles de workspace et les
//! écrans (tenus par d'autres paquets).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use crate::config::Config;
use crate::hyprsrc::{Call, Source};
use crate::settings::Settings;
use crate::tiling::OptValue;

/// Préfixe des lignes adoptées.
pub const MARK: &str = "-- [hyprdesk] ";

/// Champs de `inputs.lua` (tenus par la page Clavier et souris).
const INPUT_FIELDS: &[&str] = &[
    "input.kb_layout",
    "input.kb_variant",
    "input.kb_options",
    "input.repeat_rate",
    "input.repeat_delay",
    "input.numlock_by_default",
    "input.sensitivity",
    "input.accel_profile",
    "input.natural_scroll",
    "input.left_handed",
    "input.follow_mouse",
    "input.touchpad.natural_scroll",
    "input.touchpad.tap_to_click",
    "input.touchpad.disable_while_typing",
    "input.touchpad.clickfinger_behavior",
    "input.touchpad.scroll_factor",
];

/// Ce que l'adoption ferait.
#[derive(Debug, Default)]
pub struct Plan {
    pub settings: Settings,
    /// hyprland.lua après commentaires.
    pub hyprland: String,
    /// Lignes (1-based) commentées.
    pub commented: BTreeSet<usize>,
    /// Repris dans hyprdesk (descriptions lisibles).
    pub adopted: Vec<String>,
    /// Laissés dans hyprland.lua, avec la raison.
    pub left: Vec<String>,
    /// `inputs.lua` à régénérer depuis les valeurs en vigueur.
    pub inputs: bool,
}

/// Une ligne de hyprland.lua (1-based) peut être commentée seule : elle
/// commence par la clé et ses accolades s'équilibrent sur `[a, b]`.
fn exclusive(lines: &[&str], key: &str, (a, b): (usize, usize)) -> bool {
    let Some(first) = lines.get(a - 1) else {
        return false;
    };
    let t = first.trim_start();
    let starts = t.starts_with(&format!("{key} "))
        || t.starts_with(&format!("{key}="))
        || t.starts_with(&format!("[\"{key}\"]"));
    if !starts {
        return false;
    }
    let mut depth = 0i32;
    for l in &lines[a - 1..b.min(lines.len())] {
        let code = strip_strings(l);
        for c in code.chars() {
            match c {
                '{' | '(' => depth += 1,
                '}' | ')' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return false;
            }
        }
    }
    depth == 0
}

/// Ligne sans chaînes ni commentaire de fin.
fn strip_strings(l: &str) -> String {
    let mut out = String::new();
    let mut q: Option<char> = None;
    let mut chars = l.chars().peekable();
    while let Some(c) = chars.next() {
        match q {
            Some(open) => {
                if c == '\\' {
                    chars.next();
                } else if c == open {
                    q = None;
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    q = Some(c);
                } else if c == '-' && chars.peek() == Some(&'-') {
                    break;
                } else {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Commente les lignes données (indentation gardée).
pub fn comment(src: &str, lines: &BTreeSet<usize>) -> String {
    let mut out: Vec<String> = Vec::new();
    for (i, l) in src.lines().enumerate() {
        if lines.contains(&(i + 1)) && !l.trim().is_empty() && !l.trim_start().starts_with("--") {
            let indent = &l[..l.len() - l.trim_start().len()];
            out.push(format!("{indent}{MARK}{}", l.trim_start()));
        } else {
            out.push(l.to_string());
        }
    }
    let mut s = out.join("\n");
    if src.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn name_of(p: &Path) -> String {
    p.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Locales de premier niveau employées seulement par des instructions
/// commentées (la table de données d'une boucle adoptée) : à commenter aussi.
fn orphan_data(src: &str, commented: &BTreeSet<usize>) -> BTreeSet<usize> {
    let items = crate::binds_file::items(src);
    let in_commented =
        |it: &crate::binds_file::Item| (it.start + 1..=it.end).all(|l| commented.contains(&l));
    let mut out = BTreeSet::new();
    for def in items.iter().filter(|i| i.defines.is_some()) {
        let name = def.defines.as_deref().unwrap_or("");
        let users: Vec<&crate::binds_file::Item> = items
            .iter()
            .filter(|u| u.uses.contains(name) && !std::ptr::eq(*u, def))
            .collect();
        if !users.is_empty() && users.iter().all(|u| in_commented(u)) {
            out.extend(def.start + 1..=def.end);
        }
    }
    out
}

/// Courbe et animation lues d'un appel `hl.animation`.
fn anim_of(c: &Call) -> Option<(String, crate::fx::Anim)> {
    let a = c.args.first()?;
    let leaf = a.get("leaf")?.as_str()?.to_string();
    let l = crate::fx::leaf(&leaf)?;
    let curve = a
        .get("bezier")
        .or_else(|| a.get("spring"))
        .and_then(Json::as_str)
        .map(|s| s.strip_prefix("hd-").unwrap_or(s).to_string())
        .unwrap_or_else(|| "default".into());
    Some((
        leaf,
        crate::fx::Anim {
            enabled: a.get("enabled").and_then(Json::as_bool).unwrap_or(true),
            speed: a.get("speed").and_then(Json::as_f64).unwrap_or(5.0),
            curve,
            // Style que hyprdesk n'admet pas pour cet élément : laissé de
            // côté (settings.lua resterait illisible).
            style: a
                .get("style")
                .and_then(Json::as_str)
                .filter(|s| l.styles.contains(s))
                .map(str::to_string),
        },
    ))
}

/// Calcule l'adoption de ce que `src` (hyprland.lua relu) définit.
pub fn plan(
    hyprland_path: &Path,
    text: &str,
    src: &Source,
    settings: &Settings,
    cfg: &Config,
) -> Plan {
    let first = !text.contains(crate::hyprwrite::BLOCK);
    let lines: Vec<&str> = text.lines().collect();
    let here = |p: &PathBuf| p == hyprland_path;
    let mut s = settings.clone();
    let mut fx = s.fx.clone().unwrap_or_else(|| cfg.general.fx.clone());
    let mut tiling = s
        .tiling
        .clone()
        .unwrap_or_else(|| cfg.general.tiling.clone());
    let mut hypr = s.hypr.clone().unwrap_or_else(|| cfg.general.hypr.clone());
    let mut plan = Plan::default();
    let mut commented = BTreeSet::new();
    let mut shared_stmts: BTreeMap<(usize, usize), Vec<(String, String)>> = BTreeMap::new();
    // Hyprdesk garde sa valeur à la première adoption ; ensuite la main gagne.
    let take = |present: bool| !first || !present;

    for (path, o) in &src.options {
        if !here(&o.file) {
            continue;
        }
        let key = path.rsplit('.').next().unwrap_or(path);
        let where_ = format!("{}:{}", name_of(&o.file), o.lines.0);
        if o.computed {
            plan.left.push(crate::tr!(
                "{path} ({w}) : valeur calculée",
                path = path,
                w = where_
            ));
            continue;
        }
        let registry = crate::fx::opt(path)
            .map(|r| ("fx", r))
            .or_else(|| crate::tiling::opt(path).map(|r| ("tiling", r)))
            .or_else(|| crate::hyprset::opt(path).map(|r| ("hypr", r)));
        let is_input = INPUT_FIELDS.contains(&path.as_str());
        let is_layout = path == "general.layout";
        if registry.is_none() && !is_input && !is_layout {
            plan.left.push(crate::tr!(
                "{path} ({w}) : option non tenue par hyprdesk",
                path = path,
                w = where_
            ));
            continue;
        }
        // Ligne partagée : l'appel `hl.config` entier sera commenté si toutes
        // ses clés sont reprises (voir plus bas), sinon elle reste.
        let shared = !exclusive(&lines, key, o.lines);
        if is_input {
            plan.inputs = true;
        } else if is_layout {
            if let Some(l) = o.value.as_str()
                && take(tiling.layout.is_some())
            {
                tiling.layout = Some(l.strip_prefix("lua:hyprdesk-").unwrap_or(l).to_string());
            }
        } else if let Some((store, r)) = registry {
            let Some(v) = OptValue::from_json(&r.kind, &o.value) else {
                plan.left.push(crate::tr!(
                    "{path} ({w}) : valeur illisible",
                    path = path,
                    w = where_
                ));
                continue;
            };
            let map = match store {
                "fx" => &mut fx.options,
                "tiling" => &mut tiling.options,
                _ => &mut hypr.options,
            };
            if take(map.contains_key(path)) {
                map.insert(path.clone(), v);
            }
        }
        if shared {
            let stmt = src
                .calls_of("config")
                .find(|c| here(&c.file) && c.stmt.0 <= o.lines.0 && o.lines.1 <= c.stmt.1)
                .map_or(o.lines, |c| c.stmt);
            shared_stmts
                .entry(stmt)
                .or_default()
                .push((path.clone(), where_));
        } else {
            commented.extend(o.lines.0..=o.lines.1);
            plan.adopted
                .push(crate::tr!("{path} ({w})", path = path, w = where_));
        }
    }
    // Appels dont des clés sont sur des lignes partagées : commentés entiers
    // si plus aucune de leurs clés n'est laissée dans hyprland.lua.
    for (stmt, keys) in shared_stmts {
        let others_left = src.options.iter().any(|(p, o)| {
            here(&o.file)
                && stmt.0 <= o.lines.0
                && o.lines.1 <= stmt.1
                && !keys.iter().any(|(k, _)| k == p)
                && !(o.lines.0..=o.lines.1).all(|l| commented.contains(&l))
        });
        for (p, w) in keys {
            if others_left {
                plan.left.push(crate::tr!(
                    "{path} ({w}) : ligne partagée avec d'autres clés, à retirer à la main",
                    path = p,
                    w = w
                ));
            } else {
                plan.adopted
                    .push(crate::tr!("{path} ({w})", path = p, w = w));
            }
        }
        if !others_left {
            commented.extend(stmt.0..=stmt.1);
        }
    }

    for c in src
        .calls
        .iter()
        .filter(|c| here(&c.file) && c.context.is_none())
    {
        let where_ = format!("{}:{}", name_of(&c.file), c.line);
        let adopt = |what: String, plan: &mut Plan, commented: &mut BTreeSet<usize>| {
            commented.extend(c.stmt.0..=c.stmt.1);
            plan.adopted
                .push(crate::tr!("{what} ({w})", what = what, w = where_));
        };
        if c.computed
            && matches!(
                c.func.as_str(),
                "curve" | "animation" | "env" | "window_rule" | "layer_rule"
            )
        {
            plan.left.push(crate::tr!(
                "{f} ({w}) : valeur calculée",
                f = c.func,
                w = where_
            ));
            continue;
        }
        match c.func.as_str() {
            "curve" => {
                let (Some(name), Some(def)) = (
                    c.args.first().and_then(Json::as_str),
                    c.args.get(1).and_then(crate::fx::curve_from_json),
                ) else {
                    continue;
                };
                if take(fx.curves.contains_key(name)) {
                    fx.curves.insert(name.to_string(), def);
                }
                adopt(
                    crate::tr!("courbe {n}", n = name),
                    &mut plan,
                    &mut commented,
                );
            }
            "animation" => {
                let Some((leaf, a)) = anim_of(c) else {
                    continue;
                };
                if take(fx.anims.contains_key(&leaf)) {
                    fx.anims.insert(leaf.clone(), a);
                }
                adopt(
                    crate::tr!("animation {n}", n = leaf),
                    &mut plan,
                    &mut commented,
                );
            }
            "env" => {
                let (Some(k), Some(v)) = (
                    c.args.first().and_then(Json::as_str),
                    c.args.get(1).and_then(Json::as_str),
                ) else {
                    continue;
                };
                match hypr.env.iter_mut().find(|(n, _)| n == k) {
                    Some(e) if !first => e.1 = v.to_string(),
                    Some(_) => {}
                    None => hypr.env.push((k.to_string(), v.to_string())),
                }
                adopt(crate::tr!("variable {n}", n = k), &mut plan, &mut commented);
            }
            "window_rule" | "layer_rule" => {
                let Some(body) = c.args.first().cloned() else {
                    continue;
                };
                let kind = if c.func == "window_rule" {
                    crate::hyprset::RuleKind::Window
                } else {
                    crate::hyprset::RuleKind::Layer
                };
                let name = body.get("name").and_then(Json::as_str).map(str::to_string);
                let same = |r: &crate::hyprset::Rule| {
                    r.kind == kind
                        && match &name {
                            Some(n) => r.body.get("name").and_then(Json::as_str) == Some(n),
                            None => r.body == body,
                        }
                };
                match hypr.rules.iter().position(same) {
                    Some(i) if !first => hypr.rules[i].body = body,
                    Some(_) => {}
                    None => hypr.rules.push(crate::hyprset::Rule { kind, body }),
                }
                adopt(
                    crate::tr!("règle {n}", n = name.unwrap_or_else(|| c.func.clone())),
                    &mut plan,
                    &mut commented,
                );
            }
            "gesture" | "device" => {
                let Some(body) = c.args.first().filter(|b| b.is_object()).cloned() else {
                    continue;
                };
                // Action en fonction Lua : rien à écrire dans un fichier
                // généré, la ligne reste dans hyprland.lua.
                if c.computed || has_function(&body) {
                    plan.left.push(crate::tr!(
                        "{f} ({w}) : valeur calculée",
                        f = c.func,
                        w = where_
                    ));
                    continue;
                }
                let (list, what) = if c.func == "gesture" {
                    (&mut hypr.gestures, crate::i18n::tr("geste").to_string())
                } else {
                    let n = body
                        .get("name")
                        .and_then(Json::as_str)
                        .unwrap_or_default()
                        .to_string();
                    (&mut hypr.devices, crate::tr!("périphérique {n}", n = n))
                };
                let same = |g: &Json| match c.func.as_str() {
                    "device" => g.get("name") == body.get("name"),
                    _ => {
                        g.get("fingers") == body.get("fingers")
                            && g.get("direction") == body.get("direction")
                            && g.get("mods") == body.get("mods")
                    }
                };
                match list.iter().position(same) {
                    Some(i) if !first => list[i] = body,
                    Some(_) => {}
                    None => list.push(body),
                }
                adopt(what, &mut plan, &mut commented);
            }
            "permission" => {
                // `hl.permission("cible", "sorte", "décision")` ou une table
                // `{ binary, type, mode }`.
                let arg = |i: usize, key: &str| {
                    c.args
                        .get(i)
                        .and_then(Json::as_str)
                        .or_else(|| {
                            c.args
                                .first()
                                .and_then(|t| t.get(key))
                                .and_then(Json::as_str)
                        })
                        .map(str::to_string)
                };
                let (Some(target), Some(kind), Some(mode)) =
                    (arg(0, "binary"), arg(1, "type"), arg(2, "mode"))
                else {
                    continue;
                };
                if c.computed {
                    plan.left.push(crate::tr!(
                        "{f} ({w}) : valeur calculée",
                        f = c.func,
                        w = where_
                    ));
                    continue;
                }
                let p = crate::hyprset::Permission {
                    target: target.clone(),
                    kind: kind.clone(),
                    mode,
                };
                match hypr
                    .permissions
                    .iter()
                    .position(|q| q.target == p.target && q.kind == p.kind)
                {
                    Some(i) if !first => hypr.permissions[i] = p,
                    Some(_) => {}
                    None => hypr.permissions.push(p),
                }
                adopt(
                    crate::tr!("permission {k} de {t}", k = kind, t = target),
                    &mut plan,
                    &mut commented,
                );
            }
            _ => {}
        }
    }

    commented.extend(orphan_data(text, &commented));
    plan.hyprland = comment(text, &commented);
    plan.commented = commented;
    if !fx.is_empty() || s.fx.is_some() {
        s.fx = Some(fx);
    }
    if tiling != Default::default() || s.tiling.is_some() {
        s.tiling = Some(tiling);
    }
    if !hypr.is_empty() || s.hypr.is_some() {
        s.hypr = Some(hypr);
    }
    plan.settings = s;
    plan
}

/// Une valeur contient une fonction Lua (marqueur de `hyprsrc`).
fn has_function(v: &Json) -> bool {
    match v {
        Json::String(s) => s == "<fonction>",
        Json::Array(a) => a.iter().any(has_function),
        Json::Object(m) => m.values().any(has_function),
        _ => false,
    }
}

/// Résultat d'une adoption.
#[derive(Debug, Default)]
pub struct Done {
    pub adopted: Vec<String>,
    pub left: Vec<String>,
    pub files: usize,
}

/// Adopte ce que hyprland.lua définit sur les domaines de hyprdesk, écrit
/// les fichiers générés et hyprland.lua (vérifiés par Hyprland), puis
/// settings.lua. Rien n'est écrit si rien n'est à adopter et que les
/// fichiers générés sont à jour.
pub fn run(config_path: &Path) -> Result<Done, String> {
    let _reason = crate::history::reason("adoption");
    let dir = config_path.parent().unwrap_or(Path::new("."));
    let hyprland = dir.join("hyprland.lua");
    let text = std::fs::read_to_string(&hyprland).map_err(|e| e.to_string())?;
    let src = crate::hyprsrc::read(config_path);
    if let Some(e) = &src.error {
        return Err(crate::tr!("lecture de hyprland.lua : {e}", e = e));
    }
    let cfg = crate::config::lua::load_file(config_path)
        .map_err(|e| e.to_string())?
        .config;
    let settings = Settings::from_config(&crate::config::lua::load_settings_only(config_path)?);
    let p = plan(&hyprland, &text, &src, &settings, &cfg);
    let next_cfg = p.settings.apply_to(&cfg);
    let mut changes =
        crate::genfiles::changes_with(config_path, &next_cfg, Some(p.hyprland.clone()));
    if p.inputs && std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        // Valeurs en vigueur (hyprland.lua passait après inputs.lua) : elles
        // deviennent celles de inputs.lua, avec le bloc avancé.
        let path = crate::inputs::inputs_path(config_path);
        let body = crate::inputs::render(&crate::inputs::read());
        changes.retain(|c| c.path != path);
        changes.push(crate::hyprwrite::Change {
            path,
            content: Some(crate::genfiles::with_inputs_block(
                &body,
                &crate::genfiles::inputs_block(&next_cfg),
            )),
        });
    }
    let files = crate::hyprwrite::write_checked(&changes)?;
    if p.settings != settings {
        p.settings
            .save(config_path)
            .map_err(|e| crate::tr!("settings.lua : {e}", e = e))?;
    }
    Ok(Done {
        adopted: p.adopted,
        left: p.left,
        files,
    })
}

/// Résumé pour une notification ou la ligne de commande.
pub fn summary(d: &Done) -> String {
    let mut s = crate::tr!(
        "{n} réglage(s) de hyprland.lua repris par hyprdesk, {f} fichier(s) écrit(s)",
        n = d.adopted.len(),
        f = d.files
    );
    if !d.left.is_empty() {
        s.push_str(&crate::tr!(
            " ; {n} laissé(s) dans hyprland.lua",
            n = d.left.len()
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"hl.env("XCURSOR_SIZE", "24")
hl.env("PATH", os.getenv("HOME") .. "/bin")
hl.config({
    general = {
        gaps_in = 5,
        border_size = 2,
    },
    decoration = {
        blur = {
            enabled = true,
            proportional = true,
        },
    },
    misc = { vrr = 0 },
    input = {
        follow_mouse = 1,
    },
})
hl.curve("wind", { type = "bezier", points = { {0.05, 0.9}, {0.1, 1.05} } })
hl.animation({ leaf = "windows", enabled = true, speed = 6, bezier = "wind", style = "slide" })
hl.animation({ leaf = "border", enabled = true, speed = 1, bezier = "wind", style = "slide" })
local floats = { "^(a)$", "^(b)$" }
for i, c in ipairs(floats) do
    hl.window_rule({ name = "float-" .. i, match = { class = c }, float = true })
end
"#;

    fn setup(name: &str) -> (PathBuf, PathBuf) {
        let d = std::env::temp_dir().join(format!("hyprdesk-adopt-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let h = d.join("hyprland.lua");
        std::fs::write(&h, SAMPLE).unwrap();
        (d, h)
    }

    #[test]
    fn premiere_adoption() {
        let (_d, h) = setup("first");
        let src = crate::hyprsrc::read_file(&h);
        assert!(src.error.is_none(), "{:?}", src.error);
        let mut cfg = Config::default();
        // hyprdesk a déjà son flou : il le garde à la première adoption.
        cfg.general
            .fx
            .options
            .insert("decoration.blur.enabled".into(), OptValue::Bool(false));
        let settings = Settings::default();
        let p = plan(&h, SAMPLE, &src, &settings, &cfg);
        let fx = p.settings.fx.as_ref().unwrap();
        assert_eq!(fx.options["decoration.blur.enabled"], OptValue::Bool(false));
        assert!(fx.curves.contains_key("wind"));
        assert_eq!(fx.anims["windows"].curve, "wind");
        assert_eq!(fx.anims["border"].style, None, "style refusé pour border");
        // Ce que l'adoption écrit se relit (settings.lua jamais cassé).
        let lua = mlua::Lua::new();
        let t: mlua::Table = lua.load(format!("return {}", fx.to_lua())).eval().unwrap();
        crate::fx::parse(&t).expect("fx relisible");
        let hypr = p.settings.hypr.as_ref().unwrap();
        assert_eq!(hypr.options["general.gaps_in"], OptValue::Sides([5; 4]));
        assert_eq!(hypr.options["general.border_size"], OptValue::Num(2.0));
        assert_eq!(
            hypr.env,
            vec![("XCURSOR_SIZE".to_string(), "24".to_string())]
        );
        assert_eq!(hypr.rules.len(), 2);
        assert!(p.inputs);
        // Lignes commentées : clés seules, boucle et sa table de données.
        let out: Vec<&str> = p.hyprland.lines().collect();
        assert!(out[0].starts_with(MARK), "env littéral");
        assert!(!out[1].starts_with(MARK), "env calculé laissé");
        assert!(out[4].trim_start().starts_with(MARK), "gaps_in");
        assert!(out[9].trim_start().starts_with(MARK), "blur.enabled");
        assert!(out[10].contains(MARK), "proportional");
        // Toutes les clés de l'appel sont reprises : il est commenté entier,
        // ligne partagée `misc = { vrr = 0 }` comprise.
        assert!(out[13].contains(MARK), "misc = {{ vrr = 0 }}");
        assert!(
            out[2].starts_with(MARK) && out[17].starts_with(MARK),
            "appel entier"
        );
        assert!(out[15].contains(MARK), "follow_mouse");
        for l in &out[18..25] {
            assert!(l.trim_start().starts_with(MARK), "{l}");
        }
        assert!(!p.left.iter().any(|l| l.contains("misc.vrr")));
        // Le résultat reste du Lua valide, qui ne définit plus rien d'adopté.
        let (_d2, h2) = setup("after");
        std::fs::write(&h2, &p.hyprland).unwrap();
        let again = crate::hyprsrc::read_file(&h2);
        assert!(again.error.is_none(), "{:?}", again.error);
        assert!(!again.options.contains_key("general.gaps_in"));
        assert_eq!(
            again
                .calls
                .iter()
                .filter(|c| c.func == "window_rule")
                .count(),
            0
        );
    }

    #[test]
    fn ensuite_la_main_gagne() {
        let (_d, h) = setup("later");
        let text = format!(
            "{SAMPLE}\n{}\nrequire(\"appearance\")\n",
            crate::hyprwrite::BLOCK
        );
        std::fs::write(&h, &text).unwrap();
        let src = crate::hyprsrc::read_file(&h);
        let mut cfg = Config::default();
        cfg.general
            .fx
            .options
            .insert("decoration.blur.enabled".into(), OptValue::Bool(false));
        let p = plan(&h, &text, &src, &Settings::default(), &cfg);
        assert_eq!(
            p.settings.fx.unwrap().options["decoration.blur.enabled"],
            OptValue::Bool(true)
        );
    }

    #[test]
    fn gestes_peripheriques_permissions_repris() {
        let text = r#"hl.gesture({ fingers = 3, direction = "horizontal", action = "workspace" })
hl.gesture({ fingers = 4, direction = "up", action = function() end })
hl.device({ name = "epic-mouse-v1", sensitivity = -0.5 })
hl.permission("/usr/bin/grim", "screencopy", "allow")
"#;
        let (_d, h) = setup("gestures");
        std::fs::write(&h, text).unwrap();
        let src = crate::hyprsrc::read_file(&h);
        assert!(src.error.is_none(), "{:?}", src.error);
        let p = plan(&h, text, &src, &Settings::default(), &Config::default());
        let hypr = p.settings.hypr.as_ref().unwrap();
        assert_eq!(hypr.gestures.len(), 1, "action en fonction : laissée");
        assert_eq!(hypr.gestures[0]["action"], "workspace");
        assert_eq!(hypr.devices[0]["name"], "epic-mouse-v1");
        assert_eq!(
            hypr.permissions,
            vec![crate::hyprset::Permission {
                target: "/usr/bin/grim".into(),
                kind: "screencopy".into(),
                mode: "allow".into(),
            }]
        );
        assert!(p.left.iter().any(|l| l.contains("gesture")));
        // Lignes reprises commentées, celle à la fonction gardée.
        let lines: Vec<&str> = p.hyprland.lines().collect();
        assert!(lines[0].trim_start().starts_with("--"));
        assert!(!lines[1].trim_start().starts_with("--"));
        assert!(lines[3].trim_start().starts_with("--"));
        // settings.lua se relit.
        let lua = mlua::Lua::new();
        let t: mlua::Table = lua
            .load(format!("return {}", hypr.to_lua()))
            .eval()
            .unwrap();
        assert_eq!(&crate::hyprset::parse(&t).unwrap(), hypr);
    }
}
