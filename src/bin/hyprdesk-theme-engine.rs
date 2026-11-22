//! `hyprdesk-theme-engine` : binaire dédié du moteur de thème
//! (docs/THEME-ENGINE.md §12). La même bibliothèque que le démon, en ligne de
//! commande : il ne lie ni Lua, ni iced, ni l'index.
//!
//! Commandes : `list`, `info`, `use`, `apply`, `generate`, `palette`,
//! `verify`, `doctor`. Toutes acceptent `--json` ; les modes d'application
//! sont `persist`, `hot` et `dry-run`. La variable `HYPRDESK_THEME=id[:variante]`
//! surcharge le thème actif (équivalent de `GTK_THEME`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hyprdesk::theme_engine::{
    apply::{self, Mode, Options},
    archive, bundled, color, intent, packs,
    paths::Roots,
    recolor, resolve, state, watch,
};

/// Paire code de sortie / message d'erreur.
type Failure = (u8, String);

fn main() -> ExitCode {
    // Pipe fermé (`| head`) : sortir proprement au lieu de paniquer sur une
    // écriture à un stdout mort.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => ExitCode::from(code),
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn run(args: &[String]) -> Result<u8, Failure> {
    let mut flags = Flags::default();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--json" => flags.json = true,
            "--help" | "-h" => {
                print_usage();
                return Ok(0);
            }
            "--variant" => flags.variant = Some(arg_value(args, &mut i, "--variant")?),
            "--mode" => flags.mode = Some(arg_value(args, &mut i, "--mode")?),
            "--targets" => flags.targets = Some(arg_value(args, &mut i, "--targets")?),
            "--origin" => flags.origin = Some(arg_value(args, &mut i, "--origin")?),
            "--out" => flags.out = Some(PathBuf::from(arg_value(args, &mut i, "--out")?)),
            "--format" => flags.format = Some(arg_value(args, &mut i, "--format")?),
            "--from" => flags.from = Some(arg_value(args, &mut i, "--from")?),
            "--scheme" => flags.scheme = Some(arg_value(args, &mut i, "--scheme")?),
            "--hue-lock" => {
                flags.hue_lock = Some(arg_value(args, &mut i, "--hue-lock")?);
            }
            "--intent" => flags.intent = Some(arg_value(args, &mut i, "--intent")?),
            _ if a.starts_with('-') => {
                return Err((1, format!("option inconnue : {a} (--help pour l'usage)")));
            }
            _ => positional.push(a.to_string()),
        }
        i += 1;
    }
    let Some(command) = positional.first().cloned() else {
        print_usage();
        return Err((1, "commande attendue".to_string()));
    };
    let rest = &positional[1..];
    let roots = Roots::from_env();
    match command.as_str() {
        "list" => list(&roots, &flags),
        "info" => info(&roots, &flags, rest),
        "use" => use_pack(&roots, &flags, rest),
        "apply" => apply_active(&roots, &flags),
        "generate" => generate(&roots, &flags),
        "palette" => palette(&roots, &flags),
        "verify" => verify(&roots, &flags),
        "doctor" => doctor(&roots, &flags),
        "export" => export(&roots, &flags, rest),
        "install" => install(&roots, rest, flags.origin.as_deref()),
        "remove" => remove(&roots, rest, flags.origin.as_deref()),
        "build" => build(&roots, &flags),
        "recolor" => recolor_cmd(&roots, &flags),
        "tools" => tools(&flags),
        "diff" => diff(&roots, &flags, rest),
        "preview" => preview(&roots, &flags, rest),
        "watch" => watch_cmd(&roots, &flags),
        other => Err((
            1,
            format!("commande inconnue : {other} (--help pour l'usage)"),
        )),
    }
}

/// Options globales de la ligne de commande.
#[derive(Default, Clone)]
struct Flags {
    json: bool,
    variant: Option<String>,
    mode: Option<String>,
    targets: Option<String>,
    origin: Option<String>,
    out: Option<PathBuf>,
    format: Option<String>,
    from: Option<String>,
    scheme: Option<String>,
    hue_lock: Option<String>,
    intent: Option<String>,
}

fn arg_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, Failure> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| (1u8, format!("{flag} attend une valeur")))
}

/// Variante effective : `--variant` > `HYPRDESK_THEME` > défaut du pack.
/// `auto` suit le réglage clair/sombre du système (gsettings, sinon sombre).
fn effective_variant(flags: &Flags, pack: &packs::Pack) -> Result<String, Failure> {
    let want = flags
        .variant
        .clone()
        .or_else(|| env_theme().and_then(|(_, v)| v))
        .unwrap_or_else(|| "auto".to_string());
    Ok(match want.as_str() {
        "dark" | "light" => want,
        "auto" | "" => gsettings_scheme()
            .unwrap_or_else(|| pack.manifest.default_variant())
            .to_string(),
        other => {
            return Err((
                1,
                format!("variante inconnue : {other} (attendu : dark, light ou auto)"),
            ));
        }
    })
}

/// `HYPRDESK_THEME=id[:variante]`.
fn env_theme() -> Option<(String, Option<String>)> {
    let raw = std::env::var("HYPRDESK_THEME").ok()?;
    let raw = raw.trim().to_string();
    if raw.is_empty() {
        return None;
    }
    match raw.split_once(':') {
        Some((id, v)) => Some((id.to_string(), Some(v.to_string()))),
        None => Some((raw, None)),
    }
}

/// Schéma clair/sombre du système, si gsettings répond.
fn gsettings_scheme() -> Option<&'static str> {
    let out = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    if text.contains("light") {
        Some("light")
    } else {
        None
    }
}

fn mode_of(flags: &Flags) -> Result<Mode, Failure> {
    let want = flags.mode.clone().unwrap_or_else(|| "persist".to_string());
    Mode::parse(&want).ok_or_else(|| (1, format!("mode inconnu : {want}")))
}

fn options_of(flags: &Flags) -> Result<Options, Failure> {
    Ok(Options {
        mode: mode_of(flags)?,
        only: flags
            .targets
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect(),
        origin: flags.origin.clone().unwrap_or_else(|| "cli".to_string()),
        check_installed: true,
        reload: true,
        target_opts: BTreeMap::new(),
        reload_commands: BTreeMap::new(),
    })
}

/// Chaîne + palette résolue d'un pack, pour `info` et `use`.
fn resolved_of(
    roots: &Roots,
    flags: &Flags,
    id: &str,
) -> Result<(resolve::Resolved, Option<PathBuf>), Failure> {
    let (all, warnings) =
        packs::discover(roots, &bundled::packs()).map_err(|e| (2u8, e.to_string()))?;
    let chain = packs::chain(&all, id).map_err(|e| (2u8, e.to_string()))?;
    let pack = chain
        .last()
        .ok_or_else(|| (2u8, "chaîne d'héritage vide".to_string()))?;
    let variant = effective_variant(flags, pack)?;
    let resolved = resolve::resolve(&chain, &variant).map_err(|e| (2u8, e.to_string()))?;
    let mut warnings = warnings;
    warnings.extend(resolved.warnings.clone());
    for w in warnings {
        eprintln!("avertissement : {w}");
    }
    let pack = packs::find(&all, id).ok_or_else(|| (2u8, format!("pack introuvable : {id}")))?;
    Ok((resolved, pack.dir.clone()))
}

fn list(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    let (all, warnings) =
        packs::discover(roots, &bundled::packs()).map_err(|e| (2u8, e.to_string()))?;
    for w in warnings {
        eprintln!("avertissement : {w}");
    }
    if flags.json {
        let items: Vec<serde_json::Value> = all
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.manifest.meta.id,
                    "name": p.manifest.meta.name,
                    "version": p.manifest.meta.version,
                    "origin": p.origin.as_str(),
                    "description": p.manifest.meta.description,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&items).map_err(json_err)?
        );
    } else {
        let active = state::read_active(roots).map(|a| a.pack);
        println!("{:<20} {:<16} {:<8} origine", "id", "nom", "version");
        for p in &all {
            let mark = if active.as_deref() == Some(p.manifest.meta.id.as_str()) {
                " *"
            } else {
                ""
            };
            println!(
                "{:<20} {:<16} {:<8} {}{}",
                p.manifest.meta.id,
                p.manifest.meta.name,
                p.manifest.meta.version,
                p.origin.as_str(),
                mark
            );
        }
    }
    Ok(0)
}

fn info(roots: &Roots, flags: &Flags, rest: &[String]) -> Result<u8, Failure> {
    let id = rest
        .first()
        .ok_or_else(|| (1u8, "usage : info <id>".to_string()))?;
    let (resolved, _) = resolved_of(roots, flags, id)?;
    if flags.json {
        let palette: serde_json::Map<String, serde_json::Value> = resolved
            .palette
            .iter()
            .map(|(k, c)| {
                (
                    k.clone(),
                    serde_json::Value::String(color::fmt(*c, color::Fmt::Hex)),
                )
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": resolved.id,
                "name": resolved.name,
                "version": resolved.version,
                "variant": resolved.variant,
                "palette": palette,
                "warnings": resolved.warnings,
            }))
            .map_err(json_err)?
        );
    } else {
        println!(
            "{} {} v{} — variante {}",
            resolved.id, resolved.name, resolved.version, resolved.variant
        );
        for (k, c) in &resolved.palette {
            println!("  {k:<16} {}", color::fmt(*c, color::Fmt::Hex));
        }
    }
    Ok(0)
}

fn use_pack(roots: &Roots, flags: &Flags, rest: &[String]) -> Result<u8, Failure> {
    let id = rest
        .first()
        .cloned()
        .or_else(|| env_theme().map(|(id, _)| id))
        .ok_or_else(|| (1u8, "usage : use <id>".to_string()))?;
    let origin = flags.origin.clone().unwrap_or_else(|| "cli".to_string());
    policy_gate(&origin, true)?;
    // Pack embarqué : lu depuis le binaire, jamais recopié (MLG-254 : une
    // copie figée masquait ses corrections).
    let (resolved, dir) = resolved_of(roots, flags, &id)?;
    state::write_active(
        roots,
        &state::ActiveTheme {
            pack: id.clone(),
            variant: resolved.variant.clone(),
            origin: flags.origin.clone().unwrap_or_else(|| "cli".to_string()),
        },
    )
    .map_err(engine_err)?;
    let report = apply::apply(&resolved, dir.as_deref(), roots, &options_of(flags)?);
    print_report(&report, flags)?;
    Ok(if report.errors.is_empty() { 0 } else { 3 })
}

fn apply_active(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    let active = match state::read_active(roots) {
        Some(a) => a,
        None => match env_theme() {
            Some((id, _)) => state::ActiveTheme {
                pack: id,
                variant: "dark".to_string(),
                origin: "env".to_string(),
            },
            None => {
                return Err((
                    4,
                    "aucun thème actif : `hyprdesk-theme-engine use <id>` d'abord".to_string(),
                ));
            }
        },
    };
    let mut flags = flags.clone();
    if let Some((_, v)) = env_theme()
        && flags.variant.is_none()
    {
        flags.variant = v;
    }
    let (resolved, dir) = resolved_of(roots, &flags, &active.pack)?;
    let report = apply::apply(&resolved, dir.as_deref(), roots, &options_of(&flags)?);
    print_report(&report, &flags)?;
    Ok(if report.errors.is_empty() { 0 } else { 3 })
}

fn generate(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    let out = flags
        .out
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("hyprdesk-theme-engine-generate"));
    let active = state::read_active(roots).ok_or_else(|| {
        (
            4u8,
            "aucun thème actif : `hyprdesk-theme-engine use <id>` d'abord".to_string(),
        )
    })?;
    let (resolved, dir) = resolved_of(roots, flags, &active.pack)?;
    std::fs::create_dir_all(&out).map_err(io_err)?;
    // Racines factices : generate écrit dans --out, pas aux emplacements
    // réels ; les chemins planifiés ne servent qu'à nommer les fichiers.
    let dummy = Roots::at(&std::env::temp_dir());
    let cibles = hyprdesk::theme_engine::targets::all(&resolved);
    for target in &cibles {
        let planned =
            hyprdesk::theme_engine::targets::plan(target, &resolved, dir.as_deref(), &dummy)
                .map_err(|e| (2u8, e.to_string()))?;
        for p in planned {
            let rel = p
                .path
                .strip_prefix(&dummy.config)
                .unwrap_or(p.path.as_path());
            let name = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("--");
            std::fs::write(out.join(&name), &p.content).map_err(io_err)?;
            println!("{}", out.join(name).display());
        }
    }
    Ok(0)
}

fn palette(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    let doc = state::read_palette(roots)
        .ok_or_else(|| (4u8, "palette non publiée : apply d'abord".to_string()))?;
    let format = flags.format.clone().unwrap_or_else(|| "json".to_string());
    match format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&doc).map_err(json_err)?),
        "hex" => {
            for (k, v) in &doc.colors {
                println!("{k:<16} {v}");
            }
        }
        "env" => {
            for (slot, var) in [
                ("bg", "HYPRDESK_BG"),
                ("fg", "HYPRDESK_FG"),
                ("accent", "HYPRDESK_ACCENT"),
                ("surface", "HYPRDESK_SURFACE"),
                ("border", "HYPRDESK_BORDER"),
            ] {
                if let Some(v) = doc.colors.get(slot) {
                    println!("{var}={v}");
                }
            }
            println!("HYPRDESK_THEME_ID={}", doc.pack);
            println!("HYPRDESK_THEME_VARIANT={}", doc.variant);
        }
        "css" => {
            println!(":root {{");
            for (k, v) in &doc.colors {
                println!("  --hd-{k}: {v};");
            }
            println!("}}");
        }
        other => return Err((1, format!("format inconnu : {other} (json, hex, env, css)"))),
    }
    Ok(0)
}

fn verify(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    let doc = state::read_palette(roots)
        .ok_or_else(|| (4u8, "palette non publiée : apply d'abord".to_string()))?;
    let parse =
        |hex: &str| color::parse(hex).ok_or_else(|| (2u8, format!("couleur invalide : {hex}")));
    let pairs = [
        ("fg/bg", "fg", "bg", 4.5),
        ("fg_muted/surface", "fg_muted", "surface", 3.0),
        ("accent_fg/accent", "accent_fg", "accent", 4.5),
        ("fg/background", "fg", "background", 4.5),
    ];
    let mut results = Vec::new();
    let mut worst = f64::INFINITY;
    for (label, a, b, min) in pairs {
        let (ca, cb) = (
            parse(doc.colors.get(a).unwrap_or(&String::new()))?,
            parse(doc.colors.get(b).unwrap_or(&String::new()))?,
        );
        let ratio = color::contrast(ca, cb);
        worst = worst.min(ratio);
        let verdict = if ratio >= 7.0 {
            "AAA"
        } else if ratio >= min {
            "AA"
        } else {
            "sous AA"
        };
        results.push((label, ratio, verdict));
    }
    if flags.json {
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|(l, r, v)| {
                serde_json::json!({ "paire": l, "ratio": (r * 100.0).round() / 100.0, "verdict": v })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&items).map_err(json_err)?
        );
    } else {
        for (label, ratio, verdict) in &results {
            println!("{label:<20} {ratio:>5.2}  {verdict}");
        }
    }
    // Sous 3.0 sur une paire essentielle : la palette est illisible.
    Ok(if worst < 3.0 { 3 } else { 0 })
}

fn doctor(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    let deps = [
        ("hyprctl", "cible hyprland"),
        ("hyprlock", "cible hyprlock"),
        ("kitty", "cible kitty"),
        ("gsettings", "cible gtk (schéma clair/sombre)"),
    ];
    let mut missing = Vec::new();
    for (dep, role) in deps {
        let ok = hyprdesk::theme_engine::targets::in_path(dep);
        if !ok {
            missing.push(dep.to_string());
        }
        if !flags.json {
            println!(
                "{:<10} {} ({role})",
                dep,
                if ok { "présente" } else { "absente" }
            );
        }
    }
    // Packs invalides repérés à la découverte.
    let (_, warnings) =
        packs::discover(roots, &bundled::packs()).map_err(|e| (2u8, e.to_string()))?;
    for w in &warnings {
        if !flags.json {
            println!("avertissement : {w}");
        }
    }
    if flags.json {
        let items: Vec<serde_json::Value> = deps
            .iter()
            .map(|(d, r)| {
                serde_json::json!({
                    "dépendance": d, "rôle": r,
                    "présente": hyprdesk::theme_engine::targets::in_path(d)
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "dépendances": items,
                "avertissements": warnings,
                "manquantes": missing,
            }))
            .map_err(json_err)?
        );
    }
    Ok(0)
}

/// Surface d'outils de LYRA (§17.1) : le manifeste que le démon passe au
/// modèle (function calling). Les classes bornent ce qui demande une
/// confirmation ; la policy du poste peut tout couper.
const TOOLS: &str = r#"[
  {"outil": "theme_list", "arguments": {}, "classe": "lecture", "effet": "packs, origine, variante active"},
  {"outil": "theme_info", "arguments": {"id": "…"}, "classe": "lecture", "effet": "manifeste résolu"},
  {"outil": "theme_palette", "arguments": {"format": "json|env|css|hex"}, "classe": "lecture", "effet": "palette publiée + révision"},
  {"outil": "theme_verify", "arguments": {"id": "…?"}, "classe": "lecture", "effet": "contraste, dépendances, cohérence"},
  {"outil": "theme_diff", "arguments": {"a": "…", "b": "…"}, "classe": "lecture", "effet": "slots et gabarits divergents"},
  {"outil": "theme_preview", "arguments": {"id": "…", "variante": "dark|light"}, "classe": "préparation", "effet": "mire + aperçu, rien d'appliqué"},
  {"outil": "theme_build", "arguments": {"intent": {"name": "…", "variant": "dark|light", "hue": "…", "mood": "calme|vif|contraste", "contrast": "aa|aaa", "accent": "auto|#…"}}, "classe": "préparation", "effet": "construit un pack dans builds/, sans l'installer"},
  {"outil": "theme_recolor", "arguments": {"image": "…", "scheme": "vibrant|material|pastel", "variante": "dark|light"}, "classe": "préparation", "effet": "palette depuis le fond d'écran"},
  {"outil": "theme_use", "arguments": {"id": "…", "variante": "…", "mode": "persist|hot"}, "classe": "application", "effet": "installe et pose le thème (confirmé)"},
  {"outil": "theme_apply", "arguments": {"cibles": "…?", "mode": "…"}, "classe": "application", "effet": "réapplique l'actif (confirmé)"},
  {"outil": "theme_install", "arguments": {"chemin": "…"}, "classe": "application", "effet": "promeut un pack de builds/ ou une archive"},
  {"outil": "theme_remove", "arguments": {"id": "…"}, "classe": "application", "effet": "retire un pack"}
]"#;

fn tools(flags: &Flags) -> Result<u8, Failure> {
    if flags.json {
        println!("{TOOLS}");
        return Ok(0);
    }
    for item in serde_json::from_str::<serde_json::Value>(TOOLS)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
    {
        println!(
            "{:<14} {:<12} {}",
            item["outil"].as_str().unwrap_or_default(),
            item["classe"].as_str().unwrap_or_default(),
            item["effet"].as_str().unwrap_or_default()
        );
    }
    Ok(0)
}

/// `diff <a> <b>` : divergences de slots et d'effets entre deux palettes.
fn diff(roots: &Roots, flags: &Flags, rest: &[String]) -> Result<u8, Failure> {
    let (a, b) = match rest {
        [a, b] => (a, b),
        _ => return Err((1, "usage : diff <id|chemin> <id|chemin>".to_string())),
    };
    let ra = resolved_of(roots, flags, a)?;
    let rb = resolved_of(roots, flags, b)?;
    let d = watch::diff(&ra.0, &rb.0);
    if flags.json {
        let items: Vec<serde_json::Value> = d
            .iter()
            .map(|(s, x, y)| serde_json::json!({"slot": s, "a": x, "b": y}))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&items).map_err(json_err)?
        );
    } else if d.is_empty() {
        println!("palettes identiques");
    } else {
        println!("{:<16} {:<12} b", "slot", "a");
        for (s, x, y) in &d {
            println!("{:<16} {:<12} {}", s, x, y);
        }
    }
    Ok(0)
}

/// `preview <id> --out <png>` : mire de palette.
fn preview(roots: &Roots, flags: &Flags, rest: &[String]) -> Result<u8, Failure> {
    let id = rest
        .first()
        .ok_or_else(|| (1, "usage : preview <id> --out <png>".to_string()))?;
    let out = flags
        .out
        .clone()
        .ok_or_else(|| (1, "usage : preview <id> --out <png>".to_string()))?;
    let (resolved, _) = resolved_of(roots, flags, id)?;
    watch::mire_png(&resolved, &out).map_err(engine_err)?;
    println!("mire → {}", out.display());
    Ok(0)
}

/// `watch` : réapplication déclenchée (inotify, socket, SIGHUP).
fn watch_cmd(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    // Une veille tourne déjà : application ponctuelle, puis sortie (§11).
    if watch::running() {
        eprintln!(
            "veille déjà en cours ({}) : application ponctuelle",
            watch::socket_path().display()
        );
        let report = watch::trigger_once(roots, "cli").map_err(|e| (4u8, e.to_string()))?;
        print_report(&report, flags)?;
        return Ok(if report.errors.is_empty() { 0 } else { 3 });
    }
    eprintln!(
        "veille : {} — réveil par fichiers, socket ou SIGHUP",
        watch::socket_path().display()
    );
    watch::watch(roots, |r| {
        if r.errors.is_empty() {
            eprintln!(
                "reappliqué : {} ({}) révision {}",
                r.pack, r.variant, r.revision
            );
        } else {
            eprintln!("échec de réapplication : {}", r.errors.join(" ; "));
        }
    })
    .map(|_| 0u8)
    .map_err(|e| (4u8, e.to_string()))
}

/// Garde de policy pour les commandes pilotées par LYRA (§17.2) : la classe
/// « application » exige `theme_engine_ai_apply`, les autres
/// `theme_engine_ai`.
fn policy_gate(origin: &str, application: bool) -> Result<(), Failure> {
    if origin != "lyra" {
        return Ok(());
    }
    let policy = hyprdesk::policy::get();
    let ok = if application {
        policy.ai_theme_apply_allowed()
    } else {
        policy.ai_theme_allowed()
    };
    if ok {
        Ok(())
    } else {
        Err((
            4,
            "politique du poste (/etc/hyprdesk/policy.toml) : outils de thème refusés à LYRA"
                .to_string(),
        ))
    }
}

/// `build --intent <fichier.json>` : palette par intention, pack écrit dans
/// `builds/` — ni installé ni appliqué (§17.3).
fn build(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    let origin = flags.origin.clone().unwrap_or_else(|| "cli".to_string());
    policy_gate(&origin, false)?;
    let intent_path = flags
        .intent
        .clone()
        .or_else(|| flags.from.clone())
        .ok_or_else(|| {
            (
                1u8,
                "usage : build --intent <fichier.json> [--out <dir>]".to_string(),
            )
        })?;
    let text = std::fs::read_to_string(&intent_path).map_err(io_err)?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| (2u8, format!("intent : {e}")))?;
    let i = intent::Intent::parse(&value).map_err(engine_err)?;
    let resolved = i.build();
    let pack_dir = flags.out.clone().unwrap_or_else(|| {
        roots
            .cache_dir()
            .join("builds")
            .join(intent::pack_id(&i.name))
    });
    std::fs::create_dir_all(&pack_dir).map_err(io_err)?;
    std::fs::write(pack_dir.join("theme.toml"), i.manifest()).map_err(io_err)?;
    if flags.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "pack": resolved.id,
                "variant": resolved.variant,
                "dossier": pack_dir.display().to_string(),
                "prochaine_etape": "theme_preview puis theme_use (confirmation)",
            }))
            .map_err(json_err)?
        );
    } else {
        println!("pack {} construit dans {}", resolved.id, pack_dir.display());
        for (k, c) in &resolved.palette {
            println!("  {k:<16} {}", color::fmt(*c, color::Fmt::Hex));
        }
        println!(
            "aperçu puis, après confirmation : `hyprdesk-theme-engine use {} --origin lyra`",
            resolved.id
        );
    }
    Ok(0)
}

/// `recolor --from <image>` : palette depuis le fond d'écran, même parcours
/// que `build` (pack dans builds/, jamais appliqué seul).
fn recolor_cmd(roots: &Roots, flags: &Flags) -> Result<u8, Failure> {
    let origin = flags.origin.clone().unwrap_or_else(|| "cli".to_string());
    policy_gate(&origin, false)?;
    let image = flags.from.clone().ok_or_else(|| {
        (
            1u8,
            "usage : recolor --from <image> [--scheme vibrant|material|pastel]".to_string(),
        )
    })?;
    let scheme = match flags.scheme.as_deref() {
        Some(s) => recolor::Scheme::parse(s).ok_or_else(|| {
            (
                1u8,
                "schéma inconnu (vibrant, material, pastel)".to_string(),
            )
        })?,
        None => recolor::Scheme::Vibrant,
    };
    let variant = flags.variant.clone().unwrap_or_else(|| "dark".to_string());
    let hue_lock = flags
        .hue_lock
        .as_deref()
        .map(|s| s.trim().parse::<f64>())
        .transpose()
        .map_err(|e| (1u8, format!("--hue-lock : {e}")))?;
    let resolved =
        recolor::from_image(Path::new(&image), &variant, &scheme, hue_lock).map_err(engine_err)?;
    let pack_dir = flags.out.clone().unwrap_or_else(|| {
        roots
            .cache_dir()
            .join("builds")
            .join(recolor::pack_id_for(Path::new(&image)))
    });
    std::fs::create_dir_all(&pack_dir).map_err(io_err)?;
    // Le pack image : mêmes couleurs que la palette construite, posées
    // directement (les gabarits intégrés font le reste).
    let hex = |c: &hyprdesk::config::Rgba| color::fmt(*c, color::Fmt::Hex);
    let schéma = match scheme {
        recolor::Scheme::Vibrant => "vibrant",
        recolor::Scheme::Material => "material",
        recolor::Scheme::Pastel => "pastel",
    };
    let manifest = format!(
        "[meta]\nid = \"{}\"\nname = \"LYRA — {}\"\nversion = \"0.1.0\"\ndescription = \"Palette depuis l'image {image} (schéma {schéma}, variante {variant}).\"\nlicense = \"MIT\"\n\n[palette]\nbg = \"{}\"\nfg = \"{}\"\naccent = \"{}\"\n",
        resolved.id,
        recolor::pack_id_for(Path::new(&image)).trim_start_matches("lyra-"),
        hex(&resolved.palette["bg"]),
        hex(&resolved.palette["fg"]),
        hex(&resolved.palette["accent"])
    );
    std::fs::write(pack_dir.join("theme.toml"), &manifest).map_err(io_err)?;
    if flags.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "pack": resolved.id,
                "variant": resolved.variant,
                "dossier": pack_dir.display().to_string(),
            }))
            .map_err(json_err)?
        );
    } else {
        println!("pack {} construit dans {}", resolved.id, pack_dir.display());
        for (k, c) in &resolved.palette {
            println!("  {k:<16} {}", hex(c));
        }
    }
    Ok(0)
}

/// `export <id> --out <archive>` : pack installé ou embarqué → `.hyprtheme`.
fn export(roots: &Roots, flags: &Flags, rest: &[String]) -> Result<u8, Failure> {
    let id = rest
        .first()
        .ok_or_else(|| (1u8, "usage : export <id> --out <archive>".to_string()))?;
    let out = flags.out.clone().ok_or_else(|| {
        (
            1u8,
            "usage : export <id> --out <archive.hyprtheme>".to_string(),
        )
    })?;
    let (all, _) = packs::discover(roots, &bundled::packs()).map_err(|e| (2u8, e.to_string()))?;
    let pack = packs::find(&all, id).ok_or_else(|| (2u8, format!("pack introuvable : {id}")))?;
    let n = match &pack.dir {
        Some(dir) => archive::export_pack(dir, &out).map_err(engine_err)?,
        None => {
            // Pack embarqué : le manifeste seul suffit (installable tel quel).
            let entries = vec![archive::Entry {
                path: archive::MANIFEST_AT_ROOT.to_string(),
                data: bundled::manifest_text(id)
                    .ok_or_else(|| (2u8, format!("pack embarqué introuvable : {id}")))?
                    .as_bytes()
                    .to_vec(),
            }];
            archive::write(&out, &entries).map_err(engine_err)?;
            1
        }
    };
    if flags.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": id, "archive": out.display().to_string(), "fichiers": n,
            }))
            .map_err(json_err)?
        );
    } else {
        println!("{} fichiers → {}", n, out.display());
    }
    Ok(0)
}

/// `install <archive|dossier>` : validation complète avant la première écriture.
fn install(roots: &Roots, rest: &[String], origin: Option<&str>) -> Result<u8, Failure> {
    let source = rest.first().ok_or_else(|| {
        (
            1u8,
            "usage : install <archive.hyprtheme|dossier>".to_string(),
        )
    })?;
    if let Some(o) = origin {
        policy_gate(o, true)?;
    }
    let id = archive::install(Path::new(source), roots).map_err(engine_err)?;
    let _ = hyprdesk::theme_engine::state::journal(roots, &format!("install {id} depuis {source}"));
    println!("{id} installé dans ~/.local/share/hyprdesk/themes/{id}/");
    Ok(0)
}

/// `remove <id>` : un pack utilisateur seulement.
fn remove(roots: &Roots, rest: &[String], origin: Option<&str>) -> Result<u8, Failure> {
    let id = rest
        .first()
        .ok_or_else(|| (1u8, "usage : remove <id>".to_string()))?;
    if let Some(o) = origin {
        policy_gate(o, true)?;
    }
    archive::remove(id, roots).map_err(engine_err)?;
    let _ = hyprdesk::theme_engine::state::journal(roots, &format!("remove {id}"));
    println!("{id} retiré");
    Ok(0)
}

fn print_report(report: &apply::Report, flags: &Flags) -> Result<(), Failure> {
    if flags.json {
        println!("{}", report.to_json().map_err(|e| (2u8, e.to_string()))?);
    } else {
        print!("{}", report.summary());
    }
    Ok(())
}

fn print_usage() {
    println!(
        "hyprdesk-theme-engine — moteur de thème de hyprdesk (docs/THEME-ENGINE.md)\n\n\
USAGE :\n\
  hyprdesk-theme-engine list [--json]\n\
  hyprdesk-theme-engine info <id> [--variant dark|light|auto] [--json]\n\
  hyprdesk-theme-engine use <id> [--variant …] [--mode persist|hot|dry-run] [--targets a,b]\n\
  hyprdesk-theme-engine apply [--mode …] [--targets a,b]\n\
  hyprdesk-theme-engine generate [--out <dossier>]\n\
  hyprdesk-theme-engine palette [--format json|hex|env|css]\n\
  hyprdesk-theme-engine verify [--json]\n\
  hyprdesk-theme-engine doctor [--json]\n\
  hyprdesk-theme-engine export <id> --out <archive.hyprtheme>\n\
  hyprdesk-theme-engine build --intent <fichier.json> [--out <dir>]\n\
  hyprdesk-theme-engine recolor --from <image> [--scheme …] [--variant …]\n\
  hyprdesk-theme-engine tools [--json]\n\
  hyprdesk-theme-engine diff <a> <b> [--json]\n\
  hyprdesk-theme-engine preview <id> --out <png>\n\
  hyprdesk-theme-engine watch\n\
  hyprdesk-theme-engine install <archive|dossier>\n\
  hyprdesk-theme-engine remove <id>\n\n\
ENVIRONNEMENT :\n\
  HYPRDESK_THEME=id[:variante]   surcharge le thème actif (équivalent GTK_THEME)"
    );
}

fn json_err(e: serde_json::Error) -> Failure {
    (2, e.to_string())
}

fn io_err(e: std::io::Error) -> Failure {
    (2, e.to_string())
}

fn engine_err(e: hyprdesk::theme_engine::Error) -> Failure {
    (2, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_theme_analysé() {
        // SAFETY : essais qui se suivent dans le même fil, valeurs propres.
        unsafe { std::env::set_var("HYPRDESK_THEME", "ardoise:light") };
        assert_eq!(
            env_theme(),
            Some(("ardoise".to_string(), Some("light".to_string())))
        );
        unsafe { std::env::set_var("HYPRDESK_THEME", "encre") };
        assert_eq!(env_theme(), Some(("encre".to_string(), None)));
        unsafe { std::env::set_var("HYPRDESK_THEME", "  ") };
        assert_eq!(env_theme(), None);
        unsafe { std::env::remove_var("HYPRDESK_THEME") };
        assert_eq!(env_theme(), None);
    }

    #[test]
    fn usage_sans_commande() {
        assert!(matches!(run(&[]), Err((1, _))));
        assert!(matches!(run(&["voilà".to_string()]), Err((1, _))));
    }

    #[test]
    fn option_incomplete_en_erreur() {
        assert!(matches!(
            run(&["apply".to_string(), "--mode".to_string()]),
            Err((1, _))
        ));
    }
}
