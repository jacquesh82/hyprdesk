//! hyprdesk — bureau à la demande pour Hyprland.
//!
//! Le même binaire est le démon (`hyprdesk`, `hyprdesk daemon`) et son client
//! (`hyprdesk toggle`, à lier à une touche dans `hyprland.lua`).

mod apps;
mod apptheme;
mod assistant;
mod autostart;
mod bar_offer;
mod brightness;
mod capture;
mod chat;
mod cli_config;
mod clipboard;
mod config;
mod control;
mod desktop_files;
mod doctor;
mod fonts;
mod fx;
mod greeter;
mod greeter_ui;
mod hypr;
mod i18n;
mod idle;
mod launch;
mod layout;
mod lock;
mod notifications;
mod osd;
mod polkit;
mod processes;
mod screens;
mod search;
mod session;
mod settings;
mod share;
mod single;
mod span;
mod storage;
mod switcher;
mod theme;
mod tiling;
mod tray;
mod ui;
mod updates;
mod widgets;
mod windows;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
hyprdesk — bureau à la demande pour Hyprland

Usage : hyprdesk [--config <fichier>] [commande]

Commandes :
  daemon            lance le démon (par défaut) ; une seule instance par session
  daemon --replace  remplace proprement l'instance en place
  toggle            full-desktop, ou no-desktop s'il y est déjà
  toggle-launcher   launcher-only, ou no-desktop s'il y est déjà
  show | hide       full-desktop (fenêtres mises de côté) / no-desktop
  launcher-only     fenêtres en place, lanceur (dock) accessible en bas
  cycle             mode suivant : no-desktop → launcher-only → full-desktop
  settings [paquet[/onglet]]
                    ouvre l'écran Réglages (ex. bureau/raccourcis), sans changer le mode
  assistant         ouvre ou referme l'assistant (tiroir LYRA), sans changer le mode
  search            ouvre ou referme le lanceur avec recherche (toutes les applications)
  apps              ouvre ou referme la grille des applications (plein écran, catégories)
  switch [prev]     sélecteur de fenêtres (Alt+Tab, keys.switch) : ouvre, ou passe
                    à la fenêtre suivante (prev : précédente)
  tour              ouvre les réglages sur la visite guidée
                    (paquet : desktop, icons, launcher, wallpaper, widgets, bar,
                    screens, workspaces, effects, theme…)
  export [fichier] [--theme] [--screens]
                    archive du setup à partager (configs, plugins, fonds d'écran ;
                    --theme : le thème seul — couleurs, fonds, effets ;
                    --screens : monitors.lua aussi)
  import <archive> [--theme] [--dry-run] [--screens]
                    applique un setup partagé (sauvegarde l'actuel d'abord) ;
                    --theme : n'en prendre que le thème (une archive de thème
                    ne change jamais que le thème)
  theme [list]      « Mes thèmes »
  theme new <image|pack.zip|dossier> [--name NOM]
                    crée un thème depuis une scène : couleurs tirées de l'image,
                    scène étalée sur tous les écrans (bureau et verrouillage)
  theme apply <nom|archive>
                    applique un thème (le reste de la configuration ne change pas)
  wallpaper split <image> [dossier] [--scale N]
                    découpe une scène selon la disposition des écrans : une
                    image par écran (<écran>.png) ; --scale 2 pour du HiDPI
  context           décrit l'environnement hyprdesk (Markdown), pour un assistant IA
  lock              verrouille la session (écran de verrouillage de hyprdesk)
  open <rôle>       lance l'application par défaut : browser, terminal, files, editor,
                    image, video, audio, pdf, mail, ai
  config            réglages en ligne de commande, par paquet :
                    config [list [paquet]] | get <clé> | set <clé> <valeur> | unset <clé>
  menu <module>     ouvre un menu de la barre (volume, clock, tray, reseau, session…)
  osd <volume|mic|brightness|kbd> <+N|-N|toggle>
                    règle le son ou la luminosité (touches XF86, keys.media)
  media <play-pause|next|previous|status|seek ±N>
                    commande le lecteur en cours (MPRIS) ; status : son état
                    (clé=valeur) ; seek : avance ou recule de N secondes
  capture <zone|fenetre|ecran>
                    capture d'écran (Images/Captures, copiée ; touches Impr)
  capture enregistrer [zone] | capture arreter
                    enregistre l'écran actif ou une zone (Vidéos/Enregistrements),
                    ou arrête l'enregistrement en cours
  greeter [--tty | --demo]
                    écran de connexion pour greetd (connexion, choix de l'environnement) ;
                    --tty : au terminal ; --demo : sans greetd, pour l'essayer
  polkit-agent      agent d'authentification polkit au terminal (essai, ou session sans démon)
  updates [--upgrade | --refresh | --report]
                    paquets à mettre à jour (pacman, AUR, apt, dnf, zypper, flatpak) ;
                    --upgrade : mise à jour non interactive (pkexec), échec confié au démon
  reload            relit la configuration
  status            état du démon
  quit              arrête le démon
  chat-mock         faux moteur d'assistant (protocole serve, pour essais)
  preview [dossier] aperçu isolé ; capture les vues si un dossier est fourni
  check [fichier]   exécute et vérifie la configuration sans modifier le démon
  doctor            dépendances externes : présentes, absentes, et ce qui en dépend
  init              écrit la configuration d'exemple si aucune n'existe
  stubs [dossier]   écrit les définitions LuaLS (hyprdesk.meta.lua) pour l'autocomplétion
  snippet           lignes à ajouter à hyprland.lua (raccourci, flou, démarrage)

Configuration : $HYPRDESK_CONFIG, sinon ~/.config/hypr/hyprdesk.lua";

const STUBS: &str = include_str!("../assets/hyprdesk.meta.lua");
const STUBS_EN: &str = include_str!("../assets/hyprdesk.meta.en.lua");

/// Définitions LuaLS dans la langue de l'interface.
fn stubs() -> &'static str {
    match i18n::current() {
        i18n::Lang::Fr => STUBS,
        i18n::Lang::En => STUBS_EN,
    }
}

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

-- Captures : ajouter submap_universal AUX raccourcis existants (ne pas les dupliquer).
-- Sans cette option, Print et les autres raccourcis de capture sont suspendus sur le bureau.
-- Exemples :
-- hl.bind("Print", hl.dsp.exec_cmd("hyprshot -m region --clipboard-only"), { submap_universal = true })
-- hl.bind("SUPER + Print", hl.dsp.exec_cmd("hyprshot -m output -m active"), { submap_universal = true })

-- Barre de hyprdesk (hd.bar) : flou derrière elle, comme une barre classique.
hl.layer_rule({ name = "hyprdesk-bar-blur", match = { namespace = "^hyprdesk-bar$" }, blur = true, ignore_alpha = 0.2 })
"#;

/// Plugins de widgets d'exemple, déposés s'ils n'existent pas encore.
const EXAMPLE_WIDGETS: &[(&str, &str)] = &[
    ("meteo.lua", include_str!("../assets/widgets/meteo.lua")),
    ("notes.lua", include_str!("../assets/widgets/notes.lua")),
];

fn install_example_widgets(config_path: &std::path::Path) {
    let dir = widgets::plugins::dirs(config_path.parent().unwrap_or(std::path::Path::new(".")))[0]
        .clone();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    for (name, src) in EXAMPLE_WIDGETS {
        let p = dir.join(name);
        if !p.exists() && std::fs::write(&p, src).is_ok() {
            println!("{}", crate::tr!("écrit : {p0}", p0 = p.display()));
        }
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("HYPRDESK_LOG").unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(
            "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,iced_wgpu=warn",
        )
    });
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn daemon(path: PathBuf, replace: bool) -> ExitCode {
    init_tracing();
    let lock = single::lock_path();
    let _guard = match single::acquire(&lock) {
        Ok(g) => g,
        Err(None) => {
            eprintln!(
                "{}",
                crate::tr!(
                    "hyprdesk : verrou indisponible — vérifier le dossier de session et ses permissions"
                )
            );
            return ExitCode::FAILURE;
        }
        Err(pid) if replace => {
            // Relève propre : l'instance en place rend les fenêtres et ses
            // raccourcis en quittant, puis libère le verrou.
            tracing::info!(
                "{}",
                crate::tr!(
                    "remplacement de l'instance en place{p0}",
                    p0 = pid.map(|p| format!(" (pid {p})")).unwrap_or_default()
                )
            );
            if let Err(e) = control::send(control::Command::Quit) {
                tracing::warn!(
                    "{}",
                    crate::tr!("arrêt de l'instance en place : {e}", e = e)
                );
            }
            match single::acquire_within(&lock, std::time::Duration::from_secs(5)) {
                Ok(g) => g,
                Err(pid) => {
                    eprintln!(
                        "{}",
                        crate::tr!(
                            "hyprdesk : l'instance en place{p0} ne s'arrête pas ; arrêtez-la (hyprdesk quit, ou systemctl --user stop hyprdesk)",
                            p0 = pid.map(|p| format!(" (pid {p})")).unwrap_or_default()
                        )
                    );
                    return ExitCode::FAILURE;
                }
            }
        }
        Err(pid) => {
            // Sortie SANS erreur : systemd (Restart=on-failure) ne doit pas
            // relancer en boucle un démon qui fait double emploi.
            eprintln!(
                "{}",
                crate::tr!(
                    "hyprdesk : déjà lancé{p0} — une seule instance par session (hyprdesk daemon --replace pour la remplacer)",
                    p0 = pid.map(|p| format!(" (pid {p})")).unwrap_or_default()
                )
            );
            return ExitCode::SUCCESS;
        }
    };
    // Arrêt brutal précédent, bureau affiché : on rend les raccourcis.
    if hypr::current_submap() == hypr::SUBMAP {
        tracing::info!(
            "{}",
            crate::tr!(
                "submap « {p0} » laissée active par un arrêt précédent : remise à zéro",
                p0 = hypr::SUBMAP
            )
        );
        hypr::set_submap(None);
    }
    // Fenêtres laissées de côté par un arrêt brutal, bureau affiché : on les rend.
    hypr::restore_leftover_stash();
    // Le dossier des modules et des plugins doit exister dès le démarrage :
    // c'est lui que la surveillance suit pour détecter les nouveaux widgets.
    let widget_dir =
        widgets::plugins::dirs(path.parent().unwrap_or(std::path::Path::new(".")))[0].clone();
    if let Err(e) = std::fs::create_dir_all(&widget_dir) {
        tracing::warn!(
            "{}",
            crate::tr!("création de {p0} : {e}", p0 = widget_dir.display(), e = e)
        );
    }
    match config::ensure_exists(&path) {
        Ok(true) => {
            install_example_widgets(&path);
            tracing::info!(
                "{}",
                crate::tr!(
                    "configuration d'exemple écrite dans {p0}",
                    p0 = path.display()
                )
            );
            hypr::notify(
                1,
                6000,
                &crate::tr!(
                    "hyprdesk : configuration d'exemple créée — {p0}",
                    p0 = path.display()
                ),
            );
        }
        Ok(false) => {}
        Err(e) => tracing::warn!(
            "{}",
            crate::tr!("création de {p0} : {e}", p0 = path.display(), e = e)
        ),
    }
    // Modules de barre livrés depuis : ajoutés une fois à la barre réglée.
    bar_offer::run(&path);
    tracing::info!(
        "{}",
        crate::tr!(
            "hyprdesk {p0} — config {p1}",
            p0 = env!("CARGO_PKG_VERSION"),
            p1 = path.display()
        )
    );
    match ui::run(path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// Exécute une tâche asynchrone d'une commande cliente (hors démon).
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime tokio")
        .block_on(f)
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_path = config::default_path();
    if let Some(i) = args.iter().position(|a| a == "--config" || a == "-c") {
        let Some(p) = args.get(i + 1).cloned() else {
            eprintln!("{}", crate::tr!("--config attend un fichier"));
            return ExitCode::from(2);
        };
        config_path = PathBuf::from(p);
        args.drain(i..=i + 1);
    }
    // Langue de la ligne de commande (le démon la reprend de sa config).
    i18n::init_cli(&config_path);
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
            println!("{}", i18n::tr_lines(USAGE));
            ExitCode::SUCCESS
        }
        "-V" | "--version" | "version" => {
            println!(
                "{}",
                crate::tr!("hyprdesk {p0}", p0 = env!("CARGO_PKG_VERSION"))
            );
            ExitCode::SUCCESS
        }
        "preview" => match ui::preview::run(args.get(1).map(PathBuf::from)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}", crate::tr!("aperçu : {e}", e = e));
                ExitCode::FAILURE
            }
        },
        "check" => {
            let p = args.get(1).map(PathBuf::from).unwrap_or(config_path);
            match ui::check(&p) {
                Ok(c) => {
                    println!(
                        "{}",
                        crate::tr!(
                            "{p0} : ok — {p1} fonds, {p2} bureaux, {p3} icônes, {p4} widgets, {p5} hooks",
                            p0 = p.display(),
                            p1 = c.wallpapers.len(),
                            p2 = c.desktops.len(),
                            p3 = c.icons.len(),
                            p4 = c.widgets.len(),
                            p5 = c.hooks.len()
                        )
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
                println!("{}", crate::tr!("écrit : {p0}", p0 = config_path.display()));
                install_example_widgets(&config_path);
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!(
                    "{}",
                    crate::tr!("existe déjà : {p0}", p0 = config_path.display())
                );
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
            match std::fs::write(&p, stubs()) {
                Ok(()) => {
                    println!("{}", crate::tr!("écrit : {p0}", p0 = p.display()));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}: {e}", p.display());
                    ExitCode::FAILURE
                }
            }
        }
        // Écran de verrouillage (ext-session-lock-v1, PAM) : processus à part.
        "lock" => lock::run(),
        "chat-mock" => chat::mock(),
        // Lance l'application par défaut d'un rôle (navigateur, terminal, ai…).
        "open" => {
            let Some(r) = args.get(1).and_then(|r| apps::role(r)) else {
                let all: Vec<&str> = apps::ROLES.iter().map(|r| r.id).collect();
                eprintln!(
                    "{}",
                    crate::tr!("hyprdesk open <rôle> : {p0}", p0 = all.join(", "))
                );
                return ExitCode::FAILURE;
            };
            let cfg = match config::lua::load_file(&config_path) {
                Ok(l) => l.config,
                Err(e) => {
                    eprintln!("{}", crate::tr!("configuration : {e}", e = e));
                    return ExitCode::FAILURE;
                }
            };
            // Rôle « ai » avec l'assistant actif : le démon l'ouvre (tiroir ou fenêtre).
            if r.id == "ai"
                && assistant::lyra_installed()
                && cfg.plugin_on("assistant")
                && control::send(control::Command::Assistant).is_ok()
            {
                return ExitCode::SUCCESS;
            }
            let g = &cfg.general;
            // Rôle « ai » : LYRA (assistant du bureau), sinon l'outil choisi.
            let lyra = (r.id == "ai").then(|| assistant::command(&cfg)).flatten();
            match lyra.or_else(|| apps::command(r, &g.apps, &g.terminal)) {
                Some(cmd) => match hypr::exec(&cmd) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!(
                            "{}",
                            crate::tr!("lancement de « {cmd} » : {e}", cmd = cmd, e = e)
                        );
                        ExitCode::FAILURE
                    }
                },
                None => {
                    eprintln!(
                        "{}",
                        crate::tr!(
                            "aucune application pour « {p0} » (hyprdesk config set apps.{p1} …)",
                            p0 = r.id,
                            p1 = r.id
                        )
                    );
                    ExitCode::FAILURE
                }
            }
        }
        // Partage du setup : archive .tar.gz (configs, plugins, fonds d'écran).
        "export" => {
            let screens = args.iter().any(|a| a == "--screens");
            let scope = if args.iter().any(|a| a == "--theme") {
                share::Scope::Theme
            } else {
                share::Scope::All
            };
            let out = args[1..]
                .iter()
                .find(|a| !a.starts_with("--"))
                .map(|p| config::lua::expand_path(p, std::path::Path::new(".")))
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(format!(
                        "hyprdesk-{}-{}.tar.gz",
                        if scope == share::Scope::Theme {
                            "theme"
                        } else {
                            "setup"
                        },
                        chrono::Local::now().format("%Y%m%d")
                    ))
                });
            match share::export(&config_path, &out, scope, screens) {
                Ok(m) if scope == share::Scope::Theme => {
                    println!(
                        "{}",
                        crate::tr!(
                            "{p0} — thème « {p1} », {p2} fond(s)",
                            p0 = out.display(),
                            p1 = m.theme.map(|t| t.name).unwrap_or_default(),
                            p2 = m.wallpapers.len()
                        )
                    );
                    ExitCode::SUCCESS
                }
                Ok(m) => {
                    println!(
                        "{}",
                        crate::tr!(
                            "{p0} — {p1} fichier(s), {p2} fond(s), {p3} plugin(s){p4}",
                            p0 = out.display(),
                            p1 = m.files.len(),
                            p2 = m.wallpapers.len(),
                            p3 = m.plugins.len(),
                            p4 = if screens {
                                crate::i18n::tr(", écrans compris")
                            } else {
                                ""
                            }
                        )
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", crate::tr!("export : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        "import" => {
            let screens = args.iter().any(|a| a == "--screens");
            let dry = args.iter().any(|a| a == "--dry-run");
            let Some(file) = args[1..].iter().find(|a| !a.starts_with("--")) else {
                eprintln!(
                    "{}",
                    crate::tr!(
                        "hyprdesk import <archive.tar.gz> [--theme] [--dry-run] [--screens]"
                    )
                );
                return ExitCode::FAILURE;
            };
            let file = config::lua::expand_path(file, std::path::Path::new("."));
            let opts = share::ImportOpts {
                scope: if args.iter().any(|a| a == "--theme") {
                    share::Scope::Theme
                } else {
                    share::Scope::All
                },
                screens,
                dry_run: dry,
                backup: true,
            };
            match share::import(&file, &config_path, opts) {
                Ok(r) => {
                    println!("{r}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", crate::tr!("import : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Thèmes : créer depuis une scène, lister « Mes thèmes », appliquer.
        "theme" => match args.get(1).map(String::as_str) {
            Some("new") => {
                let Some(src) = args[2..]
                    .iter()
                    .enumerate()
                    .find(|(i, a)| !a.starts_with("--") && (*i == 0 || args[i + 1] != "--name"))
                    .map(|(_, a)| a)
                else {
                    eprintln!(
                        "{}",
                        crate::tr!("hyprdesk theme new <image|pack.zip|dossier> [--name NOM]")
                    );
                    return ExitCode::FAILURE;
                };
                let name = args
                    .iter()
                    .position(|a| a == "--name")
                    .and_then(|i| args.get(i + 1))
                    .map(String::as_str);
                let src = config::lua::expand_path(src, std::path::Path::new("."));
                match theme::create(&config_path, &src, name) {
                    Ok((out, warning)) => {
                        println!("{}", out.display());
                        if let Some(w) = warning {
                            eprintln!("{}", crate::tr!("attention : {w}", w = w));
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{}", crate::tr!("theme new : {e}", e = e));
                        ExitCode::FAILURE
                    }
                }
            }
            Some("list") | None => {
                for t in theme::library(&config_path) {
                    println!("{:<32} {}", t.summary.name, t.path.display());
                }
                ExitCode::SUCCESS
            }
            Some("apply") => {
                let Some(what) = args.get(2) else {
                    eprintln!("{}", crate::tr!("hyprdesk theme apply <nom|archive>"));
                    return ExitCode::FAILURE;
                };
                let file = theme::library(&config_path)
                    .into_iter()
                    .find(|t| t.summary.name.eq_ignore_ascii_case(what))
                    .map(|t| t.path)
                    .unwrap_or_else(|| config::lua::expand_path(what, std::path::Path::new(".")));
                let opts = share::ImportOpts {
                    scope: share::Scope::Theme,
                    backup: true,
                    ..Default::default()
                };
                match share::import(&file, &config_path, opts) {
                    Ok(r) => {
                        println!("{r}");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{}", crate::tr!("theme apply : {e}", e = e));
                        ExitCode::FAILURE
                    }
                }
            }
            Some(other) => {
                eprintln!(
                    "{}",
                    crate::tr!(
                        "« {other} » : hyprdesk theme [list | new <source> | apply <nom>]",
                        other = other
                    )
                );
                ExitCode::FAILURE
            }
        },
        // Découpe d'une scène selon la disposition des écrans.
        "wallpaper" if args.get(1).map(String::as_str) == Some("split") => {
            // Arguments positionnels : ni option, ni valeur d'option.
            let pos: Vec<&String> = args[2..]
                .iter()
                .enumerate()
                .filter(|(i, a)| !a.starts_with("--") && (*i == 0 || args[i + 1] != "--scale"))
                .map(|(_, a)| a)
                .collect();
            let scale = args
                .iter()
                .position(|a| a == "--scale")
                .and_then(|i| args.get(i + 1))
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(1.0)
                .clamp(0.25, 4.0);
            let Some(img) = pos.first() else {
                eprintln!(
                    "{}",
                    crate::tr!("hyprdesk wallpaper split <image> [dossier] [--scale N]")
                );
                return ExitCode::FAILURE;
            };
            let img = config::lua::expand_path(img, std::path::Path::new("."));
            let out = pos
                .get(1)
                .map(|p| config::lua::expand_path(p, std::path::Path::new(".")))
                .unwrap_or_else(|| PathBuf::from("."));
            let r = span::screens().and_then(|screens| {
                if let Ok((w, h)) = image::image_dimensions(&img) {
                    let rects: Vec<span::Rect> = screens
                        .iter()
                        .map(|s| span::physical(s.rect, scale))
                        .collect();
                    if let Some(warn) = span::resolution_warning(w, h, &rects, true) {
                        eprintln!("{}", crate::tr!("attention : {warn}", warn = warn));
                    }
                }
                span::split(&img, &screens, &out, scale)
            });
            match r {
                Ok(files) => {
                    for f in files {
                        println!("{}", f.display());
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", crate::tr!("wallpaper split : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Contexte de l'environnement hyprdesk, pour un assistant IA.
        "context" => match config::lua::load_file(&config_path) {
            Ok(l) => {
                print!("{}", assistant::context(&l.config, &config_path));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}", crate::tr!("configuration : {e}", e = e));
                ExitCode::FAILURE
            }
        },
        // Configuration en ligne de commande : même effet que l'écran Réglages.
        "config" => match cli_config::run(&config_path, &args[1..]) {
            Ok(out) => {
                println!("{out}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}", crate::tr!("hyprdesk config : {e}", e = e));
                ExitCode::FAILURE
            }
        },
        // Touches de volume et de luminosité : l'action se fait ici, sans démon.
        "osd" => {
            let (kind, step) = match osd::parse(&args[1..]) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
            match block_on(osd::apply(kind, step)) {
                Ok(l) => {
                    // Affichage à l'écran par le démon, s'il tourne.
                    let _ = control::send(control::Command::Osd(format!(
                        "{} {} {}",
                        kind.as_str(),
                        l.value,
                        u8::from(l.muted)
                    )));
                    println!(
                        "{} {:.0} %{}",
                        kind.as_str(),
                        l.value * 100.0,
                        if l.muted { " (muet)" } else { "" }
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", crate::tr!("hyprdesk osd : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // État du lecteur (plugin « lecteur ») ; aucun lecteur : sortie vide.
        "media" if args.get(1).map(String::as_str) == Some("status") => {
            if let Some(s) = block_on(crate::widgets::media::status()) {
                print!("{}", crate::widgets::media::status_lines(&s));
            }
            ExitCode::SUCCESS
        }
        "media" if args.get(1).map(String::as_str) == Some("seek") => {
            let Some(secs) = args.get(2).and_then(|a| a.parse::<i64>().ok()) else {
                eprintln!("hyprdesk media seek <+N|-N>");
                return ExitCode::from(2);
            };
            let r = block_on(async {
                let s = crate::widgets::media::status()
                    .await
                    .ok_or_else(|| crate::i18n::tr("aucun lecteur (MPRIS)").to_string())?;
                crate::widgets::media::seek(&s.bus, secs).await
            });
            match r {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", crate::tr!("hyprdesk media : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        "media" => {
            let Some(method) = args.get(1).and_then(|a| osd::media_method(a)) else {
                eprintln!("hyprdesk media <play-pause|next|previous|status|seek ±N>");
                return ExitCode::from(2);
            };
            match block_on(osd::media(method)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", crate::tr!("hyprdesk media : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Écran de connexion pour greetd, au terminal.
        "greeter" if args.get(1).is_none() => greeter_ui::run(false),
        "greeter" if args.get(1).map(String::as_str) == Some("--demo") => greeter_ui::run(true),
        "greeter" if args.get(1).map(String::as_str) == Some("--tty") => match greeter::run_tty() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}", crate::tr!("hyprdesk greeter : {e}", e = e));
                ExitCode::FAILURE
            }
        },
        // Captures et enregistrement de l'écran (hyprshot, wf-recorder).
        "capture" => {
            let action = match capture::parse(&args[1..]) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
            match capture::run(action) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", crate::tr!("hyprdesk capture : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Agent polkit au terminal : mot de passe demandé ici (comme pkttyagent).
        "polkit-agent" => block_on(async {
            let _conn = match polkit::register(std::sync::Arc::new(polkit::TtyPrompter)).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "{}",
                        crate::tr!(
                            "hyprdesk polkit-agent : enregistrement refusé ({e}) — un autre agent est-il déjà en place ?",
                            e = e
                        )
                    );
                    return ExitCode::FAILURE;
                }
            };
            eprintln!(
                "{}",
                crate::i18n::tr("Agent polkit en place (Ctrl+C pour l'arrêter).")
            );
            let _ = tokio::signal::ctrl_c().await;
            ExitCode::SUCCESS
        }),
        // Mises à jour en attente, et mise à jour non interactive.
        "updates" => {
            let sources = updates::configured_sources(&config_path);
            if sources.is_empty() {
                eprintln!("{}", crate::i18n::tr("aucune source de paquets reconnue"));
                return ExitCode::FAILURE;
            }
            match args.get(1).map(String::as_str) {
                // Mise à jour complète (plugin de barre) : état, relevé, rapport d'échec au démon.
                Some("--upgrade") => {
                    return match block_on(updates::upgrade_and_report(&sources)) {
                        Some(r) => {
                            eprintln!("{r}");
                            ExitCode::FAILURE
                        }
                        None => ExitCode::SUCCESS,
                    };
                }
                // Relevé écrit pour le plugin de barre.
                Some("--refresh") => {
                    let p = block_on(updates::refresh(&sources));
                    println!("{}", updates::total(&p));
                    return ExitCode::SUCCESS;
                }
                // Dernier échec, renvoyé au démon (assistant ou fenêtre de détail).
                Some("--report") => {
                    return match std::fs::read_to_string(updates::report_path()) {
                        Ok(r) => {
                            updates::send_problem(&r, true);
                            println!("{r}");
                            ExitCode::SUCCESS
                        }
                        Err(_) => {
                            eprintln!(
                                "{}",
                                crate::i18n::tr("aucun échec de mise à jour enregistré")
                            );
                            ExitCode::FAILURE
                        }
                    };
                }
                _ => {}
            }
            let pending = block_on(updates::check(&sources));
            for p in &pending {
                println!("{} : {}", crate::i18n::tr(p.label), p.packages.len());
                for name in p.packages.iter().take(30) {
                    println!("  {name}");
                }
            }
            println!(
                "{}",
                crate::tr!(
                    "{n} paquet(s) à mettre à jour",
                    n = updates::total(&pending)
                )
            );
            ExitCode::SUCCESS
        }
        // Dépendances externes : présentes, absentes, et ce qui en dépend.
        "doctor" => {
            let list = doctor::check();
            println!("{}", doctor::report(&list));
            if doctor::essential_missing(&list) {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        "snippet" => {
            println!("{}", i18n::tr_lines(SNIPPET));
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
                    eprintln!("{}", crate::tr!("hyprdesk : {e}", e = e));
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!(
                    "{}\n\n{}",
                    crate::tr!("commande inconnue : {other}", other = other),
                    i18n::tr_lines(USAGE)
                );
                ExitCode::from(2)
            }
        },
    }
}
