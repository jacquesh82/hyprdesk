//! GPU NVIDIA réservé aux VM (vfio-pci) : bascule à chaud vers `nvidia` et
//! retour, sans redémarrer (MLG-162, widget « gpu » de la barre).
//!
//! L'état se lit dans sysfs et /proc, sans lancer de processus : le widget le
//! relit souvent. Seuls comptent les GPU NVIDIA d'une machine de passthrough
//! (sur vfio-pci, ou réservés à vfio au démarrage, ou gardés sur nvidia par
//! hyprdesk) ; une carte qui affiche le bureau n'est jamais proposée.
//!
//! Choix qui protègent la session en cours (repris de `gpu-switch`) :
//! - `nvidia_drm` n'est jamais chargé : aucune carte DRM n'apparaît, Hyprland
//!   n'ouvre pas le GPU et ne retient pas le pilote. CUDA, NVENC, nvidia-smi
//!   et les conteneurs marchent ; pas le rendu graphique déporté.
//! - La fonction audio de la carte reste sur vfio-pci.
//! - `nvidia` est libéré par `rmmod`, jamais en détachant le pilote dans
//!   sysfs : `rmmod` échoue proprement si le pilote sert, le détachement peut
//!   bloquer dans le noyau.
//!
//! Prochains démarrages : à chaque bascule, l'utilisateur choisit « cette
//! session » (le démarrage reste tel quel) ou « aussi au démarrage ». Le choix
//! est gardé dans `/etc/hyprdesk/gpu-boot` et appliqué tôt par le service
//! système `hyprdesk-gpu.service`, avant le gestionnaire de connexion ; la
//! ligne de démarrage (`vfio-pci.ids`) n'est jamais modifiée. Revenir au
//! pilote d'origine efface le choix (et le service, s'il ne sert plus).
//!
//! La partie root passe par `pkexec <hyprdesk de root> gpu apply …` : jamais
//! le hyprdesk de l'utilisateur (`greeter_setup::root_bin`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Mutex;

const NVIDIA_VENDOR: &str = "0x10de";
const BOOT_FILE: &str = "/etc/hyprdesk/gpu-boot";
const UNIT_NAME: &str = "hyprdesk-gpu.service";
const UNIT_FILE: &str = "/etc/systemd/system/hyprdesk-gpu.service";
const LOCK_FILE: &str = "/run/lock/hyprdesk-gpu.lock";
/// Nœuds qui survivent au déchargement de nvidia (certains majeurs changent
/// au chargement suivant).
const NVIDIA_NODES: &[&str] = &[
    "/dev/nvidia0",
    "/dev/nvidiactl",
    "/dev/nvidia-modeset",
    "/dev/nvidia-uvm",
    "/dev/nvidia-uvm-tools",
    "/dev/nvidia-caps",
];

/// Pilote visé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Nvidia,
    Vfio,
}

impl Target {
    pub fn parse(s: &str) -> Option<Target> {
        match s {
            "nvidia" => Some(Target::Nvidia),
            "vfio" | "vfio-pci" => Some(Target::Vfio),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Target::Nvidia => "nvidia",
            Target::Vfio => "vfio",
        }
    }

    fn driver(self) -> &'static str {
        match self {
            Target::Nvidia => "nvidia",
            Target::Vfio => "vfio-pci",
        }
    }
}

/// Un GPU basculable.
#[derive(Debug, Clone, PartialEq)]
pub struct Gpu {
    /// Adresse PCI complète (`0000:01:00.0`).
    pub address: String,
    /// « GeForce RTX 3070 » (pci.ids), sinon « GPU NVIDIA ».
    pub name: String,
    /// Pilote lié : `nvidia`, `vfio-pci`, ou `none`.
    pub driver: String,
    /// Une VM tient le GPU (bascule impossible).
    pub vm: bool,
    /// Pilote du démarrage sans choix de hyprdesk (ligne de démarrage, modprobe.d).
    pub boot_default: Target,
    /// Choix gardé par hyprdesk pour les prochains démarrages.
    pub boot_saved: Option<Target>,
    /// Le module nvidia est installé pour ce noyau.
    pub nvidia_module: bool,
}

impl Gpu {
    /// Pilote des prochains démarrages.
    pub fn boot(&self) -> Target {
        self.boot_saved.unwrap_or(self.boot_default)
    }

    /// Pilote actuel, au sens de la bascule.
    pub fn current(&self) -> Option<Target> {
        match self.driver.as_str() {
            "nvidia" => Some(Target::Nvidia),
            "vfio-pci" => Some(Target::Vfio),
            _ => None,
        }
    }

    /// nvidia | vfio | vm | none
    pub fn state(&self) -> &'static str {
        match self.current() {
            Some(Target::Nvidia) => "nvidia",
            Some(Target::Vfio) if self.vm => "vm",
            Some(Target::Vfio) => "vfio",
            None => "none",
        }
    }
}

/// Racines lues (remplacées dans les tests).
pub struct Roots {
    pub sys: PathBuf,
    pub proc_: PathBuf,
    pub etc: PathBuf,
}

impl Default for Roots {
    fn default() -> Self {
        Roots {
            sys: "/sys".into(),
            proc_: "/proc".into(),
            etc: "/etc".into(),
        }
    }
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn link_name(p: &Path) -> Option<String> {
    std::fs::read_link(p)
        .ok()
        .and_then(|l| l.file_name().map(|n| n.to_string_lossy().into_owned()))
}

fn driver_of(dev: &Path) -> String {
    link_name(&dev.join("driver")).unwrap_or_else(|| "none".into())
}

/// `10de:2488` d'un périphérique PCI.
fn pci_id(dev: &Path) -> String {
    let v = read(&dev.join("vendor"));
    let d = read(&dev.join("device"));
    format!(
        "{}:{}",
        v.trim_start_matches("0x"),
        d.trim_start_matches("0x")
    )
}

/// Identifiants réservés à vfio-pci au démarrage : `vfio-pci.ids=` de la
/// ligne de démarrage, `options vfio-pci ids=` de modprobe.d.
fn vfio_boot_ids(r: &Roots) -> Vec<String> {
    let mut ids = Vec::new();
    let mut take = |list: &str| {
        ids.extend(
            list.split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty()),
        )
    };
    for w in read(&r.proc_.join("cmdline")).split_whitespace() {
        if let Some(v) = w
            .strip_prefix("vfio-pci.ids=")
            .or_else(|| w.strip_prefix("vfio_pci.ids="))
        {
            take(v);
        }
    }
    if let Ok(dir) = std::fs::read_dir(r.etc.join("modprobe.d")) {
        for e in dir.flatten() {
            for l in read(&e.path()).lines() {
                let mut w = l.split_whitespace();
                if w.next() != Some("options") {
                    continue;
                }
                if !matches!(w.next(), Some("vfio-pci" | "vfio_pci")) {
                    continue;
                }
                for o in w {
                    if let Some(v) = o.strip_prefix("ids=") {
                        take(v);
                    }
                }
            }
        }
    }
    ids
}

/// Choix gardés pour le démarrage : `adresse=nvidia|vfio` par ligne.
pub fn parse_boot(s: &str) -> BTreeMap<String, Target> {
    s.lines()
        .filter_map(|l| {
            let (a, t) = l.trim().split_once('=')?;
            Some((a.trim().to_string(), Target::parse(t.trim())?))
        })
        .collect()
}

fn render_boot(m: &BTreeMap<String, Target>) -> String {
    let mut s = String::from(
        "# Pilote du GPU aux prochains démarrages, choisi dans hyprdesk (widget « gpu »).\n\
         # Appliqué par hyprdesk-gpu.service ; une adresse absente garde le pilote du démarrage.\n",
    );
    for (a, t) in m {
        s.push_str(&format!("{a}={}\n", t.as_str()));
    }
    s
}

/// Le module nvidia est-il installé pour le noyau en cours ? (modules.dep
/// relu une fois par noyau : le widget interroge souvent.)
fn nvidia_module(r: &Roots) -> bool {
    static CACHE: Mutex<Option<(String, bool)>> = Mutex::new(None);
    let release = read(&r.proc_.join("sys/kernel/osrelease"));
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = c.as_ref()
        && *k == release
    {
        return *v;
    }
    let dep = Path::new("/lib/modules").join(&release).join("modules.dep");
    let v = std::fs::read_to_string(dep).is_ok_and(|s| s.contains("/nvidia.ko"));
    *c = Some((release, v));
    v
}

/// Nom commercial dans pci.ids, cherché une fois par identifiant.
fn pci_name(id: &str) -> Option<String> {
    static CACHE: Mutex<BTreeMap<String, Option<String>>> = Mutex::new(BTreeMap::new());
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    c.entry(id.to_string())
        .or_insert_with(|| pci_lookup(id))
        .clone()
}

/// « GA104 [GeForce RTX 3070] » → « GeForce RTX 3070 ».
fn pci_lookup(id: &str) -> Option<String> {
    let (vendor, device) = id.split_once(':')?;
    let db = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())?;
    let mut in_vendor = false;
    for l in db.lines() {
        if !l.starts_with(['\t', '#']) && !l.is_empty() {
            if in_vendor {
                return None;
            }
            in_vendor = l.starts_with(vendor);
        } else if in_vendor && l.starts_with('\t') && !l.starts_with("\t\t") {
            let l = l.trim_start();
            if let Some(name) = l.strip_prefix(device) {
                let name = name.trim();
                let short = name
                    .split_once('[')
                    .and_then(|(_, b)| b.split_once(']'))
                    .map_or(name, |(n, _)| n);
                return Some(short.trim().to_string());
            }
        }
    }
    None
}

/// Nœuds VFIO du groupe IOMMU du GPU.
fn vfio_nodes(r: &Roots, dev: &Path) -> Vec<PathBuf> {
    let Some(group) = link_name(&dev.join("iommu_group")) else {
        return Vec::new();
    };
    let mut nodes = vec![PathBuf::from("/dev/vfio").join(&group)];
    let devices = r
        .sys
        .join("kernel/iommu_groups")
        .join(&group)
        .join("devices");
    for d in std::fs::read_dir(devices).into_iter().flatten().flatten() {
        for c in std::fs::read_dir(d.path().join("vfio-dev"))
            .into_iter()
            .flatten()
            .flatten()
        {
            nodes.push(PathBuf::from("/dev/vfio/devices").join(c.file_name()));
        }
    }
    nodes
}

/// Processus de /proc : (pid, nom, ligne de commande).
fn processes(r: &Roots) -> Vec<(PathBuf, String, String)> {
    std::fs::read_dir(&r.proc_)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .map(|e| {
            let p = e.path();
            let comm = read(&p.join("comm"));
            let cmd = std::fs::read(p.join("cmdline"))
                .map(|b| String::from_utf8_lossy(&b).replace('\0', " "))
                .unwrap_or_default();
            (p, comm, cmd)
        })
        .collect()
}

/// Noms des processus dont un descripteur pointe sur un de ces fichiers
/// (ou, pour un préfixe terminé par `*`, un fichier qui commence ainsi).
/// Sans droits root, seuls les processus de l'utilisateur sont visibles.
fn holders(
    procs: &[(PathBuf, String, String)],
    targets: &[PathBuf],
    prefix: Option<&str>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (p, comm, _) in procs {
        let Ok(fds) = std::fs::read_dir(p.join("fd")) else {
            continue;
        };
        let hit = fds.flatten().any(|fd| {
            std::fs::read_link(fd.path()).is_ok_and(|l| {
                targets.contains(&l) || prefix.is_some_and(|x| l.to_string_lossy().starts_with(x))
            })
        });
        if hit && !out.contains(comm) {
            out.push(comm.clone());
        }
    }
    out
}

/// Une VM tient-elle le GPU ? Descripteurs VFIO (root), ou une commande qemu
/// qui cite l'adresse (visible de tous).
fn vm_holds(r: &Roots, dev: &Path, address: &str, procs: &[(PathBuf, String, String)]) -> bool {
    let short = address.split_once(':').map_or(address, |(_, s)| s);
    procs
        .iter()
        .any(|(_, comm, cmd)| comm.starts_with("qemu") && cmd.contains(short))
        || !holders(procs, &vfio_nodes(r, dev), None).is_empty()
}

/// GPU basculables de la machine.
pub fn list_in(r: &Roots) -> Vec<Gpu> {
    let boot_ids = vfio_boot_ids(r);
    let saved = parse_boot(&read(&r.etc.join("hyprdesk/gpu-boot")));
    let mut out = Vec::new();
    let mut procs = None;
    let mut entries: Vec<_> = std::fs::read_dir(r.sys.join("bus/pci/devices"))
        .into_iter()
        .flatten()
        .flatten()
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let dev = e.path();
        if read(&dev.join("vendor")) != NVIDIA_VENDOR
            || !read(&dev.join("class")).starts_with("0x03")
        {
            continue;
        }
        let address = e.file_name().to_string_lossy().into_owned();
        let id = pci_id(&dev);
        let driver = driver_of(&dev);
        let boot_default = if boot_ids.contains(&id) {
            Target::Vfio
        } else {
            Target::Nvidia
        };
        let boot_saved = saved.get(&address).copied();
        // Une carte DRM : elle sert (ou peut servir) au bureau, on n'y touche pas.
        let drm = dev.join("drm").exists();
        let passthrough =
            driver == "vfio-pci" || boot_default == Target::Vfio || boot_saved.is_some();
        if drm || !passthrough || !matches!(driver.as_str(), "vfio-pci" | "nvidia" | "none") {
            continue;
        }
        let procs = procs.get_or_insert_with(|| processes(r));
        let vm = driver == "vfio-pci" && vm_holds(r, &dev, &address, procs);
        out.push(Gpu {
            name: pci_name(&id).unwrap_or_else(|| crate::i18n::tr("GPU NVIDIA").to_string()),
            address,
            driver,
            vm,
            boot_default,
            boot_saved,
            nvidia_module: nvidia_module(r),
        });
    }
    out
}

pub fn list() -> Vec<Gpu> {
    list_in(&Roots::default())
}

const NO_ROOT_BIN: &str = "aucun hyprdesk root ne sait encore basculer le GPU (/usr/bin/hyprdesk est antérieur) : mettre à jour le paquet, ou poser ce build en root (sudo install -m755 target/release/hyprdesk /usr/local/bin/hyprdesk)";

/// hyprdesk root qui sait basculer le GPU : le premier des binaires root
/// (paquet, puis compilation) qui connaît `gpu`. Sondé une fois par binaire
/// et par date de modification (le widget le consulte à chaque rendu).
pub fn switch_bin() -> Option<&'static str> {
    type Probe = Vec<(&'static str, std::time::SystemTime, bool)>;
    static CACHE: Mutex<Probe> = Mutex::new(Vec::new());
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    crate::greeter_setup::root_bins().into_iter().find(|bin| {
        let Ok(mtime) = std::fs::metadata(bin).and_then(|m| m.modified()) else {
            return false;
        };
        if let Some((_, _, ok)) = cache.iter().find(|(b, t, _)| b == bin && *t == mtime) {
            return *ok;
        }
        let ok = Command::new(bin)
            .args(["gpu", "status"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        cache.retain(|(b, _, _)| b != bin);
        cache.push((bin, mtime, ok));
        ok
    })
}

/// Marqueur « bascule en cours » (dossier d'exécution de l'utilisateur).
fn busy_path() -> Option<PathBuf> {
    crate::policy::runtime_dir()
        .ok()
        .map(|d| d.join("hyprdesk-gpu-busy"))
}

/// Une bascule lancée par `hyprdesk gpu` est-elle en cours ?
pub fn busy() -> bool {
    let Some(p) = busy_path() else { return false };
    let pid = read(&p);
    !pid.is_empty() && Path::new("/proc").join(&pid).exists()
}

// ---------------------------------------------------------------- root

fn write(p: &Path, s: &str) -> Result<(), String> {
    std::fs::write(p, s).map_err(|e| format!("{} : {e}", p.display()))
}

fn run(prog: &str, args: &[&str]) -> bool {
    Command::new(prog)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn to_vfio(r: &Roots, dev: &Path) -> Result<(), String> {
    if driver_of(dev) == "vfio-pci" {
        return Ok(());
    }
    let procs = processes(r);
    let users = holders(&procs, &[], Some("/dev/nvidia"));
    if !users.is_empty() {
        return Err(crate::tr!("GPU utilisé par : {p}", p = users.join(" ")));
    }
    for module in ["nvidia_drm", "nvidia_modeset", "nvidia_uvm", "nvidia"] {
        if !r.sys.join("module").join(module).exists() {
            continue;
        }
        if !run("rmmod", &[module]) {
            if module == "nvidia" {
                run("modprobe", &["-i", "nvidia_uvm"]);
            }
            return Err(crate::tr!(
                "Impossible de décharger {m} : il est encore utilisé",
                m = module
            ));
        }
    }
    for n in NVIDIA_NODES {
        let p = Path::new(n);
        let _ = if p.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        };
    }
    run("modprobe", &["vfio-pci"]);
    let address = dev.file_name().unwrap_or_default().to_string_lossy();
    write(&dev.join("driver_override"), "vfio-pci")?;
    let _ = write(&r.sys.join("bus/pci/drivers_probe"), &address);
    let _ = write(&dev.join("driver_override"), "\n");
    match driver_of(dev).as_str() {
        "vfio-pci" => Ok(()),
        d => Err(crate::tr!(
            "vfio-pci n'a pas repris le GPU (pilote actuel : {d})",
            d = d
        )),
    }
}

fn to_nvidia(r: &Roots, dev: &Path, address: &str) -> Result<(), String> {
    if driver_of(dev) == "nvidia" {
        return Ok(());
    }
    if vm_holds(r, dev, address, &processes(r)) {
        return Err(crate::i18n::tr("GPU utilisé par une VM en cours d'exécution").to_string());
    }
    write(&dev.join("driver_override"), "nvidia")?;
    if driver_of(dev) == "vfio-pci" {
        write(&r.sys.join("bus/pci/drivers/vfio-pci/unbind"), address)?;
    }
    // -i : sans la dépendance douce vers nvidia-drm. Mémoire vidéo non
    // préservée : sans les services nvidia-suspend, la veille échouerait.
    let loaded = run(
        "modprobe",
        &["-i", "nvidia", "NVreg_PreserveVideoMemoryAllocations=0"],
    ) && run("modprobe", &["-i", "nvidia_uvm"]);
    if loaded {
        if driver_of(dev) != "nvidia" {
            let _ = write(&r.sys.join("bus/pci/drivers_probe"), address);
        }
        run("nvidia-modprobe", &["-c0", "-u"]);
        let smi_ok = !Path::new("/usr/bin/nvidia-smi").exists()
            || run("timeout", &["20", "nvidia-smi", "-L"]);
        if driver_of(dev) == "nvidia" && smi_ok {
            let _ = write(&dev.join("driver_override"), "\n");
            return Ok(());
        }
    }
    let back = to_vfio(r, dev);
    Err(match back {
        Ok(()) => crate::i18n::tr(
            "Échec du chargement de nvidia, GPU rendu à vfio-pci (voir journalctl -k)",
        )
        .to_string(),
        Err(e) => crate::tr!(
            "Échec du chargement de nvidia, et retour à vfio-pci impossible : {e}",
            e = e
        ),
    })
}

/// Service système qui applique les choix au démarrage.
fn unit(bin: &str) -> String {
    format!(
        "# Écrit par hyprdesk (widget « gpu ») : pilote du GPU choisi pour les démarrages.\n\
         [Unit]\n\
         Description=hyprdesk : pilote du GPU choisi au démarrage\n\
         After=systemd-modules-load.service systemd-udev-settle.service\n\
         Before=display-manager.service greetd.service\n\
         ConditionPathExists={BOOT_FILE}\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         ExecStart={bin} gpu boot\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n"
    )
}

/// Garde `target` pour les prochains démarrages (rien si c'est déjà le
/// pilote du démarrage : le choix est effacé).
fn save_boot(r: &Roots, gpu: &Gpu, target: Target) -> Result<(), String> {
    let path = r.etc.join("hyprdesk/gpu-boot");
    let mut m = parse_boot(&read(&path));
    if target == gpu.boot_default {
        m.remove(&gpu.address);
    } else {
        m.insert(gpu.address.clone(), target);
    }
    if m.is_empty() {
        run("systemctl", &["disable", UNIT_NAME]);
        let _ = std::fs::remove_file(UNIT_FILE);
        let _ = std::fs::remove_file(&path);
        run("systemctl", &["daemon-reload"]);
        return Ok(());
    }
    // Le binaire en cours (lancé par pkexec, donc root et à jour).
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bin = exe.to_string_lossy();
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
    }
    write(&path, &render_boot(&m))?;
    write(Path::new(UNIT_FILE), &unit(&bin))?;
    if !(run("systemctl", &["daemon-reload"]) && run("systemctl", &["enable", UNIT_NAME])) {
        return Err(crate::tr!("activation de {u} impossible", u = UNIT_NAME));
    }
    Ok(())
}

fn find(r: &Roots, address: &str) -> Result<Gpu, String> {
    list_in(r)
        .into_iter()
        .find(|g| g.address == address)
        .ok_or_else(|| crate::tr!("{a} : pas un GPU NVIDIA basculable", a = address))
}

fn switch(r: &Roots, gpu: &Gpu, target: Target) -> Result<(), String> {
    let dev = r.sys.join("bus/pci/devices").join(&gpu.address);
    match target {
        Target::Nvidia => to_nvidia(r, &dev, &gpu.address),
        Target::Vfio => to_vfio(r, &dev),
    }
}

/// `hyprdesk gpu apply <nvidia|vfio> <adresse> <session|boot>` (root).
fn apply(target: Target, address: &str, boot: bool) -> Result<(), String> {
    // SAFETY: geteuid n'a pas de précondition.
    if unsafe { libc::geteuid() } != 0 {
        return Err(
            crate::i18n::tr("à lancer en root (hyprdesk gpu passe par pkexec)").to_string(),
        );
    }
    let lock = std::fs::File::create(LOCK_FILE).map_err(|e| format!("{LOCK_FILE} : {e}"))?;
    if lock.try_lock().is_err() {
        return Err(crate::i18n::tr("Une bascule est déjà en cours").to_string());
    }
    let r = Roots::default();
    let gpu = find(&r, address)?;
    switch(&r, &gpu, target)?;
    if boot {
        save_boot(&r, &gpu, target)?;
    }
    Ok(())
}

/// `hyprdesk gpu boot` (root, hyprdesk-gpu.service) : applique les choix gardés.
fn boot() -> Result<(), String> {
    let r = Roots::default();
    let saved = parse_boot(&read(&r.etc.join("hyprdesk/gpu-boot")));
    let mut errors = Vec::new();
    for (address, target) in saved {
        let res = find(&r, &address).and_then(|g| switch(&r, &g, target));
        if let Err(e) = res {
            errors.push(format!("{address} : {e}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

// ---------------------------------------------------------- utilisateur

fn notify(ok: bool, msg: &str) {
    // Envoi direct (la commande se termine aussitôt) : 1 info, 0 avertissement.
    let _ = hyprpanel_core::ipc::notify(u8::from(ok), if ok { 5000 } else { 10000 }, "0", msg);
}

/// `hyprdesk gpu <nvidia|vfio|toggle> [--boot] [--device ADRESSE]` :
/// bascule par pkexec, notification du résultat.
fn request(target: &str, boot: bool, device: Option<&str>) -> Result<String, String> {
    let gpus = list();
    let gpu = match device {
        Some(a) => gpus
            .iter()
            .find(|g| g.address == a || g.address.ends_with(a)),
        None if gpus.len() > 1 => {
            return Err(
                crate::i18n::tr("plusieurs GPU basculables : préciser --device <adresse>")
                    .to_string(),
            );
        }
        None => gpus.first(),
    }
    .ok_or_else(|| crate::i18n::tr("aucun GPU NVIDIA basculable").to_string())?;
    let target = match target {
        "toggle" => match gpu.current() {
            Some(Target::Vfio) => Target::Nvidia,
            _ => Target::Vfio,
        },
        t => Target::parse(t).ok_or_else(|| crate::tr!("cible inconnue : {t}", t = t))?,
    };
    if target == Target::Nvidia && !gpu.nvidia_module {
        return Err(crate::i18n::tr("pilote nvidia absent pour ce noyau").to_string());
    }
    // Un hyprdesk root plus ancien ne connaît pas `gpu` : le dire avant de
    // demander le mot de passe pour rien.
    let bin = switch_bin().ok_or_else(|| crate::i18n::tr(NO_ROOT_BIN).to_string())?;
    let marker = busy_path();
    if let Some(p) = &marker {
        let _ = std::fs::write(p, std::process::id().to_string());
    }
    let out = Ok::<_, String>(bin).and_then(|bin| {
        Command::new("pkexec")
            .args([bin, "gpu", "apply", target.as_str(), &gpu.address])
            .arg(if boot { "boot" } else { "session" })
            .output()
            .map_err(|e| e.to_string())
    });
    if let Some(p) = &marker {
        let _ = std::fs::remove_file(p);
    }
    let out = out?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(match out.status.code() {
            Some(126 | 127) if err.is_empty() => {
                crate::i18n::tr("Authentification annulée.").to_string()
            }
            _ if err.is_empty() => crate::i18n::tr("Échec, sans message.").to_string(),
            _ => err.trim_start_matches("hyprdesk gpu : ").to_string(),
        });
    }
    Ok(if boot {
        crate::tr!(
            "{n} : pilote {d}, aussi aux prochains démarrages",
            n = gpu.name,
            d = target.driver()
        )
    } else {
        crate::tr!(
            "{n} : pilote {d} jusqu'au prochain démarrage (démarrage : {b})",
            n = gpu.name,
            d = target.driver(),
            b = gpu.boot().driver()
        )
    })
}

fn status() {
    let gpus = list();
    if gpus.is_empty() {
        println!("{}", crate::i18n::tr("aucun GPU NVIDIA basculable"));
    }
    for g in gpus {
        println!(
            "{}  {}  {}  {}",
            g.address,
            g.name,
            g.state(),
            crate::tr!(
                "démarrage : {b}{s}",
                b = g.boot().driver(),
                s = if g.boot_saved.is_some() {
                    crate::i18n::tr(" (choix hyprdesk)")
                } else {
                    ""
                }
            )
        );
    }
}

/// `hyprdesk gpu …`
pub fn cli(args: &[String]) -> ExitCode {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let res = match a.as_slice() {
        [] | ["status"] => {
            status();
            return ExitCode::SUCCESS;
        }
        ["apply", t, address, persist] => match Target::parse(t) {
            Some(t) => apply(t, address, *persist == "boot"),
            None => Err(crate::tr!("cible inconnue : {t}", t = t)),
        },
        ["boot"] => boot(),
        [t @ ("nvidia" | "vfio" | "toggle"), rest @ ..] => {
            let boot = rest.contains(&"--boot");
            let device = rest
                .iter()
                .position(|x| *x == "--device")
                .and_then(|i| rest.get(i + 1).copied());
            match request(t, boot, device) {
                Ok(msg) => {
                    notify(true, &msg);
                    println!("{msg}");
                    return ExitCode::SUCCESS;
                }
                Err(e) => {
                    notify(false, &crate::tr!("Bascule du GPU refusée : {e}", e = e));
                    Err(e)
                }
            }
        }
        _ => {
            eprintln!(
                "{}",
                crate::i18n::tr(
                    "usage : hyprdesk gpu [status | nvidia | vfio | toggle] [--boot] [--device <adresse>]"
                )
            );
            return ExitCode::from(2);
        }
    };
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", crate::tr!("hyprdesk gpu : {e}", e = e));
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    struct Tmp(PathBuf);
    impl Tmp {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn machine(driver: &str, cmdline: &str) -> (Tmp, Roots) {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let t = Tmp(std::env::temp_dir().join(format!("hyprdesk-gpu-{}-{n}", std::process::id())));
        let root = t.path();
        let dev = root.join("sys/bus/pci/devices/0000:01:00.0");
        std::fs::create_dir_all(&dev).unwrap();
        std::fs::write(dev.join("vendor"), "0x10de\n").unwrap();
        std::fs::write(dev.join("device"), "0x2488\n").unwrap();
        std::fs::write(dev.join("class"), "0x030000\n").unwrap();
        if driver != "none" {
            let d = root.join("sys/bus/pci/drivers").join(driver);
            std::fs::create_dir_all(&d).unwrap();
            symlink(&d, dev.join("driver")).unwrap();
        }
        // Le GPU intégré (Intel, carte DRM) n'est jamais proposé.
        let igpu = root.join("sys/bus/pci/devices/0000:00:02.0");
        std::fs::create_dir_all(igpu.join("drm")).unwrap();
        std::fs::write(igpu.join("vendor"), "0x8086\n").unwrap();
        std::fs::write(igpu.join("class"), "0x030000\n").unwrap();
        std::fs::create_dir_all(root.join("proc/sys/kernel")).unwrap();
        std::fs::write(root.join("proc/cmdline"), cmdline).unwrap();
        std::fs::create_dir_all(root.join("etc")).unwrap();
        let r = Roots {
            sys: root.join("sys"),
            proc_: root.join("proc"),
            etc: root.join("etc"),
        };
        (t, r)
    }

    #[test]
    fn gpu_reserve_aux_vm_detecte() {
        let (_t, r) = machine(
            "vfio-pci",
            "root=/dev/x vfio-pci.ids=10de:2488,10de:228b quiet",
        );
        let g = list_in(&r);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].address, "0000:01:00.0");
        assert_eq!(g[0].state(), "vfio");
        assert_eq!(g[0].boot(), Target::Vfio);
        assert_eq!(g[0].boot_saved, None);
    }

    #[test]
    fn vm_qui_tient_le_gpu() {
        let (t, r) = machine("vfio-pci", "vfio-pci.ids=10de:2488");
        let p = t.path().join("proc/4242");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("comm"), "qemu-system-x86\n").unwrap();
        std::fs::write(
            p.join("cmdline"),
            "qemu-system-x86_64\0-device\0vfio-pci,host=0000:01:00.0\0",
        )
        .unwrap();
        assert_eq!(list_in(&r)[0].state(), "vm");
    }

    #[test]
    fn choix_du_demarrage_lu() {
        let (_t, r) = machine("nvidia", "vfio-pci.ids=10de:2488");
        std::fs::create_dir_all(r.etc.join("hyprdesk")).unwrap();
        std::fs::write(
            r.etc.join("hyprdesk/gpu-boot"),
            "# x\n0000:01:00.0=nvidia\n",
        )
        .unwrap();
        let g = &list_in(&r)[0];
        assert_eq!(g.state(), "nvidia");
        assert_eq!(g.boot_default, Target::Vfio);
        assert_eq!(g.boot(), Target::Nvidia);
    }

    #[test]
    fn ids_de_modprobe_d() {
        let (_t, r) = machine("none", "quiet");
        std::fs::create_dir_all(r.etc.join("modprobe.d")).unwrap();
        std::fs::write(
            r.etc.join("modprobe.d/vfio.conf"),
            "options vfio-pci ids=10de:2488 disable_vga=1\n",
        )
        .unwrap();
        assert_eq!(list_in(&r)[0].boot_default, Target::Vfio);
    }

    #[test]
    fn carte_du_bureau_jamais_proposee() {
        // NVIDIA sans passthrough : rien à basculer, le widget se cache.
        let (_t, r) = machine("nvidia", "quiet");
        assert!(list_in(&r).is_empty());
        // Sur nvidia avec une carte DRM (nvidia_drm) : elle peut servir au bureau.
        let (_t, r) = machine("nvidia", "vfio-pci.ids=10de:2488");
        std::fs::create_dir_all(r.sys.join("bus/pci/devices/0000:01:00.0/drm")).unwrap();
        assert!(list_in(&r).is_empty());
    }

    #[test]
    fn fichier_de_demarrage_relu() {
        let mut m = BTreeMap::new();
        m.insert("0000:01:00.0".to_string(), Target::Nvidia);
        assert_eq!(parse_boot(&render_boot(&m)), m);
        assert!(parse_boot("0000:01:00.0=amdgpu\n").is_empty());
    }

    #[test]
    fn service_lance_le_hyprdesk_root() {
        let u = unit("/usr/bin/hyprdesk");
        assert!(u.contains("ExecStart=/usr/bin/hyprdesk gpu boot"));
        assert!(u.contains("Before=display-manager.service"));
    }
}
