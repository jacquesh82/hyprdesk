//! Moteur de thème côté démon (MLG-231) : le moteur fait foi.
//!
//! - À la première config chargée, le thème actif du moteur gagne : s'il
//!   diffère des réglages, il est recopié dans `settings.lua` (miroir) ;
//!   sans thème actif, le thème des réglages est posé (migration).
//! - Un thème changé dans hyprdesk (Réglages, `hyprdesk config set`) est
//!   posé au moteur : pack identique, ou pack utilisateur `perso`.
//! - Un thème posé hors du démon (`hyprdesk-theme-engine use`, LYRA,
//!   `watch`) réveille [`stream`] et est recopié dans le miroir.
//!
//! Dans chaque cas, la palette du pack actif devient celle des couleurs
//! d'état des surfaces ([`super::tone`]) et ses jetons (verre, contraste,
//! typographie, états, mouvement, écrans) ceux de [`super::tokens`]. Un pack
//! ou une variante qui change pose son fond d'écran, s'il en déclare un
//! (MLG-266). Rien de tout cela en aperçu.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::Task;
use iced::futures::Stream;

use crate::config::{Rgba, Theme};
use crate::theme_engine::design::Design;
use crate::theme_engine::{bridge, paths::Roots, resolve};

use super::{App, Msg};

/// Palette et jetons du pack actif, ou l'erreur à journaliser.
pub type Outcome = Result<(BTreeMap<String, Rgba>, Design), String>;

/// Pack et variante actifs vus en dernier par le démon : un changement pose
/// le fond du pack, le démarrage non (le fond de l'utilisateur reste).
static LAST: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Moment {
    /// Première config chargée : le moteur gagne.
    Start,
    /// Thème changé dans hyprdesk : posé au moteur.
    Changed,
    /// Le moteur a changé hors du démon : recopié dans le miroir.
    External,
}

impl App {
    /// Après chaque config installée.
    pub(super) fn sync_theme_engine(&mut self, theme_changed: bool) -> Task<Msg> {
        let first = !self.engine_synced;
        if super::tokens::set_user(
            self.cfg.general.reduced_transparency,
            self.cfg.general.reduced_motion,
            self.cfg.general.increase_contrast,
            self.cfg.general.text_scale,
        ) {
            self.theme = super::make_theme(&self.cfg);
        }
        if first || theme_changed {
            // Couleurs d'état tout de suite, sans attendre le moteur.
            super::tokens::set_palette(resolve::from_theme(&self.cfg.general.theme).palette);
        }
        if self.preview {
            return Task::none();
        }
        self.engine_synced = true;
        let moment = if first {
            Moment::Start
        } else if theme_changed {
            Moment::Changed
        } else {
            return Task::none();
        };
        self.engine_job(moment)
    }

    /// Le moteur a changé (`active.json` ou `palette.json`).
    pub(super) fn on_theme_engine_changed(&self) -> Task<Msg> {
        if self.preview {
            return Task::none();
        }
        self.engine_job(Moment::External)
    }

    fn engine_job(&self, moment: Moment) -> Task<Msg> {
        let (theme, path) = (self.cfg.general.theme.clone(), self.config_path.clone());
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || job(moment, &theme, &path, &Roots::from_env()))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r)
            },
            Msg::ThemeEngine,
        )
    }

    /// Réponse du moteur.
    pub(super) fn on_theme_engine(&mut self, r: Outcome) -> Task<Msg> {
        match r {
            Ok((palette, design)) => {
                super::tokens::set_palette(palette);
                let family = design.typography.family.clone();
                super::tokens::set(design);
                // Action, états : le thème iced suit la palette du pack.
                self.theme = super::make_theme(&self.cfg);
                // Police du pack, si l'utilisateur n'en a pas choisi.
                if self.cfg.general.font.is_none() {
                    self.ui_family = match &family {
                        Some(f) => super::intern_font(f),
                        None => crate::fonts::FAMILY,
                    };
                }
                self.font = self.ui_font(super::tokens::weight(
                    super::tokens::design().typography.weight,
                ));
            }
            Err(e) => tracing::warn!("{}", crate::tr!("moteur de thème : {e}", e = e)),
        }
        Task::none()
    }
}

fn job(moment: Moment, theme: &Theme, config_path: &Path, roots: &Roots) -> Outcome {
    let r = resolution(moment, theme, config_path, roots)?;
    pack_wallpaper(moment, &r, config_path);
    Ok((r.palette, r.design))
}

/// Fond du pack quand le pack ou sa variante change (pas au démarrage).
fn pack_wallpaper(moment: Moment, r: &resolve::Resolved, config_path: &Path) {
    let now = (r.id.clone(), r.variant.clone());
    let Ok(mut last) = LAST.lock() else { return };
    let changed = last.as_ref().is_some_and(|l| *l != now);
    *last = Some(now);
    if moment == Moment::Start || !changed {
        return;
    }
    if let Some(w) = &r.wallpaper
        && let Err(e) = crate::theme::apply_pack_wallpaper(config_path, w)
    {
        tracing::warn!("{}", crate::tr!("fond d'écran du thème : {e}", e = e));
    }
}

fn resolution(
    moment: Moment,
    theme: &Theme,
    config_path: &Path,
    roots: &Roots,
) -> Result<resolve::Resolved, String> {
    let err = |e: crate::theme_engine::Error| e.to_string();
    match moment {
        Moment::Start | Moment::External => {
            if let Some(a) = bridge::active(roots) {
                let engine = bridge::to_theme(&a.resolved);
                if !bridge::same_theme(&engine, theme) {
                    // Le moteur fait foi : les réglages suivent, le
                    // rechargement de la config redessine les surfaces.
                    crate::theme::mirror_engine(config_path, &engine)?;
                    return Ok(a.resolved);
                }
                if moment == Moment::External {
                    return Ok(a.resolved);
                }
            } else if moment == Moment::External {
                return Ok(resolve::from_theme(theme));
            }
            // Démarrage : thème des réglages posé (migration) ou confirmé,
            // palette publiée.
            bridge::adopt(roots, theme, "daemon")
                .map(|a| a.resolved)
                .map_err(err)
        }
        Moment::Changed => bridge::adopt(roots, theme, "user")
            .map(|a| a.resolved)
            .map_err(err),
    }
}

/// Réveils du moteur : `active.json` ou `palette.json` réécrits, regroupés.
pub fn stream() -> impl Stream<Item = ()> + use<> {
    let dir: PathBuf = Roots::from_env().state_dir();
    iced::stream::channel(
        4,
        async move |mut out: iced::futures::channel::mpsc::Sender<()>| {
            use iced::futures::SinkExt as _;
            use notify::{RecursiveMode, Watcher};
            let _ = std::fs::create_dir_all(&dir);
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
            let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let Ok(ev) = res else { return };
                if matches!(ev.kind, notify::EventKind::Access(_)) {
                    return;
                }
                if ev.paths.iter().any(|p| {
                    p.file_name()
                        .is_some_and(|n| n == "active.json" || n == "palette.json")
                }) {
                    let _ = tx.send(());
                }
            });
            let mut watcher = match watcher {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("{}", crate::tr!("suivi du moteur de thème : {e}", e = e));
                    return;
                }
            };
            if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
                tracing::warn!("{}", crate::tr!("suivi du moteur de thème : {e}", e = e));
                return;
            }
            while rx.recv().await.is_some() {
                tokio::time::sleep(Duration::from_millis(300)).await;
                while rx.try_recv().is_ok() {}
                let _ = out.send(()).await;
            }
            drop(watcher);
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme_engine::state;

    fn essai(tag: &str) -> (PathBuf, PathBuf, Roots) {
        let root = std::env::temp_dir().join(format!("hd-ui-engine-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("hypr")).unwrap();
        let config = root.join("hypr/hyprdesk.lua");
        (root.clone(), config, Roots::at(&root.join("xdg")))
    }

    fn thème_des_réglages(config: &Path) -> Theme {
        crate::config::lua::load_settings_only(config)
            .unwrap()
            .general
            .theme
    }

    #[test]
    fn démarrage_sans_thème_actif_migre_les_réglages() {
        let (root, config, roots) = essai("migre");
        let t = Theme::default();
        job(Moment::Start, &t, &config, &roots).unwrap();
        let a = state::read_active(&roots).unwrap();
        assert_eq!(a.pack, "ardoise");
        assert_eq!(state::read_palette(&roots).unwrap().pack, "ardoise");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn démarrage_le_moteur_gagne_et_les_réglages_suivent() {
        let (root, config, roots) = essai("gagne");
        state::write_active(
            &roots,
            &state::ActiveTheme {
                pack: "foret".to_string(),
                variant: "dark".to_string(),
                origin: "cli".to_string(),
            },
        )
        .unwrap();
        let (palette, _) = job(Moment::Start, &Theme::default(), &config, &roots).unwrap();
        assert!(palette.contains_key("warning"));
        // Le miroir porte le pack du moteur.
        let t = thème_des_réglages(&config);
        assert_eq!(t.name, "Forêt");
        assert_eq!(t.accent, crate::config::Rgba(0x8a, 0xff, 0x80, 0xff));
        // Le moteur n'a pas été réécrit par le thème des réglages d'avant.
        assert_eq!(state::read_active(&roots).unwrap().pack, "foret");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn retouche_posée_puis_changement_externe_recopié() {
        let (root, config, roots) = essai("cycle");
        let t = Theme {
            accent: crate::config::Rgba(0x10, 0x20, 0x30, 0xff),
            ..Default::default()
        };
        job(Moment::Changed, &t, &config, &roots).unwrap();
        assert_eq!(state::read_active(&roots).unwrap().pack, bridge::CUSTOM_ID);
        // `hyprdesk-theme-engine use ambre` hors du démon.
        state::write_active(
            &roots,
            &state::ActiveTheme {
                pack: "ambre".to_string(),
                variant: "dark".to_string(),
                origin: "cli".to_string(),
            },
        )
        .unwrap();
        job(Moment::External, &t, &config, &roots).unwrap();
        assert_eq!(thème_des_réglages(&config).name, "Ambre");
        // Second réveil sans changement : rien n'est réécrit.
        let avant = std::fs::metadata(crate::config::lua::settings_path(&config))
            .unwrap()
            .modified()
            .unwrap();
        job(
            Moment::External,
            &thème_des_réglages(&config),
            &config,
            &roots,
        )
        .unwrap();
        let après = std::fs::metadata(crate::config::lua::settings_path(&config))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(avant, après);
        let _ = std::fs::remove_dir_all(&root);
    }
}
