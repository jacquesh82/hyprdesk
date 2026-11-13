//! Commandes de la ligne de commande pour les fonctions récentes, que les
//! outils MCP `hyprdesk_*` du serveur hyprpanel (et LYRA, en shell)
//! appellent :
//!
//! - `hyprdesk autostart [list | add <commande> | remove <commande> | on <commande> | off <commande>]`
//! - `hyprdesk hyprland-check [--json]`
//! - `hyprdesk hyprland-fix (--safe | <ligne>… | backups)`
//! - `hyprdesk window-places [forget]`
//! - `hyprdesk icons [list | add app <id> | add path <chemin> | add exec <commande> | remove <id>]`
//! - `hyprdesk app-theme [status | apply]`
//! - `hyprdesk colors [list | set <écran> <réglage> <valeur> | reset <écran> | calibration <écran> <fichier>|none]`
//!
//! Chacune rend un texte (sortie standard) ou une erreur ; rien n'est
//! imprimé ailleurs (le serveur MCP lit la sortie).

use std::path::Path;

use crate::config::{IconDef, IconKind, Origin};

/// `Some(résultat)` si `cmd` est l'une de ces commandes.
pub fn run(cmd: &str, config_path: &Path, args: &[String]) -> Option<Result<String, String>> {
    Some(match cmd {
        "autostart" => autostart(config_path, args),
        "hyprland-check" => {
            hyprland_check(config_path, args.first().is_some_and(|a| a == "--json"))
        }
        "hyprland-fix" => hyprland_fix(config_path, args),
        "window-places" => window_places(args),
        "icons" => icons(config_path, args),
        "app-theme" => app_theme(config_path, args),
        "colors" => colors(config_path, args),
        "window" => window(args),
        "lid" => crate::config::lua::load_file(config_path)
            .and_then(|l| crate::lid::run(&l.config.general.idle, args)),
        _ => return None,
    })
}

fn joined(args: &[String]) -> Result<String, String> {
    let s = args.join(" ").trim().to_string();
    if s.is_empty() {
        Err(crate::i18n::tr("commande attendue").to_string())
    } else {
        Ok(s)
    }
}

// -- Démarrage automatique -------------------------------------------------------

fn autostart(config_path: &Path, args: &[String]) -> Result<String, String> {
    use crate::session_start as ss;
    let (ours, theirs) = ss::read(config_path);
    let list = |items: &[ss::Item]| -> Vec<(bool, String)> {
        items
            .iter()
            .map(|i| (i.enabled, i.command.clone()))
            .collect()
    };
    match args.first().map(String::as_str).unwrap_or("list") {
        "list" | "ls" => {
            let state = |i: &ss::Item| {
                if i.locked {
                    crate::i18n::tr("toujours actif")
                } else if i.enabled {
                    crate::i18n::tr("actif")
                } else {
                    crate::i18n::tr("coupé")
                }
            };
            let mut s = crate::i18n::tr("Lancés par hyprdesk (autostart.lua) :").to_string() + "\n";
            if ours.is_empty() {
                s += "  —\n";
            }
            for i in &ours {
                s += &format!("  [{}] {}\n", state(i), i.command);
            }
            s += &(crate::i18n::tr("Dans hyprland.lua (hl.on(\"hyprland.start\")) :").to_string()
                + "\n");
            for i in &theirs {
                s += &format!("  {} · [{}] {}\n", i.line + 1, state(i), i.command);
            }
            s += &(crate::i18n::tr("Applications XDG (~/.config/autostart) :").to_string() + "\n");
            for e in crate::autostart::entries() {
                s += &format!("  {} — {:?} — {}\n", e.name, e.status, e.exec);
            }
            Ok(s)
        }
        "add" => {
            let cmd = joined(&args[1..])?;
            let mut l = list(&ours);
            if !l.iter().any(|(_, c)| c == &cmd) {
                l.push((true, cmd.clone()));
            }
            ss::save_ours(config_path, &l)?;
            Ok(crate::tr!("ajouté au démarrage : {c}", c = cmd))
        }
        verb @ ("remove" | "on" | "off") => {
            let cmd = joined(&args[1..])?;
            if ours.iter().any(|i| i.command == cmd) {
                let mut l = list(&ours);
                match verb {
                    "remove" => l.retain(|(_, c)| c != &cmd),
                    v => l
                        .iter_mut()
                        .filter(|(_, c)| c == &cmd)
                        .for_each(|e| e.0 = v == "on"),
                }
                ss::save_ours(config_path, &l)?;
            } else if let Some(it) = theirs.iter().find(|i| i.command == cmd) {
                if it.locked {
                    return Err(
                        crate::i18n::tr("ligne qui prépare la session : laissée en place")
                            .to_string(),
                    );
                }
                match verb {
                    "remove" => ss::remove_hyprland(config_path, it)?,
                    v => ss::set_hyprland(config_path, it, v == "on")?,
                }
            } else {
                return Err(crate::tr!("introuvable au démarrage : {c}", c = cmd));
            }
            Ok(crate::tr!("{v} : {c}", v = verb, c = cmd))
        }
        other => Err(crate::tr!(
            "autostart {o} : list | add <commande> | remove <commande> | on <commande> | off <commande>",
            o = other
        )),
    }
}

// -- Ménage de hyprland.lua --------------------------------------------------------

fn findings(config_path: &Path) -> Result<Vec<crate::hyprcheck::Finding>, String> {
    let cfg = crate::config::lua::load_file(config_path).map(|l| l.config)?;
    let managed = crate::hyprcheck::managed(
        cfg.plugin_on("wallpaper"),
        cfg.plugin_on("bar") && cfg.bar.is_some(),
        cfg.plugin_on("notifications"),
        cfg.general.idle.manage,
        cfg.general.clipboard_history,
    );
    let monitors = hyprpanel_core::ipc::query_json("monitors")
        .ok()
        .and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|m| m["name"].as_str().map(str::to_string))
                    .collect()
            })
        })
        .unwrap_or_default();
    let session = crate::hyprcheck::Session {
        monitors,
        workspaces: cfg.plugin_on("workspaces"),
    };
    Ok(crate::hyprcheck::full(config_path, &managed, &session))
}

fn hyprland_check(config_path: &Path, json: bool) -> Result<String, String> {
    use crate::hyprcheck::Fix;
    let f = findings(config_path)?;
    if json {
        let v: Vec<serde_json::Value> = f
            .iter()
            .map(|f| {
                serde_json::json!({
                    "kind": crate::i18n::tr(f.kind.label()),
                    "line": f.line,
                    "excerpt": f.excerpt,
                    "detail": f.detail,
                    "fix": f.fix.as_ref().map(|x| match x {
                        Fix::Comment(..) => "comment",
                        Fix::Delete(..) => "delete-line",
                        Fix::DeleteFiles(..) => "delete-backups",
                    }),
                    "safe": f.safe,
                })
            })
            .collect();
        return serde_json::to_string_pretty(&v).map_err(|e| e.to_string());
    }
    if f.is_empty() {
        return Ok(crate::i18n::tr("Rien à signaler : hyprland.lua est en ordre.").to_string());
    }
    let mut s = String::new();
    for x in &f {
        let line = x.line.map(|l| format!("{l} · ")).unwrap_or_default();
        s += &format!("[{}] {line}{}", crate::i18n::tr(x.kind.label()), x.excerpt);
        if !x.detail.is_empty() {
            s += &format!(" — {}", x.detail);
        }
        if x.fix.is_some() {
            s += if x.safe {
                " (corrigeable, sûr)"
            } else {
                " (corrigeable)"
            };
        }
        s.push('\n');
    }
    Ok(s)
}

fn hyprland_fix(config_path: &Path, args: &[String]) -> Result<String, String> {
    use crate::hyprcheck::Fix;
    let f = findings(config_path)?;
    let safe = args.iter().any(|a| a == "--safe");
    let backups = args.iter().any(|a| a == "backups");
    let lines: Vec<usize> = args.iter().filter_map(|a| a.parse().ok()).collect();
    if !safe && !backups && lines.is_empty() {
        return Err(crate::i18n::tr("hyprland-fix : --safe | <ligne>… | backups").to_string());
    }
    let fixes: Vec<Fix> = f
        .iter()
        .filter(|x| {
            (safe && x.safe)
                || x.line.is_some_and(|l| lines.contains(&l))
                || (backups && matches!(x.fix, Some(Fix::DeleteFiles(_))))
        })
        .filter_map(|x| x.fix.clone())
        .collect();
    let n = crate::hyprcheck::apply(config_path, &fixes)?;
    Ok(crate::tr!(
        "{n} correction(s) faite(s) ; copie : hyprland.lua.avant-hyprdesk-menage.bak",
        n = n
    ))
}

// -- Places des fenêtres -----------------------------------------------------------

fn window_places(args: &[String]) -> Result<String, String> {
    if args.first().is_some_and(|a| a == "forget") {
        // Le démon oublie aussi celles de la session ; sans démon : le fichier.
        return match crate::control::send(crate::control::Command::ForgetWindowPlaces) {
            Ok(r) => Ok(r),
            Err(_) => {
                let p = crate::windows::places_path();
                let _ = std::fs::remove_file(&p);
                Ok(crate::i18n::tr("places des fenêtres oubliées").to_string())
            }
        };
    }
    let places = crate::windows::load_places();
    let mut keys: Vec<_> = places.keys().collect();
    keys.sort();
    let mut s = crate::tr!(
        "{n} place(s) de fenêtre choisie(s) (workspace|application) :",
        n = places.len()
    ) + "\n";
    for k in keys {
        let r = places[k];
        s += &format!("  {k} → {} {} {}×{}\n", r.0, r.1, r.2, r.3);
    }
    Ok(s)
}

// -- Icônes du bureau --------------------------------------------------------------

fn icons(config_path: &Path, args: &[String]) -> Result<String, String> {
    let cfg = crate::config::lua::load_settings_only(config_path)?;
    let mut s = crate::settings::Settings::from_config(&cfg);
    match args.first().map(String::as_str).unwrap_or("list") {
        "list" | "ls" => {
            let full = crate::config::lua::load_file(config_path).map(|l| l.config)?;
            let mut out = crate::tr!(
                "Portée : {s} ; double clic : {d}",
                s = full.general.icon_scope.as_str(),
                d = full.general.icon_double_click
            ) + "\n";
            for i in &full.icons {
                let origin = match i.origin {
                    Origin::Settings => "réglages",
                    Origin::User => "hyprdesk.lua",
                };
                let sel = match (&i.sel.monitors, &i.sel.workspaces) {
                    (None, None) => "partout".to_string(),
                    (m, w) => format!("écran {:?} workspace {:?}", m, w),
                };
                out += &format!(
                    "  {} — {} — {sel} ({origin})\n",
                    i.id,
                    i.label.clone().unwrap_or_default()
                );
            }
            Ok(out)
        }
        "add" => {
            let kind = args.get(1).map(String::as_str).unwrap_or("");
            let value = joined(args.get(2..).unwrap_or(&[]))?;
            // Portée des icônes : l'écran ou le workspace qui a le focus.
            let full = crate::config::lua::load_file(config_path).map(|l| l.config)?;
            let (mon, ws) = focused();
            let sel = full.general.icon_scope.selector(&mon, &ws);
            let id = match kind {
                "app" => {
                    s.add_app(&value, sel);
                    format!("app:{value}")
                }
                "path" => s
                    .add_dropped(Path::new(&crate::apptheme::expand_home(&value)), sel)
                    .ok_or_else(|| crate::i18n::tr("déjà sur le bureau").to_string())?,
                "exec" => {
                    let id = format!("exec:{value}");
                    if !s.icons.iter().any(|i| i.id == id) {
                        s.icons.push(IconDef {
                            origin: Origin::Settings,
                            sel,
                            id: id.clone(),
                            label: Some(value.clone()),
                            kind: IconKind::Exec {
                                cmd: value.clone(),
                                terminal: false,
                            },
                            icon: None,
                            pos: None,
                        });
                    }
                    id
                }
                _ => {
                    return Err(crate::i18n::tr(
                        "icons add app <id> | path <chemin> | exec <commande>",
                    )
                    .to_string());
                }
            };
            s.save(config_path).map_err(|e| e.to_string())?;
            Ok(crate::tr!("icône ajoutée : {id}", id = id))
        }
        "remove" | "rm" => {
            let id = joined(&args[1..])?;
            let before = s.icons.len();
            s.icons.retain(|i| i.id != id);
            if s.icons.len() == before {
                return Err(crate::tr!(
                    "{id} : pas parmi les icônes des Réglages (celles de hyprdesk.lua s'y retirent à la main)",
                    id = id
                ));
            }
            s.save(config_path).map_err(|e| e.to_string())?;
            Ok(crate::tr!("icône retirée : {id}", id = id))
        }
        other => Err(crate::tr!(
            "icons {o} : list | add app|path|exec <valeur> | remove <id>",
            o = other
        )),
    }
}

/// Écran et workspace qui ont le focus (vides si Hyprland ne répond pas).
fn focused() -> (String, String) {
    hyprpanel_core::ipc::query_json("monitors")
        .ok()
        .and_then(|v| {
            v.as_array()?
                .iter()
                .find(|m| m["focused"].as_bool() == Some(true))
                .map(|m| {
                    (
                        m["name"].as_str().unwrap_or_default().to_string(),
                        m["activeWorkspace"]["name"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                    )
                })
        })
        .unwrap_or_default()
}

// -- Thème des applications --------------------------------------------------------

fn app_theme(config_path: &Path, args: &[String]) -> Result<String, String> {
    let cfg = crate::config::lua::load_file(config_path).map(|l| l.config)?;
    let enabled = if cfg.plugin_on(crate::apptheme::PLUGIN) {
        cfg.general.app_themes.clone()
    } else {
        Default::default()
    };
    let opts = crate::apptheme::options(&cfg.general);
    match args.first().map(String::as_str).unwrap_or("status") {
        "status" => {
            let mut s = String::new();
            for t in crate::apptheme::TARGETS {
                s += &format!(
                    "{} — {} — {} — {}\n",
                    t.id,
                    if (t.installed)() {
                        "installé"
                    } else {
                        "absent"
                    },
                    if enabled.contains(t.id) {
                        "thème appliqué"
                    } else {
                        "thème coupé"
                    },
                    (t.file)(&opts).display()
                );
            }
            Ok(s)
        }
        "apply" => {
            let report = crate::apptheme::sync(&enabled, &cfg.general.theme, &opts);
            if report.is_empty() {
                return Ok(crate::i18n::tr(
                    "aucune application cochée (hyprdesk config set app_themes …)",
                )
                .to_string());
            }
            let mut s = String::new();
            let mut failed = false;
            for (id, r) in report {
                match r {
                    Ok(m) => s += &format!("{m}\n"),
                    Err(e) => {
                        failed = true;
                        s += &format!("{id} : {e}\n");
                    }
                }
            }
            if failed { Err(s) } else { Ok(s) }
        }
        other => Err(crate::tr!("app-theme {o} : status | apply", o = other)),
    }
}

// -- Couleurs des écrans -----------------------------------------------------------

fn colors(config_path: &Path, args: &[String]) -> Result<String, String> {
    let cfg = crate::config::lua::load_settings_only(config_path)?;
    let mut s = crate::settings::Settings::from_config(&cfg);
    let full = crate::config::lua::load_file(config_path).map(|l| l.config)?;
    let mut map = s
        .screen_colors
        .clone()
        .unwrap_or_else(|| full.general.screen_colors.clone());
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    let mut save =
        |map: std::collections::BTreeMap<String, crate::colors::Adjust>| -> Result<(), String> {
            s.screen_colors = Some(map.into_iter().filter(|(_, a)| !a.is_default()).collect());
            s.save(config_path).map(|_| ()).map_err(|e| e.to_string())
        };
    match args.first().map(String::as_str).unwrap_or("list") {
        "list" | "ls" => {
            let names: Vec<String> = hyprpanel_core::ipc::query_json("monitors")
                .ok()
                .and_then(|v| {
                    v.as_array().map(|a| {
                        a.iter()
                            .filter_map(|m| m["name"].as_str().map(str::to_string))
                            .collect()
                    })
                })
                .unwrap_or_default();
            let mut all: Vec<String> = names;
            for k in map.keys() {
                if !all.contains(k) {
                    all.push(k.clone());
                }
            }
            let mut out = String::new();
            for m in all {
                let a = map.get(&m).cloned().unwrap_or_default();
                let v: Vec<String> = crate::colors::FIELDS
                    .iter()
                    .map(|(k, _, _, _, _)| format!("{k}={:.2}", a.get(k).unwrap_or(1.0)))
                    .collect();
                let cal = if a.calibration.is_empty() {
                    "—".to_string()
                } else {
                    a.calibration.clone()
                };
                out += &format!("{m} : {} ; calibration={cal}\n", v.join(" "));
            }
            if !full.plugin_on("colors") {
                out += crate::i18n::tr(
                    "(paquet « Couleurs des écrans » coupé : réglages non appliqués)",
                );
                out.push('\n');
            }
            Ok(out)
        }
        "set" => {
            let (m, k) = (arg(1), arg(2));
            let v: f32 = arg(3).replace(',', ".").parse().map_err(|_| {
                crate::i18n::tr("colors set <écran> <réglage> <nombre>").to_string()
            })?;
            if m.is_empty() || !map.entry(m.to_string()).or_default().set(k, v) {
                return Err(crate::tr!(
                    "réglage inconnu « {k} » (brightness, contrast, gamma, red, green, blue, gamma_red, gamma_green, gamma_blue)",
                    k = k
                ));
            }
            let now = map[m].get(k).unwrap_or(v);
            save(map)?;
            Ok(format!("{m} : {k} = {now:.2}"))
        }
        "reset" => {
            let m = arg(1);
            if m.is_empty() {
                return Err(crate::i18n::tr("colors reset <écran>").to_string());
            }
            map.remove(m);
            save(map)?;
            Ok(crate::tr!("{m} : couleurs d'origine", m = m))
        }
        "calibration" => {
            let (m, f) = (
                arg(1),
                args.get(2..).map(|x| x.join(" ")).unwrap_or_default(),
            );
            if m.is_empty() || f.trim().is_empty() {
                return Err(crate::i18n::tr(
                    "colors calibration <écran> <fichier .icc|.icm|.cal> | none",
                )
                .to_string());
            }
            let kept = if f.trim() == "none" {
                String::new()
            } else {
                crate::colors::import(config_path, &f)?
            };
            map.entry(m.to_string()).or_default().calibration = kept.clone();
            save(map)?;
            Ok(if kept.is_empty() {
                crate::tr!("{m} : calibration retirée", m = m)
            } else {
                crate::tr!("{m} : calibration {f}", m = m, f = kept)
            })
        }
        "diag" | "doctor" => {
            crate::colors::set_icc_map(&map);
            let names: Vec<String> = hyprpanel_core::ipc::query_json("monitors")
                .ok()
                .and_then(|v| {
                    v.as_array().map(|a| {
                        a.iter()
                            .filter_map(|m| m["name"].as_str().map(str::to_string))
                            .collect()
                    })
                })
                .unwrap_or_default();
            let holders = crate::colors::gamma_holders(true);
            let daemon = holders.iter().any(|h| h.starts_with("hyprdesk"));
            let others = holders.iter().any(|h| !h.starts_with("hyprdesk"));
            // Jamais de sonde sur un écran tenu : Hyprland le verrouillerait.
            let held =
                |m: &String| others || (daemon && map.get(m).is_some_and(|a| a.needs_table()));
            let free: Vec<String> = names.iter().filter(|m| !held(m)).cloned().collect();
            let probed = crate::colors::probe(&free)?;
            let mut out = String::new();
            for name in &names {
                let r = probed.iter().find(|(n, _)| n == name).map(|(_, r)| *r);
                out += &match r {
                    Some(Ok(n)) => crate::tr!(
                        "{m} : table gamma disponible ({n} entrées)",
                        m = name,
                        n = n
                    ),
                    Some(Err(())) => crate::tr!(
                        "{m} : refusée — {e}",
                        m = name,
                        e = crate::colors::refusal_reason(name, false, false)
                    ),
                    None if others => crate::tr!(
                        "{m} : non sondé — {e}",
                        m = name,
                        e = crate::colors::refusal_reason(name, false, false)
                    ),
                    None => crate::tr!(
                        "{m} : table tenue par hyprdesk (réglages actifs) — normal",
                        m = name
                    ),
                };
                out.push('\n');
                if let Some(icc) = crate::colors::icc_in_use(name) {
                    out += &crate::tr!("  profil ICC : {p}", p = icc);
                    out.push('\n');
                }
            }
            out += &crate::tr!(
                "programmes de tables gamma en cours : {p}",
                p = if holders.is_empty() {
                    crate::i18n::tr("aucun").to_string()
                } else {
                    holders.join(", ")
                }
            );
            out.push('\n');
            Ok(out)
        }
        // Killswitch : arrêter les programmes qui tiennent les tables gamma
        // ou changent la matrice de couleur ; `--disable` : ne plus les lancer.
        "free" | "stop" => {
            let list = crate::colors::blockers();
            if list.is_empty() {
                return Ok(
                    crate::i18n::tr("aucun programme ne gêne les réglages de couleur").to_string(),
                );
            }
            crate::colors::stop_blockers(&list, args.iter().any(|a| a == "--disable"))
        }
        other => Err(crate::tr!(
            "colors {o} : list | set <écran> <réglage> <valeur> | reset <écran> | calibration <écran> <fichier>|none | diag | free [--disable]",
            o = other
        )),
    }
}

/// `hyprdesk window minimize|maximize|close` : fenêtre active (boutons de
/// Hyprbars, raccourcis).
fn window(args: &[String]) -> Result<String, String> {
    use hyprpanel_core::ipc;
    let dispatch = |lua: &str| -> Result<String, String> {
        let r = ipc::dispatch(lua).map_err(|e| e.to_string())?;
        if r.trim() == "ok" {
            Ok(String::new())
        } else {
            Err(r.trim().to_string())
        }
    };
    match args.first().map(String::as_str).unwrap_or("") {
        "minimize" => crate::hypr::minimize_active()
            .map(|a| crate::tr!("fenêtre {a} réduite (le dock la rappelle)", a = a)),
        "maximize" => {
            dispatch(r#"hl.dsp.window.fullscreen({ mode = "maximized", action = "toggle" })"#)
        }
        "close" => dispatch("hl.dsp.window.close()"),
        other => Err(crate::tr!(
            "window {o} : minimize | maximize | close",
            o = other
        )),
    }
}
