//! hyprdesk — bureau à la demande pour Hyprland.
//!
//! Le même binaire est le démon (`hyprdesk`, `hyprdesk daemon`) et son client
//! (`hyprdesk toggle`, à lier à une touche dans `hyprland.lua`).

mod config;
mod control;
mod hypr;
mod launch;
mod layout;
mod ui;
mod widgets;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
hyprdesk — bureau à la demande pour Hyprland

Usage : hyprdesk [--config <fichier>] [commande]

Commandes :
  daemon            lance le démon (par défaut)
  toggle            affiche ou masque le bureau
  show | hide       affiche / masque le bureau
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

-- Démarrage (inutile si le service systemd utilisateur est activé),
-- à mettre dans le bloc hl.on("hyprland.start", …) ; hyprdesk remplace hyprpaper.
hl.exec_cmd("hyprdesk")

-- Afficher / masquer le bureau
hl.bind(mainMod .. " + D", hl.dsp.exec_cmd("hyprdesk toggle"))

-- Mode « glass » : flou des fenêtres derrière le bureau (xray = false pour
-- flouter les fenêtres et non le seul fond d'écran)
hl.layer_rule({
    name         = "hyprdesk-blur",
    match        = { namespace = "^hyprdesk$" },
    blur         = true,
    ignore_alpha = 0.05,
    xray         = false,
})
"#;

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("HYPRDESK_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,wgpu_core=warn,wgpu_hal=warn,naga=warn,iced_wgpu=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
}

fn daemon(path: PathBuf) -> ExitCode {
    init_tracing();
    if control::daemon_running() {
        eprintln!("hyprdesk : un démon tourne déjà (hyprdesk status)");
        return ExitCode::FAILURE;
    }
    match config::ensure_exists(&path) {
        Ok(true) => {
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
    let cmd = args.first().map(String::as_str).unwrap_or("daemon");
    match cmd {
        "daemon" => daemon(config_path),
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
        other => match control::Command::parse(other) {
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
