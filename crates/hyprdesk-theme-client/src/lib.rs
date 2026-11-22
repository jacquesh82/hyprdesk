//! Palette de thème de hyprdesk pour les applications tierces
//! (docs/THEME-ENGINE.md §9.3).
//!
//! Une application iced, GTK-personnalisée ou autre lue la palette publiée
//! par `hyprdesk-theme-engine` et suit ses changements, sans dépendre du
//! démon ni de ses réglages :
//!
//! ```no_run
//! use hyprdesk_theme_client::ThemeClient;
//!
//! let client = ThemeClient::new();
//! if let Ok(pal) = client.palette() {
//!     println!("accent : {:?}", pal.slot("accent"));
//! }
//! client.subscribe(move |pal| {
//!     // hors thread UI : convertir en message d'application et redessiner
//!     // seulement les surfaces touchées.
//! });
//! ```
//!
//! Le transport est un seul fichier JSON (`palette.json`, schéma 1) publié
//! par renommage atomique ; la révision ne bouge que si le contenu change :
//! les clients comparent la révision avant de redessiner.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::Watcher as _;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Version du schéma de `palette.json`.
pub const SCHEMA: u32 = 1;

/// Emplacement publié par défaut : `$XDG_STATE_HOME/hyprdesk/theme-engine/`.
pub fn published_path() -> PathBuf {
    let state = std::env::var("XDG_STATE_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| Path::new(&h).join(".local/state"))
        })
        .unwrap_or_else(|| PathBuf::from(".local/state"));
    state
        .join("hyprdesk")
        .join("theme-engine")
        .join("palette.json")
}

/// Couleur RVBA (composantes 0..255).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Document publié, tel que sérialisé.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PaletteDoc {
    pub schema: u32,
    pub pack: String,
    pub variant: String,
    pub revision: u64,
    pub colors: BTreeMap<String, String>,
    #[serde(default)]
    pub effects: BTreeMap<String, EffectDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fonts: Option<Fonts>,
    #[serde(default)]
    pub updated: String,
}

/// Effet publié d'une surface (opacité 0..1, flou derrière).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct EffectDoc {
    pub opacity: f64,
    pub blur: bool,
}

/// Typographie déclarée par le pack, si le pack en porte.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Fonts {
    pub family: Option<String>,
    pub mono_family: Option<String>,
    pub size: Option<f64>,
}

/// Erreurs de la cliente : lecture ou analyse de la palette publiée.
#[derive(Debug, Error)]
pub enum Error {
    #[error("palette non publiée : {0}")]
    Missing(String),
    #[error("palette illisible ({0}) : {1}")]
    Invalid(String, String),
    #[error("schéma inconnu : {0} (attendu : {SCHEMA})")]
    Schema(u32),
    #[error("suivi du fichier : {0}")]
    Watch(String),
}

/// Résultats de la cliente.
pub type Result<T> = std::result::Result<T, Error>;

/// Palette analysée : slots typés et accès par nom.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub pack: String,
    pub variant: String,
    pub revision: u64,
    pub colors: BTreeMap<String, Color>,
    pub effects: BTreeMap<String, EffectDoc>,
    pub fonts: Option<Fonts>,
}

impl Palette {
    /// Slot par nom (`bg`, `fg`, `accent`, `surface`, `accent_fg`…), dérivés
    /// compris.
    pub fn slot(&self, name: &str) -> Option<Color> {
        self.colors.get(name).copied()
    }

    /// Variante sombre ?
    pub fn is_dark(&self) -> bool {
        self.variant != "light"
    }

    /// Opacité d'une surface (`bar`, `menus`, `launcher`, `settings`,
    /// `widgets`), 1.0 si inconnue.
    pub fn opacity(&self, surface: &str) -> f64 {
        self.effects.get(surface).map(|e| e.opacity).unwrap_or(1.0)
    }

    /// Flou d'une surface.
    pub fn blur(&self, surface: &str) -> bool {
        self.effects.get(surface).map(|e| e.blur).unwrap_or(false)
    }
}

impl TryFrom<&PaletteDoc> for Palette {
    type Error = Error;

    fn try_from(doc: &PaletteDoc) -> std::result::Result<Self, Self::Error> {
        if doc.schema != SCHEMA {
            return Err(Error::Schema(doc.schema));
        }
        let mut colors = BTreeMap::new();
        for (name, hex) in &doc.colors {
            colors.insert(
                name.clone(),
                parse_hex(hex).ok_or_else(|| {
                    Error::Invalid(format!("slot {name}"), format!("couleur « {hex} »"))
                })?,
            );
        }
        Ok(Palette {
            pack: doc.pack.clone(),
            variant: doc.variant.clone(),
            revision: doc.revision,
            colors,
            effects: doc.effects.clone(),
            fonts: doc.fonts.clone(),
        })
    }
}

/// `#rrggbb` ou `#rrggbbaa`.
fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some(Color {
        r: byte(0)?,
        g: byte(2)?,
        b: byte(4)?,
        a: if hex.len() == 8 { byte(6)? } else { 255 },
    })
}

/// Cliente du thème publié : un chemin de fichier, rien d'autre.
#[derive(Debug, Clone)]
pub struct ThemeClient {
    path: PathBuf,
}

impl Default for ThemeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeClient {
    /// Cliente sur l'emplacement publié par défaut.
    pub fn new() -> Self {
        Self {
            path: published_path(),
        }
    }

    /// Cliente sur un fichier précis (essais, sessions de démonstration).
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Lit et analyse la palette publiée.
    pub fn palette(&self) -> Result<Palette> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| Error::Missing(format!("{} ({e})", self.path.display())))?;
        let doc: PaletteDoc = serde_json::from_str(&text)
            .map_err(|e| Error::Invalid(self.path.display().to_string(), e.to_string()))?;
        Palette::try_from(&doc)
    }

    /// Révision publiée, sans analyser les couleurs (absente : `None`).
    pub fn revision(&self) -> Option<u64> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        let doc: PaletteDoc = serde_json::from_str(&text).ok()?;
        (doc.schema == SCHEMA).then_some(doc.revision)
    }

    /// Suit les changements de la palette : le rappel est appelé hors thread
    /// de l'application, une fois par révision nouvelle. Le [`Watcher`] retenu
    /// vit tant que l'abonnement est voulu.
    pub fn subscribe<F>(&self, mut callback: F) -> Result<Watcher>
    where
        F: FnMut(&Palette) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(
            move |event: std::result::Result<notify::Event, notify::Error>| {
                // Chaque événement du système de fichiers réveille le fil de
                // lecture ; c'est lui qui déduplique par révision.
                if event.is_ok() {
                    let _ = tx.send(());
                }
            },
        )
        .map_err(|e| Error::Watch(e.to_string()))?;
        watcher
            .watch(&self.path, notify::RecursiveMode::NonRecursive)
            .map_err(|e| Error::Watch(e.to_string()))?;
        let path = self.path.clone();
        let mut last = self.revision();
        let handle = std::thread::Builder::new()
            .name("hyprdesk-theme-client".to_string())
            .spawn(move || {
                loop {
                    // Antirebond : le fichier est souvent touché plusieurs
                    // fois de suite (écriture temporaire, renommage).
                    match rx.recv_timeout(Duration::from_millis(250)) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                    let revision = revision_of(&path);
                    if revision.is_some() && revision != last {
                        last = revision;
                        if let Ok(pal) = read_palette(&path) {
                            callback(&pal);
                        }
                    }
                }
            })
            .map_err(|e| Error::Watch(e.to_string()))?;
        Ok(Watcher {
            handle: Some(handle),
            watcher: Some(watcher),
        })
    }
}

fn revision_of(path: &Path) -> Option<u64> {
    let text = std::fs::read_to_string(path).ok()?;
    let doc: PaletteDoc = serde_json::from_str(&text).ok()?;
    (doc.schema == SCHEMA).then_some(doc.revision)
}

fn read_palette(path: &Path) -> Result<Palette> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::Missing(format!("{} ({e})", path.display())))?;
    let doc: PaletteDoc = serde_json::from_str(&text)
        .map_err(|e| Error::Invalid(path.display().to_string(), e.to_string()))?;
    Palette::try_from(&doc)
}

/// Abonnement vivant : arrêté (et remis au système) à la drop.
pub struct Watcher {
    handle: Option<std::thread::JoinHandle<()>>,
    watcher: Option<notify::RecommendedWatcher>,
}

impl Watcher {
    /// Arrête le fil d'écoute et attend sa fin : la perte du watcher ferme
    /// les événements, le fil de lecture sort au tour suivant.
    pub fn stop(&mut self) {
        self.watcher.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r##"{
  "schema": 1,
  "pack": "ardoise",
  "variant": "dark",
  "revision": 12,
  "colors": {
    "bg": "#171b27",
    "fg": "#f2f4fc",
    "accent": "#a69cff",
    "accent_fg": "#000000",
    "surface": "#252936"
  },
  "effects": { "bar": { "opacity": 0.72, "blur": true } },
  "updated": "2026-11-22T10:12:00Z"
}"##;

    fn temp_doc(content: impl AsRef<str>) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("palette.json");
        std::fs::write(&path, content.as_ref()).unwrap();
        (dir, path)
    }

    #[test]
    fn palette_lue_et_typée() {
        let (_dir, path) = temp_doc(FIXTURE);
        let client = ThemeClient::from_path(&path);
        let pal = client.palette().unwrap();
        assert_eq!(pal.pack, "ardoise");
        assert_eq!(pal.revision, 12);
        assert!(pal.is_dark());
        let accent = pal.slot("accent").unwrap();
        assert_eq!((accent.r, accent.g, accent.b), (0xa6, 0x9c, 0xff));
        assert!(pal.slot("inconnu").is_none());
        assert!((pal.opacity("bar") - 0.72).abs() < 1e-9);
        assert!(pal.blur("bar"));
        assert_eq!(pal.opacity("inconnue"), 1.0);
        assert_eq!(client.revision(), Some(12));
    }

    #[test]
    fn schéma_inconnu_et_couleur_invalide() {
        let (_dir, path) = temp_doc(FIXTURE.replace("\"schema\": 1", "\"schema\": 9"));
        assert!(matches!(
            ThemeClient::from_path(&path).palette(),
            Err(Error::Schema(9))
        ));
        let (_dir, path) = temp_doc(FIXTURE.replace("#a69cff", "#zzzzzz"));
        assert!(matches!(
            ThemeClient::from_path(&path).palette(),
            Err(Error::Invalid(_, _))
        ));
    }

    #[test]
    fn absente_en_erreur_claire() {
        let dir = tempfile::tempdir().unwrap();
        let client = ThemeClient::from_path(dir.path().join("palette.json"));
        assert!(matches!(client.palette(), Err(Error::Missing(_))));
        assert_eq!(client.revision(), None);
    }

    #[test]
    fn alpha_et_opacité_par_défaut() {
        let (_dir, path) = temp_doc(FIXTURE);
        let pal = ThemeClient::from_path(&path).palette().unwrap();
        let accent = pal.slot("accent").unwrap();
        assert_eq!(accent.a, 255);
        // Un slot avec alpha passe aussi.
        assert!(parse_hex("#a69cff66").unwrap().a == 0x66);
    }

    #[test]
    fn abonnement_sur_changement_de_révision() {
        let (dir_holder, path) = temp_doc(FIXTURE);
        let client = ThemeClient::from_path(&path);
        let (tx, rx) = mpsc::channel();
        let mut watcher = client
            .subscribe(move |pal| {
                let _ = tx.send(pal.revision);
            })
            .unwrap();
        // Révision identique : rien ne part.
        std::thread::sleep(Duration::from_millis(400));
        assert!(rx.try_recv().is_err());
        // Révision nouvelle : un rappel, une seule fois par révision.
        std::fs::write(
            &path,
            FIXTURE.replace("\"revision\": 12", "\"revision\": 13"),
        )
        .unwrap();
        let got = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(got, 13);
        // Écriture sans changement de révision : pas de rappel.
        std::fs::write(
            &path,
            FIXTURE.replace("\"revision\": 12", "\"revision\": 13"),
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(600));
        assert!(rx.try_recv().is_err());
        watcher.stop();
        drop(dir_holder);
    }
}
