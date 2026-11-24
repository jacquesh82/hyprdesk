//! Fichiers de configuration de Hyprland générés par hyprdesk, un par
//! domaine (MLG-139) : ce que les Réglages tiennent est **écrit**, chargé par
//! `hyprland.lua` (bloc de `require` en fin de fichier : ils gagnent sur ce
//! qui précède), et non plus seulement posé à chaud.
//!
//! - `appearance.lua` : gaps, bordures, décoration, flou, groupes ;
//! - `animations.lua` : courbes, animations, mouvements ;
//! - `layouts.lua` : layout par défaut, options, layout par workspace ;
//! - `rules.lua` : règles de fenêtres, de couches et de workspaces ;
//! - `env.lua` : variables d'environnement (curseur compris) ;
//! - `system.lua` : comportement, curseur, raccourcis, XWayland, rendu… ;
//! - `inputs.lua` (existant) gagne un bloc « avancé » (entrées, gestes,
//!   réglages par périphérique) ; `system.lua` porte aussi les permissions.
//!
//! Fichiers creux (seules les valeurs réglées ici), présentés en sections,
//! sans marqueur d'état ; un fichier vide garde son en-tête (le `require`
//! reste valide). L'écriture passe par `hyprwrite::write_checked` : une erreur
//! de config nouvelle remet tout comme avant.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::hyprset::Domain;
use crate::tiling::OptValue;

/// Modules générés, dans l'ordre de chargement, avec leur page de Réglages.
pub const MODULES: &[(&str, &str)] = &[
    ("env", "Réglages → Configuration → Environnement"),
    ("appearance", "Réglages → Fenêtres, Animations → Effets"),
    ("animations", "Réglages → Animations"),
    ("layouts", "Réglages → Layouts, Écrans"),
    ("rules", "Réglages → Fenêtres → Règles"),
    (
        "system",
        "Réglages → Configuration de Hyprland, Clavier et souris → Curseur",
    ),
];

/// Repère du bloc avancé de `inputs.lua`.
pub const INPUTS_BLOCK: &str = "-- Avancé (Réglages → Clavier et souris : Souris, Pavé tactile, Gestes, Périphériques) — généré par hyprdesk";

/// Repères des versions précédentes, encore reconnus à la réécriture.
const INPUTS_BLOCK_OLD: &[&str] =
    &["-- Avancé (Réglages → Clavier et souris → Avancé, Gestes) — généré par hyprdesk"];

fn header(module: &str, page: &str) -> String {
    format!(
        "{}\n{}\n",
        crate::tr!(
            "-- Généré par hyprdesk ({page}) — NE PAS ÉDITER : réécrit à chaque enregistrement.",
            page = crate::i18n::tr(page)
        ),
        crate::tr!(
            "-- Requis par hyprland.lua : require(\"{module}\").",
            module = module
        )
    )
}

/// Effets réellement appliqués (paquets Animations/Performance coupés : rien).
fn fx_of(cfg: &Config) -> crate::fx::Fx {
    let fx = if cfg.plugin_on("animations") || cfg.plugin_on("performance") {
        cfg.general.fx.clone()
    } else {
        Default::default()
    };
    // Mouvement réduit : aussi les animations de Hyprland (HIG FON-08).
    crate::fx::with_reduced_motion(fx, cfg.general.reduced_motion)
}

fn tiling_of(cfg: &Config) -> crate::tiling::Tiling {
    if cfg.plugin_on("layouts") {
        cfg.general.tiling.clone()
    } else {
        Default::default()
    }
}

/// Domaine d'une option d'effets (`crate::fx`).
fn fx_domain(path: &str) -> &'static str {
    match path.split('.').next().unwrap_or("") {
        "decoration" => "appearance",
        "animations" | "misc" => "animations",
        _ => "system",
    }
}

/// `hl.config({ … })` présenté sur plusieurs lignes, ou rien.
fn config_block(title: &str, opts: &BTreeMap<String, OptValue>) -> String {
    if opts.is_empty() {
        return String::new();
    }
    format!(
        "\n-- {}\nhl.config({})\n",
        crate::i18n::tr(title),
        crate::hyprset::nested(opts)
    )
}

/// Options Hyprland déclarées par le pack du thème en place
/// (`[hyprland.options]`, variante comprise), clés hyprlang
/// (`decoration:rounding`) déjà en liste blanche du moteur.
#[derive(Debug, Clone, Default)]
pub struct PackHypr {
    pub name: String,
    pub options: BTreeMap<String, String>,
    /// Couleurs des fenêtres dessinées par le gabarit du pack (MLG-264),
    /// clés Lua (`general.col.active_border`) ; vide : bordures du démon.
    pub borders: BTreeMap<String, OptValue>,
}

/// Le pack qui porte le thème des réglages, s'il déclare des options.
pub fn pack_hypr(cfg: &Config) -> Option<PackHypr> {
    let roots = crate::theme_engine::paths::Roots::from_env();
    let r = crate::theme_engine::bridge::resolved_for_look(&roots, &cfg.general.theme)?;
    let borders: BTreeMap<String, OptValue> = crate::theme_engine::targets::pack_window_colors(&r)
        .into_iter()
        .filter_map(|(k, v)| {
            let k = window_color_key(&k);
            let v = window_color(&k, &v)?;
            Some((k, v))
        })
        .collect();
    // Une ombre posée dans [hyprland.options] l'emporte sur celle du gabarit.
    let mut borders = borders;
    if r.hyprland_options.contains_key("decoration:shadow:color") {
        borders.remove("decoration.shadow.color");
    }
    (!r.hyprland_options.is_empty() || !borders.is_empty()).then_some(PackHypr {
        name: r.name,
        options: r.hyprland_options,
        borders,
    })
}

/// Clé hyprlang d'une couleur de fenêtre en chemin Lua ; l'ancienne
/// `decoration:col.shadow` devient `decoration.shadow.color`.
fn window_color_key(k: &str) -> String {
    match k {
        "decoration:col.shadow" | "decoration:shadow:color" => "decoration.shadow.color".into(),
        _ => k.replace(':', "."),
    }
}

/// Couleur hyprlang (`rgba(…)`, `#…`, dégradé `c1 c2 … 45deg`) en valeur
/// Lua ; les bordures (`*.col.*`) sont toujours des dégradés pour `hl.config`.
fn window_color(key: &str, v: &str) -> Option<OptValue> {
    let mut colors = Vec::new();
    let mut angle = None;
    for tok in v.split_whitespace() {
        if let Some(a) = tok.strip_suffix("deg") {
            angle = Some(a.parse::<f64>().ok()?.round() as i32);
        } else {
            colors.push(crate::config::Rgba::parse(tok)?);
        }
    }
    match (colors.as_slice(), angle) {
        ([], _) => None,
        ([c], None) if !key.contains(".col.") => Some(OptValue::Color(*c)),
        (_, a) => Some(OptValue::Gradient(colors, a.unwrap_or(0))),
    }
}

/// Valeur hyprlang d'un pack en valeur Lua : booléen, nombre, sinon texte
/// (couleurs `rgba(…)` comprises, que `hl.config` accepte en chaîne).
fn pack_value(v: &str) -> OptValue {
    let v = v.trim();
    match v {
        "true" | "yes" | "on" => return OptValue::Bool(true),
        "false" | "no" | "off" => return OptValue::Bool(false),
        _ => {}
    }
    v.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map(OptValue::Num)
        .unwrap_or_else(|| OptValue::Str(v.to_string()))
}

/// `animation:<élément>` = `1, 4, courbe[, style]` en `hl.animation`, si
/// l'élément est connu et que l'utilisateur ne l'a pas réglé lui-même. Une
/// courbe inconnue devient `default` (une courbe absente ferait échouer
/// toute la config, et write_checked la refuserait).
fn pack_animation(leaf: &str, v: &str, fx: &crate::fx::Fx) -> Option<String> {
    use hyprpanel_core::ipc::lua_escape;
    if fx.anims.contains_key(leaf) || !crate::fx::LEAVES.iter().any(|l| l.name == leaf) {
        return None;
    }
    let parts: Vec<&str> = v.split(',').map(str::trim).collect();
    let enabled = parts.first()? == &"1";
    let mut f = vec![
        format!("leaf = \"{}\"", lua_escape(leaf)),
        format!("enabled = {enabled}"),
    ];
    if enabled {
        let speed: f64 = parts
            .get(1)?
            .parse()
            .ok()
            .filter(|s: &f64| s.is_finite() && *s > 0.0)?;
        f.push(format!("speed = {}", OptValue::Num(speed).lua()));
        let courbe = match parts.get(2).copied().unwrap_or("default") {
            c @ ("default" | "linear") => c.to_string(),
            c if fx.curves.contains_key(c) => c.to_string(),
            c if crate::fx::curve(c).is_some() => format!("hd-{c}"),
            _ => "default".to_string(),
        };
        f.push(format!("bezier = \"{}\"", lua_escape(&courbe)));
        if let Some(style) = parts.get(3).filter(|s| !s.is_empty()) {
            f.push(format!("style = \"{}\"", lua_escape(style)));
        }
    }
    Some(format!("hl.animation({{ {} }})\n", f.join(", ")))
}

/// Bloc des options du pack, en tête d'`appearance.lua` : tout ce qui suit
/// (vos réglages, puis `animations.lua`…) passe après et gagne (MLG-255).
fn pack_block(p: &PackHypr, fx: &crate::fx::Fx) -> String {
    let mut opts = BTreeMap::new();
    let mut anims = String::new();
    for (k, v) in &p.options {
        match k.strip_prefix("animation:") {
            Some(leaf) => anims += &pack_animation(leaf, v, fx).unwrap_or_default(),
            None => {
                opts.insert(k.replace(':', "."), pack_value(v));
            }
        }
    }
    let mut s = config_block(
        &crate::tr!(
            "Options Hyprland du thème « {name} » (pack) ; vos réglages, plus bas, gagnent",
            name = p.name
        ),
        &opts,
    );
    if !anims.is_empty() {
        if s.is_empty() {
            s += &format!(
                "\n-- {}\n",
                crate::tr!("Animations du thème « {name} » (pack)", name = p.name)
            );
        }
        s += &anims;
    }
    s
}

/// Contenu de chaque module généré.
pub fn render(cfg: &Config) -> Vec<(&'static str, String)> {
    render_with(cfg, None)
}

/// Comme [`render`], avec les options Hyprland du pack en place.
pub fn render_with(cfg: &Config, pack: Option<&PackHypr>) -> Vec<(&'static str, String)> {
    let fx = fx_of(cfg);
    let tiling = tiling_of(cfg);
    let hypr = &cfg.general.hypr;
    let mut out = Vec::new();
    for (module, page) in MODULES {
        let mut body = String::new();
        match *module {
            "env" => {
                if !cfg.general.hyprland_gpu.is_empty() {
                    body += &format!(
                        "\n-- {}\n{}",
                        crate::i18n::tr(
                            "Carte de rendu de Hyprland (Réglages → Performance → Carte graphique), retrouvée par son adresse PCI"
                        ),
                        crate::render_gpu::env_lua(&cfg.general.hyprland_gpu)
                    );
                }
                if !hypr.env.is_empty() {
                    body += &format!("\n-- {}\n", crate::i18n::tr("Variables d'environnement"));
                    for (k, v) in &hypr.env {
                        body += &format!(
                            "hl.env(\"{}\", \"{}\")\n",
                            hyprpanel_core::ipc::lua_escape(k),
                            hyprpanel_core::ipc::lua_escape(v)
                        );
                    }
                }
            }
            "appearance" => {
                if let Some(p) = pack {
                    body += &pack_block(p, &fx);
                }
                let mut opts: BTreeMap<String, OptValue> = hypr
                    .options
                    .iter()
                    .filter(|(k, _)| crate::hyprset::domain(k) == Domain::Appearance)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                opts.extend(
                    fx.options
                        .iter()
                        .filter(|(k, _)| fx_domain(k) == "appearance")
                        .map(|(k, v)| (k.clone(), v.clone())),
                );
                if hypr.theme_borders
                    && let Some(p) = pack.filter(|p| !p.borders.is_empty())
                {
                    // Bordures aux couleurs du thème : celles que le pack
                    // dessine dans son gabarit Hyprland (MLG-264).
                    opts.extend(p.borders.iter().map(|(k, v)| (k.clone(), v.clone())));
                } else if hypr.theme_borders {
                    // Bordures aux couleurs du thème (A6) : accent pour la
                    // fenêtre active, fond translucide pour les autres.
                    let th = &cfg.general.theme;
                    let crate::config::Rgba(r, g, b, _) = th.bg;
                    opts.insert(
                        "general.col.active_border".into(),
                        OptValue::Gradient(vec![th.accent, th.fg], 45),
                    );
                    opts.insert(
                        "general.col.inactive_border".into(),
                        OptValue::Gradient(vec![crate::config::Rgba(r, g, b, 0xaa)], 0),
                    );
                }
                body += &config_block("Fenêtres, bordures, décoration, groupes", &opts);
            }
            "animations" => body += &animations_lua(&fx),
            "layouts" => {
                body += &layouts_lua(&tiling);
                // Écrans découpés (Réglages → Écrans) : indépendants du paquet Layouts.
                if !cfg.general.screen_zones.is_empty() {
                    body += &format!(
                        "\n-- {}\n{}",
                        crate::i18n::tr("Écrans découpés en zones (Réglages → Écrans)"),
                        crate::zones::screens_lua(&cfg.general.screen_zones)
                    );
                }
            }
            "rules" => {
                if hypr.rules.iter().any(crate::hyprset::rule_complete) {
                    body += &format!("\n-- {}\n", crate::i18n::tr("Règles, dans l'ordre"));
                    for r in hypr
                        .rules
                        .iter()
                        .filter(|r| crate::hyprset::rule_complete(r))
                    {
                        body +=
                            &format!("{}({})\n", r.kind.func(), crate::hyprset::json_lua(&r.body));
                    }
                }
            }
            "system" => {
                let mut opts: BTreeMap<String, OptValue> = hypr
                    .options
                    .iter()
                    .filter(|(k, _)| crate::hyprset::domain(k) == Domain::System)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                opts.extend(
                    fx.options
                        .iter()
                        .filter(|(k, _)| fx_domain(k) == "system")
                        .map(|(k, v)| (k.clone(), v.clone())),
                );
                body += &config_block("Comportement, curseur, raccourcis, rendu", &opts);
                if hypr.permissions.iter().any(|p| !p.target.trim().is_empty()) {
                    body += &format!(
                        "\n-- {}\n{}",
                        crate::i18n::tr("Permissions (lues au démarrage de Hyprland seulement)"),
                        crate::hyprset::permissions_lua(&hypr.permissions)
                    );
                }
            }
            _ => {}
        }
        out.push((*module, format!("{}{body}", header(module, page))));
    }
    out
}

fn animations_lua(fx: &crate::fx::Fx) -> String {
    let mut s = String::new();
    let opts: BTreeMap<String, OptValue> = fx
        .options
        .iter()
        .filter(|(k, _)| fx_domain(k) == "animations")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    s += &config_block("Animations", &opts);
    let lua = crate::fx::file_lua(fx);
    if !lua.is_empty() {
        s += &format!(
            "\n-- {}\n{lua}",
            crate::i18n::tr("Courbes et éléments animés")
        );
    }
    s
}

fn layouts_lua(t: &crate::tiling::Tiling) -> String {
    let lua = crate::tiling::file_lua(t);
    if lua.is_empty() {
        String::new()
    } else {
        format!("\n-- {}\n{lua}", crate::i18n::tr("Layouts"))
    }
}

/// Bloc avancé de `inputs.lua` (entrées, gestes, périphériques).
pub fn inputs_block(cfg: &Config) -> String {
    let hypr = &cfg.general.hypr;
    let opts: BTreeMap<String, OptValue> = hypr
        .options
        .iter()
        .filter(|(k, _)| crate::hyprset::domain(k) == Domain::Inputs)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let gestures = crate::hyprset::calls_lua("gesture", &hypr.gestures);
    let devices = crate::hyprset::calls_lua("device", &hypr.devices);
    if opts.is_empty() && gestures.is_empty() && devices.is_empty() {
        return String::new();
    }
    let mut out = format!("\n{}\n", crate::i18n::tr(INPUTS_BLOCK));
    if !opts.is_empty() {
        out += &format!("hl.config({})\n", crate::hyprset::nested(&opts));
    }
    if !gestures.is_empty() {
        out += &format!(
            "-- {}\n{gestures}",
            crate::i18n::tr("Gestes du pavé tactile")
        );
    }
    if !devices.is_empty() {
        out += &format!(
            "-- {}\n{devices}",
            crate::i18n::tr("Réglages par périphérique")
        );
    }
    out
}

/// `inputs.lua` avec le bloc avancé remplacé (le reste, écrit par la page
/// Clavier et souris, ne bouge pas). Repère reconnu en français et anglais.
pub fn with_inputs_block(current: &str, block: &str) -> String {
    let marks: Vec<String> = std::iter::once(INPUTS_BLOCK)
        .chain(INPUTS_BLOCK_OLD.iter().copied())
        .flat_map(|m| {
            [
                m.to_string(),
                crate::i18n::tr_in(crate::i18n::Lang::En, m).to_string(),
            ]
        })
        .collect();
    let mut base: Vec<&str> = Vec::new();
    for l in current.lines() {
        if marks.iter().any(|m| l.trim() == m) {
            break;
        }
        base.push(l);
    }
    let mut out = base.join("\n").trim_end().to_string();
    out.push('\n');
    out.push_str(block);
    out
}

fn dir(config_path: &Path) -> PathBuf {
    config_path.parent().unwrap_or(Path::new(".")).to_path_buf()
}

/// Changements à écrire pour mettre les fichiers générés (et le bloc de
/// `require` de hyprland.lua) en accord avec `cfg`.
pub fn changes(config_path: &Path, cfg: &Config) -> Vec<crate::hyprwrite::Change> {
    changes_with(config_path, cfg, None)
}

/// Comme `changes`, avec un hyprland.lua déjà modifié (adoption) au lieu de
/// celui du disque.
pub fn changes_with(
    config_path: &Path,
    cfg: &Config,
    hyprland_src: Option<String>,
) -> Vec<crate::hyprwrite::Change> {
    let d = dir(config_path);
    let mut out = Vec::new();
    for (module, content) in render_with(cfg, pack_hypr(cfg).as_ref()) {
        out.push(crate::hyprwrite::Change {
            path: d.join(format!("{module}.lua")),
            content: Some(content),
        });
    }
    let inputs = d.join("inputs.lua");
    if let Ok(cur) = std::fs::read_to_string(&inputs)
        && crate::hyprsrc::generated(&cur)
    {
        out.push(crate::hyprwrite::Change {
            path: inputs,
            content: Some(with_inputs_block(&cur, &inputs_block(cfg))),
        });
    }
    let hyprland = d.join("hyprland.lua");
    let disk = std::fs::read_to_string(&hyprland).ok();
    if let Some(src) = hyprland_src.or_else(|| disk.clone()) {
        let mut s = src.clone();
        for (module, _) in MODULES {
            if let Some(n) = crate::hyprwrite::with_require(&s, module) {
                s = n;
            }
        }
        if Some(&s) != disk.as_ref() {
            out.push(crate::hyprwrite::Change {
                path: hyprland,
                content: Some(s),
            });
        }
    }
    out
}

/// Écrit les fichiers générés (vérifiés par Hyprland) ; nombre de fichiers
/// changés.
pub fn sync(config_path: &Path, cfg: &Config) -> Result<usize, String> {
    crate::hyprwrite::write_checked(&changes(config_path, cfg))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mouvement réduit : les animations de Hyprland suivent, même sans le
    /// paquet Animations ; les fondus restent (HIG FON-08).
    #[test]
    fn mouvement_reduit_coupe_les_glissements_de_hyprland() {
        let mut cfg = Config::default();
        let calm: BTreeMap<_, _> = render(&cfg).into_iter().collect();
        assert!(
            !calm["animations"].contains("hl.animation"),
            "rien sans réglage"
        );
        cfg.general.reduced_motion = true;
        let files: BTreeMap<_, _> = render(&cfg).into_iter().collect();
        let a = &files["animations"];
        for leaf in ["windows", "windowsIn", "layers", "borderangle"] {
            assert!(
                a.contains(&format!("leaf = \"{leaf}\", enabled = false")),
                "{leaf} coupé : {a}"
            );
        }
        assert!(
            a.contains("leaf = \"workspaces\", enabled = true, speed = 2, bezier = \"default\", style = \"fade\""),
            "workspaces en fondu : {a}"
        );
        assert!(
            !a.contains("\"fade\", enabled = false"),
            "les fondus restent"
        );
    }

    #[test]
    fn fichiers_creux_et_sans_marqueur() {
        let mut cfg = Config::default();
        cfg.general
            .hypr
            .options
            .insert("general.border_size".into(), OptValue::Num(2.0));
        cfg.general
            .hypr
            .options
            .insert("misc.disable_hyprland_logo".into(), OptValue::Bool(true));
        cfg.general
            .hypr
            .options
            .insert("input.force_no_accel".into(), OptValue::Bool(true));
        cfg.general
            .hypr
            .env
            .push(("XCURSOR_SIZE".into(), "24".into()));
        cfg.general.hypr.rules.push(crate::hyprset::Rule {
            kind: crate::hyprset::RuleKind::Window,
            body: serde_json::json!({ "name": "f", "match": { "class": "^(x)$" }, "float": true }),
        });
        let files: BTreeMap<_, _> = render(&cfg).into_iter().collect();
        assert!(files["appearance"].contains("border_size = 2,"));
        assert!(files["system"].contains("disable_hyprland_logo = true,"));
        assert!(
            !files["system"].contains("force_no_accel"),
            "va dans inputs.lua"
        );
        assert!(files["env"].contains("hl.env(\"XCURSOR_SIZE\", \"24\")"));
        assert!(files["rules"].contains("hl.window_rule({"));
        for (m, c) in &files {
            assert!(crate::hyprsrc::generated(c), "{m} : en-tête");
            assert!(!c.contains("__hyprdesk"), "{m} : marqueur d'état");
        }
        // Rien de réglé : en-tête seul.
        assert_eq!(files["layouts"].lines().count(), 2);
        let block = inputs_block(&cfg);
        assert!(block.contains("force_no_accel = true,"));
        let inputs = "-- Généré par hyprdesk\nhl.config({ input = {} })\n";
        let a = with_inputs_block(inputs, &block);
        let b = with_inputs_block(&a, &block);
        assert_eq!(a, b, "bloc remplacé, pas ajouté");
        assert!(a.starts_with(inputs.trim_end()));
    }

    #[test]
    fn fichiers_valides_pour_hyprsrc() {
        // Ce que hyprdesk écrit se relit à l'identique.
        let d = std::env::temp_dir().join(format!("hyprdesk-genfiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let mut cfg = Config::default();
        cfg.general.hypr.options.insert(
            "general.col.active_border".into(),
            OptValue::Gradient(vec![crate::config::Rgba(1, 2, 3, 255)], 45),
        );
        let files: BTreeMap<_, _> = render(&cfg).into_iter().collect();
        // Sans l'en-tête « généré », pour que hyprsrc le lise.
        let body: String = files["appearance"]
            .lines()
            .skip(2)
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(d.join("hyprland.lua"), body).unwrap();
        let s = crate::hyprsrc::read_file(&d.join("hyprland.lua"));
        assert!(s.error.is_none(), "{:?}", s.error);
        let v = &s.options["general.col.active_border"].value;
        assert_eq!(v["angle"], serde_json::json!(45));
    }

    #[test]
    fn gestes_peripheriques_permissions_ecrits() {
        let mut cfg = Config::default();
        let h = &mut cfg.general.hypr;
        h.gestures.push(
            serde_json::json!({ "fingers": 3, "direction": "horizontal", "action": "workspace" }),
        );
        h.gestures.push(serde_json::json!({ "fingers": 4 }));
        h.devices
            .push(serde_json::json!({ "name": "epic-mouse-v1", "sensitivity": -0.5 }));
        h.devices.push(serde_json::json!({ "name": "" }));
        h.permissions.push(crate::hyprset::Permission {
            target: "/usr/bin/grim".into(),
            kind: "screencopy".into(),
            mode: "allow".into(),
        });
        h.rules.push(crate::hyprset::Rule {
            kind: crate::hyprset::RuleKind::Workspace,
            body: serde_json::json!({ "workspace": "" }),
        });
        h.rules.push(crate::hyprset::Rule {
            kind: crate::hyprset::RuleKind::Layer,
            body: serde_json::json!({ "match": { "namespace": "^waybar$" }, "blur": true }),
        });
        let block = inputs_block(&cfg);
        assert_eq!(
            block.matches("hl.gesture(").count(),
            1,
            "geste incomplet écarté"
        );
        assert_eq!(
            block.matches("hl.device(").count(),
            1,
            "périphérique sans nom écarté"
        );
        assert!(block.contains("sensitivity = -0.5"));
        let files: BTreeMap<_, _> = render(&cfg).into_iter().collect();
        assert!(
            files["system"].contains("hl.permission(\"/usr/bin/grim\", \"screencopy\", \"allow\")")
        );
        assert!(files["rules"].contains("hl.layer_rule("));
        assert!(
            !files["rules"].contains("hl.workspace_rule("),
            "règle sans workspace écartée"
        );
        // Ancien repère du bloc reconnu : remplacé, pas doublé.
        let old = format!(
            "hl.config({{}})\n\n{}\nhl.config({{ input = {{}} }})\n",
            INPUTS_BLOCK_OLD[0]
        );
        let new = with_inputs_block(&old, &block);
        assert!(!new.contains(INPUTS_BLOCK_OLD[0]));
        assert_eq!(new.matches(INPUTS_BLOCK).count(), 1);
    }

    #[test]
    fn options_hyprland_du_pack_en_tête_d_appearance() {
        // MLG-255 : les options du pack passent avant celles de
        // l'utilisateur, qui gagnent ; animation traduite, courbe inconnue
        // ramenée à `default`, élément réglé par l'utilisateur laissé.
        let mut cfg = Config::default();
        cfg.general
            .hypr
            .options
            .insert("decoration.rounding".to_string(), OptValue::Num(4.0));
        let pack = PackHypr {
            name: "Duo".to_string(),
            options: [
                ("decoration:rounding", "12"),
                ("decoration:blur:enabled", "true"),
                ("general:gaps_in", "6"),
                ("animation:windows", "1, 4, inconnue, popin 80%"),
                ("animation:pasunélément", "1, 4, default"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
            borders: BTreeMap::new(),
        };
        let files: BTreeMap<_, _> = render_with(&cfg, Some(&pack)).into_iter().collect();
        let a = &files["appearance"];
        let pack_at = a.find("Duo").expect(a);
        let user_at = a.rfind("rounding = 4").expect(a);
        assert!(pack_at < user_at, "{a}");
        assert!(
            a.contains("rounding = 12") && a.contains("gaps_in = 6"),
            "{a}"
        );
        assert!(a.contains("enabled = true"), "{a}");
        assert!(
            a.contains("hl.animation({ leaf = \"windows\", enabled = true, speed = 4, bezier = \"default\", style = \"popin 80%\" })"),
            "{a}"
        );
        assert!(!a.contains("pasunélément"), "{a}");
        // Sans pack : rien de plus.
        let sans: BTreeMap<_, _> = render(&cfg).into_iter().collect();
        assert!(!sans["appearance"].contains("Duo"));
    }

    #[test]
    fn bordures_dessinées_par_le_pack() {
        // MLG-264 : bordures du thème activées ; le pack qui dessine les
        // siennes remplace le dégradé d'accent du démon.
        let mut cfg = Config::default();
        cfg.general.hypr.theme_borders = true;
        let borders: BTreeMap<String, OptValue> = [
            (
                "general:col.active_border",
                "rgba(cdeeff8c) rgba(cdeeff2e) rgba(2dd4bf8c) 135deg",
            ),
            ("general:col.inactive_border", "rgba(cdeeff24)"),
            ("decoration:col.shadow", "rgba(12151c73)"),
            ("general:col.nogroup_border", "pas une couleur"),
        ]
        .into_iter()
        .filter_map(|(k, v)| {
            let k = window_color_key(k);
            let v = window_color(&k, v)?;
            Some((k, v))
        })
        .collect();
        assert_eq!(borders.len(), 3, "valeur illisible écartée");
        let pack = PackHypr {
            name: "Nacre".to_string(),
            options: BTreeMap::new(),
            borders,
        };
        let files: BTreeMap<_, _> = render_with(&cfg, Some(&pack)).into_iter().collect();
        let a = &files["appearance"];
        assert!(
            a.contains("active_border = { colors = { \"rgba(cdeeff8c)\", \"rgba(cdeeff2e)\", \"rgba(2dd4bf8c)\" }, angle = 135 }"),
            "{a}"
        );
        assert!(
            a.contains("inactive_border = { colors = { \"rgba(cdeeff24)\" }, angle = 0 }"),
            "{a}"
        );
        assert!(a.contains("color = \"rgba(12151c73)\""), "{a}");
        // Sans bordures de pack : le dégradé d'accent du démon.
        let sans: BTreeMap<_, _> = render(&cfg).into_iter().collect();
        assert!(
            sans["appearance"].contains("angle = 45"),
            "{}",
            sans["appearance"]
        );
    }
}
