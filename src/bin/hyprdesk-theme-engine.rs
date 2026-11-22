//! `hyprdesk-theme-engine` : binaire dédié du moteur de thème
//! (docs/THEME-ENGINE.md §12). La même bibliothèque que le démon, en ligne de
//! commande : il ne lie ni Lua, ni iced, ni l'index.
//!
//! Commandes : `list`, `info`, `use`, `apply`, `generate`, `palette`,
//! `verify`, `doctor`. Toutes acceptent `--json` ; les modes d'application
//! sont `persist`, `hot` et `dry-run`. La variable `HYPRDESK_THEME=id[:variante]`
//! surcharge le thème actif (équivalent de `GTK_THEME`).

use std::path::PathBuf;
use std::process::ExitCode;

use hyprdesk::theme_engine::{
    apply::{self, Mode, Options},
    bundled, color, packs,
    paths::Roots,
    resolve, state,
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
    let (resolved, dir) = resolved_of(roots, flags, &id)?;
    // Pack embarqué : installé dans le dossier utilisateur à la sélection.
    let dir = match dir {
        Some(d) => Some(d),
        None => install_bundled(roots, &id).map(Some)?,
    };
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

/// Installe le manifeste d'un pack embarqué dans le dossier utilisateur.
fn install_bundled(roots: &Roots, id: &str) -> Result<PathBuf, Failure> {
    let Some(text) = bundled::manifest_text(id) else {
        return Err((2, format!("pack embarqué introuvable : {id}")));
    };
    let dir = roots.themes_dir().join(id);
    let path = dir.join("theme.toml");
    if !path.is_file() {
        std::fs::create_dir_all(&dir).map_err(io_err)?;
        std::fs::write(&path, text).map_err(io_err)?;
    }
    Ok(dir)
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
    for target in hyprdesk::theme_engine::targets::TARGETS {
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
        ("thunar", "cible thunar"),
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
  hyprdesk-theme-engine doctor [--json]\n\n\
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
