//! hyprdesk — bureau à la demande pour Hyprland.
//!
//! Le même binaire est le démon (`hyprdesk`, `hyprdesk daemon`) et son client
//! (`hyprdesk toggle`, à lier à une touche dans `hyprland.lua`).

use hyprdesk::*;
use std::path::PathBuf;
use std::process::ExitCode;

const STUBS: &str = include_str!("../assets/hyprdesk.meta.lua");
const STUBS_EN: &str = include_str!("../assets/hyprdesk.meta.en.lua");

/// Définitions LuaLS dans la langue de l'interface.
fn stubs() -> &'static str {
    match i18n::current() {
        i18n::Lang::Fr => STUBS,
        i18n::Lang::En => STUBS_EN,
    }
}

/// Plugins de widgets d'exemple, déposés s'ils n'existent pas encore.
const EXAMPLE_WIDGETS: &[(&str, &str)] = &[
    ("meteo.lua", include_str!("../assets/widgets/meteo.lua")),
    ("notes.lua", include_str!("../assets/widgets/notes.lua")),
];

/// `hyprdesk keyboard [disposition [variante [options]]]` : sans argument, le
/// clavier en vigueur (« fr azerty ») ; sinon appliqué à chaud (si Hyprland
/// répond) et écrit dans inputs.lua, comme Réglages → Clavier et souris.
fn keyboard_cli(config_path: &std::path::Path, args: &[String]) -> ExitCode {
    let mut i = inputs::read();
    let Some(layout) = args.first() else {
        println!("{} {} {}", i.kb_layout, i.kb_variant, i.kb_options);
        return ExitCode::SUCCESS;
    };
    let ok = |v: &str| {
        v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ',' | '_' | '-' | ':' | '(' | ')'))
    };
    let variant = args.get(1).cloned().unwrap_or_default();
    let options = args.get(2).cloned().unwrap_or_default();
    if !ok(layout) || !ok(&variant) || !ok(&options) {
        eprintln!(
            "{}",
            hyprdesk::i18n::tr("hyprdesk keyboard : valeur de clavier invalide")
        );
        return ExitCode::FAILURE;
    }
    i.kb_layout = layout.clone();
    i.kb_variant = variant;
    i.kb_options = options;
    // Une variante absente de xkb sur ce poste ferait refuser tout le clavier.
    if let Some(w) = i.check_keyboard(&inputs::layouts()) {
        eprintln!("hyprdesk keyboard : {w}");
    }
    // Hyprland répond mais refuse le clavier : rien n'est écrit (inputs.lua
    // ferait l'erreur à chaque démarrage). Hyprland absent : écrit pour plus tard.
    let running = hyprpanel_core::ipc::query_json("version").is_ok();
    if let Err(e) = inputs::apply(&i) {
        if running {
            eprintln!(
                "{}",
                hyprdesk::tr!(
                    "clavier refusé par Hyprland, inputs.lua inchangé : {e}",
                    e = e
                )
            );
            return ExitCode::FAILURE;
        }
        eprintln!(
            "{}",
            hyprdesk::tr!(
                "clavier non appliqué à chaud ({e}) : il le sera au prochain démarrage",
                e = e
            )
        );
    }
    let path = inputs::inputs_path(config_path);
    match inputs::save(&path, &i) {
        Ok(()) => {
            println!("{}", hyprdesk::tr!("écrit : {p0}", p0 = path.display()));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!(
                "{}",
                hyprdesk::tr!("{p0} : {e}", p0 = path.display(), e = e.to_string())
            );
            ExitCode::FAILURE
        }
    }
}

fn install_example_widgets(config_path: &std::path::Path) {
    let dir = widgets::plugins::dirs(config_path.parent().unwrap_or(std::path::Path::new(".")))[0]
        .clone();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    for (name, src) in EXAMPLE_WIDGETS {
        let p = dir.join(name);
        if !p.exists() && std::fs::write(&p, src).is_ok() {
            println!("{}", hyprdesk::tr!("écrit : {p0}", p0 = p.display()));
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
    // Environnement de lancement (docs/THEME-ENGINE.md §9.2) : les variables
    // HYPRDESK_* de la palette publiée, héritées par tout ce que le démon
    // lance cette session. Avant tout fil.
    hyprdesk::theme_engine::env::inject_session();
    // Hyprland trop ancien : rien ne lui est envoyé (un raccourci posé par
    // hyprctl eval fait planter la 0.56). Code 75 : systemd ne relance pas.
    if let Some((a, b, c)) = hypr::running_version()
        && (a, b) < hypr::MIN_HYPRLAND
    {
        let msg = hyprdesk::tr!(
            "hyprdesk : Hyprland {v} trop ancien ({min} ou plus requis) — mettez à jour Hyprland puis reconnectez-vous",
            v = format!("{a}.{b}.{c}"),
            min = format!("{}.{}", hypr::MIN_HYPRLAND.0, hypr::MIN_HYPRLAND.1)
        );
        eprintln!("{msg}");
        hypr::notify(0, 15000, &msg);
        // Laisse le temps au fil des notifications d'envoyer le message.
        std::thread::sleep(std::time::Duration::from_millis(300));
        return ExitCode::from(75);
    }
    let lock = single::lock_path();
    let _guard = match single::acquire(&lock) {
        Ok(g) => g,
        Err(None) => {
            eprintln!(
                "{}",
                hyprdesk::tr!(
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
                hyprdesk::tr!(
                    "remplacement de l'instance en place{p0}",
                    p0 = pid.map(|p| format!(" (pid {p})")).unwrap_or_default()
                )
            );
            if let Err(e) = control::send(control::Command::Quit) {
                tracing::warn!(
                    "{}",
                    hyprdesk::tr!("arrêt de l'instance en place : {e}", e = e)
                );
            }
            match single::acquire_within(&lock, std::time::Duration::from_secs(5)) {
                Ok(g) => g,
                Err(pid) => {
                    eprintln!(
                        "{}",
                        hyprdesk::tr!(
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
                hyprdesk::tr!(
                    "hyprdesk : déjà lancé{p0} — une seule instance par session (hyprdesk daemon --replace pour la remplacer)",
                    p0 = pid.map(|p| format!(" (pid {p})")).unwrap_or_default()
                )
            );
            return ExitCode::SUCCESS;
        }
    };
    // Arrêt brutal précédent, bureau affiché : on rend les raccourcis.
    let left = hypr::current_submap();
    if left == hypr::SUBMAP || left == hypr::CAPTURE_SUBMAP {
        tracing::info!(
            "{}",
            hyprdesk::tr!(
                "submap « {p0} » laissée active par un arrêt précédent : remise à zéro",
                p0 = left
            )
        );
        hypr::set_submap(None);
    }
    // Fenêtres laissées de côté par un arrêt brutal, bureau affiché : on les rend.
    hypr::restore_leftover_stash();
    // Historique de la configuration : élagage à 30 jours, et instantané des
    // modifications faites à la main pendant l'absence du démon.
    history::start_daemon();
    // Raccourcis de hyprland.lua rangés dans binds.lua (MLG-111) ; Hyprland
    // rechargé, tout rendu tel quel s'il signale une erreur nouvelle.
    match binds_file::migrate(&path) {
        Ok(Some(d)) => {
            let msg = hyprdesk::tr!(
                "hyprdesk : {n} raccourci(s) de hyprland.lua rangé(s) dans binds.lua (Réglages → Raccourcis) ; copie : {b}",
                n = d.moved,
                b = d.backup.display()
            );
            tracing::info!("{msg}");
            hypr::notify(1, 10000, &msg);
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!("{e}");
            hypr::notify(0, 10000, &hyprdesk::tr!("hyprdesk : {e}", e = e));
        }
    }
    // binds.lua tenu par hyprdesk (MLG-198) : un fichier écrit à la main est
    // copié puis régénéré ; Hyprland vérifie, tout est rendu s'il proteste.
    match binds_file::adopt(&path) {
        Ok(Some(bak)) => {
            let msg = hyprdesk::tr!(
                "hyprdesk : binds.lua est maintenant tenu par hyprdesk (Réglages → Raccourcis) ; copie : {b}",
                b = bak.display()
            );
            tracing::info!("{msg}");
            hypr::notify(1, 10000, &msg);
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("{e}"),
    }
    // Réglages de Hyprland tenus par hyprdesk : repris de hyprland.lua (lignes
    // commentées), écrits dans les fichiers générés (MLG-139). Vérifié par
    // Hyprland : tout est rendu tel quel s'il signale une erreur nouvelle.
    match adopt::run(&path) {
        Ok(d) if !d.adopted.is_empty() => {
            let msg = hyprdesk::tr!("hyprdesk : {s}", s = adopt::summary(&d));
            tracing::info!("{msg}");
            for l in &d.left {
                tracing::info!("{}", hyprdesk::tr!("laissé dans hyprland.lua : {l}", l = l));
            }
            hypr::notify(1, 10000, &msg);
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!("{e}");
            hypr::notify(0, 10000, &hyprdesk::tr!("hyprdesk : {e}", e = e));
        }
    }
    // binds.lua en accord avec ce que hyprdesk tient (MLG-140) : touches
    // prises désactivées, outils équivalents rebranchés sur hyprdesk.
    if let Ok(l) = config::lua::load_file(&path) {
        match binds_file::sync_hyprdesk(&path, &l.config, false) {
            Ok(done) if !done.is_empty() => {
                for d in &done {
                    tracing::info!("{d}");
                }
                hypr::notify(
                    1,
                    10000,
                    &hyprdesk::tr!(
                        "hyprdesk : {n} raccourci(s) de binds.lua accordé(s) à hyprdesk (Réglages → Raccourcis)",
                        n = done.len()
                    ),
                );
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("{e}"),
        }
    }
    // Le dossier des modules et des plugins doit exister dès le démarrage :
    // c'est lui que la surveillance suit pour détecter les nouveaux widgets.
    let widget_dir =
        widgets::plugins::dirs(path.parent().unwrap_or(std::path::Path::new(".")))[0].clone();
    if let Err(e) = std::fs::create_dir_all(&widget_dir) {
        tracing::warn!(
            "{}",
            hyprdesk::tr!("création de {p0} : {e}", p0 = widget_dir.display(), e = e)
        );
    }
    match config::ensure_exists(&path) {
        Ok(true) => {
            install_example_widgets(&path);
            tracing::info!(
                "{}",
                hyprdesk::tr!(
                    "configuration d'exemple écrite dans {p0}",
                    p0 = path.display()
                )
            );
            hypr::notify(
                1,
                6000,
                &hyprdesk::tr!(
                    "hyprdesk : configuration d'exemple créée — {p0}",
                    p0 = path.display()
                ),
            );
        }
        Ok(false) => {}
        Err(e) => tracing::warn!(
            "{}",
            hyprdesk::tr!("création de {p0} : {e}", p0 = path.display(), e = e)
        ),
    }
    // Premier démarrage : l'assistant de configuration s'ouvre pour faire connaissance.
    if bar_offer::first_start() {
        ui::OPEN_WIZARD_AT_START.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    // Modules de barre livrés depuis : ajoutés une fois à la barre réglée.
    bar_offer::run(&path);
    tracing::info!(
        "{}",
        hyprdesk::tr!(
            "hyprdesk {p0} — config {p1}",
            p0 = env!("CARGO_PKG_VERSION"),
            p1 = path.display()
        )
    );
    let r = ui::run(path);
    let reason = control::EXIT_REASON.lock().ok().and_then(|r| r.clone());
    match (r, reason) {
        (Ok(()), Some(why)) => {
            tracing::info!("{}", hyprdesk::tr!("hyprdesk s'arrête : {why}", why = why));
            // `hyprdesk quit` : code dédié, que systemd ne relance pas ;
            // un signal (systemctl stop, Ctrl-C) : 0.
            if control::QUIT_REQUESTED.load(std::sync::atomic::Ordering::Relaxed) {
                ExitCode::from(control::QUIT_EXIT)
            } else {
                ExitCode::SUCCESS
            }
        }
        // Fin sans demande d'arrêt : jamais un succès (systemd relance).
        (Ok(()), None) => {
            tracing::error!(
                "{}",
                hyprdesk::i18n::tr(
                    "boucle d'affichage terminée sans demande d'arrêt : sortie en erreur"
                )
            );
            ExitCode::FAILURE
        }
        (Err(e), _) => {
            tracing::error!(
                "{}",
                hyprdesk::tr!("hyprdesk s'arrête sur une erreur : {e}", e = e)
            );
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

/// `hyprdesk index daemon|query|status|rebuild|stop` — démon et client de
/// l'index de fichiers (recherche :f du lanceur).
fn index_cli(args: &[String]) -> ExitCode {
    use fileindex::proto::{Reply, Request, request};

    let sous = args.first().map(String::as_str).unwrap_or("");
    match sous {
        "daemon" => {
            init_tracing();
            let config = config_demon_index(&args[1..]);
            match fileindex::daemon::run(config) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("démon d'index : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        "query" => {
            let mut q = String::new();
            let mut limit = fileindex::proto::LIMIT_DEFAULT;
            let mut json = false;
            let mut lits = args[1..].iter();
            while let Some(a) = lits.next() {
                match a.as_str() {
                    "--limit" => {
                        limit = lits
                            .next()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(fileindex::proto::LIMIT_DEFAULT);
                    }
                    "--json" => json = true,
                    _ => {
                        if !q.is_empty() {
                            q.push(' ');
                        }
                        q.push_str(a);
                    }
                }
            }
            if q.trim().is_empty() {
                eprintln!("{}", hyprdesk::tr!("index query attend un mot à chercher"));
                return ExitCode::from(2);
            }
            match request(&Request::Query {
                q,
                limit,
                kinds: None,
                cat: None,
            }) {
                Ok(Reply::Hits { hits, .. }) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&hits).unwrap_or_default()
                        );
                        return ExitCode::SUCCESS;
                    }
                    if hits.is_empty() {
                        println!("{}", hyprdesk::tr!("aucun résultat dans l'index"));
                        return ExitCode::SUCCESS;
                    }
                    for h in &hits {
                        let genre = match h.kind {
                            fileindex::index::KIND_DIR => hyprdesk::tr!("dossier"),
                            fileindex::index::KIND_LINK => hyprdesk::tr!("lien"),
                            _ => hyprdesk::tr!("fichier"),
                        };
                        println!("{genre}\t{}\t{}", fileindex::fmt_taille(h.size), h.path);
                    }
                    ExitCode::SUCCESS
                }
                Ok(Reply::Err(m)) => {
                    eprintln!("{m}");
                    ExitCode::FAILURE
                }
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!(
                        "{}",
                        hyprdesk::tr!(
                            "démon d'index injoignable : {e} — lancez « hyprdesk index daemon »",
                            e = e
                        )
                    );
                    ExitCode::FAILURE
                }
            }
        }
        "status" => match request(&Request::Status) {
            Ok(Reply::Status(j)) => match serde_json::from_str::<serde_json::Value>(&j) {
                Ok(v) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&v).unwrap_or_else(|_| j.clone())
                    );
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    println!("{j}");
                    ExitCode::SUCCESS
                }
            },
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!(
                    "{}",
                    hyprdesk::tr!(
                        "démon d'index injoignable : {e} — lancez « hyprdesk index daemon »",
                        e = e
                    )
                );
                ExitCode::FAILURE
            }
        },
        "rebuild" => match request(&Request::Rebuild) {
            Ok(Reply::Ok) => {
                println!("{}", hyprdesk::tr!("rescan complet demandé"));
                ExitCode::SUCCESS
            }
            Ok(Reply::Err(m)) => {
                eprintln!("{m}");
                ExitCode::FAILURE
            }
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!(
                    "{}",
                    hyprdesk::tr!("démon d'index injoignable : {e}", e = e)
                );
                ExitCode::FAILURE
            }
        },
        "stop" => match fileindex::proto::stop() {
            Ok(()) => {
                println!("{}", hyprdesk::tr!("démon d'index arrêté"));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!(
                "{}",
                hyprdesk::tr!("usage : hyprdesk index daemon|query <mot>|status|rebuild|stop")
            );
            ExitCode::from(2)
        }
    }
}

/// Réglages du démon passés en ligne de commande (le lanceur les reprend
/// de la configuration) : --paths /a,/b --exclude nom --interval N.
fn config_demon_index(args: &[String]) -> fileindex::IndexConfig {
    let mut config = fileindex::IndexConfig::default();
    let mut lits = args.iter();
    while let Some(a) = lits.next() {
        match a.as_str() {
            "--paths" => {
                if let Some(v) = lits.next() {
                    let chemins: Vec<PathBuf> = v
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(fileindex::expand_root)
                        .collect();
                    if !chemins.is_empty() {
                        config.roots = chemins;
                    }
                }
            }
            "--exclude" => {
                if let Some(v) = lits.next() {
                    config.extra_excludes = v
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            "--exclude-regex" => {
                if let Some(v) = lits.next() {
                    config.exclude_regex = v
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            "--exclude-ext" => {
                if let Some(v) = lits.next() {
                    config.exclude_ext = v
                        .split(',')
                        .map(|s| s.trim().trim_start_matches('.').to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            "--exclude-cat" => {
                if let Some(v) = lits.next() {
                    config.exclude_cat = v
                        .split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            "--max-size" => {
                if let Some(v) = lits.next()
                    && let Ok(m) = v.parse::<u64>()
                {
                    config.max_size = m;
                }
            }
            "--interval" => {
                if let Some(v) = lits.next()
                    && let Ok(s) = v.parse()
                {
                    config.interval = s;
                }
            }
            _ => {}
        }
    }
    config
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_path = config::default_path();
    if let Some(i) = args.iter().position(|a| a == "--config" || a == "-c") {
        let Some(p) = args.get(i + 1).cloned() else {
            eprintln!("{}", hyprdesk::tr!("--config attend un fichier"));
            return ExitCode::from(2);
        };
        config_path = PathBuf::from(p);
        args.drain(i..=i + 1);
    }
    // Langue de la ligne de commande (le démon la reprend de sa config).
    i18n::init_cli(&config_path);
    // Historique de la configuration (MLG-138) : instantané avant chaque
    // écriture d'un fichier suivi, quel que soit le processus.
    history::init(&config_path);
    let replace = if let Some(i) = args.iter().position(|a| a == "--replace") {
        args.remove(i);
        true
    } else {
        false
    };
    let cmd = args.first().map(String::as_str).unwrap_or("daemon");
    // Moteur de l'interface (rendu, carte graphique, fils), préparé tant
    // qu'il n'y a qu'un fil.
    render::init(&config_path, cmd == "daemon");
    match cmd {
        "daemon" => daemon(config_path, replace),
        "-h" | "--help" | "help" => {
            println!("{}", i18n::tr_lines(USAGE));
            ExitCode::SUCCESS
        }
        "-V" | "--version" | "version" => {
            println!(
                "{}",
                hyprdesk::tr!("hyprdesk {p0}", p0 = env!("CARGO_PKG_VERSION"))
            );
            ExitCode::SUCCESS
        }
        // Fenêtre myMFA en application (lancée par le démon via Hyprland).
        "mymfa" if args.get(1).map(String::as_str) == Some("window") => {
            match ui::run_mymfa_fenetre(config_path) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("fenêtre myMFA : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Une option n'est pas un dossier de sortie : `preview --help` créait
        // un dossier `--help/` rempli de captures (MLG-211).
        "preview" if args.get(1).is_some_and(|a| a.starts_with('-')) => {
            eprintln!("{}", hyprdesk::tr!("usage : hyprdesk preview [dossier]"));
            ExitCode::from(2)
        }
        "preview" => match ui::preview::run(args.get(1).map(PathBuf::from)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}", hyprdesk::tr!("aperçu : {e}", e = e));
                ExitCode::FAILURE
            }
        },
        "check" => {
            let p = args.get(1).map(PathBuf::from).unwrap_or(config_path);
            match ui::check(&p) {
                Ok(c) => {
                    println!(
                        "{}",
                        hyprdesk::tr!(
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
        "keyboard" => keyboard_cli(&config_path, &args[1..]),
        "init" => match config::ensure_exists(&config_path) {
            Ok(true) => {
                println!(
                    "{}",
                    hyprdesk::tr!("écrit : {p0}", p0 = config_path.display())
                );
                install_example_widgets(&config_path);
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!(
                    "{}",
                    if config_path.exists() {
                        hyprdesk::tr!("existe déjà : {p0}", p0 = config_path.display())
                    } else {
                        hyprdesk::tr!(
                            "configuration déjà en place (Réglages : {p0}) ; hyprdesk.lua, facultatif, n'est pas recréé",
                            p0 = config::lua::settings_path(&config_path).display()
                        )
                    }
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
                    println!("{}", hyprdesk::tr!("écrit : {p0}", p0 = p.display()));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}: {e}", p.display());
                    ExitCode::FAILURE
                }
            }
        }
        // Écran de verrouillage (ext-session-lock-v1, PAM) : processus à part.
        // Binaire dédié s'il est installé à côté (voir `companion`).
        "lock" => {
            companion::exec(companion::LOCK, &companion::forward(&config_path, &[]));
            lock::run()
        }
        "chat-mock" => chat::mock(),
        // Lance l'application par défaut d'un rôle (navigateur, terminal, ai…).
        "open" => {
            let Some(r) = args.get(1).and_then(|r| apps::role(r)) else {
                let all: Vec<&str> = apps::ROLES.iter().map(|r| r.id).collect();
                eprintln!(
                    "{}",
                    hyprdesk::tr!("hyprdesk open <rôle> : {p0}", p0 = all.join(", "))
                );
                return ExitCode::FAILURE;
            };
            let cfg = match config::lua::load_file(&config_path) {
                Ok(l) => l.config,
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("configuration : {e}", e = e));
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
                            hyprdesk::tr!("lancement de « {cmd} » : {e}", cmd = cmd, e = e)
                        );
                        ExitCode::FAILURE
                    }
                },
                None => {
                    eprintln!(
                        "{}",
                        hyprdesk::tr!(
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
            match share::export(&config_path, &out, scope) {
                Ok(m) if scope == share::Scope::Theme => {
                    println!(
                        "{}",
                        hyprdesk::tr!(
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
                        hyprdesk::tr!(
                            "{p0} — {p1} fichier(s) hyprdesk, {p2} de Hyprland, {p3} fond(s), {p4} plugin(s), {p5} image(s)",
                            p0 = out.display(),
                            p1 = m.files.len(),
                            p2 = m.hypr.len(),
                            p3 = m.wallpapers.len(),
                            p4 = m.plugins.len(),
                            p5 = m.assets.len()
                        )
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("export : {e}", e = e));
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
                    hyprdesk::tr!(
                        "hyprdesk import <archive.tar.gz> [--theme] [--dry-run] [--screens] [--trust-code]"
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
                trust_code: args.iter().any(|a| a == "--trust-code"),
            };
            match share::import(&file, &config_path, opts) {
                Ok(r) => {
                    println!("{r}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("import : {e}", e = e));
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
                        hyprdesk::tr!("hyprdesk theme new <image|pack.zip|dossier> [--name NOM]")
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
                            eprintln!("{}", hyprdesk::tr!("attention : {w}", w = w));
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{}", hyprdesk::tr!("theme new : {e}", e = e));
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
                    eprintln!("{}", hyprdesk::tr!("hyprdesk theme apply <nom|archive>"));
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
                        eprintln!("{}", hyprdesk::tr!("theme apply : {e}", e = e));
                        ExitCode::FAILURE
                    }
                }
            }
            Some(other) => {
                eprintln!(
                    "{}",
                    hyprdesk::tr!(
                        "« {other} » : hyprdesk theme [list | new <source> | apply <nom>]",
                        other = other
                    )
                );
                ExitCode::FAILURE
            }
        },
        // Comparaison à l'œil d'un effet (MLG-262) : variantes de flou…
        "fx" if args.get(1).map(String::as_str) == Some("compare") => {
            let mut pos = Vec::new();
            let (mut secs, mut rounds) = (4.0_f64, 1_u32);
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--seconds" | "--rounds" => {
                        let Some(v) = args.get(i + 1) else {
                            eprintln!("{}", hyprdesk::tr!("{o} attend une valeur", o = args[i]));
                            return ExitCode::from(2);
                        };
                        let ok = if args[i] == "--seconds" {
                            v.parse().map(|n| secs = n).is_ok()
                        } else {
                            v.parse().map(|n| rounds = n).is_ok()
                        };
                        if !ok {
                            eprintln!("{}", hyprdesk::tr!("« {v} » : nombre attendu", v = v));
                            return ExitCode::from(2);
                        }
                        i += 1;
                    }
                    a => pos.push(a.to_string()),
                }
                i += 1;
            }
            let Some(path) = pos.first() else {
                eprintln!(
                    "{}",
                    hyprdesk::i18n::tr(
                        "usage : hyprdesk fx compare <effet> [a,b,c|all] [--seconds N] [--rounds N]"
                    )
                );
                return ExitCode::from(2);
            };
            let plan = match hyprdesk::fx::compare_plan(path, pos.get(1).map_or("", String::as_str))
            {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
            // Ctrl-C : arrêt propre, la valeur d'origine est remise.
            static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            extern "C" fn on_signal(_: libc::c_int) {
                STOP.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            unsafe {
                libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
                libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
            }
            match hyprdesk::fx::compare_run(&plan, secs, rounds, &STOP, |_, label| {
                println!("{label}");
            }) {
                Ok(Some(v)) => {
                    println!("{}", hyprdesk::tr!("remis : {v}", v = v.display()));
                    ExitCode::SUCCESS
                }
                Ok(None) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("hyprdesk fx compare : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
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
                    hyprdesk::tr!("hyprdesk wallpaper split <image> [dossier] [--scale N]")
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
                        eprintln!("{}", hyprdesk::tr!("attention : {warn}", warn = warn));
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
                    eprintln!("{}", hyprdesk::tr!("wallpaper split : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Icônes du Panorama dessinées par SmartUI (fournisseur IA de la
        // fonction smartui, LYRA par défaut) : sources vidéo et raccourcis.
        // Vidéo seule dans le lecteur du Panorama : détection et plein écran.
        "panorama" if args.get(1).map(String::as_str) == Some("fullscreen") => {
            let Some((addr, _)) = hyprdesk::panorama::video::tagged().into_iter().next() else {
                eprintln!("{}", hyprdesk::i18n::tr("aucun lecteur du Panorama ouvert"));
                return ExitCode::FAILURE;
            };
            match hyprdesk::panorama::autofull::run(&addr, None) {
                Ok(Some(m)) => {
                    println!("{}", m.id());
                    ExitCode::SUCCESS
                }
                Ok(None) => {
                    println!("{}", hyprdesk::i18n::tr("déjà en vidéo seule"));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        "panorama" if args.get(1).map(String::as_str) == Some("icons") => {
            let force = args.iter().any(|a| a == "--force");
            let what = args
                .iter()
                .skip(2)
                .find(|a| !a.starts_with("--"))
                .map(String::as_str)
                .unwrap_or("all");
            let Some(kinds) = hyprdesk::panorama::icons::Kind::parse(what) else {
                eprintln!(
                    "{}",
                    hyprdesk::i18n::tr(
                        "hyprdesk panorama icons [sources|music|shortcuts|all] [--force]"
                    )
                );
                return ExitCode::FAILURE;
            };
            let mut progress = |i: usize, n: usize, name: &str| {
                println!("{i}/{n} {name}…");
            };
            match hyprdesk::panorama::icons::run(&config_path, &kinds, force, &mut progress) {
                Ok((done, errors)) => {
                    for e in &errors {
                        eprintln!("{e}");
                    }
                    println!(
                        "{}",
                        hyprdesk::tr!("{n} icône(s) dessinée(s) par SmartUI", n = done)
                    );
                    if errors.is_empty() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("hyprdesk panorama icons : {e}", e = e));
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
                eprintln!("{}", hyprdesk::tr!("configuration : {e}", e = e));
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
                eprintln!("{}", hyprdesk::tr!("hyprdesk config : {e}", e = e));
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
                    eprintln!("{}", hyprdesk::tr!("hyprdesk osd : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // État du lecteur (plugin « lecteur ») ; aucun lecteur : sortie vide.
        "media" if args.get(1).map(String::as_str) == Some("status") => {
            if let Some(s) = block_on(hyprdesk::widgets::media::status()) {
                print!("{}", hyprdesk::widgets::media::status_lines(&s));
            }
            ExitCode::SUCCESS
        }
        "media" if args.get(1).map(String::as_str) == Some("seek") => {
            let Some(secs) = args.get(2).and_then(|a| a.parse::<i64>().ok()) else {
                eprintln!("hyprdesk media seek <+N|-N>");
                return ExitCode::from(2);
            };
            let r = block_on(async {
                let s = hyprdesk::widgets::media::status()
                    .await
                    .ok_or_else(|| hyprdesk::i18n::tr("aucun lecteur (MPRIS)").to_string())?;
                hyprdesk::widgets::media::seek(&s.bus, secs).await
            });
            match r {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("hyprdesk media : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Demande à LYRA (plugins : « régler ce conflit »), affichée aussitôt.
        "ask" if args.len() >= 3 => {
            match control::send(control::Command::Ask(args[1].clone(), args[2..].join(" "))) {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        // Notifications affichées par hyprdesk : historique, Ne pas déranger.
        "notifications" => {
            let sub: Vec<&str> = args[1..].iter().map(String::as_str).collect();
            match sub.as_slice() {
                [] | ["list"] => match std::fs::read_to_string(notifications::state_path()) {
                    Ok(s) => {
                        // Fin programmée du Silence (HIG PAT-04).
                        let until = s
                            .lines()
                            .find_map(|l| l.strip_prefix("dnd_until="))
                            .and_then(|u| u.parse::<i64>().ok());
                        for line in s.lines() {
                            if let Some(n) = line.strip_prefix("n=") {
                                let mut f = n.splitn(3, '\t');
                                let (app, title, body) = (
                                    f.next().unwrap_or(""),
                                    f.next().unwrap_or(""),
                                    f.next().unwrap_or(""),
                                );
                                println!(
                                    "{app} : {title}{}",
                                    if body.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" — {body}")
                                    }
                                );
                            } else if line == "dnd=1" {
                                match until {
                                    Some(t) => println!(
                                        "{}",
                                        hyprdesk::tr!(
                                            "Ne pas déranger : activé, jusqu’à {hour}",
                                            hour = notifications::silence_end_label(t)
                                        )
                                    ),
                                    None => println!(
                                        "{}",
                                        hyprdesk::i18n::tr("Ne pas déranger : activé")
                                    ),
                                }
                            }
                        }
                        ExitCode::SUCCESS
                    }
                    Err(_) => {
                        eprintln!(
                            "{}",
                            hyprdesk::i18n::tr(
                                "les notifications sont affichées par un autre démon (wayle, mako…), pas par hyprdesk"
                            )
                        );
                        ExitCode::FAILURE
                    }
                },
                ["dnd"] | ["dnd", "on" | "off"] | ["dnd", "on", "1h" | "soir"] | ["clear"] => {
                    // « Jusqu'à ce soir » est déjà passé, le soir (HIG PAT-04).
                    if sub.last() == Some(&"soir")
                        && notifications::SilenceEnd::Evening
                            .deadline(chrono::Local::now())
                            .is_none()
                    {
                        eprintln!(
                            "{}",
                            hyprdesk::i18n::tr(
                                "il est déjà soir : « 1h » pour une heure, rien pour jusqu’à désactivation"
                            )
                        );
                        return ExitCode::FAILURE;
                    }
                    match control::send(control::Command::Notifications(sub.join(" "))) {
                        Ok(_) => ExitCode::SUCCESS,
                        Err(e) => {
                            eprintln!("{e}");
                            ExitCode::FAILURE
                        }
                    }
                }
                _ => {
                    eprintln!(
                        "{}",
                        hyprdesk::i18n::tr(
                            "usage : hyprdesk notifications [list | dnd [on [1h|soir] | off] | clear]"
                        )
                    );
                    ExitCode::from(2)
                }
            }
        }
        // LYRA vocale : installation, essai de la voix, état.
        "voice" => voice::cli(&args[1..]),
        // Télécommande : une touche pour la fenêtre active (Hyprland).
        "key" => {
            let Some(key) = args.get(1).filter(|k| valid_key(k)) else {
                eprintln!("hyprdesk key <touche> [mods]");
                return ExitCode::from(2);
            };
            let mods = args.get(2).map(String::as_str).unwrap_or("");
            if !mods
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_')
            {
                eprintln!("hyprdesk key <touche> [mods]");
                return ExitCode::from(2);
            }
            let lua = format!(
                "hl.dispatch(hl.dsp.send_shortcut({{ mods = \"{}\", key = \"{}\" }}))",
                hyprpanel_core::ipc::lua_escape(mods),
                hyprpanel_core::ipc::lua_escape(key)
            );
            match hyprpanel_core::ipc::eval(&lua) {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("hyprdesk key : {e}", e = e));
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
                    eprintln!("{}", hyprdesk::tr!("hyprdesk media : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Écran de connexion pour greetd : binaire dédié s'il est là
        // (`setup`, en root, reste ici).
        "greeter" if args.get(1).map(String::as_str) != Some("setup") => {
            let rest: Vec<&str> = args[1..].iter().map(String::as_str).collect();
            companion::exec(companion::GREETER, &companion::forward(&config_path, &rest));
            match rest.as_slice() {
                [] => greeter_ui::run(false, false),
                ["--window"] => greeter_ui::run(false, true),
                ["--demo"] => greeter_ui::run(true, false),
                ["--demo", "--window"] => greeter_ui::run(true, true),
                ["--tty"] => match greeter::run_tty() {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("{}", hyprdesk::tr!("hyprdesk greeter : {e}", e = e));
                        ExitCode::FAILURE
                    }
                },
                _ => {
                    eprintln!("{}", i18n::tr_lines(USAGE));
                    ExitCode::from(2)
                }
            }
        }
        // Mise en place (Réglages → Écran de connexion), en root par pkexec.
        "greeter" if args.get(1).map(String::as_str) == Some("setup") => {
            match greeter_setup::cli(
                args.get(2).map(String::as_str),
                args.get(3).map(String::as_str),
            ) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("hyprdesk greeter setup : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
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
                    eprintln!("{}", hyprdesk::tr!("hyprdesk capture : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Agent polkit de la session, avec sa carte : processus à part, sans
        // Lua, lancé et supervisé par le démon.
        "polkit-ui" => {
            companion::exec(companion::POLKIT, &companion::forward(&config_path, &[]));
            ui::polkit::run(config_path.clone())
        }
        // Agent polkit au terminal : mot de passe demandé ici (comme pkttyagent).
        "polkit-agent" => {
            companion::exec(
                companion::POLKIT,
                &companion::forward(&config_path, &["--tty"]),
            );
            polkit::tty_agent()
        }
        // GPU NVIDIA réservé aux VM : bascule vfio-pci ⇄ nvidia (MLG-162).
        "gpu" => gpu::cli(&args[1..]),
        // Mises à jour en attente, et mise à jour non interactive.
        "updates" => {
            // Un seul mot de passe par mise à jour (MLG-160) : règle polkit.
            if args.get(1).map(String::as_str) == Some("--polkit") {
                let on = match args.get(2).map(String::as_str) {
                    Some("on") => true,
                    Some("off") => false,
                    Some("toggle") => !updates::polkit_keep(),
                    _ => {
                        println!("{}", if updates::polkit_keep() { "on" } else { "off" });
                        return ExitCode::SUCCESS;
                    }
                };
                return match updates::set_polkit_keep(on) {
                    Ok(()) => {
                        hyprdesk::hypr::notify(
                            1,
                            5000,
                            if on {
                                hyprdesk::i18n::tr(
                                    "hyprdesk : un seul mot de passe par mise à jour (5 min)",
                                )
                            } else {
                                hyprdesk::i18n::tr(
                                    "hyprdesk : mot de passe demandé à chaque étape de mise à jour",
                                )
                            },
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        ExitCode::FAILURE
                    }
                };
            }
            let sources = updates::configured_sources(&config_path);
            if sources.is_empty() {
                eprintln!(
                    "{}",
                    hyprdesk::i18n::tr("aucune source de paquets reconnue")
                );
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
                                hyprdesk::i18n::tr("aucun échec de mise à jour enregistré")
                            );
                            ExitCode::FAILURE
                        }
                    };
                }
                _ => {}
            }
            let pending = block_on(updates::check(&sources));
            for p in &pending {
                println!("{} : {}", hyprdesk::i18n::tr(p.label), p.packages.len());
                for name in p.packages.iter().take(30) {
                    println!("  {name}");
                }
            }
            println!(
                "{}",
                hyprdesk::tr!(
                    "{n} paquet(s) à mettre à jour",
                    n = updates::total(&pending)
                )
            );
            ExitCode::SUCCESS
        }
        // Raccourcis de hyprland.lua rangés dans binds.lua (MLG-111) : aperçu,
        // ou `split` pour le faire tout de suite.
        "binds" => {
            // binds.lua tel que hyprdesk l'écrit (MLG-198) : `generate` l'affiche,
            // `generate --write` le régénère (copie si écrit à la main).
            if args.get(1).map(String::as_str) == Some("generate") {
                if args.iter().any(|a| a == "--write") {
                    return match binds_file::adopt(&config_path) {
                        Ok(Some(b)) => {
                            println!(
                                "{}",
                                hyprdesk::tr!("binds.lua régénéré ; copie : {b}", b = b.display())
                            );
                            ExitCode::SUCCESS
                        }
                        Ok(None) => {
                            println!("{}", hyprdesk::i18n::tr("binds.lua déjà à jour"));
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("{e}");
                            ExitCode::FAILURE
                        }
                    };
                }
                return match std::fs::read_to_string(binds_file::path(&config_path)) {
                    Ok(src) => {
                        print!("{}", binds_file::generate(&src));
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("binds.lua : {e}");
                        ExitCode::FAILURE
                    }
                };
            }
            // Accord de binds.lua avec ce que hyprdesk tient (MLG-140).
            if args.get(1).map(String::as_str) == Some("sync") {
                let dry = args.iter().any(|a| a == "--dry-run" || a == "-n");
                let cfg = match config::lua::load_file(&config_path) {
                    Ok(l) => l.config,
                    Err(e) => {
                        eprintln!("{e}");
                        return ExitCode::FAILURE;
                    }
                };
                return match binds_file::sync_hyprdesk(&config_path, &cfg, dry) {
                    Ok(done) if done.is_empty() => {
                        println!(
                            "{}",
                            hyprdesk::tr!("binds.lua est en accord avec hyprdesk.")
                        );
                        ExitCode::SUCCESS
                    }
                    Ok(done) => {
                        for d in done {
                            println!("{d}");
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        ExitCode::FAILURE
                    }
                };
            }
            if args.get(1).map(String::as_str) == Some("split") {
                return match binds_file::migrate(&config_path) {
                    Ok(Some(d)) => {
                        println!(
                            "{}",
                            hyprdesk::tr!(
                                "{n} raccourci(s) déplacé(s) vers binds.lua ; copie : {b}",
                                n = d.moved,
                                b = d.backup.display()
                            )
                        );
                        ExitCode::SUCCESS
                    }
                    Ok(None) => {
                        println!(
                            "{}",
                            hyprdesk::i18n::tr("hyprland.lua n'a plus de raccourci à déplacer.")
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        ExitCode::FAILURE
                    }
                };
            }
            let hypr = session_start::hyprland_path(&config_path);
            let src = std::fs::read_to_string(&hypr).unwrap_or_default();
            let old = std::fs::read_to_string(binds_file::path(&config_path)).ok();
            match binds_file::split(&src, old.as_deref()) {
                Some(s) => {
                    println!(
                        "{}",
                        hyprdesk::tr!(
                            "{n} raccourci(s) à déplacer vers binds.lua ; locales déplacées : {t} ; copiées : {c}",
                            n = s.moved,
                            t = if s.taken.is_empty() {
                                "—".to_string()
                            } else {
                                s.taken.join(", ")
                            },
                            c = if s.copied.is_empty() {
                                "—".to_string()
                            } else {
                                s.copied.join(", ")
                            }
                        )
                    );
                    println!("\n----- binds.lua -----\n{}", s.binds);
                    println!("----- hyprland.lua -----\n{}", s.hyprland);
                }
                None => println!(
                    "{}",
                    hyprdesk::i18n::tr("hyprland.lua n'a plus de raccourci à déplacer.")
                ),
            }
            ExitCode::SUCCESS
        }
        // Raccourcis : où chacun agit, conflits avec hyprland.lua (MLG-40).
        "keys" => {
            let cfg = match ui::check(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            };
            let keys = &cfg.general.keys;
            let source = keymap::load_source(&config_path);
            let layouts = hyprdesk::tiling::in_use(&cfg.general.tiling);
            match hypr::key_report(keys, cfg.general.apps_launcher.unified, &source, &layouts) {
                Ok(r) => {
                    print!("{}", keymap::text_report(keys, &r));
                    if r.issues.is_empty() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("Hyprland injoignable : {e}", e = e));
                    ExitCode::FAILURE
                }
            }
        }
        // Dépendances externes : présentes, absentes, et ce qui en dépend.
        "doctor" => {
            let list = doctor::check();
            // Pour les scripts d'installation : les absentes, sans mise en forme.
            if args.get(1).map(String::as_str) == Some("--missing") {
                print!("{}", doctor::missing_tsv(&list));
                return ExitCode::SUCCESS;
            }
            println!("{}", doctor::report(&list));
            if doctor::essential_missing(&list) {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        c @ ("autostart" | "lid" | "window" | "hyprland-check" | "hyprland-fix"
        | "hyprland-adopt" | "window-places" | "icons" | "app-theme" | "colors") => {
            match cli_tools::run(c, &config_path, &args[1..]) {
                Some(Ok(out)) => {
                    println!("{}", out.trim_end());
                    ExitCode::SUCCESS
                }
                Some(Err(e)) => {
                    eprintln!("hyprdesk {c} : {}", e.trim_end());
                    ExitCode::FAILURE
                }
                None => ExitCode::FAILURE,
            }
        }
        "snippet" => {
            println!("{}", i18n::tr_lines(SNIPPET));
            ExitCode::SUCCESS
        }
        // Index de fichiers : démon + client (recherche :f du lanceur).
        "index" => index_cli(&args[1..]),
        other => match control::Command::parse(&args.join(" ")) {
            Some(c) => match control::send(c) {
                Ok(r) => {
                    if !r.is_empty() && r != "ok" {
                        println!("{r}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", hyprdesk::tr!("hyprdesk : {e}", e = e));
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!(
                    "{}\n\n{}",
                    hyprdesk::tr!("commande inconnue : {other}", other = other),
                    i18n::tr_lines(USAGE)
                );
                ExitCode::from(2)
            }
        },
    }
}

/// Nom de touche xkb plausible (`Up`, `Return`, `f`, `XF86AudioPlay`).
fn valid_key(k: &str) -> bool {
    !k.is_empty() && k.len() <= 32 && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
