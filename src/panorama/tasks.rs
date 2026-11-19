//! Gestionnaire des tâches du Panorama (MLG-172) : les programmes en cours,
//! triés sur la mesure de la tuile touchée (CPU, mémoire, GPU, VRAM,
//! disque). CPU et disque se calculent entre deux relevés de `/proc` ; GPU
//! et VRAM viennent de `nvidia-smi pmon` (carte NVIDIA).

use std::collections::HashMap;
use std::time::Instant;

/// Un programme en cours.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Proc {
    pub pid: i32,
    pub name: String,
    /// % d'un cœur (comme `top`).
    pub cpu: f32,
    /// Mémoire résidente (octets).
    pub mem: u64,
    /// % du GPU (NVIDIA), s'il est connu.
    pub gpu: Option<f32>,
    /// Mémoire vidéo (Mio), si elle est connue.
    pub vram: Option<u64>,
    /// Lectures et écritures disque (octets/s).
    pub io: f64,
    /// Programme de la personne (peut être terminé).
    pub own: bool,
}

/// Critère de tri d'une mesure de tuile.
pub fn sort_key(metric: &str) -> &'static str {
    match metric {
        "ram" => "mem",
        "gpu" | "gpu_temp" => "gpu",
        "vram" => "vram",
        "disk" => "io",
        _ => "cpu",
    }
}

/// Relevés précédents (temps CPU et octets disque par programme).
#[derive(Debug, Default)]
pub struct Sampler {
    prev: HashMap<i32, (u64, u64)>,
    at: Option<Instant>,
}

fn clk_tck() -> f64 {
    // SAFETY : sysconf ne lit qu'une constante du système.
    let t = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if t > 0 { t as f64 } else { 100.0 }
}

fn page_size() -> u64 {
    // SAFETY : idem.
    let p = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if p > 0 { p as u64 } else { 4096 }
}

/// Temps CPU (utime + stime, en tops) d'après `/proc/<pid>/stat`.
fn cpu_ticks(stat: &str) -> Option<u64> {
    // Le nom (champ 2) peut contenir des espaces : après la dernière « ) ».
    let rest = &stat[stat.rfind(')')? + 2..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    // Champs 14 et 15 du fichier : 12e et 13e après l'état.
    Some(f.get(11)?.parse::<u64>().ok()? + f.get(12)?.parse::<u64>().ok()?)
}

/// GPU et VRAM par programme (`nvidia-smi pmon`), s'il y a une carte NVIDIA.
fn nvidia() -> HashMap<i32, (Option<f32>, Option<u64>)> {
    let Ok(out) = std::process::Command::new("nvidia-smi")
        .args(["pmon", "-c", "1", "-s", "um"])
        .stderr(std::process::Stdio::null())
        .output()
    else {
        return HashMap::new();
    };
    parse_pmon(&String::from_utf8_lossy(&out.stdout))
}

/// Lignes de `nvidia-smi pmon -s um` : gpu, pid, type, sm, mem, enc, dec,
/// jpg, ofa, fb (Mio)… ; « - » : inconnu.
fn parse_pmon(s: &str) -> HashMap<i32, (Option<f32>, Option<u64>)> {
    let mut out = HashMap::new();
    for l in s.lines().filter(|l| !l.trim_start().starts_with('#')) {
        let f: Vec<&str> = l.split_whitespace().collect();
        let Some(pid) = f.get(1).and_then(|p| p.parse::<i32>().ok()) else {
            continue;
        };
        let sm = f.get(3).and_then(|v| v.parse::<f32>().ok());
        let fb = f.get(9).and_then(|v| v.parse::<u64>().ok());
        out.insert(pid, (sm, fb));
    }
    out
}

/// Programmes en cours ; `gpu` : interroge aussi la carte NVIDIA (lent :
/// environ une seconde). Bloquant.
pub fn sample(s: &mut Sampler, gpu: bool) -> Vec<Proc> {
    let now = Instant::now();
    let dt = s.at.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);
    let (tck, page) = (clk_tck(), page_size());
    // SAFETY : getuid ne peut pas échouer.
    let me = unsafe { libc::getuid() };
    let gpus = if gpu { nvidia() } else { HashMap::new() };
    let mut next = HashMap::new();
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return out;
    };
    for e in dir.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<i32>().ok()) else {
            continue;
        };
        let p = e.path();
        let Some(ticks) = std::fs::read_to_string(p.join("stat"))
            .ok()
            .and_then(|s| cpu_ticks(&s))
        else {
            continue;
        };
        let name = std::fs::read_to_string(p.join("comm"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let mem = std::fs::read_to_string(p.join("statm"))
            .ok()
            .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
            .unwrap_or(0)
            * page;
        let io = std::fs::read_to_string(p.join("io"))
            .ok()
            .map(|s| {
                s.lines()
                    .filter(|l| l.starts_with("read_bytes") || l.starts_with("write_bytes"))
                    .filter_map(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
                    .sum::<u64>()
            })
            .unwrap_or(0);
        use std::os::unix::fs::MetadataExt;
        let own = std::fs::metadata(&p).is_ok_and(|m| m.uid() == me);
        let (cpu, io_rate) = match s.prev.get(&pid) {
            Some((t0, i0)) if dt > 0.05 => (
                (ticks.saturating_sub(*t0) as f64 / tck / dt * 100.0) as f32,
                io.saturating_sub(*i0) as f64 / dt,
            ),
            _ => (0.0, 0.0),
        };
        next.insert(pid, (ticks, io));
        let (g, v) = gpus.get(&pid).copied().unwrap_or((None, None));
        out.push(Proc {
            pid,
            name,
            cpu,
            mem,
            gpu: g,
            vram: v,
            io: io_rate,
            own,
        });
    }
    s.prev = next;
    s.at = Some(now);
    out
}

/// Trie sur le critère (`sort_key`), le plus gourmand d'abord.
pub fn sort(list: &mut [Proc], key: &str) {
    let v = |p: &Proc| -> f64 {
        match key {
            "mem" => p.mem as f64,
            "gpu" => p.gpu.unwrap_or(-1.0) as f64,
            "vram" => p.vram.map_or(-1.0, |v| v as f64),
            "io" => p.io,
            _ => p.cpu as f64,
        }
    };
    list.sort_by(|a, b| v(b).total_cmp(&v(a)).then(b.mem.cmp(&a.mem)));
}

/// Demande l'arrêt d'un programme de la personne (SIGTERM).
pub fn terminate(pid: i32) -> Result<(), String> {
    if pid <= 1 {
        return Err("pid invalide".into());
    }
    // SAFETY : simple envoi de signal ; le noyau refuse ce qui n'est pas à nous.
    let r = unsafe { libc::kill(pid, libc::SIGTERM) };
    if r == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releves_lus() {
        let stat = "1234 (Web Content) S 1 2 3 4 5 6 7 8 9 10 150 50 0 0 20 0";
        assert_eq!(cpu_ticks(stat), Some(200));
        let m = parse_pmon(
            "# gpu pid type sm mem enc dec jpg ofa fb ccpm command\n\
             # Idx # C/G % % % % % % MB MB name\n\
             0 904541 G 32 6 - - - - 289 0 Hyprland\n\
             0 909427 G - - - - - - 123 0 kitty\n",
        );
        assert_eq!(m[&904541], (Some(32.0), Some(289)));
        assert_eq!(m[&909427], (None, Some(123)));
        let mut l = vec![
            Proc { pid: 1, cpu: 5.0, mem: 10, ..Default::default() },
            Proc { pid: 2, cpu: 50.0, mem: 1, ..Default::default() },
        ];
        sort(&mut l, "cpu");
        assert_eq!(l[0].pid, 2);
        sort(&mut l, "mem");
        assert_eq!(l[0].pid, 1);
        assert_eq!(sort_key("gpu_temp"), "gpu");
        assert!(terminate(1).is_err());
    }
}
