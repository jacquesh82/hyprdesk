//! hyprdesk — bureau à la demande pour Hyprland.
//!
//! Le même binaire est le démon (`hyprdesk`, `hyprdesk daemon`) et son client
//! (`hyprdesk toggle`, à lier à une touche dans `hyprland.lua`).

mod config;
mod control;
mod hypr;
mod launch;
mod session;
mod settings;
mod single;
mod layout;
mod ui;
mod widgets;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
hyprdesk — bureau à la demande pour Hyprland

Usage : hyprdesk [--config <fichier>] [commande]

Commandes :
  daemon            lance le démon (par défaut) ; une seule instance par session
  daemon --replace  remplace proprement l'instance en place
  toggle            affiche ou masque le bureau
  show | hide       affiche / masque le bureau
  settings [onglet] affiche le bureau et ouvre l'écran Réglages
                    (onglet : bureau, icones, fond, widgets, effets)
  reload            relit la configuration
  status            état du démon
  quit              arrête le démon
  check [fichier]   vérifie une configuration sans la charger
  init              écrit la configuration d'exemple si aucune n'existe
  stubs [dossier]   écrit les définitions LuaLS (hyprdesk.meta.lua) pour l'autocomplétion
  snippet           lignes à ajouter à hyprland.lua (raccourci, flou, démarrage)

Configuration : $HYPRDESK_CONFIG, sinon ~/.config/hypr/hyprdesk.lua";

const STUBS: &str = include_str!("../assets/hyprdesk.meta.lua");

const SNIPPET: &str = r#"-- hyprdesk : à ajouter dans ~/.config/hypr/hyprland.lua

-- Démarrage : SEULEMENT si le service systemd utilisateur n'est pas activé
-- (une seule instance tourne de toute façon : la seconde s'arrête aussitôt),
-- dans le bloc hl.on("hyprland.start", …) ; hyprdesk remplace hyprpaper.
hl.exec_cmd("hyprdesk")

-- Les raccourcis de hyprdesk (SUPER + D par défaut) sont installés par
-- hyprdesk lui-même : voir hd.config({ keys = … }) ou Réglages → Bureau.
--
-- Bureau affiché, les raccourcis de Hyprland sont suspendus (submap
-- « hyprdesk ») : ils n'agissent plus sur les fenêtres cachées. Pour en
-- garder un actif sur le bureau, marquez-le submap_universal, par exemple
-- le changement de workspace (le bureau suit alors le workspace) :
hl.bind(mainMod .. " + ampersand", hl.dsp.focus({ workspace = 1 }), { submap_universal = true })
hl.bind(mainMod .. " + Right",     hl.dsp.focus({ workspace = "+1" }), { submap_universal = true })

-- Barre de hyprdesk (hd.bar) : flou derrière elle, comme une barre classique.
hl.layer_rule({ name = "hyprdesk-bar-blur", match = { namespace = "^hyprdesk-bar$" }, blur = true, ignore_alpha = 0.2 })
"#;

/// Plugins de widgets d'exemple, déposés s'ils n'existent pas encore.
const EXAMPLE_WIDGETS: &[(&str, &str)] =
    &[("meteo.lua", include_str!("../assets/widgets/meteo.lua")), ("notes.lua", include_str!("../assets/widgets/notes.lua"))];

fn install_example_widgets(config_path: &std::path::Path) {
    let dir = widgets::plugins::dirs(config_path.parent().unwrap_or(std::path::Path::new(".")))[0].clone();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    for (name, src) in EXAMPLE_WIDGETS {
        let p = dir.join(name);
        if !p.exists() && std::fs::write(&p, src).is_ok() {
            println!("écrit : {}", p.display());
        }
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("HYPRDESK_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,wgpu_core=warn,wgpu_hal=warn,naga=warn,iced_wgpu=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
}

fn daemon(path: PathBuf, replace: bool) -> ExitCode {
    init_tracing();
    let lock = single::lock_path();
    let _guard = match single::acquire(&lock) {
        Ok(g) => g,
        Err(pid) if replace => {
            // Relève propre : l'instance en place rend les fenêtres et ses
            // raccourcis en quittant, puis libère le verrou.
            tracing::info!("remplacement de l'instance en place{}", pid.map(|p| format!(" (pid {p})")).unwrap_or_default());
            if let Err(e) = control::send(control::Command::Quit) {
                tracing::warn!("arrêt de l'instance en place : {e}");
            }
            match single::acquire_within(&lock, std::time::Duration::from_secs(5)) {
                Ok(g) => g,
                Err(pid) => {
                    eprintln!(
                        "hyprdesk : l'instance en place{} ne s'arrête pas ; arrêtez-la (hyprdesk quit, ou systemctl --user stop hyprdesk)",
                        pid.map(|p| format!(" (pid {p})")).unwrap_or_default()
                    );
                    return ExitCode::FAILURE;
                }
            }
        }
        Err(pid) => {
            // Sortie SANS erreur : systemd (Restart=on-failure) ne doit pas
            // relancer en boucle un démon qui fait double emploi.
            eprintln!(
                "hyprdesk : déjà lancé{} — une seule instance par session (hyprdesk daemon --replace pour la remplacer)",
                pid.map(|p| format!(" (pid {p})")).unwrap_or_default()
            );
            return ExitCode::SUCCESS;
        }
    };
    // Arrêt brutal précédent, bureau affiché : on rend les raccourcis.
    if hypr::current_submap() == hypr::SUBMAP {
        tracing::info!("submap « {} » laissée active par un arrêt précédent : remise à zéro", hypr::SUBMAP);
        hypr::set_submap(None);
    }
    // Fenêtres laissées de côté par un arrêt brutal, bureau affiché : on les rend.
    hypr::restore_leftover_stash();
    // Le dossier des modules et des plugins doit exister dès le démarrage :
    // c'est lui que la surveillance suit pour détecter les nouveaux widgets.
    let widget_dir = widgets::plugins::dirs(path.parent().unwrap_or(std::path::Path::new(".")))[0].clone();
    if let Err(e) = std::fs::create_dir_all(&widget_dir) {
        tracing::warn!("création de {} : {e}", widget_dir.display());
    }
    match config::ensure_exists(&path) {
        Ok(true) => {
            install_example_widgets(&path);
            tracing::info!("configuration d'exemple écrite dans {}", path.display());
            hypr::notify(1, 6000, &format!("hyprdesk : configuration d'exemple créée — {}", path.display()));
        }
        Ok(false) => {}
        Err(e) => tracing::warn!("création de {} : {e}", path.display()),
    }
    tracing::info!("hyprdesk {} — config {}", env!("CARGO_PKG_VERSION"), path.display());
    match ui::run(path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_path = config::default_path();
    if let Some(i) = args.iter().position(|a| a == "--config" || a == "-c") {
        let Some(p) = args.get(i + 1).cloned() else {
            eprintln!("--config attend un fichier");
            return ExitCode::from(2);
        };
        config_path = PathBuf::from(p);
        args.drain(i..=i + 1);
    }
    let replace = if let Some(i) = args.iter().position(|a| a == "--replace") {
        args.remove(i);
        true
    } else {
        false
    };
    let cmd = args.first().map(String::as_str).unwrap_or("daemon");
    match cmd {
        "daemon" => daemon(config_path, replace),
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        "-V" | "--version" | "version" => {
            println!("hyprdesk {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "check" => {
            let p = args.get(1).map(PathBuf::from).unwrap_or(config_path);
            match ui::check(&p) {
                Ok(c) => {
                    println!(
                        "{} : ok — {} fonds, {} bureaux, {} icônes, {} widgets, {} hooks",
                        p.display(),
                        c.wallpapers.len(),
                        c.desktops.len(),
                        c.icons.len(),
                        c.widgets.len(),
                        c.hooks.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        "init" => match config::ensure_exists(&config_path) {
            Ok(true) => {
                println!("écrit : {}", config_path.display());
                install_example_widgets(&config_path);
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("existe déjà : {}", config_path.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}: {e}", config_path.display());
                ExitCode::FAILURE
            }
        },
        "stubs" => {
            let dir = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| config_path.parent().map(PathBuf::from).unwrap_or_default());
            let p = dir.join("hyprdesk.meta.lua");
            match std::fs::write(&p, STUBS) {
                Ok(()) => {
                    println!("écrit : {}", p.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}: {e}", p.display());
                    ExitCode::FAILURE
                }
            }
        }
        "snippet" => {
            print!("{SNIPPET}");
            ExitCode::SUCCESS
        }
        other => match control::Command::parse(&args.join(" ")) {
            Some(c) => match control::send(c) {
                Ok(r) => {
                    if !r.is_empty() && r != "ok" {
                        println!("{r}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("hyprdesk : {e}");
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!("commande inconnue : {other}\n\n{USAGE}");
                ExitCode::from(2)
            }
        },
    }
}
