//! Avancement d'une mise à jour (MLG-161), lu dans la sortie des
//! gestionnaires de paquets pendant qu'ils tournent.
//!
//! Chaque étape (une source) va de 0 à 1 en traversant des phases :
//! téléchargement, vérifications, installation, puis les crochets de fin. Les
//! repères sont ceux que chaque gestionnaire écrit hors terminal (`LC_ALL=C`) :
//!
//! - pacman (et yay / paru, même sortie) : `Packages (28)`, `<paquet>
//!   downloading...`, `checking keyring...`, `upgrading <paquet>...` (au
//!   terminal, précédés de `(3/28)`), `:: Running post-transaction hooks...`
//!   puis `( 2/18) …` ;
//! - apt : `28 upgraded, 1 newly installed`, `Get:3 …`, `Unpacking <paquet>`,
//!   `Setting up <paquet>` ;
//! - dnf (4 et 5), zypper, flatpak et toute commande inconnue : le dernier
//!   compteur `i/n` lu (`(3/28)`, `[ 3/28]`, `… 3/28`).
//!
//! L'avancement ne recule jamais : un compteur qui repart à 1 dans une phase
//! suivante ne fait pas rebrousser la barre.

use std::sync::LazyLock;

use regex::Regex;

/// Phase en cours, écrite telle quelle pour le plugin (qui la traduit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Prepare,
    Download,
    Check,
    Install,
    Hooks,
}

impl Phase {
    pub fn key(self) -> &'static str {
        match self {
            Phase::Prepare => "prepare",
            Phase::Download => "download",
            Phase::Check => "check",
            Phase::Install => "install",
            Phase::Hooks => "hooks",
        }
    }
}

/// Lecteur de la sortie d'une étape.
#[derive(Debug, Clone)]
pub struct Tracker {
    source: String,
    /// Paquets attendus (relevé, puis ce que le gestionnaire annonce).
    expected: usize,
    downloaded: usize,
    unpacked: usize,
    set_up: usize,
    /// pacman : paquets installés, vérifications passées.
    installed: usize,
    checks: usize,
    /// apt : la liste des paquets a été annoncée (fin de `apt-get update`).
    announced: bool,
    pub phase: Phase,
    pub current: Option<String>,
    fraction: f32,
}

static COUNTER: LazyLock<Regex> = LazyLock::new(|| {
    // `(3/28)`, `[ 3/28]`, ou `3/28` en fin de ligne.
    Regex::new(r"(?:[(\[]\s*(\d+)/(\d+)\s*[)\]])|(?:\s(\d+)/(\d+)\s*$)").expect("motif")
});
static PACMAN_STEP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(\s*(\d+)/(\d+)\)\s*").expect("motif"));
static PACMAN_TOTAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Packages \((\d+)\)").expect("motif"));
static APT_TOTAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+) upgraded, (\d+) newly installed").expect("motif"));

fn ratio(i: &str, n: &str) -> Option<f32> {
    let (i, n) = (i.parse::<f32>().ok()?, n.parse::<f32>().ok()?);
    (n > 0.0).then(|| (i / n).clamp(0.0, 1.0))
}

/// Nom de paquet nettoyé : `libcurl4:amd64` (apt) perd son architecture ;
/// `glibc-0:2.40-1.fc41.x86_64` (dnf, époque) reste tel quel.
fn package(s: &str) -> String {
    match s.rsplit_once(':') {
        Some((name, "amd64" | "i386" | "arm64" | "armhf" | "all")) => name.to_string(),
        _ => s.to_string(),
    }
}

impl Tracker {
    pub fn new(source: &str, expected: usize) -> Self {
        Tracker {
            source: source.to_string(),
            expected,
            downloaded: 0,
            unpacked: 0,
            installed: 0,
            checks: 0,
            set_up: 0,
            announced: false,
            phase: Phase::Prepare,
            current: None,
            fraction: 0.0,
        }
    }

    /// Avancement de l'étape, de 0 à 1.
    pub fn fraction(&self) -> f32 {
        self.fraction
    }

    fn advance(&mut self, phase: Phase, f: f32) {
        self.phase = phase;
        self.fraction = self.fraction.max(f.clamp(0.0, 1.0));
    }

    fn part(&self, n: usize) -> f32 {
        if self.expected == 0 {
            // Total inconnu : on avance sans jamais atteindre le bout.
            n as f32 / (n as f32 + 4.0)
        } else {
            (n as f32 / self.expected as f32).min(1.0)
        }
    }

    /// Lit une ligne ; vrai si l'avancement ou le paquet affiché a changé.
    pub fn feed(&mut self, line: &str) -> bool {
        let before = (self.fraction, self.phase, self.current.clone());
        let line = line.trim_end();
        match self.source.as_str() {
            "pacman" | "aur" => self.pacman(line.trim_start()),
            "apt" => self.apt(line.trim_start()),
            _ => self.generic(line),
        }
        before != (self.fraction, self.phase, self.current.clone())
    }

    fn pacman(&mut self, l: &str) {
        if let Some(c) = PACMAN_TOTAL.captures(l) {
            if let Ok(n) = c[1].parse() {
                self.expected = n;
            }
            return;
        }
        if l.starts_with(":: Retrieving packages") {
            self.advance(Phase::Download, 0.0);
            return;
        }
        if let Some(name) = l.strip_suffix(" downloading...") {
            self.downloaded += 1;
            self.current = Some(name.trim().to_string());
            self.advance(Phase::Download, 0.30 * self.part(self.downloaded));
            return;
        }
        if l.starts_with(":: Processing package changes") {
            self.advance(Phase::Install, 0.40);
            return;
        }
        if l.starts_with(":: Running post-transaction hooks") {
            self.current = None;
            self.advance(Phase::Hooks, 0.95);
            return;
        }
        // Crochets : `( 2/18) Configuring …`.
        let counter = PACMAN_STEP.captures(l);
        if self.phase == Phase::Hooks {
            if let Some(r) = counter.as_ref().and_then(|c| ratio(&c[1], &c[2])) {
                self.advance(Phase::Hooks, 0.95 + 0.05 * r);
            }
            return;
        }
        // Hors terminal : `upgrading glibc...`, `checking keyring...` ; au
        // terminal, précédés de `(3/28)`.
        let rest = counter
            .as_ref()
            .map_or(l, |c| &l[c.get(0).map_or(0, |m| m.end())..]);
        let mut words = rest.split_whitespace();
        match words.next() {
            Some("upgrading" | "installing" | "reinstalling" | "downgrading" | "removing") => {
                self.installed += 1;
                self.current = words
                    .next()
                    .map(|w| package(w.trim_end_matches("...")))
                    .filter(|w| !w.is_empty());
                let r = counter
                    .as_ref()
                    .and_then(|c| ratio(&c[1], &c[2]))
                    .unwrap_or_else(|| self.part(self.installed));
                self.advance(Phase::Install, 0.40 + 0.55 * r);
            }
            Some("checking" | "loading") if self.phase != Phase::Install => {
                // Cinq vérifications : clés, intégrité, chargement, conflits, place.
                self.checks += 1;
                self.current = None;
                self.advance(Phase::Check, 0.30 + 0.02 * self.checks.min(5) as f32);
            }
            _ => {}
        }
    }

    fn apt(&mut self, l: &str) {
        if let Some(c) = APT_TOTAL.captures(l) {
            let n: usize = c[1].parse::<usize>().unwrap_or(0) + c[2].parse::<usize>().unwrap_or(0);
            self.expected = n;
            self.announced = true;
            self.advance(Phase::Download, 0.05);
        } else if l.starts_with("Get:") {
            if self.announced {
                self.downloaded += 1;
                self.current = l.split_whitespace().nth(4).map(package);
                self.advance(Phase::Download, 0.05 + 0.30 * self.part(self.downloaded));
            } else {
                // `apt-get update` : listes des dépôts.
                self.advance(Phase::Prepare, 0.02);
            }
        } else if let Some(rest) = l.strip_prefix("Unpacking ") {
            self.unpacked += 1;
            self.current = rest.split_whitespace().next().map(package);
            self.advance(Phase::Install, 0.35 + 0.30 * self.part(self.unpacked));
        } else if let Some(rest) = l.strip_prefix("Setting up ") {
            self.set_up += 1;
            self.current = rest.split_whitespace().next().map(package);
            self.advance(Phase::Install, 0.65 + 0.30 * self.part(self.set_up));
        } else if l.starts_with("Processing triggers") {
            self.current = None;
            self.advance(Phase::Hooks, 0.95);
        }
    }

    /// dnf, zypper, flatpak, commande réglée : le dernier compteur `i/n`.
    /// Une ligne de transaction (installation, nettoyage, vérification)
    /// compte après le téléchargement.
    fn generic(&mut self, l: &str) {
        let Some(c) = COUNTER.captures(l) else { return };
        let (i, n) = match (c.get(1), c.get(2), c.get(3), c.get(4)) {
            (Some(i), Some(n), _, _) | (_, _, Some(i), Some(n)) => (i.as_str(), n.as_str()),
            _ => return,
        };
        let Some(r) = ratio(i, n) else { return };
        let lower = l.to_ascii_lowercase();
        let install = [
            "upgrading",
            "installing",
            "updating",
            "removing",
            "erasing",
            "cleanup",
            "running scriptlet",
            "reinstalling",
            "downgrading",
        ]
        .iter()
        .any(|w| lower.contains(w));
        if lower.contains("verifying") {
            self.advance(Phase::Hooks, 0.95 + 0.05 * r);
        } else if install {
            const VERBS: &[&str] = &[
                "upgrading",
                "installing",
                "updating",
                "removing",
                "erasing",
                "reinstalling",
                "downgrading",
            ];
            self.current = l
                .split_whitespace()
                .skip_while(|w| {
                    !VERBS.contains(&w.trim_end_matches(':').to_ascii_lowercase().as_str())
                })
                .skip(1)
                .find(|w| *w != ":")
                .map(package)
                .or_else(|| self.current.clone());
            self.advance(Phase::Install, 0.35 + 0.60 * r);
        } else if self.phase != Phase::Install && self.phase != Phase::Hooks {
            self.advance(Phase::Download, 0.35 * r);
        }
    }
}

/// Avancement de toute la mise à jour : étapes pondérées par leur nombre de
/// paquets (à poids nuls, toutes égales).
#[derive(Debug, Clone)]
pub struct Overall {
    weights: Vec<f32>,
    total: f32,
    done: f32,
    best: f32,
}

impl Overall {
    pub fn new(counts: &[usize]) -> Self {
        let mut weights: Vec<f32> = counts.iter().map(|&n| n as f32).collect();
        if weights.iter().all(|w| *w == 0.0) {
            weights.iter_mut().for_each(|w| *w = 1.0);
        }
        let total = weights.iter().sum::<f32>().max(f32::EPSILON);
        Overall {
            weights,
            total,
            done: 0.0,
            best: 0.0,
        }
    }

    /// Avancement global (0 à 1) quand l'étape `step` en est à `fraction`.
    pub fn at(&mut self, step: usize, fraction: f32) -> f32 {
        let w = self.weights.get(step).copied().unwrap_or(0.0);
        let v = ((self.done + w * fraction.clamp(0.0, 1.0)) / self.total).clamp(0.0, 1.0);
        self.best = self.best.max(v);
        self.best
    }

    /// L'étape `step` est finie.
    pub fn finish(&mut self, step: usize) -> f32 {
        self.done += self.weights.get(step).copied().unwrap_or(0.0);
        self.at(step + 1, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source: &str, expected: usize, out: &str) -> Tracker {
        let mut t = Tracker::new(source, expected);
        for l in out.lines() {
            t.feed(l);
        }
        t
    }

    #[test]
    fn pacman_par_phases() {
        let mut t = Tracker::new("pacman", 0);
        t.feed("Packages (2) firefox-128.0-1  glibc-2.40-1");
        t.feed(":: Retrieving packages...");
        t.feed(" firefox-128.0-1-x86_64 downloading...");
        assert_eq!(t.phase, Phase::Download);
        assert!((t.fraction() - 0.15).abs() < 1e-4);
        t.feed(" glibc-2.40-1-x86_64 downloading...");
        t.feed("(2/2) checking keys in keyring");
        assert_eq!(t.phase, Phase::Check);
        assert!((t.fraction() - 0.32).abs() < 1e-4);
        t.feed(":: Processing package changes...");
        t.feed("(1/2) upgrading firefox                              [------]");
        assert_eq!(t.current.as_deref(), Some("firefox"));
        assert_eq!(t.phase, Phase::Install);
        assert!((t.fraction() - 0.675).abs() < 1e-4);
        t.feed("(2/2) upgrading glibc");
        t.feed(":: Running post-transaction hooks...");
        // Les crochets repartent à 1 : la barre ne recule pas.
        t.feed("(1/4) Arming ConditionNeedsUpdate...");
        assert!(t.fraction() >= 0.95);
        t.feed("(4/4) Updating the desktop file MIME type cache...");
        assert!((t.fraction() - 1.0).abs() < 1e-4);
    }

    /// Sortie réelle de pacman hors terminal (journal des mises à jour).
    #[test]
    fn pacman_hors_terminal() {
        let t = run(
            "pacman",
            0,
            "Packages (4) glibc-2.44-1  libffi-3.8.1-1  attica-6.32.0-1  kconfig-6.32.0-1\n\
             :: Retrieving packages...\n\
             \x20glibc-2.44-1-x86_64 downloading...\n\
             \x20libffi-3.8.1-1-x86_64 downloading...\n\
             checking keyring...\n\
             checking package integrity...\n\
             loading package files...\n\
             checking for file conflicts...\n\
             checking available disk space...\n",
        );
        assert_eq!(t.phase, Phase::Check);
        assert!((t.fraction() - 0.40).abs() < 1e-4);
        let mut t = t;
        t.feed(":: Processing package changes...");
        t.feed("upgrading glibc...");
        assert_eq!(t.current.as_deref(), Some("glibc"));
        assert!((t.fraction() - (0.40 + 0.55 / 4.0)).abs() < 1e-4);
        t.feed("New optional dependencies for xdg-desktop-portal-gnome");
        t.feed("    papers: Print previews");
        t.feed("upgrading libffi...");
        t.feed(":: Running post-transaction hooks...");
        t.feed("( 1/18) Generating the configured locale...");
        t.feed("Generating locales...");
        t.feed("(18/18) Updating the MIME type database...");
        assert_eq!(t.phase, Phase::Hooks);
        assert!((t.fraction() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn apt_par_phases() {
        let t = run(
            "apt",
            0,
            "Get:1 http://deb.debian.org/debian bookworm InRelease [151 kB]\n\
             2 upgraded, 0 newly installed, 0 to remove and 0 not upgraded.\n\
             Get:1 http://deb.debian.org/debian bookworm/main amd64 curl amd64 7.88 [315 kB]\n\
             Get:2 http://deb.debian.org/debian bookworm/main amd64 libcurl4 amd64 7.88 [390 kB]\n\
             Unpacking curl (7.88) over (7.87) ...\n",
        );
        assert_eq!(t.phase, Phase::Install);
        assert_eq!(t.current.as_deref(), Some("curl"));
        assert!((t.fraction() - 0.50).abs() < 1e-4);
        let t = run(
            "apt",
            2,
            "Setting up curl (7.88) ...\nSetting up libcurl4:amd64 (7.88) ...",
        );
        assert_eq!(t.current.as_deref(), Some("libcurl4"));
        assert!((t.fraction() - 0.95).abs() < 1e-4);
    }

    #[test]
    fn compteurs_generiques() {
        // dnf 4
        let t = run(
            "dnf",
            0,
            "(1/2): firefox-128.0-1.fc41.x86_64.rpm  1.2 MB/s | 70 MB 00:58\n\
             (2/2): glibc-2.40-1.fc41.x86_64.rpm    1.0 MB/s |  2 MB 00:02\n\
             Running transaction\n\
               Upgrading        : firefox-128.0-1.fc41.x86_64        1/4\n",
        );
        assert_eq!(t.phase, Phase::Install);
        assert_eq!(t.current.as_deref(), Some("firefox-128.0-1.fc41.x86_64"));
        assert!((t.fraction() - 0.50).abs() < 1e-4);
        // dnf 5 : `[ 3/4] Upgrading x`
        let t = run(
            "dnf",
            0,
            "[3/4] Upgrading glibc-0:2.40-1.fc41.x86_64  100% |",
        );
        assert_eq!(t.current.as_deref(), Some("glibc-0:2.40-1.fc41.x86_64"));
        // zypper
        let t = run(
            "zypper",
            0,
            "(1/4) Installing: curl-8.0-1.1.x86_64 ....[done]",
        );
        assert_eq!(t.current.as_deref(), Some("curl-8.0-1.1.x86_64"));
        // Sans compteur : rien ne bouge.
        let t = run("flatpak", 0, "Looking for updates…");
        assert_eq!(t.fraction(), 0.0);
    }

    #[test]
    fn avancement_global_pondere() {
        let mut o = Overall::new(&[30, 10]);
        assert!((o.at(0, 0.5) - 0.375).abs() < 1e-4);
        assert!((o.finish(0) - 0.75).abs() < 1e-4);
        assert!((o.at(1, 0.5) - 0.875).abs() < 1e-4);
        // Jamais en arrière.
        assert!((o.at(1, 0.1) - 0.875).abs() < 1e-4);
        let mut o = Overall::new(&[0, 0]);
        assert!((o.at(1, 0.0) - 0.0).abs() < 1e-4);
        assert!((o.finish(0) - 0.5).abs() < 1e-4);
    }
}
