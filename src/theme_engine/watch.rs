//! Veille du moteur (§10 de la spécification) : réapplication déclenchée.
//!
//! Déclencheurs, sans dépendre du démon :
//! - inotify sur les packs de l'utilisateur et sur `active.json` (le pack ou
//!   la variante change) — jamais sur les autres fichiers d'état, que chaque
//!   application réécrit elle-même ;
//! - socket Unix `$XDG_RUNTIME_DIR/hyprdesk-theme-engine.sock` : une
//!   connexion (une ligne JSON `{reason: …}` ou rien) est un réveil — le
//!   démon ou LYRA pousse ainsi un changement de réglage ;
//! - `SIGHUP` : réveil manuel.
//!
//! Une seule veille à la fois : la prise exclusive du socket en est le
//! verrou ; une seconde instance applique une fois et sort (voir
//! [`running`]). Chaque réapplication suit le parcours complet transactionnel
//! (rapport, palette publiée, journal).

use std::io::{BufRead as _, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::config::Rgba;

use super::apply::{self, Options};
use super::paths::Roots;
use super::state;
use super::{Error, Result, resolve};

/// Antirebond des rafales d'inotify.
pub const DEBOUNCE: Duration = Duration::from_millis(300);

/// Réveil manuel : SIGHUP pose ce drapeau, la boucle le consomme.
static SIGHUP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sighup(_sig: libc::c_int) {
    SIGHUP.store(true, Ordering::SeqCst);
}

/// Resolve et applique le thème actif ; c'est le cœur réutilisable de la
/// veille (et le seul chemin testé en isolation).
pub fn trigger_once(roots: &Roots, origin: &str) -> Result<super::apply::Report> {
    let active = state::read_active(roots)
        .ok_or_else(|| Error::Missing("aucun thème actif : use <id> d'abord".to_string()))?;
    let (all, _) = super::packs::discover(roots, &super::bundled::packs())?;
    let chain = super::packs::chain(&all, &active.pack)?;
    let resolved = resolve::resolve(&chain, &active.variant)?;
    let pack_dir = super::packs::find(&all, &active.pack).and_then(|p| p.dir.clone());
    let opts = Options {
        mode: super::apply::Mode::Persist,
        only: Vec::new(),
        origin: origin.to_string(),
        check_installed: false,
        reload: true,
        target_opts: Default::default(),
        reload_commands: Default::default(),
    };
    Ok(apply::apply(&resolved, pack_dir.as_deref(), roots, &opts))
}

/// Chemin du socket de veille.
pub fn socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|d| PathBuf::from(d).join("hyprdesk-theme-engine.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/hyprdesk-theme-engine.sock"))
}

/// Ligne de sonde : une connexion qui l'envoie vérifie qu'une veille tourne,
/// sans la réveiller.
const PING: &str = "ping";

/// Une veille tourne-t-elle déjà (socket vivant) ?
pub fn running() -> bool {
    match UnixStream::connect(socket_path()) {
        Ok(mut s) => {
            let _ = writeln!(s, "{PING}");
            true
        }
        Err(_) => false,
    }
}

/// Événement inotify qui doit réveiller la veille : création, modification
/// ou suppression dans un pack de l'utilisateur, ou `active.json`. Les
/// lectures (le moteur relit le pack à chaque application) et les autres
/// fichiers d'état (palette, rapport, journal, réécrits par l'application
/// elle-même) n'en sont pas : sans ce filtre, chaque application en
/// déclenche une autre.
pub fn wakes(event: &notify::Event, themes: &Path, state: &Path) -> bool {
    use notify::EventKind;
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return false;
    }
    event.paths.iter().any(|p| {
        if p.starts_with(state) {
            p.file_name().is_some_and(|n| n == "active.json")
        } else {
            p.starts_with(themes)
        }
    })
}

/// Lance la veille : bloque jusqu'à une erreur fatale. `on_apply` reçoit le
/// rapport de chaque réapplication (CLI : résumé imprimé).
pub fn watch(roots: &Roots, mut on_apply: impl FnMut(&super::apply::Report)) -> Result<()> {
    let path = socket_path();
    // Socket vivant : une veille tourne déjà, on ne lui vole pas sa place.
    // Socket mort (veille tuée) : il est repris.
    if running() {
        return Err(Error::Watch(format!("déjà en cours ({})", path.display())));
    }
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)
        .map_err(|e| Error::Watch(format!("socket {} indisponible ({e})", path.display())))?;
    // Le démon de veille sort proprement : le socket saute à la fin.
    let cleanup = path.clone();
    struct Guard(PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = Guard(cleanup);

    let (tx, rx) = mpsc::channel::<&'static str>();
    // Fil socket : chaque connexion est un réveil, sauf la sonde `ping`.
    std::thread::Builder::new()
        .name("hyprdesk-theme-watch-socket".to_string())
        .spawn({
            let tx = tx.clone();
            move || {
                for conn in listener.incoming() {
                    let Ok(conn) = conn else { continue };
                    let _ = conn.set_read_timeout(Some(Duration::from_millis(200)));
                    let mut line = String::new();
                    let _ = std::io::BufReader::new(&conn).read_line(&mut line);
                    if line.trim() == PING {
                        continue;
                    }
                    if tx.send("socket").is_err() {
                        return;
                    }
                }
            }
        })
        .map_err(|e| Error::Watch(format!("fil du socket : {e}")))?;
    // Fil inotify : tous les packs de l'utilisateur (le pack actif peut
    // changer, et un parent hérité compte aussi) et `active.json`. Les
    // dossiers sont créés au besoin : sans thème actif au démarrage, la
    // veille suit quand même le premier `use`.
    let themes = roots.themes_dir();
    std::fs::create_dir_all(&themes)?;
    let state_dir = state::ensure_state_dir(roots)?;
    std::thread::Builder::new()
        .name("hyprdesk-theme-watch-fs".to_string())
        .spawn({
            let tx = tx.clone();
            move || {
                let (event_tx, event_rx) = mpsc::channel::<()>();
                let (t, st) = (themes.clone(), state_dir.clone());
                let Ok(mut watcher) = notify::recommended_watcher(
                    move |event: std::result::Result<notify::Event, notify::Error>| {
                        if let Ok(ev) = event
                            && wakes(&ev, &t, &st)
                        {
                            let _ = event_tx.send(());
                        }
                    },
                ) else {
                    return;
                };
                use notify::Watcher as _;
                let _ = watcher.watch(&themes, notify::RecursiveMode::Recursive);
                let _ = watcher.watch(&state_dir, notify::RecursiveMode::NonRecursive);
                // Le watcher vit tant que ce fil relaie ; la perte du canal
                // principal arrête le fil.
                while event_rx.recv().is_ok() {
                    if tx.send("fichiers").is_err() {
                        return;
                    }
                }
            }
        })
        .map_err(|e| Error::Watch(format!("fil de veille : {e}")))?;
    // SIGHUP : réveil manuel, sans thread dédié.
    unsafe {
        libc::signal(libc::SIGHUP, on_sighup as *const () as libc::sighandler_t);
    }

    let _ = trigger_once(roots, "daemon"); // premier état, sans bruit si absent
    loop {
        if SIGHUP.swap(false, Ordering::SeqCst) {
            match trigger_once(roots, "daemon") {
                Ok(r) => on_apply(&r),
                Err(e) => on_apply(&rapport_déchec(e)),
            }
        }
        match rx.recv_timeout(DEBOUNCE) {
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(Error::Watch("fils de veille terminés".to_string()));
            }
            Ok(_) => {}
        }
        // Antirebond : attendre que la rafale se calme (un enregistrement
        // d'éditeur touche plusieurs fois le fichier), puis appliquer une fois.
        while rx.recv_timeout(DEBOUNCE).is_ok() {}
        match trigger_once(roots, "daemon") {
            Ok(r) => on_apply(&r),
            Err(Error::Missing(_)) => {} // pas encore de thème actif : on attend
            Err(e) => on_apply(&rapport_déchec(e)),
        }
    }
}

/// Rapport minimal pour un échec de réapplication (signalé, jamais muet).
fn rapport_déchec(e: Error) -> super::apply::Report {
    super::apply::Report {
        pack: "veille".to_string(),
        variant: "-".to_string(),
        mode: "persist".to_string(),
        origin: "daemon".to_string(),
        started: chrono::Utc::now().to_rfc3339(),
        duration_ms: 0,
        revision: 0,
        targets: Vec::new(),
        warnings: Vec::new(),
        errors: vec![e.to_string()],
        rolled_back: Vec::new(),
    }
}

/// Palette résolue du thème actif, pour `diff` et `preview`.
pub fn resolved_active(
    roots: &Roots,
    id: &str,
    variant: Option<&str>,
) -> Result<resolve::Resolved> {
    let (all, _) = super::packs::discover(roots, &super::bundled::packs())?;
    let chain = super::packs::chain(&all, id)?;
    let variant = variant.map(str::to_string).unwrap_or_else(|| {
        chain
            .last()
            .map(|p| p.manifest.default_variant().to_string())
            .unwrap_or_default()
    });
    resolve::resolve(&chain, &variant)
}

/// Divergences entre deux palettes résolues : slots et effets qui changent.
pub fn diff(a: &resolve::Resolved, b: &resolve::Resolved) -> Vec<(&'static str, String, String)> {
    let mut out = Vec::new();
    for slot in [
        "bg",
        "surface",
        "surface_raised",
        "fg",
        "fg_muted",
        "border",
        "accent",
        "accent_fg",
        "accent_muted",
        "success",
        "warning",
        "danger",
        "background",
    ] {
        let (ca, cb) = (a.palette.get(slot).copied(), b.palette.get(slot).copied());
        match (ca, cb) {
            (Some(x), Some(y)) if x != y => out.push((
                slot,
                super::color::fmt(x, super::color::Fmt::Hex),
                super::color::fmt(y, super::color::Fmt::Hex),
            )),
            (Some(x), None) => out.push((
                slot,
                super::color::fmt(x, super::color::Fmt::Hex),
                "absent".to_string(),
            )),
            (None, Some(y)) => out.push((
                slot,
                "absent".to_string(),
                super::color::fmt(y, super::color::Fmt::Hex),
            )),
            _ => {}
        }
    }
    for place in crate::config::Place::ALL {
        let (ea, eb) = (a.effects.get(&place), b.effects.get(&place));
        if ea != eb {
            out.push((
                place.as_str(),
                effet_texte(ea.copied()),
                effet_texte(eb.copied()),
            ));
        }
    }
    out
}

fn effet_texte(e: Option<crate::config::Effect>) -> String {
    match e {
        Some(e) => format!("{} (flou : {})", e.opacity, e.blur),
        None => "absent".to_string(),
    }
}

/// Mire de palette en PNG (§ `preview`) : un bloc par slot, deux rangées
/// (sombre et claire quand les variantes existent). Sans texte : les blocs
/// suffisent à l'œil, les valeurs restent dans `palette --format hex`.
pub fn mire_png(resolved: &resolve::Resolved, out: &std::path::Path) -> Result<()> {
    const SLOT_SIZE: u32 = 64;
    const COLS: u32 = 4;
    const SLOTS: [&str; 13] = [
        "bg",
        "surface",
        "surface_raised",
        "fg",
        "fg_muted",
        "border",
        "accent",
        "accent_fg",
        "accent_muted",
        "success",
        "warning",
        "danger",
        "background",
    ];
    let rows = (SLOTS.len() as u32).div_ceil(COLS);
    let mut img = image::RgbImage::new(COLS * SLOT_SIZE, rows * SLOT_SIZE);
    for (i, slot) in SLOTS.iter().enumerate() {
        let c = resolved
            .palette
            .get(*slot)
            .copied()
            .unwrap_or(Rgba(0, 0, 0, 255));
        let (x0, y0) = ((i as u32 % COLS) * SLOT_SIZE, (i as u32 / COLS) * SLOT_SIZE);
        for y in y0..y0 + SLOT_SIZE {
            for x in x0..x0 + SLOT_SIZE {
                img.put_pixel(x, y, image::Rgb([c.0, c.1, c.2]));
            }
        }
    }
    img.save(out)
        .map_err(|e| Error::Manifest("mire".to_string(), e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::state::ActiveTheme;
    use super::*;

    fn roots_essai(tag: &str) -> (std::path::PathBuf, Roots) {
        let root = std::env::temp_dir().join(format!("hd-watch-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        (root.clone(), Roots::at(&root))
    }

    #[test]
    fn trigger_applique_et_publie() {
        let (root, roots) = roots_essai("trigger");
        // Aucun thème actif : erreur claire.
        assert!(matches!(
            trigger_once(&roots, "cli"),
            Err(Error::Missing(_))
        ));
        // Thème actif posé : la réapplication réussit et publie.
        state::write_active(
            &roots,
            &ActiveTheme {
                pack: "ardoise".to_string(),
                variant: "dark".to_string(),
                origin: "cli".to_string(),
            },
        )
        .unwrap();
        let r = trigger_once(&roots, "cli").unwrap();
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert!(state::read_palette(&roots).is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn diff_de_palettes() {
        let a = resolve::from_adjusted(
            Rgba(0xf2, 0xf4, 0xfc, 0xff),
            Rgba(0x17, 0x1b, 0x27, 0xff),
            Rgba(0xa6, 0x9c, 0xff, 0xff),
            Rgba(0x13, 0x17, 0x24, 0xff),
        );
        let mut b = a.clone();
        b.palette
            .insert("accent".to_string(), Rgba(0xff, 0, 0, 255));
        let d = diff(&a, &b);
        assert!(
            d.iter().any(|(s, _, y)| *s == "accent" && y == "#ff0000"),
            "{d:?}"
        );
        // Palettes identiques : aucune divergence.
        assert!(diff(&a, &a).is_empty());
    }

    #[test]
    fn mire_png_écrite() {
        let (root, roots) = roots_essai("mire");
        std::fs::create_dir_all(&root).unwrap();
        let r = resolved_active(&roots, "ardoise", Some("dark")).unwrap();
        let out = root.join("mire.png");
        mire_png(&r, &out).unwrap();
        let meta = std::fs::metadata(&out).unwrap();
        assert!(meta.len() > 1000, "mire trop petite : {}", meta.len());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn la_veille_ignore_ses_propres_écritures() {
        // MLG-221 : palette, rapport et journal réécrits par chaque
        // application relançaient une application, en boucle.
        use notify::event::{AccessKind, CreateKind, EventKind, ModifyKind};
        let themes = Path::new("/r/data/hyprdesk/themes");
        let state = Path::new("/r/state/hyprdesk/theme-engine");
        let ev = |kind, p: &str| notify::Event::new(kind).add_path(PathBuf::from(p));
        let modif = EventKind::Modify(ModifyKind::Any);
        for p in [
            "/r/state/hyprdesk/theme-engine/palette.json",
            "/r/state/hyprdesk/theme-engine/.palette.json.12.0.tmp",
            "/r/state/hyprdesk/theme-engine/last-applied.json",
            "/r/state/hyprdesk/theme-engine/journal.log",
        ] {
            assert!(!wakes(&ev(modif, p), themes, state), "{p}");
        }
        assert!(wakes(
            &ev(modif, "/r/state/hyprdesk/theme-engine/active.json"),
            themes,
            state
        ));
        assert!(wakes(
            &ev(
                EventKind::Create(CreateKind::File),
                "/r/data/hyprdesk/themes/ardoise/theme.toml"
            ),
            themes,
            state
        ));
        // Lecture du pack par l'application : pas un réveil.
        assert!(!wakes(
            &ev(
                EventKind::Access(AccessKind::Any),
                "/r/data/hyprdesk/themes/ardoise/theme.toml"
            ),
            themes,
            state
        ));
    }
}
