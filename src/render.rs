//! Moteur de l'interface de hyprdesk (Réglages → Performance → hyprdesk) :
//! rendu matériel ou logiciel, carte graphique préférée, nombre de fils
//! d'exécution. Les trois sont fixés à la création du processus : un
//! changement prend effet à la relance (`restart_daemon`).
//!
//! - **Rendu** : iced essaie les moteurs de la liste `wgpu,tiny-skia` (matériel,
//!   repli logiciel sans carte utilisable) ou `tiny-skia` seul. Le démon la
//!   passe directement à la copie locale d'iced_layershell
//!   (`iced_layershell::set_backend`) ; les processus à part (verrouillage,
//!   fenêtre myMFA, greeter) la reçoivent par `ICED_BACKEND`.
//! - **Carte graphique** : iced demande par défaut la plus puissante
//!   (`HighPerformance`), ce qui réveille la carte dédiée d'un portable
//!   hybride pour dessiner une barre. « Économe » pose `WGPU_POWER_PREF=low`,
//!   seule entrée qu'offre iced_wgpu ; « Puissante » ne pose rien.
//! - **Fils** : l'exécuteur tokio par défaut d'iced crée un fil par cœur (20
//!   sur un i7 récent) ; [`Executor`] en crée le nombre réglé, sans passer
//!   par l'environnement.
//!
//! Les variables posées ici sont listées dans `HYPRDESK_OWN_ENV` : un
//! processus hyprdesk enfant (verrouillage lancé par hypridle) les recalcule
//! depuis le réglage, alors qu'une valeur posée de l'extérieur prime et
//! s'affiche comme imposée.
//!
//! Le moteur obtenu (le matériel retombe sur le logiciel sans le dire) est
//! relevé par iced_layershell (`renderer_information`), pour que les
//! Réglages disent ce qui tourne vraiment.

use std::path::Path;
use std::sync::OnceLock;

/// Moteur de rendu demandé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Renderer {
    /// wgpu, repli tiny-skia si aucune carte n'est utilisable.
    #[default]
    Hardware,
    /// tiny-skia seulement.
    Software,
}

impl Renderer {
    pub const ALL: [Renderer; 2] = [Renderer::Hardware, Renderer::Software];

    pub fn id(self) -> &'static str {
        match self {
            Renderer::Hardware => "hardware",
            Renderer::Software => "software",
        }
    }

    pub fn from_id(s: &str) -> Option<Renderer> {
        match s.trim().to_lowercase().as_str() {
            "hardware" | "gpu" | "wgpu" => Some(Renderer::Hardware),
            "software" | "cpu" | "tiny-skia" | "tiny_skia" => Some(Renderer::Software),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Renderer::Hardware => "Matériel (carte graphique)",
            Renderer::Software => "Logiciel (processeur)",
        }
    }

    /// Liste de moteurs essayée dans l'ordre par iced.
    fn backend(self) -> &'static str {
        match self {
            Renderer::Hardware => "wgpu,tiny-skia",
            Renderer::Software => "tiny-skia",
        }
    }
}

/// Carte graphique préférée par le rendu matériel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gpu {
    /// Carte intégrée (`WGPU_POWER_PREF=low`).
    #[default]
    Low,
    /// Carte dédiée (choix d'iced sans réglage).
    High,
}

impl Gpu {
    pub const ALL: [Gpu; 2] = [Gpu::Low, Gpu::High];

    pub fn id(self) -> &'static str {
        match self {
            Gpu::Low => "low",
            Gpu::High => "high",
        }
    }

    pub fn from_id(s: &str) -> Option<Gpu> {
        match s.trim().to_lowercase().as_str() {
            "low" | "lowpower" | "integrated" | "igpu" => Some(Gpu::Low),
            "high" | "highperformance" | "discrete" | "dgpu" => Some(Gpu::High),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Gpu::Low => "Économe (carte intégrée)",
            Gpu::High => "Puissante (carte dédiée)",
        }
    }
}

/// Fils d'exécution par défaut : largement assez pour un bureau (IPC,
/// D-Bus, relevés), sans un fil par cœur.
pub const DEFAULT_WORKERS: u16 = 4;
/// Borne haute acceptée par le réglage.
pub const MAX_WORKERS: u16 = 64;
/// Choix proposés dans les Réglages (0 : un par cœur).
pub const WORKER_CHOICES: [u16; 4] = [2, 4, 8, 0];

/// Fils d'exécution valides : 0 (un par cœur) ou 1..=64.
pub fn workers_valid(n: f64) -> Option<u16> {
    (n.fract() == 0.0 && (0.0..=MAX_WORKERS as f64).contains(&n)).then_some(n as u16)
}

/// Ce que le processus a reçu au démarrage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    pub renderer: Renderer,
    pub gpu: Gpu,
    /// 0 : un par cœur.
    pub workers: u16,
    /// Variables posées de l'extérieur qui priment sur les réglages
    /// (`ICED_BACKEND=…`, `WGPU_POWER_PREF=…`).
    pub forced: Vec<String>,
}

static STARTED: OnceLock<Started> = OnceLock::new();
static RUNTIME: OnceLock<tokio::runtime::Handle> = OnceLock::new();

const OWN_ENV: &str = "HYPRDESK_OWN_ENV";

/// Lit les réglages et prépare le moteur, avant tout fil et toute fenêtre.
/// `daemon` : le démon passe le moteur de rendu à iced_layershell plutôt que
/// par l'environnement (ses sous-processus n'en héritent pas).
///
/// Appelé au début de `main`, quand le processus n'a qu'un fil : `set_var`
/// et `remove_var` n'y courent aucun risque de lecture concurrente.
pub fn init(config_path: &Path, daemon: bool) {
    let setting = |k: &str| crate::i18n::early_setting(config_path, k);
    // Variables posées par un hyprdesk parent : recalculées ici.
    let inherited: Vec<String> = std::env::var(OWN_ENV)
        .map(|v| v.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    let external = |k: &str| {
        std::env::var(k)
            .ok()
            .filter(|v| !v.trim().is_empty() && !inherited.iter().any(|i| i == k))
    };
    let mut own: Vec<&str> = Vec::new();
    let mut forced = Vec::new();

    let renderer = setting("renderer")
        .and_then(|v| Renderer::from_id(&v))
        .unwrap_or_default();
    if let Some(v) = external("ICED_BACKEND") {
        forced.push(format!("ICED_BACKEND={v}"));
    } else if daemon {
        iced_layershell::set_backend(renderer.backend());
        // SAFETY : un seul fil (voir plus haut).
        unsafe { std::env::remove_var("ICED_BACKEND") };
    } else {
        unsafe { std::env::set_var("ICED_BACKEND", renderer.backend()) };
        own.push("ICED_BACKEND");
    }

    let gpu = setting("gpu")
        .and_then(|v| Gpu::from_id(&v))
        .unwrap_or_default();
    if let Some(v) = external("WGPU_POWER_PREF") {
        forced.push(format!("WGPU_POWER_PREF={v}"));
    } else if gpu == Gpu::Low {
        unsafe { std::env::set_var("WGPU_POWER_PREF", "low") };
        own.push("WGPU_POWER_PREF");
    } else {
        unsafe { std::env::remove_var("WGPU_POWER_PREF") };
    }

    let workers = setting("worker_threads")
        .and_then(|v| v.parse::<f64>().ok())
        .and_then(workers_valid)
        .unwrap_or(DEFAULT_WORKERS);

    if own.is_empty() {
        unsafe { std::env::remove_var(OWN_ENV) };
    } else {
        unsafe { std::env::set_var(OWN_ENV, own.join(",")) };
    }
    let _ = STARTED.set(Started {
        renderer,
        gpu,
        workers,
        forced,
    });
}

/// Ce que le processus a reçu au démarrage (`None` hors `main`, en test).
pub fn started() -> Option<&'static Started> {
    STARTED.get()
}

/// Exécuteur tokio d'iced, au nombre de fils réglé (`worker_threads`).
pub struct Executor(tokio::runtime::Runtime);

impl iced::Executor for Executor {
    fn new() -> Result<Self, std::io::Error> {
        let mut b = tokio::runtime::Builder::new_multi_thread();
        b.enable_all().thread_name("hyprdesk-rt");
        let n = started().map_or(DEFAULT_WORKERS, |s| s.workers);
        if n > 0 {
            b.worker_threads(n as usize);
        }
        let rt = b.build()?;
        let _ = RUNTIME.set(rt.handle().clone());
        Ok(Self(rt))
    }

    #[allow(clippy::let_underscore_future)]
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let _ = self.0.spawn(future);
    }

    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = self.0.enter();
        f()
    }

    fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        self.0.block_on(future)
    }
}

/// Fils d'exécution qui tournent (`None` avant la création de l'exécuteur).
pub fn running_workers() -> Option<usize> {
    RUNTIME.get().map(|h| h.metrics().num_workers())
}

/// Cœurs du processeur (valeur de « un par cœur »).
pub fn cores() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// Moteur effectivement obtenu : `Some(true)` matériel, `Some(false)`
/// logiciel, `None` si aucune surface n'est encore créée.
pub fn running_hardware() -> Option<bool> {
    iced_layershell::renderer_information().map(|i| i.backend != "tiny-skia")
}

/// Description lisible du moteur en cours (« matériel, wgpu Vulkan sur … »).
pub fn running_summary() -> Option<String> {
    let i = iced_layershell::renderer_information()?;
    Some(if i.backend == "tiny-skia" {
        crate::i18n::tr("logiciel, tiny-skia sur le processeur").to_string()
    } else {
        crate::tr!("matériel, wgpu {b} sur {a}", b = i.backend, a = i.adapter)
    })
}

/// Les réglages diffèrent de ce que le processus a reçu : une relance les
/// appliquera.
pub fn restart_pending(renderer: Renderer, gpu: Gpu, workers: u16) -> bool {
    started().is_some_and(|s| s.renderer != renderer || s.gpu != gpu || s.workers != workers)
}

/// Relance le démon pour changer de moteur. Sous systemd, le service est
/// relancé (sans attendre : la commande nous arrête) ; lancé à la main, une
/// nouvelle instance prend la relève (`daemon --replace`).
pub fn restart_daemon() -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut cmd = if std::env::var_os("INVOCATION_ID").is_some() {
        let mut c = Command::new("systemctl");
        c.args(["--user", "--no-block", "restart", "hyprdesk.service"]);
        c
    } else {
        let mut c = Command::new("setsid");
        c.arg("-f")
            .arg(std::env::current_exe()?)
            .args(["daemon", "--replace"]);
        c
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiants() {
        for r in Renderer::ALL {
            assert_eq!(Renderer::from_id(r.id()), Some(r));
        }
        for g in Gpu::ALL {
            assert_eq!(Gpu::from_id(g.id()), Some(g));
        }
        assert_eq!(Renderer::from_id("GPU"), Some(Renderer::Hardware));
        assert_eq!(Renderer::from_id("tiny-skia"), Some(Renderer::Software));
        assert_eq!(Renderer::from_id("opengl"), None);
        assert_eq!(Gpu::from_id("dGPU"), Some(Gpu::High));
        assert_eq!(Renderer::default(), Renderer::Hardware);
        assert_eq!(Gpu::default(), Gpu::Low);
    }

    #[test]
    fn fils_valides() {
        assert_eq!(workers_valid(0.0), Some(0));
        assert_eq!(workers_valid(4.0), Some(4));
        assert_eq!(workers_valid(64.0), Some(64));
        assert_eq!(workers_valid(65.0), None);
        assert_eq!(workers_valid(2.5), None);
        assert_eq!(workers_valid(-1.0), None);
        assert!(WORKER_CHOICES.contains(&DEFAULT_WORKERS));
    }

    #[test]
    fn textes_traduits() {
        let missing = crate::i18n::untranslated(
            "render",
            Renderer::ALL
                .map(Renderer::label)
                .into_iter()
                .chain(Gpu::ALL.map(Gpu::label))
                .chain([
                    "logiciel, tiny-skia sur le processeur",
                    "matériel, wgpu {b} sur {a}",
                ]),
        );
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }
}
